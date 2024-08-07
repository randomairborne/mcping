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
#[serde(rename_all = "PascalCase")]
pub struct XblStatusResponse {
    pub status: XblStatusStatus,
    pub core_services: Vec<XblStatusCoreService>,
    pub titles: Vec<XblStatusCoreService>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct XblStatusStatus {
    pub overall: XblStatusStatusItem,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct XblStatusStatusItem {
    pub state: XblStatusName,
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
#[serde(rename_all = "PascalCase")]
pub struct XblStatusCoreServiceScenario {
    pub id: i64,
}

#[derive(Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(from = "String", into = "String")]
pub enum XblStatusName {
    Impacted,
    None,
    Unknown(String),
}

impl From<String> for XblStatusName {
    fn from(value: String) -> Self {
        match value.as_str() {
            "Impacted" => Self::Impacted,
            "None" => Self::None,
            _ => Self::Unknown(value),
        }
    }
}

impl From<XblStatusName> for String {
    fn from(value: XblStatusName) -> Self {
        match value {
            XblStatusName::Impacted => "Impacted".to_owned(),
            XblStatusName::None => "None".to_owned(),
            XblStatusName::Unknown(unknown) => unknown,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct XblStatusCoreServiceStatus {
    pub id: i64,
    pub name: XblStatusName,
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
    pub chat: ChatStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub struct ChatStatus {
    pub preview: bool,
    pub signing: bool,
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

#[derive(Clone, Copy, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub enum Status {
    Operational = 0,
    PossibleProblems = 1,
    DefiniteProblems = 2,
}

impl Status {
    pub fn make_at_least(&mut self, other: Self) {
        *self = std::cmp::max(*self, other);
    }
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
