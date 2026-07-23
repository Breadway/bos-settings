# bos-settings

System settings app for [BOS (Bread Operating System)](https://git.breadway.dev/Breadway/bos) — GTK4, configures every bread\* app's config plus core system settings (network, sound, power, users, firewall, snapshots, packages, AUR, firmware, Hyprland display/appearance/autostart) non-destructively.

Split out of the `bos` repo into its own repo so a bos-settings release doesn't require a BOS ISO release, and vice versa. Distributed via `bakery` — see `bread-ecosystem`'s `CONTRIBUTING.md` for the dev/beta/stable track workflow shared across the bread ecosystem.

## Building

```bash
cargo build --release
```

## Packaging / releasing

Bump `Cargo.toml`'s version, tag `vX.Y.Z`, push the tag to both remotes — `.forgejo/workflows/release.yml` builds and publishes to `dl.breadway.dev` (bakery) automatically. Pushes to `dev`/`beta` publish a dev/beta-track build the same way, no tag needed.
