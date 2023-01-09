use axum::response::IntoResponse;
use reqwest::Response;
use std::fmt::{Debug, Display};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    structures::{
        MinecraftApiStatus, MojangApiStatus, MojangAuthServerStatus, MojangSessionServerStatus,
        ServicesResponse, XblStatusResponse,
    },
    Failure,
};

const MOJANG_AUTHSERVER_URL: &str = "https://authserver.mojang.com/";
const MOJANG_SESSIONSERVER_URL: &str =
    "https://sessionserver.mojang.com/session/minecraft/profile/b5dcf182a943402bb75ba057a6508fed";
const MOJANG_API_URL: &str = "https://api.mojang.com/users/profiles/minecraft/valkyrie_pilot";
const MINECRAFT_SERVICES_API_URL: &str = "https://api.minecraftservices.com/";
const XBL_STATUS_URL: &str = "https://xnotify.xboxlive.com/servicestatusv6/US/en-US";

#[allow(clippy::unused_async)]
pub async fn handle_mcstatus(
    resp: Arc<RwLock<ServicesResponse>>,
) -> Result<impl IntoResponse, Failure> {
    Ok((
        [("Content-Type", "application/json")],
        serde_json::to_string(&*resp.read().await)?,
    ))
}

pub async fn get_mcstatus(http: reqwest::Client, resp: Arc<RwLock<ServicesResponse>>) {
    let xbl = http
        .get(XBL_STATUS_URL)
        .header("Accept", "application/json")
        .send()
        .await;
    let mojang_auth = http
        .get(MOJANG_AUTHSERVER_URL)
        .header("Accept", "application/json")
        .send()
        .await;
    let mojang_session = http
        .get(MOJANG_SESSIONSERVER_URL)
        .header("Accept", "application/json")
        .send()
        .await;
    let mojang_api = http
        .get(MOJANG_API_URL)
        .header("Accept", "application/json")
        .send()
        .await;
    let minecraft_api = http
        .get(MINECRAFT_SERVICES_API_URL)
        .header("Accept", "application/json")
        .send()
        .await;
    let xbox = match xbl {
        Ok(r) => xbl_test(r).await,
        Err(e) => Status::DefiniteProblems(Some(e)),
    };
    let mojang_auth = match mojang_auth {
        Ok(r) => mojang_auth_test(r).await,
        Err(e) => Status::DefiniteProblems(Some(e)),
    };
    let mojang_session = match mojang_session {
        Ok(r) => mojang_session_test(r).await,
        Err(e) => Status::DefiniteProblems(Some(e)),
    };
    let mojang_api = match mojang_api {
        Ok(r) => mojang_api_test(r).await,
        Err(e) => Status::DefiniteProblems(Some(e)),
    };
    let minecraft_api = match minecraft_api {
        Ok(r) => minecraft_api_test(r).await,
        Err(e) => Status::DefiniteProblems(Some(e)),
    };
    let mut response = resp.write().await;
    *response = crate::structures::ServicesResponse {
        xbox: xbox.to_string(),
        mojang_auth: mojang_auth.to_string(),
        mojang_session: mojang_session.to_string(),
        mojang_api: mojang_api.to_string(),
        minecraft_api: minecraft_api.to_string(),
    }
}

pub async fn refresh_mcstatus(http: reqwest::Client, resp: Arc<RwLock<ServicesResponse>>) {
    tokio::time::sleep(std::time::Duration::from_secs(240)).await;
    get_mcstatus(http, resp).await;
}

async fn xbl_test(res: Response) -> Status {
    let result = match res.json::<XblStatusResponse>().await {
        Ok(res) => res,
        Err(e) => return Status::PossibleProblems(Some(e)),
    };
    if result.status.overall.state != "None" {
        return Status::PossibleProblems(None);
    }
    let minecraft_adjacent_services = [13, 16, 20, 22, 23, 24, 25];
    for service in result.core_services {
        if !minecraft_adjacent_services.contains(&service.id) {
            continue;
        }
        for scenario in service.possible_scenarios {
            if scenario.id == service.status.id {
                return Status::DefiniteProblems(None);
            }
        }
    }
    for service in result.titles {
        if !minecraft_adjacent_services.contains(&service.id) {
            continue;
        }
        for scenario in service.possible_scenarios {
            if scenario.id == service.status.id {
                return Status::DefiniteProblems(None);
            }
        }
    }
    Status::Operational
}
async fn mojang_auth_test(res: Response) -> Status {
    if let Err(e) = res.json::<MojangAuthServerStatus>().await {
        return Status::PossibleProblems(Some(e));
    };
    Status::Operational
}
async fn mojang_session_test(res: Response) -> Status {
    let result = match res.json::<MojangSessionServerStatus>().await {
        Ok(res) => res,
        Err(e) => return Status::PossibleProblems(Some(e)),
    };
    if result.name != "valkyrie_pilot" {
        return Status::DefiniteProblems(None);
    }
    Status::Operational
}
async fn mojang_api_test(res: Response) -> Status {
    let result = match res.json::<MojangApiStatus>().await {
        Ok(res) => res,
        Err(e) => return Status::PossibleProblems(Some(e)),
    };
    if result.id != "b5dcf182a943402bb75ba057a6508fed" {
        return Status::DefiniteProblems(None);
    }
    Status::Operational
}
async fn minecraft_api_test(res: Response) -> Status {
    if res.json::<MinecraftApiStatus>().await.is_err() {
        return Status::PossibleProblems(None);
    }
    // TODO: Set up MSA with an account so this can be better tested
    Status::Operational
}
enum Status {
    Operational,
    PossibleProblems(Option<reqwest::Error>),
    DefiniteProblems(Option<reqwest::Error>),
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
