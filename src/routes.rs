use axum::{
    extract::{Multipart, Path, Request, State},
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

use crate::models::{ArquivoUpload, Discipline, DisciplineItem, Enterprise, NovoItemInput, NovoItemMultipart};
use crate::service::{
    criar_item_disciplina, ensure_valid_token, fetch_disciplinas, fetch_enterprises, fetch_itens_disciplina, ServiceError,
};
use crate::state::AppState;
use crate::usage::track_usage;

#[derive(OpenApi)]
#[openapi(
    paths(
        empreendimentos_handler,
        disciplinas_handler,
        itens_disciplina_handler,
        criar_item_handler,
        health_handler
    ),
    components(schemas(Enterprise, Discipline, DisciplineItem, NovoItemMultipart)),
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
            components.add_security_scheme(
                "user_email",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-User-Email"))),
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
            get(itens_disciplina_handler).post(criar_item_handler),
        )
        // Inner layer: only requests with a valid token reach it and get tracked.
        .route_layer(middleware::from_fn_with_state(Arc::clone(&state), track_usage))
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
    security(("api_key" = [], "user_email" = [])),
    responses(
        (status = 200, description = "Lista de empreendimentos do usuário logado", body = Vec<Enterprise>),
        (status = 400, description = "X-User-Email ausente"),
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
    security(("api_key" = [], "user_email" = [])),
    responses(
        (status = 200, description = "Disciplinas do empreendimento", body = Vec<Discipline>),
        (status = 400, description = "X-User-Email ausente"),
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
    security(("api_key" = [], "user_email" = [])),
    responses(
        (status = 200, description = "Itens (documentos) da disciplina", body = Vec<DisciplineItem>),
        (status = 400, description = "X-User-Email ausente"),
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
    post,
    path = "/empreendimentos/{id}/disciplinas/{sigla}/itens",
    params(
        ("id" = u32, Path, description = "Id do empreendimento"),
        ("sigla" = String, Path, description = "Sigla da disciplina, ex: EST")
    ),
    request_body(content = NovoItemMultipart, content_type = "multipart/form-data"),
    security(("api_key" = [], "user_email" = [])),
    responses(
        (status = 201, description = "Item(ns) cadastrado(s) com sucesso"),
        (status = 400, description = "Campos obrigatórios ausentes, multipart inválido ou nenhum arquivo enviado"),
        (status = 400, description = "X-User-Email ausente"),
        (status = 401, description = "X-Api-Token ausente/inválido"),
        (status = 404, description = "Disciplina não encontrada nesse empreendimento"),
        (status = 502, description = "Falha ao subir arquivo ou cadastrar item no ConstruCode")
    )
)]
async fn criar_item_handler(
    State(state): State<Arc<AppState>>,
    Path((id_obra, sigla)): Path<(u32, String)>,
    multipart: Multipart,
) -> Result<StatusCode, (StatusCode, String)> {
    let input = parse_novo_item_multipart(multipart).await?;

    ensure_valid_token(&state).await.map_err(map_service_error)?;

    criar_item_disciplina(&state, id_obra, &sigla, input)
        .await
        .map_err(map_service_error)?;

    Ok(StatusCode::CREATED)
}

fn multipart_err(e: axum::extract::multipart::MultipartError) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, format!("multipart inválido: {e}"))
}

// Lê o form multipart campo a campo. id_area/formato/vincular_como_historico
// têm default quando ausentes; fase/liberado/arquivos são obrigatórios.
async fn parse_novo_item_multipart(mut multipart: Multipart) -> Result<NovoItemInput, (StatusCode, String)> {
    let mut id_area: i64 = -1;
    let mut nome = None;
    let mut detalhamento = None;
    let mut obs = None;
    let mut prancha = None;
    let mut fase: Option<u32> = None;
    let mut liberado: Option<u32> = None;
    let mut formato = "Original".to_string();
    let mut vincular_como_historico = false;
    let mut arquivos = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(multipart_err)? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "id_area" => {
                let text = field.text().await.map_err(multipart_err)?;
                id_area = text
                    .parse()
                    .map_err(|_| (StatusCode::BAD_REQUEST, "id_area precisa ser um número".to_string()))?;
            }
            "nome" => nome = Some(field.text().await.map_err(multipart_err)?),
            "detalhamento" => detalhamento = Some(field.text().await.map_err(multipart_err)?),
            "obs" => obs = Some(field.text().await.map_err(multipart_err)?),
            "prancha" => prancha = Some(field.text().await.map_err(multipart_err)?),
            "fase" => {
                let text = field.text().await.map_err(multipart_err)?;
                fase = Some(
                    text.parse()
                        .map_err(|_| (StatusCode::BAD_REQUEST, "fase precisa ser um número".to_string()))?,
                );
            }
            "liberado" => {
                let text = field.text().await.map_err(multipart_err)?;
                liberado = Some(
                    text.parse()
                        .map_err(|_| (StatusCode::BAD_REQUEST, "liberado precisa ser um número".to_string()))?,
                );
            }
            "formato" => formato = field.text().await.map_err(multipart_err)?,
            "vincular_como_historico" => {
                let text = field.text().await.map_err(multipart_err)?;
                vincular_como_historico = text.eq_ignore_ascii_case("true") || text == "1";
            }
            "arquivos" => {
                let original_file_name = field.file_name().unwrap_or("arquivo").to_string();
                let bytes = field.bytes().await.map_err(multipart_err)?;
                arquivos.push(ArquivoUpload {
                    original_file_name,
                    bytes: bytes.to_vec(),
                });
            }
            _ => {}
        }
    }

    let nome = nome.ok_or((StatusCode::BAD_REQUEST, "campo 'nome' é obrigatório".to_string()))?;
    let fase = fase.ok_or((StatusCode::BAD_REQUEST, "campo 'fase' é obrigatório".to_string()))?;
    let liberado = liberado.ok_or((StatusCode::BAD_REQUEST, "campo 'liberado' é obrigatório".to_string()))?;

    if arquivos.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "envie ao menos 1 arquivo no campo 'arquivos'".to_string()));
    }

    Ok(NovoItemInput {
        id_area,
        nome,
        detalhamento,
        obs,
        prancha,
        fase,
        liberado,
        formato,
        vincular_como_historico,
        arquivos,
    })
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
