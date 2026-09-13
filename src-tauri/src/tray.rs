//! Icono de la bandeja (dibujado dinamicamente con el % de la sesion) y el
//! comportamiento de la ventana anclada a la bandeja.
//! En Windows la bandeja no muestra texto al lado del icono (como en macOS),
//! asi que "pintamos" el numero dentro del propio icono.

use std::collections::HashMap;
use tauri::image::Image;
use tauri::menu::CheckMenuItem;
use tauri::{AppHandle, Manager};

use crate::model::{Dashboard, ProviderSnapshot, VendorInfo};
use crate::state::TrayMenuState;

const SIZE: u32 = 32;

#[inline]
fn blend(buf: &mut [u8], x: i32, y: i32, color: [u8; 3], alpha: f32) {
    if x < 0 || y < 0 || x >= SIZE as i32 || y >= SIZE as i32 || alpha <= 0.0 {
        return;
    }
    let a = alpha.clamp(0.0, 1.0);
    let idx = ((y as u32 * SIZE + x as u32) * 4) as usize;
    for k in 0..3 {
        let bg = buf[idx + k] as f32;
        let fg = color[k] as f32;
        buf[idx + k] = (fg * a + bg * (1.0 - a)) as u8;
    }
    let cur_a = buf[idx + 3] as f32 / 255.0;
    let new_a = a + cur_a * (1.0 - a);
    buf[idx + 3] = (new_a * 255.0) as u8;
}

/// Circulo relleno centrado, con antialias de 1px.
fn fill_circle(buf: &mut [u8], color: [u8; 3], alpha: f32, r: f32) {
    let c = SIZE as f32 / 2.0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 + 0.5 - c;
            let dy = y as f32 + 0.5 - c;
            let dist = (dx * dx + dy * dy).sqrt();
            let cov = (r - dist + 0.5).clamp(0.0, 1.0) * alpha;
            blend(buf, x as i32, y as i32, color, cov);
        }
    }
}

/// Anillo (borde) centrado de radio `r` y grosor `width`.
fn stroke_ring(buf: &mut [u8], color: [u8; 3], r: f32, width: f32) {
    let c = SIZE as f32 / 2.0;
    let half = width / 2.0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 + 0.5 - c;
            let dy = y as f32 + 0.5 - c;
            let dist = (dx * dx + dy * dy).sqrt();
            let cov = (half - (dist - r).abs() + 0.5).clamp(0.0, 1.0);
            blend(buf, x as i32, y as i32, color, cov);
        }
    }
}

/// Rectangulo redondeado relleno (SDF con antialias de ~1px), compartido por
/// la barra del small mark y la tipografia numerica minima del tray.
fn fill_rounded_rect(buf: &mut [u8], color: [u8; 3], x: f32, y: f32, w: f32, h: f32, r: f32) {
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let hw = w / 2.0 - r;
    let hh = h / 2.0 - r;
    for py in 0..SIZE {
        for px in 0..SIZE {
            let dx = (px as f32 + 0.5 - cx).abs() - hw;
            let dy = (py as f32 + 0.5 - cy).abs() - hh;
            let qx = dx.max(0.0);
            let qy = dy.max(0.0);
            let dist = (qx * qx + qy * qy).sqrt() - r;
            let cov = (0.5 - dist).clamp(0.0, 1.0);
            blend(buf, px as i32, py as i32, color, cov);
        }
    }
}

/// Segmento redondeado antialias. Permite dibujar el small mark oficial sin
/// depender de un raster que Windows pueda escalar de forma distinta.
fn stroke_segment(buf: &mut [u8], color: [u8; 3], a: (f32, f32), b: (f32, f32), width: f32) {
    let vx = b.0 - a.0;
    let vy = b.1 - a.1;
    let length_sq = vx * vx + vy * vy;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let t = (((px - a.0) * vx + (py - a.1) * vy) / length_sq).clamp(0.0, 1.0);
            let dx = px - (a.0 + t * vx);
            let dy = py - (a.1 + t * vy);
            let distance = (dx * dx + dy * dy).sqrt();
            let coverage = (width / 2.0 - distance + 0.5).clamp(0.0, 1.0);
            blend(buf, x as i32, y as i32, color, coverage);
        }
    }
}

/// Small mark monocromatico: I + A con una sola barra. Se mantiene simple para
/// que sobreviva cuando Windows reduce el bitmap de 32 px a 16–24 px.
fn draw_small_mark(buf: &mut [u8]) {
    let white = [245, 245, 245];
    stroke_segment(buf, white, (7.0, 8.5), (7.0, 23.5), 4.0);
    stroke_segment(buf, white, (11.5, 23.5), (18.0, 7.5), 4.0);
    stroke_segment(buf, white, (18.0, 7.5), (26.0, 23.5), 4.0);
    fill_rounded_rect(buf, white, 14.0, 16.5, 8.0, 3.0, 1.0);
}

const DIGITS_3X5: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b010, 0b010, 0b010],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
];

fn draw_usage_number(buf: &mut [u8], percent: f64) {
    let value = percent.round().clamp(0.0, 100.0) as u32;
    let digits: Vec<usize> = value
        .to_string()
        .bytes()
        .map(|digit| (digit - b'0') as usize)
        .collect();
    let pixel = if digits.len() == 3 { 1.65 } else { 2.2 };
    let gap = pixel * 0.55;
    let digit_width = pixel * 3.0;
    let total_width = digit_width * digits.len() as f32 + gap * (digits.len() - 1) as f32;
    let origin_x = (SIZE as f32 - total_width) / 2.0;
    let origin_y = (SIZE as f32 - pixel * 5.0) / 2.0;

    for (index, digit) in digits.into_iter().enumerate() {
        let digit_x = origin_x + index as f32 * (digit_width + gap);
        for (row, bits) in DIGITS_3X5[digit].iter().enumerate() {
            for column in 0..3 {
                if *bits & (1_u8 << (2 - column)) != 0 {
                    fill_rounded_rect(
                        buf,
                        [245, 245, 245],
                        digit_x + column as f32 * pixel,
                        origin_y + row as f32 * pixel,
                        pixel * 0.82,
                        pixel * 0.82,
                        0.35,
                    );
                }
            }
        }
    }
}

/// Color del anillo segun el nivel de uso (paleta de la UI).
fn severity_color(util: f64) -> [u8; 3] {
    if util >= 90.0 {
        [248, 113, 113] // rojo
    } else if util >= 70.0 {
        [251, 146, 60] // naranja
    } else if util >= 40.0 {
        [251, 191, 36] // ambar
    } else {
        [74, 222, 128] // verde
    }
}

/// Sin datos muestra el small mark mono. Con datos prioriza la utilidad: valor
/// redondeado al centro y anillo de severidad; el porcentaje exacto queda en el
/// tooltip.
pub fn render(percent: Option<f64>) -> Image<'static> {
    let mut buf = vec![0u8; (SIZE * SIZE * 4) as usize];
    fill_circle(&mut buf, [24, 24, 27], 0.97, 15.0);
    if let Some(value) = percent {
        stroke_ring(&mut buf, severity_color(value), 13.8, 2.2);
        draw_usage_number(&mut buf, value);
    } else {
        draw_small_mark(&mut buf);
    }

    Image::new_owned(buf, SIZE, SIZE)
}

pub(crate) fn position_window(win: &tauri::WebviewWindow, anchor_x: f64, anchor_y: f64) {
    let size = win
        .outer_size()
        .unwrap_or(tauri::PhysicalSize::new(360, 500));
    let w = size.width as f64;
    let h = size.height as f64;
    let mut x = anchor_x - w + 12.0;
    let mut y = anchor_y - h - 12.0;

    if let Ok(Some(mon)) = win.current_monitor() {
        let mp = mon.position();
        let ms = mon.size();
        let left = mp.x as f64;
        let top = mp.y as f64;
        let right = left + ms.width as f64;
        let bottom = top + ms.height as f64;
        if x + w > right {
            x = right - w - 4.0;
        }
        if x < left {
            x = left + 4.0;
        }
        if y + h > bottom {
            y = bottom - h - 4.0;
        }
        if y < top {
            y = top + 4.0;
        }
    } else {
        if x < 0.0 {
            x = anchor_x + 12.0;
        }
        if y < 0.0 {
            y = anchor_y + 12.0;
        }
    }
    let _ = win.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
}

pub(crate) fn show_window(app: &AppHandle, anchor: Option<(f64, f64)>) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_skip_taskbar(false);
        if let Some((x, y)) = anchor {
            position_window(&win, x, y);
        }
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

pub(crate) fn on_tray_left_click(app: &AppHandle, x: f64, y: f64) {
    if let Some(win) = app.get_webview_window("main") {
        let visible = win.is_visible().unwrap_or(false);
        let minimized = win.is_minimized().unwrap_or(false);
        if visible && !minimized {
            let _ = win.hide();
        } else {
            show_window(app, Some((x, y)));
        }
    }
}

/// Native Windows tray tooltip: plain text, no styling, proportional font
/// (so column alignment via spaces would not actually line up). The only
/// levers available are line breaks and ordering, so each provider gets its
/// own row under a short title instead of one crowded "CLD 65% · CDX 18%"
/// line.
pub(crate) fn tooltip(dash: &Dashboard) -> String {
    let rows: Vec<String> = dash
        .providers
        .iter()
        .filter(|p| p.is_connected())
        .filter_map(|p| {
            p.primary_utilization
                .map(|u| format!("{} {:.0}%", p.short, u))
        })
        .take(4)
        .collect();
    let full = if rows.is_empty() {
        "IA Usage".into()
    } else {
        format!("IA Usage\n\n{}", rows.join("\n"))
    };
    // Windows truncates long tray tooltips; cut elegantly ourselves.
    const MAX_TOOLTIP: usize = 128;
    if full.len() <= MAX_TOOLTIP {
        return full;
    }
    let mut cut = MAX_TOOLTIP - 1;
    while cut > 0 && !full.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}…", &full[..cut])
}

/// Short root header. Deliberately terse: every extra word widens the
/// native menu. Detailed percents live in the tooltip + Proveedor submenu.
pub(crate) fn header_text(enabled_count: usize) -> String {
    format!("IA Usage · {enabled_count} activos")
}

/// One submenu line per ENABLED vendor (activar != conectado): percent
/// when there is data, otherwise a short actionable state. Pure function
/// so the wording is unit-tested, not eyeballed.
pub(crate) fn provider_menu_text(
    vendor: &VendorInfo,
    snapshot: Option<&ProviderSnapshot>,
) -> String {
    const SEP: &str = "  ·  ";
    match snapshot {
        Some(s) if s.is_connected() => match s.primary_utilization {
            Some(u) => format!("{}{SEP}{:.0}%", vendor.name, u),
            None => format!("{}{SEP}Conectado", vendor.name),
        },
        Some(s) => {
            let state = match s.status {
                crate::model::ProviderStatus::NeedsAuth => {
                    if s.status_reason
                        == Some(crate::model::ProviderStatusReason::InvalidCredential)
                    {
                        "Sesión no válida"
                    } else if !vendor.has_credential && vendor.needs_key {
                        "Falta API key"
                    } else {
                        "Falta sesión"
                    }
                }
                crate::model::ProviderStatus::NeedsPermission => "Falta permiso",
                crate::model::ProviderStatus::Error => "Error",
                crate::model::ProviderStatus::Unavailable => "No disponible",
                crate::model::ProviderStatus::Connected => "Conectado",
            };
            format!("{}{SEP}{state}", vendor.name)
        }
        None if !vendor.has_credential && vendor.needs_key => {
            format!("{}{SEP}Falta API key", vendor.name)
        }
        None => format!("{}{SEP}Sin datos", vendor.name),
    }
}

pub(crate) fn tray_percent(dash: &Dashboard) -> Option<f64> {
    let primary = dash
        .providers
        .iter()
        .find(|p| p.id == dash.primary)
        .and_then(|p| p.primary_utilization);
    if primary.is_some() {
        return primary;
    }
    dash.providers
        .iter()
        .filter_map(|p| p.primary_utilization)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

/// Rebuilds the "Proveedor" submenu: one checkbox per ENABLED vendor
/// (checked iff current primary) with live percent/state text, then
/// management entries. Called at startup, on every catalog change, and on
/// every dashboard update so percents never go stale.
pub(crate) fn rebuild_provider_submenu(
    app: &AppHandle,
    catalog: &[VendorInfo],
    primary: &str,
    snapshots: &HashMap<String, ProviderSnapshot>,
) {
    let Some(menu) = app.try_state::<TrayMenuState>() else {
        return;
    };
    let submenu = &menu.primary_submenu;
    for item in submenu.items().unwrap_or_default() {
        let _ = submenu.remove(&item);
    }
    for vendor in catalog.iter().filter(|v| v.enabled) {
        if let Ok(item) = CheckMenuItem::with_id(
            app,
            format!("primary_{}", vendor.id),
            provider_menu_text(vendor, snapshots.get(&vendor.id)),
            true,
            vendor.id == primary,
            None::<&str>,
        ) {
            let _ = submenu.append(&item);
        }
    }
    if let Ok(sep) = tauri::menu::PredefinedMenuItem::separator(app) {
        let _ = submenu.append(&sep);
    }
    if let Ok(item) = tauri::menu::MenuItem::with_id(
        app,
        "manage_providers",
        "Administrar proveedores…",
        true,
        None::<&str>,
    ) {
        let _ = submenu.append(&item);
    }
    if let Ok(item) = tauri::menu::MenuItem::with_id(
        app,
        "detect_submenu",
        "Detectar proveedores",
        true,
        None::<&str>,
    ) {
        let _ = submenu.append(&item);
    }
}

pub(crate) fn update_tray_from_dashboard(app: &AppHandle, dash: &Dashboard) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_icon(Some(render(tray_percent(dash))));
        let _ = tray.set_tooltip(Some(tooltip(dash)));
    }
    if let Some(menu) = app.try_state::<TrayMenuState>() {
        if let Some(state) = app.try_state::<crate::state::AppState>() {
            let catalog = crate::state::lock_or_recover(&state.catalog).clone();
            let snapshots = crate::state::lock_or_recover(&state.snapshots).clone();
            let config = crate::state::lock_or_recover(&state.config).clone();
            let n = catalog.iter().filter(|v| v.enabled).count();
            let _ = menu.header.set_text(header_text(n));
            rebuild_provider_submenu(app, &catalog, &config.primary, &snapshots);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{progress_pct, snapshot_ok, VendorId};

    #[test]
    fn tooltip_lists_connected_percents() {
        let claude = snapshot_ok(
            VendorId::Anthropic,
            "Max",
            vec![progress_pct(
                "session", "Sesión", 42.0, None, 18_000, "always",
            )],
        );
        let cursor = snapshot_ok(
            VendorId::Cursor,
            "Ultra",
            vec![progress_pct(
                "total", "Uso", 71.0, None, 2_592_000, "always",
            )],
        );
        let dash = Dashboard {
            providers: vec![claude, cursor],
            primary: "anthropic".into(),
            ..Default::default()
        };
        let tip = tooltip(&dash);
        assert!(tip.contains("CLD"));
        assert!(tip.contains("42"));
        assert!(tip.contains("CUR"));
        assert!(tip.contains("71"));
    }

    #[test]
    fn tray_percent_prefers_primary() {
        let claude = snapshot_ok(
            VendorId::Anthropic,
            "Max",
            vec![progress_pct(
                "session", "Sesión", 10.0, None, 18_000, "always",
            )],
        );
        let cursor = snapshot_ok(
            VendorId::Cursor,
            "Ultra",
            vec![progress_pct(
                "total", "Uso", 90.0, None, 2_592_000, "always",
            )],
        );
        let dash = Dashboard {
            providers: vec![claude, cursor],
            primary: "anthropic".into(),
            ..Default::default()
        };
        assert_eq!(tray_percent(&dash).map(|n| n.round()), Some(10.0));
    }

    fn test_vendor(id: VendorId, enabled: bool, has_credential: bool) -> VendorInfo {
        crate::model::VendorInfo {
            id: id.slug().to_string(),
            name: id.display_name().to_string(),
            short: id.short().to_string(),
            auth_kind: id.auth_kind(),
            env_key: id.env_key().map(|s| s.to_string()),
            hint: String::new(),
            needs_key: id.needs_api_key_ui(),
            enabled,
            detected: false,
            has_credential,
            credential_source: None,
            strategies: vec![],
            source_preference: None,
            links: crate::model::VendorLinks {
                usage_url: None,
                billing_url: None,
                status_url: None,
                docs_url: None,
                app_url: None,
                api_key_url: None,
                signup_url: None,
            },
        }
    }

    #[test]
    fn header_stays_short() {
        let h = header_text(4);
        assert_eq!(h, "IA Usage · 4 activos");
        assert!(h.len() < 40, "root header must not force menu width");
    }

    #[test]
    fn submenu_shows_percent_or_actionable_state() {
        use crate::model::{snapshot_with_status, ProviderStatus, ProviderStatusReason};
        let claude = test_vendor(VendorId::Anthropic, true, true);
        let mut connected = snapshot_ok(VendorId::Anthropic, "Max", vec![]);
        connected.primary_utilization = Some(92.4);
        assert_eq!(
            provider_menu_text(&claude, Some(&connected)),
            "Claude Code  ·  92%"
        );

        let deepseek = test_vendor(VendorId::Deepseek, true, false);
        let missing = snapshot_with_status(
            VendorId::Deepseek,
            ProviderStatus::NeedsAuth,
            ProviderStatusReason::MissingCredential,
            "",
        );
        assert_eq!(
            provider_menu_text(&deepseek, Some(&missing)),
            "DeepSeek  ·  Falta API key"
        );

        let invalid = snapshot_with_status(
            VendorId::Anthropic,
            ProviderStatus::NeedsAuth,
            ProviderStatusReason::InvalidCredential,
            "",
        );
        assert_eq!(
            provider_menu_text(&claude, Some(&invalid)),
            "Claude Code  ·  Sesión no válida"
        );

        // Enabled without data and without credential: actionable, not blank.
        assert_eq!(
            provider_menu_text(&deepseek, None),
            "DeepSeek  ·  Falta API key"
        );
    }

    #[test]
    fn tooltip_truncates_elegantly() {
        let snaps: Vec<_> = [
            VendorId::Anthropic,
            VendorId::Openai,
            VendorId::Cursor,
            VendorId::Copilot,
            VendorId::Openrouter,
            VendorId::Deepseek,
        ]
        .iter()
        .map(|id| {
            let mut s = snapshot_ok(*id, "Plan", vec![]);
            s.primary_utilization = Some(99.0);
            s
        })
        .collect();
        let dash = Dashboard {
            providers: snaps,
            primary: "anthropic".into(),
            ..Default::default()
        };
        let tip = tooltip(&dash);
        assert!(tip.len() <= 128, "tooltip must fit Windows limits: {tip}");
    }
}
