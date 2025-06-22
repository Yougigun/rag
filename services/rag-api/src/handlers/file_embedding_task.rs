use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use sqlx::{Pool, Postgres};

use crate::models::{CreateTaskRequest, FileEmbeddingTask, TaskStatus, UpdateTaskRequest};

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    pub status: Option<TaskStatus>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn create_task(
    State(pool): State<Pool<Postgres>>,
    Json(payload): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    match FileEmbeddingTask::create(&pool, payload).await {
        Ok(task) => (StatusCode::CREATED, Json(task)).into_response(),
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
    State(pool): State<Pool<Postgres>>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match FileEmbeddingTask::find_by_id(&pool, id).await {
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
    State(pool): State<Pool<Postgres>>,
    Query(params): Query<ListTasksQuery>,
) -> impl IntoResponse {
    match FileEmbeddingTask::list_all(&pool, params.status, params.limit, params.offset).await {
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
    State(pool): State<Pool<Postgres>>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateTaskRequest>,
) -> impl IntoResponse {
    match FileEmbeddingTask::update(&pool, id, payload).await {
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
    State(pool): State<Pool<Postgres>>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match FileEmbeddingTask::delete(&pool, id).await {
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