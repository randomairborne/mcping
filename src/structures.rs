use std::fmt::{Debug, Display};

use serde::{Deserialize, Serialize, Serializer};

#[derive(Serialize, Debug)]
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
    #[serde(rename(deserialize = "SelectedScenarios"))]
    pub selected_scenarios: XblStatusStatusItem,
}

#[derive(Deserialize, Clone, Debug)]
pub struct XblStatusStatusItem {
    #[serde(rename(deserialize = "State"))]
    pub state: String,
    #[serde(rename(deserialize = "Id"))]
    pub id: i64,
    #[serde(rename(deserialize = "LastUpdated"))]
    pub last_updated: String,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Deserialize, Clone, Debug)]
pub struct XblStatusCoreService {
    #[serde(rename(deserialize = "Id"))]
    pub id: i64,
    #[serde(rename(deserialize = "Name"))]
    pub name: String,
    #[serde(rename(deserialize = "Status"))]
    pub status: XblStatusCoreServiceStatus,
    #[serde(rename(deserialize = "Scenarios"))]
    pub possible_scenarios: Vec<XblStatusCoreServiceScenario>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct XblStatusCoreServiceScenario {
    #[serde(rename(deserialize = "Id"))]
    pub id: i64,
    #[serde(rename(deserialize = "Status"))]
    pub status: XblStatusCoreServiceStatus,
    #[serde(rename(deserialize = "Name"))]
    pub name: String,
    #[serde(rename(deserialize = "Devices"))]
    pub devices: Vec<XblStatusCoreServiceStatus>,
    #[serde(rename(deserialize = "Incidents"))]
    pub incidents: Vec<String>,
    #[serde(rename(deserialize = "Description"))]
    pub description: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct XblStatusCoreServiceStatus {
    #[serde(rename(deserialize = "Name"))]
    pub name: String,
    #[serde(rename(deserialize = "Id"))]
    pub id: i64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MojangSessionServerStatus {
    pub id: String,
    pub name: String,
    pub properties: Vec<MojangSessionServerProperties>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MojangSessionServerProperties {
    pub name: String,
    pub value: String,
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

impl axum::response::IntoResponse for MCPingResponse {
    fn into_response(self) -> axum::response::Response {
        axum::response::Response::builder()
            .header(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("application/json"),
            )
            .status(axum::http::StatusCode::OK)
            .body(axum::body::Body::from(
                serde_json::to_string(&self).unwrap_or_else(|_| r#"{"error": "Error serializing json! Please make a bug report: https://github.com/randomairborne/mcping/issues"}"#.to_string()),
            ))
            .unwrap()
    }
}

pub enum Status {
    Operational,
    PossibleProblems(Option<reqwest::Error>),
    DefiniteProblems(Option<reqwest::Error>),
}

impl Serialize for Status {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ser = match self {
            Status::Operational => "Operational",
            Status::PossibleProblems(_) => "PossibleProblems",
            Status::DefiniteProblems(_) => "DefiniteProblems",
        };
        serializer.serialize_str(ser)
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operational => write!(f, "Operational"),
            Self::PossibleProblems(_) => write!(f, "PossibleProblems"),
            Self::DefiniteProblems(_) => write!(f, "DefiniteProblems"),
        }
    }
}

impl Debug for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operational => write!(f, "Operational"),
            Self::PossibleProblems(e) => write!(f, "PossibleProblems: {e:?}"),
            Self::DefiniteProblems(e) => write!(f, "DefiniteProblems: {e:?}"),
        }
    }
}
