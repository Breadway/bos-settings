# Proposal verification

The previously identified implementation gaps are addressed in `feature/bos-settings-v2`. Rendered pixel accuracy remains unverified because UI testing is reserved for the user.

## Test artifact

- Binary: `src/target/release/bos-settings`
- Build: `npm run build`, then `cargo build --release --locked`
- Proposal: `/home/breadway/Projects/bos-ui-demos/proposed/bos-settings-v2/index.html`
- Asset verification: all 23 JavaScript, CSS, JSON, font and license assets in the current frontend build match byte for byte against compressed assets present in the executable, after Brotli decompression.
- Companion bar: `feature/notification-preferences` in `/tmp/bos-settings-v2-breadbar`. Its notification API, clock format and text preferences must be present in the running bar when testing these integrations.

The executable checksum is recorded in the build report alongside this verification after the final build.

## Gap coverage

| Area | Implementation evidence |
| --- | --- |
| Home | Six functional quick controls, measured attention items, connected network/devices, battery/storage summaries and recent-change Undo |
| Power | Battery hero/history, supported profiles, automatic switching, idle timers, battery-only sleep, lid and power-button policy |
| Notifications | Persisted typed API, DND schedule, per-app delivery, sounds and lock-time suppression in the companion server |
| Get apps | Unified ranked provider search, source filters, discovery shortcuts, real votes and installed status |
| Backup & restore | Stacked snapshot timeline and file backup, manual snapshots, folder picker and headless daily/weekly scheduling |
| Date & language | Time/timezone, clock format and installed language/region choices |
| Accessibility | Cursor/text sizing, stronger borders, reduced compositor motion, live zoom and retained Orca/kmag controls |
| Startup | Installed-app picker, visible Managed by BOS entries and guarded advanced editing |
| Sidebar | Account footer, actual update count and new-page badges |
| Shared styling | Bundled Outfit/Varela Round fonts, breadcrumbs, page hierarchy and stacked sections |
| Other tracked fixes | Shortcut labels/group/filter, window/shell previews and Undo, mirroring/login overrides, unused-dependency review and measured directory totals, typed health checks |

## Verification limits

Frontend checks report zero errors and warnings, frontend tests pass, and the production frontend builds. Rust formatting, clippy with warnings denied and 134 Settings tests pass. The companion bar's clippy and 24 headless notification tests pass, and its test binary builds with the documented dev dependency overrides.

No application launch, screenshot comparison, compositor input, live configuration writes or service starts were performed for verification. Unit tests execute generated Lua against a stub compositor interface and exercise configuration parsers and rendering in isolation.

Hardware and service availability determine which controls work. Notification sound requires the sound player. Secure lock surfaces may prevent notification windows from appearing while locked. Power profiles require a supported profile service. Locale choices are restricted to installed locales. Battery history may be empty or unsupported. Btrfs directory totals overlap; they cannot establish a disjoint system/files/snapshot allocation or an exact reclaimable amount. App metadata shown in the static mockup is replaced by available provider data.

These differences and conditional capabilities prevent an honest claim that the running binary matches every mockup pixel and sample value exactly. The remaining acceptance work is the user's rendered UI test, tracked by BSET-43; the implementation and companion changes are tracked by BSET-44, BSET-32 and BAR-56.
