#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
mod executor;
mod filters;
mod services;
mod structures;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use askama::Template;
use axum::{
    body::Body,
    extract::{Path, Query, Request, State},
    handler::Handler,
    http::{
        header::{ACCEPT, CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE},
        HeaderName, HeaderValue, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Extension, Router,
};
use axum_extra::routing::RouterExt;
use base64::{prelude::BASE64_STANDARD, Engine};
use bustdir::BustDir;
use parking_lot::RwLock;
use reqwest::{header::HeaderMap, redirect::Policy, Client};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};
use tracing::Level;

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
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .json()
        .init();
    let asset_dir = std::env::var("ASSET_DIR").unwrap_or_else(|_| "./assets/".to_owned());
    let root_url = valk_utils::get_var("ROOT_URL");
    let root_url = root_url.trim_end_matches('/').to_owned();
    let port: u16 = std::env::var("PORT").map_or(DEFAULT_PORT, |v| v.parse().unwrap());
    let bust_dir = BustDir::new(&asset_dir).expect("Failed to build cache busting directory");

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
    info!("Fetching minecraft server status");
    let current_mcstatus: Arc<RwLock<ServicesResponse>> =
        Arc::new(RwLock::new(get_mcstatus(http_client.clone()).await));
    info!(
        status = ?current_mcstatus.read(),
        "Got mojang service status"
    );
    tokio::spawn(refresh_mcstatus(http_client, Arc::clone(&current_mcstatus)));

    let state = AppState {
        svc_response: current_mcstatus,
        root_url: root_url.into(),
        bust_dir: bust_dir.into(),
    };

    let cache_none =
        SetResponseHeaderLayer::overriding(CACHE_CONTROL.clone(), CACHE_CONTROL_NONE.clone());
    let cache_medium =
        SetResponseHeaderLayer::overriding(CACHE_CONTROL.clone(), CACHE_CONTROL_MEDIUM.clone());
    let cache_max =
        SetResponseHeaderLayer::overriding(CACHE_CONTROL.clone(), CACHE_CONTROL_IMMUTABLE.clone());
    let noindex = SetResponseHeaderLayer::overriding(ROBOTS_NAME.clone(), ROBOTS_VALUE.clone());
    let csp = SetResponseHeaderLayer::overriding(CONTENT_SECURITY_POLICY, CSP_VALUE.clone());
    let clacks = SetResponseHeaderLayer::overriding(CLACKS_NAME.clone(), CLACKS_VALUE.clone());
    let error_handler = axum::middleware::from_fn_with_state(state.clone(), error_middleware);

    let serve_dir_raw = ServeDir::new(&asset_dir)
        .append_index_html_on_directories(true)
        .precompressed_gzip()
        .precompressed_br()
        .precompressed_deflate()
        .precompressed_zstd()
        .fallback(handle_404.with_state(state.clone()));
    let serve_dir = ServiceBuilder::new()
        .layer(noindex.clone())
        .layer(cache_max.clone())
        .service(serve_dir_raw);
    let api = Router::new()
        .route("/api/:address", get(handle_java_ping))
        .route("/api/java/:address", get(handle_java_ping))
        .route("/api/bedrock/:address", get(handle_bedrock_ping))
        .route("/api/java/", get(no_address))
        .route("/api/bedrock/", get(no_address))
        .route("/api/services", get(services::handle_mcstatus))
        .layer(ServiceBuilder::new().layer(noindex).layer(cache_none));
    let router = Router::new()
        .route("/", get(root))
        .route_with_tsr("/api/", get(api_info))
        .route("/ping/redirect", get(ping_redirect).layer(cache_max))
        .route_with_tsr("/ping/:edition/:hostname", get(ping_page))
        .route("/internal/ping-frame/:edition/:hostname", get(ping_frame))
        .route("/internal/ping-markup/:edition/:hostname", get(ping_markup))
        .route(
            "/internal/icon/:edition/:hostname/icon.:ext",
            get(ping_image).layer(cache_medium),
        )
        .fallback_service(serve_dir)
        .merge(api)
        .layer(
            ServiceBuilder::new()
                .layer(csp)
                .layer(clacks)
                .layer(error_handler),
        )
        .with_state(state);

    let socket_address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));
    let tcp = TcpListener::bind(socket_address).await.unwrap();
    info!(?socket_address, "Listening on socket");
    axum::serve(tcp, router)
        .with_graceful_shutdown(vss::shutdown_signal())
        .await
        .unwrap();
}

#[derive(Clone)]
pub struct AppState {
    svc_response: Arc<RwLock<ServicesResponse>>,
    root_url: Arc<str>,
    bust_dir: Arc<BustDir>,
}

static ROBOTS_NAME: HeaderName = HeaderName::from_static("x-robots-tag");
static ROBOTS_VALUE: HeaderValue = HeaderValue::from_static("noindex");
static CACHE_CONTROL_IMMUTABLE: HeaderValue =
    HeaderValue::from_static("immutable, public, max-age=31536000");
static CACHE_CONTROL_MEDIUM: HeaderValue =
    HeaderValue::from_static("max-age=7200, public, stale-while-revalidate");
static CACHE_CONTROL_NONE: HeaderValue = HeaderValue::from_static("max-age=0, no-store");

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

static CLACKS_NAME: HeaderName = HeaderName::from_static("x-clacks-overhead");
static CLACKS_VALUE: HeaderValue = HeaderValue::from_static("GNU Alexander \"Technoblade\"");

#[derive(Template)]
#[template(path = "index.html")]
pub struct RootTemplate {
    svc_status: ServicesResponse,
    root_url: Arc<str>,
    bd: Arc<BustDir>,
}

async fn root(State(state): State<AppState>) -> RootTemplate {
    RootTemplate {
        svc_status: *state.svc_response.read(),
        root_url: state.root_url,
        bd: state.bust_dir,
    }
}

#[derive(Template)]
#[template(path = "api.html")]
pub struct ApiTemplate {
    bd: Arc<BustDir>,
    root_url: Arc<str>,
}

async fn api_info(State(state): State<AppState>) -> ApiTemplate {
    ApiTemplate {
        root_url: state.root_url,
        bd: state.bust_dir,
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
    root_url: Arc<str>,
    bd: Arc<BustDir>,
    hostname: String,
    edition: String,
}

async fn ping_page(
    State(state): State<AppState>,
    Path((edition, hostname)): Path<(String, String)>,
) -> Result<PingPageTemplate, Failure> {
    match edition.as_str() {
        "java" | "bedrock" => {}
        _ => return Err(Failure::UnknownEdition),
    }
    Ok(PingPageTemplate {
        svc_status: *state.svc_response.read(),
        root_url: state.root_url,
        bd: state.bust_dir,
        hostname,
        edition,
    })
}

async fn ping_generic(edition: &str, hostname: String) -> Result<MCPingResponse, Failure> {
    let ping = match edition {
        "java" => ping_java(hostname).await?,
        "bedrock" => ping_bedrock(hostname).await?,
        _ => return Err(Failure::UnknownEdition),
    };
    Ok(ping)
}

#[derive(Template)]
#[template(path = "ping-frame.html")]
pub struct PingFrameTemplate {
    ping: MCPingResponse,
    bd: Arc<BustDir>,
    root_url: Arc<str>,
    edition: String,
    hostname: String,
}

async fn ping_frame(
    State(state): State<AppState>,
    Path((edition, hostname)): Path<(String, String)>,
) -> Result<PingFrameTemplate, Failure> {
    let ping = ping_generic(&edition, hostname.clone()).await?;
    Ok(PingFrameTemplate {
        ping,
        root_url: state.root_url,
        bd: state.bust_dir,
        edition,
        hostname,
    })
}

#[derive(Template)]
#[template(path = "ping-element.html")]
pub struct PingElementTemplate {
    ping: MCPingResponse,
    bd: Arc<BustDir>,
    root_url: Arc<str>,
    edition: String,
    hostname: String,
}

async fn ping_markup(
    State(state): State<AppState>,
    Path((edition, hostname)): Path<(String, String)>,
) -> Result<PingElementTemplate, Failure> {
    let ping = ping_generic(&edition, hostname.clone()).await?;
    Ok(PingElementTemplate {
        ping,
        bd: state.bust_dir,
        root_url: state.root_url,
        edition,
        hostname,
    })
}

async fn ping_image(Path((edition, hostname)): Path<(String, String)>) -> Result<Png, StatusCode> {
    const PREFIX_LEN: usize = "data:image/png;base64,".len();
    debug!(edition, hostname, "Serving icon");
    let ping = match ping_generic(&edition, hostname.clone()).await {
        Ok(v) => v,
        Err(e) => {
            error!(error = ?e, "Encountered error decoding icon");
            return Err(StatusCode::NOT_FOUND);
        }
    };
    let Some(icon) = ping.icon else {
        return Err(StatusCode::NOT_FOUND);
    };
    let cut_icon = icon.split_at(PREFIX_LEN).1;
    let decoded = match BASE64_STANDARD.decode(cut_icon) {
        Ok(v) => v,
        Err(e) => {
            error!(error = ?e, "Encountered error decoding icon");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    Ok(Png(decoded))
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
        bd: state.bust_dir,
        root_url: state.root_url,
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Failure {
    #[error("Error connecting to the server")]
    ConnectionFailed(#[from] pyng::Error),
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
        (status, Extension(Arc::new(self)), Body::empty()).into_response()
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
    bd: Arc<BustDir>,
    root_url: Arc<str>,
}

static HTML_CTYPE: HeaderValue = HeaderValue::from_static("text/html;charset=utf-8");

pub struct Json<T: Serialize>(pub T);

static JSON_CTYPE: HeaderValue = HeaderValue::from_static("application/json;charset=utf-8");

fn infallible_json_serialize<T: Serialize>(data: &T) -> Vec<u8> {
    serde_json::to_vec_pretty(data).unwrap_or_else(|_| {
        r#"{"error": "JSON Serialization failed, please make a bug report"}"#
            .as_bytes()
            .to_vec()
    })
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        let body = infallible_json_serialize(&self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(CONTENT_TYPE, JSON_CTYPE.clone())],
            body,
        )
            .into_response()
    }
}

async fn error_middleware(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let json = req
        .headers()
        .get(ACCEPT)
        .is_some_and(|v| v.to_str().is_ok_and(|v| v.contains("application/json")));
    let mut resp = next.run(req).await;
    if let Some(failure) = resp.extensions().get::<Arc<Failure>>().cloned() {
        let error = failure.to_string();
        if json {
            resp.headers_mut().insert(CONTENT_TYPE, JSON_CTYPE.clone());
            let error = ErrorSerialization { error };
            let json = infallible_json_serialize(&error);
            *resp.body_mut() = Body::from(json);
        } else {
            resp.headers_mut().insert(CONTENT_TYPE, HTML_CTYPE.clone());
            let error = ErrorTemplate {
                error,
                bd: state.bust_dir,
                root_url: state.root_url,
            }
            .render()
            .unwrap_or_else(|e| format!("error rendering template: {e}"));
            *resp.body_mut() = Body::from(error);
        };
    }
    resp
}

pub struct Png(pub Vec<u8>);

impl IntoResponse for Png {
    fn into_response(self) -> Response {
        static PNG_CTYPE: HeaderValue = HeaderValue::from_static("image/png");
        let headers = [(CONTENT_TYPE, PNG_CTYPE.clone())];
        (headers, self.0).into_response()
    }
}
