# BOS Settings v2 progress

Tracked by BSET-43. Remaining proposal controls are on BSET-44, with notification integration on BAR-56. Reference: `bos-ui-demos/proposed/bos-settings-v2/index.html`.

## Implemented in this pass

- The 25-page sidebar follows Connect, Devices, Personalise, Apps and System. Below 900px it becomes an icon rail; below 560px it becomes a scrolling top strip. The window minimum is 360px. Cards and controls wrap in narrow tiles.
- Ctrl K opens a keyboard-accessible settings palette. Search accepts misspellings, shows context and an empty state, and scrolls to indexed controls. Old CLI page aliases and old hub/tab destinations remain supported.
- Explicit-save configuration forms register their real state with a shared navigation guard. A sticky save notice and Save / Discard / Keep editing dialog protect edits across page and tab changes. A failed save keeps the current form and destination. Raw bar CSS participates too.
- Home retains successful readings between visits and offers real Wi-Fi, Bluetooth and night-light toggles with Undo, plus battery, updates and backup summaries. Recently changed lists changes made through these quick controls in the current process.
- Keyboard layout and mouse focus/scroll controls have their own pages. Printers, Lock screen, Startup, Users, Date & time and Accessibility have direct destinations. Desktop tools have one level of tabs.
- Storage measures filesystem usage, package downloads, app caches and Trash. Cleanup keeps three cached package versions; app caches require individual confirmation; Trash deletion requires confirmation. No simulated reclaimable-space figures or deletion Undo are shown.
- Sharing opens installed LocalSend and KDE Connect, reads SSH status and enables/disables the fixed sshd unit through polkit. It links to firewall rules rather than changing them implicitly.
- Notifications opens breadbar history and links to shell themes. Unsupported notification preferences are described explicitly. BAR-56 tracks the missing notification preferences API.
- Update everything creates a root snapshot when configured, then runs the existing typed update commands sequentially. Failures stop the sequence and preserve logs. Progress survives page navigation. Continuing without a configured snapshot requires confirmation.
- About offers a system information card, bakery channel and Copy system info. Existing hostname and hardware information controls remain.

## Bug coverage

| Issue | Implementation |
| --- | --- |
| BSET-26 | Responsive navigation, wrapping rows and a reduced window minimum |
| BSET-30 | Real form state tracking, sticky save notice, navigation and unload guards |
| BSET-33 | Core startup services separated from user apps, advanced editing and backend confirmation |
| BSET-34 | Fuzzy settings search, context, control destinations, keyboard navigation and empty state |
| BSET-35 | Bluetooth addresses and full lists; conditional package update buttons; cached Home status; default-printer marker; Secure Boot databases separated from hardware |

Earlier display, night-light, sound, brightness, keyboard and battery fixes are carried forward from `feature/settings-v2` and `main`. Keyboard and battery fixes already have PRs awaiting review. System snapshot restore remains tracked by BSET-22 and its existing PR.

## Capability changes for user review

The baseline is [capability-inventory.md](capability-inventory.md).

- Core startup service editing is now inside a collapsed System services section. It requires enabling advanced editing and confirming a changed service list. The existing controls remain available; changing them is deliberately guarded. The dedicated Help startup toggle remains available.
- Secure Boot certificate databases now appear inside a collapsed section on Firmware. They remain included in firmware updates.
- Existing keyboard, mouse, printer, startup, lock, user, date, update and backup settings moved to the sidebar destinations above. Legacy CLI aliases and hub/tab requests route to the corresponding new destinations.
- Home category shortcut tiles were replaced by dashboard controls; all categories remain in the sidebar and search.
- Package rows only show Update when an update is detected. Update all bakery remains available even when a pending-update check fails.
- The raw zoom number field, Orca/kmag controls and existing accessibility information are retained. A live zoom slider is added. No screen reader or magnifier capability was removed.

## Remaining proposal work

This is an implementation checkpoint, not completion of the entire approved mockup. BSET-43 remains in progress.

- Notifications: Do not disturb settings, scheduling, per-app permissions, sound and lock-screen preview require the BAR-56 interface and notification-server implementation.
- Mouse & touchpad: pointer speed, tap-to-click, click method and additional scrolling options need persisted input support beyond the current BOS settings loader.
- Power: power profiles, idle dim/lock/sleep timers and lid/power-button choices.
- Get apps: unified Flathub/repository/AUR search, votes, installed status and ranking. Existing AUR installation keeps its interactive PKGBUILD review.
- Date & language: language/region choices and clock-format presets. Current time-zone and NTP controls remain.
- Accessibility: cursor/text sizing and reduce motion. Live zoom now has a slider; the original number field and Apply button remain.
- Appearance/Desktop: preview graphics, presets and remaining friendly labels. Most config panels still use explicit Save with protection; Undo is currently available on Home quick controls.
- Displays: mirroring controls and confirmation that the BOS loader reapplies transform and VRR at login. The recovered PR explicitly records the loader dependency.
- Backup & restore: destination picker, scheduling, timeline and the separately reviewed system snapshot restore action.
- Storage: unused-dependency review and a trustworthy system/files/snapshot breakdown. Measured cache totals are shown as totals, not an invented reclaimable amount.
- About: bread/bakery health checks and a BOS version only when the operating system publishes one.

## Verification and manual checks

Automated checks: `npm run check`, `npm test`, `npm run build`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`.

UI testing is reserved for the user. No app window, compositor, desktop input or real configuration writes were used for testing.

1. Resize a tile through 900px, 560px and 360px. Check sidebar access, dropdowns, sliders, Save, paired devices and logs.
2. Edit DNS in Saved networks. Switch page and tab; test Keep editing, Discard, successful Save and a refused save. Repeat with raw bar CSS and a shortcut edit.
3. Search for `wallpapr`, `natural scrolling`, an indexed bar setting and a nonsense word. Use arrows, Enter and Escape.
4. Review Startup system entries and both advanced confirmations. Check Help's dedicated startup toggle.
5. Check paired devices with identical names, a default printer and an up-to-date bakery package.
6. Review Storage sizes and app-cache entries before testing any destructive action. Cancel cleanup confirmations first.
7. Check Home quick controls and Undo. A scheduled night-light setting indicates the configured state, not whether the warm period is active at that moment.
8. Review the Update everything stages before running it. Test cancelled snapshot authentication and the prompt when no root snapshot configuration exists.
9. Inspect Sharing with installed and missing apps, and cancel SSH authentication before testing a state change.
