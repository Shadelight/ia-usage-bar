//! `iausage`: CLI de IA Usage Bar sobre el mismo pipeline que la GUI.
//!
//! ```powershell
//! iausage usage --json
//! iausage best
//! iausage guard --provider claude --min-remaining 15
//! if ($LASTEXITCODE -ne 0) { # mandar el trabajo a Codex }
//! ```
//!
//! Códigos de `guard`: 0 suficiente, 1 bajo el límite, 64 args incorrectos,
//! 69 provider no disponible. El resto de comandos usa 0/64.

use std::collections::HashMap;
use std::process::ExitCode;

use iausage_core::config::AppConfig;
use iausage_core::descriptor::{descriptor, FetchStrategyKind};
use iausage_core::doctor;
use iausage_core::guard;
use iausage_core::model::{most_headroom, now_iso, ProviderSnapshot, VendorId};
use iausage_core::{providers, snapshot_v1, sync, watch};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn usage_help() -> String {
    format!(
        "iausage {VERSION} — control de cuota de IAs desde el terminal\n\
        \n\
        Uso:\n  \
          iausage usage [provider] [--json] [--refresh] [--window W]\n  \
          iausage providers [--json]\n  \
          iausage best [--json]\n  \
          iausage doctor [provider] [--json]\n  \
          iausage refresh [--provider ID]\n  \
          iausage watch --jsonl [--poll-seconds N]\n  \
          iausage guard --provider ID [--window W] [--min-remaining N]\n  \
          iausage enable <provider> | iausage disable <provider>\n  \
            iausage config validate\n  \
            iausage sync <export|verify|status|set-passphrase|enable|disable|serve|qr>\n  \
            iausage version\n\
        \n\
        --json emite DashboardSnapshotV1 (schemaVersion 1), el mismo contrato\n\
        que consumen la GUI, la futura API y los widgets.\n\
        Ventanas de guard/usage: session, 5h, daily, weekly, monthly, credits."
    )
}

fn fail(message: &str) -> ExitCode {
    eprintln!("iausage: {message}");
    eprintln!("Prueba `iausage --help`. Códigos guard: 0 ok / 1 bajo límite / 64 args / 69 no disponible.");
    ExitCode::from(guard::EXIT_USAGE as u8)
}

fn load_state(refresh: bool, only: Option<&str>) -> (AppConfig, HashMap<String, ProviderSnapshot>) {
    let cfg = AppConfig::load();
    let mut snaps = iausage_core::cache::load();
    if refresh {
        let ids: Vec<VendorId> = match only {
            Some(slug) => snapshot_v1::parse_id(slug).into_iter().collect(),
            None => cfg.enabled_ids(),
        };
        for id in ids {
            let snap = providers::refresh(id, &cfg);
            if snap.is_connected() {
                let _ = iausage_core::cache::save_valid(&snap);
            }
            snaps.insert(snap.id.clone(), snap);
        }
    }
    (cfg, snaps)
}

fn snapshot_v1_json(cfg: &AppConfig, snaps: &HashMap<String, ProviderSnapshot>) -> String {
    let catalog = providers::catalog(cfg);
    let v1 = snapshot_v1::build(snaps, &catalog, now_iso(), Some(VERSION.into()));
    serde_json::to_string_pretty(&v1).unwrap_or_else(|_| "{}".into())
}

fn fmt_pct(v: f64) -> String {
    format!("{:.0}%", v.clamp(0.0, 100.0))
}

fn print_usage_human(snaps: &HashMap<String, ProviderSnapshot>, only: Option<&str>) {
    let mut ids: Vec<&String> = snaps.keys().collect();
    ids.sort();
    for id in ids {
        if only.is_some_and(|o| o != id) {
            continue;
        }
        let s = &snaps[id];
        let state = if s.is_connected() {
            "conectado"
        } else {
            "sin conexión"
        };
        println!("{} — {} ({}){}", s.name, s.plan, state, if s.stale { " [antiguo]" } else { "" });
        for q in &s.quotas {
            let used = q.used_percent.map(fmt_pct).unwrap_or_else(|| "—".into());
            let reset = q.reset_at.as_deref().unwrap_or("sin reset");
            println!("  {}: {} usado · reinicia {}", q.label, used, reset);
        }
        if let Some(c) = &s.credits {
            println!("  Créditos: {:.0}", c.remaining);
        }
        if let Some(cost) = &s.cost {
            let conf = match cost.confidence {
                iausage_core::model::DataConfidence::Estimated => " (estimado)",
                iausage_core::model::DataConfidence::Exact => "",
                _ => "",
            };
            if let Some(m) = cost.month {
                println!("  Coste mes: ${m:.2}{conf}");
            }
        }
    }
}

fn cmd_usage(args: &[String]) -> ExitCode {
    let mut only: Option<String> = None;
    let mut json = false;
    let mut refresh = false;
    let mut window: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json = true,
            "--refresh" => refresh = true,
            "--window" => {
                i += 1;
                window = args.get(i).cloned();
                if window.is_none() {
                    return fail("usage: falta valor para --window");
                }
            }
            s if s.starts_with('-') => return fail(&format!("usage: flag desconocido {s}")),
            s => {
                if only.is_some() {
                    return fail("usage: un solo provider");
                }
                if snapshot_v1::parse_id(s).is_none() {
                    return fail(&format!("usage: provider desconocido {s}"));
                }
                only = Some(s.into());
            }
        }
        i += 1;
    }
    let (cfg, snaps) = load_state(refresh, only.as_deref());
    if json {
        println!("{}", snapshot_v1_json(&cfg, &snaps));
        return ExitCode::SUCCESS;
    }
    if let Some(o) = &only {
        if !snaps.contains_key(o) {
            eprintln!("iausage: sin datos para {o} (¿provider desactivado?)");
            return ExitCode::from(guard::EXIT_UNAVAILABLE as u8);
        }
    }
    print_usage_human(&snaps, only.as_deref());
    let _ = window;
    ExitCode::SUCCESS
}

fn cmd_providers(args: &[String]) -> ExitCode {
    let json = args.iter().any(|a| a == "--json");
    if args.iter().any(|a| a != "--json") {
        return fail("providers: solo admite --json");
    }
    let (cfg, snaps) = load_state(false, None);
    if json {
        println!("{}", snapshot_v1_json(&cfg, &snaps));
        return ExitCode::SUCCESS;
    }
    let mut catalog = providers::catalog(&cfg);
    catalog.sort_by(|a, b| a.id.cmp(&b.id));
    for v in &catalog {
        let d = descriptor(snapshot_v1::parse_id(&v.id).unwrap_or(VendorId::Anthropic));
        let strategies: Vec<&str> =
            d.strategies.iter().map(|s| s.label()).collect();
        let pref: String = cfg
            .source_preference(snapshot_v1::parse_id(&v.id).unwrap_or(VendorId::Anthropic))
            .map(|s: FetchStrategyKind| format!(" [fuente: {}]", s.label()))
            .unwrap_or_default();
        println!(
            "{:<14} {:<18} {:<9} {} ({}){pref}",
            v.id,
            v.name,
            if v.enabled { "activo" } else { "off" },
            if v.detected { "detectado" } else { "—" },
            strategies.join("→"),
        );
    }
    ExitCode::SUCCESS
}

fn cmd_best(args: &[String]) -> ExitCode {
    let json = args.iter().any(|a| a == "--json");
    if args.iter().any(|a| a != "--json") {
        return fail("best: solo admite --json");
    }
    let (cfg, snaps) = load_state(false, None);
    let list: Vec<ProviderSnapshot> = snaps.values().cloned().collect();
    match most_headroom(&list) {
        None => {
            if json {
                println!("{{\"best\": null}}");
            } else {
                println!("Sin providers conectados.");
            }
            ExitCode::from(guard::EXIT_UNAVAILABLE as u8)
        }
        Some((id, name, left)) => {
            if json {
                let v = serde_json::json!({"best": {"id": id, "name": name, "remainingPercent": left}});
                println!("{}", serde_json::to_string_pretty(&v).unwrap());
            } else {
                println!("{name} ({id}): {:.0}% disponible", left);
            }
            let _ = cfg;
            ExitCode::SUCCESS
        }
    }
}

fn cmd_doctor(args: &[String]) -> ExitCode {
    let mut only: Option<String> = None;
    let mut json = false;
    for a in args {
        match a.as_str() {
            "--json" => json = true,
            s if s.starts_with('-') => return fail(&format!("doctor: flag desconocido {s}")),
            s => {
                if doctor::diagnose_one(s, &AppConfig::load(), &HashMap::new()).is_none()
                    && snapshot_v1::parse_id(s).is_none()
                {
                    return fail(&format!("doctor: provider desconocido {s}"));
                }
                only = Some(s.into());
            }
        }
    }
    let (cfg, snaps) = load_state(false, None);
    let diags = match &only {
        Some(s) => doctor::diagnose_one(s, &cfg, &snaps).into_iter().collect(),
        None => doctor::diagnose_all(&cfg, &snaps),
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&diags).unwrap());
        return ExitCode::SUCCESS;
    }
    let mut ok_all = true;
    for d in &diags {
        if !d.enabled {
            continue;
        }
        let mark = if d.ok { "✓" } else { "✗" };
        if !d.ok {
            ok_all = false;
        }
        println!("{mark} {} ({})", d.name, d.id);
        for c in &d.checks {
            if !c.ok {
                println!("    [{}] {}", c.code, c.detail);
            }
        }
    }
    if ok_all { ExitCode::SUCCESS } else { ExitCode::from(1) }
}

fn cmd_refresh(args: &[String]) -> ExitCode {
    let mut only: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--provider" => {
                i += 1;
                only = args.get(i).cloned();
                if only.is_none() {
                    return fail("refresh: falta valor para --provider");
                }
            }
            s => return fail(&format!("refresh: argumento desconocido {s}")),
        }
        i += 1;
    }
    if let Some(o) = &only {
        if snapshot_v1::parse_id(o).is_none() {
            return fail(&format!("refresh: provider desconocido {o}"));
        }
    }
    let (cfg, snaps) = load_state(true, only.as_deref());
    println!("Actualizados {} providers.", snaps.len());
    match sync::export_current_snapshot(&cfg, &snaps, Some(VERSION.into())) {
        Ok(Some(path)) => println!("Sync exportado a {}.", path.display()),
        Ok(None) => {}
        Err(e) => eprintln!("aviso sync: {e}"),
    }
    ExitCode::SUCCESS
}

/// Stream persistente para clientes visuales. Cada línea es JSON independiente
/// y contiene el contrato V1 completo para que los clientes nunca necesiten
/// replicar parsers de proveedores.
fn cmd_watch(args: &[String]) -> ExitCode {
    let mut jsonl = false;
    let mut poll_seconds = 60u64;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--jsonl" => jsonl = true,
            "--poll-seconds" => {
                i += 1;
                poll_seconds = match args.get(i).and_then(|value| value.parse().ok()) {
                    Some(seconds) if seconds >= 15 => seconds,
                    _ => return fail("watch: --poll-seconds debe ser al menos 15"),
                };
            }
            other => return fail(&format!("watch: argumento desconocido {other}")),
        }
        i += 1;
    }
    if !jsonl {
        return fail("watch: requiere --jsonl para mantener stdout como protocolo estable");
    }
    let root = watch::codex_sessions_dir();
    if !root.is_dir() {
        return fail(&format!("watch: no existe el directorio de sesiones Codex ({})", root.display()));
    }

    let (cfg, mut snapshots) = load_state(false, None);
    emit_watch_snapshot("initial", &cfg, &snapshots);
    match watch::run_codex_session_watch(
        &root,
        watch::DEFAULT_DEBOUNCE,
        std::time::Duration::from_secs(poll_seconds),
        |tick| match tick {
            watch::WatchTick::CodexSessionChanged => {
                if let Some(snapshot) = iausage_core::providers::codex_snapshot_from_sessions() {
                    let _ = iausage_core::cache::save_valid(&snapshot);
                    snapshots.insert(snapshot.id.clone(), snapshot);
                    emit_watch_snapshot("codex-session", &cfg, &snapshots);
                }
            }
            watch::WatchTick::RemotePoll => {
                let (fresh_cfg, fresh) = load_state(true, None);
                snapshots = fresh;
                emit_watch_snapshot("remote-poll", &fresh_cfg, &snapshots);
            }
        },
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("iausage: watch: {error}");
            ExitCode::from(guard::EXIT_UNAVAILABLE as u8)
        }
    }
}

fn emit_watch_snapshot(source: &str, cfg: &AppConfig, snapshots: &HashMap<String, ProviderSnapshot>) {
    let snapshot: serde_json::Value = serde_json::from_str(&snapshot_v1_json(cfg, snapshots)).unwrap_or_else(|_| serde_json::json!({}));
    println!("{}", serde_json::json!({ "type": "snapshot", "source": source, "snapshot": snapshot }));
}

fn cmd_guard(args: &[String]) -> ExitCode {
    let mut provider: Option<String> = None;
    let mut window: Option<String> = None;
    let mut min_remaining = 15.0f64;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--provider" => {
                i += 1;
                provider = args.get(i).cloned();
            }
            "--window" => {
                i += 1;
                window = args.get(i).cloned();
            }
            "--min-remaining" => {
                i += 1;
                match args.get(i).and_then(|s| s.parse::<f64>().ok()) {
                    Some(n) if (0.0..=100.0).contains(&n) => min_remaining = n,
                    _ => return fail("guard: --min-remaining debe ser 0–100"),
                }
            }
            s => return fail(&format!("guard: argumento desconocido {s}")),
        }
        i += 1;
    }
    let Some(p) = provider else {
        return fail("guard: falta --provider ID");
    };
    if snapshot_v1::parse_id(&p).is_none() {
        return fail(&format!("guard: provider desconocido {p}"));
    }
    let (_cfg, snaps) = load_state(false, None);
    match guard::evaluate(snaps.get(&p), window.as_deref(), min_remaining) {
        guard::GuardVerdict::Ok { remaining } => {
            println!("OK: {p} tiene {:.0}% disponible (mínimo {min_remaining:.0}%).", remaining);
            ExitCode::from(guard::EXIT_OK as u8)
        }
        guard::GuardVerdict::Below { remaining, min } => {
            println!("BAJO LÍMITE: {p} tiene {:.0}% disponible (mínimo {min:.0}%).", remaining);
            ExitCode::from(guard::EXIT_BELOW as u8)
        }
        guard::GuardVerdict::Unavailable { reason } => {
            eprintln!("iausage: {p} no disponible ({reason}).");
            ExitCode::from(guard::EXIT_UNAVAILABLE as u8)
        }
    }
}

fn cmd_config(args: &[String]) -> ExitCode {
    if args != ["validate"] {
        return fail("config: uso `iausage config validate`");
    }
    let cfg = AppConfig::load();
    let known: Vec<&str> = VendorId::all().iter().map(|id| id.slug()).collect();
    let mut warnings = 0;
    for key in cfg.providers.keys() {
        if !known.contains(&key.as_str()) {
            eprintln!("aviso: provider desconocido en config: {key}");
            warnings += 1;
        }
    }
    if !(1..=30).contains(&cfg.refresh_minutes) && ![1, 2, 5, 15, 30].contains(&cfg.refresh_minutes) {
        eprintln!("aviso: refresh_minutes fuera de rango: {}", cfg.refresh_minutes);
        warnings += 1;
    }
    if warnings == 0 {
        println!("config válida.");
    } else {
        println!("config válida con {warnings} aviso(s).");
    }
    ExitCode::SUCCESS
}

fn cmd_enable(args: &[String], enabled: bool) -> ExitCode {
    if args.len() != 1 {
        return fail("uso: iausage enable|disable <provider>");
    }
    let Some(id) = snapshot_v1::parse_id(&args[0]) else {
        return fail(&format!("provider desconocido {}", args[0]));
    };
    let mut cfg = AppConfig::load();
    cfg.set_enabled(id, enabled);
    match cfg.save() {
        Ok(()) => {
            println!(
                "{} {}.",
                id.display_name(),
                if enabled { "activado" } else { "desactivado" }
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("iausage: no se pudo guardar la config: {e}");
            ExitCode::from(guard::EXIT_UNAVAILABLE as u8)
        }
    }
}

fn cmd_sync(args: &[String]) -> ExitCode {
    let Some((sub, rest)) = args.split_first() else {
        return fail(
            "sync: uso `iausage sync <export|verify|status|set-passphrase|enable|disable|serve|qr>`",
        );
    };
    match sub.as_str() {
        "export" => cmd_sync_export(rest),
        "verify" => cmd_sync_verify(rest),
        "status" => cmd_sync_status(rest),
        "set-passphrase" => cmd_sync_set_passphrase(rest),
        "enable" => cmd_sync_enable(rest, true),
        "disable" => cmd_sync_enable(rest, false),
        "serve" => cmd_sync_serve(rest),
        "qr" => cmd_sync_qr(rest),
        s => fail(&format!("sync: subcomando desconocido {s}")),
    }
}

fn cmd_sync_export(args: &[String]) -> ExitCode {
    let mut out: Option<String> = None;
    let mut refresh = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                out = args.get(i).cloned();
                if out.is_none() {
                    return fail("sync export: falta valor para --out");
                }
            }
            "--refresh" => refresh = true,
            s => return fail(&format!("sync export: argumento desconocido {s}")),
        }
        i += 1;
    }
    let (mut cfg, snaps) = load_state(refresh, None);
    if !cfg.sync_enabled {
        return fail("sync desactivado; usa `iausage sync enable` primero");
    }
    if let Some(dir) = out {
        cfg.sync_export_dir = Some(dir);
    }
    match sync::export_current_snapshot(&cfg, &snaps, Some(VERSION.into())) {
        Ok(Some(path)) => {
            println!("Sync exportado a {}.", path.display());
            ExitCode::SUCCESS
        }
        Ok(None) => fail("sync desactivado; usa `iausage sync enable` primero"),
        Err(e) => {
            eprintln!("iausage: {e}");
            ExitCode::from(guard::EXIT_UNAVAILABLE as u8)
        }
    }
}

fn cmd_sync_verify(args: &[String]) -> ExitCode {
    if args.len() != 1 {
        return fail("sync verify: uso `iausage sync verify <archivo>`");
    }
    let path = std::path::Path::new(&args[0]);
    let blob = match sync::read_blob_file(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("iausage: {e}");
            return ExitCode::from(guard::EXIT_UNAVAILABLE as u8);
        }
    };
    let passphrase = match sync::load_passphrase() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("iausage: sin passphrase guardada; usa `iausage sync set-passphrase`.");
            return ExitCode::from(guard::EXIT_UNAVAILABLE as u8);
        }
    };
    match sync::decrypt_blob(&blob, &passphrase) {
        Ok(payload) => {
            println!(
                "OK: blob v{} · dispositivo {} · generado {} · {} providers.",
                blob.v,
                payload.device_id,
                payload.generated_at,
                payload.snapshot.providers.len()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("iausage: {e}");
            ExitCode::from(guard::EXIT_UNAVAILABLE as u8)
        }
    }
}

fn cmd_sync_status(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        return fail("sync status: no admite argumentos");
    }
    let cfg = AppConfig::load();
    let device_id = sync::load_or_create_device_id().unwrap_or_else(|_| "—".into());
    println!(
        "sync: {}",
        if cfg.sync_enabled {
            "activado"
        } else {
            "desactivado"
        }
    );
    println!(
        "dispositivo: {device_id} ({})",
        sync::pairing_fingerprint(&device_id)
    );
    println!("carpeta: {}", sync::resolve_export_dir(&cfg).display());
    println!(
        "passphrase: {}",
        if sync::has_passphrase() {
            "guardada"
        } else {
            "ausente"
        }
    );
    let blob = sync::resolve_export_dir(&cfg).join(format!("{device_id}.json"));
    match std::fs::metadata(&blob) {
        Ok(meta) => println!("último export: {} ({} bytes)", blob.display(), meta.len()),
        Err(_) => println!("último export: ninguno"),
    }
    ExitCode::SUCCESS
}

fn cmd_sync_set_passphrase(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        return fail("sync set-passphrase: lee de stdin, sin argumentos");
    }
    eprintln!("Escribe la passphrase y pulsa Enter (vacía = olvidar):");
    let mut line = String::new();
    if std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut line).is_err() {
        return fail("sync set-passphrase: no se pudo leer stdin");
    }
    match sync::store_passphrase(line.trim()) {
        Ok(()) => {
            println!("passphrase actualizada.");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("iausage: {e}");
            ExitCode::from(guard::EXIT_UNAVAILABLE as u8)
        }
    }
}

fn cmd_sync_enable(args: &[String], enabled: bool) -> ExitCode {
    if !args.is_empty() {
        return fail("sync enable|disable: sin argumentos");
    }
    if enabled && !sync::has_passphrase() {
        return fail("sync: primero `iausage sync set-passphrase`");
    }
    let mut cfg = AppConfig::load();
    cfg.sync_enabled = enabled;
    match cfg.save() {
        Ok(()) => {
            println!("sync {}.", if enabled { "activado" } else { "desactivado" });
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("iausage: no se pudo guardar la config: {e}");
            ExitCode::from(guard::EXIT_UNAVAILABLE as u8)
        }
    }
}

fn cmd_sync_serve(args: &[String]) -> ExitCode {
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    let mut port = iausage_core::sync_server::SYNC_DEFAULT_PORT;
    let mut lan = false;
    let mut refresh = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                match args.get(i).and_then(|s| s.parse::<u16>().ok()) {
                    Some(p) => port = p,
                    None => return fail("sync serve: --port debe ser 1–65535"),
                }
            }
            "--lan" => lan = true,
            "--refresh" => refresh = true,
            s => return fail(&format!("sync serve: argumento desconocido {s}")),
        }
        i += 1;
    }
    let passphrase = match sync::load_passphrase() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("iausage: sin passphrase guardada; usa `iausage sync set-passphrase`.");
            return ExitCode::from(guard::EXIT_UNAVAILABLE as u8);
        }
    };
    let device_id = match sync::load_or_create_device_id() {
        Ok(id) => id,
        Err(e) => {
            eprintln!("iausage: {e}");
            return ExitCode::from(guard::EXIT_UNAVAILABLE as u8);
        }
    };
    // Foto fija al arrancar: el CLI no re-consulta providers por petición.
    let (cfg, snaps) = load_state(refresh, None);
    let catalog = providers::catalog(&cfg);
    let snapshot = snapshot_v1::build(&snaps, &catalog, now_iso(), Some(VERSION.into()));
    let payload = sync::build_payload(device_id.clone(), now_iso(), snapshot);
    let serve_cfg = if lan {
        iausage_core::sync_server::ServeConfig {
            bind: "0.0.0.0".into(),
            port,
            lan: true,
        }
    } else {
        iausage_core::sync_server::ServeConfig::loopback(port)
    };
    println!(
        "sync sirviendo en {} (Ctrl+C para parar).",
        serve_cfg.addr()
    );
    let supplier: iausage_core::sync_server::PayloadFn = Arc::new(move || Ok(payload.clone()));
    let stop = Arc::new(AtomicBool::new(false));
    match iausage_core::sync_server::run_server(
        &serve_cfg,
        VERSION,
        &device_id,
        supplier,
        &passphrase,
        stop,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("iausage: {e}");
            ExitCode::from(guard::EXIT_UNAVAILABLE as u8)
        }
    }
}

fn cmd_sync_qr(args: &[String]) -> ExitCode {
    let mut lan = false;
    let mut port = iausage_core::sync_server::SYNC_DEFAULT_PORT;
    let mut out: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--lan" => lan = true,
            "--port" => {
                i += 1;
                match args.get(i).and_then(|s| s.parse::<u16>().ok()) {
                    Some(p) => port = p,
                    None => return fail("sync qr: --port debe ser 1–65535"),
                }
            }
            "--out" => {
                i += 1;
                out = args.get(i).cloned();
                if out.is_none() {
                    return fail("sync qr: falta valor para --out");
                }
            }
            s => return fail(&format!("sync qr: argumento desconocido {s}")),
        }
        i += 1;
    }
    let device_id = match sync::load_or_create_device_id() {
        Ok(id) => id,
        Err(e) => {
            eprintln!("iausage: {e}");
            return ExitCode::from(guard::EXIT_UNAVAILABLE as u8);
        }
    };
    let host = if lan {
        match iausage_core::sync_server::lan_ip() {
            Some(ip) => ip,
            None => {
                eprintln!("iausage: sin IP LAN detectable en este equipo.");
                return ExitCode::from(guard::EXIT_UNAVAILABLE as u8);
            }
        }
    } else {
        eprintln!("aviso: sin --lan el QR apunta a 127.0.0.1 (inútil para el teléfono).");
        "127.0.0.1".to_string()
    };
    let info = iausage_core::sync_server::PairingInfo::new(host, port, &device_id);
    let uri = info.to_uri();
    let path = out.unwrap_or_else(|| "pairing-qr.png".to_string());
    match iausage_core::sync_server::pairing_qr_png(&uri, 512) {
        Ok(png) => {
            if let Err(e) = std::fs::write(&path, &png) {
                eprintln!("iausage: no se pudo escribir {path}: {e}");
                return ExitCode::from(guard::EXIT_UNAVAILABLE as u8);
            }
        }
        Err(e) => {
            eprintln!("iausage: {e}");
            return ExitCode::from(guard::EXIT_UNAVAILABLE as u8);
        }
    }
    println!("QR guardado en {path}");
    println!("URI: {uri}");
    println!(
        "fingerprint: {} (verifícalo en el teléfono)",
        info.fingerprint
    );
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.is_empty() || argv[0] == "--help" || argv[0] == "-h" || argv[0] == "help" {
        println!("{}", usage_help());
        return ExitCode::SUCCESS;
    }
    match argv[0].as_str() {
        "usage" => cmd_usage(&argv[1..]),
        "providers" => cmd_providers(&argv[1..]),
        "best" => cmd_best(&argv[1..]),
        "doctor" => cmd_doctor(&argv[1..]),
        "refresh" => cmd_refresh(&argv[1..]),
        "watch" => cmd_watch(&argv[1..]),
        "guard" => cmd_guard(&argv[1..]),
        "enable" => cmd_enable(&argv[1..], true),
        "disable" => cmd_enable(&argv[1..], false),
        "config" => cmd_config(&argv[1..]),
        "sync" => cmd_sync(&argv[1..]),
        "version" | "--version" | "-V" => {
            println!("iausage {VERSION} (snapshot v{})", iausage_core::SNAPSHOT_SCHEMA_VERSION);
            ExitCode::SUCCESS
        }
        other => fail(&format!("comando desconocido {other}")),
    }
}
