//! C++ type representation.

/// Log a type diagnostic message if FRAGILE_DIAGNOSTIC is enabled.
/// Used for debugging type conversion issues.
fn log_type_diagnostic(category: &str, message: &str) {
    if std::env::var("FRAGILE_DIAGNOSTIC")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
    {
        eprintln!("[FRAGILE-DIAG] Type {}: {}", category, message);
    }
}

/// Parse comma-separated template arguments, respecting nested templates.
/// Returns a vector of trimmed argument strings.
///
/// # Example
/// ```ignore
/// let args = parse_template_args("int, std::vector<int>, double");
/// assert_eq!(args, vec!["int", "std::vector<int>", "double"]);
/// ```
pub fn parse_template_args(args: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut angle_depth = 0; // Depth for <>
    let mut paren_depth = 0; // Depth for () - for function pointer types

    for ch in args.chars() {
        match ch {
            '<' => {
                angle_depth += 1;
                current.push(ch);
            }
            '>' => {
                angle_depth -= 1;
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth -= 1;
                current.push(ch);
            }
            ',' if angle_depth == 0 && paren_depth == 0 => {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    result.push(trimmed);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        result.push(trimmed);
    }

    result
}

fn parse_cpp_named_cast(expr: &str) -> Option<(&str, &str)> {
    let trimmed = expr.trim();
    for cast_name in ["static_cast", "reinterpret_cast", "const_cast"] {
        if !trimmed.starts_with(cast_name) {
            continue;
        }
        let after_name = trimmed[cast_name.len()..].trim_start();
        if !after_name.starts_with('<') {
            continue;
        }

        let mut angle_depth = 0i32;
        let mut cast_type_end: Option<usize> = None;
        for (idx, ch) in after_name.char_indices() {
            match ch {
                '<' => angle_depth += 1,
                '>' => {
                    angle_depth -= 1;
                    if angle_depth == 0 {
                        cast_type_end = Some(idx);
                        break;
                    }
                }
                _ => {}
            }
        }
        let cast_type_end = cast_type_end?;
        let cast_target = after_name[1..cast_type_end].trim();
        let rest = after_name[cast_type_end + 1..].trim_start();
        if !rest.starts_with('(') {
            continue;
        }

        let mut paren_depth = 0i32;
        let mut value_end: Option<usize> = None;
        for (idx, ch) in rest.char_indices() {
            match ch {
                '(' => paren_depth += 1,
                ')' => {
                    paren_depth -= 1;
                    if paren_depth == 0 {
                        value_end = Some(idx);
                        break;
                    }
                }
                _ => {}
            }
        }
        let value_end = value_end?;
        if value_end + 1 != rest.len() {
            continue;
        }

        let value_expr = rest[1..value_end].trim();
        return Some((cast_target, value_expr));
    }
    None
}

fn is_integer_size_literal(expr: &str) -> bool {
    let raw = expr.trim();
    if raw.is_empty() {
        return false;
    }
    let mut s = raw;
    while let Some(stripped) = s.strip_suffix(['u', 'U', 'l', 'L']) {
        s = stripped;
    }
    if s.starts_with("0x") || s.starts_with("0X") {
        return s[2..].chars().all(|c| c.is_ascii_hexdigit() || c == '_');
    }
    if s.starts_with("0b") || s.starts_with("0B") {
        return s[2..].chars().all(|c| c == '0' || c == '1' || c == '_');
    }
    s.chars().all(|c| c.is_ascii_digit() || c == '_')
}

fn normalize_array_size_expr(size: &str) -> String {
    let mut current = size.trim().to_string();
    while let Some((_cast_ty, inner)) = parse_cpp_named_cast(&current) {
        current = inner.to_string();
    }

    let current = current.trim();
    if current.is_empty() {
        return "0usize".to_string();
    }
    // Template parameter pack size expressions like `sizeof...(_Args)` are
    // unresolved in concrete Rust output. Emit a conservative zero-length
    // bound so downstream transpiled code remains syntactically valid.
    let compact = current.replace(' ', "");
    if compact.contains("sizeof...") || compact.contains("...") {
        return "0usize".to_string();
    }
    if is_integer_size_literal(current) {
        return current.to_string();
    }
    if current.ends_with("usize") || current.contains(" as usize") {
        return current.to_string();
    }
    format!("({}) as usize", current)
}

fn strip_cpp_type_tag_prefix(name: &str) -> &str {
    for prefix in ["class ", "struct ", "enum ", "union "] {
        if let Some(rest) = name.strip_prefix(prefix) {
            return rest.trim_start();
        }
    }
    name
}

fn strip_cv_qualifiers_and_tag_prefix(name: &str) -> &str {
    let mut current = name.trim();
    loop {
        let mut changed = false;
        for qualifier in ["const ", "volatile "] {
            if let Some(rest) = current.strip_prefix(qualifier) {
                current = rest.trim_start();
                changed = true;
            }
        }
        let stripped = strip_cpp_type_tag_prefix(current);
        if stripped != current {
            current = stripped.trim_start();
            changed = true;
        }
        if !changed {
            break;
        }
    }
    current
}

fn map_single_template_alias_to_std(
    spelling: &str,
    alias_prefix: &str,
    std_path: &str,
) -> Option<String> {
    let inner = spelling.strip_prefix(alias_prefix)?.strip_suffix('>')?;
    let args = parse_template_args(inner);
    if args.len() != 1 {
        return None;
    }
    let mapped = map_alias_template_arg_to_rust(&args[0]);
    Some(format!("{}<{}>", std_path, mapped))
}

fn is_unresolved_placeholder_type_name(name: &str) -> bool {
    let mut cleaned = strip_cv_qualifiers_and_tag_prefix(name).trim();
    cleaned = cleaned.trim_start_matches("::").trim();
    if let Some(rest) = cleaned.strip_prefix("crate::") {
        cleaned = rest.trim();
    }
    matches!(cleaned, "_" | "__" | "auto")
        || cleaned.starts_with("type-parameter-")
        || cleaned.starts_with("type_parameter_")
}

fn map_single_template_alias_to_std_with_unit_placeholder_fallback(
    spelling: &str,
    alias_prefix: &str,
    std_path: &str,
) -> Option<String> {
    let inner = spelling.strip_prefix(alias_prefix)?.strip_suffix('>')?;
    let args = parse_template_args(inner);
    if args.len() != 1 {
        return None;
    }
    let mut mapped = map_alias_template_arg_to_rust(&args[0]);
    if is_unresolved_placeholder_type_name(mapped.as_str()) {
        mapped = "()".to_string();
    }
    Some(format!("{}<{}>", std_path, mapped))
}

fn map_single_template_alias_to_std_allow_extra_args(
    spelling: &str,
    alias_prefix: &str,
    std_path: &str,
) -> Option<String> {
    let inner = spelling.strip_prefix(alias_prefix)?.strip_suffix('>')?;
    let args = parse_template_args(inner);
    if args.is_empty() {
        return None;
    }
    let mapped = map_alias_template_arg_to_rust(&args[0]);
    Some(format!("{}<{}>", std_path, mapped))
}

fn map_result_template_arg_to_rust(arg: &str) -> String {
    let trimmed = arg.trim();
    if trimmed == "()" {
        return "()".to_string();
    }
    let normalized = strip_cv_qualifiers_and_tag_prefix(trimmed);
    if normalized == "void" || is_unresolved_placeholder_type_name(normalized) {
        return "()".to_string();
    }
    map_alias_template_arg_to_rust(trimmed)
}

fn map_double_template_result_alias_to_std(
    spelling: &str,
    alias_prefix: &str,
    std_path: &str,
) -> Option<String> {
    let inner = spelling.strip_prefix(alias_prefix)?.strip_suffix('>')?;
    let args = parse_template_args(inner);
    if args.len() != 2 {
        return None;
    }
    let ok = map_result_template_arg_to_rust(&args[0]);
    let err = map_result_template_arg_to_rust(&args[1]);
    Some(format!("{}<{}, {}>", std_path, ok, err))
}

fn map_double_template_alias_to_std_allow_extra_args(
    spelling: &str,
    alias_prefix: &str,
    std_path: &str,
) -> Option<String> {
    let inner = spelling.strip_prefix(alias_prefix)?.strip_suffix('>')?;
    let args = parse_template_args(inner);
    if args.len() < 2 {
        return None;
    }
    let left = map_alias_template_arg_to_rust(&args[0]);
    let right = map_alias_template_arg_to_rust(&args[1]);
    Some(format!("{}<{}, {}>", std_path, left, right))
}

fn map_single_template_alias_to_pointer(
    spelling: &str,
    alias_prefix: &str,
    pointer_prefix: &str,
) -> Option<String> {
    let inner = spelling.strip_prefix(alias_prefix)?.strip_suffix('>')?;
    let args = parse_template_args(inner);
    if args.len() != 1 {
        return None;
    }
    let mapped = map_alias_template_arg_to_rust(&args[0]);
    Some(format!("{}{}", pointer_prefix, mapped))
}

fn is_explicit_rust_function_pointer_type(arg: &str) -> bool {
    let trimmed = arg.trim();
    trimmed.contains("extern \"C\" fn(")
        || trimmed.contains("extern \"C\" fn (")
        || trimmed.starts_with("fn(")
}

fn map_alias_template_arg_to_rust(arg: &str) -> String {
    let trimmed = arg.trim();
    if trimmed == "()" {
        // Preserve explicit unit lanes when normalizing already-lowered Rust
        // wrapper spellings (for example recursive std-wrapper normalization).
        return "()".to_string();
    }
    if is_explicit_rust_function_pointer_type(trimmed) {
        // Preserve already-lowered Rust fn pointer spellings inside template args.
        return trimmed.to_string();
    }
    CppType::Named(trimmed.to_string()).to_rust_type_str()
}

fn map_join_handle_payload_to_std(payload: &str) -> String {
    let payload = payload.trim();
    let normalized = strip_cv_qualifiers_and_tag_prefix(payload).trim_end_matches('_');
    if normalized == "()" {
        "()".to_string()
    } else if normalized.is_empty()
        || normalized == "void"
        || is_unresolved_placeholder_type_name(normalized)
    {
        "()".to_string()
    } else {
        CppType::Named(payload.trim_end_matches('_').to_string()).to_rust_type_str()
    }
}

fn map_thread_join_handle_with_prefix_to_std(spelling: &str, prefix: &str) -> Option<String> {
    let inner = spelling.strip_prefix(prefix)?.strip_suffix('>')?;
    let args = parse_template_args(inner);
    if args.len() != 1 {
        return None;
    }
    let mapped = map_join_handle_payload_to_std(&args[0]);
    Some(format!("std::thread::JoinHandle<{}>", mapped))
}

fn map_thread_join_handle_to_std(spelling: &str) -> Option<String> {
    map_thread_join_handle_with_prefix_to_std(spelling, "rusty::thread::JoinHandle<")
}

fn map_std_thread_join_handle_to_std(spelling: &str) -> Option<String> {
    map_thread_join_handle_with_prefix_to_std(spelling, "std::thread::JoinHandle<")
}

fn map_unqualified_thread_join_handle_to_std(spelling: &str) -> Option<String> {
    map_thread_join_handle_with_prefix_to_std(spelling, "JoinHandle<")
}

fn map_lowered_thread_join_handle_to_std(spelling: &str) -> Option<String> {
    for prefix in [
        "rusty::thread::rusty_thread_JoinHandle_",
        "rusty_thread_JoinHandle_",
        "std_thread_JoinHandle_",
        "JoinHandle_",
    ] {
        if let Some(rest) = spelling.strip_prefix(prefix) {
            let lowered = rest.trim_end_matches('_');
            if lowered.is_empty()
                || !lowered
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                continue;
            }
            let mapped = map_join_handle_payload_to_std(lowered);
            return Some(format!("std::thread::JoinHandle<{}>", mapped));
        }
    }
    None
}

fn strip_lowered_cpp_prefix_tokens(name: &str) -> &str {
    let mut current = name;
    loop {
        let mut changed = false;
        for qualifier in ["const", "volatile"] {
            if let Some(rest) = current.strip_prefix(qualifier) {
                current = rest.strip_prefix('_').unwrap_or(rest);
                changed = true;
            }
        }
        for tag in ["class", "struct", "enum", "union"] {
            if let Some(rest) = current.strip_prefix(tag) {
                current = rest.strip_prefix('_').unwrap_or(rest);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    current
}

fn map_lowered_mpsc_unit_to_std(spelling: &str) -> Option<String> {
    let cleaned = strip_lowered_cpp_prefix_tokens(spelling.strip_suffix('_').unwrap_or(spelling));
    if matches!(
        cleaned,
        "Unit" | "sync_mpsc_Unit" | "rusty_sync_mpsc_Unit" | "std_sync_mpsc_Unit"
    ) {
        Some("()".to_string())
    } else {
        None
    }
}

fn map_lowered_mpsc_error_enum_to_std(spelling: &str) -> Option<String> {
    let cleaned = strip_lowered_cpp_prefix_tokens(spelling.strip_suffix('_').unwrap_or(spelling));
    match cleaned {
        "rusty_sync_mpsc_RecvError" | "std_sync_mpsc_RecvError" => {
            Some("std::sync::mpsc::RecvError".to_string())
        }
        "rusty_sync_mpsc_TryRecvError" | "std_sync_mpsc_TryRecvError" => {
            Some("std::sync::mpsc::TryRecvError".to_string())
        }
        "rusty_sync_mpsc_TrySendError" | "std_sync_mpsc_TrySendError" => {
            Some("std::sync::mpsc::TrySendError<()>".to_string())
        }
        _ => None,
    }
}

fn map_lowered_result_with_mpsc_error_to_std(spelling: &str) -> Option<String> {
    let error_bases = [
        "rusty_sync_mpsc_RecvError",
        "std_sync_mpsc_RecvError",
        "rusty_sync_mpsc_TryRecvError",
        "std_sync_mpsc_TryRecvError",
        "rusty_sync_mpsc_TrySendError",
        "std_sync_mpsc_TrySendError",
    ];
    let tag_prefixes = [
        "",
        "enum",
        "class",
        "struct",
        "constenum",
        "constclass",
        "conststruct",
        "volatileenum",
        "volatileclass",
        "volatilestruct",
    ];

    for prefix in ["rusty_Result_", "Result_"] {
        let Some(rest) = spelling.strip_prefix(prefix) else {
            continue;
        };
        let lowered = rest.strip_suffix('_').unwrap_or(rest);
        if !lowered
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            continue;
        }

        for base in error_bases {
            let Some(err_mapped) = map_lowered_mpsc_error_enum_to_std(base) else {
                continue;
            };
            for tag_prefix in tag_prefixes {
                let suffix = format!("{tag_prefix}{base}");
                let Some(ok_raw) = lowered.strip_suffix(&suffix) else {
                    continue;
                };
                let Some(ok_lowered) = ok_raw.strip_suffix('_') else {
                    continue;
                };
                let ok_mapped = if ok_lowered.is_empty()
                    || ok_lowered == "void"
                    || is_unresolved_placeholder_type_name(ok_lowered)
                {
                    "()".to_string()
                } else if let Some(unit) = map_lowered_mpsc_unit_to_std(ok_lowered) {
                    unit
                } else {
                    CppType::Named(ok_lowered.to_string()).to_rust_type_str()
                };
                return Some(format!("std::result::Result<{}, {}>", ok_mapped, err_mapped));
            }
        }
    }
    None
}

fn map_lowered_mpsc_endpoint_to_std(spelling: &str) -> Option<String> {
    for (prefix, std_path) in [
        ("std_sync_mpsc_Sender_", "std::sync::mpsc::Sender"),
        ("std_sync_mpsc_Receiver_", "std::sync::mpsc::Receiver"),
        ("std_sync_mpsc_SyncSender_", "std::sync::mpsc::SyncSender"),
        ("std_sync_mpsc_TrySendError_", "std::sync::mpsc::TrySendError"),
        ("rusty_sync_mpsc_Sender_", "std::sync::mpsc::Sender"),
        ("rusty_sync_mpsc_Receiver_", "std::sync::mpsc::Receiver"),
        ("rusty_sync_mpsc_SyncSender_", "std::sync::mpsc::SyncSender"),
        (
            "rusty_sync_mpsc_TrySendError_",
            "std::sync::mpsc::TrySendError",
        ),
    ] {
        if let Some(rest) = spelling.strip_prefix(prefix) {
            let lowered = rest.strip_suffix('_').unwrap_or(rest);
            if !lowered
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                continue;
            }
            let mapped = if lowered.is_empty()
                || lowered == "void"
                || is_unresolved_placeholder_type_name(lowered)
            {
                "()".to_string()
            } else if let Some(unit) = map_lowered_mpsc_unit_to_std(lowered) {
                unit
            } else {
                CppType::Named(lowered.to_string()).to_rust_type_str()
            };
            return Some(format!("{}<{}>", std_path, mapped));
        }
    }
    None
}

fn map_lowered_set_unit_marker_to_std(spelling: &str) -> Option<String> {
    let cleaned = strip_lowered_cpp_prefix_tokens(spelling.trim_end_matches('_'));
    let is_set_unit = (cleaned.starts_with("rusty_BTreeSet_")
        || cleaned.starts_with("BTreeSet_")
        || cleaned.starts_with("rusty_HashSet_")
        || cleaned.starts_with("HashSet_"))
        && cleaned.ends_with("_Unit");
    if is_set_unit {
        Some("()".to_string())
    } else {
        None
    }
}

fn map_lowered_rusty_single_template_alias_to_std(spelling: &str) -> Option<String> {
    for lowered_spelling in [spelling, strip_lowered_cpp_prefix_tokens(spelling)] {
        for (prefix, std_path) in [
            ("rusty_Option_", "std::option::Option"),
            ("Option_", "std::option::Option"),
            ("rusty_Box_", "std::boxed::Box"),
            ("Box_", "std::boxed::Box"),
            ("rusty_Boxed_", "std::boxed::Box"),
            ("Boxed_", "std::boxed::Box"),
            ("rusty_Arc_", "std::sync::Arc"),
            ("Arc_", "std::sync::Arc"),
            ("rusty_ArcWeak_", "std::sync::Weak"),
            ("ArcWeak_", "std::sync::Weak"),
            ("rusty_Rc_", "std::rc::Rc"),
            ("Rc_", "std::rc::Rc"),
            ("rusty_Weak_", "std::rc::Weak"),
            ("Weak_", "std::rc::Weak"),
            ("rusty_Cell_", "std::cell::Cell"),
            ("Cell_", "std::cell::Cell"),
            ("rusty_RefCell_", "std::cell::RefCell"),
            ("RefCell_", "std::cell::RefCell"),
            ("rusty_UnsafeCell_", "std::cell::UnsafeCell"),
            ("UnsafeCell_", "std::cell::UnsafeCell"),
            ("rusty_Mutex_", "std::sync::Mutex"),
            ("Mutex_", "std::sync::Mutex"),
            ("rusty_RwLock_", "std::sync::RwLock"),
            ("RwLock_", "std::sync::RwLock"),
        ] {
            let Some(rest) = lowered_spelling.strip_prefix(prefix) else {
                continue;
            };
            let lowered = rest.trim_end_matches('_');
            if lowered.is_empty()
                || !lowered
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                continue;
            }

            let lowered = strip_lowered_cpp_prefix_tokens(lowered);
            if lowered.is_empty() {
                continue;
            }
            let mapped = CppType::Named(lowered.to_string()).to_rust_type_str();
            return Some(format!("{}<{}>", std_path, mapped));
        }
    }
    None
}

fn map_rusty_type_to_std(spelling: &str) -> Option<String> {
    let mut cleaned = strip_cv_qualifiers_and_tag_prefix(spelling);
    cleaned = cleaned.trim_start_matches("::").trim();
    if let Some(rest) = cleaned.strip_prefix("crate::") {
        cleaned = rest.trim();
    }
    let root_name = cleaned.split('<').next().unwrap_or(cleaned).trim();
    let root_is_unqualified = !root_name.contains("::");

    match cleaned {
        "rusty::String" => return Some("std::string::String".to_string()),
        "rusty::string::String" => return Some("std::string::String".to_string()),
        "rusty::Barrier" | "rusty::sync::Barrier" => {
            return Some("std::sync::Barrier".to_string());
        }
        "rusty::Condvar" | "rusty::sync::Condvar" => {
            return Some("std::sync::Condvar".to_string());
        }
        "rusty::Once" | "rusty::sync::Once" => return Some("std::sync::Once".to_string()),
        "rusty::WaitTimeoutResult" | "rusty::sync::WaitTimeoutResult" => {
            return Some("std::sync::WaitTimeoutResult".to_string());
        }
        "rusty::None_t" => return Some("()".to_string()),
        "rusty::sync::mpsc::Unit" => return Some("()".to_string()),
        "rusty::sync::mpsc::RecvError" => return Some("std::sync::mpsc::RecvError".to_string()),
        "rusty::sync::mpsc::TryRecvError" => {
            return Some("std::sync::mpsc::TryRecvError".to_string());
        }
        "rusty::sync::mpsc::TrySendError" => {
            // Rust std `TrySendError<T>` is generic. Rusty often surfaces a
            // non-generic spelling, so use unit payload as a conservative
            // default that keeps the path on std surfaces.
            return Some("std::sync::mpsc::TrySendError<()>".to_string());
        }
        // `using namespace rusty;` can leave aliases unqualified in Clang spellings.
        "String" => return Some("std::string::String".to_string()),
        "Barrier" => return Some("std::sync::Barrier".to_string()),
        "Condvar" => return Some("std::sync::Condvar".to_string()),
        "Once" => return Some("std::sync::Once".to_string()),
        "WaitTimeoutResult" => return Some("std::sync::WaitTimeoutResult".to_string()),
        "None_t" => return Some("()".to_string()),
        "Unit" => return Some("()".to_string()),
        "RecvError" => return Some("std::sync::mpsc::RecvError".to_string()),
        "TryRecvError" => return Some("std::sync::mpsc::TryRecvError".to_string()),
        "TrySendError" => return Some("std::sync::mpsc::TrySendError<()>".to_string()),
        _ => {}
    }

    if let Some(mapped) = map_thread_join_handle_to_std(cleaned) {
        return Some(mapped);
    }
    if let Some(mapped) = map_std_thread_join_handle_to_std(cleaned) {
        return Some(mapped);
    }
    if root_is_unqualified {
        if let Some(mapped) = map_unqualified_thread_join_handle_to_std(cleaned) {
            return Some(mapped);
        }
    }
    if let Some(mapped) = map_lowered_thread_join_handle_to_std(cleaned) {
        return Some(mapped);
    }
    if let Some(mapped) = map_lowered_mpsc_error_enum_to_std(cleaned) {
        return Some(mapped);
    }
    if let Some(mapped) = map_lowered_result_with_mpsc_error_to_std(cleaned) {
        return Some(mapped);
    }

    // Do not normalize nested guard aliases (`Mutex<T>::Guard`, etc.) here.
    // Their Rusty lowering often resolves through non-generic placeholders in
    // degraded output; forcing generic guard rewrites can introduce invalid
    // arity/private-path failures.

    for (alias, std_path, qualified_roots) in [
        (
            "Sender",
            "std::sync::mpsc::Sender",
            &["rusty::sync::mpsc::Sender", "std::sync::mpsc::Sender"] as &[&str],
        ),
        (
            "Receiver",
            "std::sync::mpsc::Receiver",
            &["rusty::sync::mpsc::Receiver", "std::sync::mpsc::Receiver"] as &[&str],
        ),
        (
            "SyncSender",
            "std::sync::mpsc::SyncSender",
            &["rusty::sync::mpsc::SyncSender", "std::sync::mpsc::SyncSender"] as &[&str],
        ),
    ] {
        for root in qualified_roots {
            let qualified_prefix = format!("{}<", root);
            if let Some(mapped) = map_single_template_alias_to_std_with_unit_placeholder_fallback(
                cleaned,
                &qualified_prefix,
                std_path,
            )
            {
                return Some(mapped);
            }
        }
        if root_is_unqualified {
            let bare_prefix = format!("{}<", alias);
            if let Some(mapped) = map_single_template_alias_to_std_with_unit_placeholder_fallback(
                cleaned,
                &bare_prefix,
                std_path,
            )
            {
                return Some(mapped);
            }
        }
    }
    for (alias, std_path, qualified_roots) in [(
        "TrySendError",
        "std::sync::mpsc::TrySendError",
        &["rusty::sync::mpsc::TrySendError", "std::sync::mpsc::TrySendError"] as &[&str],
    )] {
        for root in qualified_roots {
            let qualified_prefix = format!("{}<", root);
            if let Some(mapped) = map_single_template_alias_to_std_with_unit_placeholder_fallback(
                cleaned,
                &qualified_prefix,
                std_path,
            )
            {
                return Some(mapped);
            }
        }
        if root_is_unqualified {
            let bare_prefix = format!("{}<", alias);
            if let Some(mapped) = map_single_template_alias_to_std_with_unit_placeholder_fallback(
                cleaned,
                &bare_prefix,
                std_path,
            )
            {
                return Some(mapped);
            }
        }
    }
    if let Some(mapped) = map_lowered_mpsc_endpoint_to_std(cleaned) {
        return Some(mapped);
    }
    if let Some(mapped) = map_lowered_set_unit_marker_to_std(cleaned) {
        return Some(mapped);
    }
    if let Some(mapped) = map_lowered_rusty_single_template_alias_to_std(cleaned) {
        return Some(mapped);
    }
    for (prefix, std_path) in [
        ("std::option::Option<", "std::option::Option"),
        ("std::boxed::Box<", "std::boxed::Box"),
        ("std::sync::Arc<", "std::sync::Arc"),
        ("std::sync::Weak<", "std::sync::Weak"),
        ("std::rc::Rc<", "std::rc::Rc"),
        ("std::rc::Weak<", "std::rc::Weak"),
        ("std::cell::Cell<", "std::cell::Cell"),
        ("std::cell::RefCell<", "std::cell::RefCell"),
        ("std::cell::UnsafeCell<", "std::cell::UnsafeCell"),
        ("std::sync::Mutex<", "std::sync::Mutex"),
        ("std::sync::RwLock<", "std::sync::RwLock"),
        ("std::sync::mpsc::Sender<", "std::sync::mpsc::Sender"),
        ("std::sync::mpsc::Receiver<", "std::sync::mpsc::Receiver"),
        ("std::sync::mpsc::SyncSender<", "std::sync::mpsc::SyncSender"),
        ("std::sync::mpsc::TrySendError<", "std::sync::mpsc::TrySendError"),
    ] {
        if let Some(mapped) = map_single_template_alias_to_std(cleaned, prefix, std_path) {
            return Some(mapped);
        }
    }
    if let Some(mapped) = map_double_template_result_alias_to_std(
        cleaned,
        "std::result::Result<",
        "std::result::Result",
    ) {
        return Some(mapped);
    }
    for (prefix, std_path) in [
        ("std::collections::HashMap<", "std::collections::HashMap"),
        ("std::collections::BTreeMap<", "std::collections::BTreeMap"),
    ] {
        if let Some(mapped) = map_double_template_alias_to_std_allow_extra_args(
            cleaned,
            prefix,
            std_path,
        ) {
            return Some(mapped);
        }
    }

    for (alias, std_path, qualified_roots) in [
        (
            "Option",
            "std::option::Option",
            &["rusty::Option", "rusty::option::Option"] as &[&str],
        ),
        (
            "Box",
            "std::boxed::Box",
            &["rusty::Box", "rusty::boxed::Box", "rusty::Boxed"] as &[&str],
        ),
        (
            "Arc",
            "std::sync::Arc",
            &["rusty::Arc", "rusty::sync::Arc", "rusty::Shared"] as &[&str],
        ),
        (
            "ArcWeak",
            "std::sync::Weak",
            &["rusty::ArcWeak", "rusty::sync::Weak"] as &[&str],
        ),
        (
            "Rc",
            "std::rc::Rc",
            &["rusty::Rc", "rusty::rc::Rc", "rusty::RefCounted"] as &[&str],
        ),
        (
            "Weak",
            "std::rc::Weak",
            &["rusty::Weak", "rusty::rc::Weak"] as &[&str],
        ),
        (
            "Cell",
            "std::cell::Cell",
            &["rusty::Cell", "rusty::cell::Cell"] as &[&str],
        ),
        (
            "RefCell",
            "std::cell::RefCell",
            &["rusty::RefCell", "rusty::cell::RefCell"] as &[&str],
        ),
        (
            "Ref",
            "std::cell::Ref",
            &["rusty::Ref", "rusty::cell::Ref"] as &[&str],
        ),
        (
            "RefMut",
            "std::cell::RefMut",
            &["rusty::RefMut", "rusty::cell::RefMut"] as &[&str],
        ),
        (
            "UnsafeCell",
            "std::cell::UnsafeCell",
            &["rusty::UnsafeCell", "rusty::cell::UnsafeCell"] as &[&str],
        ),
        (
            "Mutex",
            "std::sync::Mutex",
            &["rusty::Mutex", "rusty::sync::Mutex"] as &[&str],
        ),
        (
            "RwLock",
            "std::sync::RwLock",
            &["rusty::RwLock", "rusty::sync::RwLock"] as &[&str],
        ),
    ] {
        for root in qualified_roots {
            let qualified_prefix = format!("{}<", root);
            if let Some(mapped) =
                map_single_template_alias_to_std(cleaned, &qualified_prefix, std_path)
            {
                return Some(mapped);
            }
        }
        if root_is_unqualified {
            let bare_prefix = format!("{}<", alias);
            if let Some(mapped) = map_single_template_alias_to_std(cleaned, &bare_prefix, std_path)
            {
                return Some(mapped);
            }
        }
    }

    // Keep non-isomorphic sync wrapper/result surfaces on Rusty paths to avoid
    // introducing invalid std signatures (for example missing lifetimes on guards).
    for (alias, mapped_path, qualified_roots) in [
        (
            "PoisonError",
            "rusty::PoisonError",
            &["rusty::PoisonError"] as &[&str],
        ),
        (
            "PoisonError",
            "rusty::sync::PoisonError",
            &["rusty::sync::PoisonError"] as &[&str],
        ),
        (
            "LockResult",
            "rusty::LockResult",
            &["rusty::LockResult"] as &[&str],
        ),
        (
            "LockResult",
            "rusty::sync::LockResult",
            &["rusty::sync::LockResult"] as &[&str],
        ),
        (
            "TryLockResult",
            "rusty::TryLockResult",
            &["rusty::TryLockResult"] as &[&str],
        ),
        (
            "TryLockResult",
            "rusty::sync::TryLockResult",
            &["rusty::sync::TryLockResult"] as &[&str],
        ),
    ] {
        for root in qualified_roots {
            let qualified_prefix = format!("{}<", root);
            if let Some(mapped) =
                map_single_template_alias_to_std(cleaned, &qualified_prefix, mapped_path)
            {
                return Some(mapped);
            }
        }
        if root_is_unqualified {
            let bare_prefix = format!("{}<", alias);
            if let Some(mapped) =
                map_single_template_alias_to_std(cleaned, &bare_prefix, mapped_path)
            {
                return Some(mapped);
            }
        }
    }

    if root_is_unqualified {
        for (bare_alias_prefix, std_path) in [
            ("Boxed<", "std::boxed::Box"),
            ("Shared<", "std::sync::Arc"),
            ("RefCounted<", "std::rc::Rc"),
        ] {
            if let Some(mapped) =
                map_single_template_alias_to_std(cleaned, bare_alias_prefix, std_path)
            {
                return Some(mapped);
            }
        }
    }

    for (alias, pointer_prefix, qualified_roots) in [
        (
            "Ptr",
            "*const ",
            &["rusty::Ptr", "rusty::ptr::Ptr"] as &[&str],
        ),
        (
            "MutPtr",
            "*mut ",
            &["rusty::MutPtr", "rusty::ptr::MutPtr"] as &[&str],
        ),
    ] {
        for root in qualified_roots {
            let qualified_prefix = format!("{}<", root);
            if let Some(mapped) =
                map_single_template_alias_to_pointer(cleaned, &qualified_prefix, pointer_prefix)
            {
                return Some(mapped);
            }
        }
        if root_is_unqualified {
            let bare_prefix = format!("{}<", alias);
            if let Some(mapped) =
                map_single_template_alias_to_pointer(cleaned, &bare_prefix, pointer_prefix)
            {
                return Some(mapped);
            }
        }
    }

    // Collection aliases may carry comparator/hasher/allocator template args in
    // C++ spellings. Rust std counterparts only keep the primary type params.
    for (alias, std_path, qualified_roots) in [
        (
            "Vec",
            "std::vec::Vec",
            &["rusty::Vec", "rusty::vec::Vec"] as &[&str],
        ),
        (
            "VecDeque",
            "std::collections::VecDeque",
            &["rusty::VecDeque", "rusty::collections::VecDeque"] as &[&str],
        ),
        (
            "HashSet",
            "std::collections::HashSet",
            &["rusty::HashSet", "rusty::collections::HashSet"] as &[&str],
        ),
        (
            "BTreeSet",
            "std::collections::BTreeSet",
            &["rusty::BTreeSet", "rusty::collections::BTreeSet"] as &[&str],
        ),
    ] {
        for root in qualified_roots {
            let qualified_prefix = format!("{}<", root);
            if let Some(mapped) = map_single_template_alias_to_std_allow_extra_args(
                cleaned,
                &qualified_prefix,
                std_path,
            ) {
                return Some(mapped);
            }
        }
        if root_is_unqualified {
            let bare_prefix = format!("{}<", alias);
            if let Some(mapped) =
                map_single_template_alias_to_std_allow_extra_args(cleaned, &bare_prefix, std_path)
            {
                return Some(mapped);
            }
        }
    }

    for (alias, std_path, qualified_roots) in [(
        "Result",
        "std::result::Result",
        &["rusty::Result", "rusty::result::Result"] as &[&str],
    )] {
        for root in qualified_roots {
            let qualified_prefix = format!("{}<", root);
            if let Some(mapped) =
                map_double_template_result_alias_to_std(cleaned, &qualified_prefix, std_path)
            {
                return Some(mapped);
            }
        }
        if root_is_unqualified {
            let bare_prefix = format!("{}<", alias);
            if let Some(mapped) =
                map_double_template_result_alias_to_std(cleaned, &bare_prefix, std_path)
            {
                return Some(mapped);
            }
        }
    }

    for (alias, std_path, qualified_roots) in [
        (
            "HashMap",
            "std::collections::HashMap",
            &["rusty::HashMap", "rusty::collections::HashMap"] as &[&str],
        ),
        (
            "BTreeMap",
            "std::collections::BTreeMap",
            &["rusty::BTreeMap", "rusty::collections::BTreeMap"] as &[&str],
        ),
    ] {
        for root in qualified_roots {
            let qualified_prefix = format!("{}<", root);
            if let Some(mapped) = map_double_template_alias_to_std_allow_extra_args(
                cleaned,
                &qualified_prefix,
                std_path,
            ) {
                return Some(mapped);
            }
        }
        if root_is_unqualified {
            let bare_prefix = format!("{}<", alias);
            if let Some(mapped) =
                map_double_template_alias_to_std_allow_extra_args(cleaned, &bare_prefix, std_path)
            {
                return Some(mapped);
            }
        }
    }

    for prefix in [
        "rusty::ResultVoid<",
        "rusty::result::ResultVoid<",
        "ResultVoid<",
    ] {
        if let Some(inner) = cleaned
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix('>'))
        {
            let args = parse_template_args(inner);
            if args.len() == 1 {
                let ok = CppType::Named(args[0].clone()).to_rust_type_str();
                return Some(format!("std::result::Result<{}, ()>", ok));
            }
        }
    }

    for prefix in [
        "rusty::ResultInt<",
        "rusty::result::ResultInt<",
        "ResultInt<",
    ] {
        if let Some(inner) = cleaned
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix('>'))
        {
            let args = parse_template_args(inner);
            if args.len() == 1 {
                let ok = CppType::Named(args[0].clone()).to_rust_type_str();
                return Some(format!("std::result::Result<{}, i32>", ok));
            }
        }
    }

    for prefix in [
        "rusty::ResultString<",
        "rusty::result::ResultString<",
        "ResultString<",
    ] {
        if let Some(inner) = cleaned
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix('>'))
        {
            let args = parse_template_args(inner);
            if args.len() == 1 {
                let ok = CppType::Named(args[0].clone()).to_rust_type_str();
                return Some(format!("std::result::Result<{}, *const i8>", ok));
            }
        }
    }

    None
}

pub(crate) fn normalize_rusty_type_alias_to_std(spelling: &str) -> String {
    map_rusty_type_to_std(spelling).unwrap_or_else(|| spelling.to_string())
}

/// A C++ type that can be converted to Rust types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CppType {
    /// void
    Void,
    /// bool
    Bool,
    /// char, signed char, unsigned char
    Char { signed: bool },
    /// short, unsigned short
    Short { signed: bool },
    /// int, unsigned int
    Int { signed: bool },
    /// long, unsigned long
    Long { signed: bool },
    /// long long, unsigned long long
    LongLong { signed: bool },
    /// float
    Float,
    /// double
    Double,
    /// Pointer type: T*
    Pointer {
        pointee: Box<CppType>,
        is_const: bool,
    },
    /// Reference type: T& (lvalue) or T&& (rvalue)
    Reference {
        referent: Box<CppType>,
        is_const: bool,
        /// Whether this is an rvalue reference (T&&) vs lvalue reference (T&)
        is_rvalue: bool,
    },
    /// Array type: T[N]
    Array {
        element: Box<CppType>,
        size: Option<usize>,
    },
    /// Named type (struct, class, enum, typedef)
    Named(String),
    /// Function type: R(Args...)
    Function {
        return_type: Box<CppType>,
        params: Vec<CppType>,
        is_variadic: bool,
    },
    /// Template parameter type (used in function/class templates).
    /// Represents a type that will be substituted during template instantiation.
    TemplateParam {
        /// Parameter name (e.g., "T", "U")
        name: String,
        /// Template nesting depth (0 for outermost template)
        depth: u32,
        /// Index in the template parameter list (0-based)
        index: u32,
    },
    /// A dependent type that depends on template parameters.
    /// Used for types like "const T&" where T is a template parameter.
    DependentType {
        /// The base spelling of the type (may contain template param names)
        spelling: String,
    },
    /// A template parameter pack (typename... Args).
    /// Represents a variadic template parameter that can match zero or more types.
    ParameterPack {
        /// Parameter name (e.g., "Args")
        name: String,
        /// Template nesting depth (0 for outermost template)
        depth: u32,
        /// Index in the template parameter list (0-based)
        index: u32,
    },
}

impl CppType {
    /// Create a signed int type.
    pub fn int() -> Self {
        CppType::Int { signed: true }
    }

    /// Create an unsigned int type.
    pub fn uint() -> Self {
        CppType::Int { signed: false }
    }

    /// Create a pointer to this type.
    pub fn ptr(self) -> Self {
        CppType::Pointer {
            pointee: Box::new(self),
            is_const: false,
        }
    }

    /// Create a const pointer to this type.
    pub fn const_ptr(self) -> Self {
        CppType::Pointer {
            pointee: Box::new(self),
            is_const: true,
        }
    }

    /// Get the pointee type for a pointer type.
    /// Returns None if this is not a pointer type.
    pub fn pointee(&self) -> Option<&CppType> {
        match self {
            CppType::Pointer { pointee, .. } => Some(pointee.as_ref()),
            _ => None,
        }
    }

    /// Create an lvalue reference to this type.
    pub fn ref_(self) -> Self {
        CppType::Reference {
            referent: Box::new(self),
            is_const: false,
            is_rvalue: false,
        }
    }

    /// Create a const lvalue reference to this type.
    pub fn const_ref(self) -> Self {
        CppType::Reference {
            referent: Box::new(self),
            is_const: true,
            is_rvalue: false,
        }
    }

    /// Create an rvalue reference to this type.
    pub fn rvalue_ref(self) -> Self {
        CppType::Reference {
            referent: Box::new(self),
            is_const: false,
            is_rvalue: true,
        }
    }

    /// Get the equivalent Rust type name.
    pub fn to_rust_type_str(&self) -> String {
        match self {
            CppType::Void => "()".to_string(),
            CppType::Bool => "bool".to_string(),
            CppType::Char { signed: true } => "i8".to_string(),
            CppType::Char { signed: false } => "u8".to_string(),
            CppType::Short { signed: true } => "i16".to_string(),
            CppType::Short { signed: false } => "u16".to_string(),
            CppType::Int { signed: true } => "i32".to_string(),
            CppType::Int { signed: false } => "u32".to_string(),
            CppType::Long { signed: true } => "i64".to_string(),
            CppType::Long { signed: false } => "u64".to_string(),
            CppType::LongLong { signed: true } => "i64".to_string(),
            CppType::LongLong { signed: false } => "u64".to_string(),
            CppType::Float => "f32".to_string(),
            CppType::Double => "f64".to_string(),
            CppType::Pointer { pointee, is_const } => {
                // Special case: C/C++ function pointers are nullable and use C ABI.
                if let CppType::Function {
                    return_type,
                    params,
                    is_variadic,
                } = pointee.as_ref()
                {
                    let params_str: Vec<_> = params.iter().map(|p| p.to_rust_type_str()).collect();
                    let params_joined = if *is_variadic {
                        format!("{}, ...", params_str.join(", "))
                    } else {
                        params_str.join(", ")
                    };
                    // Use Option to handle nullable function pointers.
                    format!(
                        "Option<extern \"C\" fn({}) -> {}>",
                        params_joined,
                        return_type.to_rust_type_str()
                    )
                } else {
                    // Regular pointer - respect const
                    let ptr_type = if *is_const { "*const" } else { "*mut" };
                    format!("{} {}", ptr_type, pointee.to_rust_type_str())
                }
            }
            CppType::Reference {
                referent,
                is_const,
                is_rvalue: _,
            } => {
                // C++ references map to Rust references for transpilation
                let ref_type = if *is_const { "&" } else { "&mut " };
                format!("{}{}", ref_type, referent.to_rust_type_str())
            }
            CppType::Array { element, size } => {
                if let Some(n) = size {
                    format!("[{}; {}]", element.to_rust_type_str(), n)
                } else {
                    format!("*mut {}", element.to_rust_type_str())
                }
            }
            CppType::Named(name) => {
                // Normalize the name by stripping const/volatile qualifiers for matching
                let normalized_name = name
                    .trim_start_matches("const ")
                    .trim_start_matches("volatile ")
                    .trim();
                if let Some(mapped) = map_rusty_type_to_std(normalized_name) {
                    return mapped;
                }
                // Handle special C++ types that don't map directly to Rust
                match normalized_name {
                    "void" => "std::ffi::c_void".to_string(),
                    "float" => "f32".to_string(),
                    "double" | "long double" => "f64".to_string(), // Rust doesn't have long double
                    "bool" => "bool".to_string(),
                    "long long" | "long long int" | "long_long" | "long_long_int" => {
                        "i64".to_string()
                    }
                    "unsigned long long"
                    | "unsigned long long int"
                    | "unsigned_long_long"
                    | "unsigned_long_long_int" => "u64".to_string(),
                    "long" | "long int" | "long_int" => "i64".to_string(),
                    "unsigned long" | "unsigned long int" | "unsigned_long"
                    | "unsigned_long_int" => "u64".to_string(),
                    "int" => "i32".to_string(),
                    "unsigned" | "unsigned int" => "u32".to_string(),
                    "short" | "short int" => "i16".to_string(),
                    "unsigned short" | "unsigned short int" => "u16".to_string(),
                    "signed char" => "i8".to_string(),
                    "unsigned char" => "u8".to_string(),
                    "char" => "i8".to_string(),
                    "wchar_t" => "i32".to_string(),
                    "char8_t" => "u8".to_string(),
                    "char16_t" => "u16".to_string(),
                    "char32_t" => "u32".to_string(),
                    // Standard library size types (handle both with and without std:: prefix)
                    "size_t" | "std::size_t" => "usize".to_string(),
                    "ssize_t" | "ptrdiff_t" | "std::ptrdiff_t" => "isize".to_string(),
                    "intptr_t" | "std::intptr_t" => "isize".to_string(),
                    "uintptr_t" | "std::uintptr_t" => "usize".to_string(),
                    // Fixed-width integer types from <cstdint>
                    "int8_t" | "std::int8_t" => "i8".to_string(),
                    "int16_t" | "std::int16_t" => "i16".to_string(),
                    "int32_t" | "std::int32_t" => "i32".to_string(),
                    "int64_t" | "std::int64_t" => "i64".to_string(),
                    "uint8_t" | "std::uint8_t" => "u8".to_string(),
                    "uint16_t" | "std::uint16_t" => "u16".to_string(),
                    "uint32_t" | "std::uint32_t" => "u32".to_string(),
                    "uint64_t" | "std::uint64_t" => "u64".to_string(),
                    // 128-bit integer types
                    "__int128" | "__int128_t" => "i128".to_string(),
                    "unsigned __int128" | "__uint128_t" => "u128".to_string(),
                    // C variadic function support
                    "va_list" | "__builtin_va_list" | "__va_list_tag" | "struct __va_list_tag" => {
                        "std::ffi::VaList".to_string()
                    }
                    // C standard I/O
                    "FILE"
                    | "__FILE"
                    | "struct __FILE"
                    | "_IO_FILE"
                    | "struct _IO_FILE"
                    | "__sFILE"
                    | "struct __sFILE"
                    | "std::FILE"
                    | "std::__FILE"
                    | "std::_IO_FILE"
                    | "std::__sFILE"
                    | "std::__1::FILE"
                    | "std::__1::__FILE"
                    | "std::__1::_IO_FILE"
                    | "std::__1::__sFILE"
                    | "std::__2::FILE"
                    | "std::__2::__FILE"
                    | "std::__2::_IO_FILE"
                    | "std::__2::__sFILE"
                    | "std::__ndk1::FILE"
                    | "std::__ndk1::__FILE"
                    | "std::__ndk1::_IO_FILE"
                    | "std::__ndk1::__sFILE" => "std::ffi::c_void".to_string(), // Opaque file handle aliases
                    // nullptr_t type
                    "std::nullptr_t" | "nullptr_t" | "decltype(nullptr)" => {
                        "*mut std::ffi::c_void".to_string()
                    }
                    // Common STL member type aliases used across container types
                    // These are typedefs like vector<T>::size_type that appear in template code
                    "size_type" => "usize".to_string(),
                    "difference_type" => "isize".to_string(),
                    // STL value access types - use c_void as placeholder for generic element type
                    "value_type" => "std::ffi::c_void".to_string(),
                    "reference" | "const_reference" => "&std::ffi::c_void".to_string(),
                    "pointer" | "const_pointer" => "*mut std::ffi::c_void".to_string(),
                    // STL iterator types - use raw pointers as placeholder
                    "iterator" | "const_iterator" => "*mut std::ffi::c_void".to_string(),
                    "reverse_iterator" | "const_reverse_iterator" => {
                        "*mut std::ffi::c_void".to_string()
                    }
                    // Allocator types
                    "allocator_type" => "std::ffi::c_void".to_string(),
                    // Common template parameter names that appear unresolved
                    // Use () instead of c_void so they're valid as bare types (params, return types)
                    "_Tp" | "_CharT" | "_Traits" | "_Allocator" | "_Alloc" => "()".to_string(),
                    "_Pointer" | "_Iter" | "_Iterator" | "_Size" | "_Ep" => "()".to_string(),
                    "_Rp" | "_Ip" | "_Container" | "_BaseT" | "_It" | "_CP" => "()".to_string(),
                    "_Gen" | "_Func" | "_Rollback" | "_StorageAlloc" => "()".to_string(),
                    "_ControlBlockAlloc" | "_ControlBlockAllocator" => "()".to_string(),
                    "_Sp" | "_Dp" | "_Up" | "_Yp" => "()".to_string(), // Smart pointer params
                    // libstdc++ bit vector internal types
                    "_Bit_type" => "u64".to_string(), // Typically unsigned long
                    "_Tp_alloc_type" | "_Bit_alloc_type" => "()".to_string(), // Allocator type alias
                    // Smart pointer internal types
                    "_Sp___rep" => "()".to_string(), // shared_ptr refcount
                    // Dependent types from templates
                    "_dependent_type" => "()".to_string(),
                    // libstdc++ comparison category types
                    "__cmp_cat_type" | "__cmp_cat__Ord" | "__cmp_cat__Ncmp" => "i8".to_string(),
                    "__cmp_cat___unspec" => "i8".to_string(),
                    // libc++ internal proxy and impl types
                    "__proxy" | "__value_type" => "std::ffi::c_void".to_string(),
                    "std___libcpp_refstring" => "std::ffi::c_void".to_string(),
                    // libc++ identity projection helper can appear both namespaced and pre-sanitized.
                    "std___identity"
                    | "std::__identity"
                    | "std::__1::__identity"
                    | "std::__2::__identity"
                    | "std::__ndk1::__identity" => "__identity".to_string(),
                    // libc++ internal bool atomic base alias can be referenced by generated call-shapes.
                    "__cxx_atomic_base_impl_bool"
                    | "std::__cxx_atomic_base_impl_bool"
                    | "std::__1::__cxx_atomic_base_impl_bool"
                    | "std::__2::__cxx_atomic_base_impl_bool"
                    | "std::__ndk1::__cxx_atomic_base_impl_bool" => {
                        "__cxx_atomic_impl_bool".to_string()
                    }
                    // Stream types
                    "__stream_type" | "ostream_type" | "istream_type" => {
                        "std::ffi::c_void".to_string()
                    }
                    "fmtflags" => "u32".to_string(), // ios_base::fmtflags is an integer type
                    // Optional type
                    "nullopt_t" => "()".to_string(),
                    // Time value types
                    "timeval" => "timeval".to_string(),
                    // libc++ internal string representation types
                    "__long" | "__rep" | "rep" => "std::ffi::c_void".to_string(),
                    // Duration types
                    "duration" => "i64".to_string(),
                    // C++17 std::byte - map to the generated byte enum (without std:: prefix)
                    "std::byte" => "byte".to_string(),
                    // C++11 memory_order - map to the generated memory_order enum
                    "std::memory_order" => "memory_order".to_string(),
                    // tinyxml2 namespaced enums are also auto-exported as top-level aliases.
                    // Normalize these spellings so call/return typing stays consistent.
                    "tinyxml2::XMLError" | "tinyxml2_XMLError" => "XMLError".to_string(),
                    "tinyxml2::Whitespace" | "tinyxml2_Whitespace" => "Whitespace".to_string(),
                    "tinyxml2::XMLElement::ElementClosingType"
                    | "tinyxml2_XMLElement_ElementClosingType" => "ElementClosingType".to_string(),
                    "tinyxml2::StrPair::Mode" | "tinyxml2_StrPair_Mode" => "Mode".to_string(),
                    // RapidJSON helper signatures can surface allocator-qualified StringBuffer
                    // spellings while placeholder structs are emitted as GenericStringBuffer_UTF8.
                    "GenericStringBuffer_UTF8_char__CrtAllocator"
                    | "rapidjson::GenericStringBuffer<rapidjson::UTF8<char>, rapidjson::CrtAllocator>"
                    | "rapidjson::GenericStringBuffer<struct rapidjson::UTF8<>, class rapidjson::CrtAllocator>"
                    | "rapidjson::GenericStringBuffer<rapidjson::UTF8<>, rapidjson::CrtAllocator>"
                    | "rapidjson::GenericStringBuffer<UTF8<char>, CrtAllocator>"
                    | "rapidjson::GenericStringBuffer<UTF8<>, CrtAllocator>" => {
                        "GenericStringBuffer_UTF8".to_string()
                    }
                    // C++11 chars_format (from <charconv>)
                    "std::chars_format" => "u32".to_string(), // Flags enum, treat as u32
                    // iostream base types
                    "std::ios_base::fmtflags" => "u32".to_string(),
                    "std::ios_base::iostate" => "u32".to_string(),
                    "std::ios_base::openmode" => "u32".to_string(),
                    // codecvt types
                    "std::codecvt_base::result" | "codecvt_base::result" => "i32".to_string(),
                    // libc++ internal string types
                    "__self_view" => "std::ffi::c_void".to_string(),
                    "string" | "std::string" => "std_string".to_string(),
                    "__storage_pointer" => "*mut std::ffi::c_void".to_string(),
                    // Allocator-related types that appear in container implementations
                    "__alloc_traits_difference_type" => "isize".to_string(),
                    // libc++ internal types
                    "__syscall_slong_t" | "__syscall_ulong_t" => "i64".to_string(),
                    "__type_name_t" => "*const i8".to_string(), // RTTI type name pointer
                    // Boolean type traits used for tag dispatching
                    "true_type" | "std::true_type" => "bool".to_string(),
                    "false_type" | "std::false_type" => "bool".to_string(),
                    // C++ exception types - pass through as proper types
                    // Previously mapped to c_void, but this breaks inheritance (bad_alloc : exception)
                    // Now these types are generated as proper structs with inheritance fields
                    "logic_error" | "std::logic_error" => "logic_error".to_string(),
                    "runtime_error" | "std::runtime_error" => "runtime_error".to_string(),
                    "bad_alloc" | "std::bad_alloc" => "bad_alloc".to_string(),
                    "exception" | "std::exception" => "exception".to_string(),
                    // Time and stream types
                    "timespec" => "timespec".to_string(),
                    "streambuf_type" | "char_type" => "std::ffi::c_void".to_string(),
                    "memory_resource" => "std::ffi::c_void".to_string(),
                    // More template parameter placeholders
                    "_ValueType" | "_Sent" | "_Hp" => "()".to_string(),
                    "__storage_type" => "usize".to_string(),
                    // NOTE: STL string type mappings removed - types pass through as-is
                    // See Section 22 in TODO.md for rationale
                    _ => {
                        // NOTE: std::vector<T> mapping removed - let types go through
                        // normal conversion to generate unique struct per instantiation
                        // e.g., std::vector<int> -> std_vector_int_ (fixed in sanitize step)

                        // Map std::_Bit_iterator to _Bit_iterator (strip std:: prefix)
                        if normalized_name == "std::_Bit_iterator" {
                            return "_Bit_iterator".to_string();
                        }
                        if normalized_name == "std::_Bit_const_iterator" {
                            return "_Bit_const_iterator".to_string();
                        }
                        // NOTE: STL type mappings removed - types pass through as-is
                        // std::vector, std::string, std::optional, std::array, std::span
                        // See Section 22 in TODO.md for rationale
                        // NOTE: std::map and std::unordered_map mappings removed - types pass through as-is
                        // See Section 22 in TODO.md for rationale
                        // NOTE: All remaining STL mappings removed - types pass through as-is
                        // smart pointers, I/O streams, std::variant
                        // See Section 22 in TODO.md for rationale
                        // Handle decltype expressions - replace with unit type placeholder
                        if name.starts_with("decltype(") {
                            return "()".to_string();
                        }
                        // Handle typeof expressions similarly
                        if name.starts_with("typeof(") || name.starts_with("__typeof__(") {
                            return "()".to_string();
                        }
                        // Handle lambda types - use inference placeholder
                        // Lambda types look like "(lambda at /path/file.cpp:line:col)"
                        if name.starts_with("(lambda at ") || name.contains("lambda at ") {
                            return "_".to_string(); // Let Rust infer the closure type
                        }
                        // Handle auto type (C++11) - use Rust type inference
                        if name == "auto" {
                            return "_".to_string();
                        }
                        // Handle "type" which is a Rust keyword - can appear from unresolved
                        // template parameters or typename expressions
                        if normalized_name == "type" {
                            return "()".to_string();
                        }
                        // Handle Clang template parameter placeholders like type-parameter-0-0
                        // These are unresolved template parameters from template definitions
                        // Note: Use normalized_name to handle const-qualified types
                        if normalized_name.starts_with("type-parameter-")
                            || normalized_name.starts_with("type_parameter_")
                        {
                            return "()".to_string();
                        }
                        // Handle complex conditional types from libc++ template metaprogramming
                        // These are SFINAE/conditional type expressions that can't be represented
                        // Check both the template form (_If<...>) and sanitized form (_If_...)
                        if normalized_name.starts_with("__conditional_t")
                            || normalized_name.starts_with("_If<")  // Original template form
                            || normalized_name.starts_with("_If_")  // Sanitized form
                            || normalized_name.contains("__conditional_t")
                        // Also catch it in middle
                        {
                            return "()".to_string();
                        }
                        // Handle typename-prefixed dependent types
                        if normalized_name.starts_with("typename")
                            || normalized_name.starts_with("typename_")
                        {
                            return "()".to_string();
                        }
                        // Handle libc++ variant implementation detail types
                        if normalized_name.starts_with("__variant_detail") {
                            return "()".to_string();
                        }
                        // Handle iterator traits types
                        if normalized_name.starts_with("iter_") {
                            return "()".to_string();
                        }
                        // Handle type trait result types (add_pointer_t, make_unsigned_t, etc.)
                        if normalized_name.starts_with("add_pointer_t")
                            || normalized_name.starts_with("make_unsigned_t")
                            || normalized_name.starts_with("sentinel_t")
                            || normalized_name.starts_with("iterator_t")
                            || normalized_name.starts_with("__insert_iterator")
                            || normalized_name.starts_with("__impl_")
                            || normalized_name.starts_with("__add_lvalue_reference")
                        {
                            return "()".to_string();
                        }
                        // libc++ functional hash internals use anonymous helper structs
                        // in union fields. Treat them as opaque 64-bit payloads.
                        let functional_hash_helper_name = normalized_name
                            .trim_start_matches("struct ")
                            .trim_start_matches("class ")
                            .trim();
                        if functional_hash_helper_name.starts_with(
                            "_unnamed_struct_at__home_shuai_workspace_fragile_vendor_llvm_project_libcxx_include___functional_hash_h_",
                        ) {
                            return "u64".to_string();
                        }
                        // Strip C++ qualifiers that aren't valid in Rust type names
                        let cleaned = name
                            .trim_start_matches("const ")
                            .trim_start_matches("volatile ")
                            .trim_start_matches("struct ")
                            .trim_start_matches("class ")
                            .trim_start_matches("enum ")
                            .trim_end(); // Remove trailing whitespace

                        // Strip inline namespace versioning used by libc++ (e.g., std::__1:: -> std::)
                        // libc++ uses __1, __2, etc. as ABI versioning namespaces
                        let cleaned = cleaned
                            .replace("::__1::", "::")
                            .replace("::__2::", "::")
                            .replace("::__ndk1::", "::"); // Android NDK uses __ndk1

                        // Handle remaining "unsigned TYPE" patterns
                        let cleaned: String = if cleaned.starts_with("unsigned ") {
                            match cleaned.trim_start_matches("unsigned ") {
                                "int" => "u32".to_string(),
                                "long" => "u64".to_string(),
                                "short" => "u16".to_string(),
                                "char" => "u8".to_string(),
                                _ => cleaned.clone(),
                            }
                        } else if cleaned.starts_with("signed ") {
                            match cleaned.trim_start_matches("signed ") {
                                "int" => "i32".to_string(),
                                "long" => "i64".to_string(),
                                "short" => "i16".to_string(),
                                "char" => "i8".to_string(),
                                _ => cleaned.clone(),
                            }
                        } else {
                            cleaned
                        };

                        // Handle C++ array types that appear as Named types with bracket notation
                        // e.g., _Tp[_Size] -> [_Tp; _Size] (Rust array syntax)
                        // This happens with template parameters like std::array's __elems_ field
                        // BUT skip if the result would be used as a struct name (no size specified)
                        // e.g., type-parameter-0-0[] should become type_parameter_0_0_Arr not [type; ]
                        if let Some(bracket_idx) = cleaned.find('[') {
                            let element_type = &cleaned[..bracket_idx];
                            let rest = &cleaned[bracket_idx + 1..];
                            if let Some(close_bracket) = rest.find(']') {
                                let size = &rest[..close_bracket].trim();
                                // Only convert to array syntax if size is non-empty (looks like actual array)
                                if !size.is_empty()
                                    && element_type
                                        .chars()
                                        .all(|c| c.is_alphanumeric() || c == '_')
                                {
                                    // Recursively convert the element type and size
                                    let elem_rust =
                                        CppType::Named(element_type.to_string()).to_rust_type_str();
                                    let size_rust = normalize_array_size_expr(size);
                                    return format!("[{}; {}]", elem_rust, size_rust);
                                }
                                // Empty size like T[] - just convert to Arr suffix
                                // This is used for unique_ptr<T[]> style types
                            }
                        }

                        // Replace :: with _ for namespaced types
                        // Convert template syntax to valid Rust identifiers:
                        // e.g., std::vector<int> -> std_vector_int
                        // e.g., type-parameter-0-0 -> type_parameter_0_0
                        // Note: replace "::" first, then single ":" for line:col references
                        let result = cleaned
                            .replace("::", "_")
                            .replace(":", "_") // Single colon in file line:col references
                            .replace("<", "_") // Convert template open bracket
                            .replace(">", "") // Remove template close bracket
                            .replace(",", "_") // Handle multiple template params
                            .replace(" *", "") // Remove trailing pointer indicators
                            .replace("*", "")
                            .replace("&&", "_") // C++ rvalue reference in type names
                            .replace("&", "_") // C++ reference in type names
                            .replace("[]", "_Arr") // Array type notation (e.g., T[] -> T_Arr)
                            .replace("[", "_") // Expression grouping
                            .replace("]", "_") // Expression grouping
                            .replace(" ", "_")
                            .replace("-", "_") // Clang uses dashes in template param names
                            .replace(".", "_") // Variadic pack expansion uses ...
                            .replace("+", "_") // Template expressions (Index + 1)
                            .replace("'", "_") // Char-literal template args (e.g., '_')
                            .replace("(", "_") // Expression grouping
                            .replace(")", "_")
                            .replace("/", "_") // File paths in anonymous union names from system headers
                            .replace("==", "_eq_") // C++ SFINAE template expressions
                            .replace("!=", "_ne_") // C++ SFINAE template expressions
                            .replace("!", "_not_") // C++ SFINAE negation
                            .replace("=", "_") // Assignment/equality leftovers in dependent type spellings
                            .replace("?", "_cond_") // C++ ternary/conditional in template expressions
                            .replace("{", "_") // C++ initializer list / pack expansion
                            .replace("}", "_"); // C++ initializer list / pack expansion

                        // Log diagnostic for complex type transformations
                        if result != cleaned && (cleaned.contains('<') || cleaned.contains("::")) {
                            log_type_diagnostic(
                                "conversion",
                                &format!("'{}' -> '{}'", cleaned, result),
                            );
                        }

                        result
                    }
                }
            }
            CppType::Function {
                return_type,
                params,
                is_variadic,
            } => {
                let params_str: Vec<_> = params.iter().map(|p| p.to_rust_type_str()).collect();
                let params_joined = if *is_variadic {
                    format!("{}, ...", params_str.join(", "))
                } else {
                    params_str.join(", ")
                };
                format!(
                    "extern \"C\" fn({}) -> {}",
                    params_joined,
                    return_type.to_rust_type_str()
                )
            }
            CppType::TemplateParam { name, .. } => {
                // Template parameters are represented by their name
                // In Rust generics, this would be a generic type parameter
                name.clone()
            }
            CppType::DependentType { spelling } => {
                // Dependent types are preserved as their spelling
                // These need to be resolved during template instantiation
                spelling.clone()
            }
            CppType::ParameterPack { name, .. } => {
                // Parameter packs need special handling during expansion
                // For now, represent as the pack name with ... suffix
                format!("{}...", name)
            }
        }
    }

    /// Convert to Rust type string suitable for struct fields.
    /// References are converted to raw pointers since Rust struct fields
    /// can't have references without explicit lifetime parameters.
    pub fn to_rust_type_str_for_field(&self) -> String {
        match self {
            CppType::Reference {
                referent, is_const, ..
            } => {
                // Convert references to raw pointers for struct fields
                let ptr_type = if *is_const { "*const" } else { "*mut" };
                format!("{} {}", ptr_type, referent.to_rust_type_str_for_field())
            }
            _ => self.to_rust_type_str(),
        }
    }

    /// Check if this type is or contains template parameters.
    pub fn is_dependent(&self) -> bool {
        match self {
            CppType::TemplateParam { .. }
            | CppType::DependentType { .. }
            | CppType::ParameterPack { .. } => true,
            CppType::Pointer { pointee, .. } => pointee.is_dependent(),
            CppType::Reference { referent, .. } => referent.is_dependent(),
            CppType::Array { element, .. } => element.is_dependent(),
            CppType::Function {
                return_type,
                params,
                ..
            } => return_type.is_dependent() || params.iter().any(|p| p.is_dependent()),
            _ => false,
        }
    }

    /// Create a template parameter type.
    pub fn template_param(name: &str, depth: u32, index: u32) -> Self {
        CppType::TemplateParam {
            name: name.to_string(),
            depth,
            index,
        }
    }

    /// Create a template parameter pack type.
    pub fn parameter_pack(name: &str, depth: u32, index: u32) -> Self {
        CppType::ParameterPack {
            name: name.to_string(),
            depth,
            index,
        }
    }

    /// Check if this type is a parameter pack.
    pub fn is_parameter_pack(&self) -> bool {
        matches!(self, CppType::ParameterPack { .. })
    }

    /// Substitute template parameters with concrete types.
    ///
    /// Given a mapping of template parameter names to concrete types,
    /// returns a new type with all template parameters replaced.
    ///
    /// # Example
    /// ```ignore
    /// // T* with T = int becomes int*
    /// let ty = CppType::Pointer { pointee: CppType::TemplateParam { name: "T", ... } };
    /// let subst = HashMap::from([("T".to_string(), CppType::Int { signed: true })]);
    /// let result = ty.substitute(&subst); // int*
    /// ```
    pub fn substitute(
        &self,
        substitutions: &std::collections::HashMap<String, CppType>,
    ) -> CppType {
        match self {
            CppType::TemplateParam { name, .. } => substitutions
                .get(name)
                .cloned()
                .unwrap_or_else(|| self.clone()),
            CppType::DependentType { spelling } => {
                // Try to find a template param in the spelling and substitute
                // This is a simplified approach
                if let Some(replacement) = substitutions.get(spelling) {
                    replacement.clone()
                } else {
                    self.clone()
                }
            }
            CppType::ParameterPack { name, .. } => {
                // Parameter packs require special expansion logic.
                // For now, if a single type is provided, use it directly.
                // Full pack expansion is more complex and handled elsewhere.
                substitutions
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| self.clone())
            }
            CppType::Pointer { pointee, is_const } => CppType::Pointer {
                pointee: Box::new(pointee.substitute(substitutions)),
                is_const: *is_const,
            },
            CppType::Reference {
                referent,
                is_const,
                is_rvalue,
            } => CppType::Reference {
                referent: Box::new(referent.substitute(substitutions)),
                is_const: *is_const,
                is_rvalue: *is_rvalue,
            },
            CppType::Array { element, size } => CppType::Array {
                element: Box::new(element.substitute(substitutions)),
                size: *size,
            },
            CppType::Function {
                return_type,
                params,
                is_variadic,
            } => CppType::Function {
                return_type: Box::new(return_type.substitute(substitutions)),
                params: params.iter().map(|p| p.substitute(substitutions)).collect(),
                is_variadic: *is_variadic,
            },
            // Non-dependent types remain unchanged
            _ => self.clone(),
        }
    }

    /// Get the type properties for SFINAE/type trait evaluation.
    /// Returns None for dependent types (template parameters).
    pub fn properties(&self) -> Option<TypeProperties> {
        match self {
            // Template parameters have unknown properties
            CppType::TemplateParam { .. }
            | CppType::DependentType { .. }
            | CppType::ParameterPack { .. } => None,

            CppType::Void => Some(TypeProperties {
                is_integral: false,
                is_signed: false,
                is_floating_point: false,
                is_scalar: false,
                is_pointer: false,
                is_reference: false,
                is_trivially_copyable: true,
                is_trivially_destructible: true,
            }),

            CppType::Bool => Some(TypeProperties {
                is_integral: true,
                is_signed: false,
                is_floating_point: false,
                is_scalar: true,
                is_pointer: false,
                is_reference: false,
                is_trivially_copyable: true,
                is_trivially_destructible: true,
            }),

            CppType::Char { signed } => Some(TypeProperties {
                is_integral: true,
                is_signed: *signed,
                is_floating_point: false,
                is_scalar: true,
                is_pointer: false,
                is_reference: false,
                is_trivially_copyable: true,
                is_trivially_destructible: true,
            }),

            CppType::Short { signed }
            | CppType::Int { signed }
            | CppType::Long { signed }
            | CppType::LongLong { signed } => Some(TypeProperties {
                is_integral: true,
                is_signed: *signed,
                is_floating_point: false,
                is_scalar: true,
                is_pointer: false,
                is_reference: false,
                is_trivially_copyable: true,
                is_trivially_destructible: true,
            }),

            CppType::Float | CppType::Double => Some(TypeProperties {
                is_integral: false,
                is_signed: true, // Floating point types are always signed
                is_floating_point: true,
                is_scalar: true,
                is_pointer: false,
                is_reference: false,
                is_trivially_copyable: true,
                is_trivially_destructible: true,
            }),

            CppType::Pointer { .. } => Some(TypeProperties {
                is_integral: false,
                is_signed: false,
                is_floating_point: false,
                is_scalar: true,
                is_pointer: true,
                is_reference: false,
                is_trivially_copyable: true,
                is_trivially_destructible: true,
            }),

            CppType::Reference { .. } => Some(TypeProperties {
                is_integral: false,
                is_signed: false,
                is_floating_point: false,
                is_scalar: false,
                is_pointer: false,
                is_reference: true,
                is_trivially_copyable: false,
                is_trivially_destructible: true,
            }),

            CppType::Array { .. } => Some(TypeProperties {
                is_integral: false,
                is_signed: false,
                is_floating_point: false,
                is_scalar: false,
                is_pointer: false,
                is_reference: false,
                // Arrays of trivially copyable types are trivially copyable
                is_trivially_copyable: false, // Conservative default
                is_trivially_destructible: true,
            }),

            CppType::Named(_) => Some(TypeProperties {
                is_integral: false,
                is_signed: false,
                is_floating_point: false,
                is_scalar: false,
                is_pointer: false,
                is_reference: false,
                // Named types need lookup to determine properties
                is_trivially_copyable: false,     // Conservative default
                is_trivially_destructible: false, // Conservative default
            }),

            CppType::Function { .. } => Some(TypeProperties {
                is_integral: false,
                is_signed: false,
                is_floating_point: false,
                is_scalar: false,
                is_pointer: false,
                is_reference: false,
                is_trivially_copyable: false,
                is_trivially_destructible: true,
            }),
        }
    }

    /// Check if this is an integral type (bool, char, short, int, long, long long).
    pub fn is_integral(&self) -> Option<bool> {
        self.properties().map(|p| p.is_integral)
    }

    /// Check if this is a signed type.
    pub fn is_signed(&self) -> Option<bool> {
        self.properties().map(|p| p.is_signed)
    }

    /// Check if this is a scalar type (arithmetic types, pointers, enum types).
    pub fn is_scalar(&self) -> Option<bool> {
        self.properties().map(|p| p.is_scalar)
    }

    /// Check if this is a floating point type (float, double).
    pub fn is_floating_point(&self) -> Option<bool> {
        self.properties().map(|p| p.is_floating_point)
    }

    /// Check if this is an arithmetic type (integral or floating point).
    pub fn is_arithmetic(&self) -> Option<bool> {
        self.properties()
            .map(|p| p.is_integral || p.is_floating_point)
    }

    /// Get the bit width of this type.
    ///
    /// Returns None for types that don't have a fixed bit width (named types,
    /// dependent types, function types, etc.).
    ///
    /// Assumes LP64 data model (common on 64-bit Unix):
    /// - char: 8 bits
    /// - short: 16 bits
    /// - int: 32 bits
    /// - long: 64 bits
    /// - long long: 64 bits
    pub fn bit_width(&self) -> Option<u32> {
        match self {
            CppType::Bool => Some(8), // Rust bool is 1 byte for FFI compatibility
            CppType::Char { .. } => Some(8),
            CppType::Short { .. } => Some(16),
            CppType::Int { .. } => Some(32),
            CppType::Long { .. } => Some(64), // LP64 model
            CppType::LongLong { .. } => Some(64),
            CppType::Float => Some(32),
            CppType::Double => Some(64),
            CppType::Pointer { .. } => Some(64), // 64-bit pointers
            CppType::Reference { .. } => Some(64), // References are pointer-sized
            // Types without fixed bit width
            CppType::Void
            | CppType::Array { .. }
            | CppType::Named(_)
            | CppType::Function { .. }
            | CppType::TemplateParam { .. }
            | CppType::DependentType { .. }
            | CppType::ParameterPack { .. } => None,
        }
    }
}

/// Type properties for SFINAE and type trait evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeProperties {
    /// True for bool, char, short, int, long, long long (signed or unsigned)
    pub is_integral: bool,
    /// True for signed types, false for unsigned
    pub is_signed: bool,
    /// True for float, double, long double
    pub is_floating_point: bool,
    /// True for arithmetic types and pointers
    pub is_scalar: bool,
    /// True for pointer types
    pub is_pointer: bool,
    /// True for reference types (lvalue or rvalue)
    pub is_reference: bool,
    /// True if the type can be safely memcpy'd
    pub is_trivially_copyable: bool,
    /// True if the destructor is trivial
    pub is_trivially_destructible: bool,
}

/// Type trait evaluation results.
/// Used for evaluating Clang's built-in type traits like __is_integral(T).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeTraitResult {
    /// The trait evaluates to a known boolean value
    Value(bool),
    /// The trait cannot be evaluated (e.g., depends on template parameters)
    Dependent,
}

impl TypeTraitResult {
    /// Returns true if this result is a definite true value.
    pub fn is_true(&self) -> bool {
        matches!(self, TypeTraitResult::Value(true))
    }

    /// Returns true if this result is a definite false value.
    pub fn is_false(&self) -> bool {
        matches!(self, TypeTraitResult::Value(false))
    }

    /// Returns true if the result depends on template parameters.
    pub fn is_dependent(&self) -> bool {
        matches!(self, TypeTraitResult::Dependent)
    }

    /// Get the boolean value if known, None if dependent.
    pub fn to_bool(&self) -> Option<bool> {
        match self {
            TypeTraitResult::Value(v) => Some(*v),
            TypeTraitResult::Dependent => None,
        }
    }
}

/// Evaluates type traits against concrete or dependent types.
pub struct TypeTraitEvaluator;

impl TypeTraitEvaluator {
    /// Evaluate __is_integral(T)
    pub fn is_integral(ty: &CppType) -> TypeTraitResult {
        match ty.is_integral() {
            Some(v) => TypeTraitResult::Value(v),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_signed(T)
    pub fn is_signed(ty: &CppType) -> TypeTraitResult {
        match ty.is_signed() {
            Some(v) => TypeTraitResult::Value(v),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_unsigned(T)
    pub fn is_unsigned(ty: &CppType) -> TypeTraitResult {
        match ty.is_signed() {
            Some(signed) => TypeTraitResult::Value(!signed),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_floating_point(T)
    pub fn is_floating_point(ty: &CppType) -> TypeTraitResult {
        match ty.is_floating_point() {
            Some(v) => TypeTraitResult::Value(v),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_arithmetic(T)
    pub fn is_arithmetic(ty: &CppType) -> TypeTraitResult {
        match ty.is_arithmetic() {
            Some(v) => TypeTraitResult::Value(v),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_scalar(T)
    pub fn is_scalar(ty: &CppType) -> TypeTraitResult {
        match ty.is_scalar() {
            Some(v) => TypeTraitResult::Value(v),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_pointer(T)
    pub fn is_pointer(ty: &CppType) -> TypeTraitResult {
        match ty.properties() {
            Some(p) => TypeTraitResult::Value(p.is_pointer),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_reference(T)
    pub fn is_reference(ty: &CppType) -> TypeTraitResult {
        match ty.properties() {
            Some(p) => TypeTraitResult::Value(p.is_reference),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_same(T, U)
    pub fn is_same(ty1: &CppType, ty2: &CppType) -> TypeTraitResult {
        // If either type is dependent, result is dependent
        if ty1.is_dependent() || ty2.is_dependent() {
            return TypeTraitResult::Dependent;
        }
        TypeTraitResult::Value(ty1 == ty2)
    }

    /// Evaluate __is_trivially_copyable(T)
    pub fn is_trivially_copyable(ty: &CppType) -> TypeTraitResult {
        match ty.properties() {
            Some(p) => TypeTraitResult::Value(p.is_trivially_copyable),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_trivially_destructible(T)
    pub fn is_trivially_destructible(ty: &CppType) -> TypeTraitResult {
        match ty.properties() {
            Some(p) => TypeTraitResult::Value(p.is_trivially_destructible),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_base_of(Base, Derived)
    /// Note: This requires class hierarchy information which we don't have yet.
    /// For now, returns Dependent for named types.
    pub fn is_base_of(base: &CppType, derived: &CppType) -> TypeTraitResult {
        // If either type is dependent, result is dependent
        if base.is_dependent() || derived.is_dependent() {
            return TypeTraitResult::Dependent;
        }

        // If types are the same, a class is considered a base of itself
        if base == derived {
            return TypeTraitResult::Value(true);
        }

        // For Named types, we would need class hierarchy information
        // For now, return Dependent to indicate we can't evaluate this
        match (base, derived) {
            (CppType::Named(_), CppType::Named(_)) => TypeTraitResult::Dependent,
            // Non-class types: false (not a class hierarchy relationship)
            _ => TypeTraitResult::Value(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_width_primitive_types() {
        // Bool
        assert_eq!(CppType::Bool.bit_width(), Some(8));

        // Char
        assert_eq!(CppType::Char { signed: true }.bit_width(), Some(8));
        assert_eq!(CppType::Char { signed: false }.bit_width(), Some(8));

        // Short
        assert_eq!(CppType::Short { signed: true }.bit_width(), Some(16));
        assert_eq!(CppType::Short { signed: false }.bit_width(), Some(16));

        // Int
        assert_eq!(CppType::Int { signed: true }.bit_width(), Some(32));
        assert_eq!(CppType::Int { signed: false }.bit_width(), Some(32));

        // Long (LP64 model)
        assert_eq!(CppType::Long { signed: true }.bit_width(), Some(64));
        assert_eq!(CppType::Long { signed: false }.bit_width(), Some(64));

        // Long Long
        assert_eq!(CppType::LongLong { signed: true }.bit_width(), Some(64));
        assert_eq!(CppType::LongLong { signed: false }.bit_width(), Some(64));

        // Float/Double
        assert_eq!(CppType::Float.bit_width(), Some(32));
        assert_eq!(CppType::Double.bit_width(), Some(64));
    }

    #[test]
    fn test_bit_width_pointer_and_reference() {
        // Pointers are 64-bit on LP64
        let ptr = CppType::Pointer {
            pointee: Box::new(CppType::Int { signed: true }),
            is_const: false,
        };
        assert_eq!(ptr.bit_width(), Some(64));

        // References are also pointer-sized
        let ref_ = CppType::Reference {
            referent: Box::new(CppType::Int { signed: true }),
            is_const: false,
            is_rvalue: false,
        };
        assert_eq!(ref_.bit_width(), Some(64));
    }

    #[test]
    fn test_bit_width_no_fixed_width() {
        // Void
        assert_eq!(CppType::Void.bit_width(), None);

        // Named types
        assert_eq!(CppType::Named("Foo".to_string()).bit_width(), None);

        // Template parameters
        let tp = CppType::TemplateParam {
            name: "T".to_string(),
            depth: 0,
            index: 0,
        };
        assert_eq!(tp.bit_width(), None);
    }

    #[test]
    fn test_is_signed_integer_types() {
        // Signed types return Some(true)
        assert_eq!(CppType::Char { signed: true }.is_signed(), Some(true));
        assert_eq!(CppType::Short { signed: true }.is_signed(), Some(true));
        assert_eq!(CppType::Int { signed: true }.is_signed(), Some(true));
        assert_eq!(CppType::Long { signed: true }.is_signed(), Some(true));
        assert_eq!(CppType::LongLong { signed: true }.is_signed(), Some(true));

        // Unsigned types return Some(false)
        assert_eq!(CppType::Char { signed: false }.is_signed(), Some(false));
        assert_eq!(CppType::Short { signed: false }.is_signed(), Some(false));
        assert_eq!(CppType::Int { signed: false }.is_signed(), Some(false));
        assert_eq!(CppType::Long { signed: false }.is_signed(), Some(false));
        assert_eq!(CppType::LongLong { signed: false }.is_signed(), Some(false));

        // Bool is unsigned
        assert_eq!(CppType::Bool.is_signed(), Some(false));

        // Floating point is signed
        assert_eq!(CppType::Float.is_signed(), Some(true));
        assert_eq!(CppType::Double.is_signed(), Some(true));
    }

    #[test]
    fn test_smart_pointer_type_mappings() {
        // NOTE: Smart pointer mappings removed - types pass through as-is
        // See Section 22 in TODO.md for rationale
        // Template syntax converted to valid Rust identifiers

        // std::unique_ptr<T> passes through (no longer mapped to Box<T>)
        assert_eq!(
            CppType::Named("std::unique_ptr<int>".to_string()).to_rust_type_str(),
            "std_unique_ptr_int"
        );
        assert_eq!(
            CppType::Named("std::unique_ptr<int, std::default_delete<int>>".to_string())
                .to_rust_type_str(),
            "std_unique_ptr_int__std_default_delete_int"
        );
        assert_eq!(
            CppType::Named("std::unique_ptr<MyClass>".to_string()).to_rust_type_str(),
            "std_unique_ptr_MyClass"
        );

        // __detail::__unique_ptr_t<T> passes through
        assert_eq!(
            CppType::Named("__detail::__unique_ptr_t<int>".to_string()).to_rust_type_str(),
            "__detail___unique_ptr_t_int"
        );

        // std::shared_ptr<T> passes through (no longer mapped to Arc<T>)
        assert_eq!(
            CppType::Named("std::shared_ptr<int>".to_string()).to_rust_type_str(),
            "std_shared_ptr_int"
        );
        assert_eq!(
            CppType::Named("std::shared_ptr<MyClass>".to_string()).to_rust_type_str(),
            "std_shared_ptr_MyClass"
        );

        // shared_ptr<_NonArray<T>> passes through
        assert_eq!(
            CppType::Named("shared_ptr<_NonArray<int>>".to_string()).to_rust_type_str(),
            "shared_ptr__NonArray_int"
        );

        // std::weak_ptr<T> passes through (no longer mapped to Weak<T>)
        assert_eq!(
            CppType::Named("std::weak_ptr<int>".to_string()).to_rust_type_str(),
            "std_weak_ptr_int"
        );
        assert_eq!(
            CppType::Named("std::weak_ptr<MyClass>".to_string()).to_rust_type_str(),
            "std_weak_ptr_MyClass"
        );
    }

    #[test]
    fn test_rusty_cpp_types_map_to_std_library_types() {
        assert_eq!(
            CppType::Named("rusty::RefCell<int>".to_string()).to_rust_type_str(),
            "std::cell::RefCell<i32>"
        );
        assert_eq!(
            CppType::Named("rusty::HashMap<int, long>".to_string()).to_rust_type_str(),
            "std::collections::HashMap<i32, i64>"
        );
        assert_eq!(
            CppType::Named("rusty::Option<rusty::String>".to_string()).to_rust_type_str(),
            "std::option::Option<std::string::String>"
        );
        assert_eq!(
            CppType::Named("rusty::Result<int, rusty::String>".to_string()).to_rust_type_str(),
            "std::result::Result<i32, std::string::String>"
        );
        assert_eq!(
            CppType::Named("rusty::Result<void, int>".to_string()).to_rust_type_str(),
            "std::result::Result<(), i32>"
        );
        assert_eq!(
            CppType::Named("rusty::Arc<int>".to_string()).to_rust_type_str(),
            "std::sync::Arc<i32>"
        );
        assert_eq!(
            CppType::Named("rusty::Shared<int>".to_string()).to_rust_type_str(),
            "std::sync::Arc<i32>"
        );
        assert_eq!(
            CppType::Named("rusty::RefCounted<int>".to_string()).to_rust_type_str(),
            "std::rc::Rc<i32>"
        );
        assert_eq!(
            CppType::Named("rusty::Boxed<int>".to_string()).to_rust_type_str(),
            "std::boxed::Box<i32>"
        );
        assert_eq!(
            CppType::Named("rusty::ResultVoid<long>".to_string()).to_rust_type_str(),
            "std::result::Result<i64, ()>"
        );
        assert_eq!(
            CppType::Named("rusty::BTreeSet<int, std::less<int>>".to_string()).to_rust_type_str(),
            "std::collections::BTreeSet<i32>"
        );
        assert_eq!(
            CppType::Named("rusty::HashMap<int, long, std::hash<int>>".to_string())
                .to_rust_type_str(),
            "std::collections::HashMap<i32, i64>"
        );
        assert_eq!(
            CppType::Named("rusty::BTreeMap<int, long, std::less<int>>".to_string())
                .to_rust_type_str(),
            "std::collections::BTreeMap<i32, i64>"
        );
    }

    #[test]
    fn test_unqualified_rusty_aliases_map_to_std_library_types() {
        assert_eq!(
            CppType::Named("RefCell<int>".to_string()).to_rust_type_str(),
            "std::cell::RefCell<i32>"
        );
        assert_eq!(
            CppType::Named("HashMap<int, long>".to_string()).to_rust_type_str(),
            "std::collections::HashMap<i32, i64>"
        );
        assert_eq!(
            CppType::Named("Option<String>".to_string()).to_rust_type_str(),
            "std::option::Option<std::string::String>"
        );
        assert_eq!(
            CppType::Named("Result<int, String>".to_string()).to_rust_type_str(),
            "std::result::Result<i32, std::string::String>"
        );
        assert_eq!(
            CppType::Named("Result<void, String>".to_string()).to_rust_type_str(),
            "std::result::Result<(), std::string::String>"
        );
        assert_eq!(
            CppType::Named("ResultInt<long>".to_string()).to_rust_type_str(),
            "std::result::Result<i64, i32>"
        );
        assert_eq!(
            CppType::Named("Boxed<int>".to_string()).to_rust_type_str(),
            "std::boxed::Box<i32>"
        );
        assert_eq!(
            CppType::Named("Shared<int>".to_string()).to_rust_type_str(),
            "std::sync::Arc<i32>"
        );
        assert_eq!(
            CppType::Named("RefCounted<int>".to_string()).to_rust_type_str(),
            "std::rc::Rc<i32>"
        );
        assert_eq!(
            CppType::Named("HashMap<int, long, std::hash<int>>".to_string()).to_rust_type_str(),
            "std::collections::HashMap<i32, i64>"
        );
    }

    #[test]
    fn test_nested_rusty_namespace_aliases_map_to_std_library_types() {
        assert_eq!(
            CppType::Named("rusty::string::String".to_string()).to_rust_type_str(),
            "std::string::String"
        );
        assert_eq!(
            CppType::Named("rusty::cell::RefCell<int>".to_string()).to_rust_type_str(),
            "std::cell::RefCell<i32>"
        );
        assert_eq!(
            CppType::Named("rusty::collections::HashMap<int, long>".to_string()).to_rust_type_str(),
            "std::collections::HashMap<i32, i64>"
        );
        assert_eq!(
            CppType::Named("rusty::rc::Weak<int>".to_string()).to_rust_type_str(),
            "std::rc::Weak<i32>"
        );
        assert_eq!(
            CppType::Named("rusty::sync::Weak<int>".to_string()).to_rust_type_str(),
            "std::sync::Weak<i32>"
        );
        assert_eq!(
            CppType::Named("rusty::Ptr<rusty::String>".to_string()).to_rust_type_str(),
            "*const std::string::String"
        );
        assert_eq!(
            CppType::Named("rusty::MutPtr<int>".to_string()).to_rust_type_str(),
            "*mut i32"
        );
    }

    #[test]
    fn test_rusty_thread_and_mpsc_types_map_to_std_library_types() {
        assert_eq!(
            CppType::Named("rusty::thread::JoinHandle<void>".to_string()).to_rust_type_str(),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            CppType::Named("JoinHandle<void>".to_string()).to_rust_type_str(),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            CppType::Named("const class rusty::thread::JoinHandle<void>".to_string())
                .to_rust_type_str(),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            CppType::Named("rusty::thread::JoinHandle<int>".to_string()).to_rust_type_str(),
            "std::thread::JoinHandle<i32>"
        );
        assert_eq!(
            CppType::Named("std::thread::JoinHandle<void>".to_string()).to_rust_type_str(),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            CppType::Named("std::thread::JoinHandle<()>".to_string()).to_rust_type_str(),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            CppType::Named("std_thread_JoinHandle_void_".to_string()).to_rust_type_str(),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            CppType::Named("std_thread_JoinHandle_void__".to_string()).to_rust_type_str(),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            CppType::Named("rusty_thread_JoinHandle_void__".to_string()).to_rust_type_str(),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            CppType::Named("JoinHandle_void__".to_string()).to_rust_type_str(),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            CppType::Named("JoinHandle<void_>".to_string()).to_rust_type_str(),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            CppType::Named("rusty::sync::mpsc::Sender<int>".to_string()).to_rust_type_str(),
            "std::sync::mpsc::Sender<i32>"
        );
        assert_eq!(
            CppType::Named("rusty::sync::mpsc::Receiver<int>".to_string()).to_rust_type_str(),
            "std::sync::mpsc::Receiver<i32>"
        );
        assert_eq!(
            CppType::Named("std::sync::mpsc::Sender<int>".to_string()).to_rust_type_str(),
            "std::sync::mpsc::Sender<i32>"
        );
        assert_eq!(
            CppType::Named("std::sync::mpsc::Receiver<int>".to_string()).to_rust_type_str(),
            "std::sync::mpsc::Receiver<i32>"
        );
        assert_eq!(
            CppType::Named("std::sync::mpsc::SyncSender<int>".to_string()).to_rust_type_str(),
            "std::sync::mpsc::SyncSender<i32>"
        );
        assert_eq!(
            CppType::Named("Sender<int>".to_string()).to_rust_type_str(),
            "std::sync::mpsc::Sender<i32>"
        );
        assert_eq!(
            CppType::Named("Receiver<int>".to_string()).to_rust_type_str(),
            "std::sync::mpsc::Receiver<i32>"
        );
        assert_eq!(
            CppType::Named("SyncSender<int>".to_string()).to_rust_type_str(),
            "std::sync::mpsc::SyncSender<i32>"
        );
        assert_eq!(
            CppType::Named("std_sync_mpsc_Sender_int".to_string()).to_rust_type_str(),
            "std::sync::mpsc::Sender<i32>"
        );
        assert_eq!(
            CppType::Named("std_sync_mpsc_Receiver_int".to_string()).to_rust_type_str(),
            "std::sync::mpsc::Receiver<i32>"
        );
        assert_eq!(
            CppType::Named("std_sync_mpsc_SyncSender_int".to_string()).to_rust_type_str(),
            "std::sync::mpsc::SyncSender<i32>"
        );
        assert_eq!(
            CppType::Named("rusty_sync_mpsc_Sender_int".to_string()).to_rust_type_str(),
            "std::sync::mpsc::Sender<i32>"
        );
        assert_eq!(
            CppType::Named(
                "volatile struct rusty::sync::mpsc::Sender<const class rusty::String>".to_string()
            )
            .to_rust_type_str(),
            "std::sync::mpsc::Sender<std::string::String>"
        );
        assert_eq!(
            CppType::Named("rusty::sync::mpsc::Unit".to_string()).to_rust_type_str(),
            "()"
        );
        assert_eq!(
            CppType::Named("rusty::None_t".to_string()).to_rust_type_str(),
            "()"
        );
        assert_eq!(CppType::Named("None_t".to_string()).to_rust_type_str(), "()");
        assert_eq!(CppType::Named("Unit".to_string()).to_rust_type_str(), "()");
        assert_eq!(
            CppType::Named("rusty::sync::mpsc::RecvError".to_string()).to_rust_type_str(),
            "std::sync::mpsc::RecvError"
        );
        assert_eq!(
            CppType::Named("RecvError".to_string()).to_rust_type_str(),
            "std::sync::mpsc::RecvError"
        );
        assert_eq!(
            CppType::Named("rusty::sync::mpsc::TryRecvError".to_string()).to_rust_type_str(),
            "std::sync::mpsc::TryRecvError"
        );
        assert_eq!(
            CppType::Named("TryRecvError".to_string()).to_rust_type_str(),
            "std::sync::mpsc::TryRecvError"
        );
        assert_eq!(
            CppType::Named("rusty::sync::mpsc::TrySendError".to_string()).to_rust_type_str(),
            "std::sync::mpsc::TrySendError<()>"
        );
        assert_eq!(
            CppType::Named("TrySendError".to_string()).to_rust_type_str(),
            "std::sync::mpsc::TrySendError<()>"
        );
        assert_eq!(
            CppType::Named("rusty::sync::mpsc::TrySendError<long>".to_string()).to_rust_type_str(),
            "std::sync::mpsc::TrySendError<i64>"
        );
        assert_eq!(
            CppType::Named("Condvar".to_string()).to_rust_type_str(),
            "std::sync::Condvar"
        );
        let mutex_member_guard =
            CppType::Named("rusty::sync::Mutex<int>::Guard".to_string()).to_rust_type_str();
        assert!(
            mutex_member_guard.contains("Guard")
                && !mutex_member_guard.contains("std::sync::MutexGuard"),
            "mutex member guard lowering should avoid invalid std::sync::MutexGuard lifetimes, got: {}",
            mutex_member_guard
        );
        let rwlock_member_read =
            CppType::Named("rusty::RwLock<long>::ReadGuard".to_string()).to_rust_type_str();
        assert!(
            rwlock_member_read.contains("ReadGuard")
                && !rwlock_member_read.contains("std::sync::RwLockReadGuard"),
            "rwlock member read-guard lowering should avoid invalid std::sync::RwLockReadGuard lifetimes, got: {}",
            rwlock_member_read
        );
        let rwlock_member_write =
            CppType::Named("RwLock<long>::WriteGuard".to_string()).to_rust_type_str();
        assert!(
            rwlock_member_write.contains("WriteGuard")
                && !rwlock_member_write.contains("std::sync::RwLockWriteGuard"),
            "rwlock member write-guard lowering should avoid invalid std::sync::RwLockWriteGuard lifetimes, got: {}",
            rwlock_member_write
        );
        assert_eq!(
            CppType::Named("rusty::sync::PoisonError<int>".to_string()).to_rust_type_str(),
            "rusty::sync::PoisonError<i32>"
        );
        assert_eq!(
            CppType::Named("rusty::LockResult<int>".to_string()).to_rust_type_str(),
            "rusty::LockResult<i32>"
        );
        assert_eq!(
            CppType::Named("TryLockResult<int>".to_string()).to_rust_type_str(),
            "rusty::TryLockResult<i32>"
        );
        let mutex_guard =
            CppType::Named("rusty::MutexGuard<int>".to_string()).to_rust_type_str();
        assert!(
            mutex_guard.contains("MutexGuard")
                && !mutex_guard.contains("std::sync::MutexGuard"),
            "mutex guard lowering should avoid invalid std::sync::MutexGuard lifetime rewrites, got: {}",
            mutex_guard
        );
        let rwlock_read_guard =
            CppType::Named("rusty::sync::RwLockReadGuard<long>".to_string()).to_rust_type_str();
        assert!(
            rwlock_read_guard.contains("RwLockReadGuard")
                && !rwlock_read_guard.contains("std::sync::RwLockReadGuard"),
            "rwlock read guard lowering should avoid invalid std::sync::RwLockReadGuard lifetime rewrites, got: {}",
            rwlock_read_guard
        );
        let rwlock_write_guard =
            CppType::Named("RwLockWriteGuard<long>".to_string()).to_rust_type_str();
        assert!(
            rwlock_write_guard.contains("RwLockWriteGuard")
                && !rwlock_write_guard.contains("std::sync::RwLockWriteGuard"),
            "rwlock write guard lowering should avoid invalid std::sync::RwLockWriteGuard lifetime rewrites, got: {}",
            rwlock_write_guard
        );
        assert_eq!(
            CppType::Named("rusty::Ref<rusty::Vec<int>>".to_string()).to_rust_type_str(),
            "std::cell::Ref<std::vec::Vec<i32>>"
        );
        assert_eq!(
            CppType::Named("RefMut<rusty::String>".to_string()).to_rust_type_str(),
            "std::cell::RefMut<std::string::String>"
        );
        assert_eq!(
            CppType::Named("rusty::UnsafeCell<int>".to_string()).to_rust_type_str(),
            "std::cell::UnsafeCell<i32>"
        );
        assert_eq!(
            CppType::Named("rusty::thread::rusty_thread_JoinHandle_void_".to_string())
                .to_rust_type_str(),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            CppType::Named("rusty::thread::JoinHandle<__>".to_string()).to_rust_type_str(),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            CppType::Named("rusty::Barrier".to_string()).to_rust_type_str(),
            "std::sync::Barrier"
        );
        assert_eq!(
            CppType::Named("rusty::Condvar".to_string()).to_rust_type_str(),
            "std::sync::Condvar"
        );
        assert_eq!(
            CppType::Named("rusty::sync::Once".to_string()).to_rust_type_str(),
            "std::sync::Once"
        );
        assert_eq!(
            CppType::Named("rusty::WaitTimeoutResult".to_string()).to_rust_type_str(),
            "std::sync::WaitTimeoutResult"
        );
        assert_eq!(
            CppType::Named("::rusty::Barrier".to_string()).to_rust_type_str(),
            "std::sync::Barrier"
        );
        assert_eq!(
            CppType::Named("crate::rusty::sync::mpsc::RecvError".to_string()).to_rust_type_str(),
            "std::sync::mpsc::RecvError"
        );
        assert_eq!(
            CppType::Named("rusty::sync::mpsc::TrySendError<__>".to_string()).to_rust_type_str(),
            "std::sync::mpsc::TrySendError<()>"
        );
    }

    #[test]
    fn test_normalize_rusty_type_alias_to_std_maps_wrappers_and_preserves_non_rusty_paths() {
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::Option<rusty::String>"),
            "std::option::Option<std::string::String>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::HashMap<int, long>"),
            "std::collections::HashMap<i32, i64>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("testing::internal::Visible"),
            "testing::internal::Visible"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::sync::Weak<int>"),
            "std::sync::Weak<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::thread::JoinHandle<void>"),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std::thread::JoinHandle<void>"),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std::thread::JoinHandle<()>"),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std_thread_JoinHandle_void_"),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std_thread_JoinHandle_void__"),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty_thread_JoinHandle_void__"),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("JoinHandle_void__"),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("JoinHandle<void_>"),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("JoinHandle<void>"),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::thread::JoinHandle<__>"),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std::thread::JoinHandle<__>"),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("JoinHandle<__>"),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::sync::mpsc::Sender<int>"),
            "std::sync::mpsc::Sender<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::sync::mpsc::Sender<__>"),
            "std::sync::mpsc::Sender<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std::sync::mpsc::Sender<int>"),
            "std::sync::mpsc::Sender<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std::sync::mpsc::Receiver<int>"),
            "std::sync::mpsc::Receiver<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std::sync::mpsc::Receiver<__>"),
            "std::sync::mpsc::Receiver<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std::sync::mpsc::SyncSender<int>"),
            "std::sync::mpsc::SyncSender<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std::sync::mpsc::SyncSender<__>"),
            "std::sync::mpsc::SyncSender<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("Sender<int>"),
            "std::sync::mpsc::Sender<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("Receiver<int>"),
            "std::sync::mpsc::Receiver<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("SyncSender<int>"),
            "std::sync::mpsc::SyncSender<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std_sync_mpsc_Sender_int"),
            "std::sync::mpsc::Sender<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std_sync_mpsc_Receiver_int"),
            "std::sync::mpsc::Receiver<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std_sync_mpsc_SyncSender_int"),
            "std::sync::mpsc::SyncSender<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty_sync_mpsc_Sender_int"),
            "std::sync::mpsc::Sender<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std_sync_mpsc_TrySendError_int"),
            "std::sync::mpsc::TrySendError<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty_sync_mpsc_TrySendError_int"),
            "std::sync::mpsc::TrySendError<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std_sync_mpsc_TrySendError__"),
            "std::sync::mpsc::TrySendError<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std_sync_mpsc_TrySendError_"),
            "std::sync::mpsc::TrySendError<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty_Arc_struct_rrr_RpcServiceContext__"),
            "std::sync::Arc<rrr_RpcServiceContext>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty_Rc_class_rrr_Reactor__"),
            "std::rc::Rc<rrr_Reactor>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("classrusty_Rc_classrrr_Reactor__"),
            "std::rc::Rc<rrr_Reactor>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("classrusty_Arc_classrrr_ClientConnection__"),
            "std::sync::Arc<rrr_ClientConnection>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("constclassrusty_Arc_structrrr_RpcServiceContext__"),
            "std::sync::Arc<rrr_RpcServiceContext>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("Option_classrusty_Rc_classrrr_Reactor___"),
            "std::option::Option<std::rc::Rc<rrr_Reactor>>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("Option_classrusty_thread_JoinHandle_void__"),
            "std::option::Option<std::thread::JoinHandle<()>>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std::option::Option<rusty_Rc_class_rrr_Reactor__>"),
            "std::option::Option<std::rc::Rc<rrr_Reactor>>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std(
                "std::option::Option<rusty_Arc_struct_rrr_RpcServiceContext__>"
            ),
            "std::option::Option<std::sync::Arc<rrr_RpcServiceContext>>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std(
                "std::sync::Mutex<std::option::Option<rusty_Rc_class_rrr_Reactor__>>"
            ),
            "std::sync::Mutex<std::option::Option<std::rc::Rc<rrr_Reactor>>>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std(
                "std::sync::Mutex<std::option::Option<std::thread::JoinHandle<()>>>"
            ),
            "std::sync::Mutex<std::option::Option<std::thread::JoinHandle<()>>>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("Mutex_Option_JoinHandle_void___"),
            "std::sync::Mutex<std::option::Option<std::thread::JoinHandle<()>>>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std(
                "Mutex_classrusty_Option_classrusty_thread_JoinHandle_void___"
            ),
            "std::sync::Mutex<std::option::Option<std::thread::JoinHandle<()>>>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty_BTreeSet_class_rusty_Rc_class_rrr_Fiber__Unit"),
            "()"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("constclassrusty_BTreeSet_classrusty_Rc_classrrr_Fiber___Unit_"),
            "()"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std(
                "std::option::Option<rusty_BTreeSet_class_rusty_Rc_class_rrr_Fiber__Unit>"
            ),
            "std::option::Option<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std(
                "std::collections::BTreeMap<rusty_Rc_class_rrr_Fiber__, rusty_BTreeSet_class_rusty_Rc_class_rrr_Fiber__Unit>"
            ),
            "std::collections::BTreeMap<std::rc::Rc<rrr_Fiber>, ()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std(
                "std::collections::BTreeMap<std::rc::Rc<rrr_Fiber>, ()>"
            ),
            "std::collections::BTreeMap<std::rc::Rc<rrr_Fiber>, ()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std::option::Option<()>"),
            "std::option::Option<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::Result<void, int>"),
            "std::result::Result<(), i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("std::result::Result<type-parameter-0-0, void>"),
            "std::result::Result<(), ()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("enumrusty_sync_mpsc_RecvError"),
            "std::sync::mpsc::RecvError"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("enumrusty_sync_mpsc_TryRecvError"),
            "std::sync::mpsc::TryRecvError"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("enumrusty_sync_mpsc_TrySendError"),
            "std::sync::mpsc::TrySendError<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std(
                "rusty_Result_structrusty_sync_mpsc_Unit_enumrusty_sync_mpsc_TrySendError_"
            ),
            "std::result::Result<(), std::sync::mpsc::TrySendError<()>>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty_Result_int_enumrusty_sync_mpsc_RecvError_"),
            "std::result::Result<i32, std::sync::mpsc::RecvError>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std(
                "rusty_Result_long_constenumrusty_sync_mpsc_TryRecvError_"
            ),
            "std::result::Result<i64, std::sync::mpsc::TryRecvError>"
        );
        assert_eq!(normalize_rusty_type_alias_to_std("None_t"), "()");
        assert_eq!(normalize_rusty_type_alias_to_std("rusty::None_t"), "()");
        assert_eq!(normalize_rusty_type_alias_to_std("Unit"), "()");
        assert_eq!(
            normalize_rusty_type_alias_to_std("Condvar"),
            "std::sync::Condvar"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("RecvError"),
            "std::sync::mpsc::RecvError"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::sync::mpsc::TrySendError"),
            "std::sync::mpsc::TrySendError<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("TrySendError"),
            "std::sync::mpsc::TrySendError<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("TrySendError<int>"),
            "std::sync::mpsc::TrySendError<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("TrySendError<__>"),
            "std::sync::mpsc::TrySendError<()>"
        );
        let mutex_member_guard = normalize_rusty_type_alias_to_std("rusty::sync::Mutex<int>::Guard");
        assert!(
            mutex_member_guard.contains("Guard")
                && !mutex_member_guard.contains("std::sync::MutexGuard"),
            "alias normalization should avoid invalid std::sync::MutexGuard lifetime rewrites, got: {}",
            mutex_member_guard
        );
        let rwlock_member_read = normalize_rusty_type_alias_to_std("RwLock<long>::ReadGuard");
        assert!(
            rwlock_member_read.contains("ReadGuard")
                && !rwlock_member_read.contains("std::sync::RwLockReadGuard"),
            "alias normalization should avoid invalid std::sync::RwLockReadGuard lifetime rewrites, got: {}",
            rwlock_member_read
        );
        let rwlock_member_write =
            normalize_rusty_type_alias_to_std("rusty::RwLock<long>::WriteGuard");
        assert!(
            rwlock_member_write.contains("WriteGuard")
                && !rwlock_member_write.contains("std::sync::RwLockWriteGuard"),
            "alias normalization should avoid invalid std::sync::RwLockWriteGuard lifetime rewrites, got: {}",
            rwlock_member_write
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::PoisonError<int>"),
            "rusty::PoisonError<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("LockResult<int>"),
            "rusty::LockResult<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::sync::TryLockResult<int>"),
            "rusty::sync::TryLockResult<i32>"
        );
        let mutex_guard = normalize_rusty_type_alias_to_std("rusty::MutexGuard<int>");
        assert!(
            mutex_guard.contains("MutexGuard")
                && !mutex_guard.contains("std::sync::MutexGuard"),
            "alias normalization should avoid invalid std::sync::MutexGuard lifetime rewrites, got: {}",
            mutex_guard
        );
        let rwlock_read_guard = normalize_rusty_type_alias_to_std("RwLockReadGuard<long>");
        assert!(
            rwlock_read_guard.contains("RwLockReadGuard")
                && !rwlock_read_guard.contains("std::sync::RwLockReadGuard"),
            "alias normalization should avoid invalid std::sync::RwLockReadGuard lifetime rewrites, got: {}",
            rwlock_read_guard
        );
        let rwlock_write_guard =
            normalize_rusty_type_alias_to_std("rusty::sync::RwLockWriteGuard<long>");
        assert!(
            rwlock_write_guard.contains("RwLockWriteGuard")
                && !rwlock_write_guard.contains("std::sync::RwLockWriteGuard"),
            "alias normalization should avoid invalid std::sync::RwLockWriteGuard lifetime rewrites, got: {}",
            rwlock_write_guard
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::Ref<rusty::Vec<int>>"),
            "std::cell::Ref<std::vec::Vec<i32>>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("RefMut<rusty::String>"),
            "std::cell::RefMut<std::string::String>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::Boxed<int>"),
            "std::boxed::Box<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::Shared<int>"),
            "std::sync::Arc<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::RefCounted<int>"),
            "std::rc::Rc<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::Ptr<rusty::String>"),
            "*const std::string::String"
        );
        assert_eq!(normalize_rusty_type_alias_to_std("MutPtr<int>"), "*mut i32");
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::UnsafeCell<rusty::Vec<int>>"),
            "std::cell::UnsafeCell<std::vec::Vec<i32>>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::thread::rusty_thread_JoinHandle_void_"),
            "std::thread::JoinHandle<()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std(
                "Option<extern \"C\" fn(*mut ReqHandle, *mut ()) -> ()>"
            ),
            "std::option::Option<extern \"C\" fn(*mut ReqHandle, *mut ()) -> ()>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::Barrier"),
            "std::sync::Barrier"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::Condvar"),
            "std::sync::Condvar"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::Once"),
            "std::sync::Once"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::WaitTimeoutResult"),
            "std::sync::WaitTimeoutResult"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("::rusty::Option<rusty::String>"),
            "std::option::Option<std::string::String>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("crate::rusty::sync::mpsc::RecvError"),
            "std::sync::mpsc::RecvError"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::Vec<int, std::allocator<int>>"),
            "std::vec::Vec<i32>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::HashMap<int, long, std::hash<int>>"),
            "std::collections::HashMap<i32, i64>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("rusty::Option<int, long>"),
            "rusty::Option<int, long>"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std("const class ::rusty::Barrier"),
            "std::sync::Barrier"
        );
        assert_eq!(
            normalize_rusty_type_alias_to_std(
                "volatile struct crate::rusty::sync::mpsc::Receiver<const class rusty::String>"
            ),
            "std::sync::mpsc::Receiver<std::string::String>"
        );
    }

    #[test]
    fn test_template_char_literal_type_name_sanitizes_apostrophes() {
        let lowered = CppType::Named("inline_str_fixed<9, '_'>".to_string()).to_rust_type_str();
        assert!(
            !lowered.contains('\''),
            "template-char type lowering must strip apostrophes from generated Rust identifiers, got: {}",
            lowered
        );
    }

    #[test]
    fn test_std_array_type_mapping() {
        // NOTE: STL mappings removed - all types pass through as-is
        // See Section 22 in TODO.md for rationale

        // std::array passes through (no longer mapped to [T; N])
        // Template syntax converted to valid Rust identifiers
        assert_eq!(
            CppType::Named("std::array<int, 5>".to_string()).to_rust_type_str(),
            "std_array_int__5"
        );
        assert_eq!(
            CppType::Named("std::array<double, 10>".to_string()).to_rust_type_str(),
            "std_array_double__10"
        );

        // Nested template types also pass through
        assert_eq!(
            CppType::Named("std::array<std::vector<int>, 2>".to_string()).to_rust_type_str(),
            "std_array_std_vector_int__2"
        );
    }

    #[test]
    fn test_named_array_size_static_cast_normalizes_to_rust_usize_expr() {
        assert_eq!(
            CppType::Named("char[static_cast<size_t>(ITEM_SIZE)]".to_string()).to_rust_type_str(),
            "[i8; (ITEM_SIZE) as usize]"
        );
        assert_eq!(
            CppType::Named("char[static_cast<std::size_t>(4)]".to_string()).to_rust_type_str(),
            "[i8; 4]"
        );
    }

    #[test]
    fn test_named_array_size_pack_expansion_falls_back_to_zero() {
        assert_eq!(
            CppType::Named("bool[sizeof...(_Args)]".to_string()).to_rust_type_str(),
            "[bool; 0usize]"
        );
    }

    #[test]
    fn test_std_span_type_mapping() {
        // NOTE: STL mappings removed - all types pass through as-is
        // See Section 22 in TODO.md for rationale
        // Template syntax converted to valid Rust identifiers

        // std::span passes through (no longer mapped to &[T])
        assert_eq!(
            CppType::Named("std::span<int>".to_string()).to_rust_type_str(),
            "std_span_int"
        );
        assert_eq!(
            CppType::Named("std::span<const int>".to_string()).to_rust_type_str(),
            "std_span_const_int"
        );
        assert_eq!(
            CppType::Named("std::span<int, 10>".to_string()).to_rust_type_str(),
            "std_span_int__10"
        );
    }

    #[test]
    fn test_std_variant_type_mapping() {
        // NOTE: STL mappings removed - all types pass through as-is
        // See Section 22 in TODO.md for rationale
        // Template syntax converted to valid Rust identifiers

        // std::variant passes through (no longer mapped to Variant_...)
        assert_eq!(
            CppType::Named("std::variant<int, double>".to_string()).to_rust_type_str(),
            "std_variant_int__double"
        );
        assert_eq!(
            CppType::Named("std::variant<int, std::string>".to_string()).to_rust_type_str(),
            "std_variant_int__std_string"
        );
        assert_eq!(
            CppType::Named("std::variant<MyClass, OtherClass>".to_string()).to_rust_type_str(),
            "std_variant_MyClass__OtherClass"
        );
    }

    #[test]
    fn test_stream_type_mappings() {
        // NOTE: STL mappings removed - all types pass through as-is
        // See Section 22 in TODO.md for rationale

        // Stream types pass through (no longer mapped to Rust I/O types)
        assert_eq!(
            CppType::Named("std::ostream".to_string()).to_rust_type_str(),
            "std_ostream"
        );
        assert_eq!(
            CppType::Named("std::istream".to_string()).to_rust_type_str(),
            "std_istream"
        );
        assert_eq!(
            CppType::Named("std::iostream".to_string()).to_rust_type_str(),
            "std_iostream"
        );
        assert_eq!(
            CppType::Named("std::stringstream".to_string()).to_rust_type_str(),
            "std_stringstream"
        );
        assert_eq!(
            CppType::Named("std::ofstream".to_string()).to_rust_type_str(),
            "std_ofstream"
        );
        assert_eq!(
            CppType::Named("std::ifstream".to_string()).to_rust_type_str(),
            "std_ifstream"
        );
        assert_eq!(
            CppType::Named("std::fstream".to_string()).to_rust_type_str(),
            "std_fstream"
        );
    }

    #[test]
    fn test_inline_namespace_stripping() {
        // libc++ uses inline namespaces like std::__1:: for ABI versioning
        // These should be stripped to produce cleaner type names

        // std::__1::vector<int> -> std_vector_int
        assert_eq!(
            CppType::Named("std::__1::vector<int>".to_string()).to_rust_type_str(),
            "std_vector_int"
        );

        // std::__1::string -> std_string
        assert_eq!(
            CppType::Named("std::__1::string".to_string()).to_rust_type_str(),
            "std_string"
        );

        // std::__1::basic_string<char> -> std_basic_string_char
        assert_eq!(
            CppType::Named("std::__1::basic_string<char>".to_string()).to_rust_type_str(),
            "std_basic_string_char"
        );

        // Nested inline namespaces: std::__1::__detail::__helper -> std___detail___helper
        assert_eq!(
            CppType::Named("std::__1::__detail::__helper".to_string()).to_rust_type_str(),
            "std___detail___helper"
        );

        // std::__2:: (alternative version) should also be stripped
        assert_eq!(
            CppType::Named("std::__2::vector<int>".to_string()).to_rust_type_str(),
            "std_vector_int"
        );

        // Android NDK uses __ndk1
        assert_eq!(
            CppType::Named("std::__ndk1::vector<int>".to_string()).to_rust_type_str(),
            "std_vector_int"
        );
    }

    #[test]
    fn test_tinyxml2_namespaced_enum_alias_mappings() {
        assert_eq!(
            CppType::Named("tinyxml2::XMLError".to_string()).to_rust_type_str(),
            "XMLError"
        );
        assert_eq!(
            CppType::Named("tinyxml2_XMLError".to_string()).to_rust_type_str(),
            "XMLError"
        );
        assert_eq!(
            CppType::Named("tinyxml2::Whitespace".to_string()).to_rust_type_str(),
            "Whitespace"
        );
        assert_eq!(
            CppType::Named("tinyxml2_Whitespace".to_string()).to_rust_type_str(),
            "Whitespace"
        );
        assert_eq!(
            CppType::Named("tinyxml2::XMLElement::ElementClosingType".to_string())
                .to_rust_type_str(),
            "ElementClosingType"
        );
        assert_eq!(
            CppType::Named("tinyxml2_XMLElement_ElementClosingType".to_string()).to_rust_type_str(),
            "ElementClosingType"
        );
        assert_eq!(
            CppType::Named("tinyxml2::StrPair::Mode".to_string()).to_rust_type_str(),
            "Mode"
        );
        assert_eq!(
            CppType::Named("tinyxml2_StrPair_Mode".to_string()).to_rust_type_str(),
            "Mode"
        );
    }

    #[test]
    fn test_rapidjson_generic_string_buffer_alias_normalization() {
        assert_eq!(
            CppType::Named("GenericStringBuffer_UTF8_char__CrtAllocator".to_string())
                .to_rust_type_str(),
            "GenericStringBuffer_UTF8"
        );
        assert_eq!(
            CppType::Named(
                "rapidjson::GenericStringBuffer<struct rapidjson::UTF8<>, class rapidjson::CrtAllocator>"
                    .to_string()
            )
            .to_rust_type_str(),
            "GenericStringBuffer_UTF8"
        );
    }

    #[test]
    fn test_named_type_conversion_strips_equals_from_identifier() {
        let converted = CppType::Named(
            "StaticAssertTest<sizeof(rapidjson::STATIC_ASSERTION_FAILURE<bool(sizeof(Ch)==2)>)>"
                .to_string(),
        )
        .to_rust_type_str();
        assert!(
            !converted.contains('='),
            "converted type name should not contain '=', got: {}",
            converted
        );
    }

    #[test]
    fn test_file_like_aliases_lower_to_opaque_c_void() {
        assert_eq!(
            CppType::Named("__FILE".to_string()).to_rust_type_str(),
            "std::ffi::c_void"
        );
        assert_eq!(
            CppType::Named("struct __FILE".to_string()).to_rust_type_str(),
            "std::ffi::c_void"
        );
        assert_eq!(
            CppType::Named("struct __sFILE".to_string()).to_rust_type_str(),
            "std::ffi::c_void"
        );
        assert_eq!(
            CppType::Named("std::FILE".to_string()).to_rust_type_str(),
            "std::ffi::c_void"
        );
        assert_eq!(
            CppType::Named("std::__1::FILE".to_string()).to_rust_type_str(),
            "std::ffi::c_void"
        );
        assert_eq!(
            CppType::Pointer {
                pointee: Box::new(CppType::Named("__FILE".to_string())),
                is_const: false,
            }
            .to_rust_type_str(),
            "*mut std::ffi::c_void"
        );
        assert_eq!(
            CppType::Pointer {
                pointee: Box::new(CppType::Named("__FILE".to_string())),
                is_const: true,
            }
            .to_rust_type_str(),
            "*const std::ffi::c_void"
        );
    }

    #[test]
    fn test_named_void_lowers_to_opaque_c_void() {
        assert_eq!(
            CppType::Named("void".to_string()).to_rust_type_str(),
            "std::ffi::c_void"
        );
        assert_eq!(
            CppType::Pointer {
                pointee: Box::new(CppType::Named("void".to_string())),
                is_const: false,
            }
            .to_rust_type_str(),
            "*mut std::ffi::c_void"
        );
    }

    #[test]
    fn test_timeval_named_type_preserves_struct_spelling() {
        assert_eq!(
            CppType::Named("timeval".to_string()).to_rust_type_str(),
            "timeval"
        );
    }

    #[test]
    fn test_std_identity_aliases_lower_to_generated_identity_type() {
        assert_eq!(
            CppType::Named("std___identity".to_string()).to_rust_type_str(),
            "__identity"
        );
        assert_eq!(
            CppType::Named("std::__1::__identity".to_string()).to_rust_type_str(),
            "__identity"
        );
    }

    #[test]
    fn test_functional_hash_unnamed_struct_aliases_lower_to_u64() {
        let helper =
            "_unnamed_struct_at__home_shuai_workspace_fragile_vendor_llvm_project_libcxx_include___functional_hash_h_285_7_";
        assert_eq!(CppType::Named(helper.to_string()).to_rust_type_str(), "u64");
        assert_eq!(
            CppType::Named(format!("struct {}", helper)).to_rust_type_str(),
            "u64"
        );
        assert_eq!(
            CppType::Named(format!("class {}", helper)).to_rust_type_str(),
            "u64"
        );
        assert_eq!(
            CppType::Pointer {
                pointee: Box::new(CppType::Named(helper.to_string())),
                is_const: false,
            }
            .to_rust_type_str(),
            "*mut u64"
        );
    }

    #[test]
    fn test_cxx_atomic_base_impl_bool_alias_normalizes_to_impl_bool() {
        assert_eq!(
            CppType::Named("__cxx_atomic_base_impl_bool".to_string()).to_rust_type_str(),
            "__cxx_atomic_impl_bool"
        );
        assert_eq!(
            CppType::Named("std::__1::__cxx_atomic_base_impl_bool".to_string()).to_rust_type_str(),
            "__cxx_atomic_impl_bool"
        );
        assert_eq!(
            CppType::Pointer {
                pointee: Box::new(CppType::Named("__cxx_atomic_base_impl_bool".to_string())),
                is_const: true,
            }
            .to_rust_type_str(),
            "*const __cxx_atomic_impl_bool"
        );
    }

    #[test]
    fn test_function_pointer_type_uses_option_extern_c_fn() {
        let ty = CppType::Pointer {
            pointee: Box::new(CppType::Function {
                return_type: Box::new(CppType::Int { signed: true }),
                params: vec![CppType::Int { signed: true }, CppType::Int { signed: true }],
                is_variadic: false,
            }),
            is_const: false,
        };
        assert_eq!(
            ty.to_rust_type_str(),
            "Option<extern \"C\" fn(i32, i32) -> i32>"
        );
    }

    #[test]
    fn test_parse_template_args() {
        // Basic arguments
        assert_eq!(parse_template_args("int, double"), vec!["int", "double"]);

        // Single argument
        assert_eq!(parse_template_args("int"), vec!["int"]);

        // With nested templates
        assert_eq!(
            parse_template_args("int, std::vector<int>, double"),
            vec!["int", "std::vector<int>", "double"]
        );

        // Deeply nested
        assert_eq!(
            parse_template_args("std::map<int, std::vector<double>>, bool"),
            vec!["std::map<int, std::vector<double>>", "bool"]
        );

        // With whitespace
        assert_eq!(
            parse_template_args("  int  ,  double  "),
            vec!["int", "double"]
        );

        // Empty
        assert_eq!(parse_template_args(""), Vec::<String>::new());

        // Function pointer in template arguments
        assert_eq!(
            parse_template_args("char, void (*)(void *)"),
            vec!["char", "void (*)(void *)"]
        );

        // More complex function pointer
        assert_eq!(
            parse_template_args("int, int (*)(int, int), double"),
            vec!["int", "int (*)(int, int)", "double"]
        );

        // Function pointer with nested templates
        assert_eq!(
            parse_template_args("std::vector<int>, void (*)(std::string &)"),
            vec!["std::vector<int>", "void (*)(std::string &)"]
        );
    }
}
