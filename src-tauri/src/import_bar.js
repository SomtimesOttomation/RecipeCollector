// Injected into the in-app browser window. Adds a small floating bar with
// "Import this page"; clicking it sends the page as the user sees it to the app.
(function () {
  if (window.top !== window) return; // only the main page, not ads/iframes
  if (window.__rcBar) return;
  window.__rcBar = true;

  function mount() {
    if (!document.body) return setTimeout(mount, 50);
    const host = document.createElement("div");
    host.style.cssText = "all:initial;position:fixed;z-index:2147483647;left:50%;bottom:20px;transform:translateX(-50%);";
    const root = host.attachShadow({ mode: "closed" });
    root.innerHTML = `
      <style>
        .bar{display:flex;align-items:center;gap:12px;padding:10px 12px 10px 16px;border-radius:14px;
          background:#2a2420;color:#faf7f2;font:14px -apple-system,BlinkMacSystemFont,"Segoe UI",system-ui,sans-serif;
          box-shadow:0 8px 30px rgba(0,0,0,.35);max-width:90vw}
        .msg{opacity:.85}
        button{font:inherit;font-weight:600;border:0;border-radius:10px;padding:8px 14px;cursor:pointer}
        .go{background:#c2553a;color:#fff}
        .go:disabled{opacity:.6;cursor:default}
        .x{background:transparent;color:#faf7f2;opacity:.7;padding:8px}
        .dbg{background:transparent;color:#faf7f2;opacity:.75;font-weight:500;padding:8px 6px}
        .dbg:hover{opacity:1;text-decoration:underline}
      </style>
      <div class="bar">
        <span class="msg">Close any pop-ups, then</span>
        <button class="go">Import this page</button>
        <button class="dbg" title="Save this page and a report to the library's debug folder">Save for analysis</button>
        <button class="x" title="Hide">✕</button>
      </div>`;
    const msg = root.querySelector(".msg");
    const go = root.querySelector(".go");
    const dbg = root.querySelector(".dbg");
    root.querySelector(".x").onclick = () => host.remove();

    dbg.onclick = async () => {
      const ipc = window.__TAURI_INTERNALS__;
      if (!ipc) {
        msg.textContent = "Couldn't connect to Recipe Collector.";
        return;
      }
      dbg.disabled = true;
      try {
        await ipc.invoke("save_debug_page", {
          html: document.documentElement.outerHTML,
          url: location.href,
        });
        msg.textContent = "Saved to the debug folder. Send both files for analysis.";
      } catch (e) {
        msg.textContent = String(e);
      } finally {
        dbg.disabled = false;
      }
    };

    go.onclick = async () => {
      const ipc = window.__TAURI_INTERNALS__;
      if (!ipc) {
        msg.textContent = "Couldn't connect to Recipe Collector.";
        return;
      }
      go.disabled = true;
      go.textContent = "Importing…";
      msg.textContent = "Reading the recipe";
      try {
        await ipc.invoke("import_page_html", {
          html: document.documentElement.outerHTML,
          url: location.href,
        });
        msg.textContent = "Imported!";
        go.textContent = "Done";
      } catch (e) {
        msg.textContent = String(e);
        go.disabled = false;
        go.textContent = "Try again";
      }
    };
    document.documentElement.appendChild(host);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", mount);
  } else {
    mount();
  }
})();
