function gebi(elId) {
  return document.getElementById(elId);
}

const serverStatusElement = gebi("server-status");
const playersElement = gebi("server-players");
const faviconElement = gebi("server-favicon");
const latencyElement = gebi("server-latency");
const versionElement = gebi("server-version");
const ipElement = gebi("user-ip");
const ipMsgElement = gebi("ip-msg");
const ipDescriptorElement = gebi("ip-descriptor");
const motdElement = gebi("server-motd");
const addressEntry = gebi("address-entry");
const selectElement = gebi("select-ping");
const responseElement = gebi("server-response");
const resetPingElement = gebi("reset-ping");
const javaTriggerElement = gebi("java-btn");
const bedrockTriggerElement = gebi("bedrock-btn");
const xblApiStatusElement = gebi("api-status-xbl");
const mojangApiStatusElement = gebi("api-status-mjapi");
const mojangSessionServerStatusElement = gebi("api-status-mjss");
const minecraftApiStatusElement = gebi("api-status-mcapi");
const apiStatusElements = [
  xblApiStatusElement,
  mojangApiStatusElement,
  mojangSessionServerStatusElement,
  minecraftApiStatusElement,
];

function ipClick() {
  ipElement.hidden = !ipElement.hidden;
  ipMsgElement.hidden = !ipMsgElement.hidden;
}

ipElement.addEventListener("click", ipClick);
ipMsgElement.addEventListener("click", ipClick);
ipDescriptorElement.addEventListener("click", ipClick);

fetch("https://v4.giveip.io/raw")
  .then((rsp) => rsp.text())
  .then((s) => {
    ipElement.innerText = s.trim();
  });

resetPingElement.addEventListener("click", function (_) {
  selectElement.hidden = false;
  responseElement.hidden = true;
  motdElement.textContent = "";
  versionElement.textContent = "";
});
javaTriggerElement.addEventListener("click", function (_) {
  doPing("/api/java/").then(() => {});
});
bedrockTriggerElement.addEventListener("click", function (_) {
  doPing("/api/bedrock/").then(() => {});
});

async function doPing(apiLocation) {
  let address = addressEntry.value;
  serverStatusElement.innerHTML = "Pinging...";
  let response = await fetch(apiLocation + address, {}).then((response) =>
    response.json(),
  );
  if (response["error"] !== undefined) {
    serverStatusElement.innerText = response["error"];
    return;
  }
  if (response["icon"] === "" || response["icon"] === undefined) {
    faviconElement.src = "/icon.png";
  } else {
    faviconElement.src = response["icon"];
  }
  latencyElement.innerText =
    "Ping (Toronto): " + response["latency"] + " milliseconds";
  playersElement.innerText =
    "Players: " +
    response["players"]["online"] +
    " / " +
    response["players"]["maximum"];
  motdElement.append(...highlightMotd(response["motd"]));
  versionElement.append(...highlightMotd(response["version"]["broadcast"]));
  selectElement.hidden = true;
  responseElement.hidden = false;
  serverStatusElement.innerHTML = null;
}

async function doAutoPing() {
  let windowHash = window.location.hash.substring(1);
  let [actionString, edition, hostname] = windowHash.split(";", 3);
  if (actionString === "ping") {
    addressEntry.value = hostname;
    if (edition.startsWith("b")) {
      await doPing("/api/bedrock/");
    } else if (edition.startsWith("j")) {
      await doPing("/api/java/");
    }
  }
}

doAutoPing().then(() => {});

async function checkMojangStatus() {
  const response = await fetch("/api/services", {}).then((response) => {
    return response.json();
  });
  for (let i = 0; i < apiStatusElements.length; i++) {
    const statusElement = apiStatusElements[i];
    switch (response[statusElement.dataset.field]) {
      case "Operational": {
        statusElement.classList.add("green");
        statusElement.textContent = "OK";
        break;
      }
      case "PossibleProblems": {
        statusElement.classList.add("yellow");
        statusElement.textContent = "Flaky";
        break;
      }
      case "DefiniteProblems": {
        statusElement.classList.add("red");
        statusElement.textContent = "Down";
        break;
      }
      default: {
        statusElement.classList.add("blue");
        statusElement.textContent = "Unknown";
      }
    }
  }
}

checkMojangStatus().then(() => {});

function highlightMotd(motd) {
  const SECTION = "§";
  let output = [];
  let lastColor = "";
  let alternateStyling = [];
  let lastStart = 0;
  const isCharCode = /([a-f]|[0-9])/;
  let lastCharWasSection = false;
  for (let i = 0; i < motd.length; i++) {
    const next = motd.charAt(i);
    if (next === SECTION) {
      output.push(
        getNextElement(lastColor, alternateStyling, motd, lastStart, i),
      );
      lastCharWasSection = true;
      continue;
    }
    if (next === "\n") {
      output.push(
        getNextElement(lastColor, alternateStyling, motd, lastStart, i),
      );
      const br = document.createElement("br");
      output.push(br);
      lastStart = i;
    }
    if (lastCharWasSection) {
      if (next.match(isCharCode)) {
        lastColor = `motd-style-${next}`;
      } else if (next === "r") {
        lastColor = "";
        alternateStyling = [];
      } else {
        alternateStyling.push(`motd-style-${next}`);
      }
      lastStart = i + 1;
    }
    lastCharWasSection = false;
  }
  output.push(
    getNextElement(lastColor, alternateStyling, motd, lastStart, motd.length),
  );
  return output;
}

function getNextElement(lastColor, alternateStyling, motd, lastStart, i) {
  const nextEl = document.createElement("span");
  if (lastColor !== "") {
    nextEl.classList.add(lastColor);
  }
  nextEl.classList.add(...alternateStyling);
  nextEl.textContent = motd.substring(lastStart, i);
  return nextEl;
}
