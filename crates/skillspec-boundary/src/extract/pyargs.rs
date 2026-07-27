//! Extracting call arguments from a line of Python, lexically.
//!
//! Not a parser. It finds a call by name, then reads its first argument by
//! balancing brackets and respecting quotes. A string literal yields its
//! contents; a list literal yields its first element; anything else - a
//! variable, an expression - yields nothing, which the caller treats as an
//! unresolved target.

/// A single positional argument, and whether it was a literal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Argument {
    /// A string literal's contents, unquoted.
    Str(String),
    /// The first element of a list literal, if it was itself a string.
    ListHead(String),
    /// Present but not a literal: a variable or expression.
    Dynamic,
}

/// The first positional argument of the call opened by `open_paren`.
///
/// `line[open_paren]` must be `(`. Reads to the matching close or the first
/// top-level comma.
pub fn first_argument(line: &str, open_paren: usize) -> Option<Argument> {
    let bytes = line.as_bytes();
    if bytes.get(open_paren) != Some(&b'(') {
        return None;
    }
    let arg = slice_first_arg(&line[open_paren + 1..])?;
    Some(classify(arg.trim()))
}

/// Find `name(` on the line and return the byte index of the `(`.
///
/// Matches only when `name` is preceded by a non-identifier character, so
/// `run(` does not match inside `overrun(`.
pub fn find_call(line: &str, name: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(rel) = line[from..].find(name) {
        let start = from + rel;
        let end = start + name.len();
        let before_ok = start == 0
            || !line.as_bytes()[start - 1].is_ascii_alphanumeric()
                && line.as_bytes()[start - 1] != b'_';
        if before_ok {
            let rest = line[end..].trim_start();
            if rest.starts_with('(') {
                return Some(end + (line[end..].len() - rest.len()));
            }
        }
        from = end;
    }
    None
}

fn slice_first_arg(rest: &str) -> Option<&str> {
    let bytes = rest.as_bytes();
    let mut depth = 0i32;
    let mut quote: Option<u8> = None;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        match quote {
            Some(q) => {
                if byte == b'\\' {
                    index += 2;
                    continue;
                }
                if byte == q {
                    quote = None;
                }
            }
            None => match byte {
                b'\'' | b'"' => quote = Some(byte),
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' if depth == 0 => return Some(&rest[..index]),
                b')' | b']' | b'}' => depth -= 1,
                b',' if depth == 0 => return Some(&rest[..index]),
                _ => {}
            },
        }
        index += 1;
    }
    Some(rest)
}

fn classify(arg: &str) -> Argument {
    if let Some(text) = string_literal(arg) {
        return Argument::Str(text);
    }
    if let Some(inner) = arg.strip_prefix('[') {
        let head = slice_first_arg(inner).unwrap_or("").trim();
        if let Some(text) = string_literal(head) {
            return Argument::ListHead(text);
        }
    }
    Argument::Dynamic
}

/// The contents of a string literal, or `None` if `arg` is not one.
///
/// Serves both Python and JS/TS callers. Handles Python `f`, `r`, `b` prefixes
/// (harmless for JS, which has no such prefixes) and JavaScript backtick
/// template literals. An interpolated string is still returned; the normalizer
/// decides whether the interpolation matters.
fn string_literal(arg: &str) -> Option<String> {
    let trimmed = arg.trim();
    let without_prefix = trimmed.trim_start_matches(['f', 'r', 'b', 'F', 'R', 'B']);
    for quote in ['"', '\'', '`'] {
        if let Some(rest) = without_prefix.strip_prefix(quote) {
            if let Some(end) = rest.find(quote) {
                return Some(rest[..end].to_owned());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{find_call, first_argument, Argument};

    fn first(line: &str, name: &str) -> Option<Argument> {
        find_call(line, name).and_then(|paren| first_argument(line, paren))
    }

    #[test]
    fn a_string_argument_is_extracted() {
        assert_eq!(
            first("requests.get(\"https://x.test/a\")", "requests.get"),
            Some(Argument::Str("https://x.test/a".to_owned()))
        );
    }

    #[test]
    fn a_list_head_is_extracted_for_subprocess() {
        assert_eq!(
            first(
                "subprocess.run([\"gtimeout\", \"--version\"], check=False)",
                "subprocess.run"
            ),
            Some(Argument::ListHead("gtimeout".to_owned()))
        );
    }

    #[test]
    fn a_variable_argument_is_dynamic() {
        assert_eq!(
            first("subprocess.run(cmd, timeout=5)", "subprocess.run"),
            Some(Argument::Dynamic)
        );
    }

    #[test]
    fn an_f_string_literal_is_still_read() {
        assert_eq!(
            first("requests.post(f\"https://{host}/x\")", "requests.post"),
            Some(Argument::Str("https://{host}/x".to_owned()))
        );
    }

    #[test]
    fn commas_inside_nested_brackets_do_not_end_the_argument() {
        assert_eq!(
            first("open(os.path.join(a, b), \"w\")", "open"),
            Some(Argument::Dynamic)
        );
    }

    #[test]
    fn a_name_is_not_matched_as_a_suffix_of_another() {
        assert!(find_call("overrun(x)", "run").is_none());
        assert!(find_call("subprocess.run(x)", "run").is_some());
    }

    #[test]
    fn a_call_with_whitespace_before_the_paren_is_found() {
        assert!(find_call("os.system ('ls')", "os.system").is_some());
    }

    #[test]
    fn a_bare_name_without_a_call_is_not_matched() {
        assert!(find_call("subprocess.run", "subprocess.run").is_none());
    }
}
