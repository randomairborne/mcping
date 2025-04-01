#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
mod executor;
mod filters;
mod services;
mod structures;

use std::{
    convert::Infallible,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use arc_swap::ArcSwap;
use askama::Template;
use axum::{
    Extension, Router,
    body::Body,
    extract::{FromRequestParts, Path, Query, Request, State},
    handler::Handler,
    http::{
        HeaderName, HeaderValue, StatusCode,
        header::{ACCEPT, CACHE_CONTROL, CONTENT_TYPE},
        request::Parts,
    },
    middleware::Next,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use axum_extra::routing::RouterExt;
use base64::{Engine, prelude::BASE64_STANDARD};
use bustdir::BustDir;
use reqwest::{Client, header::HeaderMap, redirect::Policy};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};
use tower_sombrero::{
    Sombrero,
    csp::CspNonce,
    headers::{ContentSecurityPolicy, CspSchemeSource, CspSource},
};
use tracing::Level;

use crate::{
    executor::{ping_bedrock, ping_java},
    services::{get_mcstatus, refresh_mcstatus},
    structures::{MCPingResponse, ServicesResponse},
};

#[macro_use]
extern crate tracing;

const DEFAULT_PORT: u16 = 8080;
static JSON_CONTENT_TYPE: HeaderValue = HeaderValue::from_static("application/json;charset=utf-8");

#[allow(clippy::too_many_lines)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .json()
        .init();
    let asset_dir = std::env::var("ASSET_DIR").unwrap_or_else(|_| "./assets/".to_owned());
    let root_url = valk_utils::get_var("ROOT_URL");
    let root_url = root_url.trim_end_matches('/').to_owned();
    let port: u16 = valk_utils::parse_var_or("PORT", DEFAULT_PORT);
    let contact_email = valk_utils::get_var("CONTACT_EMAIL");
    let bust_dir = BustDir::new(&asset_dir).expect("Failed to build cache busting directory");

    let mut default_headers = HeaderMap::new();
    default_headers.insert(ACCEPT, JSON_CONTENT_TYPE.clone());

    let user_agent = format!(
        "mcping.me/{} (https://github.com/randomairborne/mcping; {contact_email})",
        env!("CARGO_PKG_VERSION")
    );
    let http_client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .default_headers(default_headers)
        .user_agent(user_agent)
        .redirect(Policy::limited(100))
        .build()
        .unwrap();

    info!("Fetching minecraft server status");
    let current_mcstatus: Arc<ArcSwap<ServicesResponse>> = Arc::new(ArcSwap::new(Arc::new(
        get_mcstatus(http_client.clone()).await,
    )));
    info!(
        status = ?**current_mcstatus.load(),
        "Got mojang service status"
    );
    let shutdown_token = CancellationToken::new();
    let status_refresh = tokio::spawn(refresh_mcstatus(
        http_client,
        Arc::clone(&current_mcstatus),
        shutdown_token.clone(),
    ));

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
    let shared_cors =
        SetResponseHeaderLayer::overriding(ALLOW_CORS_NAME.clone(), ALLOW_CORS_VALUE.clone());
    let clacks = SetResponseHeaderLayer::appending(CLACKS_NAME.clone(), CLACKS_VALUE.clone());

    let csp = get_csp();
    let sombrero = Sombrero::default().content_security_policy(csp);

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
        .route("/api/{address}", get(handle_java_ping))
        .route("/api/java/{address}", get(handle_java_ping))
        .route("/api/bedrock/{address}", get(handle_bedrock_ping))
        .route("/api/java/", get(no_address))
        .route("/api/bedrock/", get(no_address))
        .route("/api/services", get(services::handle_mcstatus))
        .layer(
            ServiceBuilder::new()
                .layer(noindex)
                .layer(cache_none)
                .layer(shared_cors),
        );
    let router = Router::new()
        .route("/", get(root))
        .route_with_tsr("/api/", get(api_info))
        .route("/ping/redirect", get(ping_redirect).layer(cache_max))
        .route_with_tsr("/ping/{edition}/{hostname}", get(ping_page))
        .route("/internal/ping-frame/{edition}/{hostname}", get(ping_frame))
        .route(
            "/internal/ping-markup/{edition}/{hostname}",
            get(ping_markup),
        )
        .route(
            "/internal/icon/{edition}/{hostname}/icon.png",
            get(ping_image).layer(cache_medium),
        )
        .fallback_service(serve_dir)
        .merge(api)
        .layer(
            ServiceBuilder::new()
                .layer(sombrero)
                .layer(clacks)
                .layer(error_handler),
        )
        .with_state(state);

    let socket_address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));
    let tcp = TcpListener::bind(socket_address).await.unwrap();
    info!(?socket_address, "Listening on socket");
    let server_shutdown_token = shutdown_token.clone();
    let server_task = tokio::spawn(async move {
        axum::serve(tcp, router)
            .with_graceful_shutdown(server_shutdown_token.cancelled_owned())
            .await
            .unwrap();
    });

    vss::shutdown_signal().await;

    shutdown_token.cancel();

    server_task.await.expect("Server panicked");
    status_refresh.await.expect("Updater tasked panicked");
    Ok(())
}

fn get_csp() -> ContentSecurityPolicy {
    ContentSecurityPolicy::new_empty()
        .upgrade_insecure_requests(true)
        .default_src(CspSource::SelfOrigin)
        .frame_src(CspSource::SelfOrigin)
        .object_src(CspSource::None)
        .base_uri(CspSource::None)
        .style_src(CspSource::Nonce)
        .img_src([CspSource::SelfOrigin, CspSchemeSource::Data.into()])
        .connect_src([
            CspSource::SelfOrigin,
            CspSource::Host("https://v4.giveip.io".to_string()),
            CspSource::Host("cloudflareinsights.com".to_string()),
        ])
        .script_src([
            CspSource::StrictDynamic,
            CspSource::Nonce,
            CspSource::UnsafeInline,
            CspSchemeSource::Http.into(),
            CspSchemeSource::Https.into(),
        ])
}

#[derive(Clone)]
pub struct AppState {
    svc_response: Arc<ArcSwap<ServicesResponse>>,
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

static CLACKS_NAME: HeaderName = HeaderName::from_static("x-clacks-overhead");
static CLACKS_VALUE: HeaderValue = HeaderValue::from_static("GNU Alexander \"Technoblade\"");

static ALLOW_CORS_NAME: HeaderName = HeaderName::from_static("access-control-allow-origin");
static ALLOW_CORS_VALUE: HeaderValue = HeaderValue::from_static("*");

#[derive(Template)]
#[template(path = "index.hbs", escape = "html")]
pub struct RootTemplate {
    svc_status: ServicesResponse,
    root_url: Arc<str>,
    bd: Arc<BustDir>,
    nonce: String,
}

async fn root(
    State(state): State<AppState>,
    CspNonce(nonce): CspNonce,
) -> HtmlTemplate<RootTemplate> {
    // create a copy of the services response
    let svc_status = **state.svc_response.load();
    RootTemplate {
        svc_status,
        root_url: state.root_url,
        bd: state.bust_dir,
        nonce,
    }
    .into()
}

#[derive(Template)]
#[template(path = "api.hbs", escape = "html")]
pub struct ApiTemplate {
    bd: Arc<BustDir>,
    root_url: Arc<str>,
    nonce: String,
}

async fn api_info(
    State(state): State<AppState>,
    CspNonce(nonce): CspNonce,
) -> HtmlTemplate<ApiTemplate> {
    ApiTemplate {
        root_url: state.root_url,
        bd: state.bust_dir,
        nonce,
    }
    .into()
}

#[derive(Deserialize)]
pub struct PingQuery {
    edition: String,
    address: String,
}

async fn ping_redirect(
    State(state): State<AppState>,
    Query(form): Query<PingQuery>,
) -> Result<Redirect, Infallible> {
    Ok(Redirect::to(&format!(
        "{}/ping/{}/{}",
        state.root_url, form.edition, form.address
    )))
}

#[derive(Template)]
#[template(path = "ping-page.hbs", escape = "html")]
pub struct PingPageTemplate {
    svc_status: ServicesResponse,
    root_url: Arc<str>,
    bd: Arc<BustDir>,
    hostname: String,
    edition: String,
    nonce: String,
}

async fn ping_page(
    State(state): State<AppState>,
    CspNonce(nonce): CspNonce,
    Path((edition, hostname)): Path<(String, String)>,
) -> Result<HtmlTemplate<PingPageTemplate>, Failure> {
    match edition.as_str() {
        "java" | "bedrock" => {}
        _ => return Err(Failure::UnknownEdition),
    }
    Ok(PingPageTemplate {
        svc_status: **state.svc_response.load(),
        root_url: state.root_url,
        bd: state.bust_dir,
        hostname,
        edition,
        nonce,
    }
    .into())
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
#[template(path = "ping-frame.hbs", escape = "html")]
pub struct PingFrameTemplate {
    ping: MCPingResponse,
    bd: Arc<BustDir>,
    root_url: Arc<str>,
    edition: String,
    hostname: String,
    nonce: String,
}

async fn ping_frame(
    State(state): State<AppState>,
    CspNonce(nonce): CspNonce,
    Path((edition, hostname)): Path<(String, String)>,
    CfConnectingIp(ip): CfConnectingIp,
) -> Result<HtmlTemplate<PingFrameTemplate>, Failure> {
    info!(edition, path = "frame", target = hostname, on_behalf = ?ip, "Pinging server");
    let ping = ping_generic(&edition, hostname.clone()).await?;
    Ok(HtmlTemplate(PingFrameTemplate {
        ping,
        root_url: state.root_url,
        bd: state.bust_dir,
        edition,
        hostname,
        nonce,
    }))
}

#[derive(Template)]
#[template(path = "ping-element.hbs", escape = "html")]
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
    CfConnectingIp(ip): CfConnectingIp,
) -> Result<HtmlTemplate<PingElementTemplate>, MarkupOnlyFailure> {
    info!(edition, path = "markup", target = hostname, on_behalf = ?ip, "Pinging server");
    let ping = ping_generic(&edition, hostname.clone()).await?;
    Ok(PingElementTemplate {
        ping,
        bd: state.bust_dir,
        root_url: state.root_url,
        edition,
        hostname,
    }
    .into())
}

async fn ping_image(
    Path((edition, hostname)): Path<(String, String)>,
    CfConnectingIp(ip): CfConnectingIp,
) -> Result<Png, StatusCode> {
    const PREFIX_LEN: usize = "data:image/png;base64,".len();
    info!(edition, path = "image", target = hostname, on_behalf = ?ip, "Pinging server");
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

async fn handle_java_ping(
    Path(address): Path<String>,
    CfConnectingIp(ip): CfConnectingIp,
) -> Result<Json<MCPingResponse>, Failure> {
    info!(edition = "java", path = "api", target = address, on_behalf = ?ip, "Pinging server");
    Ok(Json(ping_java(address).await?))
}

async fn handle_bedrock_ping(
    Path(address): Path<String>,
    CfConnectingIp(ip): CfConnectingIp,
) -> Result<Json<MCPingResponse>, Failure> {
    info!(edition = "bedrock", path = "api", target = address, on_behalf = ?ip, "Pinging server");
    Ok(Json(ping_bedrock(address).await?))
}

async fn no_address() -> Failure {
    Failure::NoHostname
}

#[allow(clippy::unused_async)]
async fn handle_404(
    State(state): State<AppState>,
    CspNonce(nonce): CspNonce,
) -> HtmlTemplate<ErrorTemplate> {
    ErrorTemplate {
        error: "404 not found".to_owned(),
        bd: state.bust_dir,
        root_url: state.root_url,
        nonce,
    }
    .into()
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
    #[error("Could not convert header to string")]
    HeaderToStr(#[from] axum::http::header::ToStrError),
    #[error("Could not convert string to IP address")]
    AddressParse(#[from] std::net::AddrParseError),
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
            Self::NoHostname
            | Self::UnknownEdition
            | Self::AddressParse(_)
            | Self::HeaderToStr(_) => StatusCode::BAD_REQUEST,
        };
        error!(error = ?self, "Error processing request");
        (status, Extension(Arc::new(self)), Body::empty()).into_response()
    }
}

pub struct MarkupOnlyFailure(pub Failure);

#[derive(Copy, Clone, Debug)]
pub struct SendErrorElement;

impl IntoResponse for MarkupOnlyFailure {
    fn into_response(self) -> Response {
        let mut resp = self.0.into_response();
        resp.extensions_mut().insert(SendErrorElement);
        resp
    }
}

impl From<Failure> for MarkupOnlyFailure {
    fn from(value: Failure) -> Self {
        Self(value)
    }
}

#[derive(Serialize)]
pub struct ErrorSerialization {
    error: String,
}

#[derive(Template)]
#[template(path = "error.hbs", escape = "html")]
pub struct ErrorTemplate {
    error: String,
    bd: Arc<BustDir>,
    root_url: Arc<str>,
    nonce: String,
}

#[derive(Template)]
#[template(path = "error-element.hbs", escape = "html")]
pub struct ErrorElement {
    error: String,
}

pub struct Json<T: Serialize>(pub T);

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
            [(CONTENT_TYPE, JSON_CONTENT_TYPE.clone())],
            body,
        )
            .into_response()
    }
}

async fn error_middleware(
    State(state): State<AppState>,
    CspNonce(nonce): CspNonce,
    req: Request,
    next: Next,
) -> Response {
    let json = req
        .headers()
        .get(ACCEPT)
        .is_some_and(|v| v.to_str().is_ok_and(|v| v.contains("application/json")));

    let mut resp = next.run(req).await;
    if let Some(failure) = resp.extensions().get::<Arc<Failure>>().cloned() {
        let error = failure.to_string();
        let status = resp.status();
        let markup_only = resp.extensions().get::<SendErrorElement>().is_some();

        if json {
            resp.headers_mut()
                .insert(CONTENT_TYPE, JSON_CONTENT_TYPE.clone());
            let error = ErrorSerialization { error };
            (status, Json(infallible_json_serialize(&error))).into_response()
        } else if markup_only {
            let error = ErrorElement { error };
            (status, HtmlTemplate(error)).into_response()
        } else {
            let error = ErrorTemplate {
                error,
                bd: state.bust_dir,
                root_url: state.root_url,
                nonce,
            };
            (status, HtmlTemplate(error)).into_response()
        }
    } else {
        resp
    }
}

pub struct Png(pub Vec<u8>);

impl IntoResponse for Png {
    fn into_response(self) -> Response {
        static PNG_CONTENT_TYPE: HeaderValue = HeaderValue::from_static("image/png");
        let headers = [(CONTENT_TYPE, PNG_CONTENT_TYPE.clone())];
        (headers, self.0).into_response()
    }
}

pub struct CfConnectingIp(pub IpAddr);

impl<S: Sync> FromRequestParts<S> for CfConnectingIp {
    type Rejection = Failure;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        static NAME: HeaderName = HeaderName::from_static("cf-connecting-ip");

        let Some(ip_hdr) = parts.headers.get(&NAME) else {
            warn!("Cloudflare did not send cf-connecting-ip");
            return Ok(Self(IpAddr::V4(Ipv4Addr::UNSPECIFIED)));
        };

        let ip_str = ip_hdr.to_str()?;
        let ip = IpAddr::from_str(ip_str)?;
        Ok(Self(ip))
    }
}

pub struct HtmlTemplate<T>(pub T);

impl<T: askama::Template> IntoResponse for HtmlTemplate<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(v) => Html(v).into_response(),
            Err(e) => {
                error!(source = ?e, "Could not template");
                (StatusCode::INTERNAL_SERVER_ERROR, "Templating error").into_response()
            }
        }
    }
}

impl<T: askama::Template> From<T> for HtmlTemplate<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}
