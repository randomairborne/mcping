const frame = document.getElementById("response-fake-frame");

async function load() {
  const req = await fetch(frame.dataset.src);
  frame.innerHTML = await req.text();
}

load().then(() => {});
