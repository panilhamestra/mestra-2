use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use axum::http::StatusCode;
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{Local, NaiveDateTime};
use reqwest::cookie::{CookieStore, Jar};
use serde::Deserialize;
use serde_json::Value;

use crate::models::{ArquivoUpload, Discipline, DisciplineItem, Enterprise, NovoItemInput, SessionToken, StoredToken};
use crate::state::AppState;
use crate::utils;

const TOKEN_FILE_PATH: &str = "data/token.json";
const EXPIRATION_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

// ConstruCode (páginas ASP.NET legadas) responde em Windows-1252, mas o
// reqwest tá sem a feature "charset" — sem isso, .text() faz UTF-8 lossy e
// todo acento vira caractere de substituição/some. Decodifica manual.
fn decode_windows_1252(bytes: &[u8]) -> String {
    encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned()
}

// Mesma razão do decode acima, na direção oposta: o ConstruCode lê o body
// de x-www-form-urlencoded como Windows-1252, mas reqwest::form() serializa
// em UTF-8. Sem isso, "Ç" (UTF-8 C3 87) chega no servidor e vira "Ã‡"
// (cada byte UTF-8 relido como um char Windows-1252). Monta o body na mão,
// codificando cada campo em Windows-1252 antes do percent-encoding.
fn encode_form_windows_1252(params: &[(String, String)]) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode_windows_1252(k), percent_encode_windows_1252(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode_windows_1252(s: &str) -> String {
    let (bytes, _, _) = encoding_rs::WINDOWS_1252.encode(s);
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes.iter() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[derive(Debug)]
pub enum ServiceError {
    Request(reqwest::Error),
    VerificationTokenMissing,
    InvalidCredentials,
    SessionCookieMissing,
    SessionCookieInvalid,
    Persist(std::io::Error),
    ProjetoPageInvalid,
    DisciplinaNotFound(String),
    UpstreamParse(String),
    UploadFailed(String),
    CreateFailed(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::Request(e) => write!(f, "erro de rede falando com o ConstruCode: {e}"),
            ServiceError::VerificationTokenMissing => {
                write!(f, "não achei __RequestVerificationToken na página de login")
            }
            ServiceError::InvalidCredentials => {
                write!(f, "login falhou — confira CONSTRUCODE_EMAIL/CONSTRUCODE_PASSWORD no .env")
            }
            ServiceError::SessionCookieMissing => {
                write!(f, "login OK, mas não achei o cookie __session")
            }
            ServiceError::SessionCookieInvalid => {
                write!(f, "cookie __session veio num formato inesperado")
            }
            ServiceError::Persist(e) => write!(f, "erro gravando o token em disco: {e}"),
            ServiceError::ProjetoPageInvalid => {
                write!(f, "não consegui ler a página do empreendimento no ConstruCode")
            }
            ServiceError::DisciplinaNotFound(sigla) => {
                write!(f, "disciplina '{sigla}' não encontrada nesse empreendimento")
            }
            ServiceError::UpstreamParse(msg) => write!(f, "resposta inesperada do ConstruCode: {msg}"),
            ServiceError::UploadFailed(msg) => write!(f, "falha no upload do arquivo no ConstruCode: {msg}"),
            ServiceError::CreateFailed(msg) => write!(f, "falha ao cadastrar o item no ConstruCode: {msg}"),
        }
    }
}

impl From<reqwest::Error> for ServiceError {
    fn from(e: reqwest::Error) -> Self {
        ServiceError::Request(e)
    }
}

impl ServiceError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ServiceError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            ServiceError::DisciplinaNotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::BAD_GATEWAY,
        }
    }
}

// ===================== Login =====================

// Chegada: pega o __RequestVerificationToken da página de login
async fn fetch_verification_token(state: &AppState) -> Result<String, ServiceError> {
    let html = state
        .http_client
        .get(&state.login_url)
        .send()
        .await?
        .text()
        .await?;

    extract_verification_token(&html)
}

fn extract_verification_token(html: &str) -> Result<String, ServiceError> {
    let marker = "name=\"__RequestVerificationToken\"";
    let marker_pos = html.find(marker).ok_or(ServiceError::VerificationTokenMissing)?;
    let tag_start = html[..marker_pos]
        .rfind('<')
        .ok_or(ServiceError::VerificationTokenMissing)?;
    let tag_end = marker_pos
        + html[marker_pos..]
            .find('>')
            .ok_or(ServiceError::VerificationTokenMissing)?;
    let tag = &html[tag_start..tag_end];

    let value_marker = "value=\"";
    let value_start =
        tag.find(value_marker).ok_or(ServiceError::VerificationTokenMissing)? + value_marker.len();
    let value_end = value_start
        + tag[value_start..]
            .find('"')
            .ok_or(ServiceError::VerificationTokenMissing)?;

    Ok(tag[value_start..value_end].to_string())
}

// Operação: autentica e decodifica o cookie de sessão
async fn authenticate(state: &AppState, verification_token: &str) -> Result<SessionToken, ServiceError> {
    let response = state
        .http_client
        .post(&state.login_url)
        .form(&[
            ("__RequestVerificationToken", verification_token),
            ("email", state.credentials.email.as_str()),
            ("password", state.credentials.password.as_str()),
        ])
        .send()
        .await?;

    if response.url().as_str().trim_end_matches('/').ends_with("/Account/Login") {
        return Err(ServiceError::InvalidCredentials);
    }

    extract_session_token(&state.cookie_jar, &state.web_base_url)
}

fn extract_session_token(jar: &Arc<Jar>, web_base_url: &str) -> Result<SessionToken, ServiceError> {
    let web_url: reqwest::Url = web_base_url
        .parse()
        .map_err(|_| ServiceError::SessionCookieMissing)?;

    let cookie_header = jar.cookies(&web_url).ok_or(ServiceError::SessionCookieMissing)?;
    let cookie_str = cookie_header
        .to_str()
        .map_err(|_| ServiceError::SessionCookieMissing)?;

    let raw_session = cookie_str
        .split(';')
        .map(str::trim)
        .find_map(|kv| kv.strip_prefix("__session="))
        .ok_or(ServiceError::SessionCookieMissing)?;

    decode_session_cookie(raw_session)
}

fn decode_session_cookie(raw: &str) -> Result<SessionToken, ServiceError> {
    let decoded = percent_decode(raw);
    let payload_b64 = decoded.split('.').next().unwrap_or(&decoded);
    let padded = pad_base64(payload_b64);

    let bytes = STANDARD
        .decode(padded)
        .map_err(|_| ServiceError::SessionCookieInvalid)?;

    serde_json::from_slice(&bytes).map_err(|_| ServiceError::SessionCookieInvalid)
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn pad_base64(s: &str) -> String {
    let remainder = s.len() % 4;
    if remainder == 0 {
        s.to_string()
    } else {
        format!("{s}{}", "=".repeat(4 - remainder))
    }
}

// Partida: grava token + data de expiração em data/token.json
async fn persist_token(session_token: &SessionToken) -> Result<(), ServiceError> {
    let stored = StoredToken {
        token: session_token.token.clone(),
        expiration_date: session_token.expiration_date.clone(),
    };
    let json = serde_json::to_string_pretty(&stored).expect("StoredToken sempre serializa");

    tokio::fs::create_dir_all("data").await.map_err(ServiceError::Persist)?;
    tokio::fs::write(TOKEN_FILE_PATH, json)
        .await
        .map_err(ServiceError::Persist)?;

    Ok(())
}

// Ciclo completo: Chegada -> Operação -> Partida
async fn login_and_persist_token(state: &AppState) -> Result<StoredToken, ServiceError> {
    let verification_token = fetch_verification_token(state).await?;
    let session_token = authenticate(state, &verification_token).await?;
    persist_token(&session_token).await?;

    Ok(StoredToken {
        token: session_token.token,
        expiration_date: session_token.expiration_date,
    })
}

// ===================== Validação/renovação do token =====================

// Lê data/token.json e devolve o token só se existir e ainda não tiver
// expirado. Qualquer problema (arquivo ausente, corrompido, expirado) vira
// None — quem chama decide o que fazer (aqui: logar de novo).
async fn read_valid_stored_token() -> Option<StoredToken> {
    let bytes = tokio::fs::read(TOKEN_FILE_PATH).await.ok()?;
    let stored: StoredToken = serde_json::from_slice(&bytes).ok()?;
    let expiration = NaiveDateTime::parse_from_str(&stored.expiration_date, EXPIRATION_FORMAT).ok()?;

    // expirationDate do ConstruCode não tem timezone — assume o mesmo
    // horário local do servidor que gerou o token (Brasil).
    if Local::now().naive_local() >= expiration {
        return None;
    }

    Some(stored)
}

// Portão de entrada de todos os endpoints que dependem de sessão: usa o
// token salvo se ainda for válido, senão loga de novo no ConstruCode e
// grava o novo token — quem chamou nunca precisa se preocupar com login.
pub async fn ensure_valid_token(state: &AppState) -> Result<StoredToken, ServiceError> {
    if let Some(stored) = read_valid_stored_token().await {
        return Ok(stored);
    }

    login_and_persist_token(state).await
}

// ===================== Empreendimentos =====================

pub async fn fetch_enterprises(state: &AppState) -> Result<Vec<Enterprise>, ServiceError> {
    let text = state
        .http_client
        .get(&state.enterprises_url)
        .header("Accept", "*/*")
        .header("Referer", format!("{}/Enterprises", state.web_base_url))
        .send()
        .await?
        .text()
        .await?;

    let decoded = utils::turbo_stream_decode(&text);

    let page_data = decoded
        .get("routes/_App.Enterprises")
        .and_then(|v| v.get("data"))
        .and_then(|v| v.get("pageData"))
        .cloned()
        .unwrap_or(Value::Null);

    let my_enterprises: Vec<Enterprise> = page_data
        .get("myEnterprises")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| ServiceError::UpstreamParse(e.to_string()))?
        .unwrap_or_default();

    let shared_enterprises: Vec<Enterprise> = page_data
        .get("sharedEnterprises")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| ServiceError::UpstreamParse(e.to_string()))?
        .unwrap_or_default();

    let mut enterprises = my_enterprises;
    enterprises.extend(shared_enterprises);
    Ok(enterprises)
}

// ===================== Disciplinas / Itens =====================

struct ProjetoPage {
    id_user: String,
    disciplinas: Vec<Discipline>,
}

async fn fetch_projeto_page(state: &AppState, id_obra: u32) -> Result<ProjetoPage, ServiceError> {
    let html = state
        .http_client
        .get(&state.projeto_url)
        .query(&[("id", id_obra.to_string())])
        .header("Referer", format!("{}/", state.web_base_url))
        .send()
        .await?
        .bytes()
        .await?;
    let html = decode_windows_1252(&html);

    Ok(ProjetoPage {
        id_user: extract_id_user(&html)?,
        disciplinas: extract_disciplinas(&html)?,
    })
}

fn extract_id_user(html: &str) -> Result<String, ServiceError> {
    let marker = "const idUser = '";
    let start = html.find(marker).ok_or(ServiceError::ProjetoPageInvalid)? + marker.len();
    let end = start + html[start..].find('\'').ok_or(ServiceError::ProjetoPageInvalid)?;
    Ok(html[start..end].to_string())
}

fn extract_disciplinas(html: &str) -> Result<Vec<Discipline>, ServiceError> {
    let marker_pos = html
        .find("id=\"idDisciplina\"")
        .ok_or(ServiceError::ProjetoPageInvalid)?;
    let select_start = html[..marker_pos]
        .rfind("<select")
        .ok_or(ServiceError::ProjetoPageInvalid)?;
    let select_end = select_start
        + html[select_start..]
            .find("</select>")
            .ok_or(ServiceError::ProjetoPageInvalid)?;
    let select_block = &html[select_start..select_end];

    let mut disciplinas = Vec::new();
    let mut rest = select_block;
    let option_marker = "<option value=\"";

    while let Some(opt_pos) = rest.find(option_marker) {
        let after_marker = &rest[opt_pos + option_marker.len()..];
        let Some(value_end) = after_marker.find('"') else { break };
        let value_str = &after_marker[..value_end];

        let after_value = &after_marker[value_end + 1..];
        let Some(gt_pos) = after_value.find('>') else { break };
        let after_gt = &after_value[gt_pos + 1..];
        let Some(lt_pos) = after_gt.find('<') else { break };
        let text = &after_gt[..lt_pos];

        rest = &after_gt[lt_pos..];

        if value_str != "-1" {
            if let Ok(id) = value_str.parse::<u32>() {
                let (sigla, name) = split_sigla_name(&decode_html_entities(text));
                disciplinas.push(Discipline { id, sigla, name });
            }
        }
    }

    Ok(disciplinas)
}

// Texto da option vem como "(EST) Estrutura" — separa sigla e nome.
fn split_sigla_name(text: &str) -> (String, String) {
    if let Some(rest) = text.strip_prefix('(') {
        if let Some(close) = rest.find(')') {
            let sigla = rest[..close].to_string();
            let name = rest[close + 1..].trim().to_string();
            return (sigla, name);
        }
    }
    (String::new(), text.to_string())
}

fn decode_html_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input.as_bytes()[i] == b'&' {
            if let Some(semi_rel) = input[i..].find(';') {
                let entity = &input[i + 1..i + semi_rel];
                let decoded_char = if let Some(hex) = entity.strip_prefix("#x").or_else(|| entity.strip_prefix("#X")) {
                    u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
                } else if let Some(dec) = entity.strip_prefix('#') {
                    dec.parse::<u32>().ok().and_then(char::from_u32)
                } else {
                    match entity {
                        "amp" => Some('&'),
                        "lt" => Some('<'),
                        "gt" => Some('>'),
                        "quot" => Some('"'),
                        "apos" => Some('\''),
                        _ => None,
                    }
                };
                if let Some(c) = decoded_char {
                    out.push(c);
                    i += semi_rel + 1;
                    continue;
                }
            }
        }
        let ch_len = input[i..].chars().next().map(char::len_utf8).unwrap_or(1);
        out.push_str(&input[i..i + ch_len]);
        i += ch_len;
    }
    out
}

pub async fn fetch_disciplinas(state: &AppState, id_obra: u32) -> Result<Vec<Discipline>, ServiceError> {
    Ok(fetch_projeto_page(state, id_obra).await?.disciplinas)
}

#[derive(Debug, Deserialize)]
struct PlantasByAreaResponse {
    #[serde(rename = "Success")]
    success: bool,
    #[serde(rename = "JsonObject")]
    json_object: Option<PlantasJsonObject>,
}

#[derive(Debug, Deserialize)]
struct PlantasJsonObject {
    #[serde(rename = "Documentos")]
    documentos: Option<Vec<DisciplineItem>>,
}

pub async fn fetch_itens_disciplina(
    state: &AppState,
    id_obra: u32,
    sigla: &str,
) -> Result<Vec<DisciplineItem>, ServiceError> {
    let page = fetch_projeto_page(state, id_obra).await?;

    let disciplina = page
        .disciplinas
        .iter()
        .find(|d| d.sigla.eq_ignore_ascii_case(sigla))
        .ok_or_else(|| ServiceError::DisciplinaNotFound(sigla.to_string()))?;

    let id_obra_str = id_obra.to_string();
    let disciplina_id_str = disciplina.id.to_string();

    let params = [
        ("idArea", "-1"),
        ("idObra", id_obra_str.as_str()),
        ("ordenacao", "4"),
        ("hdnExtensoes", ""),
        ("U", page.id_user.as_str()),
        ("exibirDocsSemArea", "false"),
        ("exibirTodosOsDocumentos", "false"),
        ("planta", ""),
        ("exibirAreas", "false"),
        ("idDisciplina", disciplina_id_str.as_str()),
        ("disciplinaID", disciplina_id_str.as_str()),
        ("faseID", ""),
        ("dt", ""),
        ("dtF", ""),
        ("filtroSituacao", ""),
        ("statusID", ""),
    ];

    let bytes = state
        .http_client
        .post(&state.plantas_url)
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Referer", format!("{}/Projetos/Index?id={id_obra}", state.base_url))
        .form(&params)
        .send()
        .await?
        .bytes()
        .await?;
    let text = decode_windows_1252(&bytes);

    let parsed: PlantasByAreaResponse =
        serde_json::from_str(&text).map_err(|e| ServiceError::UpstreamParse(e.to_string()))?;

    if !parsed.success {
        return Err(ServiceError::UpstreamParse(
            "PostPlantasByArea retornou Success=false".to_string(),
        ));
    }

    Ok(parsed.json_object.and_then(|obj| obj.documentos).unwrap_or_default())
}

// ===================== Criação de item =====================

#[derive(Debug, Deserialize)]
struct UploadMultipleResponse {
    #[serde(rename = "Success")]
    success: bool,
    #[serde(rename = "Projetos", default)]
    projetos: HashMap<String, UploadedProjeto>,
}

#[derive(Debug, Deserialize)]
struct UploadedProjeto {
    #[serde(rename = "FileName")]
    file_name: String,
    #[serde(rename = "Success")]
    success: bool,
    #[serde(rename = "Message")]
    message: Option<String>,
    // Revisao vem pronta do ConstruCode; Descricao (nome do item) já chega
    // preenchida em NovoItemInput::nome, não é derivada daqui.
    #[serde(rename = "Revisao", default)]
    revisao: String,
}

#[derive(Debug, Deserialize)]
struct BatchCreateResponse {
    #[serde(rename = "Success")]
    success: bool,
    #[serde(rename = "Message")]
    message: Option<String>,
}

struct ArquivoResolvido<'a> {
    arquivo: &'a ArquivoUpload,
    file_name: String,
    revisao: String,
    descricao: String,
}

// Sobe todos os arquivos de uma vez (mesmo request multipart do ConstruCode,
// campos file[0], file[1]... — é o que o uploader Dropzone da tela usa com
// uploadMultiple:true). Devolve FileName/Revisao por OriginalFileName.
async fn upload_files(
    state: &AppState,
    id_obra: u32,
    arquivos: &[ArquivoUpload],
) -> Result<HashMap<String, UploadedProjeto>, ServiceError> {
    let mut form = reqwest::multipart::Form::new();
    for (i, arquivo) in arquivos.iter().enumerate() {
        let part = reqwest::multipart::Part::bytes(arquivo.bytes.clone()).file_name(arquivo.original_file_name.clone());
        form = form.part(format!("file[{i}]"), part);
    }

    let bytes = state
        .http_client
        .post(&state.upload_url)
        .query(&[("idObra", id_obra.to_string())])
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Referer", format!("{}/Plantas/BatchCreate?idObra={id_obra}", state.base_url))
        .multipart(form)
        .send()
        .await?
        .bytes()
        .await?;
    let text = decode_windows_1252(&bytes);

    let parsed: UploadMultipleResponse =
        serde_json::from_str(&text).map_err(|e| ServiceError::UpstreamParse(e.to_string()))?;

    if !parsed.success {
        return Err(ServiceError::UploadFailed("UploadMultiple retornou Success=false".to_string()));
    }

    for (original_name, projeto) in &parsed.projetos {
        if !projeto.success {
            let msg = projeto.message.clone().unwrap_or_default();
            return Err(ServiceError::UploadFailed(format!("'{original_name}': {msg}")));
        }
    }

    Ok(parsed.projetos)
}

// Cadastra os itens já enviados (upload_files) — id_tipo é o id da
// disciplina resolvido em criar_item_disciplina (mesmo namespace de
// idDisciplina usado no GET: confirmado comparando os <select> das páginas
// Projetos/Index e Plantas/BatchCreate, valores idênticos pra mesma sigla).
async fn batch_create(
    state: &AppState,
    id_obra: u32,
    id_area: i64,
    id_tipo: u32,
    input: &NovoItemInput,
    resolvidos: &[ArquivoResolvido<'_>],
) -> Result<(), ServiceError> {
    let mut params: Vec<(String, String)> = vec![
        ("idObra".to_string(), id_obra.to_string()),
        ("idArea".to_string(), id_area.to_string()),
    ];

    for (i, item) in resolvidos.iter().enumerate() {
        let prefix = format!("Plantas[{i}]");
        params.push((format!("{prefix}.OriginalFileName"), item.arquivo.original_file_name.clone()));
        params.push((format!("{prefix}.FileName"), item.file_name.clone()));
        params.push((format!("{prefix}.ID"), "-1".to_string()));
        params.push((format!("{prefix}.VincularComoHistorico"), input.vincular_como_historico.to_string()));
        params.push((format!("{prefix}.filesize"), item.arquivo.bytes.len().to_string()));
        params.push((format!("{prefix}.selectAreas"), String::new()));
        params.push((format!("{prefix}.Formato"), input.formato.clone()));
        params.push((format!("{prefix}.IDTipo"), id_tipo.to_string()));
        params.push((format!("{prefix}.Descricao"), item.descricao.clone()));
        params.push((format!("{prefix}.Revisao"), item.revisao.clone()));
        params.push((format!("{prefix}.Prancha"), input.prancha.clone().unwrap_or_default()));
        params.push((format!("{prefix}.Detalhamento"), input.detalhamento.clone().unwrap_or_default()));
        params.push((format!("{prefix}.Obs"), input.obs.clone().unwrap_or_default()));
        params.push((format!("{prefix}.Fase"), input.fase.to_string()));
        params.push((format!("{prefix}.Liberado"), input.liberado.to_string()));
    }

    let bytes = state
        .http_client
        .post(&state.batch_create_url)
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Referer", format!("{}/Plantas/BatchCreate?idObra={id_obra}", state.base_url))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(encode_form_windows_1252(&params))
        .send()
        .await?
        .bytes()
        .await?;
    let text = decode_windows_1252(&bytes);

    let parsed: BatchCreateResponse =
        serde_json::from_str(&text).map_err(|e| ServiceError::UpstreamParse(e.to_string()))?;

    if !parsed.success {
        return Err(ServiceError::CreateFailed(
            parsed.message.unwrap_or_else(|| "BatchCreate retornou Success=false".to_string()),
        ));
    }

    Ok(())
}

// Ciclo completo: resolve a disciplina (sigla -> id, mesmo id usado como
// IDTipo no BatchCreate) -> sobe os arquivos -> cadastra o item.
pub async fn criar_item_disciplina(
    state: &AppState,
    id_obra: u32,
    sigla: &str,
    input: NovoItemInput,
) -> Result<(), ServiceError> {
    let page = fetch_projeto_page(state, id_obra).await?;
    let disciplina = page
        .disciplinas
        .iter()
        .find(|d| d.sigla.eq_ignore_ascii_case(sigla))
        .ok_or_else(|| ServiceError::DisciplinaNotFound(sigla.to_string()))?;

    let uploads = upload_files(state, id_obra, &input.arquivos).await?;

    let mut resolvidos = Vec::with_capacity(input.arquivos.len());
    for arquivo in &input.arquivos {
        let projeto = uploads.get(&arquivo.original_file_name).ok_or_else(|| {
            ServiceError::UploadFailed(format!("upload não devolveu dados para '{}'", arquivo.original_file_name))
        })?;
        resolvidos.push(ArquivoResolvido {
            arquivo,
            file_name: projeto.file_name.clone(),
            revisao: projeto.revisao.clone(),
            descricao: input.nome.clone(),
        });
    }

    batch_create(state, id_obra, input.id_area, disciplina.id, &input, &resolvidos).await
}
