mod models;
mod routes;
mod service;
mod state;
mod utils;

use std::sync::Arc;

use tower_http::cors::CorsLayer;

use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState::from_env());
    let port = state.port;
    let app = routes::build_router(state).layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("Falha ao abrir a porta!");

    tracing::info!("Servidor rodando na porta {port}.");

    axum::serve(listener, app).await.expect("Erro no servidor!");
}
