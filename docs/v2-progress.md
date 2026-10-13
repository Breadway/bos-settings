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

Earlier display, night-light, sound, brightness, keyboard and battery fixes are carried forward from `feature/settings-v2` and `main`. Keyboard and battery fixes already have PRs awaiting review. System snapshot restore from BSET-22 landed on main and is included in this branch.

## Capability changes for user review

The baseline is [capability-inventory.md](capability-inventory.md).

- Core startup service editing is now inside a collapsed System services section. It requires enabling advanced editing and confirming a changed service list. The existing controls remain available; changing them is deliberately guarded. The dedicated Help startup toggle remains available.
- Secure Boot certificate databases now appear inside a collapsed section on Firmware. They remain included in firmware updates.
- Existing keyboard, mouse, printer, startup, lock, user, date, update and backup settings moved to the sidebar destinations above. Legacy CLI aliases and hub/tab requests route to the corresponding new destinations.
- Home category shortcut tiles were replaced by dashboard controls; all categories remain in the sidebar and search.
- Package rows only show Update when an update is detected. Update all bakery remains available even when a pending-update check fails.
- The raw zoom number field, Orca/kmag controls and existing accessibility information are retained. A live zoom slider is added. No screen reader or magnifier capability was removed.

## Second implementation pass

The proposal differences identified by the binary audit are implemented in source and prepared for user testing:

- Home has the six quick controls, account greeting, connected network/devices, battery and storage summaries, measured cache/Trash attention, real update counts and Undo.
- Power has supported power profiles, an optional fixed systemd timer for automatic switching, real UPower history, idle timers, battery-only sleep, lid choices and a power-button menu. Button/lid changes require a restart. Hardware-dependent battery protection remains.
- Notifications now has typed preferences backed by the companion breadbar API: DND, overnight scheduling, sound, lock-time policy and per-app delivery policy. The running bar must contain BAR-56. Sound is disabled when the sound player is missing. Secure lock surfaces control whether notification windows can actually appear while locked.
- Mouse and accessibility settings persist through managed Lua includes without replacing the BOS loader. Text/font sizes also apply to Settings, GTK settings and the companion bar. Existing Orca/kmag controls and focus settings remain.
- Get apps searches repositories, Flathub and AUR together, ranks results, shows real AUR votes and installed status, and offers source filters and discovery shortcuts. Provider failures are reported separately. AUR keeps terminal PKGBUILD review. No fabricated verified badge or download count is displayed.
- Startup has an installed-app dialog and a visible Managed by BOS section. Editing core services and custom commands remains available through advanced controls.
- Appearance has shell previews, an interactive window preview, font/text choices and Undo for shell/window changes. Pages now stack their sections. Saved networks, keybindings and other forms retain their individual save guards.
- Displays has mirroring and a managed login include for transform/VRR, which the stock monitor JSON loader did not consume.
- Backup & restore has a timeline, manual snapshot action, destination picker and daily/weekly file-backup timers. Scheduled backups use a headless executable mode and keep secrets out of unit files.
- Date & language has installed language/region choices and clock-format presets. Locale spelling aliases are handled, and clock-only edits do not request a system locale change.
- Storage has unused-dependency review and selected-only removal, plus measured directory totals. Btrfs figures are explicitly identified as overlapping directory usage; they are not presented as disjoint or reclaimable bytes.
- About runs typed bread/bakery health checks and shows an OS version only if `/etc/os-release` publishes `VERSION_ID`.
- Sidebar has the account footer, update count and the proposal's new-page markers. Display/UI fonts are bundled with their OFL licenses.

BSET-44, BSET-32 and BAR-56 are ready for review after the headless checks. BSET-43 remains open for the user's full UI verification. Rendered pixel accuracy has not been tested.

## Additional capability changes for review

- Tabbed hub pages now display all sections together. Legacy tab destinations scroll to their section.
- Raw startup and shortcut commands remain available under advanced disclosures. Core service editing is still guarded.
- The duplicate natural-scroll control in the old Mouse subpanel was consolidated into the persisted Mouse setting. Focus options remain.
- Recommended apps, installed packages, manual bar restarts, raw bar CSS, zoom number/Apply controls, snapshot deletion and GRUB boot actions remain available.
- A later edit of bar-specific CSS takes precedence over the global text preference, keeping the existing advanced bar text controls usable.
- Snapshot and directory figures represent real readings. Mockup-only app ratings, trust claims and sample device statistics are not emitted as real data.

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
