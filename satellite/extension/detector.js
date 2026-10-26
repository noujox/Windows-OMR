// Conservative, read-only DOM detector. Never accesses call audio, media,
// page globals, or meeting text. URLs alone do NOT trigger recording.
(() => {
  const provider = (() => {
    const host = location.hostname.toLowerCase();
    if (host === "meet.google.com") return "meet";
    if (["teams.microsoft.com", "teams.cloud.microsoft"].includes(host)) return "teams";
    if (host.endsWith(".zoom.us")) return "zoom";
    return null;
  })();
  if (!provider) return;
  const session = provider + "." + crypto.randomUUID().replaceAll("-", "");
  let active = false;
  let positives = 0;
  let negatives = 0;
  let lastBeat = 0;
  let lastMic = "unknown";
  let liveSamples = 0;
  // We trust explicit button labels, not icons, clicks, or arbitrary page text.
  // Unknown stays muted if Smart Mic Sync is enabled in the desktop app.
  const micMuted = [
    /\b(?:turn|switch) (?:the )?microphone on\b/i, /\bturn on (?:the )?microphone\b/i,
    /\bunmute\b/i, /\bmicrophone (?:is )?(?:off|muted)\b/i,
    /\bmic (?:is )?(?:off|muted)\b/i,
    /\b(?:activar|encender|habilitar) (?:el )?micr[oó]fono\b/i,
    /\bmicr[oó]fono (?:apagado|silenciado|desactivado)\b/i,
    /\b(?:reativar|reabrir) (?:o )?microfone\b/i,
  ];
  const micLive = [
    /\bturn (?:the )?microphone off\b/i, /\bturn off (?:the )?microphone\b/i,
    /\bmute (?:my )?(?:microphone|mic|audio)\b/i,
    /^mute$/i, /\bmicrophone (?:is )?on\b/i,
    /\bmic (?:is )?on\b/i,
    /\b(?:silenciar|desactivar|apagar) (?:el )?micr[oó]fono\b/i,
    /\bmicr[oó]fono activado\b/i,
    /^silenciar$/i,
    /\bsilenciar microfone\b/i,
  ];
  function readMic() {
    let found = null;
    for (const el of document.querySelectorAll("button,[role=\"button\"]")) {
      if (!el.getClientRects().length || el.disabled || el.getAttribute("aria-hidden") === "true") continue;
      const label = [
        el.getAttribute("aria-label"), el.getAttribute("title"),
        el.getAttribute("data-tooltip-content"), el.getAttribute("data-testid"),
      ].filter(Boolean).join(" ").trim().slice(0, 180);
      if (!label) continue;
      const muted = micMuted.some(re => re.test(label));
      const live = micLive.some(re => re.test(label));
      if (muted && live) return "unknown";
      if (!muted && !live) continue;
      const state = muted ? "muted" : "live";
      if (found && found !== state) return "unknown";
      found = state;
    }
    return found || "unknown";
  }
  function confirmedMic() {
    const observed = readMic();
    if (observed !== "live") {
      liveSamples = 0;
      return observed;
    }
    // Require two successive observations to re-enable the local mic track.
    liveSamples = Math.min(liveSamples + 1, 2);
    return liveSamples >= 2 ? "live" : "unknown";
  }

  const words = {
    meet: [
      /leave (?:the )?call/i, /salir de la llamada/i, /abandonar (?:la )?llamada/i,
      /sair da chamada/i, /raccrocher/i
    ],
    teams: [
      /^leave(?: call)?$/i, /^salir$/i, /^abandonar$/i, /^quitter$/i,
      /leave(?:[- ]call)?[- ]button/i, /call[- ]leave/i
    ],
    zoom: [
      /^leave(?: meeting)?$/i, /^salir$/i, /^abandonar(?: la reunión)?$/i,
      /^quitter$/i, /leave[- ]meeting/i, /end[- ]meeting/i
    ]
  };
  const controls = {
    meet: 'button,[role="button"]',
    teams: 'button,[role="button"]',
    zoom: 'button,[role="button"]'
  };
  function qualifyingLocation() {
    if (provider === "meet") {
      // Current Meet also uses /meet/<code>, not only /<code>.
      // Do not activate from the Meet home/lobby page by URL alone.
      return /^\/(?:meet\/)?[a-z]{3}-[a-z]{4}-[a-z]{3}(?:\/|$)/i.test(location.pathname);
    }
    if (provider === "zoom") {
      return /\/(?:wc|j|s)\/|\/meeting\/|\/w\/|\/webinar\//i.test(location.pathname)
        || document.querySelector("#wc-footer, #zmmtg-root, .meeting-client") !== null;
    }
    return true; // Teams meetings often use SPA URLs without a meeting path.
  }
  function hasLeaveControl() {
    if (!qualifyingLocation()) return false;
    const candidates = document.querySelectorAll(controls[provider]);
    for (const el of candidates) {
      if (!el.getClientRects().length || el.disabled ||
          el.getAttribute("aria-hidden") === "true") continue;
      const text = [
        el.getAttribute("aria-label"), el.getAttribute("title"),
        el.getAttribute("data-tid"), el.getAttribute("data-testid"),
        el.getAttribute("data-tooltip-content"), el.textContent?.trim()
      ].filter(Boolean).join(" ").slice(0, 220);
      if (words[provider].some(re => re.test(text))) return true;
    }
    return false;
  }
  function notify(event, mic = "unknown") {
    chrome.runtime.sendMessage({ type: "satellite", event, session, mic }).catch(() => {});
  }
  function probe() {
    const present = hasLeaveControl();
    const mic = present ? confirmedMic() : "unknown";
    positives = present ? Math.min(positives + 1, 3) : 0;
    negatives = !present ? Math.min(negatives + 1, 3) : 0;
    if (!active && positives >= 2) {
      active = true;
      lastMic = mic;
      lastBeat = Date.now();
      notify("join", mic);
    } else if (active && negatives >= 2) {
      active = false;
      lastMic = "unknown";
      liveSamples = 0;
      notify("leave");
    } else if (active && (mic !== lastMic || Date.now() - lastBeat > 5000)) {
      // React quickly to mute; do not wait five seconds for the heartbeat.
      lastMic = mic;
      lastBeat = Date.now();
      notify("beat", mic);
    }
  }
  let pending = false;
  const observer = new MutationObserver(() => {
    if (pending) return;
    pending = true;
    setTimeout(() => { pending = false; probe(); }, 500);
  });
  observer.observe(document.documentElement, { childList: true, subtree: true, attributes: true,
    attributeFilter: ["aria-label", "title", "data-tid", "data-testid", "data-tooltip-content", "class", "hidden", "aria-pressed"] });
  setInterval(probe, 2000);
  // Explicit opt-in retry from the popup. Never send a join just because the
  // native host connected: this requires the user's click and a live call.
  chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
    if (msg?.type !== "satellite-retry") return false;
    const live = active && hasLeaveControl();
    if (live) notify("join", confirmedMic());
    sendResponse({active: live});
    return false;
  });
  addEventListener("pagehide", () => { if (active) notify("leave"); }, { once: true });
  probe();
})();
