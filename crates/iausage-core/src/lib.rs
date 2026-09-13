//! IA Usage Core: provider pipeline compartido por la GUI y el CLI.
//!
//! Frontera arquitectónica v0.3: todo lo que no necesita Tauri vive aquí.
//! `src-tauri` es un thin wrapper (ventana/tray/notificaciones) y
//! `iausage-cli` consume este mismo pipeline. Ningún módulo de este crate
//! puede depender de Tauri.

pub mod cache;
pub mod config;
pub mod cost;
pub mod descriptor;
pub mod doctor;
pub mod guard;
pub mod health;
pub mod http;
pub mod jwt;
pub mod logfile;
pub mod model;
pub mod pace;
pub mod pairing;
pub mod paths;
pub mod pricing;
pub mod providers;
pub mod recommend;
pub mod refresh_policy;
pub mod snapshot_v1;
pub mod sync;
pub mod sync_server;
pub mod watch;

/// Versión del contrato `DashboardSnapshotV1` que emite `--json` y la GUI.
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;
/// Versión del crate (para `iausage version`).
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
