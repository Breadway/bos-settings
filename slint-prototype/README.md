# BOS Settings, Slint prototype

A Slint version of the settings window, built page for page against the
shipping app. It reads the same files and commands the shipping app reads; it
does not write to them yet.

```sh
cargo build --offline
./target/debug/bos-settings-slint --page updates
```

Without `--page` it opens on Home. `--page` accepts any sidebar id, including
the hub ids (`desktop`, `keyboard`, `privacy`, `restore`, `getapps`, `about`).

## Parity check

`parity-capture.sh` renders the shipping app and the prototype at 1920, 960 and
860 wide on the headless sway output, and `scripts/pixel-diff.py` compares them.
A pixel counts as different when its luma differs by more than 40; a page fails
when more than 1% of its pixels differ.

```sh
./parity-capture.sh
scripts/pixel-diff.py /tmp/opencode/ref3 /tmp/opencode/shot3 \
    --widths 1920,960,860 --diffs /tmp/opencode/diffs
```

The shipping app's `--screenshot --width` crops a 1920 render instead of
resizing the webview, so the capture script uses a real resized window at 960
and 860. Both scripts are test tooling only; neither is part of the app.

Text is rasterized by Slint's software renderer here and by WebKit in the
shipping app. With the same layout, glyph edges still differ enough to count at
this threshold, so the difference masks on text-heavy pages show letter edges,
not displaced rows. Cards, controls, spacing and icons are compared the same way
and are expected to line up.

### The one-pixel text baseline

Slint's 13.5px and 12px line boxes place the glyph tops one pixel lower than
WebKit does for the same row geometry. Wrapping those labels in fixed 17px and
15px boxes and offsetting the text by 1px moves the glyph bounding box onto the
shipping pixels exactly (checked against "Inactive workspaces" at 1920: both
ends read 323..352, 740..959). This is a real correction, not a blur hack, and
it removes a systematic error on every page.

It does not make the 1% limit reachable at 960 or 860. Once the text sits on the
same pixels, the remaining difference is glyph antialiasing: the same letters are
drawn with different edge coverage by the two rasterizers. That error is
proportional to the amount of text, not to its placement, so the changed-pixel
fraction roughly doubles when the same page is measured at half the width. A
page with this much small text cannot pass the strict check at 860 without
using WebKit's rasterizer; the check stays strict and reports those pages.

## Deliberate divergences

These are choices, not missing work. Everything else aims to match the shipping
app at the pixel level.

- **One generic renderer.** The pages are data (`src/pagedata.rs`) drawn by
  `ui/spec.slint` with the shipping app's Group, Row, Hero, chip and device
  pieces, instead of one Svelte component per view. Shared geometry is therefore
  identical across pages, and a shipping layout change is a data change here.
- **Read-only.** Toggles, selects, sliders and chips remember their position in
  `~/.local/state/bos-settings-slint/state.toml`, but no system command runs.
  The shipping app's write paths are still to be ported.
- **Icons.** `@lucide/svelte` is not installed on this machine, so a checked
  subset of Lucide paths lives in `src/extra_icons.rs` and misses nothing the
  prototype uses. An icon the subset does not know renders blank rather than
  as a wrong glyph.
- **Keyboard shortcuts.** Laid out as label and key-binding rows in the same
  groups, not as the shipping app's filterable table with kbd chips. The data
  comes from the same `~/.config/hypr/binds.json`.
- **Get apps.** Shows the recommended entries and the installed package count;
  there is no live repository search, because the search belongs to the write
  path.
- **Hub pages.** Render every section of the hub stacked, as the shipping Hub
  does, without the scroll-to-tab plumbing.
- **Missing hardware.** Where the shipping app shows a friendly empty state
  (`No devices found.`, `No monitors detected.`) the prototype uses the same
  copy and the same group shape.
