//! Implementation of the Java Minecraft ping protocol.
//! https://wiki.vg/Server_List_Ping

use std::{
    io::{self, Read, Write},
    time::Duration,
};

use byteorder::{ReadBytesExt, WriteBytesExt};
use serde::Deserialize;
use thiserror::Error;

/// Configuration for pinging a Java server.
///
/// # Examples
///
/// ```
/// use mcping::Java;
/// use std::time::Duration;
///
/// let bedrock_config = Java {
///     server_address: "mc.hypixel.net".to_string(),
///     timeout: Some(Duration::from_secs(10)),
/// };
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Java {
    /// The java server address.
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
    /// The connection timeout if a connection cannot be made.
    pub timeout: Option<Duration>,
}

#[derive(Deserialize)]
pub struct ForgeModMetadata {
    pub modid: String,
    pub version: String,
}

#[derive(Deserialize)]
pub struct ForgeModInfoList {
    #[serde(rename = "modList")]
    pub mod_list: Vec<ForgeModMetadata>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ModInfo {
    #[serde(rename = "FML")]
    Fml(ForgeModInfoList),
}

/// The server status reponse
///
/// More information can be found [here](https://wiki.vg/Server_List_Ping).
#[derive(Deserialize)]
pub struct JavaResponse {
    /// The version of the server.
    pub version: Version,
    /// Information about online players
    pub players: Players,
    /// The description of the server (MOTD).
    pub description: Chat,
    /// The server icon (a Base64-encoded PNG image)
    pub favicon: Option<String>,
    /// Mod information
    pub modinfo: Option<ModInfo>,
    /// Does this server enforce server signing?
    #[serde(rename = "enforcesSecureChat")]
    pub enforces_secure_chat: Option<bool>,
    /// Does this server have chat previews?
    #[serde(rename = "previewsChat")]
    pub previews_chat: Option<bool>,
}

/// Information about the server's version
#[derive(Deserialize)]
pub struct Version {
    /// The name of the version the server is running
    ///
    /// In practice this comes in a large variety of different formats.
    pub name: String,
    /// See https://wiki.vg/Protocol_version_numbers
    pub protocol: i64,
}

/// An online player of the server.
#[derive(Deserialize)]
pub struct Player {
    /// The name of the player.
    pub name: String,
    /// The player's UUID
    pub id: String,
}

/// The stats for players on the server.
#[derive(Deserialize)]
pub struct Players {
    /// The max amount of players.
    pub max: i64,
    /// The amount of players online.
    pub online: i64,
    /// A preview of which players are online
    ///
    /// In practice servers often don't send this or use it for more advertising
    pub sample: Option<Vec<Player>>,
}

/// This is a partial implemenation of a Minecraft chat component limited to just text
// TODO: Finish this object.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum Chat {
    Text { text: String },
    String(String),
}

impl Chat {
    pub fn text(&self) -> &str {
        match self {
            Chat::Text { text } => text.as_str(),
            Chat::String(s) => s.as_str(),
        }
    }
}

trait ReadJavaExt: Read + ReadBytesExt {
    fn read_varint(&mut self) -> io::Result<i32> {
        let mut res = 0i32;
        for i in 0..5 {
            let part = self.read_u8()?;
            res |= (part as i32 & 0x7F) << (7 * i);
            if part & 0x80 == 0 {
                return Ok(res);
            }
        }
        Err(io::Error::new(io::ErrorKind::Other, "VarInt too big!"))
    }

    fn read_string(&mut self) -> io::Result<String> {
        let len = self.read_varint()? as usize;
        let mut buf = vec![0; len];
        self.read_exact(&mut buf)?;
        Ok(String::from_utf8(buf).expect("Invalid UTF-8 String."))
    }
}

impl<T> ReadJavaExt for T where T: Read + ReadBytesExt {}

trait WriteJavaExt: Write + WriteBytesExt {
    fn write_varint(&mut self, mut val: i32) -> io::Result<()> {
        for _ in 0..5 {
            if val & !0x7F == 0 {
                self.write_u8(val as u8)?;
                return Ok(());
            }
            self.write_u8((val & 0x7F | 0x80) as u8)?;
            val >>= 7;
        }
        Err(io::Error::new(io::ErrorKind::Other, "VarInt too big!"))
    }

    fn write_string(&mut self, s: &str) -> io::Result<()> {
        self.write_varint(s.len() as i32)?;
        self.write_all(s.as_bytes())?;
        Ok(())
    }
}

impl<T> WriteJavaExt for T where T: Write + WriteBytesExt {}

#[derive(Debug, Error)]
#[error("invalid packet response `{packet:?}`")]
pub struct InvalidPacket {
    packet: Packet,
}

#[derive(Debug)]
pub(crate) enum Packet {
    Handshake {
        version: i32,
        host: String,
        port: u16,
        next_state: i32,
    },
    Response {
        response: String,
    },
    Pong {
        payload: u64,
    },
    Request {},
    Ping {
        payload: u64,
    },
}
