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
use iausage_core::{providers, snapshot_v1};

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
          iausage guard --provider ID [--window W] [--min-remaining N]\n  \
          iausage enable <provider> | iausage disable <provider>\n  \
          iausage config validate\n  \
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
    let _ = cfg;
    ExitCode::SUCCESS
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
        "guard" => cmd_guard(&argv[1..]),
        "enable" => cmd_enable(&argv[1..], true),
        "disable" => cmd_enable(&argv[1..], false),
        "config" => cmd_config(&argv[1..]),
        "version" | "--version" | "-V" => {
            println!("iausage {VERSION} (snapshot v{})", iausage_core::SNAPSHOT_SCHEMA_VERSION);
            ExitCode::SUCCESS
        }
        other => fail(&format!("comando desconocido {other}")),
    }
}
