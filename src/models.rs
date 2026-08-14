use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

// ConstruCode manda `null` em vez de omitir campos numéricos quando não há
// valor (ex: limite "sem limite"). Trata null como 0 em vez de dar erro de
// deserialização.
fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

// ===== Login (email/senha) =====

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct LoginCredentials {
    pub email: String,
    pub password: String,
}

// ===== Token de sessão =====
// Formato exato do JSON decodificado do cookie __session após o login
// (POST em CONSTRUCODE_LOGIN_URL).
#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct SessionToken {
    pub admin: String,
    pub email: String,
    #[serde(rename = "expirationDate")]
    pub expiration_date: String,
    #[serde(rename = "idSystem")]
    pub id_system: String,
    pub image: String,
    pub name: String,
    pub token: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "companyLogo")]
    pub company_logo: String,
    #[serde(rename = "companyName")]
    pub company_name: String,
}

// ===== Token gravado em disco (data/token.json) =====
// Só o essencial pra validar sessão sem precisar logar de novo a cada request.
#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct StoredToken {
    pub token: String,
    pub expiration_date: String,
}

// ===== Empreendimento =====
// Formato exato devolvido por GET CONSTRUCODE_ENTERPRISES_URL
// (item de myEnterprises / sharedEnterprises, já decodificado do turbo-stream).
// value/percentage/documents/people/etc podem vir -1 do ConstruCode
// (sentinela de "sem limite"/"não aplicável"), por isso i64 e não u32.
#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct EnterpriseRemainingDays {
    #[serde(default, deserialize_with = "null_as_default")]
    pub value: i64,
    #[serde(default, deserialize_with = "null_as_default")]
    pub percentage: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct EnterpriseCount {
    #[serde(default, deserialize_with = "null_as_default")]
    pub documents: i64,
    #[serde(default, deserialize_with = "null_as_default")]
    pub prints: i64,
    #[serde(default, deserialize_with = "null_as_default")]
    pub tasks: i64,
    #[serde(default, deserialize_with = "null_as_default")]
    pub people: i64,
    #[serde(rename = "remainingDays")]
    pub remaining_days: EnterpriseRemainingDays,
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct EnterpriseLimits {
    #[serde(default, deserialize_with = "null_as_default")]
    pub documents: i64,
    #[serde(default, deserialize_with = "null_as_default")]
    pub people: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct Enterprise {
    #[serde(default, deserialize_with = "null_as_default")]
    pub id: u32,
    #[serde(rename = "idSystem", default, deserialize_with = "null_as_default")]
    pub id_system: u32,
    pub name: String,
    pub image: String,
    pub plan: String,
    pub active: bool,
    pub company: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub count: EnterpriseCount,
    pub limits: EnterpriseLimits,
    pub permissions: HashMap<String, bool>,
}

// ===== Disciplina =====
// Extraída do <select id="idDisciplina"> da página
// GET CONSTRUCODE_PROJETO_URL?id={idObra}.
#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct Discipline {
    pub id: u32,
    pub sigla: String,
    pub name: String,
}

// ===== Item da disciplina (documento/planta) =====
// Formato exato de cada elemento de JsonObject.Documentos, devolvido por
// POST CONSTRUCODE_PLANTAS_URL.
// Campos numéricos/bool usam null_as_default: o ConstruCode manda null
// neles com frequência (ex: Fase/Liberado/BIMID sem valor aplicável).
#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct DisciplineItem {
    #[serde(rename = "Id", default, deserialize_with = "null_as_default")]
    pub id: u32,
    #[serde(rename = "idPlantaPai", default)]
    pub id_planta_pai: Option<u32>,
    #[serde(rename = "Descricao")]
    pub descricao: String,
    #[serde(rename = "Detalhamento")]
    pub detalhamento: Option<String>,
    #[serde(rename = "Obs")]
    pub obs: Option<String>,
    #[serde(rename = "Revisao")]
    pub revisao: String,
    #[serde(rename = "Usuario")]
    pub usuario: String,
    #[serde(rename = "FileName")]
    pub file_name: String,
    #[serde(rename = "FileSize", default, deserialize_with = "null_as_default")]
    pub file_size: u64,
    #[serde(rename = "Extensao")]
    pub extensao: String,
    #[serde(rename = "Extensoes")]
    pub extensoes: Vec<String>,
    #[serde(rename = "PathUrl")]
    pub path_url: String,
    #[serde(rename = "OriginalFileName")]
    pub original_file_name: String,
    #[serde(rename = "TituloCc")]
    pub titulo_cc: String,
    #[serde(rename = "OriginalFileId")]
    pub original_file_id: String,
    #[serde(rename = "Disciplina")]
    pub disciplina: String,
    #[serde(rename = "Sigla")]
    pub sigla: String,
    #[serde(rename = "Cor")]
    pub cor: String,
    #[serde(rename = "ManterRevisaoAtual", default, deserialize_with = "null_as_default")]
    pub manter_revisao_atual: bool,
    #[serde(rename = "IdObra", default, deserialize_with = "null_as_default")]
    pub id_obra: u32,
    #[serde(rename = "IdTipo", default, deserialize_with = "null_as_default")]
    pub id_tipo: u32,
    #[serde(rename = "Fase", default, deserialize_with = "null_as_default")]
    pub fase: u32,
    #[serde(rename = "Liberado", default, deserialize_with = "null_as_default")]
    pub liberado: u32,
    #[serde(rename = "IsCc", default, deserialize_with = "null_as_default")]
    pub is_cc: bool,
    #[serde(rename = "Novo", default, deserialize_with = "null_as_default")]
    pub novo: bool,
    #[serde(rename = "ExtensoesCc")]
    pub extensoes_cc: Vec<String>,
    #[serde(rename = "IsOwner", default, deserialize_with = "null_as_default")]
    pub is_owner: bool,
    #[serde(rename = "IsAtuante", default, deserialize_with = "null_as_default")]
    pub is_atuante: bool,
    #[serde(rename = "PodeBaixarDisciplinasNaoAtuantes", default, deserialize_with = "null_as_default")]
    pub pode_baixar_disciplinas_nao_atuantes: bool,
    #[serde(rename = "DataAtualizacao")]
    pub data_atualizacao: String,
    #[serde(rename = "Formato")]
    pub formato: String,
    #[serde(rename = "CanDeleteDocuments", default, deserialize_with = "null_as_default")]
    pub can_delete_documents: bool,
    #[serde(rename = "PermitirDownload", default, deserialize_with = "null_as_default")]
    pub permitir_download: bool,
    #[serde(rename = "PermitirImpressao", default, deserialize_with = "null_as_default")]
    pub permitir_impressao: bool,
    #[serde(rename = "PermitirCompartilhar", default, deserialize_with = "null_as_default")]
    pub permitir_compartilhar: bool,
    #[serde(rename = "PermitirExportarListados", default, deserialize_with = "null_as_default")]
    pub permitir_exportar_listados: bool,
    #[serde(rename = "PermitirReposicionarEtiqueta", default, deserialize_with = "null_as_default")]
    pub permitir_reposicionar_etiqueta: bool,
    #[serde(rename = "PermitirDownloadEmLote", default, deserialize_with = "null_as_default")]
    pub permitir_download_em_lote: bool,
    #[serde(rename = "PermitirMoverDocumentos", default, deserialize_with = "null_as_default")]
    pub permitir_mover_documentos: bool,
    #[serde(rename = "PermitirAlterarDisciplina", default, deserialize_with = "null_as_default")]
    pub permitir_alterar_disciplina: bool,
    #[serde(rename = "PermitirAlterarFase", default, deserialize_with = "null_as_default")]
    pub permitir_alterar_fase: bool,
    #[serde(rename = "PermitirAlterarStatus", default, deserialize_with = "null_as_default")]
    pub permitir_alterar_status: bool,
    #[serde(rename = "PermitirAlterarFormato", default, deserialize_with = "null_as_default")]
    pub permitir_alterar_formato: bool,
    #[serde(rename = "PermitirEditar", default, deserialize_with = "null_as_default")]
    pub permitir_editar: bool,
    #[serde(rename = "ManterObsoletarRevisao", default, deserialize_with = "null_as_default")]
    pub manter_obsoletar_revisao: bool,
    #[serde(rename = "IsTimeCampo", default, deserialize_with = "null_as_default")]
    pub is_time_campo: bool,
    #[serde(rename = "ProcessarBIM", default, deserialize_with = "null_as_default")]
    pub processar_bim: bool,
    #[serde(rename = "HasVizualizarDocumentos", default, deserialize_with = "null_as_default")]
    pub has_vizualizar_documentos: bool,
    #[serde(rename = "HasEtiquetaQRCode", default, deserialize_with = "null_as_default")]
    pub has_etiqueta_qr_code: bool,
    #[serde(rename = "HasGerenciadorDocumentos", default, deserialize_with = "null_as_default")]
    pub has_gerenciador_documentos: bool,
    #[serde(rename = "HasDownloadDocumentos", default, deserialize_with = "null_as_default")]
    pub has_download_documentos: bool,
    #[serde(rename = "RevisaoAnteriorNomePlanta")]
    pub revisao_anterior_nome_planta: Option<String>,
    #[serde(rename = "RevisaoAnteriorRevisao")]
    pub revisao_anterior_revisao: Option<String>,
    #[serde(rename = "RevisaoAnteriorDataAtualizacao")]
    pub revisao_anterior_data_atualizacao: Option<String>,
    #[serde(rename = "HasTask", default, deserialize_with = "null_as_default")]
    pub has_task: bool,
    #[serde(rename = "HasTaskCount", default, deserialize_with = "null_as_default")]
    pub has_task_count: u32,
    #[serde(rename = "HasOpenedTaskCount", default, deserialize_with = "null_as_default")]
    pub has_opened_task_count: u32,
    #[serde(rename = "SaldoBim", default, deserialize_with = "null_as_default")]
    pub saldo_bim: i64,
    #[serde(rename = "UserCanProcessBimFile", default, deserialize_with = "null_as_default")]
    pub user_can_process_bim_file: bool,
    #[serde(rename = "BIMFileName")]
    pub bim_file_name: Option<String>,
    #[serde(rename = "BIMPathURL")]
    pub bim_path_url: Option<String>,
    #[serde(rename = "BIMID", default, deserialize_with = "null_as_default")]
    pub bim_id: u32,
    #[serde(rename = "BIMFileSize", default, deserialize_with = "null_as_default")]
    pub bim_file_size: u64,
    #[serde(rename = "DWGFileName")]
    pub dwg_file_name: Option<String>,
    #[serde(rename = "DWGPathURL")]
    pub dwg_path_url: Option<String>,
    #[serde(rename = "DWGID", default, deserialize_with = "null_as_default")]
    pub dwg_id: u32,
    #[serde(rename = "DWGFileSize", default, deserialize_with = "null_as_default")]
    pub dwg_file_size: u64,
    #[serde(rename = "Emitente")]
    pub emitente: String,
    #[serde(rename = "IdUsuario", default, deserialize_with = "null_as_default")]
    pub id_usuario: u32,
    #[serde(rename = "FotoAccount")]
    pub foto_account: String,
    #[serde(rename = "DataLiberacao")]
    pub data_liberacao: String,
}
