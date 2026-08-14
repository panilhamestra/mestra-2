use std::sync::Arc;

use reqwest::cookie::Jar;

use crate::models::LoginCredentials;

pub struct AppState {
    pub http_client: reqwest::Client,

    // Guarda o cookie __session devolvido pelo login no ConstruCode
    // (upstream) e o reenvia automaticamente nas chamadas seguintes feitas
    // com http_client. É sessão com o ConstruCode, não com o cliente da
    // nossa API — a nossa API em si não tem sessão/cookie nenhum, só
    // confere o api_token a cada request (ver require_api_token em routes.rs).
    pub cookie_jar: Arc<Jar>,

    pub login_url: String,
    pub enterprises_url: String,
    pub projeto_url: String,
    pub plantas_url: String,

    pub credentials: LoginCredentials,

    // Token único que os clientes da nossa API devem mandar no header
    // X-Api-Token. Comparado em require_api_token.
    pub api_token: String,

    pub port: u16,
}

impl AppState {
    pub fn from_env() -> Self {
        let login_url = required_env("CONSTRUCODE_LOGIN_URL");
        let enterprises_url = required_env("CONSTRUCODE_ENTERPRISES_URL");
        let projeto_url = required_env("CONSTRUCODE_PROJETO_URL");
        let plantas_url = required_env("CONSTRUCODE_PLANTAS_URL");

        let credentials = LoginCredentials {
            email: required_env("CONSTRUCODE_EMAIL"),
            password: required_env("CONSTRUCODE_PASSWORD"),
        };

        let api_token = required_env("API_ACCESS_TOKEN");

        let port = required_env("PORT")
            .parse()
            .expect("PORT precisa ser um número válido!");

        let cookie_jar = Arc::new(Jar::default());

        // cookie_provider mantém o cookie de sessão do login entre as
        // e nos deixa ler o cookie de volta pra decodificar o token.
        let http_client = reqwest::Client::builder()
            .cookie_provider(Arc::clone(&cookie_jar))
            .user_agent("Mozilla/5.0")
            .build()
            .expect("Falha ao montar o cliente HTTP!");

        Self {
            http_client,
            cookie_jar,
            login_url,
            enterprises_url,
            projeto_url,
            plantas_url,
            credentials,
            api_token,
            port,
        }
    }
}

fn required_env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{key} precisa estar setada!"))
}
