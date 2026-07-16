# bos-settings

System settings app for [BOS (Bread Operating System)](https://git.breadway.dev/Breadway/bos) — GTK4, configures every bread\* app's config plus core system settings (network, sound, power, users, firewall, snapshots, packages, AUR, firmware, Hyprland display/appearance/autostart) non-destructively.

Split out of the `bos` repo into its own repo so a bos-settings release doesn't require a BOS ISO release, and vice versa. Still the only pacman-packaged (not bakery-managed) bread app — see `packaging/README.md`.

## Building

```bash
cargo build --release
```

## Packaging / releasing

See `packaging/README.md`. In short: bump `Cargo.toml`'s version, tag `vX.Y.Z`, push the tag to both remotes — `.forgejo/workflows/package.yml` builds and publishes to the `[breadway]` pacman repo automatically.
