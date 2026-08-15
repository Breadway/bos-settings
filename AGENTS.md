# AGENTS.md — Repo hygiene

Scope: this file covers *repo hygiene* — branching, remotes, CI, cleanup. It is not project documentation.

This repo is a **Tauri 2 + Svelte 5** settings app, bakery-distributed. It follows the branch/release workflow in `CONTRIBUTING.md` — read and follow it for any git, branch, or release work here (the single-trunk model, `feature/x`/`fix/x` branch naming, how RC tags work, etc). Don't improvise a different workflow. The short version: there is one long-lived branch, `main` — no `dev` or `beta` branch exists. `main` auto-publishes a bakery **dev-track** build on every push. "Beta" and "stable" are both just tags, not branches: push a `vX.Y.Z-rc.N` tag to publish a beta-track build, push a plain `vX.Y.Z` tag to cut the signed stable release. "Freezing" for stabilization means pausing pushes to `main`, not moving a branch.

This is not a GTK4 app. There is no `package.yml` pacman workflow here; bakery is the distribution channel.

## Layout

- `frontend/` — Svelte 5 + SvelteKit (static adapter) + TypeScript
- `src/` — Tauri 2 Rust crate (`bos-settings`). Commands live in `src/src/commands/`.
- Config edits are non-destructive (`toml_edit` / `bread_utils::tomlcfg`) except for JSON files that have no comments to preserve.

## Remotes

- `origin` — Forgejo (`git.breadway.dev` via Hestia, SSH) — authoritative.
- `github` — GitHub mirror. Push `origin` only; the github remote auto-mirrors.

## CI

- `dev-release.yml` — push to `main`.
- `rc-release.yml` — `vX.Y.Z-rc.N` tags.
- `release.yml` — other `v*` tags (signed stable).

No build/lint/test CI runs on ordinary commits or PRs to `main` beyond the dev-track workflow above.

## Don't

- Don't commit `frontend/node_modules`.
- Don't embed credentials in remote URLs — SSH or a credential helper only.
- Don't write Wi-Fi passwords back into `breadcrumbs.toml`. Networks live in `~/.config/breadcrumbs/networks.toml` (0600).
- Don't expose a generic argv runner to the webview. Streaming updates are typed commands (`bakery_update`, `pacman_system_update`, `fwupd_*`).
