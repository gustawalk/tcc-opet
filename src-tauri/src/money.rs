const BASIS_POINTS_SCALE: i128 = 10_000;
pub const JS_MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

pub fn apply_discount(amount_cents: i64, discount_basis_points: i64) -> Option<i64> {
    if !(0..=10_000).contains(&discount_basis_points) {
        return None;
    }

    let numerator =
        i128::from(amount_cents).checked_mul(i128::from(10_000 - discount_basis_points))?;
    let rounded = if numerator >= 0 {
        numerator.checked_add(BASIS_POINTS_SCALE / 2)? / BASIS_POINTS_SCALE
    } else {
        numerator.checked_sub(BASIS_POINTS_SCALE / 2)? / BASIS_POINTS_SCALE
    };
    i64::try_from(rounded).ok()
}

pub fn checked_div_round_nearest(numerator: i64, denominator: i64) -> Option<i64> {
    if denominator == 0 {
        return None;
    }
    let numerator = i128::from(numerator);
    let denominator = i128::from(denominator);
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let rounds_away = remainder.abs().checked_mul(2)? >= denominator.abs();
    let rounded = if rounds_away {
        quotient.checked_add(if numerator.signum() == denominator.signum() {
            1
        } else {
            -1
        })?
    } else {
        quotient
    };
    i64::try_from(rounded).ok()
}

pub fn is_js_safe_integer(value: i64) -> bool {
    (-JS_MAX_SAFE_INTEGER..=JS_MAX_SAFE_INTEGER).contains(&value)
}

pub fn format_brl(cents: i64) -> String {
    let value = i128::from(cents);
    let absolute = value.abs();
    let integer = absolute / 100;
    let decimal = absolute % 100;
    let digits = integer.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(character);
    }
    let sign = if value < 0 { "-" } else { "" };
    format!(
        "{sign}R$ {},{decimal:02}",
        grouped.chars().rev().collect::<String>()
    )
}

pub fn format_csv(cents: i64) -> String {
    let value = i128::from(cents);
    let sign = if value < 0 { "-" } else { "" };
    let absolute = value.abs();
    format!("{sign}{}.{:02}", absolute / 100, absolute % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discount_rounds_to_nearest_cent() {
        assert_eq!(apply_discount(101, 5_000), Some(51));
        assert_eq!(apply_discount(100, 3_333), Some(67));
    }

    #[test]
    fn discount_uses_i128_for_large_values() {
        assert_eq!(apply_discount(i64::MAX, 0), Some(i64::MAX));
        assert_eq!(apply_discount(i64::MAX, 10_000), Some(0));
    }

    #[test]
    fn discount_rejects_invalid_basis_points() {
        assert_eq!(apply_discount(100, -1), None);
        assert_eq!(apply_discount(100, 10_001), None);
    }

    #[test]
    fn division_rounds_to_nearest_cent_away_on_ties() {
        assert_eq!(checked_div_round_nearest(101, 2), Some(51));
        assert_eq!(checked_div_round_nearest(-101, 2), Some(-51));
        assert_eq!(checked_div_round_nearest(100, 3), Some(33));
        assert_eq!(checked_div_round_nearest(1, 0), None);
    }

    #[test]
    fn identifies_javascript_safe_integers() {
        assert!(is_js_safe_integer(JS_MAX_SAFE_INTEGER));
        assert!(is_js_safe_integer(-JS_MAX_SAFE_INTEGER));
        assert!(!is_js_safe_integer(JS_MAX_SAFE_INTEGER + 1));
    }

    #[test]
    fn formats_brl_and_csv_exactly() {
        assert_eq!(format_brl(123_456), "R$ 1.234,56");
        assert_eq!(format_brl(-5), "-R$ 0,05");
        assert_eq!(format_csv(123_456), "1234.56");
        assert_eq!(format_csv(-5), "-0.05");
    }
}
