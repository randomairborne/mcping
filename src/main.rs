#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
mod services;
mod structures;
use crate::structures::{MCPingResponse, PlayerSample, Players, Version};
use axum::{extract::Path, http::StatusCode, response::IntoResponse, routing::get};
use mcping::{Bedrock, Java};
use services::{get_mcstatus, refresh_mcstatus};
use std::{borrow::Cow, sync::Arc};
use structures::ServicesResponse;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let http_client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .user_agent(concat!(
            "minecraftserviceschecker/",
            env!("CARGO_PKG_VERSION"),
            " (https://github.com/randomairborne/mcping)"
        ))
        .redirect(reqwest::redirect::Policy::limited(100))
        .build()
        .unwrap();
    let current_mcstatus: Arc<RwLock<ServicesResponse>> = Arc::new(RwLock::new(ServicesResponse {
        xbox: "".to_string(),
        mojang_auth: "".to_string(),
        mojang_session: "".to_string(),
        mojang_api: "".to_string(),
        minecraft_api: "".to_string(),
    }));
    get_mcstatus(http_client.clone(), Arc::clone(&current_mcstatus)).await;
    tokio::spawn(refresh_mcstatus(http_client, Arc::clone(&current_mcstatus)));
    let app = axum::Router::new()
        .route(
            "/",
            get(|| async {
                (
                    [("Content-Type", "text/html")],
                    include_str!("../ping.html"),
                )
            }),
        )
        .route(
            "/icon.png",
            get(|| async {
                (
                    [("Content-Type", "image/png")],
                    include_bytes!("../icon.png").to_vec(),
                )
            }),
        )
        .route(
            "/jetbrains.woff2",
            get(|| async {
                (
                    [("Content-Type", "font/woff2")],
                    include_bytes!("../jetbrains.woff2").to_vec(),
                )
            }),
        )
        .route(
            "/api",
            get(|| async { ([("Content-Type", "text/html")], include_str!("../api.html")) }),
        )
        .route(
            "/api/",
            get(|| async { ([("Content-Type", "text/html")], include_str!("../api.html")) }),
        )
        .route("/api/:address", get(handle_java_ping))
        .route("/api/java/:address", get(handle_java_ping))
        .route("/api/bedrock/:address", get(handle_bedrock_ping))
        .route(
            "/api/services",
            get({
                let current_mcstatus = Arc::clone(&current_mcstatus);
                move || services::handle_mcstatus(Arc::clone(&current_mcstatus))
            }),
        );
    axum::Server::bind(
        &(
            [0, 0, 0, 0],
            std::env::var("PORT")
                .unwrap_or_else(|_| 8080.to_string())
                .parse::<u16>()
                .unwrap_or(8080),
        )
            .into(),
    )
    .serve(app.into_make_service())
    .await
    .unwrap();
}

async fn handle_java_ping(Path(address): Path<String>) -> Result<impl IntoResponse, Failure> {
    let (latency, response) = match mcping::tokio::get_status(Java {
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
    let (latency, response) = match mcping::tokio::get_status(Bedrock {
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
    ConnectionFailed(mcping::Error),
    StatusReqwestFailed(reqwest::Error),
    JsonSerializationFailed(serde_json::Error),
}

impl From<mcping::Error> for Failure {
    fn from(e: mcping::Error) -> Self {
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
                format!("Error connecting to the server: {}", e).into(),
                StatusCode::OK,
            ),
            Self::StatusReqwestFailed(e) => (
                format!("Error connecting to the Xbox or Mojang API: {}", e).into(),
                StatusCode::BAD_GATEWAY,
            ),
            Self::JsonSerializationFailed(e) => (
                format!("Error serializing JSON: {}", e).into(),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        };
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            println!("Error processing request: {}", error);
        }
        axum::response::Response::builder()
            .header(
                axum::http::header::CONTENT_TYPE,
                axum::headers::HeaderValue::from_static("application/json"),
            )
            .status(status)
            .body(axum::body::boxed(axum::body::Full::from(
                serde_json::json!({ "error": error }).to_string(),
            )))
            .unwrap()
    }
}
