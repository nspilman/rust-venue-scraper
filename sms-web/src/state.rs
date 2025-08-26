use reqwest::Client;

#[derive(Clone)]
pub struct AppState {
    pub graphql_client: Client,
    pub graphql_url: String,
}
