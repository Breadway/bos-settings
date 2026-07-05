//! ufw firewall rules — BOS enables ufw by default (deny incoming, allow
//! outgoing, mDNS allowed for printer discovery; see post-install.sh) but
//! previously offered no graphical way to add/remove a rule for something
//! like a local dev server or a LAN game, only a terminal.
//!
//! `ufw status` itself requires root — confirmed against the installed
//! ufw script, which exits with "ERROR: You need to be root to run this
//! script" for a plain status check, not just for changes. Every other
//! panel's read-only state is free to query eagerly in `build()`, but doing
//! that here would mean a polkit password prompt on every single
//! bos-settings launch (every view is constructed immediately at startup —
//! see window.rs). So this panel starts blank and loads state only when the
//! user clicks Refresh, deferring the one unavoidable prompt to an explicit
//! action instead of forcing it on app open.

use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, Switch, TextView};
use std::cell::Cell;
use std::rc::Rc;

use crate::ui::widgets as w;

#[derive(Clone)]
struct Rule {
    number: String,
    text: String,
}

struct Status {
    active: bool,
    rules: Vec<Rule>,
}

/// One `pkexec ufw status numbered` call, parsed for both the active/inactive
/// line and the numbered rules — a single privileged read instead of two.
/// `Err` carries stderr (or a description of the exec failure) so callers can
/// show *why* it failed instead of leaving the page silently on "Status not
/// loaded" forever — previously this returned `Option<Status>`, discarding
/// the reason entirely, so a failed pkexec call (wrong password, no polkit
/// agent running in the session, ufw missing, ...) looked identical to
/// simply never having clicked Refresh.
fn fetch_status() -> Result<Status, String> {
    let output = std::process::Command::new("pkexec")
        .args(["ufw", "status", "numbered"])
        .output()
        .map_err(|e| format!("couldn't run pkexec: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            match output.status.code() {
                Some(127) => "no polkit authentication agent is available in this session".to_string(),
                Some(code) => format!("pkexec exited with status {code}"),
                None => "pkexec was terminated by a signal".to_string(),
            }
        } else {
            stderr
        });
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let active = text.lines().next().is_some_and(|l| l.trim() == "Status: active");
    let rules = text
        .lines()
        .filter_map(|l| {
            let l = l.trim_start();
            if !l.starts_with('[') {
                return None;
            }
            let (num, rest) = l.split_once(']')?;
            let number = num.trim_start_matches('[').trim().to_string();
            Some(Rule { number, text: rest.trim().to_string() })
        })
        .collect();
    Ok(Status { active, rules })
}

fn render_rules(list: &ListBox, rules: &[Rule], programmatic: &Rc<Cell<bool>>, log_buf: &gtk4::TextBuffer, log_view: &TextView) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    if rules.is_empty() {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        row.set_child(Some(&w::empty_state(
            "security-high-symbolic",
            "No rules",
            "Only the default policy (deny incoming, allow outgoing) applies.",
        )));
        list.append(&row);
        return;
    }
    for rule in rules {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        let hbox = GBox::new(Orientation::Horizontal, 16);
        hbox.add_css_class("card");
        hbox.set_margin_top(3);
        hbox.set_margin_bottom(3);

        let text_lbl = Label::new(Some(&rule.text));
        text_lbl.set_hexpand(true);
        text_lbl.set_xalign(0.0);
        text_lbl.set_wrap(true);

        let remove_btn = Button::with_label("Remove");
        remove_btn.add_css_class("destructive-action");
        {
            let list = list.clone();
            let number = rule.number.clone();
            let programmatic = programmatic.clone();
            let log_buf = log_buf.clone();
            let log_view = log_view.clone();
            remove_btn.connect_clicked(move |_| {
                log_buf.set_text("");
                log_view.set_visible(true);
                let list2 = list.clone();
                let programmatic2 = programmatic.clone();
                let log_buf2 = log_buf.clone();
                let log_view2 = log_view.clone();
                w::stream_command_then(
                    &["pkexec", "ufw", "--force", "delete", &number],
                    log_buf.clone(),
                    move || refresh(&list2, None, &programmatic2, &log_buf2, &log_view2),
                );
            });
        }

        hbox.append(&text_lbl);
        hbox.append(&remove_btn);
        row.set_child(Some(&hbox));
        list.append(&row);
    }
}

/// Shows `message` as the list's only row, in place of the rule list —
/// used both for the initial "not loaded yet" state and for a failed
/// Refresh, so a failure looks like a state, not a no-op.
fn render_message(list: &ListBox, icon: &str, title: &str, detail: &str) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let row = ListBoxRow::new();
    row.set_selectable(false);
    row.set_child(Some(&w::empty_state(icon, title, detail)));
    list.append(&row);
}

/// Re-fetch status on a background thread and update the list (+ switch, if
/// given) on completion. `enabled_sw` is `None` when called from a row
/// action (delete/add) where the enabled state can't have changed.
///
/// `programmatic` guards against `set_active` below re-triggering the
/// switch's own `connect_active_notify` handler (which calls `ufw enable`/
/// `disable`) — without it, the very first Refresh after ufw turns out to
/// already be active would immediately fire an unwanted `ufw enable` the
/// moment `set_active(true)` flips a switch that just became sensitive.
fn refresh(list: &ListBox, enabled_sw: Option<&Switch>, programmatic: &Rc<Cell<bool>>, log_buf: &gtk4::TextBuffer, log_view: &TextView) {
    let (tx, rx) = async_channel::bounded::<Result<Status, String>>(1);
    std::thread::spawn(move || {
        let _ = tx.send_blocking(fetch_status());
    });
    let list = list.clone();
    let enabled_sw = enabled_sw.cloned();
    let programmatic = programmatic.clone();
    let log_buf = log_buf.clone();
    let log_view = log_view.clone();
    glib::spawn_future_local(async move {
        match rx.recv().await {
            Ok(Ok(status)) => {
                render_rules(&list, &status.rules, &programmatic, &log_buf, &log_view);
                if let Some(sw) = &enabled_sw {
                    programmatic.set(true);
                    sw.set_sensitive(true);
                    sw.set_active(status.active);
                    programmatic.set(false);
                }
            }
            Ok(Err(reason)) => {
                render_message(
                    &list,
                    "dialog-warning-symbolic",
                    "Couldn't read firewall status",
                    &reason,
                );
            }
            Err(_) => {
                render_message(
                    &list,
                    "dialog-warning-symbolic",
                    "Couldn't read firewall status",
                    "the background check never reported back",
                );
            }
        }
    });
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Firewall");
    content.append(&w::hint(
        "Reading and changing firewall state needs your password (polkit) \
         — ufw requires root even just to check status. Click Refresh to \
         load the current state.",
    ));

    let programmatic = Rc::new(Cell::new(false));

    let enabled_sw = Switch::new();
    enabled_sw.set_sensitive(false);
    content.append(&w::row("Firewall enabled", &enabled_sw));
    content.append(&w::hint(
        "Default policy: deny incoming, allow outgoing. mDNS (5353/udp) is \
         allowed by default so printer/network discovery keeps working.",
    ));

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    render_message(
        &list,
        "security-high-symbolic",
        "Status not loaded",
        "Click Refresh below to check the firewall's current state.",
    );
    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_min_content_height(260);
    scroll.set_child(Some(&list));
    content.append(&scroll);

    // Shared log view for every pkexec/ufw call on this page (refresh,
    // enable/disable, add, remove) — previously each action created its own
    // throwaway `TextBuffer::new(None)` that was never attached to any
    // visible widget, so stderr/stdout from a failing command (including
    // pkexec itself failing) went nowhere the user could see.
    let log_buf = gtk4::TextBuffer::new(None);
    let log_view = TextView::with_buffer(&log_buf);
    log_view.set_editable(false);
    log_view.set_monospace(true);
    log_view.set_height_request(140);
    log_view.set_margin_top(8);
    log_view.set_visible(false);

    let refresh_btn = Button::with_label("Refresh status");
    {
        let list = list.clone();
        let enabled_sw = enabled_sw.clone();
        let programmatic = programmatic.clone();
        let log_buf = log_buf.clone();
        let log_view = log_view.clone();
        refresh_btn.connect_clicked(move |_| refresh(&list, Some(&enabled_sw), &programmatic, &log_buf, &log_view));
    }
    content.append(&refresh_btn);

    {
        let list = list.clone();
        let programmatic = programmatic.clone();
        let log_buf = log_buf.clone();
        let log_view = log_view.clone();
        enabled_sw.connect_active_notify(move |s| {
            // Skip both the pre-refresh insensitive state and any
            // programmatic set_active() from refresh() itself — only a
            // real user click should call out to ufw enable/disable.
            if !s.is_sensitive() || programmatic.get() {
                return;
            }
            let verb = if s.is_active() { "enable" } else { "disable" };
            log_buf.set_text("");
            log_view.set_visible(true);
            let list2 = list.clone();
            let programmatic2 = programmatic.clone();
            let log_buf2 = log_buf.clone();
            let log_view2 = log_view.clone();
            w::stream_command_then(&["pkexec", "ufw", "--force", verb], log_buf.clone(), move || {
                refresh(&list2, None, &programmatic2, &log_buf2, &log_view2);
            });
        });
    }

    content.append(&w::section("Add rule"));
    content.append(&w::hint(
        "e.g. \"8080/tcp\", \"22/tcp\", or a service name like \"OpenSSH\".",
    ));
    let add_row = GBox::new(Orientation::Horizontal, 8);
    let add_entry = Entry::new();
    add_entry.set_hexpand(true);
    add_entry.set_placeholder_text(Some("port/proto or service name"));
    let add_btn = Button::with_label("Allow");
    {
        let list = list.clone();
        let add_entry = add_entry.clone();
        let programmatic = programmatic.clone();
        let log_buf = log_buf.clone();
        let log_view = log_view.clone();
        add_btn.connect_clicked(move |_| {
            let rule = add_entry.text().to_string();
            if rule.trim().is_empty() {
                return;
            }
            log_buf.set_text("");
            log_view.set_visible(true);
            let list2 = list.clone();
            let add_entry2 = add_entry.clone();
            let programmatic2 = programmatic.clone();
            let log_buf2 = log_buf.clone();
            let log_view2 = log_view.clone();
            w::stream_command_then(
                &["pkexec", "ufw", "allow", rule.trim()],
                log_buf.clone(),
                move || {
                    add_entry2.set_text("");
                    refresh(&list2, None, &programmatic2, &log_buf2, &log_view2);
                },
            );
        });
    }
    add_row.append(&add_entry);
    add_row.append(&add_btn);
    content.append(&add_row);

    content.append(&log_view);

    outer
}
