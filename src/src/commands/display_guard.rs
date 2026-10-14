//! Backend-owned keep-or-revert safeguard for display changes.
//!
//! Every change to resolution, scale, rotation, position or mirroring goes
//! through [`DisplayGuard::apply`]. It captures the complete live layout
//! (mirrors included) first, applies the new one temporarily, checks that the
//! compositor really ended up there, and then holds the snapshot while a
//! deadline runs. The deadline lives here, in a backend thread, not in the
//! webview: leaving the page, reloading it or closing the window cannot cancel
//! the revert. Only [`DisplayGuard::keep`] writes anything to disk.
//!
//! All compositor access goes through the [`Compositor`] trait so the whole
//! flow is tested against a mock and never touches a real session.

use std::collections::HashSet;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::Serialize;

use super::hyprland::{self, LiveMonitor};

/// The three things the guard needs from the compositor.
pub trait Compositor: Send + Sync + 'static {
    /// Every output, including turned-off ones and mirrors.
    fn read(&self) -> Result<Vec<LiveMonitor>, String>;
    /// Run Lua through `hyprctl eval`; Ok only when the reply was "ok".
    fn eval(&self, lua: &str) -> Result<(), String>;
    /// Write the layout to the login configuration.
    fn persist(&self, layout: &[LiveMonitor]) -> Result<(), String>;
}

pub struct HyprCompositor;

impl Compositor for HyprCompositor {
    fn read(&self) -> Result<Vec<LiveMonitor>, String> {
        hyprland::read_all_monitors()
    }
    fn eval(&self, lua: &str) -> Result<(), String> {
        hyprland::hyprctl_eval(lua)
    }
    fn persist(&self, layout: &[LiveMonitor]) -> Result<(), String> {
        hyprland::persist_layout(layout)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Timing {
    /// Default length of the keep-or-revert window.
    pub window: Duration,
    /// How often the pending change is checked for a disconnected display.
    pub poll: Duration,
    /// Pause between attempts to read back the compositor state after a change.
    pub settle: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            window: Duration::from_secs(15),
            poll: Duration::from_secs(1),
            settle: Duration::from_millis(300),
        }
    }
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OutcomeKind {
    Kept,
    Reverted,
    TimedOut,
    Disconnected,
    RevertFailed,
    SaveFailed,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct GuardOutcome {
    pub id: u64,
    pub kind: OutcomeKind,
    pub message: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PendingInfo {
    pub id: u64,
    pub seconds_left: u64,
    pub seconds_total: u64,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct GuardStatus {
    pub pending: Option<PendingInfo>,
    pub last: Option<GuardOutcome>,
}

struct Pending {
    id: u64,
    before: Vec<LiveMonitor>,
    after: Vec<LiveMonitor>,
    deadline: Instant,
    total: Duration,
}

#[derive(Default)]
struct State {
    pending: Option<Pending>,
    next_id: u64,
    last: Option<GuardOutcome>,
}

type Notify = Arc<dyn Fn(&GuardOutcome) + Send + Sync>;

/// Where the guard leaves a crash-recovery record and how it starts the
/// out-of-process watchdog (see display_rescue). None in tests.
pub struct Rescue {
    pub path: std::path::PathBuf,
    pub spawn: fn(u64) -> Result<(), String>,
    /// Whether the process that wrote a record is still a running BOS Settings.
    pub alive: fn(u32) -> bool,
}

struct Inner {
    comp: Arc<dyn Compositor>,
    timing: Timing,
    rescue: Option<Rescue>,
    notify: Notify,
    /// Serialises apply / keep / revert / timeout so two of them can never
    /// drive the compositor at once. Always taken before `state`.
    op: Mutex<()>,
    state: Mutex<State>,
    wake: Condvar,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Managed Tauri state.
#[derive(Clone)]
pub struct DisplayGuard {
    inner: Arc<Inner>,
}

// ---------------------------------------------------------------------
// Pure planning: validation, statements, comparison.
// ---------------------------------------------------------------------

/// Reject a layout that must never reach the compositor.
pub fn validate_layout(layout: &[LiveMonitor]) -> Result<(), String> {
    if layout.is_empty() {
        return Err("There are no displays to apply.".into());
    }
    let mut seen = HashSet::new();
    for m in layout {
        hyprland::lua_ident(&m.name)?;
        if !seen.insert(m.name.as_str()) {
            return Err(format!("{} is listed twice.", m.name));
        }
        if m.disabled {
            continue;
        }
        if m.width == 0 || m.height == 0 || !m.refresh.is_finite() {
            return Err(format!("{} has no usable resolution.", m.name));
        }
        if !m.scale.is_finite() || !(0.1..=10.0).contains(&m.scale) {
            return Err(format!("{} has an unusable scale.", m.name));
        }
        if m.transform > 7 || m.vrr > 2 {
            return Err(format!(
                "{} has an unusable rotation or sync value.",
                m.name
            ));
        }
    }
    if layout.iter().all(|m| m.disabled) {
        return Err("At least one display has to stay on, so this change was not applied.".into());
    }
    for m in layout
        .iter()
        .filter(|m| !m.disabled && !m.mirror_of.is_empty())
    {
        hyprland::lua_ident(&m.mirror_of)?;
        if m.mirror_of == m.name {
            return Err(format!("{} cannot mirror itself.", m.name));
        }
        match layout.iter().find(|o| o.name == m.mirror_of) {
            None => {
                return Err(format!(
                    "{} cannot mirror {}: that display is not connected.",
                    m.name, m.mirror_of
                ))
            }
            Some(src) if src.disabled => {
                return Err(format!(
                    "{} cannot mirror {}: that display is turned off.",
                    m.name, m.mirror_of
                ))
            }
            Some(src) if !src.mirror_of.is_empty() => {
                return Err(format!(
                    "{} cannot mirror {}: it is mirroring another display.",
                    m.name, m.mirror_of
                ))
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// One `hl.monitor` statement per output, mirror sources first so a mirror
/// never refers to an output that has not been configured yet.
pub fn plan_statements(layout: &[LiveMonitor]) -> Result<Vec<String>, String> {
    let mut ordered: Vec<&LiveMonitor> = layout.iter().collect();
    ordered.sort_by_key(|m| !m.mirror_of.is_empty());
    ordered.into_iter().map(hyprland::lua_statement).collect()
}

/// First difference between what was asked for and what the compositor
/// reports, or None when they agree. With `allow_missing` an expected output
/// that is no longer connected is skipped (used when restoring).
pub fn layout_mismatch(
    expected: &[LiveMonitor],
    actual: &[LiveMonitor],
    allow_missing: bool,
) -> Option<String> {
    if actual.iter().all(|a| a.disabled) {
        return Some("no display is on".into());
    }
    for e in expected {
        let Some(a) = actual.iter().find(|a| a.name == e.name) else {
            if allow_missing {
                continue;
            }
            return Some(format!("{} is no longer connected", e.name));
        };
        if e.disabled != a.disabled {
            return Some(format!(
                "{} is not {}",
                e.name,
                if e.disabled { "off" } else { "on" }
            ));
        }
        if e.disabled {
            continue;
        }
        if e.mirror_of != a.mirror_of {
            return Some(format!(
                "{} mirrors {} instead of {}",
                e.name,
                or_none(&a.mirror_of),
                or_none(&e.mirror_of)
            ));
        }
        if (e.width, e.height) != (a.width, a.height) || (e.refresh - a.refresh).abs() > 0.5 {
            return Some(format!(
                "{} runs {}x{}@{:.2} instead of {}x{}@{:.2}",
                e.name, a.width, a.height, a.refresh, e.width, e.height, e.refresh
            ));
        }
        if (e.scale - a.scale).abs() > 0.01 {
            return Some(format!(
                "{} scale is {} instead of {}",
                e.name, a.scale, e.scale
            ));
        }
        if e.transform != a.transform {
            return Some(format!("{} rotation did not change", e.name));
        }
        if (e.vrr > 0) != (a.vrr > 0) {
            return Some(format!("{} adaptive sync did not change", e.name));
        }
        if e.mirror_of.is_empty() && (e.x, e.y) != (a.x, a.y) {
            return Some(format!(
                "{} sits at {},{} instead of {},{}",
                e.name, a.x, a.y, e.x, e.y
            ));
        }
    }
    None
}

fn or_none(s: &str) -> &str {
    if s.is_empty() {
        "nothing"
    } else {
        s
    }
}

/// Why a pending layout can no longer stand, judged from the live state: an
/// output it relies on was unplugged, or nothing is on any more.
pub fn disconnect_reason(after: &[LiveMonitor], actual: &[LiveMonitor]) -> Option<String> {
    for m in after.iter().filter(|m| !m.disabled) {
        if !actual.iter().any(|a| a.name == m.name) {
            return Some(format!("{} was disconnected.", m.name));
        }
    }
    if actual.iter().all(|a| a.disabled) {
        return Some("No display is on any more.".into());
    }
    None
}

// ---------------------------------------------------------------------
// The guard.
// ---------------------------------------------------------------------

impl DisplayGuard {
    pub fn new<F>(comp: Arc<dyn Compositor>, timing: Timing, notify: F) -> Self
    where
        F: Fn(&GuardOutcome) + Send + Sync + 'static,
    {
        Self {
            inner: Arc::new(Inner {
                comp,
                timing,
                rescue: None,
                notify: Arc::new(notify),
                op: Mutex::new(()),
                // Seeded from the clock, so ids from separate runs never meet
                // in the crash-recovery record (BSET-67 review).
                state: Mutex::new(State {
                    next_id: super::display_rescue::now_ms().saturating_mul(1000),
                    ..State::default()
                }),
                wake: Condvar::new(),
            }),
        }
    }

    pub fn production<F>(notify: F) -> Self
    where
        F: Fn(&GuardOutcome) + Send + Sync + 'static,
    {
        let mut g = Self::new(Arc::new(HyprCompositor), Timing::default(), notify);
        if let Some(path) = super::display_rescue::record_path() {
            g.set_rescue(Rescue {
                path,
                spawn: super::display_rescue::spawn_watchdog,
                alive: super::display_rescue::owner_alive,
            });
        }
        g
    }

    /// Only valid before the guard is shared.
    pub fn set_rescue(&mut self, rescue: Rescue) {
        if let Some(inner) = Arc::get_mut(&mut self.inner) {
            inner.rescue = Some(rescue);
        }
    }

    pub fn status(&self) -> GuardStatus {
        self.inner.status()
    }

    /// Apply `target` temporarily and start the revert deadline. `seconds`
    /// is clamped to 5..=60 and defaults to the configured window.
    pub fn apply(
        &self,
        target: Vec<LiveMonitor>,
        seconds: Option<u64>,
    ) -> Result<GuardStatus, String> {
        let window = seconds
            .map(|s| Duration::from_secs(s.clamp(5, 60)))
            .unwrap_or(self.inner.timing.window);
        self.apply_for(target, window)
    }

    pub fn apply_for(
        &self,
        target: Vec<LiveMonitor>,
        window: Duration,
    ) -> Result<GuardStatus, String> {
        Inner::apply(&self.inner, target, window)
    }

    /// Persist the pending layout and end the countdown.
    pub fn keep(&self, id: u64) -> Result<GuardOutcome, String> {
        self.inner.keep(id)
    }

    /// Restore the captured layout now. Idempotent for a change that was
    /// already reverted.
    pub fn revert(&self, id: u64) -> Result<GuardOutcome, String> {
        self.inner.revert(id)
    }

    /// Called when the app is exiting: a change nobody confirmed is undone
    /// before the process goes away.
    pub fn revert_on_exit(&self) {
        self.inner.revert_on_exit();
    }
}

impl Inner {
    fn status(&self) -> GuardStatus {
        let st = lock(&self.state);
        GuardStatus {
            pending: st.pending.as_ref().map(|p| PendingInfo {
                id: p.id,
                seconds_left: p
                    .deadline
                    .saturating_duration_since(Instant::now())
                    .as_secs_f64()
                    .ceil() as u64,
                seconds_total: p.total.as_secs(),
            }),
            last: st.last.clone(),
        }
    }

    /// Read the compositor until it agrees with `expected`, or give up after
    /// a few attempts and report the last difference.
    fn confirm(&self, expected: &[LiveMonitor], allow_missing: bool) -> Result<(), String> {
        let mut last = String::from("could not read the display state");
        for attempt in 0..4 {
            if attempt > 0 {
                std::thread::sleep(self.timing.settle);
            }
            match self.comp.read() {
                Ok(actual) => match layout_mismatch(expected, &actual, allow_missing) {
                    None => return Ok(()),
                    Some(why) => last = why,
                },
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    fn arm_rescue(&self, id: u64, before: &[LiveMonitor], window: Duration) -> Result<(), String> {
        let Some(r) = &self.rescue else { return Ok(()) };
        let rec = super::display_rescue::Record {
            id,
            pid: std::process::id(),
            deadline_ms: super::display_rescue::now_ms() + window.as_millis() as u64,
            before: before.to_vec(),
        };
        super::display_rescue::write_record(&r.path, &rec).map_err(|e| {
            format!("The safety record could not be written ({e}), so nothing was changed.")
        })?;
        if let Err(e) = (r.spawn)(id) {
            super::display_rescue::clear_record(&r.path, id);
            return Err(format!(
                "The safety watchdog could not start ({e}), so nothing was changed."
            ));
        }
        Ok(())
    }

    /// A record from another run must never be overwritten: its `before` is
    /// the only copy of the layout that run promised to restore. If that run
    /// is gone, put its layout back now (its own watchdog then finds the
    /// record cleared and exits); if it is still running, refuse.
    fn settle_foreign_record(&self) -> Result<(), String> {
        use super::display_rescue as dr;
        let Some(r) = &self.rescue else { return Ok(()) };
        let Some(rec) = dr::read_record(&r.path) else {
            return Ok(());
        };
        if rec.pid == std::process::id() {
            return Ok(());
        }
        if (r.alive)(rec.pid) {
            return Err(
                "Another BOS Settings window is waiting for an answer about a display change. Keep or revert it there first."
                    .into(),
            );
        }
        dr::restore(&*self.comp, &rec).map_err(|e| {
            format!("An earlier display change that was never confirmed could not be undone ({e}), so nothing was changed.")
        })?;
        dr::clear_record(&r.path, rec.id);
        Ok(())
    }

    fn disarm_rescue(&self, id: u64) {
        if let Some(r) = &self.rescue {
            super::display_rescue::clear_record(&r.path, id);
        }
    }

    /// Put `before` back and check the compositor took it.
    fn restore(&self, before: &[LiveMonitor]) -> Result<(), String> {
        let stmts = plan_statements(before)?;
        self.comp.eval(&stmts.join("; "))?;
        self.confirm(before, true)
    }

    fn apply(
        self: &Arc<Self>,
        target: Vec<LiveMonitor>,
        window: Duration,
    ) -> Result<GuardStatus, String> {
        validate_layout(&target)?;
        let _op = lock(&self.op);
        if lock(&self.state).pending.is_some() {
            return Err("Another display change is still waiting to be kept or reverted.".into());
        }
        self.settle_foreign_record()?;
        let before = self.comp.read().map_err(|e| {
            format!("The current display layout could not be read, so nothing was changed. {e}")
        })?;
        for t in &target {
            if !before.iter().any(|b| b.name == t.name) {
                return Err(format!(
                    "{} is no longer connected. Nothing was changed.",
                    t.name
                ));
            }
        }
        let stmts = plan_statements(&target)?;
        let id = {
            let mut st = lock(&self.state);
            st.next_id += 1;
            st.next_id
        };
        // Before anything changes: if this process dies during the countdown,
        // the watchdog still puts `before` back (BSET-67).
        self.arm_rescue(id, &before, window)?;
        let applied = self.comp.eval(&stmts.join("; ")).and_then(|()| {
            self.confirm(&target, false)
                .map_err(|why| format!("Hyprland did not end up with the requested layout: {why}."))
        });
        if let Err(problem) = applied {
            let restored = self.restore(&before);
            self.disarm_rescue(id);
            return Err(match restored {
                Ok(()) => format!("{problem} The previous layout was restored."),
                Err(e) => format!("{problem} Restoring the previous layout also failed: {e}"),
            });
        }
        {
            let mut st = lock(&self.state);
            st.pending = Some(Pending {
                id,
                before,
                after: target,
                deadline: Instant::now() + window,
                total: window,
            });
        }
        let me = Arc::clone(self);
        let spawned = std::thread::Builder::new()
            .name("display-guard".into())
            .spawn(move || me.watch(id));
        if let Err(e) = spawned {
            // No timer means no safety net: undo the change right away.
            let pending = lock(&self.state).pending.take();
            if let Some(p) = pending {
                let _ = self.restore(&p.before);
            }
            self.disarm_rescue(id);
            return Err(format!(
                "The safety timer could not start ({e}). The change was undone."
            ));
        }
        Ok(self.status())
    }

    /// The deadline. Runs in its own thread for as long as a change is
    /// pending, and ends it by reverting on timeout or when a display the
    /// pending layout needs disappears.
    fn watch(self: Arc<Self>, id: u64) {
        loop {
            let (expired, after) = {
                let mut st = lock(&self.state);
                loop {
                    let Some(p) = st.pending.as_ref().filter(|p| p.id == id) else {
                        return;
                    };
                    let now = Instant::now();
                    if now >= p.deadline {
                        break (true, p.after.clone());
                    }
                    let wait = (p.deadline - now).min(self.timing.poll);
                    let after = p.after.clone();
                    let (guard, res) = self
                        .wake
                        .wait_timeout(st, wait)
                        .unwrap_or_else(|e| e.into_inner());
                    st = guard;
                    if res.timed_out() {
                        break (false, after);
                    }
                }
            };
            let (kind, lead) = if expired {
                (
                    OutcomeKind::TimedOut,
                    "No answer in time, so the previous display settings were restored."
                        .to_string(),
                )
            } else {
                match self.comp.read() {
                    Ok(actual) => match disconnect_reason(&after, &actual) {
                        Some(why) => (
                            OutcomeKind::Disconnected,
                            format!("{why} The previous display settings were restored."),
                        ),
                        None => continue,
                    },
                    Err(_) => continue,
                }
            };
            let _op = lock(&self.op);
            let pending = {
                let mut st = lock(&self.state);
                if st.pending.as_ref().map(|p| p.id) == Some(id) {
                    st.pending.take()
                } else {
                    None
                }
            };
            if let Some(p) = pending {
                self.finish_revert(p, kind, lead);
            }
            return;
        }
    }

    fn finish_revert(&self, p: Pending, kind: OutcomeKind, lead: String) -> GuardOutcome {
        let restored = self.restore(&p.before);
        // Whatever happened, the out-of-process watchdog must not redo it.
        self.disarm_rescue(p.id);
        let outcome = match restored {
            Ok(()) => GuardOutcome {
                id: p.id,
                kind,
                message: lead,
            },
            Err(e) => GuardOutcome {
                id: p.id,
                kind: OutcomeKind::RevertFailed,
                message: format!("{lead} Restoring the previous layout failed: {e}"),
            },
        };
        self.record(outcome)
    }

    fn record(&self, outcome: GuardOutcome) -> GuardOutcome {
        lock(&self.state).last = Some(outcome.clone());
        (self.notify)(&outcome);
        outcome
    }

    fn take_pending(&self, id: u64) -> Result<Pending, String> {
        let mut st = lock(&self.state);
        if st.pending.as_ref().map(|p| p.id) == Some(id) {
            return Ok(st.pending.take().expect("checked above"));
        }
        Err(match &st.last {
            Some(o) if o.id == id => format!("Too late: {}", o.message),
            _ => "No display change is waiting for an answer.".into(),
        })
    }

    fn keep(&self, id: u64) -> Result<GuardOutcome, String> {
        let _op = lock(&self.op);
        let p = self.take_pending(id)?;
        self.wake.notify_all();
        // Kept: the watchdog must not revert it, even if saving fails below.
        self.disarm_rescue(id);
        let layout: Vec<LiveMonitor> = p.after.iter().filter(|m| !m.disabled).cloned().collect();
        match self.comp.persist(&layout) {
            Ok(()) => Ok(self.record(GuardOutcome {
                id,
                kind: OutcomeKind::Kept,
                message: "Display settings kept.".into(),
            })),
            Err(e) => {
                let message =
                    format!("The layout is active but could not be saved for next login: {e}");
                self.record(GuardOutcome {
                    id,
                    kind: OutcomeKind::SaveFailed,
                    message: message.clone(),
                });
                Err(message)
            }
        }
    }

    fn revert(&self, id: u64) -> Result<GuardOutcome, String> {
        let _op = lock(&self.op);
        let p = match self.take_pending(id) {
            Ok(p) => p,
            Err(e) => {
                return match &lock(&self.state).last {
                    Some(o) if o.id == id && o.kind != OutcomeKind::Kept => Ok(o.clone()),
                    _ => Err(e),
                }
            }
        };
        self.wake.notify_all();
        let outcome = self.finish_revert(
            p,
            OutcomeKind::Reverted,
            "Previous display settings restored.".into(),
        );
        if outcome.kind == OutcomeKind::RevertFailed {
            Err(outcome.message)
        } else {
            Ok(outcome)
        }
    }

    fn revert_on_exit(&self) {
        let _op = lock(&self.op);
        let pending = lock(&self.state).pending.take();
        self.wake.notify_all();
        if let Some(p) = pending {
            self.finish_revert(
                p,
                OutcomeKind::Reverted,
                "Settings closed before the change was kept, so the previous display settings were restored.".into(),
            );
        }
    }
}

// ---------------------------------------------------------------------
// Tauri commands.
// ---------------------------------------------------------------------

#[tauri::command]
pub async fn apply_display_layout(
    guard: tauri::State<'_, DisplayGuard>,
    monitors: Vec<LiveMonitor>,
    seconds: Option<u64>,
) -> Result<GuardStatus, String> {
    let g = guard.inner().clone();
    tauri::async_runtime::spawn_blocking(move || g.apply(monitors, seconds))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn keep_display_layout(
    guard: tauri::State<'_, DisplayGuard>,
    id: u64,
) -> Result<GuardOutcome, String> {
    let g = guard.inner().clone();
    tauri::async_runtime::spawn_blocking(move || g.keep(id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn revert_display_layout(
    guard: tauri::State<'_, DisplayGuard>,
    id: u64,
) -> Result<GuardOutcome, String> {
    let g = guard.inner().clone();
    tauri::async_runtime::spawn_blocking(move || g.revert(id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn display_guard_status(guard: tauri::State<'_, DisplayGuard>) -> GuardStatus {
    guard.status()
}

/// SIGTERM (logout, `pkill`, systemd stop), SIGINT and SIGHUP skip
/// `RunEvent::Exit`, so a pending change would otherwise outlive the app.
/// Revert first, then let Tauri exit normally. SIGKILL cannot be caught.
pub fn revert_on_signal(app: tauri::AppHandle) {
    use tauri::Manager;
    use tokio::signal::unix::{signal, SignalKind};
    tauri::async_runtime::spawn(async move {
        let (Ok(mut term), Ok(mut int), Ok(mut hup)) = (
            signal(SignalKind::terminate()),
            signal(SignalKind::interrupt()),
            signal(SignalKind::hangup()),
        ) else {
            eprintln!(
                "bos-settings: cannot watch exit signals; a pending display change survives them"
            );
            return;
        };
        tokio::select! {
            _ = term.recv() => {}
            _ = int.recv() => {}
            _ = hup.recv() => {}
        }
        if let Some(guard) = app.try_state::<DisplayGuard>() {
            let g = guard.inner().clone();
            let _ = tauri::async_runtime::spawn_blocking(move || g.revert_on_exit()).await;
        }
        app.exit(0);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("testdata/hyprctl-monitors-all.json");

    fn fixture() -> Vec<LiveMonitor> {
        hyprland::parse_monitors(&serde_json::from_str(FIXTURE).unwrap())
    }

    fn named<'a>(l: &'a mut [LiveMonitor], name: &str) -> &'a mut LiveMonitor {
        l.iter_mut().find(|m| m.name == name).unwrap()
    }

    /// A compositor stand-in. `eval` really runs the generated Lua against a
    /// stub `hl.monitor`, so a syntax error or a missing key fails the test.
    struct Mock {
        sim: Mutex<Vec<LiveMonitor>>,
        log: Mutex<Vec<String>>,
        persisted: Mutex<Vec<Vec<LiveMonitor>>>,
        fail_eval: Mutex<Option<String>>,
        ignore_eval: Mutex<bool>,
        fail_persist: Mutex<bool>,
    }

    impl Mock {
        fn new(start: Vec<LiveMonitor>) -> Arc<Self> {
            Arc::new(Self {
                sim: Mutex::new(start),
                log: Mutex::new(Vec::new()),
                persisted: Mutex::new(Vec::new()),
                fail_eval: Mutex::new(None),
                ignore_eval: Mutex::new(false),
                fail_persist: Mutex::new(false),
            })
        }
        fn state(&self) -> Vec<LiveMonitor> {
            lock(&self.sim).clone()
        }
        fn unplug(&self, name: &str) {
            lock(&self.sim).retain(|m| m.name != name);
        }
    }

    struct Call {
        output: String,
        mode: Option<String>,
        position: Option<String>,
        scale: Option<String>,
        transform: Option<u32>,
        vrr: Option<u32>,
        mirror: Option<String>,
        disabled: bool,
    }

    impl Compositor for Mock {
        fn read(&self) -> Result<Vec<LiveMonitor>, String> {
            Ok(self.state())
        }
        fn eval(&self, src: &str) -> Result<(), String> {
            lock(&self.log).push(src.to_string());
            if let Some(e) = lock(&self.fail_eval).clone() {
                return Err(e);
            }
            let calls: Arc<Mutex<Vec<Call>>> = Arc::default();
            let lua = mlua::Lua::new();
            let sink = Arc::clone(&calls);
            let f = lua
                .create_function(move |_, t: mlua::Table| {
                    lock(&sink).push(Call {
                        output: t.get("output")?,
                        mode: t.get("mode")?,
                        position: t.get("position")?,
                        scale: t.get("scale")?,
                        transform: t.get("transform")?,
                        vrr: t.get("vrr")?,
                        mirror: t.get("mirror")?,
                        disabled: t.get::<_, Option<bool>>("disabled")?.unwrap_or(false),
                    });
                    Ok(())
                })
                .unwrap();
            let hl = lua.create_table().unwrap();
            hl.set("monitor", f).unwrap();
            lua.globals().set("hl", hl).unwrap();
            lua.load(src).exec().map_err(|e| format!("lua: {e}"))?;
            if *lock(&self.ignore_eval) {
                return Ok(());
            }
            let mut sim = lock(&self.sim);
            for c in lock(&calls).iter() {
                let Some(m) = sim.iter_mut().find(|m| m.name == c.output) else {
                    continue;
                };
                m.disabled = c.disabled;
                if c.disabled {
                    continue;
                }
                if let Some((res, hz)) = c.mode.as_deref().and_then(|s| s.split_once('@')) {
                    let (w, h) = res.split_once('x').unwrap();
                    m.width = w.parse().unwrap();
                    m.height = h.parse().unwrap();
                    // The compositor snaps to the nearest real mode.
                    let hz: f64 = hz.parse().unwrap();
                    if (m.refresh - hz).abs() > 0.5 {
                        m.refresh = hz;
                    }
                }
                if let Some((x, y)) = c.position.as_deref().and_then(|s| s.split_once('x')) {
                    m.x = x.parse().unwrap();
                    m.y = y.parse().unwrap();
                }
                if let Some(s) = &c.scale {
                    m.scale = s.parse().unwrap();
                }
                m.transform = c.transform.unwrap_or(0);
                m.vrr = c.vrr.unwrap_or(0);
                m.mirror_of = c.mirror.clone().unwrap_or_default();
            }
            Ok(())
        }
        fn persist(&self, layout: &[LiveMonitor]) -> Result<(), String> {
            if *lock(&self.fail_persist) {
                return Err("disk is read-only".into());
            }
            lock(&self.persisted).push(layout.to_vec());
            Ok(())
        }
    }

    fn fast() -> Timing {
        Timing {
            window: Duration::from_millis(80),
            poll: Duration::from_millis(5),
            settle: Duration::ZERO,
        }
    }

    type Events = Arc<Mutex<Vec<GuardOutcome>>>;

    fn guard(mock: &Arc<Mock>) -> (DisplayGuard, Events) {
        let events: Events = Arc::default();
        let sink = Arc::clone(&events);
        let g = DisplayGuard::new(mock.clone(), fast(), move |o| lock(&sink).push(o.clone()));
        (g, events)
    }

    fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
        let end = Instant::now() + Duration::from_secs(3);
        while !cond() {
            assert!(Instant::now() < end, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// The user's layout with DVI-I-2 rescaled and nothing else touched.
    fn rescaled() -> Vec<LiveMonitor> {
        let mut t = fixture();
        named(&mut t, "DVI-I-2").scale = 1.25;
        t
    }

    // --- BSET-49: mirror parsing from the real capture ---

    #[test]
    fn fixture_has_dvi1_mirroring_edp_and_dvi2_separate_and_rotated() {
        let l = fixture();
        assert_eq!(l.len(), 3);
        let by = |n: &str| l.iter().find(|m| m.name == n).unwrap();
        assert_eq!(by("DVI-I-1").mirror_of, "eDP-1", "mirrorOf 0 is eDP-1's id");
        assert_eq!(by("eDP-1").mirror_of, "");
        assert_eq!(by("DVI-I-2").mirror_of, "");
        assert_eq!(by("DVI-I-2").transform, 1);
        assert!(l.iter().all(|m| !m.disabled));
    }

    #[test]
    fn mirror_of_accepts_ids_names_and_none_markers() {
        let make = |mirror: serde_json::Value| {
            serde_json::json!([
                {"id": 0, "name": "A", "width": 1, "height": 1, "mirrorOf": "none"},
                {"id": 7, "name": "B", "width": 1, "height": 1, "mirrorOf": mirror}
            ])
        };
        let of = |v: serde_json::Value| hyprland::parse_monitors(&make(v))[1].mirror_of.clone();
        assert_eq!(of(serde_json::json!("0")), "A");
        assert_eq!(of(serde_json::json!(0)), "A");
        assert_eq!(of(serde_json::json!("A")), "A");
        assert_eq!(of(serde_json::json!("none")), "");
        assert_eq!(of(serde_json::json!("-1")), "");
        assert_eq!(of(serde_json::json!("")), "");
        assert_eq!(of(serde_json::Value::Null), "");
    }

    #[test]
    fn disabled_outputs_are_parsed() {
        let v = serde_json::json!([
            {"id": 0, "name": "A", "width": 1, "height": 1, "disabled": false},
            {"id": 1, "name": "B", "width": 1, "height": 1, "disabled": true}
        ]);
        let l = hyprland::parse_monitors(&v);
        assert!(!l[0].disabled && l[1].disabled);
    }

    // --- Planning ---

    #[test]
    fn statements_keep_the_mirror_and_put_sources_first() {
        let stmts = plan_statements(&fixture()).unwrap();
        assert_eq!(stmts.len(), 3);
        let mirror_at = stmts
            .iter()
            .position(|s| s.contains("mirror = \"eDP-1\""))
            .unwrap();
        assert!(stmts[mirror_at].contains("output = \"DVI-I-1\""));
        assert_eq!(mirror_at, 2, "the mirror comes after its source");
        let dvi2 = stmts.iter().find(|s| s.contains("DVI-I-2")).unwrap();
        assert!(dvi2.contains("transform = 1") && !dvi2.contains("mirror"));
    }

    #[test]
    fn statements_keep_fractional_refresh_rates() {
        let mut l = fixture();
        let m = named(&mut l, "DVI-I-2");
        m.refresh = 59.94;
        assert!(hyprland::lua_statement(m)
            .unwrap()
            .contains("1920x1080@59.94"));
        m.refresh = 60.0;
        assert!(hyprland::lua_statement(m)
            .unwrap()
            .contains("1920x1080@60\""));
    }

    #[test]
    fn layout_without_an_enabled_display_is_refused() {
        let mut l = fixture();
        for m in &mut l {
            m.disabled = true;
        }
        let e = validate_layout(&l).unwrap_err();
        assert!(e.contains("At least one display"), "{e}");
        assert!(validate_layout(&[]).is_err());
    }

    #[test]
    fn bad_mirror_layouts_are_refused() {
        let mut l = fixture();
        named(&mut l, "DVI-I-1").mirror_of = "DVI-I-1".into();
        assert!(validate_layout(&l).unwrap_err().contains("itself"));

        let mut l = fixture();
        named(&mut l, "DVI-I-1").mirror_of = "HDMI-A-9".into();
        assert!(validate_layout(&l).unwrap_err().contains("not connected"));

        let mut l = fixture();
        named(&mut l, "DVI-I-2").mirror_of = "DVI-I-1".into();
        assert!(validate_layout(&l)
            .unwrap_err()
            .contains("mirroring another"));

        let mut l = fixture();
        named(&mut l, "eDP-1").disabled = true;
        assert!(validate_layout(&l).unwrap_err().contains("turned off"));

        let mut l = fixture();
        named(&mut l, "DVI-I-2").scale = 0.0;
        assert!(validate_layout(&l).is_err());
        assert!(validate_layout(&fixture()).is_ok());
    }

    // --- The guard against the mock ---

    #[test]
    fn unrelated_change_keeps_the_mirror_and_snapshot_includes_it() {
        let mock = Mock::new(fixture());
        let (g, _) = guard(&mock);
        let status = g.apply_for(rescaled(), Duration::from_secs(30)).unwrap();
        assert!(status.pending.is_some());
        let now = mock.state();
        assert_eq!(
            now.iter().find(|m| m.name == "DVI-I-2").unwrap().scale,
            1.25
        );
        assert_eq!(
            now.iter().find(|m| m.name == "DVI-I-1").unwrap().mirror_of,
            "eDP-1",
            "mirroring survives a change to an unrelated output"
        );
        let id = status.pending.unwrap().id;
        g.revert(id).unwrap();
        assert_eq!(mock.state(), fixture());
        let log = lock(&mock.log);
        assert_eq!(log.len(), 2);
        assert!(
            log[1].contains("mirror = \"eDP-1\""),
            "restore replays the mirror: {}",
            log[1]
        );
    }

    #[test]
    fn unattended_timeout_reverts_from_the_backend() {
        let mock = Mock::new(fixture());
        let (g, events) = guard(&mock);
        g.apply_for(rescaled(), Duration::from_millis(60)).unwrap();
        assert_ne!(mock.state(), fixture());
        // Nobody calls keep or revert, as if the window had been closed.
        wait_until("timeout revert", || g.status().last.is_some());
        assert_eq!(mock.state(), fixture());
        let last = g.status().last.unwrap();
        assert_eq!(last.kind, OutcomeKind::TimedOut);
        assert!(
            lock(&mock.persisted).is_empty(),
            "nothing persisted without Keep"
        );
        assert_eq!(lock(&events).len(), 1);
    }

    #[test]
    fn keep_persists_and_the_deadline_then_does_nothing() {
        let mock = Mock::new(fixture());
        let (g, _) = guard(&mock);
        let id = g
            .apply_for(rescaled(), Duration::from_millis(60))
            .unwrap()
            .pending
            .unwrap()
            .id;
        assert!(
            lock(&mock.persisted).is_empty(),
            "nothing is saved before Keep"
        );
        let out = g.keep(id).unwrap();
        assert_eq!(out.kind, OutcomeKind::Kept);
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(mock.state(), rescaled(), "kept layout stays live");
        let saved = lock(&mock.persisted);
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].len(), 3);
        assert!(saved[0]
            .iter()
            .any(|m| m.name == "DVI-I-1" && m.mirror_of == "eDP-1"));
        drop(saved);
        assert!(g.keep(id).unwrap_err().contains("Too late"));
    }

    #[test]
    fn explicit_revert_is_immediate_and_idempotent_after_a_timeout() {
        let mock = Mock::new(fixture());
        let (g, _) = guard(&mock);
        let id = g
            .apply_for(rescaled(), Duration::from_secs(30))
            .unwrap()
            .pending
            .unwrap()
            .id;
        let out = g.revert(id).unwrap();
        assert_eq!(out.kind, OutcomeKind::Reverted);
        assert_eq!(mock.state(), fixture());
        assert!(g.status().pending.is_none());
        assert_eq!(g.revert(id).unwrap().kind, OutcomeKind::Reverted);
        assert!(g.revert(id + 99).is_err());
    }

    #[test]
    fn revert_after_timeout_reports_the_timeout() {
        let mock = Mock::new(fixture());
        let (g, _) = guard(&mock);
        let id = g
            .apply_for(rescaled(), Duration::from_millis(30))
            .unwrap()
            .pending
            .unwrap()
            .id;
        wait_until("timeout", || g.status().last.is_some());
        // The webview's own countdown ends later and asks for a revert.
        assert_eq!(g.revert(id).unwrap().kind, OutcomeKind::TimedOut);
    }

    #[test]
    fn failed_apply_restores_and_leaves_nothing_pending() {
        let mock = Mock::new(fixture());
        let (g, _) = guard(&mock);
        *lock(&mock.fail_eval) = Some("hyprctl: invalid mode".into());
        let e = g
            .apply_for(rescaled(), Duration::from_secs(30))
            .unwrap_err();
        assert!(e.contains("invalid mode"), "{e}");
        assert!(g.status().pending.is_none());
        assert_eq!(mock.state(), fixture());
    }

    #[test]
    fn compositor_that_ignores_the_change_is_caught_by_the_readback() {
        let mock = Mock::new(fixture());
        let (g, _) = guard(&mock);
        *lock(&mock.ignore_eval) = true;
        let e = g
            .apply_for(rescaled(), Duration::from_secs(30))
            .unwrap_err();
        assert!(e.contains("did not end up"), "{e}");
        assert!(e.contains("scale"), "{e}");
        assert!(g.status().pending.is_none());
        assert!(lock(&mock.persisted).is_empty());
    }

    #[test]
    fn layout_with_no_enabled_display_never_reaches_the_compositor() {
        let mock = Mock::new(fixture());
        let (g, _) = guard(&mock);
        let mut t = fixture();
        for m in &mut t {
            m.disabled = true;
        }
        assert!(g.apply_for(t, Duration::from_secs(30)).is_err());
        assert!(lock(&mock.log).is_empty());
        assert_eq!(mock.state(), fixture());
    }

    #[test]
    fn change_naming_an_unplugged_output_is_refused_untouched() {
        let mock = Mock::new(fixture());
        mock.unplug("DVI-I-2");
        let (g, _) = guard(&mock);
        let e = g
            .apply_for(rescaled(), Duration::from_secs(30))
            .unwrap_err();
        assert!(
            e.contains("DVI-I-2") && e.contains("no longer connected"),
            "{e}"
        );
        assert!(lock(&mock.log).is_empty());
    }

    #[test]
    fn unplugging_a_display_while_pending_reverts_early() {
        let mock = Mock::new(fixture());
        let (g, events) = guard(&mock);
        g.apply_for(rescaled(), Duration::from_secs(30)).unwrap();
        mock.unplug("DVI-I-2");
        wait_until("disconnect revert", || g.status().last.is_some());
        let last = g.status().last.unwrap();
        assert_eq!(last.kind, OutcomeKind::Disconnected);
        assert!(last.message.contains("DVI-I-2"));
        assert_eq!(lock(&events).len(), 1);
        let eventual = mock.state();
        assert_eq!(eventual.len(), 2);
        assert_eq!(
            eventual
                .iter()
                .find(|m| m.name == "DVI-I-1")
                .unwrap()
                .mirror_of,
            "eDP-1"
        );
    }

    #[test]
    fn second_change_is_refused_while_one_is_pending() {
        let mock = Mock::new(fixture());
        let (g, _) = guard(&mock);
        g.apply_for(rescaled(), Duration::from_secs(30)).unwrap();
        let e = g.apply_for(fixture(), Duration::from_secs(30)).unwrap_err();
        assert!(e.contains("still waiting"), "{e}");
    }

    #[test]
    fn exiting_the_app_reverts_an_unconfirmed_change() {
        let mock = Mock::new(fixture());
        let (g, _) = guard(&mock);
        g.apply_for(rescaled(), Duration::from_secs(30)).unwrap();
        g.revert_on_exit();
        assert_eq!(mock.state(), fixture());
        assert!(g.status().pending.is_none());
        g.revert_on_exit();
    }

    #[test]
    fn save_failure_after_keep_is_reported() {
        let mock = Mock::new(fixture());
        let (g, _) = guard(&mock);
        *lock(&mock.fail_persist) = true;
        let id = g
            .apply_for(rescaled(), Duration::from_secs(30))
            .unwrap()
            .pending
            .unwrap()
            .id;
        let e = g.keep(id).unwrap_err();
        assert!(e.contains("could not be saved"), "{e}");
        assert_eq!(g.status().last.unwrap().kind, OutcomeKind::SaveFailed);
    }

    #[test]
    fn stopping_a_mirror_and_mirroring_again_round_trips() {
        let mock = Mock::new(fixture());
        let (g, _) = guard(&mock);
        let mut t = fixture();
        let m = named(&mut t, "DVI-I-1");
        m.mirror_of.clear();
        m.x = 1920;
        let id = g
            .apply_for(t, Duration::from_secs(30))
            .unwrap()
            .pending
            .unwrap()
            .id;
        g.keep(id).unwrap();
        let now = mock.state();
        let d1 = now.iter().find(|m| m.name == "DVI-I-1").unwrap();
        assert_eq!((d1.mirror_of.as_str(), d1.x), ("", 1920));
        let mut back = now.clone();
        named(&mut back, "DVI-I-1").mirror_of = "eDP-1".into();
        let id = g
            .apply_for(back, Duration::from_secs(30))
            .unwrap()
            .pending
            .unwrap()
            .id;
        g.keep(id).unwrap();
        let d1 = mock
            .state()
            .into_iter()
            .find(|m| m.name == "DVI-I-1")
            .unwrap();
        assert_eq!(d1.mirror_of, "eDP-1");
    }

    fn rescued(mock: &Arc<Mock>, tag: &str) -> (DisplayGuard, std::path::PathBuf) {
        static SPAWNED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        fn spawn(id: u64) -> Result<(), String> {
            SPAWNED.store(id, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        let dir = std::env::temp_dir().join(format!("bset67-g-{tag}-{}", std::process::id()));
        let path = dir.join("display-pending.json");
        let (mut g, _) = guard(mock);
        g.set_rescue(Rescue {
            path: path.clone(),
            spawn,
            alive: |_| false,
        });
        (g, path)
    }

    #[test]
    fn rescue_record_lives_exactly_while_a_change_is_pending() {
        use super::super::display_rescue::read_record;
        for end in ["keep", "revert", "timeout", "exit"] {
            let mock = Mock::new(fixture());
            let (g, path) = rescued(&mock, end);
            let window = if end == "timeout" {
                Duration::from_millis(60)
            } else {
                Duration::from_secs(30)
            };
            let id = g.apply_for(rescaled(), window).unwrap().pending.unwrap().id;
            let rec = read_record(&path).expect("record written before the change");
            assert_eq!(rec.id, id);
            assert_eq!(rec.pid, std::process::id());
            assert_eq!(
                rec.before,
                fixture(),
                "the record holds the layout to restore"
            );
            match end {
                "keep" => {
                    g.keep(id).unwrap();
                }
                "revert" => {
                    g.revert(id).unwrap();
                }
                "exit" => g.revert_on_exit(),
                _ => wait_until("timeout revert", || g.status().last.is_some()),
            }
            assert!(read_record(&path).is_none(), "{end} clears the record");
            let _ = std::fs::remove_dir_all(path.parent().unwrap());
        }
    }

    #[test]
    fn a_failed_apply_leaves_no_rescue_record() {
        let mock = Mock::new(fixture());
        *lock(&mock.fail_eval) = Some("refused".into());
        let (g, path) = rescued(&mock, "fail");
        assert!(g.apply_for(rescaled(), Duration::from_secs(30)).is_err());
        assert!(super::super::display_rescue::read_record(&path).is_none());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn watchdog_restores_a_change_whose_app_died() {
        use super::super::display_rescue::{check_once, read_record, write_record};
        let mock = Mock::new(fixture());
        let (g, path) = rescued(&mock, "killed");
        let id = g
            .apply_for(rescaled(), Duration::from_secs(30))
            .unwrap()
            .pending
            .unwrap()
            .id;
        assert_ne!(mock.state(), fixture(), "the change is live");
        // Simulate SIGKILL: the guard never runs again, and the deadline passes.
        let mut rec = read_record(&path).unwrap();
        rec.deadline_ms = 0;
        write_record(&path, &rec).unwrap();
        std::mem::forget(g);
        assert_eq!(check_once(&*mock, &path, Some(id), |_| false), None);
        assert_eq!(
            mock.state(),
            fixture(),
            "the watchdog put the old layout back"
        );
        assert!(read_record(&path).is_none());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// BSET-67 review: run A applies and is killed; the app is restarted
    /// inside A's window and a new change is made. A's layout must not be
    /// lost, the new change must not collide with A's id, and A's watchdog
    /// must not act on the new record.
    #[test]
    fn restart_inside_a_dead_runs_window_keeps_both_safe() {
        use super::super::display_rescue::{check_once, read_record, write_record, Record};
        let mock = Mock::new(fixture());
        let (g, path) = rescued(&mock, "restart");
        // Run A left this: the good layout, a deadline still in the future,
        // a pid that is gone, and an id a fresh counter could also produce.
        let mut changed = fixture();
        named(&mut changed, "DVI-I-2").transform = 0;
        *lock(&mock.sim) = changed.clone();
        write_record(
            &path,
            &Record {
                id: 1,
                pid: 4_000_000,
                deadline_ms: super::super::display_rescue::now_ms() + 60_000,
                before: fixture(),
            },
        )
        .unwrap();
        let id = g
            .apply_for(rescaled(), Duration::from_secs(30))
            .unwrap()
            .pending
            .unwrap()
            .id;
        assert_ne!(id, 1, "ids are unique across runs");
        let rec = read_record(&path).unwrap();
        assert_eq!(rec.id, id);
        assert_eq!(
            rec.before,
            fixture(),
            "run A's layout was put back first, so it is what this change restores"
        );
        // Run A's watchdog wakes up: the record is not its own, so it leaves it.
        assert_eq!(check_once(&*mock, &path, Some(1), |_| false), None);
        assert_eq!(read_record(&path).map(|r| r.id), Some(id));
        g.revert(id).unwrap();
        assert_eq!(mock.state(), fixture(), "revert lands on the good layout");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn a_live_other_window_blocks_a_new_change() {
        use super::super::display_rescue::{write_record, Record};
        let mock = Mock::new(fixture());
        let dir = std::env::temp_dir().join(format!("bset67-g-live-{}", std::process::id()));
        let path = dir.join("display-pending.json");
        let (mut g, _) = guard(&mock);
        g.set_rescue(Rescue {
            path: path.clone(),
            spawn: |_| Ok(()),
            alive: |_| true,
        });
        write_record(
            &path,
            &Record {
                id: 9,
                pid: 4_000_000,
                deadline_ms: super::super::display_rescue::now_ms() + 60_000,
                before: fixture(),
            },
        )
        .unwrap();
        let err = g
            .apply_for(rescaled(), Duration::from_secs(30))
            .unwrap_err();
        assert!(err.contains("Another BOS Settings window"), "{err}");
        assert_eq!(mock.state(), fixture(), "nothing changed");
        assert!(lock(&mock.log).is_empty(), "the compositor was not touched");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
