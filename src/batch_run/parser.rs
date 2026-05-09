//! Tokenize lines from stdin into argv-style command vectors,
//! ready to feed into `Cli::try_parse_from`.

use anyhow::{Result, anyhow};
use std::io::{self, BufRead};

/// Maximum length of a single batch-run input line, in bytes (16 KiB).
/// The cap is enforced incrementally during read so a pathological
/// multi-GB single line cannot exhaust memory before tokenization.
pub(crate) const MAX_LINE_LEN: usize = 16 * 1024;

/// One parsed line: line number (1-based), the original raw text, and
/// either the tokenized argv (with `"s7cmd"` as argv[0]) or a
/// pre-formatted single-line tokenization error message. Per-line
/// tokenization errors are surfaced as `TokenizeError` rather than
/// bailing the whole `read_all` pass so they can be counted toward
/// `--max-errors` / `--continue-on-error` like any other failed line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLine {
    pub line_no: usize,
    pub raw: String,
    pub kind: ParsedLineKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedLineKind {
    /// Successful tokenization. The vector starts with `"s7cmd"`.
    Ok(Vec<String>),
    /// Tokenization failed (unbalanced quotes, etc.). The string carries
    /// only the underlying error text — no `"line N:"` prefix and no raw
    /// line tail. Both are attached at log time as structured fields.
    TokenizeError(String),
}

/// Tokenize a single line. Returns:
///   - `Ok(Some(argv))` if the line is a real command (with `"s7cmd"` prepended)
///   - `Ok(None)` if the line is blank or a comment
///   - `Err` if shlex rejects it (unbalanced quotes, etc.)
pub fn tokenize_line(line: &str) -> Result<Option<Vec<String>>> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(None);
    }
    let tokens = shlex::split(trimmed).ok_or_else(|| anyhow!("malformed quoting"))?;
    if tokens.is_empty() {
        return Ok(None);
    }
    let mut argv = Vec::with_capacity(tokens.len() + 1);
    argv.push("s7cmd".to_string());
    argv.extend(tokens);
    Ok(Some(argv))
}

/// Read the whole reader, returning a Vec of ParsedLine. Read I/O
/// errors (including lines longer than `MAX_LINE_LEN` and non-UTF-8
/// content) still bail out with the line number in the message —
/// recovering past those is not safe. Per-line tokenize errors are
/// surfaced as `ParsedLineKind::TokenizeError` so the executor can
/// count them like any other per-line failure.
pub fn read_all<R: BufRead>(mut reader: R) -> Result<Vec<ParsedLine>> {
    let mut out = Vec::new();
    let mut line_no: usize = 0;
    loop {
        line_no += 1;
        let line = match read_line_capped(&mut reader) {
            Ok(Some(s)) => s,
            Ok(None) => break,
            Err(e) => return Err(anyhow!("read error at line {line_no}: {e}")),
        };
        match tokenize_line(&line) {
            Ok(Some(argv)) => out.push(ParsedLine {
                line_no,
                raw: line,
                kind: ParsedLineKind::Ok(argv),
            }),
            Ok(None) => continue,
            Err(e) => {
                // Reason is the underlying error only; line_no and raw are
                // attached at log time as structured fields.
                out.push(ParsedLine {
                    line_no,
                    raw: line,
                    kind: ParsedLineKind::TokenizeError(e.to_string()),
                });
            }
        }
    }
    Ok(out)
}

/// Read one line from `reader`, capped at `MAX_LINE_LEN` bytes (excluding
/// the terminating `\n`). Returns:
///   - `Ok(Some(line))` for a successfully read line (newline stripped)
///   - `Ok(None)` at EOF before any byte was read
///   - `Err(InvalidData)` if the line exceeds the cap or contains
///     non-UTF-8 bytes
///
/// The cap is enforced incrementally via `BufRead::fill_buf` /
/// `consume`, so a single multi-GB line is rejected after roughly one
/// buffer's worth of memory rather than after fully buffering the line.
pub(crate) fn read_line_capped<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut buf: Vec<u8> = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if buf.is_empty() {
                Ok(None)
            } else {
                String::from_utf8(buf)
                    .map(Some)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            };
        }
        match available.iter().position(|&b| b == b'\n') {
            Some(idx) => {
                buf.extend_from_slice(&available[..idx]);
                reader.consume(idx + 1);
                if buf.len() > MAX_LINE_LEN {
                    return Err(too_long_err());
                }
                return String::from_utf8(buf)
                    .map(Some)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
            }
            None => {
                buf.extend_from_slice(available);
                let n = available.len();
                reader.consume(n);
                if buf.len() > MAX_LINE_LEN {
                    return Err(too_long_err());
                }
            }
        }
    }
}

pub(crate) fn too_long_err() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("line exceeds {}-byte limit (16 KiB)", MAX_LINE_LEN),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_simple_command() {
        let argv = tokenize_line("ls s3://bucket").unwrap().unwrap();
        assert_eq!(argv, vec!["s7cmd", "ls", "s3://bucket"]);
    }

    #[test]
    fn tokenize_quoted_argument_with_spaces() {
        let argv = tokenize_line(r#"cp "s3://b/with spaces/key" /tmp/dst"#)
            .unwrap()
            .unwrap();
        assert_eq!(
            argv,
            vec!["s7cmd", "cp", "s3://b/with spaces/key", "/tmp/dst"]
        );
    }

    #[test]
    fn skip_blank_line() {
        assert!(tokenize_line("").unwrap().is_none());
        assert!(tokenize_line("   ").unwrap().is_none());
    }

    #[test]
    fn skip_comment_line() {
        assert!(tokenize_line("# this is a comment").unwrap().is_none());
        assert!(tokenize_line("   # leading spaces ok").unwrap().is_none());
    }

    #[test]
    fn malformed_quoting_errors() {
        let err = tokenize_line(r#"cp "unterminated"#).unwrap_err();
        assert!(err.to_string().contains("malformed"));
    }

    #[test]
    fn read_all_collects_with_line_numbers_and_skips_blanks() {
        let input = "ls s3://b1\n\n# skipme\ncp /a s3://b2/k\n";
        let lines = read_all(input.as_bytes()).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].line_no, 1);
        assert!(
            matches!(&lines[0].kind, ParsedLineKind::Ok(a) if a == &vec!["s7cmd".to_string(), "ls".to_string(), "s3://b1".to_string()])
        );
        assert_eq!(lines[1].line_no, 4);
        assert!(
            matches!(&lines[1].kind, ParsedLineKind::Ok(a) if a == &vec!["s7cmd".to_string(), "cp".to_string(), "/a".to_string(), "s3://b2/k".to_string()])
        );
    }

    #[test]
    fn read_all_surfaces_tokenize_error_as_kind_and_continues() {
        // Per-line tokenize errors used to bail the whole pass; now they
        // appear as a `TokenizeError` entry, and the next line still
        // parses. This is what lets `--max-errors` count tokenize
        // failures alongside runtime ones.
        let input = "ls s3://b1\ncp \"oops\nls s3://b2\n";
        let lines = read_all(input.as_bytes()).unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].line_no, 1);
        assert!(matches!(lines[0].kind, ParsedLineKind::Ok(_)));
        assert_eq!(lines[1].line_no, 2);
        match &lines[1].kind {
            ParsedLineKind::TokenizeError(msg) => {
                // The reason is the underlying tokenize error text only —
                // no "line N:" prefix and no raw line tail (those are
                // attached at log time as structured fields).
                assert!(msg.contains("malformed"), "msg: {msg}");
                assert!(
                    !msg.starts_with("line "),
                    "reason must not bake line prefix: {msg}"
                );
            }
            other => panic!("expected TokenizeError, got {other:?}"),
        }
        assert_eq!(lines[2].line_no, 3);
        assert!(matches!(lines[2].kind, ParsedLineKind::Ok(_)));
    }

    #[test]
    fn read_all_empty_input_returns_empty() {
        let lines = read_all("".as_bytes()).unwrap();
        assert!(lines.is_empty());
    }

    #[test]
    fn tokenize_unicode_argument() {
        let argv = tokenize_line("rm s3://b/файл").unwrap().unwrap();
        assert_eq!(argv, vec!["s7cmd", "rm", "s3://b/файл"]);
    }

    // ---- line-length cap ----

    #[test]
    fn read_all_accepts_line_at_max_len() {
        // Comment line of exactly MAX_LINE_LEN bytes (`#` + (MAX-1) padding).
        // Tokenize_line skips it as a comment, so output is empty — the
        // assertion is "no error". This is the boundary that must succeed.
        let mut s = String::with_capacity(MAX_LINE_LEN + 1);
        s.push('#');
        s.extend(std::iter::repeat_n('x', MAX_LINE_LEN - 1));
        s.push('\n');
        assert_eq!(s.len(), MAX_LINE_LEN + 1);
        let lines = read_all(s.as_bytes()).unwrap();
        assert!(lines.is_empty(), "comment line should be skipped");
    }

    #[test]
    fn read_all_rejects_line_over_max_len() {
        // MAX_LINE_LEN + 1 bytes before the trailing newline.
        let mut s = String::with_capacity(MAX_LINE_LEN + 3);
        s.push('#');
        s.extend(std::iter::repeat_n('x', MAX_LINE_LEN));
        s.push('\n');
        let err = read_all(s.as_bytes()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("line 1"), "msg: {msg}");
        assert!(msg.contains("exceeds"), "msg: {msg}");
    }

    #[test]
    fn read_all_rejects_long_line_without_terminator() {
        // No trailing newline — limit must still apply (don't loop forever
        // and don't OOM). MAX_LINE_LEN + 1 raw bytes.
        let mut s = String::with_capacity(MAX_LINE_LEN + 1);
        s.push('#');
        s.extend(std::iter::repeat_n('x', MAX_LINE_LEN));
        let err = read_all(s.as_bytes()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("line 1"), "msg: {msg}");
        assert!(msg.contains("exceeds"), "msg: {msg}");
    }

    #[test]
    fn read_all_long_line_after_short_lines_reports_correct_line_no() {
        let mut s = String::new();
        s.push_str("# ok\n");
        s.push_str("# also ok\n");
        s.push('#');
        s.extend(std::iter::repeat_n('x', MAX_LINE_LEN));
        s.push('\n');
        let err = read_all(s.as_bytes()).unwrap_err();
        assert!(err.to_string().contains("line 3"));
    }

    #[test]
    fn read_line_capped_returns_none_at_eof() {
        let mut r = io::Cursor::new(b"" as &[u8]);
        assert!(read_line_capped(&mut r).unwrap().is_none());
    }

    #[test]
    fn read_line_capped_handles_final_line_without_newline() {
        let mut r = io::Cursor::new(b"hello" as &[u8]);
        assert_eq!(read_line_capped(&mut r).unwrap().unwrap(), "hello");
        assert!(read_line_capped(&mut r).unwrap().is_none());
    }

    #[test]
    fn read_line_capped_rejects_invalid_utf8() {
        let mut r = io::Cursor::new(&[0xff, 0xfe, b'\n'][..]);
        let err = read_line_capped(&mut r).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
