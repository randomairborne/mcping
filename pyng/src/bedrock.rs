//! Implementation of the `RakNet` ping/pong protocol.
//! [Raknet: Unconnected Ping](https://wiki.vg/Raknet_Protocol#Unconnected_Ping)

use std::{
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

/// Raknets default `OFFLINE_MESSAGE_DATA_ID`.
/// See more: [Raknet: Data Types](https://wiki.vg/Raknet_Protocol#Data_types)
pub const OFFLINE_MESSAGE_DATA_ID: &[u8] = &[
    0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];

/// The default port of a Raknet Bedrock Server.
pub const DEFAULT_PORT: u16 = 19132;

/// Configuration for pinging a Bedrock server.
///
/// # Examples
///
/// ```
/// use pyng::Bedrock;
/// use std::time::Duration;
///
/// let bedrock_config = Bedrock {
///     server_address: "play.nethergames.org".to_string(),
///     timeout: Some(Duration::from_secs(10)),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Bedrock {
    /// The bedrock server address.
    ///
    /// This can be either an IP or a hostname, and both may optionally have a
    /// port at the end.
    ///
    /// DNS resolution will be performed on hostnames.
    ///
    /// # Examples
    ///
    /// ```text
    /// test.server.com
    /// test.server.com:19384
    /// 13.212.76.209
    /// 13.212.76.209:23193
    /// ```
    pub server_address: String,
    /// The read and write timeouts for the socket.
    pub timeout: Option<Duration>,
    /// The amount of times to try to send the ping packet.
    ///
    /// In case of packet loss an attempt can be made to send more than a single ping.
    pub tries: usize,
    /// The amount of time to wait in-between sending ping packets.
    pub wait_to_try: Option<Duration>,
    /// The socket addresses to try binding the UDP socket to.
    pub socket_addresses: Vec<SocketAddr>,
}

impl Default for Bedrock {
    fn default() -> Self {
        Self {
            server_address: String::new(),
            timeout: None,
            tries: 5,
            wait_to_try: Some(Duration::from_millis(10)),
            socket_addresses: vec![
                SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), 25567)),
                SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), 25568)),
                SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), 25569)),
            ],
        }
    }
}

/// Represents the edition of a bedrock server.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum BedrockEdition {
    PocketEdition,
    EducationEdition,
    /// An unknown edition string.
    Other(String),
}

impl std::fmt::Display for BedrockEdition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PocketEdition => f.write_str("MCPE"),
            Self::EducationEdition => f.write_str("MCEE"),
            Self::Other(s) => f.write_str(s),
        }
    }
}

impl From<String> for BedrockEdition {
    fn from(edition: String) -> Self {
        match edition.to_lowercase().as_ref() {
            "mcpe" => Self::PocketEdition,
            "mcee" => Self::EducationEdition,
            _ => Self::Other(edition),
        }
    }
}

/// Bedrock Server Payload Response
///
/// See More: [Raknet: Unconnected Pong](https://wiki.vg/Raknet_Protocol#Unconnected_Pong)
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct BedrockResponse {
    /// The server's edition.
    pub edition: BedrockEdition,
    /// The first line of the server's Message Of The Day (MOTD).
    ///
    /// In practice, this seems to be the only line that the bedrock clients
    /// display, and therefore the only line servers usually send.
    pub motd_1: String,
    /// The server's protocol version (ex: 390).
    pub protocol_version: Option<i64>,
    /// The name of the servers version (ex: 1.16.200).
    ///
    /// Bedrock clients display this after the first line of the MOTD, in the
    /// format `motd_1 - v{version_name}`. This is ommitted if no version name
    /// is in the response.
    pub version_name: String,
    /// The numbers of players online.
    pub players_online: Option<i64>,
    /// The maximum number of players that could be online at once.
    pub players_max: Option<i64>,
    /// The server UUID.
    pub server_id: Option<i64>,
    /// The second line of the server's MOTD.
    ///
    /// In practice, it looks like servers don't really use this. It seems to get
    /// used sometimes to communicate the server software being used (e.g.
    /// PocketMine-MP).
    pub motd_2: Option<String>,
    /// The game mode the server defaults new users to (e.g. "Survival").
    pub game_mode: Option<String>,
    /// The numerical representation of `game_mode` (e.g. "1").
    pub game_mode_id: Option<i64>,
    /// The port to connect to the server on with an IPv4 address.
    pub port_v4: Option<u16>,
    /// The port to connect to the server on with an IPv6 address.
    pub port_v6: Option<u16>,
}

impl BedrockResponse {
    /// Extracts information from the semicolon-separated payload.
    ///
    /// Edition (MCPE or MCEE for Education Edition)
    /// MOTD line 1
    /// Protocol Version
    /// Version Name
    /// Player Count
    /// Max Player Count
    /// Server Unique ID
    /// MOTD line 2
    /// Game mode
    /// Game mode (numeric)
    /// Port (IPv4)
    /// Port (IPv6)
    pub(crate) fn extract(payload: &str) -> Option<Self> {
        let mut parts = payload.split(';').map(ToString::to_string);

        Some(Self {
            edition: parts.next().map(BedrockEdition::from)?,
            motd_1: parts.next()?,
            protocol_version: parts.next().map(|s| s.parse().ok())?,
            version_name: parts.next()?,
            players_online: parts.next().and_then(|s| s.parse().ok()),
            players_max: parts.next().and_then(|s| s.parse().ok()),
            server_id: parts.next().and_then(|s| s.parse().ok()),
            motd_2: parts.next(),
            game_mode: parts.next(),
            game_mode_id: parts.next().and_then(|s| s.parse().ok()),
            port_v4: parts.next().and_then(|s| s.parse().ok()),
            port_v6: parts.next().and_then(|s| s.parse().ok()),
        })
    }
}

/// Represents a `RakNet` Unconnected Ping Protocol.
#[derive(Debug)]

pub enum Packet {
    UnconnectedPing,
    UnconnectedPong {
        #[allow(dead_code)]
        time: u64,
        #[allow(dead_code)]
        server_id: u64,
        payload: String,
    },
}
