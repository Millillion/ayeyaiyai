use super::super::*;
use super::{
    bindings::{
        collect_pattern_binding_names, collect_using_decl_bound_names, collect_var_decl_bound_names,
    },
    declarations::{
        BindingRestrictions, is_await_like_identifier, is_yield_like_identifier,
        validate_escaped_identifier_text, validate_pattern_syntax_with_restrictions,
    },
    functions::{
        ensure_parameter_names_are_valid, validate_class_syntax_with_restrictions,
        validate_function_syntax_with_restrictions,
        validate_property_name_syntax_with_restrictions,
    },
    statements::validate_statement_syntax_with_restrictions,
};
use std::collections::BTreeSet;
use swc_common::Spanned;

fn validate_digit_sequence(
    digits: &str,
    raw: &str,
    valid_digit: impl Fn(u8) -> bool,
) -> Result<()> {
    ensure!(!digits.is_empty(), "invalid numeric literal `{raw}`");

    let mut saw_digit = false;
    let mut previous_was_separator = false;

    for byte in digits.bytes() {
        if byte == b'_' {
            ensure!(
                saw_digit && !previous_was_separator,
                "invalid numeric literal `{raw}`"
            );
            previous_was_separator = true;
            continue;
        }

        ensure!(valid_digit(byte), "invalid numeric literal `{raw}`");
        saw_digit = true;
        previous_was_separator = false;
    }

    ensure!(
        saw_digit && !previous_was_separator,
        "invalid numeric literal `{raw}`"
    );
    Ok(())
}

fn validate_based_integer_literal(
    digits: &str,
    raw: &str,
    valid_digit: impl Fn(u8) -> bool,
) -> Result<()> {
    validate_digit_sequence(digits, raw, valid_digit)
}

fn validate_decimal_integer_digits(digits: &str, raw: &str) -> Result<()> {
    validate_digit_sequence(digits, raw, |byte| byte.is_ascii_digit())?;
    if digits.contains('_') && digits.starts_with('0') && digits.len() > 1 {
        bail!("invalid numeric literal `{raw}`");
    }
    Ok(())
}

fn validate_decimal_literal(raw: &str) -> Result<()> {
    let (mantissa, exponent) = match raw.find(['e', 'E']) {
        Some(index) => (&raw[..index], Some(&raw[index + 1..])),
        None => (raw, None),
    };

    if let Some(dot_index) = mantissa.find('.') {
        let integer = &mantissa[..dot_index];
        let fraction = &mantissa[dot_index + 1..];
        ensure!(
            !integer.is_empty() || !fraction.is_empty(),
            "invalid numeric literal `{raw}`"
        );
        if !integer.is_empty() {
            validate_decimal_integer_digits(integer, raw)?;
        }
        if !fraction.is_empty() {
            validate_digit_sequence(fraction, raw, |byte| byte.is_ascii_digit())?;
        }
    } else {
        validate_decimal_integer_digits(mantissa, raw)?;
    }

    if let Some(exponent_digits) = exponent {
        let exponent_digits = exponent_digits
            .strip_prefix('+')
            .or_else(|| exponent_digits.strip_prefix('-'))
            .unwrap_or(exponent_digits);
        validate_digit_sequence(exponent_digits, raw, |byte| byte.is_ascii_digit())?;
    }

    Ok(())
}

fn validate_number_literal_syntax(number: &Number, file: &swc_common::SourceFile) -> Result<()> {
    let raw = number.raw.as_deref().map(str::to_owned).unwrap_or_else(|| {
        source_slice_for_span(file, number.span)
            .unwrap_or("")
            .to_string()
    });
    if !raw.contains('_') {
        return Ok(());
    }

    let normalized = raw.strip_suffix('n').unwrap_or(&raw);
    if let Some(digits) = normalized
        .strip_prefix("0b")
        .or_else(|| normalized.strip_prefix("0B"))
    {
        return validate_based_integer_literal(digits, &raw, |byte| matches!(byte, b'0' | b'1'));
    }
    if let Some(digits) = normalized
        .strip_prefix("0o")
        .or_else(|| normalized.strip_prefix("0O"))
    {
        return validate_based_integer_literal(digits, &raw, |byte| (b'0'..=b'7').contains(&byte));
    }
    if let Some(digits) = normalized
        .strip_prefix("0x")
        .or_else(|| normalized.strip_prefix("0X"))
    {
        return validate_based_integer_literal(digits, &raw, |byte| byte.is_ascii_hexdigit());
    }

    validate_decimal_literal(normalized)
}

fn ascii_hex_digit_value(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'a'..=b'f' => Some(u32::from(byte - b'a') + 10),
        b'A'..=b'F' => Some(u32::from(byte - b'A') + 10),
        _ => None,
    }
}

fn validate_string_literal_unicode_code_point_escapes(raw: &str) -> Result<()> {
    let bytes = raw.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\\' {
            index += 1;
            continue;
        }

        index += 1;
        if index >= bytes.len() {
            return Ok(());
        }

        match bytes[index] {
            b'u' if bytes.get(index + 1) == Some(&b'{') => {
                let mut cursor = index + 2;
                let mut value = 0u32;
                let mut digit_count = 0usize;
                while cursor < bytes.len() && bytes[cursor] != b'}' {
                    let Some(digit) = ascii_hex_digit_value(bytes[cursor]) else {
                        bail!("invalid unicode code point escape in string literal");
                    };
                    value = value.saturating_mul(16).saturating_add(digit);
                    digit_count += 1;
                    cursor += 1;
                }
                ensure!(
                    cursor < bytes.len() && digit_count > 0 && value <= 0x10ffff,
                    "invalid unicode code point escape in string literal"
                );
                index = cursor + 1;
            }
            b'\r' => {
                index += if bytes.get(index + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                };
            }
            _ => index += 1,
        }
    }
    Ok(())
}

fn validate_string_literal_syntax(string: &Str) -> Result<()> {
    if let Some(raw) = string.raw.as_ref() {
        validate_string_literal_unicode_code_point_escapes(raw.as_ref())?;
    }
    Ok(())
}

fn regex_literal_pattern_source(raw: &str) -> Option<&str> {
    if !raw.starts_with('/') {
        return None;
    }

    let mut escaped = false;
    let mut in_class = false;
    for (offset, ch) in raw[1..].char_indices() {
        let index = 1 + offset;
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '[' if !in_class => in_class = true,
            ']' if in_class => in_class = false,
            '/' if !in_class => return raw.get(1..index),
            _ => {}
        }
    }
    None
}

fn validate_regex_modifier_flags(flags: &str) -> Result<Vec<char>> {
    let mut seen = HashSet::new();
    let mut parsed = Vec::new();
    for ch in flags.chars() {
        ensure!(
            matches!(ch, 'i' | 'm' | 's'),
            "invalid regular expression modifier flag"
        );
        ensure!(
            seen.insert(ch),
            "duplicate regular expression modifier flag"
        );
        parsed.push(ch);
    }
    Ok(parsed)
}

fn validate_regex_modifier_group(pattern: &str, question_offset: usize) -> Result<()> {
    let head_start = question_offset + '?'.len_utf8();
    let Some(head) = pattern.get(head_start..) else {
        return Ok(());
    };
    if head.starts_with([':', '=', '!', '<']) {
        return Ok(());
    }

    let mut terminator = None;
    for (offset, ch) in head.char_indices() {
        if matches!(ch, ':' | ')') {
            terminator = Some((head_start + offset, ch));
            break;
        }
    }
    let Some((terminator_offset, terminator_ch)) = terminator else {
        return Ok(());
    };

    let modifier_head = &pattern[head_start..terminator_offset];
    ensure!(
        terminator_ch == ':',
        "regular expression modifier group requires ':'"
    );

    let (enabled, disabled, arithmetic) =
        if let Some((enabled, disabled)) = modifier_head.split_once('-') {
            (enabled, disabled, true)
        } else {
            (modifier_head, "", false)
        };
    ensure!(
        !arithmetic || !enabled.is_empty() || !disabled.is_empty(),
        "regular expression arithmetic modifier cannot be empty"
    );

    let enabled_flags = validate_regex_modifier_flags(enabled)?;
    let disabled_flags = validate_regex_modifier_flags(disabled)?;
    ensure!(
        !enabled_flags
            .iter()
            .any(|flag| disabled_flags.contains(flag)),
        "regular expression modifier flag cannot be both enabled and disabled"
    );
    Ok(())
}

fn validate_regex_modifier_syntax(pattern: &str) -> Result<()> {
    let mut escaped = false;
    let mut in_class = false;
    let mut chars = pattern.char_indices().peekable();
    while let Some((offset, ch)) = chars.next() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '[' if !in_class => in_class = true,
            ']' if in_class => in_class = false,
            '(' if !in_class => {
                if let Some((question_offset, '?')) = chars.peek().copied() {
                    validate_regex_modifier_group(pattern, question_offset)?;
                }
            }
            _ => {
                let _ = offset;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum RegexGroupKind {
    Capturing,
    QuantifiableAssertion,
    LookbehindAssertion,
}

impl RegexGroupKind {
    fn can_be_quantified_after_close(self, unicode_mode: bool) -> bool {
        match self {
            Self::Capturing => true,
            Self::QuantifiableAssertion => !unicode_mode,
            Self::LookbehindAssertion => false,
        }
    }
}

fn regex_group_prefix(pattern: &str, open_offset: usize) -> (RegexGroupKind, usize) {
    let Some(group_source) = pattern.get(open_offset..) else {
        return (RegexGroupKind::Capturing, '('.len_utf8());
    };
    if group_source.starts_with("(?<=") || group_source.starts_with("(?<!") {
        return (RegexGroupKind::LookbehindAssertion, "(?<=".len());
    }
    if group_source.starts_with("(?=") || group_source.starts_with("(?!") {
        return (RegexGroupKind::QuantifiableAssertion, "(?=".len());
    }
    if group_source.starts_with("(?:") {
        return (RegexGroupKind::Capturing, "(?:".len());
    }
    if group_source.starts_with("(?<") {
        return (RegexGroupKind::Capturing, "(?<".len());
    }
    if let Some(head) = group_source.strip_prefix("(?") {
        for (offset, ch) in head.char_indices() {
            if ch == ':' {
                return (
                    RegexGroupKind::Capturing,
                    "(?".len() + offset + ch.len_utf8(),
                );
            }
            if ch == ')' {
                break;
            }
        }
    }
    (RegexGroupKind::Capturing, '('.len_utf8())
}

fn regex_braced_quantifier_len(pattern: &str, open_offset: usize) -> Option<usize> {
    let bytes = pattern.as_bytes();
    if bytes.get(open_offset) != Some(&b'{') {
        return None;
    }

    let mut index = open_offset + 1;
    let minimum_start = index;
    while matches!(bytes.get(index), Some(byte) if byte.is_ascii_digit()) {
        index += 1;
    }
    if index == minimum_start {
        return None;
    }

    if bytes.get(index) == Some(&b'}') {
        return Some(index + 1 - open_offset);
    }
    if bytes.get(index) != Some(&b',') {
        return None;
    }
    index += 1;
    while matches!(bytes.get(index), Some(byte) if byte.is_ascii_digit()) {
        index += 1;
    }
    if bytes.get(index) == Some(&b'}') {
        return Some(index + 1 - open_offset);
    }
    None
}

fn regex_quantifier_len(pattern: &str, offset: usize, ch: char) -> Option<usize> {
    let base_len = match ch {
        '*' | '+' | '?' => ch.len_utf8(),
        '{' => regex_braced_quantifier_len(pattern, offset)?,
        _ => return None,
    };
    let lazy_len = pattern
        .get(offset + base_len..)
        .and_then(|tail| tail.chars().next())
        .filter(|next| *next == '?')
        .map_or(0, char::len_utf8);
    Some(base_len + lazy_len)
}

fn is_regex_syntax_character(character: char) -> bool {
    matches!(
        character,
        '^' | '$' | '\\' | '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
    )
}

fn regex_fixed_hex_escape_end(
    pattern: &str,
    digits_start: usize,
    digits_len: usize,
) -> Result<usize> {
    let digits_end = digits_start + digits_len;
    let Some(digits) = pattern.get(digits_start..digits_end) else {
        bail!("regular expression escape is incomplete");
    };
    ensure!(
        digits.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "regular expression escape has invalid hexadecimal digits"
    );
    Ok(digits_end)
}

fn regex_braced_unicode_escape_end(pattern: &str, digits_start: usize) -> Result<usize> {
    let Some(tail) = pattern.get(digits_start..) else {
        bail!("regular expression unicode escape is incomplete");
    };
    let Some(relative_end) = tail.find('}') else {
        bail!("regular expression unicode escape is incomplete");
    };
    let digits = &tail[..relative_end];
    ensure!(
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "regular expression unicode escape has invalid hexadecimal digits"
    );
    let value =
        u32::from_str_radix(digits, 16).context("unicode escape digits should be hexadecimal")?;
    ensure!(
        value <= 0x10ffff,
        "regular expression unicode escape is out of range"
    );
    Ok(digits_start + relative_end + '}'.len_utf8())
}

fn validate_regex_unicode_escape(
    pattern: &str,
    escape_offset: usize,
    in_class: bool,
) -> Result<(usize, Option<u32>, bool)> {
    let escaped_start = escape_offset + '\\'.len_utf8();
    let Some((relative_offset, escaped)) = pattern
        .get(escaped_start..)
        .and_then(|tail| tail.char_indices().next())
    else {
        bail!("regular expression escape is incomplete");
    };
    let escaped_offset = escaped_start + relative_offset;
    let escaped_end = escaped_offset + escaped.len_utf8();

    match escaped {
        'd' | 'D' | 's' | 'S' | 'w' | 'W' => Ok((escaped_end, None, false)),
        'b' | 'B' | 'f' | 'n' | 'r' | 't' | 'v' => Ok((escaped_end, None, true)),
        '/' => Ok((escaped_end, None, true)),
        '-' if in_class => Ok((escaped_end, None, true)),
        'c' => {
            let Some(control) = pattern
                .get(escaped_end..)
                .and_then(|tail| tail.chars().next())
            else {
                bail!("regular expression control escape is incomplete");
            };
            ensure!(
                control.is_ascii_alphabetic(),
                "regular expression control escape must use an ASCII letter"
            );
            Ok((escaped_end + control.len_utf8(), None, true))
        }
        '0' => {
            ensure!(
                !matches!(
                    pattern.get(escaped_end..).and_then(|tail| tail.chars().next()),
                    Some(next) if next.is_ascii_digit()
                ),
                "regular expression legacy octal escape is not allowed in unicode mode"
            );
            Ok((escaped_end, None, true))
        }
        'x' => regex_fixed_hex_escape_end(pattern, escaped_end, 2).map(|end| (end, None, true)),
        'u' if pattern
            .get(escaped_end..)
            .is_some_and(|tail| tail.starts_with('{')) =>
        {
            regex_braced_unicode_escape_end(pattern, escaped_end + '{'.len_utf8())
                .map(|end| (end, None, true))
        }
        'u' => regex_fixed_hex_escape_end(pattern, escaped_end, 4).map(|end| (end, None, true)),
        'p' | 'P'
            if pattern
                .get(escaped_end..)
                .is_some_and(|tail| tail.starts_with('{')) =>
        {
            let Some(relative_end) = pattern[escaped_end + '{'.len_utf8()..].find('}') else {
                bail!("regular expression property escape is incomplete");
            };
            Ok((
                escaped_end + '{'.len_utf8() + relative_end + '}'.len_utf8(),
                None,
                false,
            ))
        }
        decimal if decimal.is_ascii_digit() => {
            ensure!(
                !in_class,
                "regular expression decimal escape is not allowed in unicode character class"
            );
            let mut decimal_end = escaped_end;
            let mut value = decimal
                .to_digit(10)
                .expect("ascii digit escape should parse as decimal");
            while let Some((_, next)) = pattern
                .get(decimal_end..)
                .and_then(|tail| tail.char_indices().next())
                .filter(|(_, next)| next.is_ascii_digit())
            {
                value = value * 10
                    + next
                        .to_digit(10)
                        .expect("ascii digit escape should parse as decimal");
                decimal_end += next.len_utf8();
            }
            Ok((decimal_end, Some(value), true))
        }
        syntax if is_regex_syntax_character(syntax) => Ok((escaped_end, None, true)),
        _ => bail!("regular expression identity escape is not allowed in unicode mode"),
    }
}

fn regex_class_range_right_atom(
    pattern: &str,
    atom_offset: usize,
    unicode_mode: bool,
) -> Result<Option<(usize, bool)>> {
    let Some((_, atom)) = pattern
        .get(atom_offset..)
        .and_then(|tail| tail.char_indices().next())
    else {
        return Ok(None);
    };
    if atom == ']' {
        return Ok(None);
    }
    if atom == '\\' && unicode_mode {
        let (atom_end, _, single_character) =
            validate_regex_unicode_escape(pattern, atom_offset, true)?;
        return Ok(Some((atom_end, single_character)));
    }
    Ok(Some((atom_offset + atom.len_utf8(), true)))
}

fn is_regex_identifier_continue(character: char) -> bool {
    matches!(character, '\u{200C}' | '\u{200D}') || Ident::is_valid_continue(character)
}

fn validate_regex_group_name(name: &str) -> Result<()> {
    ensure!(!name.is_empty(), "regular expression group name is empty");
    if name.contains('\\') {
        validate_escaped_identifier_text(name)?;
        return Ok(());
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        bail!("regular expression group name is empty");
    };
    ensure!(
        Ident::is_valid_start(first),
        "regular expression group name is not identifier-like"
    );
    ensure!(
        chars.all(is_regex_identifier_continue),
        "regular expression group name is not identifier-like"
    );
    Ok(())
}

fn regex_group_name(pattern: &str, name_start: usize) -> Result<(String, usize)> {
    let Some(tail) = pattern.get(name_start..) else {
        bail!("regular expression group name is incomplete");
    };
    let Some(relative_end) = tail.find('>') else {
        bail!("regular expression group name is incomplete");
    };
    let name = &tail[..relative_end];
    validate_regex_group_name(name)?;
    Ok((name.to_string(), name_start + relative_end + '>'.len_utf8()))
}

fn validate_regex_pattern_syntax(pattern: &str, unicode_mode: bool) -> Result<()> {
    let mut escaped = false;
    let mut in_class = false;
    let mut can_quantify = false;
    let mut groups = Vec::new();
    let mut named_groups = BTreeSet::new();
    let mut named_references = Vec::new();
    let mut capture_count = 0;
    let mut numeric_references = Vec::new();
    let mut bare_k_escape = false;
    let mut last_class_atom_single = None;
    let mut chars = pattern.char_indices().peekable();

    while let Some((offset, ch)) = chars.next() {
        if escaped {
            if !in_class && ch == 'k' {
                bare_k_escape = true;
            }
            escaped = false;
            if !in_class {
                can_quantify = true;
            }
            continue;
        }
        if in_class {
            match ch {
                '\\' if unicode_mode => {
                    let (escape_end, _, single_character) =
                        validate_regex_unicode_escape(pattern, offset, true)?;
                    while matches!(chars.peek(), Some((next_offset, _)) if *next_offset < escape_end)
                    {
                        chars.next();
                    }
                    last_class_atom_single = Some(single_character);
                }
                '\\' => escaped = true,
                ']' => {
                    in_class = false;
                    last_class_atom_single = None;
                    can_quantify = true;
                }
                '-' if unicode_mode => {
                    let dash_end = offset + ch.len_utf8();
                    if let Some(left_single_character) = last_class_atom_single
                        && let Some((right_atom_end, right_single_character)) =
                            regex_class_range_right_atom(pattern, dash_end, unicode_mode)?
                    {
                        ensure!(
                            left_single_character && right_single_character,
                            "regular expression character class range endpoint is not a single character"
                        );
                        while matches!(chars.peek(), Some((next_offset, _)) if *next_offset < right_atom_end)
                        {
                            chars.next();
                        }
                    }
                    last_class_atom_single = Some(true);
                }
                _ => last_class_atom_single = Some(true),
            }
            continue;
        }

        if let Some(quantifier_len) = regex_quantifier_len(pattern, offset, ch) {
            ensure!(
                can_quantify,
                "regular expression quantifier has no preceding atom"
            );
            let quantifier_end = offset + quantifier_len;
            while matches!(chars.peek(), Some((next_offset, _)) if *next_offset < quantifier_end) {
                chars.next();
            }
            can_quantify = false;
            continue;
        }
        if unicode_mode && matches!(ch, '{' | '}') {
            bail!("regular expression extended pattern character is not allowed in unicode mode");
        }

        match ch {
            '\\' => {
                if let Some(name_start) = pattern
                    .get(offset..)
                    .and_then(|tail| tail.strip_prefix("\\k<"))
                    .map(|_| offset + "\\k<".len())
                {
                    let (name, group_name_end) = regex_group_name(pattern, name_start)?;
                    named_references.push(name);
                    while matches!(chars.peek(), Some((next_offset, _)) if *next_offset < group_name_end)
                    {
                        chars.next();
                    }
                    can_quantify = true;
                } else if unicode_mode {
                    let (escape_end, numeric_reference, _) =
                        validate_regex_unicode_escape(pattern, offset, false)?;
                    if let Some(numeric_reference) = numeric_reference {
                        numeric_references.push(numeric_reference);
                    }
                    while matches!(chars.peek(), Some((next_offset, _)) if *next_offset < escape_end)
                    {
                        chars.next();
                    }
                    can_quantify = true;
                } else {
                    escaped = true;
                }
            }
            '[' => {
                in_class = true;
                last_class_atom_single = None;
                can_quantify = false;
            }
            '(' => {
                if pattern.get(offset..).is_some_and(|tail| {
                    tail.starts_with("(?<")
                        && !tail.starts_with("(?<=")
                        && !tail.starts_with("(?<!")
                }) {
                    let (name, group_name_end) = regex_group_name(pattern, offset + "(?<".len())?;
                    ensure!(
                        named_groups.insert(name),
                        "regular expression duplicate group name"
                    );
                    while matches!(chars.peek(), Some((next_offset, _)) if *next_offset < group_name_end)
                    {
                        chars.next();
                    }
                    groups.push(RegexGroupKind::Capturing);
                    capture_count += 1;
                    can_quantify = false;
                    continue;
                }
                let (kind, prefix_len) = regex_group_prefix(pattern, offset);
                if pattern
                    .get(offset..)
                    .is_some_and(|tail| !tail.starts_with("(?"))
                {
                    capture_count += 1;
                }
                let prefix_end = offset + prefix_len;
                while matches!(chars.peek(), Some((next_offset, _)) if *next_offset < prefix_end) {
                    chars.next();
                }
                groups.push(kind);
                can_quantify = false;
            }
            ')' => {
                can_quantify = groups.pop().map_or(true, |kind| {
                    kind.can_be_quantified_after_close(unicode_mode)
                });
            }
            '|' | '^' | '$' => can_quantify = false,
            _ => can_quantify = true,
        }
    }
    ensure!(
        !(bare_k_escape && (unicode_mode || !named_groups.is_empty())),
        "regular expression named backreference is incomplete"
    );
    for reference in named_references {
        ensure!(
            named_groups.contains(&reference),
            "regular expression named backreference has no matching group"
        );
    }
    for reference in numeric_references {
        ensure!(
            reference > 0 && reference <= capture_count,
            "regular expression decimal escape has no matching capture"
        );
    }
    Ok(())
}

fn validate_regex_literal_syntax(regex: &Regex, file: &swc_common::SourceFile) -> Result<()> {
    let raw = source_slice_for_span(file, regex.span)?;
    ensure!(
        !raw.contains(['\u{2028}', '\u{2029}']),
        "regular expression literals cannot contain line terminators"
    );
    if let Some(pattern) = regex_literal_pattern_source(raw) {
        validate_regex_modifier_syntax(pattern)?;
        let flags = regex.flags.to_string();
        validate_regex_pattern_syntax(pattern, flags.contains('u') || flags.contains('v'))?;
    }
    Ok(())
}

pub(crate) fn validate_expression_syntax(
    expression: &Expr,
    file: &swc_common::SourceFile,
) -> Result<()> {
    validate_expression_syntax_with_restrictions(expression, file, BindingRestrictions::default())
}

fn static_object_literal_property_name(name: &PropName) -> Option<&str> {
    match name {
        PropName::Ident(identifier) => Some(identifier.sym.as_ref()),
        PropName::Str(string) => string.value.as_str(),
        _ => None,
    }
}

fn validate_object_literal_duplicate_proto_setters(object: &ObjectLit) -> Result<()> {
    let mut saw_proto_setter = false;
    for property in &object.props {
        let PropOrSpread::Prop(property) = property else {
            continue;
        };
        let Prop::KeyValue(property) = &**property else {
            continue;
        };
        if static_object_literal_property_name(&property.key) != Some("__proto__") {
            continue;
        }
        ensure!(
            !saw_proto_setter,
            "duplicate __proto__ property in object literal"
        );
        saw_proto_setter = true;
    }
    Ok(())
}

fn validate_async_object_method_no_line_terminator(
    method: &MethodProp,
    file: &swc_common::SourceFile,
) -> Result<()> {
    if !method.function.is_async {
        return Ok(());
    }

    let method_start = method.function.span.lo;
    let key_start = method.key.span().lo();
    if method_start >= key_start {
        return Ok(());
    }

    let prefix = source_slice_for_span(file, swc_common::Span::new(method_start, key_start))?;
    ensure!(
        !prefix.contains(['\n', '\r', '\u{2028}', '\u{2029}']),
        "async object methods cannot contain a line terminator between `async` and the property name"
    );

    Ok(())
}

fn validate_object_accessor_contextual_keyword(
    accessor_start: swc_common::BytePos,
    key_start: swc_common::BytePos,
    keyword: &str,
    file: &swc_common::SourceFile,
) -> Result<()> {
    if accessor_start >= key_start {
        return Ok(());
    }

    let prefix = source_slice_for_span(file, swc_common::Span::new(accessor_start, key_start))?;
    let token = prefix.trim_start();
    ensure!(
        token.starts_with(keyword),
        "object accessor keyword `{keyword}` cannot contain escape sequences"
    );

    Ok(())
}

fn validate_object_getter_contextual_keyword(
    getter: &GetterProp,
    file: &swc_common::SourceFile,
) -> Result<()> {
    validate_object_accessor_contextual_keyword(getter.span.lo, getter.key.span().lo(), "get", file)
}

fn validate_object_setter_contextual_keyword(
    setter: &SetterProp,
    file: &swc_common::SourceFile,
) -> Result<()> {
    validate_object_accessor_contextual_keyword(setter.span.lo, setter.key.span().lo(), "set", file)
}

fn validate_assignment_identifier_reference_syntax(
    identifier: &Ident,
    file: &swc_common::SourceFile,
    restrictions: BindingRestrictions,
) -> Result<()> {
    validate_identifier_reference_syntax(identifier, file, restrictions)
}

fn validate_identifier_reference_syntax(
    identifier: &Ident,
    file: &swc_common::SourceFile,
    restrictions: BindingRestrictions,
) -> Result<()> {
    let raw = source_slice_for_span(file, identifier.span)?;
    if raw.contains('\\') {
        validate_escaped_identifier_text(raw)?;
    }
    ensure!(
        !identifier.is_reserved(),
        "reserved word `{}` cannot be used as an identifier reference",
        identifier.sym
    );
    ensure!(
        !(restrictions.reserves_await_identifier()
            && is_await_like_identifier(identifier.sym.as_ref())),
        "`await` cannot be used as an identifier in this context"
    );
    ensure!(
        !(restrictions.yield_reserved && is_yield_like_identifier(identifier.sym.as_ref())),
        "`yield` cannot be used as an identifier in a generator function"
    );
    Ok(())
}

fn validate_assignment_pattern_syntax(
    pattern: &Pat,
    file: &swc_common::SourceFile,
    restrictions: BindingRestrictions,
) -> Result<()> {
    match pattern {
        Pat::Ident(identifier) => {
            validate_assignment_identifier_reference_syntax(&identifier.id, file, restrictions)?
        }
        Pat::Assign(assign) => {
            validate_assignment_pattern_syntax(&assign.left, file, restrictions)?;
            validate_expression_syntax_with_restrictions(&assign.right, file, restrictions)?;
        }
        Pat::Array(array) => {
            for element in array.elems.iter().flatten() {
                validate_assignment_pattern_syntax(element, file, restrictions)?;
            }
        }
        Pat::Object(object) => {
            for property in &object.props {
                match property {
                    ObjectPatProp::KeyValue(property) => {
                        validate_property_name_syntax_with_restrictions(
                            &property.key,
                            file,
                            restrictions,
                        )?;
                        validate_assignment_pattern_syntax(&property.value, file, restrictions)?;
                    }
                    ObjectPatProp::Assign(property) => {
                        validate_assignment_identifier_reference_syntax(
                            &property.key,
                            file,
                            restrictions,
                        )?;
                        if let Some(value) = &property.value {
                            validate_expression_syntax_with_restrictions(
                                value,
                                file,
                                restrictions,
                            )?;
                        }
                    }
                    ObjectPatProp::Rest(rest) => {
                        validate_assignment_pattern_syntax(&rest.arg, file, restrictions)?
                    }
                }
            }
        }
        Pat::Rest(rest) => validate_assignment_pattern_syntax(&rest.arg, file, restrictions)?,
        Pat::Expr(expression) => {
            validate_expression_syntax_with_restrictions(expression, file, restrictions)?
        }
        _ => {}
    }

    Ok(())
}

fn trim_js_whitespace_and_comments(mut source: &str) -> &str {
    loop {
        let trimmed = source.trim_start_matches(|character: char| character.is_whitespace());
        if let Some(rest) = trimmed.strip_prefix("//") {
            let Some(line_end) = rest.find(['\n', '\r']) else {
                return "";
            };
            source = &rest[line_end..];
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("/*") {
            let Some(comment_end) = rest.find("*/") else {
                return "";
            };
            source = &rest[comment_end + 2..];
            continue;
        }
        return trimmed;
    }
}

fn ascii_identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

fn strip_import_keyword(source: &str) -> Option<&str> {
    let trimmed = trim_js_whitespace_and_comments(source);
    let rest = trimmed.strip_prefix("import")?;
    if rest
        .as_bytes()
        .first()
        .is_some_and(|byte| ascii_identifier_continue(*byte))
    {
        return None;
    }
    Some(rest)
}

fn strip_import_phase<'a>(rest: &'a str, phase: &str) -> Option<&'a str> {
    let rest = trim_js_whitespace_and_comments(rest).strip_prefix('.')?;
    let rest = trim_js_whitespace_and_comments(rest);
    let rest = rest.strip_prefix(phase)?;
    if rest
        .as_bytes()
        .first()
        .is_some_and(|byte| ascii_identifier_continue(*byte))
    {
        return None;
    }
    Some(rest)
}

fn new_expression_callee_is_import_call(
    callee: &Expr,
    file: &swc_common::SourceFile,
) -> Result<bool> {
    let raw_callee = source_slice_for_span(file, callee.span())?;
    let Some(rest) = strip_import_keyword(raw_callee) else {
        return Ok(false);
    };
    let rest = trim_js_whitespace_and_comments(rest);
    if rest.starts_with('(') {
        return Ok(true);
    }
    if strip_import_phase(rest, "defer").is_some() || strip_import_phase(rest, "source").is_some() {
        return Ok(true);
    }
    Ok(false)
}

fn collect_direct_block_lexically_declared_names(statements: &[Stmt]) -> Result<Vec<String>> {
    let mut names = Vec::new();

    for statement in statements {
        match statement {
            Stmt::Decl(Decl::Var(variable_declaration))
                if !matches!(variable_declaration.kind, VarDeclKind::Var) =>
            {
                names.extend(collect_var_decl_bound_names(variable_declaration)?);
            }
            Stmt::Decl(Decl::Using(using_declaration)) => {
                names.extend(collect_using_decl_bound_names(using_declaration)?);
            }
            Stmt::Decl(Decl::Fn(function_declaration)) => {
                names.push(function_declaration.ident.sym.to_string());
            }
            Stmt::Decl(Decl::Class(class_declaration)) => {
                names.push(class_declaration.ident.sym.to_string());
            }
            _ => {}
        }
    }

    Ok(names)
}

fn validate_arrow_parameters_do_not_overlap_body_lexical_names(
    parameters: &[Pat],
    body: &BlockStmt,
) -> Result<()> {
    let lexical_names = collect_direct_block_lexically_declared_names(&body.stmts)?
        .into_iter()
        .collect::<HashSet<_>>();

    if lexical_names.is_empty() {
        return Ok(());
    }

    for parameter in parameters {
        let mut parameter_names = Vec::new();
        collect_pattern_binding_names(parameter, &mut parameter_names)?;
        for name in parameter_names {
            ensure!(
                !lexical_names.contains(&name),
                "arrow parameter name `{name}` conflicts with lexical declaration in body"
            );
        }
    }

    Ok(())
}

fn validate_assignment_target_syntax(
    target: &AssignTarget,
    file: &swc_common::SourceFile,
    restrictions: BindingRestrictions,
) -> Result<()> {
    match target {
        AssignTarget::Simple(SimpleAssignTarget::Ident(identifier)) => {
            validate_assignment_identifier_reference_syntax(&identifier.id, file, restrictions)?
        }
        AssignTarget::Simple(SimpleAssignTarget::Member(member)) => {
            validate_expression_syntax_with_restrictions(&member.obj, file, restrictions)?;
            if let MemberProp::Computed(property) = &member.prop {
                validate_expression_syntax_with_restrictions(&property.expr, file, restrictions)?;
            }
        }
        AssignTarget::Simple(SimpleAssignTarget::SuperProp(super_property)) => {
            if let SuperProp::Computed(property) = &super_property.prop {
                validate_expression_syntax_with_restrictions(&property.expr, file, restrictions)?;
            }
        }
        AssignTarget::Simple(SimpleAssignTarget::Paren(parenthesized)) => {
            validate_expression_syntax_with_restrictions(&parenthesized.expr, file, restrictions)?
        }
        AssignTarget::Simple(SimpleAssignTarget::OptChain(optional_chain)) => {
            match optional_chain.base.as_ref() {
                OptChainBase::Member(member) => {
                    validate_expression_syntax_with_restrictions(&member.obj, file, restrictions)?;
                    if let MemberProp::Computed(property) = &member.prop {
                        validate_expression_syntax_with_restrictions(
                            &property.expr,
                            file,
                            restrictions,
                        )?;
                    }
                }
                OptChainBase::Call(call) => {
                    validate_expression_syntax_with_restrictions(&call.callee, file, restrictions)?;
                    for argument in &call.args {
                        validate_expression_syntax_with_restrictions(
                            &argument.expr,
                            file,
                            restrictions,
                        )?;
                    }
                }
            }
        }
        AssignTarget::Pat(pattern) => {
            let pattern: Pat = pattern.clone().into();
            validate_assignment_pattern_syntax(&pattern, file, restrictions)?;
        }
        AssignTarget::Simple(_) => {}
    }

    Ok(())
}

pub(crate) fn validate_expression_syntax_with_restrictions(
    expression: &Expr,
    file: &swc_common::SourceFile,
    restrictions: BindingRestrictions,
) -> Result<()> {
    match expression {
        Expr::Lit(Lit::Num(number)) => validate_number_literal_syntax(number, file)?,
        Expr::Lit(Lit::Str(string)) => validate_string_literal_syntax(string)?,
        Expr::Lit(Lit::Regex(regex)) => validate_regex_literal_syntax(regex, file)?,
        Expr::Ident(identifier) => {
            validate_identifier_reference_syntax(identifier, file, restrictions)?;
        }
        Expr::Call(call) => {
            if let Callee::Expr(callee) = &call.callee {
                validate_expression_syntax_with_restrictions(callee, file, restrictions)?;
            }
            for argument in &call.args {
                validate_expression_syntax_with_restrictions(&argument.expr, file, restrictions)?;
            }
        }
        Expr::New(new_expression) => {
            ensure!(
                !new_expression_callee_is_import_call(&new_expression.callee, file)?,
                "dynamic import calls cannot be used as constructors"
            );
            validate_expression_syntax_with_restrictions(
                &new_expression.callee,
                file,
                restrictions,
            )?;
            for argument in new_expression.args.iter().flatten() {
                validate_expression_syntax_with_restrictions(&argument.expr, file, restrictions)?;
            }
        }
        Expr::Await(await_expression) => {
            ensure!(
                !restrictions.await_expression_forbidden,
                "`await` cannot be used in a class static initialization block"
            );
            validate_expression_syntax_with_restrictions(
                &await_expression.arg,
                file,
                restrictions,
            )?;
        }
        Expr::Yield(yield_expression) => {
            if let Some(argument) = &yield_expression.arg {
                validate_expression_syntax_with_restrictions(argument, file, restrictions)?;
            }
        }
        Expr::Paren(parenthesized) => {
            validate_expression_syntax_with_restrictions(&parenthesized.expr, file, restrictions)?
        }
        Expr::Array(array) => {
            for element in array.elems.iter().flatten() {
                validate_expression_syntax_with_restrictions(&element.expr, file, restrictions)?;
            }
        }
        Expr::Object(object) => {
            validate_object_literal_duplicate_proto_setters(object)?;
            for property in &object.props {
                match property {
                    PropOrSpread::Spread(spread) => validate_expression_syntax_with_restrictions(
                        &spread.expr,
                        file,
                        restrictions,
                    )?,
                    PropOrSpread::Prop(property) => match &**property {
                        Prop::Shorthand(identifier) => {
                            validate_identifier_reference_syntax(identifier, file, restrictions)?;
                        }
                        Prop::KeyValue(property) => {
                            validate_property_name_syntax_with_restrictions(
                                &property.key,
                                file,
                                restrictions,
                            )?;
                            validate_expression_syntax_with_restrictions(
                                &property.value,
                                file,
                                restrictions,
                            )?;
                        }
                        Prop::Getter(property) => {
                            validate_object_getter_contextual_keyword(property, file)?;
                            validate_property_name_syntax_with_restrictions(
                                &property.key,
                                file,
                                restrictions,
                            )?;
                            if let Some(body) = &property.body {
                                for statement in &body.stmts {
                                    validate_statement_syntax_with_restrictions(
                                        statement,
                                        file,
                                        restrictions,
                                    )?;
                                }
                            }
                        }
                        Prop::Setter(property) => {
                            validate_object_setter_contextual_keyword(property, file)?;
                            validate_property_name_syntax_with_restrictions(
                                &property.key,
                                file,
                                restrictions,
                            )?;
                            validate_pattern_syntax_with_restrictions(
                                &property.param,
                                file,
                                restrictions,
                            )?;
                            if let Some(body) = &property.body {
                                for statement in &body.stmts {
                                    validate_statement_syntax_with_restrictions(
                                        statement,
                                        file,
                                        restrictions,
                                    )?;
                                }
                            }
                        }
                        Prop::Method(property) => {
                            validate_async_object_method_no_line_terminator(property, file)?;
                            validate_property_name_syntax_with_restrictions(
                                &property.key,
                                file,
                                restrictions,
                            )?;
                            ensure_parameter_names_are_valid(
                                property
                                    .function
                                    .params
                                    .iter()
                                    .map(|parameter| &parameter.pat),
                                property
                                    .function
                                    .params
                                    .iter()
                                    .all(|parameter| matches!(parameter.pat, Pat::Ident(_))),
                                true,
                            )?;
                            validate_function_syntax_with_restrictions(
                                &property.function,
                                file,
                                restrictions,
                            )?;
                        }
                        Prop::Assign(property) => {
                            validate_identifier_reference_syntax(
                                &property.key,
                                file,
                                restrictions,
                            )?;
                            validate_expression_syntax_with_restrictions(
                                &property.value,
                                file,
                                restrictions,
                            )?;
                        }
                    },
                }
            }
        }
        Expr::Member(member) => {
            validate_expression_syntax_with_restrictions(&member.obj, file, restrictions)?;
            if let MemberProp::Computed(property) = &member.prop {
                validate_expression_syntax_with_restrictions(&property.expr, file, restrictions)?;
            }
        }
        Expr::Unary(unary) => {
            validate_expression_syntax_with_restrictions(&unary.arg, file, restrictions)?
        }
        Expr::Update(update) => {
            validate_expression_syntax_with_restrictions(&update.arg, file, restrictions)?
        }
        Expr::Bin(binary) => {
            validate_expression_syntax_with_restrictions(&binary.left, file, restrictions)?;
            validate_expression_syntax_with_restrictions(&binary.right, file, restrictions)?;
        }
        Expr::Assign(assignment) => {
            validate_assignment_target_syntax(&assignment.left, file, restrictions)?;
            validate_expression_syntax_with_restrictions(&assignment.right, file, restrictions)?;
        }
        Expr::Cond(conditional) => {
            validate_expression_syntax_with_restrictions(&conditional.test, file, restrictions)?;
            validate_expression_syntax_with_restrictions(&conditional.cons, file, restrictions)?;
            validate_expression_syntax_with_restrictions(&conditional.alt, file, restrictions)?;
        }
        Expr::Seq(sequence) => {
            for expression in &sequence.exprs {
                validate_expression_syntax_with_restrictions(expression, file, restrictions)?;
            }
        }
        Expr::Fn(function) => {
            if let Some(identifier) = &function.ident {
                ensure!(
                    !((restrictions.module_await_reserved || function.function.is_async)
                        && is_await_like_identifier(identifier.sym.as_ref())),
                    "`await` cannot be used as a binding identifier in this context"
                );
                ensure!(
                    !(function.function.is_generator
                        && is_yield_like_identifier(identifier.sym.as_ref())),
                    "`yield` cannot be used as a binding identifier in a generator function"
                );
            }
            validate_function_syntax_with_restrictions(&function.function, file, restrictions)?
        }
        Expr::Arrow(arrow) => {
            // A non-async arrow body is parsed with [~Await]
            // (ConciseBody : { FunctionBody[~Yield, ~Await] }), so an
            // enclosing await restriction (async function or class static
            // block) does not reach into the body: `await` is a valid
            // binding there. Arrow parameters, however, inherit [?Await]
            // from the enclosing context.
            let body_restrictions = BindingRestrictions {
                await_reserved: arrow.is_async,
                module_await_reserved: restrictions.module_await_reserved,
                yield_reserved: false,
                await_expression_forbidden: false,
            };
            let parameter_restrictions = BindingRestrictions {
                await_reserved: restrictions.await_reserved || arrow.is_async,
                module_await_reserved: restrictions.module_await_reserved,
                yield_reserved: body_restrictions.yield_reserved,
                await_expression_forbidden: restrictions.await_expression_forbidden
                    || restrictions.await_reserved
                    || arrow.is_async,
            };
            ensure_parameter_names_are_valid(
                arrow.params.iter(),
                arrow
                    .params
                    .iter()
                    .all(|parameter| matches!(parameter, Pat::Ident(_))),
                true,
            )?;
            for parameter in &arrow.params {
                validate_pattern_syntax_with_restrictions(parameter, file, parameter_restrictions)?;
            }
            match &*arrow.body {
                BlockStmtOrExpr::BlockStmt(block) => {
                    validate_arrow_parameters_do_not_overlap_body_lexical_names(
                        &arrow.params,
                        block,
                    )?;
                    for statement in &block.stmts {
                        validate_statement_syntax_with_restrictions(
                            statement,
                            file,
                            body_restrictions,
                        )?;
                    }
                }
                BlockStmtOrExpr::Expr(expression) => validate_expression_syntax_with_restrictions(
                    expression,
                    file,
                    body_restrictions,
                )?,
            }
        }
        Expr::Class(class) => {
            if let Some(identifier) = &class.ident {
                ensure!(
                    !(restrictions.reserves_await_identifier()
                        && is_await_like_identifier(identifier.sym.as_ref())),
                    "`await` cannot be used as a binding identifier in this context"
                );
                ensure!(
                    !(restrictions.yield_reserved
                        && is_yield_like_identifier(identifier.sym.as_ref())),
                    "`yield` cannot be used as a binding identifier in a generator function"
                );
            }
            validate_class_syntax_with_restrictions(&class.class, file, restrictions)?
        }
        Expr::Tpl(template) => {
            for expression in &template.exprs {
                validate_expression_syntax_with_restrictions(expression, file, restrictions)?;
            }
        }
        Expr::TaggedTpl(tagged) => {
            validate_expression_syntax_with_restrictions(&tagged.tag, file, restrictions)?;
            for expression in &tagged.tpl.exprs {
                validate_expression_syntax_with_restrictions(expression, file, restrictions)?;
            }
        }
        _ => {}
    }

    Ok(())
}
