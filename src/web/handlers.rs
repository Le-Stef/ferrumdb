//! HTTP handlers for the web interface

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

use crate::dispatch::Dispatcher;
use crate::cluster::ClusterManager;
use crate::protocol::RespValue;
use bytes::Bytes;
use sysinfo::System;

/// Shared application state
pub type AppState = Arc<Mutex<Dispatcher>>;

/// Request body for command execution
#[derive(Debug, Deserialize)]
pub struct CommandRequest {
    /// The command as a string, e.g., "SET key value"
    pub command: String,
}

/// Response for command execution
#[derive(Debug, Serialize)]
pub struct CommandResponse {
    /// Whether the command succeeded
    pub success: bool,
    /// The result or error message
    pub result: String,
}

/// System statistics response
#[derive(Debug, Serialize)]
pub struct SystemStats {
    /// Total system memory in MB
    pub total_memory_mb: f64,
    /// Used memory by the process in MB
    pub used_memory_mb: f64,
    /// Free system memory in MB
    pub free_memory_mb: f64,
    /// CPU usage percentage (0-100)
    pub cpu_usage: f64,
    /// Database memory usage in MB
    pub db_memory_mb: f64,
}

/// Home page handler - serves the HTML interface
pub async fn index_handler() -> impl IntoResponse {
    Html(include_str!("static/index.html"))
}

/// Execute a command
pub async fn execute_command(
    State(dispatcher): State<AppState>,
    Json(req): Json<CommandRequest>,
) -> impl IntoResponse {
    debug!("Executing command: {}", req.command);

    // Parse command string into parts and convert to RESP values
    let parts: Vec<RespValue> = req.command
        .trim()
        .split_whitespace()
        .map(|s| RespValue::BulkString(Bytes::from(s.to_string())))
        .collect();

    if parts.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(CommandResponse {
                success: false,
                result: "Empty command".to_string(),
            }),
        );
    }

    // Build RESP array command
    let command = RespValue::Array(parts);

    // Execute command
    let mut dispatcher = dispatcher.lock().await;
    let response = dispatcher.dispatch(command);

    // Convert response to string
    let result = format_resp_value(&response);

    (
        StatusCode::OK,
        Json(CommandResponse {
            success: !matches!(response, RespValue::Error(_)),
            result,
        }),
    )
}

/// Format a RESP value for display
fn format_resp_value(value: &RespValue) -> String {
    match value {
        RespValue::SimpleString(s) => s.clone(),
        RespValue::Error(e) => format!("Error: {}", e),
        RespValue::Integer(i) => i.to_string(),
        RespValue::BulkString(bytes) => {
            String::from_utf8_lossy(bytes).to_string()
        }
        RespValue::Array(arr) => {
            if arr.is_empty() {
                "(empty array)".to_string()
            } else {
                let items: Vec<String> = arr
                    .iter()
                    .enumerate()
                    .map(|(i, v)| format!("{}) {}", i + 1, format_resp_value(v)))
                    .collect();
                items.join("\n")
            }
        }
        RespValue::Null => "(nil)".to_string(),
    }
}

/// Get system statistics
pub async fn stats_handler(State(dispatcher): State<AppState>) -> impl IntoResponse {
    let mut sys = System::new_all();
    sys.refresh_all();

    // Get total and available memory
    let total_mem_bytes = sys.total_memory();
    let available_mem_bytes = sys.available_memory();
    let used_mem_bytes = total_mem_bytes - available_mem_bytes;

    // Convert to MB
    let total_memory_mb = total_mem_bytes as f64 / 1024.0 / 1024.0;
    let free_memory_mb = available_mem_bytes as f64 / 1024.0 / 1024.0;
    let used_memory_mb = used_mem_bytes as f64 / 1024.0 / 1024.0;

    // Get CPU usage (average across all cores)
    let cpu_usage = sys.global_cpu_usage() as f64;

    // Get database memory usage
    let dispatcher = dispatcher.lock().await;
    let store_stats = dispatcher.context().store.stats();
    let db_memory_mb = store_stats.used_memory_bytes as f64 / 1024.0 / 1024.0;

    let stats = SystemStats {
        total_memory_mb,
        used_memory_mb,
        free_memory_mb,
        cpu_usage,
        db_memory_mb,
    };

    (StatusCode::OK, Json(stats))
}

/// Execute command with cluster
pub async fn execute_command_cluster(
    State(cluster): State<Arc<ClusterManager>>,
    Json(req): Json<CommandRequest>,
) -> impl IntoResponse {
    debug!("Executing command on cluster: {}", req.command);

    let parts: Vec<RespValue> = req.command
        .trim()
        .split_whitespace()
        .map(|s| RespValue::BulkString(Bytes::from(s.to_string())))
        .collect();

    if parts.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(CommandResponse {
                success: false,
                result: "Empty command".to_string(),
            }),
        );
    }

    let command = RespValue::Array(parts);
    let response = cluster.execute(command).await;
    let result = format_resp_value(&response);

    (
        StatusCode::OK,
        Json(CommandResponse {
            success: !matches!(response, RespValue::Error(_)),
            result,
        }),
    )
}

/// Get stats with cluster
pub async fn stats_handler_cluster(State(cluster): State<Arc<ClusterManager>>) -> impl IntoResponse {
    let mut sys = System::new_all();
    sys.refresh_all();

    let total_mem_bytes = sys.total_memory();
    let available_mem_bytes = sys.available_memory();
    let used_mem_bytes = total_mem_bytes - available_mem_bytes;

    let total_memory_mb = total_mem_bytes as f64 / 1024.0 / 1024.0;
    let free_memory_mb = available_mem_bytes as f64 / 1024.0 / 1024.0;
    let used_memory_mb = used_mem_bytes as f64 / 1024.0 / 1024.0;
    let cpu_usage = sys.global_cpu_usage() as f64;

    let cluster_stats = cluster.get_cluster_stats().await;
    let db_memory_mb = cluster_stats.total_memory_bytes as f64 / 1024.0 / 1024.0;

    let stats = SystemStats {
        total_memory_mb,
        used_memory_mb,
        free_memory_mb,
        cpu_usage,
        db_memory_mb,
    };

    (StatusCode::OK, Json(stats))
}

/// Get detailed shard statistics
pub async fn shard_stats_handler(State(cluster): State<Arc<ClusterManager>>) -> impl IntoResponse {
    let shard_details = cluster.get_shard_details().await;
    (StatusCode::OK, Json(shard_details))
}
