<div align="center">
  <img src="crates/spectra-tauri/app-icon.png" alt="Spectra Logo" width="128" height="128" />

  # Spectra

  **The AI-native API client.** Fast, beautiful, and built for the agentic era.

  [![Built with Tauri](https://img.shields.io/badge/Built_with-Tauri_v2-FFC131?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app)
  [![Frontend](https://img.shields.io/badge/Frontend-React_19-61DAFB?style=flat-square&logo=react&logoColor=white)](https://react.dev)
  [![Backend](https://img.shields.io/badge/Backend-Rust-CE422B?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
  [![MCP](https://img.shields.io/badge/MCP-Supported-8B5CF6?style=flat-square)](https://modelcontextprotocol.io)
  [![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)](#license)

  [Features](#-features) · [Installation](#-installation) · [Quick Start](#-quick-start) · [MCP Integration](#-mcp-integration) · [Architecture](#-architecture) · [Contributing](#-contributing)

  🌐 **[fanaticalnerd.com/Spectra](https://fanaticalnerd.com/Spectra..html)**

</div>

---

## ✨ Features

### 🚀 Core API Client
- **Full HTTP support** — GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS
- **Request builder** — Headers, query params, JSON/form/raw body editor with syntax highlighting
- **Response viewer** — Monaco-powered viewer with JSON/XML/HTML/YAML formatting, line numbers, word wrap, and schema validation
- **Workspaces & Collections** — Organize requests into folders and workspaces with drag-and-drop
- **Environments** — Variable management with secret masking and per-environment overrides
- **Request history** — Automatic logging with replay, search, and saved responses

### 🎨 Beautiful Interface
- **Native macOS feel** — Overlay title bar, transparent window, system-native rendering
- **Adaptive themes** — System (auto), Light, Dark, and **Crimson** themes
- **Monaco Editor** — Full VS Code-grade syntax highlighting for request and response bodies
- **Resizable panels** — Drag to resize sidebar, request/response split, and console

### 🔐 Authentication
- **OAuth 2.0** — Authorization Code, Client Credentials, PKCE flows with token management
- **Bearer Token, Basic Auth, API Key** — First-class support at workspace, folder, or request level
- **AWS Signature v4, Hawk, Digest** — Advanced auth schemes built-in
- **Auth inheritance** — Set auth once at the workspace level, override per-folder or per-request

### 🤖 AI-Native (MCP Integration)
- **Built-in MCP server** — Ships as a sidecar binary, zero configuration required
- **60+ MCP tools** — Your AI agent can read workspaces, create requests, send them, analyze responses, and even take GUI screenshots
- **Works with any MCP client** — Claude Desktop, Cursor, Windsurf, or any agent that speaks MCP

### ⚡ Performance
- **Rust backend** — All HTTP execution, SQLite storage, and crypto run natively
- **Web Worker formatting** — Large JSON payloads are formatted off the main thread
- **Virtual scrolling** — Handles massive request trees and response headers without lag
- **~105 KB gzipped** — Tiny frontend bundle

---

## 📦 Installation

### Download (macOS Apple Silicon)

Grab the latest `.dmg` from the [Releases](https://github.com/rishabhyadavm07/Spectra/releases) page.

> **Note:** The app is ad-hoc signed. On first launch, right-click the app → **Open** to bypass Gatekeeper.

### Build from Source

#### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| **Node.js** | ≥ 18 | [nodejs.org](https://nodejs.org) |
| **Rust** | ≥ 1.75 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **Tauri CLI** | v2 | Included in `devDependencies` |

#### Steps

```bash
# 1. Clone the repository
git clone https://github.com/rishabhyadavm07/Spectra.git
cd Spectra

# 2. Install frontend dependencies
npm install

# 3. Run in development mode (hot reload)
npm run tauri dev

# 4. Build for production
npm run build:mac
```

The production build outputs:
```
target/release/bundle/macos/Spectra.app
target/release/bundle/dmg/Spectra_0.1.0_aarch64.dmg
```

---

## 🚀 Quick Start

### 1. Create a Workspace
Open Spectra and click the workspace switcher in the top-left. Create a new workspace (e.g., "My API").

### 2. Create a Request
Click **+** in the sidebar to create a new request. Give it a name, set the method and URL.

### 3. Add an Environment
Switch to the **Environments** tab in the sidebar. Create an environment with your variables (e.g., `base_url`, `api_key`).

### 4. Use Variables
Reference variables in your URL, headers, or body using `{{variable_name}}` syntax. Spectra will resolve them before sending.

### 5. Send & Inspect
Hit **Send**. The response panel shows the body (with syntax highlighting), headers, status code, timing, and size.

---

## 🤖 MCP Integration

Spectra ships with a built-in [Model Context Protocol](https://modelcontextprotocol.io) server, enabling AI agents to natively interact with your APIs.

### Setup with Claude Desktop

Add the following to your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "spectra-mcp": {
      "command": "/Applications/Spectra.app/Contents/Resources/bin/spectra-mcp",
      "args": []
    }
  }
}
```

### What Your AI Agent Can Do

| Category | Example Tools |
|----------|---------------|
| **Workspaces** | `list_workspaces`, `open_workspace`, `create_workspace` |
| **Requests** | `create_request`, `send_request`, `set_method`, `set_url`, `set_headers`, `set_body` |
| **Collections** | `list_folders`, `create_folder`, `move_request` |
| **Environments** | `list_environments`, `create_environment`, `set_active_environment` |
| **History** | `list_history`, `replay_history_entry`, `save_response` |
| **Auth** | `set_auth`, `get_auth`, `start_oauth_flow`, `fetch_oauth_token` |
| **Response Analysis** | `analyze_response`, `search_response`, `export_request` |
| **GUI Automation** | `automation_screenshot_request`, `send_and_screenshot` |

---

## 🏗 Architecture

Spectra is a **Tauri v2** application with a Rust backend and React frontend.

```
spectra-app/
├── src/                          # React frontend (TypeScript)
│   ├── App.tsx                   # Main application shell
│   ├── App.css                   # Design system + all styles
│   ├── ResponsePanel.tsx         # Monaco-powered response viewer
│   ├── RequestTabs.tsx           # Request body/params/headers editor
│   ├── RequestTree.tsx           # Sidebar collection tree
│   ├── ErrorBoundary.tsx         # Graceful crash recovery
│   ├── theme.ts                  # Theme engine (System/Light/Dark/Crimson)
│   ├── store/                    # Zustand state management
│   └── worker/                   # Web Workers (JSON formatting)
│
├── crates/
│   ├── spectra-core/             # Core domain models + SQLite storage
│   ├── spectra-api/              # Tauri command handlers (IPC bridge)
│   ├── spectra-tauri/            # Tauri app shell + window config
│   └── spectra-mcp/              # MCP sidecar server (60+ tools)
│
├── package.json                  # Frontend dependencies + scripts
├── Cargo.toml                    # Rust workspace definition
├── vitest.config.ts              # Test configuration
└── vite.config.ts                # Vite bundler configuration
```

### Tech Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| **Shell** | Tauri v2 | Native window, IPC, bundling |
| **Backend** | Rust | HTTP execution, SQLite, crypto, MCP server |
| **Frontend** | React 19 + TypeScript | UI components and state |
| **Editor** | Monaco Editor | Syntax highlighting for requests/responses |
| **State** | Zustand | Lightweight React state management |
| **Database** | SQLite (via sqlx) | Persistent storage for workspaces, history |
| **Bundler** | Vite 7 | Fast HMR and production builds |
| **Testing** | Vitest | Unit testing framework |

---

## 🧪 Testing

```bash
# Run the test suite
npx vitest run

# Run in watch mode
npx vitest
```

Current test coverage:
- `src/theme.test.ts` — Theme application logic (dark, crimson, light switching)
- `src/worker/formatter.test.ts` — JSON formatting and error fallback

---

## 🎨 Themes

Spectra ships with four built-in themes:

| Theme | Description |
|-------|-------------|
| **System** | Automatically follows your macOS appearance setting |
| **Light** | Clean, bright interface with white panels |
| **Dark** | Modern dark interface with `#121212` backgrounds |
| **Crimson** | Premium dark-red aesthetic with ruby accents |

Change your theme in **Settings** (⚙️) → **General** → **Theme**.

---

## 📁 Project Scripts

| Script | Description |
|--------|-------------|
| `npm run dev` | Start Vite dev server (frontend only) |
| `npm run tauri dev` | Start full Tauri app with hot reload |
| `npm run build` | Build frontend (sidecar + TypeScript + Vite) |
| `npm run build:mac` | Build production `.app` and `.dmg` bundles |
| `npx vitest run` | Run test suite |

---

## 🤝 Contributing

Contributions are welcome! Here's how to get started:

1. **Fork** the repository
2. **Create a branch** for your feature (`git checkout -b feature/amazing-feature`)
3. **Commit** your changes (`git commit -m 'Add amazing feature'`)
4. **Push** to the branch (`git push origin feature/amazing-feature`)
5. **Open a Pull Request**

### Development Tips

- The frontend hot-reloads instantly via Vite HMR when running `npm run tauri dev`
- Rust changes require a recompile — Tauri handles this automatically
- CSS variables in `:root`, `html.dark`, and `html.crimson` blocks control all theming
- The MCP sidecar is a separate binary in `crates/spectra-mcp/`

---

## 📄 License

This project is licensed under the **MIT License** — see the [LICENSE](LICENSE) file for details.

---

<div align="center">
  <br />
  <strong>Built with ❤️ by <a href="https://github.com/rishabhyadavm07">Rishabh</a></strong>
  <br />
  <sub>Powered by Rust · React · Tauri · MCP</sub>
</div>
