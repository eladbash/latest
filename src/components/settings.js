const { invoke } = window.__TAURI__.core;

export function initSettings() {
  const btn = document.getElementById("settings-btn");
  const panel = document.getElementById("settings-panel");
  const list = document.getElementById("app-list");

  btn.addEventListener("click", async () => {
    const opening = panel.classList.contains("hidden");
    if (opening) {
      await renderSettings();
      panel.classList.remove("hidden");
      list.classList.add("hidden");
      btn.classList.add("active");
    } else {
      panel.classList.add("hidden");
      list.classList.remove("hidden");
      btn.classList.remove("active");
    }
  });
}

async function renderSettings() {
  const panel = document.getElementById("settings-panel");

  let settings;
  try {
    settings = await invoke("get_settings");
  } catch (e) {
    panel.innerHTML = `<div class="empty-state"><div class="empty-sub">${e}</div></div>`;
    return;
  }

  const intervals = [
    { value: "ThirtyMinutes", label: "30 min" },
    { value: "OneHour", label: "1 hour" },
    { value: "SixHours", label: "6 hours" },
    { value: "Daily", label: "Daily" },
  ];

  panel.innerHTML = `
    <div class="settings-group-label">General</div>
    <div class="settings-card">
      <div class="settings-row">
        <label>Check interval</label>
        <select id="interval-select">
          ${intervals.map((i) => `<option value="${i.value}" ${i.value === settings.check_interval ? "selected" : ""}>${i.label}</option>`).join("")}
        </select>
      </div>
      <div class="settings-row">
        <label for="notif-toggle">Notifications</label>
        <input type="checkbox" id="notif-toggle" class="toggle" ${settings.show_notifications ? "checked" : ""} />
      </div>
      <div class="settings-row">
        <label for="show-all-toggle">Show all apps</label>
        <input type="checkbox" id="show-all-toggle" class="toggle" ${settings.show_all_apps ? "checked" : ""} />
      </div>
    </div>

    ${
      settings.ignored_apps.length > 0
        ? `<div class="settings-group-label">Ignored apps</div>
    <div class="settings-card">
      ${settings.ignored_apps
        .map(
          (id) => `<div class="ignored-row">
        <span>${id}</span>
        <button class="unignore-btn" data-id="${id}">Remove</button>
      </div>`,
        )
        .join("")}
    </div>`
        : ""
    }

    <div class="settings-group-label">About</div>
    <div class="settings-card">
      <div class="settings-row">
        <label>Version</label>
        <span style="font-size:12px;color:var(--text-dim)">0.1.0</span>
      </div>
      <div class="settings-row">
        <label>Made by</label>
        <a href="#" class="footer-link" data-url="https://github.com/eladbash" style="font-size:12px;color:var(--blue)">eladbash</a>
      </div>
      <div class="settings-row">
        <label>Support development</label>
        <a href="#" class="footer-link sponsor-link" data-url="https://github.com/sponsors/eladbash" style="font-size:12px;font-weight:600;color:var(--blue)">&hearts; Sponsor</a>
      </div>
    </div>
  `;

  document.getElementById("interval-select").addEventListener("change", async (e) => {
    try { await invoke("set_check_interval", { interval: e.target.value }); }
    catch (err) { console.error(err); }
  });

  document.getElementById("notif-toggle").addEventListener("change", async (e) => {
    try { await invoke("set_show_notifications", { enabled: e.target.checked }); }
    catch (err) { console.error(err); }
  });

  document.getElementById("show-all-toggle").addEventListener("change", async (e) => {
    try {
      await invoke("set_show_all_apps", { enabled: e.target.checked });
      // Close settings, show updated app list immediately
      const panel = document.getElementById("settings-panel");
      const list = document.getElementById("app-list");
      const settingsBtn = document.getElementById("settings-btn");
      panel.classList.add("hidden");
      list.classList.remove("hidden");
      settingsBtn.classList.remove("active");
      window.dispatchEvent(new CustomEvent("show-all-changed", { detail: e.target.checked }));
    }
    catch (err) { console.error(err); }
  });

  panel.querySelectorAll(".unignore-btn").forEach((btn) => {
    btn.addEventListener("click", async () => {
      try { await invoke("toggle_ignore_app", { bundleId: btn.dataset.id }); await renderSettings(); }
      catch (err) { console.error(err); }
    });
  });
}
