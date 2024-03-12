#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
mod services;
mod structures;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::{Path, Request},
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE},
        HeaderName, HeaderValue, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Response},
    routing::get,
};
use libmcping::{Bedrock, Java};
use parking_lot::RwLock;
use reqwest::{header::HeaderMap, redirect::Policy, Client};
use serde::Serialize;
use tokio::{net::TcpListener, select};
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    services::{get_mcstatus, refresh_mcstatus},
    structures::{MCPingResponse, PlayerSample, Players, ServicesResponse, Version},
};

#[macro_use]
extern crate tracing;

const DEFAULT_PORT: u16 = 8080;

#[tokio::main]
async fn main() {
    start_tracing();
    let asset_dir = std::env::var("ASSET_DIR").unwrap_or_else(|_| "./assets/".to_owned());
    let port: u16 = std::env::var("PORT").map_or(DEFAULT_PORT, |v| v.parse().unwrap());
    let mut default_headers = HeaderMap::new();
    default_headers.insert("Accept", "application/json".parse().unwrap());
    let http_client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .default_headers(default_headers)
        .user_agent(concat!(
            "minecraftserviceschecker/",
            env!("CARGO_PKG_VERSION"),
            " (https://github.com/randomairborne/mcping)"
        ))
        .redirect(Policy::limited(100))
        .build()
        .unwrap();
    let current_mcstatus: Arc<RwLock<ServicesResponse>> =
        Arc::new(RwLock::new(get_mcstatus(http_client.clone()).await));
    tokio::spawn(refresh_mcstatus(http_client, Arc::clone(&current_mcstatus)));

    let serve_404 = ServeFile::new(asset_dir.trim_end_matches('/').to_string() + "/404.html");
    let serve_dir = ServeDir::new(&asset_dir)
        .append_index_html_on_directories(true)
        .precompressed_gzip()
        .precompressed_br()
        .precompressed_deflate()
        .precompressed_zstd()
        .fallback(serve_404);
    let app = axum::Router::new()
        .route("/api/:address", get(handle_java_ping))
        .route("/api/java/:address", get(handle_java_ping))
        .route("/api/bedrock/:address", get(handle_bedrock_ping))
        .route("/api/services", get(services::handle_mcstatus))
        .layer(axum::middleware::from_fn(noindex_cache))
        .fallback_service(serve_dir)
        .with_state(current_mcstatus);
    let socket_address = SocketAddr::from(([0, 0, 0, 0], port));
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
        .insert(CACHE_CONTROL, CACHE_CONTROL_AGE.clone());
    resp
}

async fn handle_java_ping(Path(address): Path<String>) -> Result<Json<MCPingResponse>, Failure> {
    let ping_future = libmcping::tokio::get_status(Java {
        server_address: address,
        timeout: Some(Duration::from_secs(1)),
    });
    let sleep_future = tokio::time::sleep(Duration::from_secs(5));
    #[allow(clippy::redundant_pub_crate)]
    let (latency, response) = select! {
        val = ping_future => val?,
        () = sleep_future => return Err(Failure::TimedOut),
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
    Ok(Json(MCPingResponse {
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
    }))
}

async fn handle_bedrock_ping(Path(address): Path<String>) -> Result<Json<MCPingResponse>, Failure> {
    let (latency, response) = libmcping::tokio::get_status(Bedrock {
        server_address: address,
        timeout: Some(Duration::from_secs(5)),
        tries: 5,
        wait_to_try: Some(Duration::from_millis(100)),
        ..Default::default()
    })
    .await
    .map_err(Failure::ConnectionFailed)?;
    Ok(Json(MCPingResponse {
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
    }))
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
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let status = match self {
            Self::ConnectionFailed(_) | Self::TimedOut => StatusCode::OK,
            Self::StatusReqwestFailed(_) => StatusCode::BAD_GATEWAY,
            Self::JsonProcessingFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        error!(error = ?self, "Error processing request");
        let ser = ErrorSerialization {
            error: self.to_string(),
        };
        (status, Json(ser)).into_response()
    }
}

#[derive(serde::Serialize)]
pub struct ErrorSerialization {
    error: String,
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
