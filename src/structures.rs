use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Clone)]
pub struct ServicesResponse {
    #[serde(rename(serialize = "Xbox services"))]
    pub xbox: String,
    #[serde(rename(serialize = "AuthServer"))]
    pub mojang_auth: String,
    #[serde(rename(serialize = "SessionServer"))]
    pub mojang_session: String,
    #[serde(rename(serialize = "Mojang API"))]
    pub mojang_api: String,
    #[serde(rename(serialize = "Minecraft API"))]
    pub minecraft_api: String,
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
pub struct MojangAuthServerStatus {
    #[serde(rename(deserialize = "Status"))]
    pub status: String,
    #[serde(rename(deserialize = "Runtime-Mode"))]
    pub runtime_mode: String,
    #[serde(rename(deserialize = "Application-Author"))]
    pub application_author: String,
    #[serde(rename(deserialize = "Application-Description"))]
    pub application_description: String,
    #[serde(rename(deserialize = "Specification-Version"))]
    pub specification_version: String,
    #[serde(rename(deserialize = "Application-Name"))]
    pub application_name: String,
    #[serde(rename(deserialize = "Implementation-Version"))]
    pub implementation_version: String,
    #[serde(rename(deserialize = "Application-Owner"))]
    pub application_owner: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MojangSessionServerStatus {
    pub id: String,
    pub name: String,
    pub properties: Vec<MojangSessionServerProperties>
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

#[derive(Deserialize, Clone, Debug)]
pub struct MinecraftApiStatus {
    pub path: String,
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
                axum::headers::HeaderValue::from_static("application/json"),
            )
            .status(axum::http::StatusCode::OK)
            .body(axum::body::boxed(axum::body::Full::from(
                serde_json::to_string(&self).unwrap_or_else(|_| r#"{"error": "Error serializing json! Please make a bug report: https://github.com/randomairborne/mcping/issues"}"#.to_string()),
            )))
            .unwrap()
    }
}
