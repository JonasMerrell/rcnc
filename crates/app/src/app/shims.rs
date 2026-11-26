//! Shims a program needs under the work: `(RCNC REQUIRE_SHIM …)` lines. Cycle Start from Idle asks first
//! whether they are in. A job posted for parts standing on shims cuts that much deeper into the vise when the
//! shims are left out (2026-11-26: the Eclipse car's op 1 ran without its 1.67 mm shims and cut 1.37 mm off
//! both jaws of the left station).

/// What each `(RCNC REQUIRE_SHIM: …)` line says, in program order, e.g. "1.67 mm shim strips under both cars".
pub fn required(program: &str) -> Vec<String> {
    program
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            let up = t.to_ascii_uppercase();
            let rest = up.strip_prefix("(RCNC REQUIRE_SHIM")?;
            // keep the operator's text as written (not upper-cased)
            let text = &t[t.len() - rest.len()..];
            let text = text.trim_end().trim_end_matches(')').trim().trim_start_matches(':').trim();
            Some(if text.is_empty() { "shims under the work (the program does not say which)".to_string() } else { text.to_string() })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::required;

    #[test]
    fn finds_each_shim_line_and_keeps_its_text() {
        let p = "(Eclipse car op1 bottom)\n(RCNC REQUIRE_GUARD: …)\n(RCNC REQUIRE_SHIM: 1.67 mm shim strips under both cars, left station [op 1])\n  (rcnc require_shim)\nG0 X0\nM30";
        assert_eq!(
            required(p),
            vec!["1.67 mm shim strips under both cars, left station [op 1]".to_string(), "shims under the work (the program does not say which)".to_string()]
        );
        assert!(required("(RCNC REQUIRE_GUARD)\n(fixture 1.67 mm shim strip)\nM30").is_empty());
    }
}
