//! Salud del servicio del proveedor, separada de la conexión.
//!
//! `ProviderStatus` describe **nuestra conexión con la fuente**
//! (Connected/NeedsAuth/...). `ServiceHealth` describe **el estado del
//! proveedor en el mundo** (Operational/Degraded/Outage). No mezclar ambos:
//! la UI los muestra en dos filas distintas.
//!
//! v0.3: modelo + enlace verificado. El polling en vivo de Statuspage/feeds
//! de incidentes llega en 0.3.x; hasta entonces todo es `Unknown` salvo que
//! el llamante aporte un estado observado.

use crate::model::{ServiceHealth, VendorId};

/// URL oficial de estado, solo si está verificada en `VendorId::links`.
/// Nada adivinado: `None` cuando no hay URL conocida.
pub fn status_url(id: VendorId) -> Option<String> {
    id.links().status_url
}

/// ¿Tiene sentido mostrar fila de servicio para este proveedor?
pub fn has_service_signal(id: VendorId) -> bool {
    status_url(id).is_some()
}

pub fn label(health: ServiceHealth) -> &'static str {
    match health {
        ServiceHealth::Operational => "operational",
        ServiceHealth::Degraded => "degraded",
        ServiceHealth::Outage => "outage",
        ServiceHealth::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vendors_expose_status_pages() {
        assert!(has_service_signal(VendorId::Anthropic));
        assert!(has_service_signal(VendorId::Openai));
        assert!(has_service_signal(VendorId::Copilot));
    }

    #[test]
    fn unlisted_vendors_have_no_guessed_status_url() {
        assert!(!has_service_signal(VendorId::Kiro));
        assert!(!has_service_signal(VendorId::Nous));
        assert!(status_url(VendorId::Kiro).is_none());
    }
}
