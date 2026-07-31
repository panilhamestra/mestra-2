pub struct AppState {
    pub http_client: reqwest::Client,
    pub api_url: String,
    pub port: u16,
}

impl AppState {
    pub fn from_env() -> Self {
        let api_url =
            std::env::var("SOURCE_API_URL").expect("SOURCE_API_URL precisa estar setada!");

        let port = std::env::var("PORT")
            .expect("PORT precisa estar setada!")
            .parse()
            .expect("PORT precisa ser um número válido!");

        Self {
            http_client: reqwest::Client::new(),
            api_url,
            port,
        }
    }
}
