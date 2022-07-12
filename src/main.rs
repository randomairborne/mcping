#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use axum::{extract::Path, http::StatusCode, response::IntoResponse, routing::get};
use mcping::{Bedrock, Java};
use std::borrow::Cow;

#[tokio::main]
async fn main() {
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
            "/api",
            get(|| async { ([("Content-Type", "text/html")], include_str!("../api.html")) }),
        )
        .route(
            "/api/",
            get(|| async { ([("Content-Type", "text/html")], include_str!("../api.html")) }),
        )
        .route("/api/:address", get(handle_java_ping))
        .route("/api/java/:address", get(handle_java_ping))
        .route("/api/bedrock/:address", get(handle_bedrock_ping));
    axum::Server::bind(&([0, 0, 0, 0], 8080).into())
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

#[derive(serde::Serialize, Debug, Clone)]
struct MCPingResponse {
    pub latency: u64,
    pub players: Players,
    pub motd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub version: Version,
}

#[derive(serde::Serialize, Debug, Clone)]
struct Version {
    pub protocol: i64,
    pub broadcast: String,
}

#[derive(serde::Serialize, Debug, Clone)]
struct Players {
    pub online: i64,
    pub maximum: i64,
    pub sample: Vec<PlayerSample>,
}

#[derive(serde::Serialize, Debug, Clone)]
struct PlayerSample {
    pub uuid: String,
    pub name: String,
}

enum Failure {
    ConnectionFailed(mcping::Error),
}

impl IntoResponse for Failure {
    fn into_response(self) -> axum::response::Response {
        let (error, status): (Cow<str>, StatusCode) = match self {
            Self::ConnectionFailed(e) => (
                format!("Error connecting to the server: {}", e).into(),
                StatusCode::OK,
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

impl IntoResponse for MCPingResponse {
    fn into_response(self) -> axum::response::Response {
        axum::response::Response::builder()
            .header(
                axum::http::header::CONTENT_TYPE,
                axum::headers::HeaderValue::from_static("application/json"),
            )
            .status(StatusCode::OK)
            .body(axum::body::boxed(axum::body::Full::from(
                serde_json::to_string(&self).unwrap_or_else(|_| r#"{"error": "Error serializing json! Please make a bug report: https://github.com/randomairborne/mcping/issues"}"#.to_string()),
            )))
            .unwrap()
    }
}
