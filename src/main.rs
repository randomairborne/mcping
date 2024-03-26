#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
mod executor;
mod filters;
mod services;
mod structures;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use askama::Template;
use axum::{
    extract::{Path, Query, Request, State},
    handler::Handler,
    http::{
        header::{CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE},
        HeaderName, HeaderValue, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use axum_extra::routing::RouterExt;
use parking_lot::RwLock;
use reqwest::{header::HeaderMap, redirect::Policy, Client};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    executor::{ping_bedrock, ping_java},
    services::{get_mcstatus, refresh_mcstatus},
    structures::{MCPingResponse, ServicesResponse},
};

#[macro_use]
extern crate tracing;

const DEFAULT_PORT: u16 = 8080;

#[tokio::main]
async fn main() {
    start_tracing();
    let asset_dir = std::env::var("ASSET_DIR").unwrap_or_else(|_| "./assets/".to_owned());
    let root_url = valk_utils::get_var("ROOT_URL");
    let root_url = root_url.trim_end_matches('/').to_owned();
    let port: u16 = std::env::var("PORT").map_or(DEFAULT_PORT, |v| v.parse().unwrap());
    let mut default_headers = HeaderMap::new();
    default_headers.insert("Accept", "application/json".parse().unwrap());
    let http_client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .default_headers(default_headers)
        .user_agent(concat!(
            "mcping.me/",
            env!("CARGO_PKG_VERSION"),
            " (https://github.com/randomairborne/mcping)"
        ))
        .redirect(Policy::limited(100))
        .build()
        .unwrap();
    let current_mcstatus: Arc<RwLock<ServicesResponse>> =
        Arc::new(RwLock::new(get_mcstatus(http_client.clone()).await));
    tokio::spawn(refresh_mcstatus(http_client, Arc::clone(&current_mcstatus)));

    let state = AppState {
        svc_response: current_mcstatus,
        root_url: root_url.into(),
    };

    let serve_dir = ServeDir::new(&asset_dir)
        .append_index_html_on_directories(true)
        .precompressed_gzip()
        .precompressed_br()
        .precompressed_deflate()
        .precompressed_zstd()
        .fallback(handle_404.with_state(state.clone()));
    let app = Router::new()
        .route("/", get(root))
        .route_with_tsr("/api/", get(api_info))
        .route_with_tsr("/ping/:edition/:hostname", get(ping_page))
        .route("/ping/redirect", get(ping_redirect))
        .route("/internal/ping-frame/:edition/:hostname", get(ping_frame))
        .route("/internal/ping-markup/:edition/:hostname", get(ping_markup))
        .route("/api/:address", get(handle_java_ping))
        .route("/api/java/:address", get(handle_java_ping))
        .route("/api/bedrock/:address", get(handle_bedrock_ping))
        .route("/api/java/", get(no_address))
        .route("/api/bedrock/", get(no_address))
        .route("/api/services", get(services::handle_mcstatus))
        .layer(axum::middleware::from_fn(noindex_cache))
        .fallback_service(serve_dir)
        .layer(axum::middleware::from_fn(csp))
        .with_state(state);
    let socket_address = SocketAddr::from(([0, 0, 0, 0], port));
    let tcp = TcpListener::bind(socket_address).await.unwrap();
    axum::serve(tcp, app)
        .with_graceful_shutdown(vss::shutdown_signal())
        .await
        .unwrap();
}

#[derive(Clone)]
pub struct AppState {
    svc_response: Arc<RwLock<ServicesResponse>>,
    root_url: Arc<str>,
}

static ROBOTS_NAME: HeaderName = HeaderName::from_static("x-robots-tag");
static ROBOTS_VALUE: HeaderValue = HeaderValue::from_static("noindex");
static CACHE_CONTROL_AGE: HeaderValue = HeaderValue::from_static("s-maxage=30");

static CSP_VALUE: HeaderValue = HeaderValue::from_static(
    "default-src 'self'; \
    frame-src 'self'; \
    img-src 'self' data:; \
    connect-src 'self' https://v4.giveip.io; \
    script-src 'self' https://static.cloudflareinsights.com; \
    style-src 'self'; \
    object-src 'none'; \
    base-uri 'none';",
);

async fn noindex_cache(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    resp.headers_mut()
        .insert(ROBOTS_NAME.clone(), ROBOTS_VALUE.clone());
    resp.headers_mut()
        .insert(CACHE_CONTROL, CACHE_CONTROL_AGE.clone());
    resp
}

async fn csp(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    resp.headers_mut()
        .insert(CONTENT_SECURITY_POLICY, CSP_VALUE.clone());
    resp
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct RootTemplate {
    svc_status: ServicesResponse,
    root_url: String,
}

async fn root(State(state): State<AppState>) -> RootTemplate {
    RootTemplate {
        svc_status: *state.svc_response.read(),
        root_url: state.root_url.to_string(),
    }
}

#[derive(Deserialize)]
pub struct PingQuery {
    edition: String,
    address: String,
}

async fn ping_redirect(
    State(state): State<AppState>,
    Query(form): Query<PingQuery>,
) -> Result<Redirect, ErrorTemplate> {
    Ok(Redirect::to(&format!(
        "{}/ping/{}/{}",
        state.root_url, form.edition, form.address
    )))
}

#[derive(Template)]
#[template(path = "ping-page.html")]
pub struct PingPageTemplate {
    svc_status: ServicesResponse,
    root_url: String,
    hostname: String,
    edition: String,
}

async fn ping_page(
    State(state): State<AppState>,
    Path((edition, hostname)): Path<(String, String)>,
) -> Result<PingPageTemplate, ErrorTemplate> {
    match edition.as_str() {
        "java" | "bedrock" => {}
        _ => {
            return Err(ErrorTemplate::from_failure(
                &Failure::UnknownEdition,
                &state,
            ))
        }
    }
    Ok(PingPageTemplate {
        svc_status: *state.svc_response.read(),
        root_url: state.root_url.to_string(),
        hostname,
        edition,
    })
}

async fn ping_generic(
    edition: String,
    hostname: String,
    state: &AppState,
) -> Result<MCPingResponse, ErrorTemplate> {
    let ping = match edition.as_str() {
        "java" => ping_java(hostname).await,
        "bedrock" => ping_bedrock(hostname).await,
        _ => {
            return Err(ErrorTemplate::from_failure(
                &Failure::UnknownEdition,
                &state,
            ))
        }
    };
    let ping = match ping {
        Ok(v) => v,
        Err(e) => return Err(ErrorTemplate::from_failure(&e, &state)),
    };
    Ok(ping)
}

#[derive(Template)]
#[template(path = "ping-frame.html")]
pub struct PingFrameTemplate {
    ping: MCPingResponse,
    root_url: String,
}

async fn ping_frame(
    State(state): State<AppState>,
    Path((edition, hostname)): Path<(String, String)>,
) -> Result<PingFrameTemplate, ErrorTemplate> {
    let ping = ping_generic(edition, hostname, &state).await?;
    Ok(PingFrameTemplate {
        ping,
        root_url: state.root_url.to_string(),
    })
}

#[derive(Template)]
#[template(path = "ping-element.html")]
pub struct PingElementTemplate {
    ping: MCPingResponse,
    root_url: String,
}

async fn ping_markup(
    State(state): State<AppState>,
    Path((edition, hostname)): Path<(String, String)>,
) -> Result<PingElementTemplate, ErrorTemplate> {
    let ping = ping_generic(edition, hostname, &state).await?;
    Ok(PingElementTemplate {
        ping,
        root_url: state.root_url.to_string(),
    })
}

#[derive(Template)]
#[template(path = "api.html")]
pub struct ApiTemplate {
    root_url: String,
}

async fn api_info(State(state): State<AppState>) -> ApiTemplate {
    ApiTemplate {
        root_url: state.root_url.to_string(),
    }
}

async fn handle_java_ping(Path(address): Path<String>) -> Result<Json<MCPingResponse>, Failure> {
    Ok(Json(ping_java(address).await?))
}

async fn handle_bedrock_ping(Path(address): Path<String>) -> Result<Json<MCPingResponse>, Failure> {
    Ok(Json(ping_bedrock(address).await?))
}

async fn no_address() -> Failure {
    Failure::NoHostname
}

#[allow(clippy::unused_async)]
async fn handle_404(State(state): State<AppState>) -> ErrorTemplate {
    ErrorTemplate {
        error: "404 not found".to_owned(),
        root_url: state.root_url.to_string(),
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Failure {
    #[error("Error connecting to the server")]
    ConnectionFailed(#[from] libmcping::Error),
    #[error("Timed out connecting to the server")]
    TimedOut,
    #[error("HTTP error")]
    StatusReqwestFailed(#[from] reqwest::Error),
    #[error("JSON processing error")]
    JsonProcessingFailed(#[from] serde_json::Error),
    #[error("No server address specified!")]
    NoHostname,
    #[error("Unknown edition!")]
    UnknownEdition,
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let status = match self {
            Self::ConnectionFailed(_) | Self::TimedOut => StatusCode::OK,
            Self::StatusReqwestFailed(_) => StatusCode::BAD_GATEWAY,
            Self::JsonProcessingFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NoHostname | Self::UnknownEdition => StatusCode::BAD_REQUEST,
        };
        error!(error = ?self, "Error processing request");
        let ser = ErrorSerialization {
            error: self.to_string(),
        };
        (status, Json(ser)).into_response()
    }
}

#[derive(Serialize)]
pub struct ErrorSerialization {
    error: String,
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    error: String,
    root_url: String,
}

impl ErrorTemplate {
    fn from_failure(failure: &Failure, state: &AppState) -> Self {
        Self {
            root_url: state.root_url.to_string(),
            error: failure.to_string(),
        }
    }
}

pub struct Json<T: Serialize>(pub T);

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        static JSON_CTYPE: HeaderValue = HeaderValue::from_static("application/json;charset=utf-8");

        let body = serde_json::to_vec_pretty(&self.0).unwrap_or_else(|_| {
            r#"{"error": "JSON Serialization failed, please make a bug report"}"#
                .as_bytes()
                .to_vec()
        });
        ([(CONTENT_TYPE, JSON_CTYPE.clone())], body).into_response()
    }
}

fn start_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(concat!(env!("CARGO_PKG_NAME"), "=info").parse().unwrap())
        .with_env_var("LOG")
        .from_env()
        .expect("failed to parse env");
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(env_filter)
        .init();
}
