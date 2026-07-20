<div align="center">
  <img src="Spectra/assets/logo.png" alt="Spectra Logo" width="128" height="128" />

  # Spectra

  **The AI-native API client.** Fast, beautiful, and built for the agentic era.

  [![Built with Tauri](https://img.shields.io/badge/Built_with-Tauri_v2-FFC131?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app)
  [![Frontend](https://img.shields.io/badge/Frontend-React_19-61DAFB?style=flat-square&logo=react&logoColor=white)](https://react.dev)
  [![Backend](https://img.shields.io/badge/Backend-Rust-CE422B?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
  [![MCP](https://img.shields.io/badge/MCP-Supported-8B5CF6?style=flat-square)](https://modelcontextprotocol.io)
  [![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)](#license)

  🌐 **[fanaticalnerd.com/Spectra](https://fanaticalnerd.com/Spectra.html)**

</div>

---

## Two platform projects, one repo

Spectra ships separately for macOS and Windows. Each platform has its own self-contained Tauri project, versioned and released independently:

| Platform | Project folder | Release tags | Docs |
|----------|----------------|--------------|------|
| macOS | [`Spectra/`](Spectra) | `mac-v*` | [README](Spectra/README.md) · [CONTRIBUTING](Spectra/CONTRIBUTING.md) |
| Windows | [`Spectra-Windows/`](Spectra-Windows) | `win-v*` | [README](Spectra-Windows/README.md) · [CONTRIBUTING](Spectra-Windows/CONTRIBUTING.md) |

Pick your platform folder above for installation, build instructions, architecture, and MCP setup — the two READMEs cover the full feature set and are otherwise identical aside from platform-specific steps.

## Releases

Both platforms build via a single GitHub Actions workflow ([`.github/workflows/release.yml`](.github/workflows/release.yml)) gated by tag prefix:

- Pushing a `mac-v*` tag builds and drafts a release from `Spectra/`
- Pushing a `win-v*` tag builds and drafts a release from `Spectra-Windows/`

## License

This project is licensed under the **MIT License** — see [`Spectra/LICENSE`](Spectra/LICENSE).

---

<div align="center">
  <br />
  <strong>Built with ❤️ by <a href="https://github.com/rishabhyadavm07">Rishabh</a></strong>
  <br />
  <sub>Powered by Rust · React · Tauri · MCP</sub>
</div>
