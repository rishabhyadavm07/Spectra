# Contributing to Spectra

Thank you for considering contributing to Spectra! This document provides guidelines to help you get started.

## Development Setup

### Prerequisites

- **Node.js** ≥ 18
- **Rust** ≥ 1.75 (with `cargo`)
- **macOS** (primary development target)

### Getting Started

```bash
# Clone your fork
git clone https://github.com/<your-username>/Spectra.git
cd Spectra

# Install frontend dependencies
npm install

# Start in development mode
npm run tauri dev
```

This launches the full Tauri application with Vite hot module replacement for the frontend. Rust changes will trigger an automatic recompile.

## Project Structure

| Directory | Language | Purpose |
|-----------|----------|---------|
| `src/` | TypeScript/React | Frontend UI components |
| `crates/spectra-core/` | Rust | Domain models, SQLite storage |
| `crates/spectra-api/` | Rust | Tauri IPC command handlers |
| `crates/spectra-tauri/` | Rust | Tauri app shell |
| `crates/spectra-mcp/` | Rust | MCP sidecar server |

## Making Changes

### Frontend (React/TypeScript)

- All styles live in `src/App.css` using CSS custom properties for theming
- Theme variables are defined in `:root`, `html.dark`, and `html.crimson` blocks
- State management uses Zustand stores in `src/store/`
- Heavy computation (JSON formatting) runs in Web Workers (`src/worker/`)

### Backend (Rust)

- Add new Tauri commands in `crates/spectra-api/src/commands/`
- Register them in `crates/spectra-tauri/src/lib.rs`
- Database migrations live in `crates/spectra-core/`

### MCP Server

- Tools are defined in `crates/spectra-mcp/src/server.rs`
- Each tool has a JSON Schema for its parameters (via `schemars`)
- The server communicates with the Tauri app via local HTTP

## Testing

```bash
# Run all tests
npx vitest run

# Run in watch mode
npx vitest
```

## Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add OAuth PKCE support
fix: resolve dark mode flicker on startup
docs: update MCP integration guide
refactor: extract theme logic into theme.ts
test: add formatter edge case tests
```

## Pull Requests

1. Fork the repo and create your branch from `main`
2. If you've added code, add tests
3. Ensure the test suite passes (`npx vitest run`)
4. Ensure the app builds (`npm run build:mac`)
5. Write a clear PR description explaining your changes

## Code Style

- **TypeScript**: Prettier-formatted, strict mode
- **Rust**: `cargo fmt` and `cargo clippy`
- **CSS**: Custom properties for all colors, no hardcoded hex values in component styles

## Reporting Issues

When filing an issue, please include:
- macOS version
- Steps to reproduce
- Expected vs actual behavior
- Console output (if applicable)

---

Thank you for helping make Spectra better! 🎉
