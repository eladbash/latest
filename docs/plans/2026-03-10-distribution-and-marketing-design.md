# Latest — Distribution & Marketing Design

**Date:** 2026-03-10
**Author:** eladbash + Claude

## Overview

Add installation script, README, GitHub Pages marketing site, GitHub Sponsors, and in-app credits to the Latest macOS menu bar update checker.

All deliverables live in the same repo (`eladbash/latest`). Marketing site served from `/docs` via GitHub Pages.

## Deliverables

### 1. Installation Script (`install.sh`)

- Detects architecture (arm64 / x86_64)
- Fetches latest `.dmg` from GitHub Releases
- Mounts DMG, copies `Latest.app` to `/Applications`
- Unmounts and cleans up temp files
- macOS only — fails with message on Linux/Windows
- Usage: `curl -fsSL https://raw.githubusercontent.com/eladbash/latest/main/install.sh | sh`

### 2. README (`README.md`)

Structure:
1. Hero — app name, one-line description, screenshot
2. Features — bullet list (menu bar tray, Sparkle/Homebrew/MAS, one-click updates, auto-check)
3. Install — curl command + link to install.sh
4. Build from source — prerequisites (Rust, Node, Tauri CLI), commands
5. Credits — "Made by eladbash" + sponsor link

Screenshots saved to `screenshots/` folder, captured programmatically.

### 3. GitHub Pages Marketing Site (`docs/index.html`)

Single-page, self-contained HTML. Apple-inspired design.

Sections:
1. Nav — "Latest" logo left, "Download" CTA right
2. Hero — large headline, subtitle, download button, app screenshot with shadow/glow
3. Features grid — 3 cards: Menu Bar Native, Multiple Sources, One-Click Updates
4. How it works — 3-step flow: Scan → Check → Update
5. Install section — terminal-style block with curl command + copy button
6. Footer — "Made by eladbash" + GitHub link + Sponsor link

Style: white background with dark mode toggle, Inter/DM Sans typography, smooth scroll-reveal animations, glassmorphism card accents.

### 4. GitHub Sponsors (`.github/FUNDING.yml`)

```yaml
github: eladbash
```

Enables "Sponsor" button on the GitHub repo page.

### 5. In-App Credits

**Main view footer** (persistent, subtle):
- "Made by eladbash" in dim text at bottom of app list

**Settings panel — About section:**
- "Made by eladbash" row
- "Sponsor" row — opens `https://github.com/sponsors/eladbash` in system browser
- App version (0.1.0)

Sponsor links open in system default browser via Tauri shell API.

### 6. Screenshots (`screenshots/`)

Captured programmatically:
- Main view with update list (hero shot)
- Settings view with About section

## Approach

All-in-one repo. Marketing site in `/docs`, GitHub Pages configured to serve from `/docs` on main branch. No separate repos or branches.
