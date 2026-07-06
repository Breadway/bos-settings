//! Bluetooth over `bluetoothctl`'s non-interactive mode (BlueZ's own CLI has
//! supported single-shot argv commands since 5.48 — no D-Bus dependency
//! needed here, same "shell out to the standard CLI" choice as Network
//! (nmcli) and Snapshots (snapper). Unlike Firewall, none of this needs
//! `pkexec` — BlueZ's D-Bus policy already allows the active session user.
//!
//! Pairing only covers "Just Works" Simple Secure Pairing — bluetoothd's own
//! built-in default agent auto-accepts that for most audio/HID devices with
//! no prompt. A device that requires PIN/passkey confirmation isn't
//! supported: that would need this app to register its own bluetoothd
//! agent, a bigger lift no bread-ecosystem app takes on today. Pairing such
//! a device just fails, surfaced via the log view below like any other
//! failure here.

use gtk4::prelude::*;
use gtk4::{
    AlertDialog, Box as GBox, Button, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow,
    Switch, TextView,
};
use std::collections::HashSet;
use std::process::Command;

use crate::ui::widgets as w;

#[derive(Clone)]
struct BtDevice {
    address: String,
    name: String,
    connected: bool,
}

/// `None` means no controller present at all (no Bluetooth hardware, or the
/// kernel module isn't loaded) — distinct from "present but powered off",
/// which still has plenty to show (the power switch itself).
fn adapter_powered() -> Option<bool> {
    let output = Command::new("bluetoothctl").arg("show").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    if !text.trim_start().starts_with("Controller") {
        return None;
    }
    Some(text.lines().any(|l| l.trim() == "Powered: yes"))
}

fn set_powered(on: bool) {
    let val = if on { "on" } else { "off" };
    let _ = Command::new("bluetoothctl").args(["power", val]).spawn();
}

/// Parses `bluetoothctl devices[...]` output — one `Device <MAC> <name>` per
/// line, name may contain spaces.
fn parse_device_lines(text: &str) -> Vec<BtDevice> {
    text.lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("Device ")?;
            let (addr, name) = rest.split_once(' ')?;
            Some(BtDevice {
                address: addr.trim().to_string(),
                name: name.trim().to_string(),
                connected: false,
            })
        })
        .collect()
}

fn run_devices(filter: Option<&str>) -> Vec<BtDevice> {
    let mut args = vec!["devices"];
    if let Some(f) = filter {
        args.push(f);
    }
    let Ok(out) = Command::new("bluetoothctl").args(&args).output() else {
        return Vec::new();
    };
    parse_device_lines(&String::from_utf8_lossy(&out.stdout))
}

fn list_paired() -> Vec<BtDevice> {
    let mut devices = run_devices(Some("Paired"));
    let connected: HashSet<String> =
        run_devices(Some("Connected")).into_iter().map(|d| d.address).collect();
    for d in &mut devices {
        d.connected = connected.contains(&d.address);
    }
    devices
}

/// Scans for a few seconds (blocking — call this on a background thread) and
/// returns every device BlueZ has seen that isn't already paired. `devices`
/// with no filter returns BlueZ's whole device cache (previously-seen plus
/// anything the scan just found), so filtering out the paired set gives
/// exactly "new, not-yet-paired" devices.
fn scan_unpaired() -> Vec<BtDevice> {
    let _ = Command::new("bluetoothctl").args(["--timeout", "5", "scan", "on"]).output();
    let paired: HashSet<String> = run_devices(Some("Paired")).into_iter().map(|d| d.address).collect();
    run_devices(None).into_iter().filter(|d| !paired.contains(&d.address)).collect()
}

fn populate_paired(list: &ListBox, log_buf: &gtk4::TextBuffer, log_view: &TextView) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let devices = list_paired();
    if devices.is_empty() {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        row.set_child(Some(&w::empty_state(
            "bluetooth-symbolic",
            "No paired devices",
            "Scan below and pair a device to see it here.",
        )));
        list.append(&row);
        return;
    }
    for dev in devices {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        let hbox = GBox::new(Orientation::Horizontal, 16);
        hbox.add_css_class("card");
        hbox.set_margin_top(3);
        hbox.set_margin_bottom(3);

        let name = if dev.connected {
            format!("{}  (connected)", dev.name)
        } else {
            dev.name.clone()
        };
        let name_lbl = Label::new(Some(&name));
        name_lbl.set_hexpand(true);
        name_lbl.set_xalign(0.0);
        if dev.connected {
            name_lbl.add_css_class("heading");
        }

        let toggle_btn = Button::with_label(if dev.connected { "Disconnect" } else { "Connect" });
        {
            let list = list.clone();
            let log_buf = log_buf.clone();
            let log_view = log_view.clone();
            let addr = dev.address.clone();
            let connected = dev.connected;
            toggle_btn.connect_clicked(move |_| {
                log_buf.set_text("");
                log_view.set_visible(true);
                let verb = if connected { "disconnect" } else { "connect" };
                let list2 = list.clone();
                let log_buf2 = log_buf.clone();
                let log_view2 = log_view.clone();
                w::stream_command_then(&["bluetoothctl", verb, &addr], log_buf.clone(), move || {
                    populate_paired(&list2, &log_buf2, &log_view2);
                });
            });
        }

        let forget_btn = Button::with_label("Forget");
        forget_btn.add_css_class("destructive-action");
        {
            let list = list.clone();
            let log_buf = log_buf.clone();
            let log_view = log_view.clone();
            let addr = dev.address.clone();
            let name_for_dialog = dev.name.clone();
            forget_btn.connect_clicked(move |btn| {
                let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
                let dialog = AlertDialog::builder()
                    .message(format!("Forget \"{name_for_dialog}\"?"))
                    .detail("You'll need to pair it again to reconnect.")
                    .buttons(["Cancel", "Forget"])
                    .cancel_button(0)
                    .default_button(0)
                    .build();
                let list = list.clone();
                let log_buf = log_buf.clone();
                let log_view = log_view.clone();
                let addr = addr.clone();
                dialog.choose(window.as_ref(), gtk4::gio::Cancellable::NONE, move |result| {
                    if result != Ok(1) {
                        return;
                    }
                    log_buf.set_text("");
                    log_view.set_visible(true);
                    let list2 = list.clone();
                    let log_buf2 = log_buf.clone();
                    let log_view2 = log_view.clone();
                    w::stream_command_then(
                        &["bluetoothctl", "remove", &addr],
                        log_buf.clone(),
                        move || populate_paired(&list2, &log_buf2, &log_view2),
                    );
                });
            });
        }

        hbox.append(&name_lbl);
        hbox.append(&toggle_btn);
        hbox.append(&forget_btn);
        row.set_child(Some(&hbox));
        list.append(&row);
    }
}

fn populate_scan(
    list: &ListBox,
    devices: Vec<BtDevice>,
    paired_list: &ListBox,
    log_buf: &gtk4::TextBuffer,
    log_view: &TextView,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    if devices.is_empty() {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        row.set_child(Some(&w::empty_state(
            "bluetooth-symbolic",
            "No new devices found",
            "Make sure the device is powered on and in pairing mode, then Scan again.",
        )));
        list.append(&row);
        return;
    }
    for dev in devices {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        let hbox = GBox::new(Orientation::Horizontal, 16);
        hbox.add_css_class("card");
        hbox.set_margin_top(3);
        hbox.set_margin_bottom(3);

        let name_lbl = Label::new(Some(&dev.name));
        name_lbl.set_hexpand(true);
        name_lbl.set_xalign(0.0);

        let pair_btn = Button::with_label("Pair");
        {
            let list = list.clone();
            let paired_list = paired_list.clone();
            let log_buf = log_buf.clone();
            let log_view = log_view.clone();
            let addr = dev.address.clone();
            // A weak ref to just this row: on success we remove only it
            // (cheap and correct) rather than re-running the 5s scan on the
            // main thread — `stream_command_then`'s completion callback runs
            // on the GTK main loop, so anything blocking in it would freeze
            // the whole window for the scan's duration.
            let row_weak = row.downgrade();
            pair_btn.connect_clicked(move |_| {
                log_buf.set_text("");
                log_view.set_visible(true);
                let list2 = list.clone();
                let paired_list2 = paired_list.clone();
                let log_buf2 = log_buf.clone();
                let log_view2 = log_view.clone();
                let addr2 = addr.clone();
                let row_weak2 = row_weak.clone();
                w::stream_command_then(&["bluetoothctl", "pair", &addr], log_buf.clone(), move || {
                    // Both of these are one-shot device-cache lookups, not a
                    // scan, so they return in well under a second.
                    populate_paired(&paired_list2, &log_buf2, &log_view2);
                    let now_paired = run_devices(Some("Paired")).iter().any(|d| d.address == addr2);
                    if !now_paired {
                        return; // pairing failed — leave the row so the user can retry, error is in the log view above
                    }
                    if let Some(row) = row_weak2.upgrade() {
                        list2.remove(&row);
                    }
                    if list2.first_child().is_none() {
                        let empty_row = ListBoxRow::new();
                        empty_row.set_selectable(false);
                        empty_row.set_child(Some(&w::empty_state(
                            "bluetooth-symbolic",
                            "No new devices found",
                            "Make sure the device is powered on and in pairing mode, then Scan again.",
                        )));
                        list2.append(&empty_row);
                    }
                });
            });
        }

        hbox.append(&name_lbl);
        hbox.append(&pair_btn);
        row.set_child(Some(&hbox));
        list.append(&row);
    }
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Bluetooth");

    let Some(powered) = adapter_powered() else {
        content.append(&w::empty_state(
            "bluetooth-disabled-symbolic",
            "No Bluetooth adapter found",
            "This machine doesn't have Bluetooth hardware, or the kernel module isn't loaded.",
        ));
        return outer;
    };

    content.append(&w::section("Adapter"));
    let power_sw = Switch::new();
    power_sw.set_active(powered);
    power_sw.connect_active_notify(|s| set_powered(s.is_active()));
    content.append(&w::row("Bluetooth", &power_sw));

    content.append(&w::section("Paired devices"));
    let paired_list = ListBox::new();
    paired_list.set_selection_mode(gtk4::SelectionMode::None);
    let paired_scroll = ScrolledWindow::new();
    paired_scroll.set_min_content_height(160);
    paired_scroll.set_child(Some(&paired_list));
    content.append(&paired_scroll);

    content.append(&w::section("Available devices"));
    content.append(&w::hint(
        "Scanning takes a few seconds. Devices that need a PIN to pair \
         aren't supported here — only \"just works\" pairing (most \
         headphones, speakers, keyboards, and mice).",
    ));
    let scan_list = ListBox::new();
    scan_list.set_selection_mode(gtk4::SelectionMode::None);
    {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        row.set_child(Some(&w::empty_state(
            "bluetooth-symbolic",
            "Not scanned yet",
            "Press Scan to look for nearby devices.",
        )));
        scan_list.append(&row);
    }
    let scan_scroll = ScrolledWindow::new();
    scan_scroll.set_min_content_height(160);
    scan_scroll.set_child(Some(&scan_list));
    content.append(&scan_scroll);

    // Shared log view for every connect/disconnect/pair/forget action on
    // this page — same "don't discard the reason for a failure" pattern
    // Firewall uses, since a failed pair (PIN required, device out of
    // range, ...) is exactly the case a user needs an explanation for.
    let log_buf = gtk4::TextBuffer::new(None);
    let log_view = TextView::with_buffer(&log_buf);
    log_view.set_editable(false);
    log_view.set_monospace(true);
    log_view.set_height_request(120);
    log_view.set_margin_top(8);
    log_view.set_visible(false);

    populate_paired(&paired_list, &log_buf, &log_view);

    let scan_btn = Button::with_label("Scan");
    scan_btn.set_halign(gtk4::Align::Start);
    scan_btn.set_margin_top(6);
    {
        let scan_list = scan_list.clone();
        let paired_list = paired_list.clone();
        let log_buf = log_buf.clone();
        let log_view = log_view.clone();
        scan_btn.connect_clicked(move |btn| {
            btn.set_sensitive(false);
            let (tx, rx) = async_channel::bounded::<Vec<BtDevice>>(1);
            std::thread::spawn(move || {
                let _ = tx.send_blocking(scan_unpaired());
            });
            let btn = btn.clone();
            let scan_list = scan_list.clone();
            let paired_list = paired_list.clone();
            let log_buf = log_buf.clone();
            let log_view = log_view.clone();
            glib::spawn_future_local(async move {
                if let Ok(devices) = rx.recv().await {
                    populate_scan(&scan_list, devices, &paired_list, &log_buf, &log_view);
                }
                btn.set_sensitive(true);
            });
        });
    }
    content.append(&scan_btn);
    content.append(&log_view);

    outer
}
