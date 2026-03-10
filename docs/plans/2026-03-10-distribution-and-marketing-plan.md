# Distribution & Marketing Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add install script, README with screenshots, GitHub Pages marketing site, GitHub Sponsors config, and in-app author credits with sponsor link.

**Architecture:** All deliverables in the same repo. Marketing site as self-contained `docs/index.html` served by GitHub Pages from `/docs`. In-app credits added to existing frontend (HTML + JS + CSS). External URLs opened via `tauri-plugin-opener`.

**Tech Stack:** Bash (install script), HTML/CSS/JS (marketing site), Tauri v2 + tauri-plugin-opener (in-app sponsor link), macOS screencapture (screenshots)

---

### Task 1: Add tauri-plugin-opener for external URL support

The app needs to open `https://github.com/sponsors/eladbash` in the system browser. Tauri v2 uses `tauri-plugin-opener` for this.

**Files:**
- Modify: `src-tauri/Cargo.toml` (add dependency)
- Modify: `src-tauri/src/lib.rs` (register plugin)
- Modify: `src-tauri/capabilities/default.json` (add permission)

**Step 1: Add tauri-plugin-opener dependency**

Run:
```bash
cd src-tauri && cargo add tauri-plugin-opener
```

**Step 2: Register the plugin in `src-tauri/src/lib.rs`**

Add `.plugin(tauri_plugin_opener::init())` after the existing `.plugin(tauri_plugin_store::Builder::default().build())` line:

```rust
.plugin(tauri_plugin_store::Builder::default().build())
.plugin(tauri_plugin_opener::init())
```

**Step 3: Add opener permission to `src-tauri/capabilities/default.json`**

Add `"opener:default"` to the permissions array:

```json
{
  "identifier": "default",
  "description": "Default capabilities for Latest",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:default",
    "core:window:allow-close",
    "core:window:allow-hide",
    "core:window:allow-show",
    "core:window:allow-set-focus",
    "positioner:default",
    "store:default",
    "opener:default"
  ]
}
```

**Step 4: Add the npm package for frontend access**

Run:
```bash
npm install @tauri-apps/plugin-opener
```

**Step 5: Verify it compiles**

Run:
```bash
cd src-tauri && cargo check
```
Expected: compiles without errors.

**Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/capabilities/default.json package.json package-lock.json
git commit -m "feat: add tauri-plugin-opener for external URL support"
```

---

### Task 2: Add in-app credits — main view footer

Add a subtle "Made by eladbash" footer at the bottom of the main app list view.

**Files:**
- Modify: `index.html` (add footer element)
- Modify: `src/styles.css` (add footer styles)

**Step 1: Add footer HTML to `index.html`**

Insert after the `<div id="content">...</div>` block, before the toast div:

```html
<!-- Footer -->
<div id="app-footer">
  <span class="footer-credit">Made by <a href="#" class="footer-link" data-url="https://github.com/eladbash">eladbash</a></span>
</div>
```

**Step 2: Add footer styles to `src/styles.css`**

Add at the end of the file, before no other section:

```css
/* ═══════════════════════════════════════════
   Footer
   ═══════════════════════════════════════════ */

#app-footer {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 8px 14px;
  border-top: 1px solid var(--border);
  flex-shrink: 0;
}

.footer-credit {
  font-size: 11px;
  color: var(--text-faint);
}

.footer-link {
  color: var(--text-dim);
  text-decoration: none;
  transition: color 0.15s;
}

.footer-link:hover {
  color: var(--text);
}
```

**Step 3: Add click handler for footer link in `src/main.js`**

Add at the end of the DOMContentLoaded handler, after `checkUpdates()`:

```javascript
// Footer external links
document.addEventListener("click", (e) => {
  const link = e.target.closest("[data-url]");
  if (link) {
    e.preventDefault();
    import("@tauri-apps/plugin-opener").then(({ openUrl }) => {
      openUrl(link.dataset.url);
    });
  }
});
```

**Step 4: Build and verify the app runs**

Run:
```bash
npm run build
```
Expected: builds without errors.

**Step 5: Commit**

```bash
git add index.html src/styles.css src/main.js
git commit -m "feat: add 'Made by eladbash' footer to main view"
```

---

### Task 3: Add in-app credits — settings About section

Add an "About" section at the bottom of the settings panel with author credit, sponsor button, and version.

**Files:**
- Modify: `src/components/settings.js` (add About section to renderSettings)

**Step 1: Modify `renderSettings()` in `src/components/settings.js`**

Replace the existing `settings-footer` div at the end of the `panel.innerHTML` template with an About section:

```javascript
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
        <a href="#" class="footer-link sponsor-link" data-url="https://github.com/sponsors/eladbash" style="font-size:12px;font-weight:600;color:var(--blue)">♥ Sponsor</a>
      </div>
    </div>
  `;
```

Remove the old `settings-footer` div that said "Updates are checked automatically..."

**Step 2: Verify the settings panel renders correctly**

Run:
```bash
npm run build
```
Expected: builds without errors.

**Step 3: Commit**

```bash
git add src/components/settings.js
git commit -m "feat: add About section with sponsor link to settings"
```

---

### Task 4: Create `.github/FUNDING.yml`

**Files:**
- Create: `.github/FUNDING.yml`

**Step 1: Create the file**

```yaml
github: eladbash
```

**Step 2: Commit**

```bash
git add .github/FUNDING.yml
git commit -m "feat: add GitHub Sponsors funding config"
```

---

### Task 5: Create installation script

**Files:**
- Create: `install.sh`

**Step 1: Write `install.sh`**

```bash
#!/bin/sh
set -e

# Latest — macOS app update checker
# Install: curl -fsSL https://raw.githubusercontent.com/eladbash/latest/main/install.sh | sh

REPO="eladbash/latest"
APP_NAME="Latest.app"
INSTALL_DIR="/Applications"

# ── Checks ──────────────────────────────────
case "$(uname -s)" in
  Darwin) ;;
  *) echo "Error: Latest is only supported on macOS." >&2; exit 1 ;;
esac

command -v curl >/dev/null 2>&1 || { echo "Error: curl is required." >&2; exit 1; }
command -v hdiutil >/dev/null 2>&1 || { echo "Error: hdiutil is required." >&2; exit 1; }

# ── Architecture ────────────────────────────
ARCH="$(uname -m)"
case "$ARCH" in
  arm64)  ASSET_PATTERN="aarch64.dmg" ;;
  x86_64) ASSET_PATTERN="x64.dmg" ;;
  *)      echo "Error: Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

# ── Fetch latest release ────────────────────
echo "Fetching latest release..."
RELEASE_URL="https://api.github.com/repos/${REPO}/releases/latest"
RELEASE_JSON="$(curl -fsSL "$RELEASE_URL")"

DMG_URL="$(echo "$RELEASE_JSON" | grep -o "\"browser_download_url\": *\"[^\"]*${ASSET_PATTERN}\"" | head -1 | cut -d'"' -f4)"

if [ -z "$DMG_URL" ]; then
  echo "Error: Could not find a .dmg asset for $ARCH in the latest release." >&2
  exit 1
fi

VERSION="$(echo "$RELEASE_JSON" | grep -o '"tag_name": *"[^"]*"' | head -1 | cut -d'"' -f4)"
echo "Installing Latest ${VERSION} for ${ARCH}..."

# ── Download ────────────────────────────────
TMP_DIR="$(mktemp -d)"
DMG_PATH="${TMP_DIR}/Latest.dmg"
trap 'rm -rf "$TMP_DIR"' EXIT

curl -fSL --progress-bar -o "$DMG_PATH" "$DMG_URL"

# ── Mount & copy ────────────────────────────
MOUNT_POINT="$(hdiutil attach -nobrowse -noautoopen "$DMG_PATH" 2>/dev/null | tail -1 | awk '{print $NF}')"

if [ ! -d "${MOUNT_POINT}/${APP_NAME}" ]; then
  # Some DMGs nest the app differently — search for it
  APP_FOUND="$(find "$MOUNT_POINT" -maxdepth 2 -name "$APP_NAME" -type d | head -1)"
  if [ -z "$APP_FOUND" ]; then
    hdiutil detach "$MOUNT_POINT" -quiet 2>/dev/null || true
    echo "Error: Could not find ${APP_NAME} in the DMG." >&2
    exit 1
  fi
  MOUNT_APP="$APP_FOUND"
else
  MOUNT_APP="${MOUNT_POINT}/${APP_NAME}"
fi

# Remove existing installation
if [ -d "${INSTALL_DIR}/${APP_NAME}" ]; then
  echo "Removing existing installation..."
  rm -rf "${INSTALL_DIR}/${APP_NAME}"
fi

echo "Copying to ${INSTALL_DIR}..."
cp -R "$MOUNT_APP" "${INSTALL_DIR}/"

# ── Cleanup ─────────────────────────────────
hdiutil detach "$MOUNT_POINT" -quiet 2>/dev/null || true

echo ""
echo "✓ Latest ${VERSION} installed to ${INSTALL_DIR}/${APP_NAME}"
echo "  Open it from your Applications folder or Spotlight."
```

**Step 2: Make executable**

Run:
```bash
chmod +x install.sh
```

**Step 3: Commit**

```bash
git add install.sh
git commit -m "feat: add installation script for macOS"
```

---

### Task 6: Build app and capture screenshots

**Files:**
- Create: `screenshots/` directory
- Create: `screenshots/main-view.png`
- Create: `screenshots/settings-view.png`

**Step 1: Build the app**

Run:
```bash
npm run build
```

**Step 2: Launch the app**

Run the built app in the background:
```bash
open src-tauri/target/release/bundle/macos/Latest.app &
```

**Step 3: Wait for the app to start, then capture the window**

```bash
sleep 3
mkdir -p screenshots
screencapture -l $(osascript -e 'tell application "System Events" to get id of first window of (first process whose name is "Latest")') screenshots/main-view.png
```

If the window-ID approach fails, fall back to:
```bash
screencapture -w screenshots/main-view.png
```
(interactive — click the Latest window)

**Step 4: Open settings and capture**

Click the settings button in the app manually, then:
```bash
screencapture -w screenshots/settings-view.png
```

**Step 5: Commit**

```bash
git add screenshots/
git commit -m "docs: add app screenshots"
```

---

### Task 7: Create README

**Files:**
- Create: `README.md`

**Step 1: Write README.md**

```markdown
<div align="center">

# Latest

**Keep every Mac app up to date.**

Latest lives in your menu bar and quietly checks for updates across Homebrew, Sparkle, and the Mac App Store — then lets you update with one click.

![Latest main view](screenshots/main-view.png)

</div>

## Features

- **Menu bar native** — lives in your tray, out of your way
- **Multiple sources** — checks Homebrew Casks, Sparkle feeds, and the Mac App Store
- **One-click updates** — download and install without leaving the app
- **Auto-check** — configurable intervals from 30 minutes to daily
- **Lightweight** — built with Tauri, minimal resource usage

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/eladbash/latest/main/install.sh | sh
```

Or download the latest `.dmg` from [Releases](https://github.com/eladbash/latest/releases).

## Build from Source

**Prerequisites:** [Rust](https://rustup.rs/), [Node.js](https://nodejs.org/) (v18+), [Tauri CLI](https://v2.tauri.app/start/prerequisites/)

```sh
git clone https://github.com/eladbash/latest.git
cd latest
npm install
npm run build
```

The built app will be in `src-tauri/target/release/bundle/macos/`.

To run in development mode:

```sh
npm run dev
```

## Settings

![Latest settings](screenshots/settings-view.png)

Configure check intervals, notifications, and ignore specific apps from the settings panel.

## Credits

Made by [eladbash](https://github.com/eladbash)

If you find Latest useful, consider [sponsoring](https://github.com/sponsors/eladbash) the project.

## License

MIT
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README with screenshots and install instructions"
```

---

### Task 8: Create GitHub Pages marketing site

**Files:**
- Create: `docs/index.html`

**Step 1: Write `docs/index.html`**

A single self-contained HTML file with Apple-inspired design. Key requirements:
- DM Sans / Inter font
- Clean white default with dark mode toggle (respects `prefers-color-scheme` too)
- Smooth scroll-reveal animations using IntersectionObserver
- Nav with "Latest" logo + "Download" CTA
- Hero section: large headline "Keep every app up to date", subtitle, download button, app screenshot with glow
- Features grid: 3 cards (Menu Bar Native, Multiple Sources, One-Click Updates) with SVG icons
- How it works: 3-step horizontal flow
- Install section: dark terminal block with curl command + copy button
- Footer: "Made by eladbash" + GitHub link + Sponsor link
- Fully responsive (mobile-friendly)
- All CSS inline in `<style>`, all JS inline in `<script>`
- Screenshot images referenced as `../screenshots/main-view.png` (relative path from docs/)

The full HTML is large — implement it as a polished, production-quality single-page site. Use CSS custom properties for theming (light/dark). Animations should be subtle and performant (transform/opacity only).

**Step 2: Verify the site works**

Run:
```bash
open docs/index.html
```

**Step 3: Commit**

```bash
git add docs/index.html
git commit -m "feat: add GitHub Pages marketing site"
```

---

### Task 9: Final verification

**Step 1: Build and run the app to verify in-app credits**

```bash
npm run build
open src-tauri/target/release/bundle/macos/Latest.app
```

Verify:
- Footer with "Made by eladbash" visible at bottom of main view
- Settings panel has "About" section with version, author, sponsor link
- Sponsor link opens in system browser

**Step 2: Verify install script syntax**

```bash
shellcheck install.sh || bash -n install.sh
```

**Step 3: Verify marketing site renders**

```bash
open docs/index.html
```

**Step 4: Verify all files are committed**

```bash
git status
```
Expected: clean working tree.

**Step 5: Final commit if needed, then done**
