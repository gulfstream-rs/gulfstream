use axum::http::{HeaderMap, HeaderValue, header};

use crate::error::AppError;

pub struct SetCookie<'a> {
    pub name: &'a str,
    pub value: &'a str,
    pub path: &'a str,
    pub domain: Option<&'a str>,
    pub max_age_seconds: Option<i64>,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: &'a str,
}

#[must_use]
pub fn read(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|cookie| {
                let (cookie_name, value) = cookie.trim().split_once('=')?;
                (cookie_name == name).then(|| value.to_owned())
            })
        })
}

pub fn append(headers: &mut HeaderMap, settings: SetCookie<'_>) -> Result<(), AppError> {
    let mut cookie = format!(
        "{}={}; Path={}; SameSite={}",
        settings.name, settings.value, settings.path, settings.same_site
    );
    if let Some(domain) = settings.domain {
        cookie.push_str("; Domain=");
        cookie.push_str(domain);
    }
    if let Some(max_age_seconds) = settings.max_age_seconds {
        cookie.push_str("; Max-Age=");
        cookie.push_str(&max_age_seconds.to_string());
    }
    if settings.http_only {
        cookie.push_str("; HttpOnly");
    }
    if settings.secure {
        cookie.push_str("; Secure");
    }
    headers.append(
        header::SET_COOKIE,
        cookie.parse::<HeaderValue>().map_err(AppError::internal)?,
    );
    Ok(())
}
