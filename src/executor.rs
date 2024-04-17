use std::time::Duration;

use libmcping::{Bedrock, Java};
use tokio::select;

use crate::{
    structures::{ChatStatus, MCPingResponse, PlayerSample, Players, Version},
    Failure,
};

pub async fn ping_java(address: String) -> Result<MCPingResponse, Failure> {
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
        chat: ChatStatus {
            signing: response.enforces_secure_chat,
            preview: response.previews_chat,
        },
    })
}

pub async fn ping_bedrock(address: String) -> Result<MCPingResponse, Failure> {
    let (latency, response) = libmcping::tokio::get_status(Bedrock {
        server_address: address,
        timeout: Some(Duration::from_secs(5)),
        tries: 5,
        wait_to_try: Some(Duration::from_millis(100)),
        ..Default::default()
    })
    .await
    .map_err(Failure::ConnectionFailed)?;
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
        chat: ChatStatus::default(),
    })
}
