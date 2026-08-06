use serde::Serialize;

use crate::{
    domain::media::{JobKind, JobStatus, MediaStatus, Visibility},
    state::AppState,
};

#[derive(Debug, Serialize)]
pub(super) struct RuntimeConfig {
    site: SiteConfig,
    links: LinkConfig,
    presentation: PresentationConfig,
    routes: UiRoutes,
    api: ApiEndpoints,
    registration: RegistrationUiConfig,
    features: FeatureConfig,
    limits: LimitConfig,
    options: OptionConfig,
    csrf_header_name: String,
}

#[derive(Debug, Serialize)]
struct SiteConfig {
    name: String,
    tagline: String,
}

#[derive(Debug, Serialize)]
struct LinkConfig {
    repository: String,
    documentation: String,
    support: Option<String>,
}

#[derive(Debug, Serialize)]
struct PresentationConfig {
    date_locale: String,
    time_zone: Option<String>,
    brand_color: String,
    dashboard_refresh_seconds: u64,
    jobs_refresh_seconds: u64,
    page_size_options: Vec<u32>,
}

#[derive(Debug, Serialize)]
struct UiRoutes {
    dashboard: String,
    register: String,
    login: String,
    upload: String,
    media: String,
    jobs: String,
    analytics: String,
    account: String,
}

#[derive(Debug, Serialize)]
struct ApiEndpoints {
    accounts: String,
    login: String,
    session: String,
    logout: String,
    account: String,
    api_keys: String,
    media: String,
    media_imports: String,
    media_item_template: String,
    media_jobs_template: String,
    media_retry_template: String,
    media_playback_token_template: String,
    jobs: String,
    dashboard: String,
    analytics_summary: String,
    analytics_time_series: String,
}

#[derive(Debug, Serialize)]
struct RegistrationUiConfig {
    mode: String,
    admin_header_name: String,
}

#[derive(Debug, Serialize)]
struct FeatureConfig {
    remote_imports: bool,
    transcoding: bool,
    analytics: bool,
}

#[derive(Debug, Serialize)]
struct LimitConfig {
    maximum_upload_bytes: u64,
    maximum_display_name_bytes: usize,
    maximum_api_key_name_bytes: usize,
    maximum_text_field_bytes: usize,
    minimum_password_bytes: usize,
    maximum_password_bytes: usize,
    maximum_page_size: u32,
}

#[derive(Debug, Serialize)]
struct OptionConfig {
    media_statuses: &'static [MediaStatus],
    media_visibilities: &'static [Visibility],
    job_statuses: &'static [JobStatus],
    job_kinds: &'static [JobKind],
}

pub(super) fn build(state: &AppState) -> RuntimeConfig {
    let web = &state.config.routes.web;
    let api = &state.config.routes.api;
    RuntimeConfig {
        site: SiteConfig {
            name: state.config.web.site_name.clone(),
            tagline: state.config.web.tagline.clone(),
        },
        links: LinkConfig {
            repository: state.config.web.repository_url.clone(),
            documentation: state.config.web.documentation_url.clone(),
            support: state.config.web.support_url.clone(),
        },
        presentation: PresentationConfig {
            date_locale: state.config.web.date_locale.clone(),
            time_zone: state.config.web.time_zone.clone(),
            brand_color: state.config.web.brand_color.clone(),
            dashboard_refresh_seconds: state.config.web.dashboard_refresh_seconds,
            jobs_refresh_seconds: state.config.web.jobs_refresh_seconds,
            page_size_options: state.config.web.page_size_options.clone(),
        },
        routes: UiRoutes {
            dashboard: web.clone(),
            register: format!("{web}/register"),
            login: format!("{web}/login"),
            upload: format!("{web}/upload"),
            media: format!("{web}/media"),
            jobs: format!("{web}/jobs"),
            analytics: format!("{web}/analytics"),
            account: format!("{web}/account"),
        },
        api: ApiEndpoints {
            accounts: format!("{api}/accounts"),
            login: format!("{api}/auth/login"),
            session: format!("{api}/auth/session"),
            logout: format!("{api}/auth/logout"),
            account: format!("{api}/account"),
            api_keys: format!("{api}/account/api-keys"),
            media: format!("{api}/media"),
            media_imports: format!("{api}/media/imports"),
            media_item_template: format!("{api}/media/{{media_id}}"),
            media_jobs_template: format!("{api}/media/{{media_id}}/jobs"),
            media_retry_template: format!("{api}/media/{{media_id}}/retry"),
            media_playback_token_template: format!("{api}/media/{{media_id}}/playback-tokens"),
            jobs: format!("{api}/jobs"),
            dashboard: format!("{api}/dashboard"),
            analytics_summary: format!("{api}/analytics/summary"),
            analytics_time_series: format!("{api}/analytics/time-series"),
        },
        registration: RegistrationUiConfig {
            mode: state.config.registration.mode.as_str().to_owned(),
            admin_header_name: state.config.registration.admin_header_name.clone(),
        },
        features: FeatureConfig {
            remote_imports: state.config.remote_imports.enabled,
            transcoding: state.config.transcoding.enabled,
            analytics: state.config.analytics.enabled,
        },
        limits: LimitConfig {
            maximum_upload_bytes: state.config.uploads.maximum_source_bytes,
            maximum_display_name_bytes: state.config.registration.maximum_display_name_bytes,
            maximum_api_key_name_bytes: state.config.registration.maximum_api_key_name_bytes,
            maximum_text_field_bytes: state.config.uploads.maximum_text_field_bytes,
            minimum_password_bytes: state.config.browser_auth.minimum_password_bytes,
            maximum_password_bytes: state.config.browser_auth.maximum_password_bytes,
            maximum_page_size: state.config.api.maximum_page_size,
        },
        options: OptionConfig {
            media_statuses: MediaStatus::ALL,
            media_visibilities: Visibility::ALL,
            job_statuses: JobStatus::ALL,
            job_kinds: JobKind::ALL,
        },
        csrf_header_name: state.config.browser_auth.csrf_header_name.clone(),
    }
}
