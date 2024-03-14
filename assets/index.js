const serverStatusElement = document.getElementById("server-status");
const playersElement = document.getElementById("server-players");
const faviconElement = document.getElementById("server-favicon");
const latencyElement = document.getElementById("server-latency");
const versionElement = document.getElementById("server-version");
const ipElement = document.getElementById("user-ip");
const ipMsgElement = document.getElementById("ip-msg");
const ipDescriptorElement = document.getElementById("ip-descriptor");
const motdElement = document.getElementById("server-motd");
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
  selectElement.hidden = false;
  responseElement.hidden = true;
  motdElement.textContent = "";
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
    "/" +
    response["players"]["maximum"];
  motdElement.innerHTML = mineParse(response["motd"]).raw;
  versionElement.innerHTML = mineParse(response["version"]["broadcast"]).raw;
  selectElement.hidden = true;
  responseElement.hidden = false;
  serverStatusElement.innerHTML = null;
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

  let currId = 0,
    obfuscators = {},
    alreadyParsed = [];

  function obfuscate(elem, string) {
    let multiMagic, currNode, listLen, nodeI;

    function randInt(min, max) {
      return Math.floor(Math.random() * (max - min + 1)) + min;
    }

    function replaceRand(string, i) {
      let randChar = String.fromCharCode(randInt(64, 95));
      return (
        string.substr(0, i) + randChar + string.substr(i + 1, string.length)
      );
    }

    function initMagic(el, str) {
      let i = 0,
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
    let elem = document.createElement("span"),
      obfuscated = false;

    string = string.replace(/\x00/g, "");

    const is_color_code = /(\d|[a-f])/im;
    codes.forEach(function (code) {
      const raw_code = code.replace("§", "");
      if (is_color_code.test(raw_code)) {
        elem.classList.forEach((cls) => {
          const cls_code = cls.replace("motd-style-", "");
          if (is_color_code.test(cls_code)) {
            elem.classList.remove(`motd-style-${cls_code}`);
          }
        });
      }
      console.debug(code);
      if (code === "§k") {
        obfuscate(elem, string);
        obfuscated = true;
      } else {
        elem.classList.add(`motd-style-${raw_code}`);
      }
    });

    if (!obfuscated) elem.innerHTML = string;

    return elem;
  }

  function parseStyle(string) {
    let finalPre = document.createElement("pre"),
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
    let parsed,
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
