#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
mod services;
mod structures;

use std::{borrow::Cow, net::SocketAddr, sync::Arc};

use axum::{
    extract::{Path, Request},
    http::{HeaderName, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::get,
};
use libmcping::{Bedrock, Java};
use reqwest::header::HeaderMap;
use tokio::{net::TcpListener, sync::RwLock};
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    services::{get_mcstatus, refresh_mcstatus},
    structures::{MCPingResponse, PlayerSample, Players, ServicesResponse, Version},
};

#[macro_use]
extern crate tracing;

#[tokio::main]
async fn main() {
    start_tracing();
    let asset_dir = std::env::var("ASSET_DIR").unwrap_or_else(|_| "./assets/".to_owned());
    let mut default_headers = HeaderMap::new();
    default_headers.insert("Accept", "application/json".parse().unwrap());
    let http_client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .default_headers(default_headers)
        .user_agent(concat!(
            "minecraftserviceschecker/",
            env!("CARGO_PKG_VERSION"),
            " (https://github.com/randomairborne/mcping)"
        ))
        .redirect(reqwest::redirect::Policy::limited(100))
        .build()
        .unwrap();
    let current_mcstatus: Arc<RwLock<ServicesResponse>> =
        Arc::new(RwLock::new(get_mcstatus(http_client.clone()).await));
    tokio::spawn(refresh_mcstatus(http_client, Arc::clone(&current_mcstatus)));
    let serve_dir = ServeDir::new(&asset_dir)
        .append_index_html_on_directories(true)
        .precompressed_gzip()
        .precompressed_br()
        .precompressed_deflate()
        .precompressed_zstd();
    let app = axum::Router::new()
        .route("/api/:address", get(handle_java_ping))
        .route("/api/java/:address", get(handle_java_ping))
        .route("/api/bedrock/:address", get(handle_bedrock_ping))
        .route("/api/services", get(services::handle_mcstatus))
        .layer(axum::middleware::from_fn(noindex_cache))
        .fallback_service(serve_dir)
        .with_state(current_mcstatus);
    let socket_address = SocketAddr::from((
        [0, 0, 0, 0],
        std::env::var("PORT")
            .unwrap_or_else(|_| 8080.to_string())
            .parse::<u16>()
            .unwrap(),
    ));
    let tcp = TcpListener::bind(socket_address).await.unwrap();
    axum::serve(tcp, app)
        .with_graceful_shutdown(vss::shutdown_signal())
        .await
        .unwrap();
}
static ROBOTS_NAME: HeaderName = HeaderName::from_static("x-robots-tag");
static ROBOTS_VALUE: HeaderValue = HeaderValue::from_static("noindex");
static CACHE_CONTROL_AGE: HeaderValue = HeaderValue::from_static("s-maxage=30");

async fn noindex_cache(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    resp.headers_mut()
        .insert(ROBOTS_NAME.clone(), ROBOTS_VALUE.clone());
    resp.headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, CACHE_CONTROL_AGE.clone());
    resp
}

async fn handle_java_ping(Path(address): Path<String>) -> Result<impl IntoResponse, Failure> {
    let (latency, response) = match libmcping::tokio::get_status(Java {
        server_address: address,
        timeout: Some(std::time::Duration::from_secs(5)),
    })
    .await
    {
        Ok(ok) => ok,
        Err(e) => return Err(Failure::ConnectionFailed(e)),
    };
    let mut player_sample: Vec<PlayerSample> = Vec::new();
    if let Some(sample) = response.players.sample {
        for player in sample {
            player_sample.push(PlayerSample {
                uuid: player.id,
                name: player.name,
            });
        }
    }
    Ok(MCPingResponse {
        latency,
        players: Players {
            online: response.players.online,
            maximum: response.players.max,
            sample: player_sample,
        },
        motd: response.description.text().to_string(),
        icon: response.favicon,
        version: Version {
            protocol: response.version.protocol,
            broadcast: response.version.name,
        },
    })
}

async fn handle_bedrock_ping(Path(address): Path<String>) -> Result<impl IntoResponse, Failure> {
    let (latency, response) = match libmcping::tokio::get_status(Bedrock {
        server_address: address,
        timeout: Some(std::time::Duration::from_secs(5)),
        tries: 5,
        wait_to_try: Some(std::time::Duration::from_millis(100)),
        ..Default::default()
    })
    .await
    {
        Ok(ok) => ok,
        Err(e) => return Err(Failure::ConnectionFailed(e)),
    };
    Ok(MCPingResponse {
        latency,
        players: Players {
            online: response.players_online.unwrap_or(-1),
            maximum: response.players_max.unwrap_or(-1),
            sample: Vec::new(),
        },
        motd: response.motd_1,
        icon: None,
        version: Version {
            protocol: response.protocol_version.unwrap_or(-1),
            broadcast: response.version_name,
        },
    })
}

pub enum Failure {
    ConnectionFailed(libmcping::Error),
    StatusReqwestFailed(reqwest::Error),
    JsonSerializationFailed(serde_json::Error),
}

impl From<libmcping::Error> for Failure {
    fn from(e: libmcping::Error) -> Self {
        Self::ConnectionFailed(e)
    }
}

impl From<reqwest::Error> for Failure {
    fn from(e: reqwest::Error) -> Self {
        Self::StatusReqwestFailed(e)
    }
}

impl From<serde_json::Error> for Failure {
    fn from(e: serde_json::Error) -> Self {
        Self::JsonSerializationFailed(e)
    }
}

impl IntoResponse for Failure {
    fn into_response(self) -> axum::response::Response {
        let (error, status): (Cow<str>, StatusCode) = match self {
            Self::ConnectionFailed(e) => (
                format!("Error connecting to the server: {e}").into(),
                StatusCode::OK,
            ),
            Self::StatusReqwestFailed(e) => (
                format!("Error connecting to the Xbox or Mojang API: {e}").into(),
                StatusCode::BAD_GATEWAY,
            ),
            Self::JsonSerializationFailed(e) => (
                format!("Error serializing JSON: {e}").into(),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        };
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            println!("Error processing request: {error}");
        }
        axum::response::Response::builder()
            .header(
                axum::http::header::CONTENT_TYPE,
                HeaderValue::from_str("application/json").unwrap(),
            )
            .status(status)
            .body(axum::body::Body::new(
                serde_json::json!({ "error": error }).to_string(),
            ))
            .unwrap()
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
