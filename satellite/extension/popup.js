const toggle = document.querySelector("#enabled");
const status = document.querySelector("#status");
const connection = document.querySelector("#connection");
const id = document.querySelector("#extension-id");
id.textContent = chrome.runtime.id;

function update(enabled) {
  status.textContent = enabled
    ? "Detección habilitada. Meeting Recorder también debe tener su interruptor activado."
    : "Detección desactivada. No se iniciará ninguna grabación.";
}
async function showConnection() {
  const {connected, lastChecked, lastEvent} = await chrome.storage.session.get({
    connected: null, lastChecked: null, lastEvent: null
  });
  if (!lastChecked || Date.now() - lastChecked > 120_000) {
    connection.textContent = "Enlace: esperando detectar una reunión activa.";
  } else if (connected) {
    connection.textContent = "Enlace con Meeting Recorder: correcto (" + lastEvent + ").";
  } else {
    connection.textContent = "Reunión detectada, pero no se pudo contactar a Meeting Recorder.";
  }
}
chrome.storage.local.get({enabled:false}).then(({enabled}) => {
  toggle.checked = enabled;
  update(enabled);
});
showConnection().catch(() => {});
chrome.storage.onChanged.addListener((_changes, area) => {
  if (area === "session") showConnection().catch(() => {});
});
toggle.addEventListener("change", async () => {
  await chrome.storage.local.set({enabled: toggle.checked});
  update(toggle.checked);
});
document.querySelector("#copy").addEventListener("click", () =>
  navigator.clipboard.writeText(chrome.runtime.id));
