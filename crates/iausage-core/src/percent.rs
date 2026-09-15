//! Contrato de porcentajes de resumen: used y remaining siempre suman 100.

/// `used = round(clamp(used_exact, 0, 100)); remaining = 100 - used`.
pub fn summary_percents(used_exact: f64) -> (i32, i32) {
    let used = if !used_exact.is_finite() {
        0
    } else {
        used_exact.clamp(0.0, 100.0).round() as i32
    };
    (used, 100 - used)
}

#[cfg(test)]
mod tests {
    use super::summary_percents;

    #[test]
    fn rounding_vectors() {
        let cases = [
            (5.48, 5, 95),
            (5.50, 6, 94),
            (14.88, 15, 85),
            (38.2711, 38, 62),
            (99.6, 100, 0),
            (100.0, 100, 0),
            (-1.0, 0, 100),
        ];
        for (exact, used, remaining) in cases {
            assert_eq!(
                summary_percents(exact),
                (used, remaining),
                "usedExact={exact}"
            );
        }
    }
}
