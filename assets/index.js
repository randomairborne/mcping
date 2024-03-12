const serverStatusElement = document.getElementById("server-status");
const playersElement = document.getElementById("server-players");
const faviconElement = document.getElementById("server-favicon");
const latencyElement = document.getElementById("server-latency");
const versionElement = document.getElementById("server-version");
const ipElement = document.getElementById("user-ip");
const ipMsgElement = document.getElementById("ip-msg");
const ipDescriptorElement = document.getElementById("ip-descriptor");
const motdElement = document.getElementById("server-motd");
const specialBreak = document.getElementById("special-break");
const addressEntry = document.getElementById("address-entry");
const selectElement = document.getElementById("select-ping");
const responseElement = document.getElementById("server-response");
const resetPingElement = document.getElementById("reset-ping");
const javaTriggerElement = document.getElementById("java-btn");
const bedrockTriggerElement = document.getElementById("bedrock-btn");
const xblApiStatusElement = document.getElementById("api-status-xbl");
const mojangApiStatusElement = document.getElementById("api-status-mjapi");
const mojangSessionServerStatusElement =
  document.getElementById("api-status-mjss");
const minecraftApiStatusElement = document.getElementById("api-status-mcapi");
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
  // TODO: this is stupid, should actually reset everything instead
  window.location.reload();
});
javaTriggerElement.addEventListener("click", function (_) {
  doPing("/api/java/").then(() => {});
});
bedrockTriggerElement.addEventListener("click", function (_) {
  doPing("/api/bedrock/").then(() => {});
});

async function doPing(apiLocation) {
  let address = addressEntry.value;
  specialBreak.hidden = false;
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
    "/" +
    response["players"]["maximum"];
  motdElement.innerHTML = mineParse(response["motd"]).raw;
  versionElement.innerHTML = mineParse(response["version"]["broadcast"]).raw;
  selectElement.hidden = true;
  responseElement.hidden = false;
}

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

(function () {
  "use strict";

  var currId = 0,
    obfuscators = {},
    alreadyParsed = [],
    styleMap = {
      "§0": "color:#000000",
      "§1": "color:#0000AA",
      "§2": "color:#00AA00",
      "§3": "color:#00AAAA",
      "§4": "color:#AA0000",
      "§5": "color:#AA00AA",
      "§6": "color:#FFAA00",
      "§7": "color:#AAAAAA",
      "§8": "color:#555555",
      "§9": "color:#5555FF",
      "§a": "color:#55FF55",
      "§b": "color:#55FFFF",
      "§c": "color:#FF5555",
      "§d": "color:#FF55FF",
      "§e": "color:#FFFF55",
      "§f": "color:#FFFFFF",
      "§l": "font-weight:bold",
      "§m": "text-decoration:line-through",
      "§n": "text-decoration:underline",
      "§o": "font-style:italic",
    };

  function obfuscate(elem, string) {
    var multiMagic, currNode, listLen, nodeI;

    function randInt(min, max) {
      return Math.floor(Math.random() * (max - min + 1)) + min;
    }

    function replaceRand(string, i) {
      var randChar = String.fromCharCode(randInt(64, 95));
      return (
        string.substr(0, i) + randChar + string.substr(i + 1, string.length)
      );
    }

    function initMagic(el, str) {
      var i = 0,
        obsStr = str || el.innerHTML,
        strLen = obsStr.length;
      if (!strLen) return;
      obfuscators[currId].push(
        window.setInterval(function () {
          if (i >= strLen) i = 0;
          obsStr = replaceRand(obsStr, i);
          el.innerHTML = obsStr;
          i++;
        }, 0),
      );
    }

    if (string.indexOf("<br>") > -1) {
      elem.innerHTML = string;
      listLen = elem.childNodes.length;
      for (nodeI = 0; nodeI < listLen; nodeI++) {
        currNode = elem.childNodes[nodeI];
        if (currNode.nodeType === 3) {
          multiMagic = document.createElement("span");
          multiMagic.innerHTML = currNode.nodeValue;
          elem.replaceChild(multiMagic, currNode);
          initMagic(multiMagic);
        }
      }
    } else {
      initMagic(elem, string);
    }
  }

  function applyCode(string, codes) {
    var elem = document.createElement("span"),
      obfuscated = false;

    string = string.replace(/\x00/g, "");

    codes.forEach(function (code) {
      elem.style.cssText += styleMap[code] + ";";
      if (code === "§k") {
        obfuscate(elem, string);
        obfuscated = true;
      }
    });

    if (!obfuscated) elem.innerHTML = string;

    return elem;
  }

  function parseStyle(string) {
    var finalPre = document.createElement("pre"),
      codes = string.match(/§.{1}/g) || [],
      codesLen = codes.length,
      indexes = [],
      indexDelta,
      apply = [],
      strSlice,
      i;

    if (!obfuscators[currId]) obfuscators[currId] = [];

    string = string.replace(/\n|\\n/g, "<br>");

    for (i = 0; i < codesLen; i++) {
      indexes.push(string.indexOf(codes[i]));
      string = string.replace(codes[i], "\x00\x00");
    }

    if (indexes[0] !== 0) {
      finalPre.appendChild(applyCode(string.substring(0, indexes[0]), []));
    }

    for (i = 0; i < codesLen; i++) {
      indexDelta = indexes[i + 1] - indexes[i];
      if (indexDelta === 2) {
        while (indexDelta === 2) {
          apply.push(codes[i]);
          i++;
          indexDelta = indexes[i + 1] - indexes[i];
        }
        apply.push(codes[i]);
      } else {
        apply.push(codes[i]);
      }
      if (apply.lastIndexOf("§r") > -1) {
        apply = apply.slice(apply.lastIndexOf("§r") + 1);
      }
      strSlice = string.substring(indexes[i], indexes[i + 1]);
      finalPre.appendChild(applyCode(strSlice, apply));
    }

    return finalPre;
  }

  function clearObfuscators(id) {
    obfuscators[id].forEach(function (interval) {
      clearInterval(interval);
    });
    alreadyParsed[id] = [];
    obfuscators[id] = [];
  }

  window.mineParse = function initParser(input) {
    var parsed,
      i = currId;
    if (i > 0) {
      while (i--) {
        if (alreadyParsed[i].nodeType) {
          if (!document.contains(alreadyParsed[i])) {
            clearObfuscators(i);
          }
        }
      }
    }
    parsed = parseStyle(input);
    alreadyParsed.push(parsed);
    currId++;
    return {
      parsed: parsed,
      raw: parsed.innerHTML,
    };
  };
})();
