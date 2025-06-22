use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Postgres};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl From<String> for TaskStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pending" => TaskStatus::Pending,
            "processing" => TaskStatus::Processing,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            _ => TaskStatus::Pending,
        }
    }
}

impl From<TaskStatus> for String {
    fn from(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Pending => "pending".to_string(),
            TaskStatus::Processing => "processing".to_string(),
            TaskStatus::Completed => "completed".to_string(),
            TaskStatus::Failed => "failed".to_string(),
        }
    }
}

#[derive(Debug, FromRow, Serialize)]
pub struct FileEmbeddingTask {
    pub id: i32,
    pub file_name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub embedding_count: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub file_name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub status: Option<TaskStatus>,
    pub error_message: Option<String>,
    pub embedding_count: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: i32,
    pub file_name: String,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub embedding_count: Option<i32>,
}

impl From<FileEmbeddingTask> for TaskResponse {
    fn from(task: FileEmbeddingTask) -> Self {
        Self {
            id: task.id,
            file_name: task.file_name,
            status: TaskStatus::from(task.status),
            created_at: task.created_at,
            updated_at: task.updated_at,
            started_at: task.started_at,
            completed_at: task.completed_at,
            error_message: task.error_message,
            embedding_count: task.embedding_count,
        }
    }
}

impl FileEmbeddingTask {
    pub async fn create(pool: &Pool<Postgres>, request: CreateTaskRequest) -> Result<TaskResponse> {
        let task = sqlx::query_as!(
            FileEmbeddingTask,
            r#"
            INSERT INTO file_to_embedding_task (file_name)
            VALUES ($1)
            RETURNING id, file_name, status, created_at, updated_at, started_at, completed_at, error_message, embedding_count
            "#,
            request.file_name
        )
        .fetch_one(pool)
        .await?;

        Ok(TaskResponse::from(task))
    }

    pub async fn find_by_id(pool: &Pool<Postgres>, id: i32) -> Result<Option<TaskResponse>> {
        let task = sqlx::query_as!(
            FileEmbeddingTask,
            r#"
            SELECT id, file_name, status, created_at, updated_at, started_at, completed_at, error_message, embedding_count
            FROM file_to_embedding_task
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(task.map(TaskResponse::from))
    }

    pub async fn list_all(
        pool: &Pool<Postgres>,
        status_filter: Option<TaskStatus>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<TaskResponse>> {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        let tasks = match status_filter {
            Some(status) => {
                let status_str: String = status.into();
                sqlx::query_as!(
                    FileEmbeddingTask,
                    r#"
                    SELECT id, file_name, status, created_at, updated_at, started_at, completed_at, error_message, embedding_count
                    FROM file_to_embedding_task
                    WHERE status = $1
                    ORDER BY created_at DESC
                    LIMIT $2 OFFSET $3
                    "#,
                    status_str,
                    limit,
                    offset
                )
                .fetch_all(pool)
                .await?
            }
            None => {
                sqlx::query_as!(
                    FileEmbeddingTask,
                    r#"
                    SELECT id, file_name, status, created_at, updated_at, started_at, completed_at, error_message, embedding_count
                    FROM file_to_embedding_task
                    ORDER BY created_at DESC
                    LIMIT $1 OFFSET $2
                    "#,
                    limit,
                    offset
                )
                .fetch_all(pool)
                .await?
            }
        };

        Ok(tasks.into_iter().map(TaskResponse::from).collect())
    }

    pub async fn update(
        pool: &Pool<Postgres>,
        id: i32,
        request: UpdateTaskRequest,
    ) -> Result<Option<TaskResponse>> {
        let mut query_parts = Vec::new();
        let mut param_count = 1;

        if request.status.is_some() {
            query_parts.push(format!("status = ${}", param_count));
            param_count += 1;
        }
        if request.error_message.is_some() {
            query_parts.push(format!("error_message = ${}", param_count));
            param_count += 1;
        }
        if request.embedding_count.is_some() {
            query_parts.push(format!("embedding_count = ${}", param_count));
            param_count += 1;
        }

        // Always update the updated_at timestamp
        query_parts.push(format!("updated_at = NOW()"));

        // Set started_at when status changes to processing
        if let Some(TaskStatus::Processing) = request.status {
            query_parts.push(format!("started_at = NOW()"));
        }

        // Set completed_at when status changes to completed or failed
        if let Some(status) = &request.status {
            if matches!(status, TaskStatus::Completed | TaskStatus::Failed) {
                query_parts.push(format!("completed_at = NOW()"));
            }
        }

        if query_parts.is_empty() {
            return Self::find_by_id(pool, id).await;
        }

        let set_clause = query_parts.join(", ");
        let query = format!(
            r#"
            UPDATE file_to_embedding_task
            SET {}
            WHERE id = ${}
            RETURNING id, file_name, status, created_at, updated_at, started_at, completed_at, error_message, embedding_count
            "#,
            set_clause,
            param_count
        );

        let mut query_builder = sqlx::query_as::<_, FileEmbeddingTask>(&query);

        if let Some(status) = request.status {
            let status_str: String = status.into();
            query_builder = query_builder.bind(status_str);
        }
        if let Some(error_message) = request.error_message {
            query_builder = query_builder.bind(error_message);
        }
        if let Some(embedding_count) = request.embedding_count {
            query_builder = query_builder.bind(embedding_count);
        }

        query_builder = query_builder.bind(id);

        let task = query_builder.fetch_optional(pool).await?;

        Ok(task.map(TaskResponse::from))
    }

    pub async fn delete(pool: &Pool<Postgres>, id: i32) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            DELETE FROM file_to_embedding_task
            WHERE id = $1
            "#,
            id
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}