use axum::{extract::State, http::StatusCode, response::Json, routing::get, Router};
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::models::OrganizedItem;
use crate::service::fetch_and_organize;
use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(paths(get_items_handler, health_handler), components(schemas(OrganizedItem)))]
struct ApiDoc;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/dados", get(get_items_handler))
        .route("/health", get(health_handler))
        .merge(SwaggerUi::new("/").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/dados",
    responses(
        (status = 200, description = "Lista de itens organizados, com receita_estimada calculada", body = Vec<OrganizedItem>)
    )
)]
async fn get_items_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<OrganizedItem>>, (StatusCode, String)> {
    let items = fetch_and_organize(&state)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("erro buscando API: {e}")))?;

    Ok(Json(items))
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Servidor no ar", body = String)
    )
)]
async fn health_handler() -> &'static str {
    "ok"
}
