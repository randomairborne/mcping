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
    });
  fetch("/api/services", {})
    .then((response) => {
      return response.json();
    })
    .then((resp) => {
      const services_string = JSON.stringify(resp, null, "  ");
      servicesElement.innerHTML = new Option(services_string).innerHTML;
    });
}

window.addEventListener("load", (_) => {
  loadExampleResponses().then(() => {
    hljs.highlightAll();
  });
});
