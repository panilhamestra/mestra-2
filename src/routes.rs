use axum::{
    extract::{Path, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::models::{Discipline, DisciplineItem, Enterprise};
use crate::service::{ensure_valid_token, fetch_disciplinas, fetch_enterprises, fetch_itens_disciplina, ServiceError};
use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(
        empreendimentos_handler,
        disciplinas_handler,
        itens_disciplina_handler,
        health_handler
    ),
    components(schemas(Enterprise, Discipline, DisciplineItem)),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

// Registra o esquema "api_key" (header X-Api-Token) pro botão Authorize
// aparecer no Swagger UI.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Api-Token"))),
            );
        }
    }
}

pub fn build_router(state: Arc<AppState>) -> Router {
    // Rotas de dados: exigem o header X-Api-Token batendo com API_ACCESS_TOKEN.
    let protected = Router::new()
        .route("/empreendimentos", get(empreendimentos_handler))
        .route("/empreendimentos/:id/disciplinas", get(disciplinas_handler))
        .route(
            "/empreendimentos/:id/disciplinas/:sigla/itens",
            get(itens_disciplina_handler),
        )
        .route_layer(middleware::from_fn_with_state(Arc::clone(&state), require_api_token));

    Router::new()
        .merge(protected)
        .route("/health", get(health_handler))
        .merge(SwaggerUi::new("/").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state)
}

// Confere o header X-Api-Token contra API_ACCESS_TOKEN (.env) em toda rota
// protegida. Não é sessão/cookie — cada request prova o token de novo.
async fn require_api_token(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let token_ok = req
        .headers()
        .get("X-Api-Token")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|t| t == state.api_token);

    if !token_ok {
        return (StatusCode::UNAUTHORIZED, "token de acesso inválido ou ausente (header X-Api-Token)")
            .into_response();
    }

    next.run(req).await
}

fn map_service_error(e: ServiceError) -> (StatusCode, String) {
    (e.status_code(), e.to_string())
}

#[utoipa::path(
    get,
    path = "/empreendimentos",
    security(("api_key" = [])),
    responses(
        (status = 200, description = "Lista de empreendimentos do usuário logado", body = Vec<Enterprise>),
        (status = 401, description = "X-Api-Token ausente/inválido"),
        (status = 502, description = "Falha ao buscar empreendimentos no ConstruCode")
    )
)]
async fn empreendimentos_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Enterprise>>, (StatusCode, String)> {
    ensure_valid_token(&state).await.map_err(map_service_error)?;

    let enterprises = fetch_enterprises(&state).await.map_err(map_service_error)?;
    Ok(Json(enterprises))
}

#[utoipa::path(
    get,
    path = "/empreendimentos/{id}/disciplinas",
    params(("id" = u32, Path, description = "Id do empreendimento")),
    security(("api_key" = [])),
    responses(
        (status = 200, description = "Disciplinas do empreendimento", body = Vec<Discipline>),
        (status = 401, description = "X-Api-Token ausente/inválido"),
        (status = 502, description = "Falha ao buscar disciplinas no ConstruCode")
    )
)]
async fn disciplinas_handler(
    State(state): State<Arc<AppState>>,
    Path(id_obra): Path<u32>,
) -> Result<Json<Vec<Discipline>>, (StatusCode, String)> {
    ensure_valid_token(&state).await.map_err(map_service_error)?;

    let disciplinas = fetch_disciplinas(&state, id_obra).await.map_err(map_service_error)?;
    Ok(Json(disciplinas))
}

#[utoipa::path(
    get,
    path = "/empreendimentos/{id}/disciplinas/{sigla}/itens",
    params(
        ("id" = u32, Path, description = "Id do empreendimento"),
        ("sigla" = String, Path, description = "Sigla da disciplina, ex: EST")
    ),
    security(("api_key" = [])),
    responses(
        (status = 200, description = "Itens (documentos) da disciplina", body = Vec<DisciplineItem>),
        (status = 401, description = "X-Api-Token ausente/inválido"),
        (status = 404, description = "Disciplina não encontrada nesse empreendimento"),
        (status = 502, description = "Falha ao buscar itens no ConstruCode")
    )
)]
async fn itens_disciplina_handler(
    State(state): State<Arc<AppState>>,
    Path((id_obra, sigla)): Path<(u32, String)>,
) -> Result<Json<Vec<DisciplineItem>>, (StatusCode, String)> {
    ensure_valid_token(&state).await.map_err(map_service_error)?;

    let itens = fetch_itens_disciplina(&state, id_obra, &sigla)
        .await
        .map_err(map_service_error)?;
    Ok(Json(itens))
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
