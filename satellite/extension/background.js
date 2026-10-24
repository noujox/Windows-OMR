// Only the extension service worker is allowed to invoke Native Messaging.
// Validate sender URL, platform and session before passing anything to the host.
const HOST = "com.computux.meeting_recorder_satellite";
const sessions = new Map();

function providerFor(url) {
  try {
    const u = new URL(url);
    if (u.protocol !== "https:") return null;
    if (u.hostname === "meet.google.com") return "meet";
    if (["teams.microsoft.com", "teams.cloud.microsoft"].includes(u.hostname)) return "teams";
    if (u.hostname.endsWith(".zoom.us") && u.hostname !== ".zoom.us") return "zoom";
  } catch {}
  return null;
}
function validSession(session, provider) {
  return typeof session === "string" && new RegExp("^" + provider + "\\.[A-Za-z0-9_-]{8,64}$").test(session);
}
async function forward(event, session) {
  try {
    const response = await chrome.runtime.sendNativeMessage(HOST, {event, session});
    const connected = response?.connected === true;
    await chrome.storage.session.set({connected, lastEvent: event, lastChecked: Date.now()});
    return connected;
  } catch (err) {
    await chrome.storage.session.set({connected: false, lastEvent: event, lastChecked: Date.now()});
    console.warn("Meeting Recorder native host:", err?.message || err);
    return false;
  }
}
chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg?.type !== "satellite" || !sender.tab?.id) return false;
  const provider = providerFor(sender.url || sender.origin || "");
  if (!provider || !validSession(msg.session, provider) ||
      !["join", "beat", "leave"].includes(msg.event)) return false;
  const tabId = sender.tab.id;
  // In Chromium/Edge onMessage must return literal `true` to keep the
  // asynchronous reply channel alive. Returning a Promise is unsupported in
  // many shipped Chromium versions and may cancel the native-message relay.
  (async () => {
    const enabled = (await chrome.storage.local.get({enabled: false})).enabled === true;
    const previous = sessions.get(tabId);
    if (msg.event === "leave") {
      if (previous === msg.session) {
        sessions.delete(tabId);
        await forward("leave", msg.session);
      }
      return {enabled, ok: true};
    }
    if (!enabled) return {enabled: false};
    if (previous && previous !== msg.session) {
      sessions.delete(tabId);
      await forward("leave", previous);
    }
    // If the browser service worker was restarted (or the app was not open),
    // promote the next heartbeat to a join. No restart of the recorder.
    const event = sessions.get(tabId) === msg.session ? msg.event : "join";
    const ok = await forward(event, msg.session);
    if (ok) sessions.set(tabId, msg.session);
    else sessions.delete(tabId);
    return {enabled: true, ok};
  })().then(sendResponse, err => {
    console.warn("Meeting Recorder satellite event:", err?.message || err);
    sendResponse({ok: false, error: "Message relay failed"});
  });
  return true;
});
chrome.tabs.onRemoved.addListener(tabId => {
  const session = sessions.get(tabId);
  if (session) { sessions.delete(tabId); forward("leave", session); }
});
chrome.storage.onChanged.addListener((changes, area) => {
  if (area === "local" && changes.enabled?.newValue === false) {
    for (const session of sessions.values()) forward("leave", session);
    sessions.clear();
  }
});
