#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]
//! `mcping` is a Rust crate that provides Minecraft server ping protocol
//! implementations. It can be used to ping servers and collect information such
//! as the MOTD, max player count, online player sample, server icon, etc.
//!
//! The library supports both Java and Bedrock servers, and has comprehensive DNS
//! handling (such as SRV record lookup). An async implemention on top of the tokio
//! runtime is also provided.
//!
//! The main API surface is [`get_status`].

pub mod tokio;

mod bedrock;
mod java;

pub use bedrock::{Bedrock, BedrockResponse};
pub use java::{Chat, Java, JavaResponse, Player, Players, Version};

/// Errors that can occur when pinging a server.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("an invalid packet configuration was sent")]
    InvalidPacket,
    #[error("VarInt length was negative or too large")]
    InvalidVarInt(#[from] std::num::TryFromIntError),
    #[error("an I/O error occurred: {0}")]
    IoError(#[from] std::io::Error),
    #[error("a JSON error occurred: {0}")]
    JsonErr(#[from] serde_json::Error),
    #[error("an invalid address was provided")]
    InvalidAddress,
    #[error("DNS lookup for the host provided failed")]
    DnsLookupFailed,
}
