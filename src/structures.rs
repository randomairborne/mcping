use std::fmt::{Debug, Display};

use serde::{Deserialize, Serialize, Serializer};

#[derive(Serialize, Debug, Clone, Copy)]
pub struct ServicesResponse {
    #[serde(rename(serialize = "Xbox services"))]
    pub xbox: Status,
    #[serde(rename(serialize = "SessionServer"))]
    pub mojang_session: Status,
    #[serde(rename(serialize = "Mojang API"))]
    pub mojang_api: Status,
    #[serde(rename(serialize = "Minecraft API"))]
    pub minecraft_api: Status,
}

#[derive(Deserialize, Clone, Debug)]
pub struct XblStatusResponse {
    #[serde(rename(deserialize = "Status"))]
    pub status: XblStatusStatus,
    #[serde(rename(deserialize = "CoreServices"))]
    pub core_services: Vec<XblStatusCoreService>,
    #[serde(rename(deserialize = "Titles"))]
    pub titles: Vec<XblStatusCoreService>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct XblStatusStatus {
    #[serde(rename(deserialize = "Overall"))]
    pub overall: XblStatusStatusItem,
}

#[derive(Deserialize, Clone, Debug)]
pub struct XblStatusStatusItem {
    #[serde(rename(deserialize = "State"))]
    pub state: String,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Deserialize, Clone, Debug)]
pub struct XblStatusCoreService {
    #[serde(rename(deserialize = "Id"))]
    pub id: i64,
    #[serde(rename(deserialize = "Status"))]
    pub status: XblStatusCoreServiceStatus,
    #[serde(rename(deserialize = "Scenarios"))]
    pub possible_scenarios: Vec<XblStatusCoreServiceScenario>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct XblStatusCoreServiceScenario {
    #[serde(rename(deserialize = "Id"))]
    pub id: i64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct XblStatusCoreServiceStatus {
    #[serde(rename(deserialize = "Id"))]
    pub id: i64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MojangSessionServerStatus {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MojangApiStatus {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct MinecraftApiStatusEntry {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MCPingResponse {
    pub latency: u64,
    pub players: Players,
    pub motd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub version: Version,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Version {
    pub protocol: i64,
    pub broadcast: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Players {
    pub online: i64,
    pub maximum: i64,
    pub sample: Vec<PlayerSample>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerSample {
    pub uuid: String,
    pub name: String,
}

#[derive(Clone, Copy, Debug)]
pub enum Status {
    Operational,
    PossibleProblems,
    DefiniteProblems,
}

impl Serialize for Status {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operational => write!(f, "Operational"),
            Self::PossibleProblems => write!(f, "PossibleProblems"),
            Self::DefiniteProblems => write!(f, "DefiniteProblems"),
        }
    }
}
