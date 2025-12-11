# Marketeer

A desktop market tool for EVE Online that automatically monitors your market exports, calculates competitive prices, and copies them to your clipboard.

![Tauri](https://img.shields.io/badge/Tauri-2.0-blue?logo=tauri)
![Vue](https://img.shields.io/badge/Vue-3.5-green?logo=vuedotjs)
![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
![License](https://img.shields.io/badge/License-MIT-yellow)

## Features

- **Automatic Market Log Watching** — Detects when you export market data in EVE Online and instantly processes it
- **Smart Price Adjustment** — Calculates the optimal undercut/overbid price based on EVE's minimum increment rules
- **Clipboard Integration** — Automatically copies the adjusted price to your clipboard, ready to paste in-game
- **Global Keyboard Shortcut** — Toggle between buy/sell mode with `Cmd+M` (macOS) or `Ctrl+M` (Windows/Linux), even when the app isn't focused
- **Price History Charts** — View 30-day price trends from EVE's ESI API
- **Order Book Display** — See the top 10 buy and sell orders at a glance
- **Cross-Platform** — Works on macOS, Windows, and Linux

## How It Works

1. Open the market for any item in EVE Online
2. Click "Export to File" in the market window
3. Marketeer automatically detects the new export and displays the market data
4. The adjusted price is copied to your clipboard
5. Paste directly into EVE's market order price field

## Installation

Download the latest release for your platform from the [Releases](https://github.com/eve-marketeer/marketeer/releases) page.

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | `.dmg` |
| macOS (Intel) | `.dmg` |
| Windows | `.msi` or `.exe` |
| Linux | `.deb` or `.AppImage` |

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) (LTS)
- [Rust](https://rustup.rs/) (stable)
- [Tauri CLI](https://tauri.app/start/prerequisites/)

### Setup

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

### Tech Stack

**Frontend**
- Vue 3 with Composition API
- TypeScript
- Tailwind CSS 4
- Pinia (state management)
- Chart.js (price history visualization)

**Backend**
- Rust
- Tauri 2
- notify (file system watching)
- reqwest (ESI API calls)
- clipboard-rs (clipboard access)

## Configuration

Marketeer reads market logs from EVE Online's default export location:

| Platform | Path |
|----------|------|
| macOS | `~/Documents/EVE/logs/Marketlogs/` |
| Windows | `%USERPROFILE%\Documents\EVE\logs\Marketlogs\` |
| Linux | `~/Documents/EVE/logs/Marketlogs/` |

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl + M` | Toggle between buy and sell mode (global) |

## License

MIT
