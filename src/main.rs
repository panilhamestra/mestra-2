mod models;
mod routes;
mod service;
mod state;

use std::sync::Arc;

use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState::from_env());
    let port = state.port;
    let app = routes::build_router(state);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("Falha ao abrir a porta!");

    tracing::info!("Servidor rodando na porta {port}.");

    axum::serve(listener, app).await.expect("Erro no servidor!");
}
