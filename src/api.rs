use crate::config::API_PORT;
use crate::db::Database;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
pub struct HealthCheckResponse {
    pub current_height: u64,
    pub current_root: String, // hex encoded
    pub timestamp: String,
    pub status: String,
}

pub struct AppState {
    pub db: Arc<Database>,
}

pub fn create_api_server(db: Arc<Database>) -> Router {
    let state = Arc::new(AppState { db });

    Router::new()
        .route("/health", get(get_health_check))
        .route("/", get(root))
        .with_state(state)
}

async fn root() -> &'static str {
    "Helios Proof Relayer API\nUse /health to get latest health check data"
}

async fn get_health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    tracing::info!("Received request for latest health check data");

    match state.db.get_latest_health_check() {
        Ok(Some(health_data)) => {
            let now = chrono::Utc::now();
            let threshold = now - chrono::Duration::minutes(30);
            let status = if health_data.timestamp > threshold {
                "healthy"
            } else {
                "unhealthy"
            };

            let response = HealthCheckResponse {
                current_height: health_data.current_height,
                current_root: hex::encode(&health_data.current_root),
                timestamp: health_data.timestamp.to_rfc3339(),
                status: status.to_string(),
            };
            tracing::info!(
                "Returning health check data: height={}, status={}",
                health_data.current_height,
                status
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(None) => {
            let response = HealthCheckResponse {
                current_height: 0,
                current_root: "".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                status: "no_data".to_string(),
            };
            tracing::info!("No health check data available");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get health check data: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn start_api_server(router: Router) -> Result<(), Box<dyn std::error::Error>> {
    // Get server port from environment or use default from config
    let port = std::env::var("API_PORT").unwrap_or_else(|_| API_PORT.to_string());
    let addr = format!("0.0.0.0:{}", port);

    // Parse the address properly
    let socket_addr: std::net::SocketAddr = addr.parse()?;

    let listener = tokio::net::TcpListener::bind(socket_addr).await?;
    tracing::info!("API server listening on http://{}", addr);
    tracing::info!("🌐 Server is externally reachable on port {}", port);

    axum::serve(listener, router).await?;
    Ok(())
}
