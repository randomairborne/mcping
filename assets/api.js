import hljs from "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/es/highlight.min.js";
import json from "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/es/languages/json.min.js";

hljs.registerLanguage("json", json);

async function loadExampleResponses() {
  const servicesElement = document.getElementById("exampleServicesResponse");
  const pingResponseElement = document.getElementById("examplePingResponse");
  fetch("/api/java/mcping.me", {})
    .then((response) => {
      return response.json();
    })
    .then((resp) => {
      let ping_response = JSON.stringify(resp, null, "  ");
      pingResponseElement.innerHTML = new Option(ping_response).innerHTML;
      hljs.highlightElement(pingResponseElement);
    });
  fetch("/api/services", {})
    .then((response) => {
      return response.json();
    })
    .then((resp) => {
      const services_string = JSON.stringify(resp, null, "  ");
      servicesElement.innerHTML = new Option(services_string).innerHTML;
      hljs.highlightElement(servicesElement);
    });
}

loadExampleResponses().then(() => {});
