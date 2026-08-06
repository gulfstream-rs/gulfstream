use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Serialize;

use crate::{
    application::sessions::{self, LoginRequest},
    auth::BrowserSessionIssue,
    cookie::{self, SetCookie},
    domain::account::{Account, AuthenticationMethod},
    error::AppError,
    state::AppState,
};

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub account: Account,
    pub csrf_token: String,
    pub expires_at: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<(StatusCode, HeaderMap, Json<SessionResponse>), AppError> {
    let (account, session) = sessions::login(&state, request).await?;
    let mut headers = HeaderMap::new();
    append_session_cookie(&state, &mut headers, &session)?;
    Ok((
        StatusCode::OK,
        headers,
        Json(SessionResponse {
            account,
            csrf_token: session.csrf_token,
            expires_at: session.expires_at,
        }),
    ))
}

pub async fn current(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SessionResponse>, AppError> {
    let session = state.auth.refresh_browser_session(&headers).await?;
    let account =
        crate::application::accounts::get_account_by_id(&state, &session.account_id).await?;
    Ok(Json(SessionResponse {
        account,
        csrf_token: session.csrf_token,
        expires_at: session.expires_at,
    }))
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<(StatusCode, HeaderMap), AppError> {
    let authenticated = state.auth.authenticate_for_write(&headers).await?;
    let AuthenticationMethod::BrowserSession { session_id } = authenticated.method else {
        return Err(AppError::Forbidden);
    };
    state
        .auth
        .revoke_browser_session(&session_id, &authenticated.account_id)
        .await?;
    let mut response_headers = HeaderMap::new();
    cookie::append(
        &mut response_headers,
        SetCookie {
            name: &state.config.browser_auth.session_cookie_name,
            value: "",
            path: &state.config.browser_auth.session_cookie_path,
            domain: state.config.browser_auth.session_cookie_domain.as_deref(),
            max_age_seconds: Some(0),
            http_only: state.config.browser_auth.session_cookie_http_only,
            secure: state.config.browser_auth.session_cookie_secure,
            same_site: state
                .config
                .browser_auth
                .session_cookie_same_site
                .as_cookie_value(),
        },
    )?;
    Ok((StatusCode::NO_CONTENT, response_headers))
}

fn append_session_cookie(
    state: &AppState,
    headers: &mut HeaderMap,
    session: &BrowserSessionIssue,
) -> Result<(), AppError> {
    cookie::append(
        headers,
        SetCookie {
            name: &state.config.browser_auth.session_cookie_name,
            value: &session.session_token,
            path: &state.config.browser_auth.session_cookie_path,
            domain: state.config.browser_auth.session_cookie_domain.as_deref(),
            max_age_seconds: Some(state.config.browser_auth.session_ttl_seconds),
            http_only: state.config.browser_auth.session_cookie_http_only,
            secure: state.config.browser_auth.session_cookie_secure,
            same_site: state
                .config
                .browser_auth
                .session_cookie_same_site
                .as_cookie_value(),
        },
    )
}
