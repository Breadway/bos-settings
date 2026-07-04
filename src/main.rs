mod config;
mod theme;
mod ui;

use gtk4::gio::ApplicationFlags;
use gtk4::prelude::*;

fn main() {
    let app = gtk4::Application::builder()
        .application_id("com.breadway.bos-settings")
        // HANDLES_COMMAND_LINE: without it, GApplication validates argv
        // against registered GOptionEntries (none here) and would reject
        // `--page <id>` with "unknown option" before `activate` ever runs.
        .flags(ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.connect_command_line(|app, cmdline| {
        let page = parse_page_arg(&cmdline.arguments());
        ui::window::build_ui(app, page);
        glib::ExitCode::SUCCESS
    });

    app.run();
}

/// Best-effort: like any non-command-line-aware launch of an already-running
/// GApplication, this is only seen by the launch that becomes primary — a
/// `--page` while bos-settings is already open just refocuses the existing
/// window on whatever page it was already showing.
fn parse_page_arg(args: &[std::ffi::OsString]) -> Option<String> {
    let mut it = args.iter().skip(1);
    while let Some(arg) = it.next() {
        if arg == "--page" {
            return it.next().and_then(|s| s.to_str()).map(str::to_string);
        }
    }
    None
}
