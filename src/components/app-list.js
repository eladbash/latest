const { invoke } = window.__TAURI__.core;

// Rich, saturated colors for avatars
const COLORS = [
  "#e5484d", "#e54666", "#d24ce0", "#8e4ec6", "#6e56cf",
  "#3e63dd", "#0091ff", "#00a2c7", "#12a594", "#30a46c",
  "#978365", "#e5484d", "#f76b15", "#ffc53d", "#46a758",
];

function hashColor(name) {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = name.charCodeAt(i) + ((h << 5) - h);
  return COLORS[Math.abs(h) % COLORS.length];
}

export async function loadApps() {
  const list = document.getElementById("app-list");
  list.innerHTML = `
    <div class="loading-state">
      <div class="loading-spinner"></div>
      <div class="loading-text">Scanning installed apps&hellip;</div>
    </div>`;

  try {
    const apps = await invoke("get_apps");
    renderApps(apps);
    return apps;
  } catch (e) {
    list.innerHTML = `<div class="empty-state">
      <div class="empty-title">Something went wrong</div>
      <div class="empty-sub">${esc(String(e))}</div>
    </div>`;
    return [];
  }
}

export function renderApps(apps, updateResults = [], showAll = false) {
  const list = document.getElementById("app-list");
  const updateMap = new Map();
  for (const r of updateResults) updateMap.set(r.app_path, r);

  // If no results yet (still checking), show loading state
  if (updateResults.length === 0 && apps.length > 0 && !showAll) {
    list.innerHTML = `
      <div class="loading-state">
        <div class="loading-spinner"></div>
        <div class="loading-text">Checking for updates&hellip;</div>
      </div>`;
    return;
  }

  // Show all mode but no results yet — show all apps without status
  if (updateResults.length === 0 && apps.length > 0 && showAll) {
    let html = `<div class="status-bar">
      <span class="status-text">${apps.length} apps</span>
    </div>`;
    for (const app of [...apps].sort((a, b) => a.name.localeCompare(b.name))) {
      const initial = (app.name || "?").charAt(0).toUpperCase();
      const color = hashColor(app.name || "?");
      html += `<div class="update-card uptodate-card">
        <div class="update-card-top">
          <div class="update-avatar" style="background:${color}">${initial}</div>
          <div class="update-info">
            <div class="update-name">${esc(app.name)}</div>
            <div class="update-versions">
              <span>${esc(app.current_version)}</span>
            </div>
          </div>
          <span class="uptodate-badge checking-badge">Checking&hellip;</span>
        </div>
      </div>`;
    }
    list.innerHTML = html;
    return;
  }

  // Apps with updates
  const updates = apps
    .filter((a) => updateMap.get(a.path)?.has_update)
    .map((a) => ({ app: a, result: updateMap.get(a.path) }))
    .sort((a, b) => a.app.name.localeCompare(b.app.name));

  // Apps without updates (for "show all" mode)
  const upToDate = showAll
    ? apps
        .filter((a) => !updateMap.get(a.path)?.has_update)
        .sort((a, b) => a.name.localeCompare(b.name))
    : [];

  if (updates.length === 0 && !showAll) {
    list.innerHTML = `
      <div class="empty-state">
        <div class="empty-icon">
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"></polyline>
          </svg>
        </div>
        <div class="empty-title">All up to date</div>
        <div class="empty-sub">${apps.length} apps checked &mdash; no updates available</div>
      </div>`;
    return;
  }

  let html = "";

  if (updates.length > 0) {
    html += `<div class="status-bar">
      <span class="status-text">${updates.length} update${updates.length !== 1 ? "s" : ""} available</span>
    </div>`;

    for (const { app, result } of updates) {
      html += renderUpdateCard(app, result);
    }
  } else if (showAll) {
    html += `<div class="status-bar">
      <span class="status-text">All up to date</span>
    </div>`;
  }

  if (showAll && upToDate.length > 0) {
    html += `<div class="status-bar" style="margin-top:8px">
      <span class="status-text">${upToDate.length} up to date</span>
    </div>`;

    for (const app of upToDate) {
      const initial = app.name.charAt(0).toUpperCase();
      const color = hashColor(app.name);
      html += `<div class="update-card uptodate-card">
        <div class="update-card-top">
          <div class="update-avatar" style="background:${color}">${initial}</div>
          <div class="update-info">
            <div class="update-name">${esc(app.name)}</div>
            <div class="update-versions">
              <span>${esc(app.current_version)}</span>
            </div>
          </div>
          <span class="uptodate-badge">Up to date</span>
        </div>
      </div>`;
    }
  }

  list.innerHTML = html;
}

function renderUpdateCard(app, result) {
  const initial = app.name.charAt(0).toUpperCase();
  const color = hashColor(app.name);
  const src = fmtSource(result.source);

  return `<div class="update-card" data-app-path="${escAttr(app.path)}">
    <div class="update-card-top">
      <div class="update-avatar" style="background:${color}">${initial}</div>
      <div class="update-info">
        <div class="update-name">${esc(app.name)}</div>
        <div class="update-versions">
          <span>${esc(app.current_version)}</span>
          <span class="arrow">&rarr;</span>
          <span class="new">${esc(result.latest_version)}</span>
          ${src ? `<span class="update-source">${src}</span>` : ""}
        </div>
      </div>
      <button class="update-btn" data-path="${escAttr(app.path)}" data-source="${result.source}">Update</button>
    </div>
    <div class="progress-wrap hidden" data-progress-for="${escAttr(app.path)}">
      <div class="progress-track">
        <div class="progress-fill"></div>
      </div>
      <div class="progress-label">Downloading&hellip;</div>
    </div>
    ${result.error ? `<div class="update-error">${esc(result.error)}</div>` : ""}
  </div>`;
}

export function filterApps(apps, updateResults, query) {
  // not used in updates-only view, but kept for compatibility
  renderApps(apps, updateResults);
}

function fmtSource(s) {
  return { Sparkle: "Sparkle", Homebrew: "Brew", MacAppStore: "MAS" }[s] || "";
}

function esc(str) {
  if (!str) return "";
  const d = document.createElement("div");
  d.textContent = str;
  return d.innerHTML;
}

function escAttr(str) {
  if (!str) return "";
  return str.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
}
