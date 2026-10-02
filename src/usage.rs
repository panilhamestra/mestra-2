use std::{sync::Arc, time::Instant};

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;

use crate::state::AppState;

// Usage tracking (telemetry) sent to HubCetec — same values as the Office
// Script "Lista Mestra - OrganizadorTabajara".
const USAGE_API_URL: &str = "https://hubcetec.com/api/v1/plugins/use/create/";
const USAGE_TITLE: &str = "Lista Mestra - OrganizadorTabajara - Hackathon26";
const SCRIPT_VERSION: &str = "1.0";
const USAGE_PARAMETER: u32 = 1;
const USAGE_FILE: &str = "mestra-2";

// JSON field names are the HubCetec contract, so they keep their original names.
#[derive(Serialize)]
struct UsageRecord {
    #[serde(rename = "Titulo")]
    title: &'static str,
    #[serde(rename = "DataHorario")]
    timestamp: String,
    #[serde(rename = "Parametro")]
    parameter: u32,
    #[serde(rename = "Caminho")]
    path: String,
    #[serde(rename = "Colaborador")]
    user: String,
    #[serde(rename = "Arquivo")]
    file: &'static str,
    #[serde(rename = "Tempo")]
    elapsed_secs: u64,
    version: &'static str,
    is_active: bool,
}

// Records the usage of every endpoint that passes through here. The user is
// the email sent in the X-User-Email header (required). Sending runs in the
// background and failures are only logged — they never affect the response
// to the client.
pub async fn track_usage(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let email = req
        .headers()
        .get("X-User-Email")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(str::to_owned);

    let Some(email) = email else {
        return (StatusCode::BAD_REQUEST, "email ausente (header X-User-Email)").into_response();
    };

    let start = Instant::now();
    let route = format!("{} {}", req.method(), req.uri().path());

    let response = next.run(req).await;

    let record = UsageRecord {
        title: USAGE_TITLE,
        timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        parameter: USAGE_PARAMETER,
        path: format!("https://ceteceng.sharepoint.com/ | {USAGE_FILE} | {route}"),
        user: email,
        file: USAGE_FILE,
        elapsed_secs: start.elapsed().as_secs(),
        version: SCRIPT_VERSION,
        is_active: true,
    };

    tokio::spawn(async move {
        match state.http_client.post(USAGE_API_URL).json(&record).send().await {
            Ok(r) if !r.status().is_success() => tracing::warn!("Usage tracking failed: {}", r.status()),
            Ok(_) => {}
            Err(e) => tracing::warn!("Usage tracking failed: {e}"),
        }
    });

    response
}
