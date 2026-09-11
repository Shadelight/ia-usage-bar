//! Política de refresh adaptativo: pura, determinista y testeable.
//!
//! El scheduler (`src-tauri`) solo pregunta `next_interval_secs`; toda la
//! decisión vive aquí. Sin escaneo de procesos en v0.3 (llega después, si
//! llega): la actividad se mide por interacción con la ventana.

/// Minutos manuales permitidos en Ajustes.
pub const MANUAL_MINUTES: &[u64] = &[1, 2, 5, 15, 30];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityClass {
    /// Ventana abierta o usada hace <10 min.
    Foreground,
    /// Uso en los últimos 60 min.
    Recent,
    /// Inactiva 1–4 h.
    IdleShort,
    /// Inactiva >4 h.
    IdleLong,
}

pub fn classify(secs_since_activity: u64) -> ActivityClass {
    if secs_since_activity < 10 * 60 {
        ActivityClass::Foreground
    } else if secs_since_activity < 60 * 60 {
        ActivityClass::Recent
    } else if secs_since_activity < 4 * 60 * 60 {
        ActivityClass::IdleShort
    } else {
        ActivityClass::IdleLong
    }
}

pub struct PolicyInput {
    pub adaptive: bool,
    /// `None` = modo Manual con minutos fijos.
    pub manual_minutes: u64,
    pub secs_since_activity: u64,
    pub battery_saver: bool,
}

/// Segundos hasta el próximo refresh.
pub fn next_interval_secs(input: &PolicyInput) -> u64 {
    if !input.adaptive {
        return clamp_manual(input.manual_minutes) * 60;
    }
    if input.battery_saver {
        return 30 * 60;
    }
    let mins = match classify(input.secs_since_activity) {
        ActivityClass::Foreground => 2,
        ActivityClass::Recent => 5,
        ActivityClass::IdleShort => 15,
        ActivityClass::IdleLong => 30,
    };
    mins * 60
}

fn clamp_manual(mins: u64) -> u64 {
    if MANUAL_MINUTES.contains(&mins) {
        return mins;
    }
    if mins < 3 {
        1
    } else if mins < 8 {
        5
    } else if mins < 20 {
        15
    } else {
        30
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adaptive(secs: u64, saver: bool) -> u64 {
        next_interval_secs(&PolicyInput {
            adaptive: true,
            manual_minutes: 5,
            secs_since_activity: secs,
            battery_saver: saver,
        })
    }

    #[test]
    fn adaptive_tiers_match_spec() {
        assert_eq!(adaptive(0, false), 2 * 60);
        assert_eq!(adaptive(30 * 60, false), 5 * 60);
        assert_eq!(adaptive(2 * 3_600, false), 15 * 60);
        assert_eq!(adaptive(5 * 3_600, false), 30 * 60);
    }

    #[test]
    fn battery_saver_forces_slowest_tier() {
        assert_eq!(adaptive(0, true), 30 * 60);
    }

    #[test]
    fn manual_mode_keeps_fixed_interval() {
        let mins = next_interval_secs(&PolicyInput {
            adaptive: false,
            manual_minutes: 15,
            secs_since_activity: 0,
            battery_saver: false,
        });
        assert_eq!(mins, 15 * 60);
    }

    #[test]
    fn manual_clamps_to_known_steps() {
        assert_eq!(clamp_manual(7), 5);
        assert_eq!(clamp_manual(0), 1);
        assert_eq!(clamp_manual(99), 30);
    }
}
