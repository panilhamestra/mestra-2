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
// (POST https://construcode.com.br/Account/Login).
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
// Formato exato devolvido por GET https://web.construcode.com.br/Enterprises.data
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
// GET https://construcode.com.br/Projetos/Index?id={idObra}.
#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct Discipline {
    pub id: u32,
    pub sigla: String,
    pub name: String,
}

// ===== Item da disciplina (documento/planta) =====
// Formato exato de cada elemento de JsonObject.Documentos, devolvido por
// POST https://construcode.com.br/Plantas/PostPlantasByArea.
#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct DisciplineItem {
    #[serde(rename = "Id")]
    pub id: u32,
    #[serde(rename = "idPlantaPai")]
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
    #[serde(rename = "FileSize")]
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
    #[serde(rename = "ManterRevisaoAtual")]
    pub manter_revisao_atual: bool,
    #[serde(rename = "IdObra")]
    pub id_obra: u32,
    #[serde(rename = "IdTipo")]
    pub id_tipo: u32,
    #[serde(rename = "Fase")]
    pub fase: u32,
    #[serde(rename = "Liberado")]
    pub liberado: u32,
    #[serde(rename = "IsCc")]
    pub is_cc: bool,
    #[serde(rename = "Novo")]
    pub novo: bool,
    #[serde(rename = "ExtensoesCc")]
    pub extensoes_cc: Vec<String>,
    #[serde(rename = "IsOwner")]
    pub is_owner: bool,
    #[serde(rename = "IsAtuante")]
    pub is_atuante: bool,
    #[serde(rename = "PodeBaixarDisciplinasNaoAtuantes")]
    pub pode_baixar_disciplinas_nao_atuantes: bool,
    #[serde(rename = "DataAtualizacao")]
    pub data_atualizacao: String,
    #[serde(rename = "Formato")]
    pub formato: String,
    #[serde(rename = "CanDeleteDocuments")]
    pub can_delete_documents: bool,
    #[serde(rename = "PermitirDownload")]
    pub permitir_download: bool,
    #[serde(rename = "PermitirImpressao")]
    pub permitir_impressao: bool,
    #[serde(rename = "PermitirCompartilhar")]
    pub permitir_compartilhar: bool,
    #[serde(rename = "PermitirExportarListados")]
    pub permitir_exportar_listados: bool,
    #[serde(rename = "PermitirReposicionarEtiqueta")]
    pub permitir_reposicionar_etiqueta: bool,
    #[serde(rename = "PermitirDownloadEmLote")]
    pub permitir_download_em_lote: bool,
    #[serde(rename = "PermitirMoverDocumentos")]
    pub permitir_mover_documentos: bool,
    #[serde(rename = "PermitirAlterarDisciplina")]
    pub permitir_alterar_disciplina: bool,
    #[serde(rename = "PermitirAlterarFase")]
    pub permitir_alterar_fase: bool,
    #[serde(rename = "PermitirAlterarStatus")]
    pub permitir_alterar_status: bool,
    #[serde(rename = "PermitirAlterarFormato")]
    pub permitir_alterar_formato: bool,
    #[serde(rename = "PermitirEditar")]
    pub permitir_editar: bool,
    #[serde(rename = "ManterObsoletarRevisao")]
    pub manter_obsoletar_revisao: bool,
    #[serde(rename = "IsTimeCampo")]
    pub is_time_campo: bool,
    #[serde(rename = "ProcessarBIM")]
    pub processar_bim: bool,
    #[serde(rename = "HasVizualizarDocumentos")]
    pub has_vizualizar_documentos: bool,
    #[serde(rename = "HasEtiquetaQRCode")]
    pub has_etiqueta_qr_code: bool,
    #[serde(rename = "HasGerenciadorDocumentos")]
    pub has_gerenciador_documentos: bool,
    #[serde(rename = "HasDownloadDocumentos")]
    pub has_download_documentos: bool,
    #[serde(rename = "RevisaoAnteriorNomePlanta")]
    pub revisao_anterior_nome_planta: Option<String>,
    #[serde(rename = "RevisaoAnteriorRevisao")]
    pub revisao_anterior_revisao: Option<String>,
    #[serde(rename = "RevisaoAnteriorDataAtualizacao")]
    pub revisao_anterior_data_atualizacao: Option<String>,
    #[serde(rename = "HasTask")]
    pub has_task: bool,
    #[serde(rename = "HasTaskCount")]
    pub has_task_count: u32,
    #[serde(rename = "HasOpenedTaskCount")]
    pub has_opened_task_count: u32,
    #[serde(rename = "SaldoBim")]
    pub saldo_bim: i64,
    #[serde(rename = "UserCanProcessBimFile")]
    pub user_can_process_bim_file: bool,
    #[serde(rename = "BIMFileName")]
    pub bim_file_name: Option<String>,
    #[serde(rename = "BIMPathURL")]
    pub bim_path_url: Option<String>,
    #[serde(rename = "BIMID")]
    pub bim_id: u32,
    #[serde(rename = "BIMFileSize")]
    pub bim_file_size: u64,
    #[serde(rename = "DWGFileName")]
    pub dwg_file_name: Option<String>,
    #[serde(rename = "DWGPathURL")]
    pub dwg_path_url: Option<String>,
    #[serde(rename = "DWGID")]
    pub dwg_id: u32,
    #[serde(rename = "DWGFileSize")]
    pub dwg_file_size: u64,
    #[serde(rename = "Emitente")]
    pub emitente: String,
    #[serde(rename = "IdUsuario")]
    pub id_usuario: u32,
    #[serde(rename = "FotoAccount")]
    pub foto_account: String,
    #[serde(rename = "DataLiberacao")]
    pub data_liberacao: String,
}
