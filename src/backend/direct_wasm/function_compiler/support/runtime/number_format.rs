fn finite_number_to_js_string(value: f64) -> String {
    let magnitude = value.abs();
    if magnitude >= 1e21 || magnitude < 1e-6 {
        let exponential = format!("{value:e}");
        let Some((mantissa, exponent)) = exponential.split_once('e') else {
            return exponential;
        };
        let (sign, digits) = match exponent.strip_prefix('-') {
            Some(digits) => ("-", digits),
            None => ("+", exponent.strip_prefix('+').unwrap_or(exponent)),
        };
        let digits = digits.trim_start_matches('0');
        return format!(
            "{mantissa}e{sign}{}",
            if digits.is_empty() { "0" } else { digits }
        );
    }

    value.to_string()
}

pub(in crate::backend::direct_wasm) fn js_number_to_string(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "Infinity".to_string()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else if value == 0.0 {
        "0".to_string()
    } else if value.is_finite() && value.fract() == 0.0 && value.abs() < 1e21 {
        (value as i64).to_string()
    } else {
        finite_number_to_js_string(value)
    }
}

pub(in crate::backend::direct_wasm) fn js_console_number_to_string(value: f64) -> String {
    if value == 0.0 && value.is_sign_negative() {
        "-0".to_string()
    } else {
        js_number_to_string(value)
    }
}
