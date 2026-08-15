# bos-settings

System settings app for [BOS (Bread Operating System)](https://git.breadway.dev/Breadway/bos) — Tauri 2 + Svelte 5. Configures every bread\* app's config plus core system settings (network, sound, power, users, firewall, snapshots, packages, AUR, firmware, Hyprland display/appearance/autostart) non-destructively.

Distributed via `bakery`. There is one long-lived branch, `main`; see `CONTRIBUTING.md` for the single-trunk / RC-tag release model shared across the bread ecosystem.

## Building

The Svelte frontend lives in `frontend/`, the Rust backend in `src/` (this repo's crate is not named `src-tauri`). `cargo tauri build` runs the frontend build hook; a plain `cargo build` does not.

```bash
cd frontend && npm ci && npm run build
cd ../src && cargo build --release
```

Dev (Vite + `cargo tauri dev`):

```bash
cd src && cargo tauri dev
```

## Packaging / releasing

Bump `src/Cargo.toml` (and `frontend/package.json`) version, then follow `CONTRIBUTING.md`: work lands on `main` via `feature/` / `fix/` branches (every push to `main` publishes a bakery **dev** build). Tag `vX.Y.Z-rc.N` for beta, `vX.Y.Z` for the signed stable release. Do not push to a `dev` branch — there isn't one.
