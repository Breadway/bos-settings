//! Initial-page handoff for `--page <id>`.
//!
//! `page::parse` reads `--page` from argv before Tauri or the webview
//! exist, but the frontend can only see it through a command or an event.
//! An event emitted from `setup()` would race the frontend's `listen()`
//! call not being attached yet (the same race `screenshot::dispatch` papers
//! over with a fixed [`screenshot::INITIAL_SETTLE_DELAY`] sleep before its
//! `screenshot-set-view` emit). A plain command sidesteps that: the
//! frontend calls it once in `+page.svelte`'s `onMount`, before deciding
//! which view to render, no delay needed.

use std::sync::Mutex;

/// Holds the `--page` value parsed at startup, managed once via
/// `app.manage()` in `lib.rs::run()`.
pub struct InitialPage(pub Mutex<Option<String>>);

/// Returns the `--page` value from this launch, if any, and clears it so a
/// later window reload (e.g. opening devtools) doesn't keep re-navigating
/// away from wherever the user has since moved.
#[tauri::command]
pub fn get_initial_page(state: tauri::State<InitialPage>) -> Option<String> {
    state.0.lock().unwrap_or_else(|e| e.into_inner()).take()
}
