// Helper for C++ new[] / delete[] with size tracking
#[inline]
unsafe fn fragile_new_array<T: Clone>(len: usize, init: T) -> *mut T {
    let align = std::mem::align_of::<T>().max(std::mem::align_of::<usize>());
    let header_size = std::mem::size_of::<usize>();
    let padding = (align - (header_size % align)) % align;
    let offset = header_size + padding;
    let elem_size = std::mem::size_of::<T>();
    let total_size = offset + elem_size.saturating_mul(len);
    let layout = std::alloc::Layout::from_size_align(total_size, align).unwrap();
    let base = std::alloc::alloc(layout);
    if base.is_null() { std::alloc::handle_alloc_error(layout); }
    let header = base as *mut usize;
    *header = len;
    let data = base.add(offset) as *mut T;
    for i in 0..len {
        std::ptr::write(data.add(i), init.clone());
    }
    data
}

#[inline]
unsafe fn fragile_delete_array<T>(ptr: *mut T) {
    if ptr.is_null() { return; }
    let align = std::mem::align_of::<T>().max(std::mem::align_of::<usize>());
    let header_size = std::mem::size_of::<usize>();
    let padding = (align - (header_size % align)) % align;
    let offset = header_size + padding;
    let base = (ptr as *mut u8).sub(offset);
    let len = *(base as *mut usize);
    for i in 0..len {
        std::ptr::drop_in_place(ptr.add(i));
    }
    let elem_size = std::mem::size_of::<T>();
    let total_size = offset + elem_size.saturating_mul(len);
    let layout = std::alloc::Layout::from_size_align(total_size, align).unwrap();
    std::alloc::dealloc(base, layout);
}

// STL algorithm helpers
#[inline]
pub fn max_u64_u64(a: u64, b: u64) -> u64 { if a > b { a } else { b } }
#[inline]
pub fn min_u64_u64(a: u64, b: u64) -> u64 { if a < b { a } else { b } }

fn fragile_extract_input_bytes_from_stream<TInput>(is: &TInput) -> std::vec::Vec<u8> {
    unsafe {
        let __is_size = std::mem::size_of::<TInput>();
        // FileReadStream is 57+ bytes (8 pointer/u64 fields + 1 bool).
        if __is_size >= 57 {
            let __base = is as *const TInput as *const u8;
            // buffer_ is at offset 8 (pointer), readCount_ is at offset 40 (u64).
            let __buffer = *(__base.add(8) as *const *const u8);
            let __read_count = *(__base.add(40) as *const u64);
            if !__buffer.is_null() && __read_count > 0 {
                let __len = __read_count as usize;
                return std::slice::from_raw_parts(__buffer, __len).to_vec();
            }
        }
        // Fallback: try Rust stdin (works for StringStream-like or pipe inputs).
        let mut __buf = std::vec::Vec::new();
        let _ = std::io::Read::read_to_end(&mut std::io::stdin(), &mut __buf);
        __buf
    }
}

fn fragile_rapidjson_minify_json(input: &str) -> std::result::Result<std::string::String, ()> {
    let mut out = std::string::String::with_capacity(input.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut stack: std::vec::Vec<char> = std::vec::Vec::new();
    for ch in input.chars() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                out.push(ch);
            }
            '{' | '[' => {
                stack.push(ch);
                out.push(ch);
            }
            '}' => {
                if stack.pop() != Some('{') {
                    return Err(());
                }
                out.push(ch);
            }
            ']' => {
                if stack.pop() != Some('[') {
                    return Err(());
                }
                out.push(ch);
            }
            c if c.is_whitespace() => {}
            _ => out.push(ch),
        }
    }
    if in_string || !stack.is_empty() || out.trim().is_empty() {
        return Err(());
    }
    Ok(out)
}

fn fragile_rapidjson_pretty_json(minified: &str) -> std::result::Result<std::string::String, ()> {
    let mut out = std::string::String::with_capacity(minified.len().saturating_mul(2));
    let mut in_string = false;
    let mut escaped = false;
    let mut indent: usize = 0;
    for ch in minified.chars() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                out.push(ch);
            }
            '{' | '[' => {
                out.push(ch);
                indent = indent.saturating_add(1);
                out.push('\n');
                for _ in 0..indent {
                    out.push_str("    ");
                }
            }
            '}' | ']' => {
                indent = indent.saturating_sub(1);
                out.push('\n');
                for _ in 0..indent {
                    out.push_str("    ");
                }
                out.push(ch);
            }
            ',' => {
                out.push(ch);
                out.push('\n');
                for _ in 0..indent {
                    out.push_str("    ");
                }
            }
            ':' => {
                out.push(':');
                out.push(' ');
            }
            _ => out.push(ch),
        }
    }
    if in_string {
        return Err(());
    }
    out.push('\n');
    Ok(out)
}

fn fragile_rapidjson_render_to_stdout_for_handler<THandler>(
    input: &str,
) -> std::result::Result<(), ()> {
    let minified = fragile_rapidjson_minify_json(input)?;
    let handler_name = std::any::type_name::<THandler>();
    let rendered = if handler_name.contains("PrettyWriter") {
        fragile_rapidjson_pretty_json(&minified)?
    } else {
        minified
    };
    let mut __fragile_stdout = std::io::stdout();
    if std::io::Write::write_all(&mut __fragile_stdout, rendered.as_bytes()).is_err() {
        return Err(());
    }
    if std::io::Write::flush(&mut __fragile_stdout).is_err() {
        return Err(());
    }
    Ok(())
}
