const toggle = document.querySelector("#enabled");
const status = document.querySelector("#status");
const connection = document.querySelector("#connection");
const micStatus = document.querySelector("#mic-status");
const id = document.querySelector("#extension-id");
id.textContent = chrome.runtime.id;

function update(enabled) {
  status.textContent = enabled
    ? "Detección habilitada. Meeting Recorder también debe tener su interruptor activado."
    : "Detección desactivada. No se iniciará ninguna grabación.";
}
async function showConnection() {
  const {connected, lastChecked, lastEvent, lastMic} = await chrome.storage.session.get({
    connected: null, lastChecked: null, lastEvent: null, lastMic: null
  });
  micStatus.textContent = !lastChecked || Date.now() - lastChecked > 120_000
    ? "Micrófono de la reunión: sin detectar. Smart Mic Sync (BETA)."
    : `Micrófono detectado: ${lastMic === "muted" ? "silenciado" : lastMic === "live" ? "activo" : "desconocido"}. La pista local solo cambia con Smart Mic Sync activado (BETA).`;
  if (!lastChecked || Date.now() - lastChecked > 120_000) {
    connection.textContent = "Enlace: esperando detectar una reunión activa.";
  } else if (connected) {
    connection.textContent = "Host nativo conectado (" + lastEvent + "). Esto no confirma que la app esté grabando.";
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
const diagnosticStatus = document.querySelector("#diagnostic-status");
document.querySelector("#diagnose").addEventListener("click", async () => {
  const button = document.querySelector("#diagnose");
  button.disabled = true;
  diagnosticStatus.textContent = "Comprobando el detector en la pestaña activa...";
  try {
    const tabs = await chrome.tabs.query({active:true, currentWindow:true});
    const tab = tabs[0];
    if (!tab?.id) {
      diagnosticStatus.textContent = "No se pudo identificar la pestaña activa.";
      return;
    }
    const reply = await chrome.tabs.sendMessage(tab.id, {type:"satellite-diagnostic"});
    if (!reply?.detectorLoaded) {
      diagnosticStatus.textContent = "El detector no respondió. Recarga la pestaña de Meet.";
      return;
    }
    diagnosticStatus.textContent =
      "Detector: cargado (" + reply.provider + "). " +
      "URL: " + (reply.urlRecognized ? "válida" : "no reconocida") + ". " +
      "Botón de salida: " + (reply.leaveControlFound ? "reconocido" : "no reconocido") +
      " (" + (reply.visibleControls ?? 0) + " controles visibles). " +
      "Reunión activa: " + (reply.active ? "sí" : "no") + ". " +
      "Micrófono: " + (reply.mic === "muted" ? "silenciado" : reply.mic === "live" ? "activo" : "desconocido") + ".";
  } catch (error) {
    diagnosticStatus.textContent = "Detector no disponible en esta pestaña. " +
      "Comprueba que sea Meet, recarga la extensión y después recarga la pestaña. " +
      ("Detalle: " + (error?.message || "sin respuesta")).slice(0, 110);
  } finally {
    button.disabled = false;
  }
});
document.querySelector("#retry").addEventListener("click", async () => {
  const retry = document.querySelector("#retry");
  retry.disabled = true;
  try {
    if (!toggle.checked) {
      connection.textContent = "Activa primero Detección automática.";
      return;
    }
    const tabs = await chrome.tabs.query({active: true, currentWindow: true});
    if (!tabs[0]?.id) throw new Error("No hay pestaña activa");
    const result = await chrome.tabs.sendMessage(tabs[0].id, {type: "satellite-retry"});
    connection.textContent = result?.active
      ? "Aviso de inicio reenviado. Revisa el estado de grabación y vuelve a abrir el popup."
      : "No se detectó una llamada activa en esta pestaña.";
  } catch (_err) {
    connection.textContent = "No se pudo contactar al detector de la pestaña. Recarga Meet.";
  } finally {
    retry.disabled = false;
  }
});
document.querySelector("#copy").addEventListener("click", () =>
  navigator.clipboard.writeText(chrome.runtime.id));
