use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Account {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub status: String,
    pub storage_quota_bytes: i64,
    pub storage_used_bytes: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct IssuedApiKey {
    pub id: String,
    pub name: String,
    pub api_key: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApiKeySummary {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub revoked_at: Option<String>,
}

#[derive(Clone, Debug)]
pub enum AuthenticationMethod {
    ApiKey,
    BrowserSession { session_id: String },
}

#[derive(Clone, Debug)]
pub struct AuthenticatedAccount {
    pub account_id: String,
    pub method: AuthenticationMethod,
}
