# Contributing to Spectra

Thank you for considering contributing to Spectra! This repo contains two independent platform projects — pick the one you're working on:

- **macOS** — [`Spectra/CONTRIBUTING.md`](Spectra/CONTRIBUTING.md)
- **Windows** — [`Spectra-Windows/CONTRIBUTING.md`](Spectra-Windows/CONTRIBUTING.md)

Each project has its own `package.json`, `Cargo.toml`, and build/test steps. Changes to one platform generally don't need to touch the other unless you're fixing a bug that affects shared logic (e.g. request handling, auth signing) present in both trees.

## Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add OAuth PKCE support
fix: resolve dark mode flicker on startup
docs: update MCP integration guide
```

## Pull Requests

1. Fork the repo and create your branch from `main`
2. Follow the platform-specific setup and build/test steps linked above
3. Write a clear PR description explaining your changes, and note which platform(s) it affects
