//! Watchers locales para consumidores persistentes (GUI, CLI watch, extensiones IDE).
//!
//! Los eventos no hacen HTTP por sí mismos: solo señalan que una fuente local
//! cambió. Cada consumidor decide si puede reconstruir un provider desde el
//! archivo o si debe esperar al polling remoto de respaldo.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::paths::home_dir;

/// Frecuencia conservadora de endpoints remotos mientras un consumidor está
/// vivo. Los cambios de sesión no adelantan esta llamada.
pub const DEFAULT_REMOTE_POLL: Duration = Duration::from_secs(60);
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchTick {
    /// Un `.jsonl` de Codex se estabilizó tras el debounce.
    CodexSessionChanged,
    /// Respaldo periódico: es el único tick que debe consultar APIs remotas.
    RemotePoll,
}

pub fn codex_sessions_dir() -> PathBuf {
    if let Ok(home) = std::env::var("CODEX_HOME") {
        if !home.trim().is_empty() {
            return PathBuf::from(home).join("sessions");
        }
    }
    home_dir().join(".codex").join("sessions")
}

/// Mantiene un watcher recursivo sobre sesiones Codex. Devuelve solo al
/// producirse un error del backend de filesystem; Ctrl+C termina el proceso
/// llamador como es habitual en CLI.
pub fn run_codex_session_watch<F>(
    root: &Path,
    debounce: Duration,
    remote_poll: Duration,
    mut on_tick: F,
) -> Result<(), String>
where
    F: FnMut(WatchTick),
{
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = RecommendedWatcher::new(tx, Config::default()).map_err(|e| e.to_string())?;
    watcher
        .watch(root, RecursiveMode::Recursive)
        .map_err(|e| format!("no se puede observar {}: {e}", root.display()))?;

    let mut pending_local = None::<Instant>;
    let mut next_remote = Instant::now() + remote_poll;
    loop {
        let now = Instant::now();
        let next = pending_local
            .map(|d| d.min(next_remote))
            .unwrap_or(next_remote);
        match rx.recv_timeout(next.saturating_duration_since(now)) {
            Ok(Ok(event)) if touches_session_jsonl(&event) => {
                pending_local = Some(Instant::now() + debounce)
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => eprintln!("iausage watch: evento de filesystem ignorado: {error}"),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("watcher de filesystem desconectado".into())
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        let now = Instant::now();
        if pending_local.is_some_and(|due| due <= now) {
            pending_local = None;
            on_tick(WatchTick::CodexSessionChanged);
        }
        if now >= next_remote {
            next_remote = now + remote_poll;
            on_tick(WatchTick::RemotePoll);
        }
    }
}

fn touches_session_jsonl(event: &Event) -> bool {
    event.paths.iter().any(|path| {
        path.extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::{event::ModifyKind, EventKind};

    #[test]
    fn only_jsonl_changes_trigger_the_fast_path() {
        let event = Event {
            kind: EventKind::Modify(ModifyKind::Any),
            paths: vec![PathBuf::from("session.jsonl")],
            attrs: Default::default(),
        };
        assert!(touches_session_jsonl(&event));
        let other = Event {
            kind: EventKind::Modify(ModifyKind::Any),
            paths: vec![PathBuf::from("auth.json")],
            attrs: Default::default(),
        };
        assert!(!touches_session_jsonl(&other));
    }
}
