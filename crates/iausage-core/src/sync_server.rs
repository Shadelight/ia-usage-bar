//! M5 V1b: servidor HTTP local + pareo por QR.
//!
//! El servidor vive solo mientras el colector corre (la GUI lo arranca en un
//! hilo cuando sync está activo; el CLI con `iausage sync serve`). Sirve el
//! **blob cifrado**, nunca plaintext: la seguridad es la passphrase, no la
//! red. Por defecto solo loopback; LAN requiere opt-in explícito.
//!
//! ```text
//! GET /v1/meta      -> { schemaVersion, blobVersion, deviceId,
//!                        fingerprint, appVersion, lan } (público)
//! GET /v1/snapshot  -> EncryptedBlob JSON (opaco sin passphrase)
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::sync::{self, EncryptedBlob, SyncPayload, SYNC_BLOB_VERSION};
use crate::{CORE_VERSION, SNAPSHOT_SCHEMA_VERSION};

/// Puerto por defecto del sync local. Fijo para que el QR sea estable.
pub const SYNC_DEFAULT_PORT: u16 = 28741;

#[derive(Debug, Clone)]
pub struct ServeConfig {
    /// IP de bind, p.ej. `127.0.0.1` o `0.0.0.0` (LAN, solo con opt-in).
    pub bind: String,
    pub port: u16,
    /// `true` si se expone fuera de loopback (el QR entonces usa la IP LAN).
    pub lan: bool,
}

impl ServeConfig {
    pub fn loopback(port: u16) -> Self {
        Self {
            bind: "127.0.0.1".into(),
            port,
            lan: false,
        }
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.bind, self.port)
    }
}

pub type PayloadFn = Arc<dyn Fn() -> Result<SyncPayload, String> + Send + Sync>;

fn json_response(body: String, status: u16) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    use tiny_http::{Header, Response, StatusCode};
    let status = StatusCode::from(status);
    let content_type = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
    Response::from_data(body.into_bytes())
        .with_header(content_type)
        .with_status_code(status)
}

/// Cuerpo de `/v1/meta`. Sin datos sensibles: es lo que el teléfono lee
/// antes de decidir si habla con este colector.
pub fn meta_json(app_version: &str, device_id: &str, lan: bool) -> String {
    serde_json::json!({
        "schemaVersion": SNAPSHOT_SCHEMA_VERSION,
        "blobVersion": SYNC_BLOB_VERSION,
        "deviceId": device_id,
        "fingerprint": sync::pairing_fingerprint(device_id),
        "appVersion": app_version,
        "lan": lan,
    })
    .to_string()
}

fn snapshot_json(make_payload: &PayloadFn, passphrase: &str) -> (u16, String) {
    let blob: Result<EncryptedBlob, String> = (|| {
        let payload = make_payload()?;
        let salt = rand_salt_nonce_16();
        let nonce = rand_salt_nonce_24();
        sync::encrypt_payload_with(&payload, passphrase, &salt, &nonce)
    })();
    match blob {
        Ok(blob) => (
            200,
            serde_json::to_string(&blob).unwrap_or_else(|_| "{}".into()),
        ),
        Err(e) => (500, serde_json::json!({ "error": e }).to_string()),
    }
}

// La passphrase se resuelve por llamada para no retenerla en memoria del
// servidor más de lo necesario. El entorno no la guarda: viene de keyring.
// NOTA: hoy sin uso directo (run_server la recibe al arrancar); se conserva
// para futuros llamadores que prefieran resolución perezosa.
#[allow(dead_code)]
fn current_passphrase() -> Result<String, String> {
    sync::load_passphrase()
}

fn rand_salt_nonce_16() -> [u8; 16] {
    use rand::RngCore;
    let mut buf = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    buf
}

fn rand_salt_nonce_24() -> [u8; 24] {
    use rand::RngCore;
    let mut buf = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    buf
}

fn handle(
    request: tiny_http::Request,
    app_version: &str,
    device_id: &str,
    lan: bool,
    make_payload: &PayloadFn,
    passphrase: &str,
) {
    let (status, body) = match (request.method(), request.url()) {
        (&tiny_http::Method::Get, "/v1/meta") => (200, meta_json(app_version, device_id, lan)),
        (&tiny_http::Method::Get, "/v1/snapshot") => snapshot_json(make_payload, passphrase),
        _ => (
            404,
            serde_json::json!({ "error": "no encontrado" }).to_string(),
        ),
    };
    let _ = request.respond(json_response(body, status));
}

/// Solo el bind: separado de `serve` para que un llamador (la GUI) pueda
/// confirmar que el puerto se abrió antes de marcar el servidor "activo" y
/// recién entonces mover el bucle bloqueante a un hilo de fondo. Sin esto,
/// un puerto ocupado deja el estado en pantalla diciendo "activo" con
/// ningún socket real detrás.
pub fn bind(cfg: &ServeConfig) -> Result<tiny_http::Server, String> {
    tiny_http::Server::http(&cfg.addr()).map_err(|e| e.to_string())
}

/// Sirve hasta que `stop` se active. Bloquea el hilo actual.
///
/// La passphrase se carga UNA vez al arrancar (quien llama la lee de
/// keyring) y solo vive en este hilo: nada de depender del store en cada
/// petición, y el servidor es testeable sin keyring.
pub fn serve(
    server: &tiny_http::Server,
    app_version: &str,
    device_id: &str,
    lan: bool,
    make_payload: &PayloadFn,
    passphrase: &str,
    stop: &Arc<AtomicBool>,
) -> Result<(), String> {
    while !stop.load(Ordering::Relaxed) {
        match server.recv_timeout(Duration::from_millis(200)) {
            Ok(Some(request)) => handle(
                request,
                app_version,
                device_id,
                lan,
                make_payload,
                passphrase,
            ),
            Ok(None) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

/// Bind + serve en una sola llamada bloqueante. Usado por el CLI y los
/// tests, donde no hace falta separar "¿abrió el puerto?" de "servir".
pub fn run_server(
    cfg: &ServeConfig,
    app_version: &str,
    device_id: &str,
    make_payload: PayloadFn,
    passphrase: &str,
    stop: Arc<AtomicBool>,
) -> Result<(), String> {
    let server = bind(cfg)?;
    serve(
        &server,
        app_version,
        device_id,
        cfg.lan,
        &make_payload,
        passphrase,
        &stop,
    )
}

// ---------------------------------------------------------------------------
// Pareo
// ---------------------------------------------------------------------------

/// Datos para `iausage://pair?...`. Todo URL-safe por construcción: el QR no
/// necesita escaping y el teléfono lo parsea sin librerías extra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingInfo {
    pub host: String,
    pub port: u16,
    pub device_id: String,
    pub fingerprint: String,
    pub min_app_version: String,
}

impl PairingInfo {
    pub fn new(host: String, port: u16, device_id: &str) -> Self {
        let fingerprint = sync::pairing_fingerprint(device_id);
        Self {
            host,
            port,
            device_id: device_id.into(),
            fingerprint,
            min_app_version: CORE_VERSION.into(),
        }
    }

    pub fn to_uri(&self) -> String {
        format!(
            "iausage://pair?v=1&host={}&port={}&device={}&fp={}&minApp={}",
            self.host, self.port, self.device_id, self.fingerprint, self.min_app_version
        )
    }

    /// Valida el formato que genera [`PairingInfo::to_uri`] (el M6 lo
    /// reimplementa en Kotlin; esto fija el contrato aquí).
    pub fn parse_uri(uri: &str) -> Result<PairingInfo, String> {
        let query = uri
            .strip_prefix("iausage://pair?")
            .ok_or_else(|| "sync: URI de pareo inválida".to_string())?;
        let mut host = None;
        let mut port = None;
        let mut device = None;
        let mut fp = None;
        let mut min_app = None;
        let mut version = None;
        for part in query.split('&') {
            let (key, value) = part
                .split_once('=')
                .ok_or_else(|| "sync: parámetro de pareo inválido".to_string())?;
            match key {
                "v" => version = Some(value),
                "host" => host = Some(value),
                "port" => port = Some(value),
                "device" => device = Some(value),
                "fp" => fp = Some(value),
                "minApp" => min_app = Some(value),
                _ => {}
            }
        }
        if version != Some("1") {
            return Err("sync: versión de pareo no soportada".into());
        }
        let info = PairingInfo {
            host: host.ok_or("sync: pareo sin host")?.into(),
            port: port
                .ok_or("sync: pareo sin puerto")?
                .parse()
                .map_err(|_| "sync: puerto de pareo inválido")?,
            device_id: device.ok_or("sync: pareo sin device")?.into(),
            fingerprint: fp.ok_or("sync: pareo sin fingerprint")?.into(),
            min_app_version: min_app.ok_or("sync: pareo sin minApp")?.into(),
        };
        let expect = sync::pairing_fingerprint(&info.device_id);
        if info.fingerprint != expect {
            return Err("sync: fingerprint no coincide con el device (¿QR manipulado?)".into());
        }
        Ok(info)
    }
}

/// IP LAN para el QR (la que el teléfono debe alcanzar). `None` = sin red
/// local detectable.
///
/// `local_ip_address::local_ip()` toma en Windows la primera interfaz con
/// ruta por defecto en el orden de enumeración de `GetAdaptersAddresses` —
/// no la de menor métrica. Probado en hardware real con Wi-Fi + Ethernet
/// activos a la vez, devolvía la IP de Ethernet aunque el teléfono estaba
/// en Wi-Fi.
///
/// Tampoco basta con preguntarle al SO la ruta por defecto real (probado:
/// en esta misma máquina la ruta por defecto del sistema también prefiere
/// Ethernet por métrica). El pareo con el móvil solo puede llegar por la
/// red a la que el teléfono está conectado, casi siempre Wi-Fi, así que se
/// prioriza por tipo de adaptador en vez de por métrica de ruta: se
/// enumeran las interfaces IPv4 privadas, se descartan las virtuales
/// conocidas (VPN, contenedores, hipervisores) y se prefiere Wi-Fi sobre
/// Ethernet sobre cualquier otra.
pub fn lan_ip() -> Option<String> {
    lan_candidates()
        .into_iter()
        .min_by_key(|c| c.priority())
        .map(|c| c.ip.to_string())
}

struct LanCandidate {
    ip: std::net::Ipv4Addr,
    name: String,
}

impl LanCandidate {
    fn priority(&self) -> u8 {
        let name = self.name.to_lowercase();
        if name.contains("wi-fi") || name.contains("wifi") || name.contains("wlan") {
            0
        } else if name.contains("ethernet") || name.contains("lan") {
            1
        } else {
            2
        }
    }
}

/// Nombres de adaptadores virtuales que nunca deben ofrecerse como IP LAN
/// para el pareo móvil: el teléfono no puede alcanzarlos.
const VIRTUAL_ADAPTER_HINTS: &[&str] = &[
    "virtualbox",
    "vmware",
    "hyper-v",
    "vethernet",
    "docker",
    "wsl",
    "tailscale",
    "zerotier",
    "loopback",
    "tap-",
    "tun",
    "npcap",
    "vpn",
];

fn lan_candidates() -> Vec<LanCandidate> {
    let Ok(interfaces) = local_ip_address::list_afinet_netifas() else {
        return Vec::new();
    };
    interfaces
        .into_iter()
        .filter_map(|(name, ip)| as_candidate(&name, ip))
        .collect()
}

/// Filtro puro (sin tocar la red) para poder probar las reglas de exclusión
/// sin depender de las interfaces reales de la máquina que corre el test.
fn as_candidate(name: &str, ip: std::net::IpAddr) -> Option<LanCandidate> {
    let std::net::IpAddr::V4(v4) = ip else {
        return None;
    };
    // is_private() cubre 10/8, 172.16/12 y 192.168/16; is_link_local()
    // descarta el rango APIPA 169.254/16 que Windows asigna cuando la
    // interfaz no tiene DHCP funcionando.
    if v4.is_loopback() || v4.is_link_local() || !v4.is_private() {
        return None;
    }
    let lower = name.to_lowercase();
    if VIRTUAL_ADAPTER_HINTS
        .iter()
        .any(|hint| lower.contains(hint))
    {
        return None;
    }
    Some(LanCandidate {
        ip: v4,
        name: name.to_string(),
    })
}

/// QR en PNG (zona de silencio incluida) listo para mostrar o guardar.
pub fn pairing_qr_png(uri: &str, max_px: u32) -> Result<Vec<u8>, String> {
    use image::{ImageEncoder, Luma};
    let code = qrcode::QrCode::new(uri.as_bytes()).map_err(|e| e.to_string())?;
    let pixels = code
        .render::<Luma<u8>>()
        .max_dimensions(max_px, max_px)
        .build();
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            pixels.as_raw(),
            pixels.width(),
            pixels.height(),
            image::ExtendedColorType::L8,
        )
        .map_err(|e| e.to_string())?;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{progress_pct, snapshot_ok, VendorId};

    const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

    fn sample_payload() -> SyncPayload {
        let snap = snapshot_ok(
            VendorId::Anthropic,
            "Max",
            vec![progress_pct(
                "session", "Sesión", 10.0, None, 18_000, "always",
            )],
        );
        let mut snaps = std::collections::HashMap::new();
        snaps.insert(snap.id.clone(), snap);
        let catalog = vec![crate::model::VendorInfo {
            id: "anthropic".into(),
            name: "Claude Code".into(),
            short: "CLD".into(),
            auth_kind: crate::model::AuthKind::Oauth,
            env_key: None,
            hint: String::new(),
            needs_key: false,
            enabled: true,
            detected: true,
            has_credential: true,
            credential_source: None,
            links: VendorId::Anthropic.links(),
            strategies: vec!["oauth".into()],
            source_preference: None,
        }];
        let snapshot = crate::snapshot_v1::build(&snaps, &catalog, "now".into(), None);
        sync::build_payload("testdev".into(), "now".into(), snapshot)
    }

    fn sample_payload_fn() -> PayloadFn {
        Arc::new(move || Ok(sample_payload()))
    }

    fn free_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn serve_in_background(
        port: u16,
        make_payload: PayloadFn,
        passphrase: &'static str,
    ) -> (std::thread::JoinHandle<()>, Arc<AtomicBool>) {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();
        let cfg = ServeConfig::loopback(port);
        let handle = std::thread::spawn(move || {
            run_server(
                &cfg,
                "test",
                "testdev",
                make_payload,
                passphrase,
                stop_clone,
            )
            .unwrap();
        });
        // Espera a que el puerto acepte.
        for _ in 0..50 {
            if std::net::TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        (handle, stop)
    }

    // Regresión: la GUI (sync_service::ensure_sync_server) solo puede marcar
    // el servidor "activo" DESPUÉS de que `bind` confirme el puerto. Este
    // test prueba la primitiva de la que depende esa garantía: un puerto ya
    // ocupado debe fallar aquí, no en silencio.
    #[test]
    fn bind_falla_con_puerto_ocupado() {
        let port = free_port();
        let _holder = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
        let cfg = ServeConfig::loopback(port);
        assert!(
            bind(&cfg).is_err(),
            "el puerto ya estaba ocupado por _holder"
        );
    }

    #[test]
    fn bind_y_serve_atienden_meta() {
        let port = free_port();
        let cfg = ServeConfig::loopback(port);
        let server = bind(&cfg).expect("puerto libre debería bindear");
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();
        let make_payload = sample_payload_fn();
        let handle = std::thread::spawn(move || {
            serve(
                &server,
                "test",
                "testdev",
                false,
                &make_payload,
                "clave-test",
                &stop_clone,
            )
            .unwrap();
        });
        let (status, _) = http_get(port, "/v1/meta");
        stop.store(true, Ordering::Relaxed);
        let _ = handle.join();
        assert_eq!(status, 200);
    }

    fn http_get(port: u16, path: &str) -> (u16, String) {
        // Cliente mínimo sobre TCP: el core no depende de un cliente HTTP en tests.
        use std::io::{Read, Write};
        let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        write!(stream, "GET {path} HTTP/1.0\r\nHost: x\r\n\r\n").unwrap();
        let mut raw = String::new();
        stream.read_to_string(&mut raw).unwrap();
        let status: u16 = raw
            .lines()
            .next()
            .unwrap_or("")
            .split_whitespace()
            .nth(1)
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
        (status, body)
    }

    #[test]
    fn meta_expone_identidad_sin_secretos() {
        let port = free_port();
        // make_payload nunca se llama en /v1/meta; la passphrase no se usa.
        let (handle, stop) = serve_in_background(port, sample_payload_fn(), "clave-test");
        let (status, body) = http_get(port, "/v1/meta");
        stop.store(true, Ordering::Relaxed);
        let _ = handle.join();
        assert_eq!(status, 200);
        assert!(body.contains("\"schemaVersion\":1"));
        assert!(body.contains("\"deviceId\":\"testdev\""));
        assert!(body.contains("fingerprint"));
        assert!(!body.to_lowercase().contains("passphrase"));
    }

    #[test]
    fn snapshot_sirve_blob_cifrado_descifrable() {
        // Sin keyring: la passphrase entra al arrancar el servidor.
        let port = free_port();
        let (handle, stop) = serve_in_background(port, sample_payload_fn(), "clave-srv-test");
        let (status, body) = http_get(port, "/v1/snapshot");
        stop.store(true, Ordering::Relaxed);
        let _ = handle.join();
        assert_eq!(status, 200, "{body}");
        let blob: EncryptedBlob = serde_json::from_str(&body).unwrap();
        let back = sync::decrypt_blob(&blob, "clave-srv-test").unwrap();
        assert_eq!(back.device_id, "testdev");
        assert_eq!(back.snapshot.providers.len(), 1);
        assert!(sync::decrypt_blob(&blob, "otra").is_err());
    }

    #[test]
    fn ruta_desconocida_404() {
        let port = free_port();
        let (handle, stop) = serve_in_background(port, sample_payload_fn(), "clave-test");
        let (not_found, _) = http_get(port, "/no-existe");
        stop.store(true, Ordering::Relaxed);
        let _ = handle.join();
        assert_eq!(not_found, 404);
    }

    #[test]
    fn pairing_uri_roundtrip_y_fingerprint() {
        let info = PairingInfo::new("192.168.1.20".into(), SYNC_DEFAULT_PORT, "testdev");
        let uri = info.to_uri();
        assert!(uri.starts_with("iausage://pair?v=1&"));
        let back = PairingInfo::parse_uri(&uri).unwrap();
        assert_eq!(back, info);

        let mut evil = info.clone();
        evil.fingerprint = "AAAA-AAAA".into();
        let evil_uri = evil.to_uri();
        assert!(PairingInfo::parse_uri(&evil_uri).is_err());

        assert!(PairingInfo::parse_uri("https://example.com").is_err());
        assert!(PairingInfo::parse_uri("iausage://pair?v=9&host=x").is_err());
    }

    #[test]
    fn qr_png_valido() {
        let info = PairingInfo::new("192.168.1.20".into(), SYNC_DEFAULT_PORT, "testdev");
        let png = pairing_qr_png(&info.to_uri(), 256).unwrap();
        assert!(png.starts_with(PNG_MAGIC));
        assert!(png.len() > 500, "QR sospechosamente pequeño");
    }

    // Regresión: la IP para el QR nunca debe ser loopback ni una interfaz
    // arbitraria elegida solo porque apareció primero al enumerar adaptadores.
    #[test]
    fn lan_ip_no_es_loopback() {
        if let Some(ip) = lan_ip() {
            assert_ne!(ip, "127.0.0.1");
            assert!(ip.parse::<std::net::Ipv4Addr>().is_ok(), "{ip} no es IPv4");
        }
        // `None` es válido en runners sin red (CI aislado): no falla el test.
    }

    fn v4(s: &str) -> std::net::IpAddr {
        std::net::IpAddr::V4(s.parse().unwrap())
    }

    #[test]
    fn loopback_se_descarta() {
        assert!(as_candidate("Loopback", v4("127.0.0.1")).is_none());
    }

    #[test]
    fn apipa_se_descarta() {
        assert!(as_candidate("Wi-Fi", v4("169.254.1.4")).is_none());
    }

    #[test]
    fn ip_publica_se_descarta() {
        assert!(as_candidate("Ethernet", v4("8.8.8.8")).is_none());
    }

    #[test]
    fn adaptadores_virtuales_conocidos_se_descartan() {
        for name in [
            "VirtualBox Host-Only Network",
            "vEthernet (WSL)",
            "Tailscale",
            "Hyper-V Virtual Ethernet Adapter",
            "Docker vEthernet",
        ] {
            assert!(
                as_candidate(name, v4("192.168.56.1")).is_none(),
                "{name} debería descartarse"
            );
        }
    }

    #[test]
    fn wifi_gana_a_ethernet_aunque_ethernet_aparezca_primero() {
        // Reproduce el hardware real: Ethernet 192.168.100.2 y Wi-Fi
        // 192.168.50.116 activos a la vez, Ethernet enumerado primero.
        let candidates = vec![
            as_candidate("Ethernet", v4("192.168.100.2")).unwrap(),
            as_candidate("Wi-Fi", v4("192.168.50.116")).unwrap(),
        ];
        let chosen = candidates.into_iter().min_by_key(|c| c.priority()).unwrap();
        assert_eq!(chosen.ip.to_string(), "192.168.50.116");
    }

    #[test]
    fn seleccion_manual_respeta_una_interfaz_disponible_cuando_no_hay_wifi() {
        let candidates = vec![as_candidate("Ethernet", v4("192.168.100.2")).unwrap()];
        let chosen = candidates.into_iter().min_by_key(|c| c.priority()).unwrap();
        assert_eq!(chosen.ip.to_string(), "192.168.100.2");
    }
}
