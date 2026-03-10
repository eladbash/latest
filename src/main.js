import { loadApps, renderApps } from "./components/app-list.js";
import { initSettings } from "./components/settings.js";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let currentApps = [];
let currentResults = [];
let showAllApps = false;

document.addEventListener("DOMContentLoaded", async () => {
  initSettings();
  initRefresh();

  // Load show_all_apps preference
  try {
    const settings = await invoke("get_settings");
    showAllApps = settings.show_all_apps || false;
  } catch (_) {}

  currentApps = await loadApps();

  if (currentApps.length > 0) {
    checkUpdates();
  }
});

// Listen for show-all toggle changes from settings
window.addEventListener("show-all-changed", (e) => {
  showAllApps = e.detail;
  renderApps(currentApps, currentResults, showAllApps);
});

async function checkUpdates() {
  setLoading(true);
  try {
    currentResults = await invoke("check_updates_now");
  } catch (e) {
    showToast(`Update check failed: ${e}`, true);
  } finally {
    renderApps(currentApps, currentResults, showAllApps);
    setLoading(false);
  }
}

function initRefresh() {
  document.getElementById("refresh-btn").addEventListener("click", async () => {
    setLoading(true);
    currentApps = await loadApps();
    if (currentApps.length > 0) {
      await checkUpdates();
    }
    setLoading(false);
  });
}

function setLoading(on) {
  document.getElementById("refresh-btn").classList.toggle("spinning", on);
}

function showToast(message, isError = false) {
  const toast = document.getElementById("toast");
  toast.textContent = message;
  toast.className = isError ? "toast error" : "toast";
  setTimeout(() => (toast.className = "hidden"), 4000);
}

// Backend events
listen("updates-found", (event) => {
  currentResults = event.payload;
  renderApps(currentApps, currentResults, showAllApps);
});

// Progress bar updates
listen("update-progress", (event) => {
  const { app_path, phase, percent } = event.payload;
  const wrap = document.querySelector(`[data-progress-for="${CSS.escape(app_path)}"]`);
  if (!wrap) return;

  wrap.classList.remove("hidden");
  const fill = wrap.querySelector(".progress-fill");
  const label = wrap.querySelector(".progress-label");

  if (phase === "downloading") {
    fill.classList.remove("installing");
    fill.style.width = `${percent}%`;
    label.textContent = percent > 0 ? `Downloading\u2026 ${percent}%` : "Downloading\u2026";
  } else if (phase === "installing") {
    fill.classList.add("installing");
    fill.style.width = percent >= 100 ? "100%" : "50%";
    label.textContent = percent >= 100 ? "Installed" : "Installing\u2026";
  }
});

// Update button delegation
document.addEventListener("click", async (e) => {
  const btn = e.target.closest(".update-btn");
  if (!btn || btn.disabled) return;

  const appPath = btn.dataset.path;
  const source = btn.dataset.source;

  console.log("[Latest] Triggering update:", { appPath, source });

  btn.disabled = true;

  try {
    // Check if the app is running
    const running = await invoke("is_app_running", { appPath });
    let wasRunning = false;

    if (running) {
      // Show confirmation dialog
      const appName = btn.closest(".update-card").querySelector(".update-name")?.textContent || "This app";
      const confirmed = await showConfirm(`${appName} is running. Quit it to update?`);
      if (!confirmed) {
        btn.disabled = false;
        return;
      }

      // Quit the app
      btn.textContent = "Closing\u2026";
      await invoke("quit_app", { appPath });
      wasRunning = true;
    }

    btn.textContent = "Updating\u2026";
    const result = await invoke("trigger_update", { appPath, source });
    console.log("[Latest] Update result:", result);

    // Open the app after update
    btn.textContent = "Opening\u2026";
    try {
      await invoke("reopen_app", { appPath });
    } catch (reopenErr) {
      console.warn("[Latest] Failed to reopen:", reopenErr);
    }

    // Hide progress bar
    const wrap = document.querySelector(`[data-progress-for="${CSS.escape(appPath)}"]`);
    if (wrap) wrap.classList.add("hidden");

    btn.textContent = "Done";
    btn.classList.add("done");
    showToast(result);

    // Re-check updates after a short delay to refresh the list
    setTimeout(async () => {
      try {
        currentApps = await invoke("get_apps");
        currentResults = await invoke("check_updates_now");
        renderApps(currentApps, currentResults, showAllApps);
      } catch (_) {}
    }, 3000);
  } catch (err) {
    console.error("[Latest] Update failed:", err);
    showToast(String(err), true);
    btn.textContent = "Retry";
    btn.disabled = false;
  }
});

// External links (footer, settings)
document.addEventListener("click", (e) => {
  const link = e.target.closest("[data-url]");
  if (link) {
    e.preventDefault();
    import("@tauri-apps/plugin-opener").then(({ openUrl }) => {
      openUrl(link.dataset.url);
    });
  }
});

// Confirmation dialog
function showConfirm(message) {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.className = "confirm-overlay";
    overlay.innerHTML = `
      <div class="confirm-dialog">
        <div class="confirm-message">${message}</div>
        <div class="confirm-actions">
          <button class="confirm-btn cancel">Cancel</button>
          <button class="confirm-btn ok">Quit & Update</button>
        </div>
      </div>`;
    document.body.appendChild(overlay);

    overlay.querySelector(".cancel").addEventListener("click", () => {
      overlay.remove();
      resolve(false);
    });
    overlay.querySelector(".ok").addEventListener("click", () => {
      overlay.remove();
      resolve(true);
    });
  });
}
