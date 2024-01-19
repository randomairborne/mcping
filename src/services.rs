use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};
use reqwest::Client;
use tokio::{join, select, sync::RwLock};

use crate::{
    structures::{
        MinecraftApiStatusEntry, MojangApiStatus, MojangSessionServerStatus, ServicesResponse,
        Status, XblStatusResponse,
    },
    Failure,
};

const MOJANG_SESSIONSERVER_URL: &str =
    "https://sessionserver.mojang.com/session/minecraft/profile/b5dcf182a943402bb75ba057a6508fed";
const MOJANG_API_URL: &str = "https://api.mojang.com/users/profiles/minecraft/valkyrie_pilot";
const MINECRAFT_SERVICES_API_URL: &str =
    "https://api.minecraftservices.com/minecraft/profile/lookup/bulk/byname";
const XBL_STATUS_URL: &str = "https://xnotify.xboxlive.com/servicestatusv6/US/en-US";

#[allow(clippy::unused_async)]
pub async fn handle_mcstatus(
    State(state): State<Arc<RwLock<ServicesResponse>>>,
) -> Result<impl IntoResponse, Failure> {
    Ok((
        [("Content-Type", "application/json")],
        serde_json::to_string(&*state.read().await)?,
    ))
}

pub async fn get_mcstatus(http: Client) -> ServicesResponse {
    let (xbox, mojang_session, mojang_api, minecraft_api) = join!(
        get_xbox(http.clone()),
        get_session(http.clone()),
        get_mojang(http.clone()),
        get_minecraft(http.clone())
    );
    trace!(
        ?xbox,
        ?mojang_session,
        ?mojang_api,
        ?minecraft_api,
        "Got statuses"
    );
    ServicesResponse {
        xbox,
        mojang_session,
        mojang_api,
        minecraft_api,
    }
}

pub async fn refresh_mcstatus(http: Client, resp: Arc<RwLock<ServicesResponse>>) {
    loop {
        let sleep = tokio::time::sleep(std::time::Duration::from_secs(240));
        select! {
            _ = sleep => {},
            _ = vss::shutdown_signal() => break,
        }
        let status = get_mcstatus(http.clone()).await;
        let mut response = resp.write().await;
        *response = status;
    }
}

async fn get_xbox(client: Client) -> Status {
    let res = match client.get(XBL_STATUS_URL).send().await {
        Ok(v) => v,
        Err(e) => return Status::DefiniteProblems(Some(e)),
    };
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

async fn get_session(client: Client) -> Status {
    let res = match client.get(MOJANG_SESSIONSERVER_URL).send().await {
        Ok(v) => v,
        Err(e) => return Status::DefiniteProblems(Some(e)),
    };
    let result = match res.json::<MojangSessionServerStatus>().await {
        Ok(res) => res,
        Err(e) => return Status::PossibleProblems(Some(e)),
    };
    if result.name != "valkyrie_pilot" {
        return Status::DefiniteProblems(None);
    }
    Status::Operational
}

async fn get_mojang(client: Client) -> Status {
    let res = match client.get(MOJANG_API_URL).send().await {
        Ok(v) => v,
        Err(e) => return Status::DefiniteProblems(Some(e)),
    };
    let result = match res.json::<MojangApiStatus>().await {
        Ok(res) => res,
        Err(e) => return Status::PossibleProblems(Some(e)),
    };
    if result.id != "b5dcf182a943402bb75ba057a6508fed" {
        return Status::DefiniteProblems(None);
    }
    Status::Operational
}

async fn get_minecraft(client: Client) -> Status {
    let names = ["valkyrie_pilot", "pawlet", "lzaisanerd"];
    let res = match client
        .post(MINECRAFT_SERVICES_API_URL)
        .json(&names)
        .send()
        .await
    {
        Ok(v) => v,
        Err(e) => return Status::DefiniteProblems(Some(e)),
    };

    let expected = [
        MinecraftApiStatusEntry {
            id: "bbb47773bb48438e806b7731b2724e84".to_string(),
            name: "lzaisanerd".to_string(),
        },
        MinecraftApiStatusEntry {
            id: "c5ff333a8ef3423babac8d0338f731d5".to_string(),
            name: "pawlet".to_string(),
        },
        MinecraftApiStatusEntry {
            id: "b5dcf182a943402bb75ba057a6508fed".to_string(),
            name: "valkyrie_pilot".to_string(),
        },
    ];

    let mut data = match res.json::<Vec<MinecraftApiStatusEntry>>().await {
        Ok(v) => v,
        Err(e) => return Status::PossibleProblems(Some(e)),
    };
    data.sort_by(|a, b| a.name.cmp(&b.name));
    if data.as_slice() == expected {
        Status::Operational
    } else {
        Status::PossibleProblems(None)
    }
}
