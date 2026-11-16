//! Calcula el costo a partir de los logs locales de Claude Code.
//!
//! Parsea %USERPROFILE%\.claude\projects\**\*.jsonl, igual que ccusage:
//!  - solo lineas con type=="assistant" que traen message.usage
//!  - deduplica por requestId (se queda con el registro mas completo)
//!  - multiplica tokens por el precio del modelo
//!
//! Nota: Claude Code no guarda el costo real, asi que esto es una ESTIMACION.
//! Los tokens "mostrados" excluyen la lectura de cache (que es enorme y barata)
//! para que el numero se parezca al de los dashboards oficiales.

use crate::paths::claude_dir;
use crate::pricing;
use chrono::{DateTime, Datelike, Duration, Local, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// Solo miramos archivos modificados en los ultimos N dias (cubre hoy / semana
/// / mes-calendario de 31d / 30d rolling). Acelera mucho el escaneo. Margen
/// extra por desfase de mtime de OneDrive y zonas horarias.
const WINDOW_DAYS: i64 = 35;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostReport {
    pub today_usd: f64,
    pub today_tokens: u64,
    pub week_usd: f64,
    pub week_tokens: u64,
    pub month_usd: f64,
    pub month_tokens: u64,
    pub last30_usd: f64,
    pub last30_tokens: u64,
    pub updated_at: String,
    pub empty: bool,
}

#[derive(Clone)]
struct Record {
    ts: DateTime<Utc>,
    model: String,
    input: u64,
    output: u64,
    cache_create_5m: u64,
    cache_create_1h: u64,
    cache_read: u64,
}

impl Record {
    fn total_tokens(&self) -> u64 {
        self.input + self.output + self.cache_create_5m + self.cache_create_1h + self.cache_read
    }
    /// Tokens "visibles" (sin lectura de cache).
    fn display_tokens(&self) -> u64 {
        self.input + self.output + self.cache_create_5m + self.cache_create_1h
    }
    fn cost(&self) -> f64 {
        pricing::cost_usd(
            &self.model,
            self.input,
            self.output,
            self.cache_create_5m,
            self.cache_create_1h,
            self.cache_read,
        )
    }
}

fn u64_at(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(|x| x.as_u64()).unwrap_or(0)
}

/// Parsea una linea jsonl a Record (o None si no es un assistant con usage).
fn parse_line(line: &str) -> Option<(String, Record)> {
    if !line.contains("\"assistant\"") {
        return None;
    }
    let v: Value = serde_json::from_str(line).ok()?;
    if v.get("type").and_then(|x| x.as_str()) != Some("assistant") {
        return None;
    }
    let msg = v.get("message")?;
    let usage = msg.get("usage")?;

    let ts_str = v.get("timestamp").and_then(|x| x.as_str())?;
    let ts = DateTime::parse_from_rfc3339(ts_str)
        .ok()?
        .with_timezone(&Utc);

    let model = msg
        .get("model")
        .and_then(|x| x.as_str())
        .unwrap_or("unknown")
        .to_string();

    // Desglose de cache_creation si existe; si no, todo va al bucket de 5m.
    let cc_total = u64_at(usage, "cache_creation_input_tokens");
    let (cc_5m, cc_1h) = match usage.get("cache_creation") {
        Some(b) if b.is_object() => (
            u64_at(b, "ephemeral_5m_input_tokens"),
            u64_at(b, "ephemeral_1h_input_tokens"),
        ),
        _ => (cc_total, 0),
    };
    // Si el desglose no cuadra con el total, usamos el total como 5m.
    let (cc_5m, cc_1h) = if cc_5m + cc_1h == 0 && cc_total > 0 {
        (cc_total, 0)
    } else {
        (cc_5m, cc_1h)
    };

    let record = Record {
        ts,
        model,
        input: u64_at(usage, "input_tokens"),
        output: u64_at(usage, "output_tokens"),
        cache_create_5m: cc_5m,
        cache_create_1h: cc_1h,
        cache_read: u64_at(usage, "cache_read_input_tokens"),
    };

    // Clave de deduplicacion: requestId; si no, el uuid de la linea.
    let key = v
        .get("requestId")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("uuid").and_then(|x| x.as_str()))
        .unwrap_or("")
        .to_string();
    if key.is_empty() {
        return None;
    }
    Some((key, record))
}

/// Registros ya parseados de un archivo, válidos mientras no cambien su
/// tamaño ni su mtime.
struct CachedFile {
    len: u64,
    modified: SystemTime,
    records: Vec<(String, Record)>,
}

/// Caché por archivo para todo el proceso. Usuarios intensivos acumulan GB
/// de logs; releerlos enteros en cada refresh superaba el plazo del
/// dashboard y el uso de Claude nunca se actualizaba. Con la caché solo se
/// reparsean los archivos que cambiaron (la sesión activa). El Mutex
/// además serializa escaneos concurrentes.
static FILE_CACHE: Mutex<Option<HashMap<PathBuf, CachedFile>>> = Mutex::new(None);

fn parse_file(path: &Path) -> Option<Vec<(String, Record)>> {
    let file = std::fs::File::open(path).ok()?;
    // Línea a línea: no cargamos archivos de cientos de MB enteros en memoria.
    Some(
        BufReader::new(file)
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| parse_line(&line))
            .collect(),
    )
}

/// Recorre los logs y agrega el costo en ventanas de tiempo.
pub fn compute() -> CostReport {
    let projects = claude_dir().join("projects");
    let cutoff_file = SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(
            (WINDOW_DAYS as u64) * 86_400,
        ))
        .unwrap_or(std::time::UNIX_EPOCH);

    let mut guard = FILE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let mut previous = guard.take().unwrap_or_default();
    let mut cache: HashMap<PathBuf, CachedFile> = HashMap::with_capacity(previous.len());

    for entry in walkdir::WalkDir::new(&projects).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().map(|e| e != "jsonl").unwrap_or(true) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let modified = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
        // Salta archivos viejos (fuera de la ventana).
        if modified < cutoff_file {
            continue;
        }
        let len = meta.len();
        let cached = previous
            .remove(path)
            .filter(|c| c.len == len && c.modified == modified);
        let file = match cached {
            Some(c) => c,
            None => match parse_file(path) {
                Some(records) => CachedFile { len, modified, records },
                None => continue,
            },
        };
        cache.insert(path.to_path_buf(), file);
    }

    // requestId -> registro mas completo (mayor cantidad de tokens).
    let mut records: HashMap<&str, &Record> = HashMap::new();
    for file in cache.values() {
        for (key, rec) in &file.records {
            match records.get(key.as_str()) {
                Some(existing) if existing.total_tokens() >= rec.total_tokens() => {}
                _ => {
                    records.insert(key, rec);
                }
            }
        }
    }

    let now: DateTime<Local> = Local::now();
    let today = now.date_naive();
    let week_cutoff = now - Duration::days(7);
    let last30_cutoff = now - Duration::days(30);

    let mut report = CostReport {
        updated_at: now.to_rfc3339(),
        empty: records.is_empty(),
        ..Default::default()
    };

    for rec in records.values() {
        let local = rec.ts.with_timezone(&Local);
        let date = local.date_naive();
        let cost = rec.cost();
        let tokens = rec.display_tokens();

        if date == today {
            report.today_usd += cost;
            report.today_tokens += tokens;
        }
        if local >= week_cutoff {
            report.week_usd += cost;
            report.week_tokens += tokens;
        }
        if date.year() == today.year() && date.month() == today.month() {
            report.month_usd += cost;
            report.month_tokens += tokens;
        }
        if local >= last30_cutoff {
            report.last30_usd += cost;
            report.last30_tokens += tokens;
        }
    }

    drop(records);
    *guard = Some(cache);
    report
}
