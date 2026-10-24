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
      return /^\/[a-z]{3}-[a-z]{4}-[a-z]{3}(?:\/|$)/i.test(location.pathname);
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
  function notify(event) {
    chrome.runtime.sendMessage({ type: "satellite", event, session }).catch(() => {});
  }
  function probe() {
    const present = hasLeaveControl();
    positives = present ? Math.min(positives + 1, 3) : 0;
    negatives = !present ? Math.min(negatives + 1, 3) : 0;
    if (!active && positives >= 2) {
      active = true;
      lastBeat = Date.now();
      notify("join");
    } else if (active && negatives >= 2) {
      active = false;
      notify("leave");
    } else if (active && Date.now() - lastBeat > 5000) {
      lastBeat = Date.now();
      notify("beat");
    }
  }
  let pending = false;
  const observer = new MutationObserver(() => {
    if (pending) return;
    pending = true;
    setTimeout(() => { pending = false; probe(); }, 500);
  });
  observer.observe(document.documentElement, { childList: true, subtree: true, attributes: true,
    attributeFilter: ["aria-label", "title", "data-tid", "class", "hidden"] });
  setInterval(probe, 2000);
  addEventListener("pagehide", () => { if (active) notify("leave"); }, { once: true });
  probe();
})();
