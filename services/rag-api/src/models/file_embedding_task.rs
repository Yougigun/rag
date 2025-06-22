use anyhow::Result;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Postgres};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Unknown,
}

impl From<String> for TaskStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pending" => Self::Pending,
            "processing" => Self::Processing,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            _ => Self::Unknown,
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
            TaskStatus::Unknown => "unknown".to_string(),
        }
    }
}

#[derive(Debug, FromRow, Serialize)]
pub struct FileEmbeddingTask {
    pub id: i32,
    pub file_name: String,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub started_at: Option<NaiveDateTime>,
    pub completed_at: Option<NaiveDateTime>,
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub started_at: Option<NaiveDateTime>,
    pub completed_at: Option<NaiveDateTime>,
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
        let task = sqlx::query_as::<_, Self>(
            "
            INSERT INTO file_to_embedding_task (file_name)
            VALUES ($1)
            RETURNING id, file_name, status, created_at, updated_at, started_at, completed_at, error_message, embedding_count
            ",
        )
        .bind(request.file_name)
        .fetch_one(pool)
        .await?;

        Ok(TaskResponse::from(task))
    }

    pub async fn find_by_id(pool: &Pool<Postgres>, id: i32) -> Result<Option<TaskResponse>> {
        let task = sqlx::query_as::<_, Self>(
            "
            SELECT id, file_name, status, created_at, updated_at, started_at, completed_at, error_message, embedding_count
            FROM file_to_embedding_task
            WHERE id = $1
            ",
        )
        .bind(id)
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
                sqlx::query_as::<_, Self>(
                    "
                    SELECT id, file_name, status, created_at, updated_at, started_at, completed_at, error_message, embedding_count
                    FROM file_to_embedding_task
                    WHERE status = $1
                    ORDER BY created_at DESC
                    LIMIT $2 OFFSET $3
                    ",
                )
                .bind(status_str)
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, Self>(
                    "
                    SELECT id, file_name, status, created_at, updated_at, started_at, completed_at, error_message, embedding_count
                    FROM file_to_embedding_task
                    ORDER BY created_at DESC
                    LIMIT $1 OFFSET $2
                    ",
                )
                .bind(limit)
                .bind(offset)
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
        // Simple update - only update provided fields
        if request.status.is_none()
            && request.error_message.is_none()
            && request.embedding_count.is_none()
        {
            return Self::find_by_id(pool, id).await;
        }

        // For now, we'll do a basic update that handles status changes
        let status_str: Option<String> = request.status.map(|s| s.into());

        let task = sqlx::query_as::<_, Self>(
            "
            UPDATE file_to_embedding_task
            SET status = COALESCE($1, status),
                error_message = COALESCE($2, error_message),
                embedding_count = COALESCE($3, embedding_count),
                updated_at = NOW(),
                started_at = CASE 
                    WHEN $1 = 'processing' AND started_at IS NULL THEN NOW() 
                    ELSE started_at 
                END,
                completed_at = CASE 
                    WHEN $1 IN ('completed', 'failed') AND completed_at IS NULL THEN NOW()
                    ELSE completed_at 
                END
            WHERE id = $4
            RETURNING id, file_name, status, created_at, updated_at, started_at, completed_at, error_message, embedding_count
            ",
        )
        .bind(status_str)
        .bind(request.error_message)
        .bind(request.embedding_count)
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(task.map(TaskResponse::from))
    }

    pub async fn delete(pool: &Pool<Postgres>, id: i32) -> Result<bool> {
        let result = sqlx::query(
            "
            DELETE FROM file_to_embedding_task
            WHERE id = $1
            ",
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
