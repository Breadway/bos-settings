import assert from "node:assert/strict";
import { snapshotLabel } from "./snapshotLabel.ts";

assert.equal(snapshotLabel("/usr/bin/pacman -U --noconfirm -- /tmp/codex-update-manager-484822-17910"), "Before installing codex-update-manager");
assert.equal(snapshotLabel("pacman -U --noconfirm /home/b/dist/codex-desktop-1.2.0-1-x86_64.pkg.tar.zst"), "Before installing codex-desktop");
assert.equal(snapshotLabel("pacman -Rs --noconfirm bos-settings breadhelp calamares python-pywal"), "Before removing bos-settings, breadhelp, calamares and 1 more");
assert.equal(snapshotLabel("pacman -Syu"), "Before updating the system");
assert.equal(snapshotLabel("pacman -S --needed --noconfirm gnome-disk-utility gufw"), "Before installing gnome-disk-utility, gufw");
assert.equal(snapshotLabel("codex-desktop", "post"), "After changing codex-desktop");
assert.equal(snapshotLabel("pre vanilla-BOS alignment", "single"), "pre vanilla-BOS alignment");
console.log("snapshotLabel: pacman pre/post descriptions read as sentences");
