use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use sqlx::{Pool, Postgres};

use crate::{AppState, models::file_embedding_task::{CreateTaskRequest, FileEmbeddingTask, TaskStatus, UpdateTaskRequest}};

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    pub status: Option<TaskStatus>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn create_task(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    // Create task in database
    match FileEmbeddingTask::create(&app_state.db_pool, payload).await {
        Ok(task) => {
            // Send Kafka message after successful task creation
            let kafka_payload = serde_json::json!({
                "task_id": task.id,
                "file_name": task.file_name,
                "status": task.status
            });

            if let Err(e) = app_state.kafka_client
                .produce_event("file-embedding-tasks", "task_created", kafka_payload)
                .await
            {
                tracing::error!("Failed to send Kafka message: {}", e);
                // Continue anyway - don't fail the API call if Kafka is down
            } else {
                tracing::info!("Sent Kafka message for task creation: {}", task.id);
            }

            (StatusCode::CREATED, Json(task)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create task: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to create task"})),
            )
                .into_response()
        }
    }
}

pub async fn get_task(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match FileEmbeddingTask::find_by_id(&app_state.db_pool, id).await {
        Ok(Some(task)) => (StatusCode::OK, Json(task)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Task not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to get task: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to get task"})),
            )
                .into_response()
        }
    }
}

pub async fn list_tasks(
    State(app_state): State<AppState>,
    Query(params): Query<ListTasksQuery>,
) -> impl IntoResponse {
    match FileEmbeddingTask::list_all(&app_state.db_pool, params.status, params.limit, params.offset).await {
        Ok(tasks) => (StatusCode::OK, Json(tasks)).into_response(),
        Err(e) => {
            tracing::error!("Failed to list tasks: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to list tasks"})),
            )
                .into_response()
        }
    }
}

pub async fn update_task(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateTaskRequest>,
) -> impl IntoResponse {
    match FileEmbeddingTask::update(&app_state.db_pool, id, payload).await {
        Ok(Some(task)) => (StatusCode::OK, Json(task)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Task not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to update task: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to update task"})),
            )
                .into_response()
        }
    }
}

pub async fn delete_task(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match FileEmbeddingTask::delete(&app_state.db_pool, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Task not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to delete task: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to delete task"})),
            )
                .into_response()
        }
    }
}