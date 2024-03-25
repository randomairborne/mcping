const ipElement = document.getElementById("user-ip");
const ipMsgElement = document.getElementById("ip-msg");
const ipDescriptorElement = document.getElementById("ip-descriptor");

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

async function doAutoPing() {
  let windowHash = window.location.hash.substring(1);
  let [actionString, edition, hostname] = windowHash.split(";", 3);
  if (actionString === "ping") {
    if (edition.startsWith("b")) {
      window.location.pathname = `/ping/bedrock/${hostname}`;
    } else if (edition.startsWith("j")) {
      window.location.pathname = `/ping/java/${hostname}`;
    }
  }
}

doAutoPing().then(() => {});
