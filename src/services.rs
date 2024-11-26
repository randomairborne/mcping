use std::{pin::pin, sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use axum::extract::State;
use futures_util::future::Either;
use reqwest::Client;
use tokio::join;
use tokio_util::sync::CancellationToken;

use crate::{
    structures::{
        MinecraftApiStatusEntry, MojangApiStatus, MojangSessionServerStatus, ServicesResponse,
        Status, XblStatusName, XblStatusResponse,
    },
    AppState, Json,
};

const MOJANG_SESSIONSERVER_URL: &str =
    "https://sessionserver.mojang.com/session/minecraft/profile/bbb47773bb48438e806b7731b2724e84";
const MOJANG_API_URL: &str = "https://api.mojang.com/users/profiles/minecraft/mcping_me";
const MINECRAFT_SERVICES_API_URL: &str =
    "https://api.minecraftservices.com/minecraft/profile/lookup/bulk/byname";
const XBL_STATUS_URL: &str = "https://xnotify.xboxlive.com/servicestatusv6/US/en-US";

pub async fn handle_mcstatus(State(state): State<AppState>) -> Json<ServicesResponse> {
    Json(**state.svc_response.load())
}

pub async fn get_mcstatus(http: Client) -> ServicesResponse {
    let (xbox, mojang_session, mojang_api, minecraft_api) = join!(
        get_xbox(http.clone()),
        get_session(http.clone()),
        get_mojang(http.clone()),
        get_minecraft(http.clone())
    );
    debug!(
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

const MOJANG_API_REFRESH: Duration = Duration::from_secs(240);

pub async fn refresh_mcstatus(
    http: Client,
    status: Arc<ArcSwap<ServicesResponse>>,
    shutdown: CancellationToken,
) {
    loop {
        let sleep = tokio::time::sleep(MOJANG_API_REFRESH);
        match futures_util::future::select(pin!(sleep), pin!(shutdown.cancelled())).await {
            Either::Left(_) => {}
            Either::Right(_) => break,
        }
        let new_status = Arc::new(get_mcstatus(http.clone()).await);
        status.store(new_status);
    }
}

async fn get_xbox(client: Client) -> Status {
    let res = match client.get(XBL_STATUS_URL).send().await {
        Ok(v) => v,
        Err(source) => {
            warn!(?source, "Could not reach XBL status url");
            return Status::DefiniteProblems;
        }
    };
    let result = match res.json::<XblStatusResponse>().await {
        Ok(res) => res,
        Err(source) => {
            warn!(?source, "Could not decode JSON from XBL api");
            return Status::PossibleProblems;
        }
    };
    let mut status = Status::Operational;
    if result.status.overall.state != XblStatusName::None {
        warn!("overall status was not None");
        status.make_at_least(Status::PossibleProblems);
    }
    let minecraft_adjacent_services = [13, 16, 20, 22, 23, 24, 25];
    for service in result.core_services.iter().chain(result.titles.iter()) {
        if !minecraft_adjacent_services.contains(&service.id) {
            continue;
        }
        match &service.status.name {
            XblStatusName::Impacted => status.make_at_least(Status::DefiniteProblems),
            XblStatusName::None => {}
            XblStatusName::Unknown(q_status) => {
                warn!(status = q_status, "Unknown XBL status returned");
                status.make_at_least(Status::PossibleProblems);
            }
        }
        for scenario in &service.possible_scenarios {
            if scenario.id == service.status.id {
                warn!(id = scenario.id, "Got report of XBL problem from XBL api");
                status.make_at_least(Status::DefiniteProblems);
            }
        }
    }
    status
}

async fn get_session(client: Client) -> Status {
    let res = match client.get(MOJANG_SESSIONSERVER_URL).send().await {
        Ok(v) => v,
        Err(source) => {
            warn!(?source, "Could not reach Mojang session-server url");
            return Status::DefiniteProblems;
        }
    };
    let result = match res.json::<MojangSessionServerStatus>().await {
        Ok(res) => res,
        Err(source) => {
            warn!(?source, "Could not decode Mojang session-server JSON");
            return Status::PossibleProblems;
        }
    };
    if result.name != "mcping_me" || result.id != "bbb47773bb48438e806b7731b2724e84" {
        Status::DefiniteProblems
    } else {
        Status::Operational
    }
}

async fn get_mojang(client: Client) -> Status {
    let res = match client.get(MOJANG_API_URL).send().await {
        Ok(v) => v,
        Err(source) => {
            warn!(?source, "Could not reach Mojang API url");
            return Status::DefiniteProblems;
        }
    };
    let result = match res.json::<MojangApiStatus>().await {
        Ok(res) => res,
        Err(source) => {
            warn!(?source, "Could not decode Mojang API JSON");
            return Status::PossibleProblems;
        }
    };
    if result.name != "mcping_me" || result.id != "bbb47773bb48438e806b7731b2724e84" {
        Status::DefiniteProblems
    } else {
        Status::Operational
    }
}

async fn get_minecraft(client: Client) -> Status {
    let names = ["valkyrie_pilot", "pawlet", "mcping_me"];
    let res = match client
        .post(MINECRAFT_SERVICES_API_URL)
        .json(&names)
        .send()
        .await
    {
        Ok(v) => v,
        Err(source) => {
            warn!(?source, "Could not reach Minecraft API URL");
            return Status::DefiniteProblems;
        }
    };

    let expected = [
        MinecraftApiStatusEntry {
            id: "bbb47773bb48438e806b7731b2724e84".to_string(),
            name: "mcping_me".to_string(),
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
        Err(source) => {
            warn!(?source, "Could not decode Minecraft API JSON");
            return Status::PossibleProblems;
        }
    };
    data.sort_by(|a, b| a.name.cmp(&b.name));
    if data.as_slice() == expected {
        Status::Operational
    } else {
        warn!(expected = ?expected, ?data, "Got non-matching Minecraft API data");
        Status::PossibleProblems
    }
}
