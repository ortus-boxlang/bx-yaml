//! Writes a value tree as YAML in the layout SnakeYAML uses for block style: sequences are not indented under their key,
//! nested mappings are indented two spaces, and a string is quoted whenever a YAML reader could take it for something else.

use crate::model::{DateTimeValue, Failure, Kind, Result, Value};
use crate::read;
use crate::zone::offset_seconds;
use chrono::{DateTime, Datelike, Timelike};

/// A serialization failure with `message`.
fn fail(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Serialization, message)
}

/// The YAML document for `value`.
pub fn write(value: &Value) -> Result<String> {
    let mut out = String::new();
    match value {
        Value::Table(entries) if !entries.is_empty() => map(entries, 0, &mut out)?,
        Value::Array(items) if !items.is_empty() => sequence(items, 0, &mut out)?,
        other => match scalar(other)? {
            Scalar::Inline(text) => {
                out.push_str(&text);
                out.push('\n');
            }
            Scalar::Block { header, lines } => {
                out.push_str(&header);
                out.push('\n');
                for line in lines {
                    block_line(&line, 2, &mut out);
                }
            }
        },
    }
    Ok(out)
}

/// A scalar as it is written after `key:` or `- `.
enum Scalar {
    /// The text of a one-line scalar.
    Inline(String),
    /// A `|` literal block: the header (`|`, `|-` or `|+`) and its lines.
    Block {
        /// The block header: `|`, `|-` or `|+`.
        header: String,
        /// The lines of the block.
        lines: Vec<String>,
    },
}

/// A value as a scalar. Empty collections are written inline as `[]` and `{}`.
fn scalar(value: &Value) -> Result<Scalar> {
    Ok(Scalar::Inline(match value {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => float(*f),
        Value::DateTime(dt) => format!("'{}'", iso(dt)?),
        Value::Str(s) => return Ok(string(s)),
        Value::Array(_) => "[]".into(),
        Value::Table(_) => "{}".into(),
    }))
}

/// `width` spaces.
fn pad(width: usize) -> String {
    " ".repeat(width)
}

/// Appends one line of a block scalar at `indent`, leaving an empty line empty.
fn block_line(line: &str, indent: usize, out: &mut String) {
    if !line.is_empty() {
        out.push_str(&pad(indent));
        out.push_str(line);
    }
    out.push('\n');
}

/// A mapping key as text, quoting it when it could be misread.
fn key(name: &str) -> String {
    match string(name) {
        Scalar::Inline(text) => text,
        // A multi-line key is written in double quotes.
        Scalar::Block { .. } => double_quoted(name),
    }
}

/// Writes a mapping's entries at `indent`, nesting collections beneath their keys.
fn map(entries: &[(String, Value)], indent: usize, out: &mut String) -> Result<()> {
    for (name, value) in entries {
        out.push_str(&pad(indent));
        out.push_str(&key(name));
        out.push(':');
        match value {
            Value::Table(inner) if !inner.is_empty() => {
                out.push('\n');
                map(inner, indent + 2, out)?;
            }
            Value::Array(items) if !items.is_empty() => {
                out.push('\n');
                sequence(items, indent, out)?;
            }
            other => match scalar(other)? {
                Scalar::Inline(text) => {
                    out.push(' ');
                    out.push_str(&text);
                    out.push('\n');
                }
                Scalar::Block { header, lines } => {
                    out.push(' ');
                    out.push_str(&header);
                    out.push('\n');
                    for line in lines {
                        block_line(&line, indent + 2, out);
                    }
                }
            },
        }
    }
    Ok(())
}

/// Writes a sequence's items at `indent`, one `- ` entry each.
fn sequence(items: &[Value], indent: usize, out: &mut String) -> Result<()> {
    for item in items {
        match item {
            Value::Table(inner) if !inner.is_empty() => nested(|text| map(inner, indent + 2, text), indent, out)?,
            Value::Array(inner) if !inner.is_empty() => nested(|text| sequence(inner, indent + 2, text), indent, out)?,
            other => {
                out.push_str(&pad(indent));
                out.push('-');
                match scalar(other)? {
                    Scalar::Inline(text) => {
                        out.push(' ');
                        out.push_str(&text);
                        out.push('\n');
                    }
                    Scalar::Block { header, lines } => {
                        out.push(' ');
                        out.push_str(&header);
                        out.push('\n');
                        for line in lines {
                            block_line(&line, indent + 2, out);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Writes a collection that is a sequence item: its first line starts with `- ` where the indent would be.
fn nested(body: impl FnOnce(&mut String) -> Result<()>, indent: usize, out: &mut String) -> Result<()> {
    let mut text = String::new();
    body(&mut text)?;
    let dash = format!("{}- ", pad(indent));
    let first_pad = pad(indent + 2);
    out.push_str(&dash);
    out.push_str(text.strip_prefix(first_pad.as_str()).unwrap_or(&text));
    Ok(())
}

/// A float that reads back as a float: `1.5`, `2.0`, `1.0e+300`, `.inf`, `.nan`.
fn float(value: f64) -> String {
    if value.is_nan() {
        return ".nan".into();
    }
    if value.is_infinite() {
        return if value > 0.0 { ".inf" } else { "-.inf" }.into();
    }
    let magnitude = value.abs();
    if magnitude == 0.0 || (1e-4..1e16).contains(&magnitude) {
        let text = format!("{value}");
        return if text.contains('.') { text } else { format!("{text}.0") };
    }
    let scientific = format!("{value:e}");
    let (mantissa, exponent) = scientific.split_once('e').expect("scientific notation has an exponent");
    let mantissa = if mantissa.contains('.') { mantissa.to_string() } else { format!("{mantissa}.0") };
    let sign = if exponent.starts_with('-') { "" } else { "+" };
    format!("{mantissa}e{sign}{exponent}")
}

/// An ISO 8601 date and time in the value's own zone, `Z` when it has no offset: `2024-03-05T06:07:08.5+05:30`.
fn iso(value: &DateTimeValue) -> Result<String> {
    let offset = offset_seconds(&value.zone, value.seconds)?;
    let wall = DateTime::from_timestamp(value.seconds + i64::from(offset), value.nanos)
        .ok_or_else(|| fail(format!("Date out of range: {} seconds", value.seconds)))?
        .naive_utc();
    let mut text = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        wall.year(),
        wall.month(),
        wall.day(),
        wall.hour(),
        wall.minute(),
        wall.second()
    );
    if wall.nanosecond() != 0 {
        text.push('.');
        text.push_str(format!("{:09}", wall.nanosecond()).trim_end_matches('0'));
    }
    if offset == 0 {
        text.push('Z');
    } else {
        let total = offset.abs();
        text.push_str(&format!("{}{:02}:{:02}", if offset < 0 { '-' } else { '+' }, total / 3600, total % 3600 / 60));
    }
    Ok(text)
}

/// A string as plain text when a reader would read it back as the same string, otherwise quoted or as a literal block.
fn string(text: &str) -> Scalar {
    if text.chars().any(needs_escape) {
        return Scalar::Inline(double_quoted(text));
    }
    if text.contains('\n') {
        let lines: Vec<&str> = text.split('\n').collect();
        let usable = !text.starts_with(['\n', ' ']) && lines.iter().all(|line| !line.ends_with(' ') && !line.starts_with('\t'));
        if !usable {
            return Scalar::Inline(double_quoted(text));
        }
        let trailing = text.len() - text.trim_end_matches('\n').len();
        let header = match trailing {
            0 => "|-",
            1 => "|",
            _ => "|+",
        };
        let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
        // The break after the last line belongs to the header's chomping, not to a line of its own.
        if trailing > 0 {
            lines.pop();
        }
        return Scalar::Block { header: header.into(), lines };
    }
    if is_plain_safe(text) {
        Scalar::Inline(text.to_string())
    } else {
        Scalar::Inline(format!("'{}'", text.replace('\'', "''")))
    }
}

/// Characters a quoted scalar must escape: controls, tabs, line separators and the byte order mark.
fn needs_escape(c: char) -> bool {
    (c as u32) < 0x20 && c != '\n' || matches!(c as u32, 0x7f..=0x9f | 0x2028 | 0x2029 | 0xfeff)
}

/// Text in double quotes, escaping quotes, backslashes and control characters.
fn double_quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if needs_escape(c) => {
                if (c as u32) <= 0xff {
                    out.push_str(&format!("\\x{:02x}", c as u32));
                } else {
                    out.push_str(&format!("\\u{:04x}", c as u32));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Whether `text` can be written without quotes and still read back as the same string, by a YAML 1.1 or 1.2 reader.
fn is_plain_safe(text: &str) -> bool {
    if text.is_empty() || text.starts_with([' ', '\t']) || text.ends_with([' ', '\t']) {
        return false;
    }
    let first = text.chars().next().expect("not empty");
    if matches!(first, ',' | '[' | ']' | '{' | '}' | '#' | '&' | '*' | '!' | '|' | '>' | '\'' | '"' | '%' | '@' | '`') {
        return false;
    }
    // `-`, `?` and `:` start a plain scalar only when something other than a space follows.
    if matches!(first, '-' | '?' | ':') && (text.len() == 1 || text[1..].starts_with(' ')) {
        return false;
    }
    if text.contains(": ") || text.contains(" #") || text.ends_with(':') || text == "<<" || text == "=" {
        return false;
    }
    !looks_typed(text)
}

/// Text a reader could take for null, a boolean, a number or a date.
fn looks_typed(text: &str) -> bool {
    // YAML 1.1 also reads these as booleans. A lone `y` or `n` is left alone, as SnakeYAML leaves it.
    const BOOLEAN_WORDS: [&str; 22] = [
        "yes", "Yes", "YES", "no", "No", "NO", "on", "On", "ON", "off", "Off", "OFF", "true", "True", "TRUE", "false", "False", "FALSE",
        "null", "Null", "NULL", "~",
    ];
    if BOOLEAN_WORDS.contains(&text) {
        return true;
    }
    // Anything the reader types, and anything YAML 1.1 types: underscores, binary, a leading-zero octal, sexagesimal.
    if !matches!(read::read(text), Ok(Value::Str(_))) {
        return true;
    }
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    let first = digits.chars().next();
    if first.is_some_and(|c| c.is_ascii_digit() || c == '.') {
        return digits.starts_with("0b")
            || digits.chars().all(|c| c.is_ascii_digit() || matches!(c, '_' | ',' | ':' | '.' | 'e' | 'E' | '+' | '-'))
            || text.starts_with("0o")
            || text.starts_with("0x");
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper: a string value.
    fn s(text: &str) -> Value {
        Value::Str(text.into())
    }
    /// Test helper: a table value from `(key, value)` pairs.
    fn table(entries: Vec<(&str, Value)>) -> Value {
        Value::Table(entries.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    /// The layout matches SnakeYAML's block style.
    #[test]
    fn layout_matches_snakeyaml_block_style() {
        let doc = table(vec![
            ("a", s("x")),
            ("list", Value::Array(vec![Value::Int(1), s("two"), table(vec![("k", s("v")), ("n", Value::Int(2))])])),
            ("nested", table(vec![("b", table(vec![("c", Value::Int(1))]))])),
            ("m", Value::Array(vec![table(vec![("a", Value::Int(1)), ("b", Value::Array(vec![Value::Int(1), Value::Int(2)]))]), Value::Array(vec![Value::Int(3), Value::Array(vec![Value::Int(4)])])])),
            ("e", Value::Array(vec![])),
            ("t", table(vec![])),
            ("flag", Value::Bool(true)),
            ("nothing", Value::Null),
        ]);
        assert_eq!(
            write(&doc).unwrap(),
            "a: x\nlist:\n- 1\n- two\n- k: v\n  n: 2\nnested:\n  b:\n    c: 1\nm:\n- a: 1\n  b:\n  - 1\n  - 2\n- - 3\n  - - 4\ne: []\nt: {}\nflag: true\nnothing: null\n"
        );
    }

    /// The root can be a scalar, a sequence or a mapping.
    #[test]
    fn roots_may_be_any_value() {
        assert_eq!(write(&s("just text")).unwrap(), "just text\n");
        assert_eq!(write(&Value::Int(42)).unwrap(), "42\n");
        assert_eq!(write(&Value::Null).unwrap(), "null\n");
        assert_eq!(write(&Value::Array(vec![Value::Int(1), Value::Int(2)])).unwrap(), "- 1\n- 2\n");
        assert_eq!(write(&Value::Array(vec![])).unwrap(), "[]\n");
        assert_eq!(write(&table(vec![])).unwrap(), "{}\n");
    }

    /// Text that would read back as another type, or break the syntax, is quoted.
    #[test]
    fn strings_are_quoted_when_they_could_be_misread() {
        let doc = table(vec![
            ("a", s("true")), ("b", s("null")), ("c", s("~")), ("d", s("1.5")), ("e", s("0x1f")), ("f", s("2024-03-05")), ("g", s("on")),
            ("h", s("y")), ("i", s("no")), ("j", s("- x")), ("k", s("x: y")), ("l", s("x:")), ("m", s(" lead")), ("n", s("trail ")),
            ("o", s("")), ("p", s("it's")), ("q", s("q\"d")), ("r", s("x,y")), ("s", s("'x'")), ("t", s("\"x\"")), ("u", s("-x")),
            ("v", s("tab\tx")), ("w", s("1_000")), ("x", s("012")), ("y", s("1:30")), ("z", s("*a")),
        ]);
        assert_eq!(
            write(&doc).unwrap(),
            "a: 'true'\nb: 'null'\nc: '~'\nd: '1.5'\ne: '0x1f'\nf: '2024-03-05'\ng: 'on'\nh: y\ni: 'no'\nj: '- x'\nk: 'x: y'\nl: 'x:'\nm: ' lead'\nn: 'trail '\no: ''\np: it's\nq: q\"d\nr: x,y\ns: '''x'''\nt: '\"x\"'\nu: -x\nv: \"tab\\tx\"\nw: '1_000'\nx: '012'\ny: '1:30'\nz: '*a'\n"
        );
    }

    /// Multi-line text is written as a `|` literal block.
    #[test]
    fn multi_line_strings_are_literal_blocks() {
        let doc = table(vec![("a", s("line1\nline2")), ("b", s("a\n")), ("c", s("a\n\nb")), ("d", s("a\n\n")), ("e", s("\na")), ("f", s("x \ny"))]);
        assert_eq!(
            write(&doc).unwrap(),
            "a: |-\n  line1\n  line2\nb: |\n  a\nc: |-\n  a\n\n  b\nd: |+\n  a\n\ne: \"\\na\"\nf: \"x \\ny\"\n"
        );
    }

    /// Numbers, dates and unusual keys are written as the JVM module writes them.
    #[test]
    fn numbers_dates_and_odd_keys() {
        let dt = Value::DateTime(DateTimeValue { seconds: 1_709_599_028, nanos: 500_000_000, zone: "+05:30".into() });
        let utc = Value::DateTime(DateTimeValue { seconds: 1_709_618_828, nanos: 0, zone: "UTC".into() });
        let doc = table(vec![
            ("big", Value::Int(12_345_678_901)), ("neg", Value::Int(-7)), ("f", Value::Float(1.5)), ("whole", Value::Float(2.0)),
            ("tiny", Value::Float(0.00001)), ("huge", Value::Float(1e300)), ("inf", Value::Float(f64::INFINITY)), ("nan", Value::Float(f64::NAN)),
            ("dt", dt), ("utc", utc), ("has space", Value::Int(1)), ("a:b", Value::Int(2)), ("", Value::Int(3)), ("true", Value::Int(4)),
        ]);
        assert_eq!(
            write(&doc).unwrap(),
            "big: 12345678901\nneg: -7\nf: 1.5\nwhole: 2.0\ntiny: 1.0e-5\nhuge: 1.0e+300\ninf: .inf\nnan: .nan\ndt: '2024-03-05T06:07:08.5+05:30'\nutc: '2024-03-05T06:07:08Z'\nhas space: 1\na:b: 2\n'': 3\n'true': 4\n"
        );
    }

    /// Written scalars read back as the same values.
    #[test]
    fn what_is_written_reads_back_the_same() {
        let strings = [
            "", " ", "a", "yes", "No", "y", "n", "true", "null", "~", "1", "-1", "+1", "1.5", ".5", "1e3", "1_000", "0x1f", "0o17", "0b101", "012", "1:30", ".inf",
            "2024-03-05", "2024-03-05 06:07:08", "- x", "-", "?", "? x", ":", ": x", "x:", "x: y", "x #c", "#c", "&a", "*a", "!t", "|x", ">x", "%x", "@x", "`x",
            "[x", "]", "{x", "}", "x,y", "'x'", "\"x\"", "it's", "tab\tx", "a\nb", "a\n", "\na", "a\n\n", " a\nb", "a \nb", "é😀", "<<", "=", "a\u{0}b", "line1\r\nline2",
        ];
        for text in strings {
            let value = table(vec![("k", s(text))]);
            let written = write(&value).unwrap_or_else(|e| panic!("{text:?}: {}", e.message));
            assert_eq!(read::read(&written).unwrap_or_else(|e| panic!("{text:?} wrote {written:?}: {}", e.message)), value, "{text:?} wrote {written:?}");
            let in_list = Value::Array(vec![s(text)]);
            let written = write(&in_list).unwrap();
            assert_eq!(read::read(&written).unwrap(), in_list, "{text:?} in a list wrote {written:?}");
        }
        let keys = ["", "a b", "a:b", "x: y", "1", "true", "null", "é", "-x", "a\nb", "'q'"];
        for text in keys {
            if matches!(text, "null") {
                continue;
            }
            let value = table(vec![(text, Value::Int(1))]);
            let written = write(&value).unwrap();
            assert_eq!(read::read(&written).unwrap_or_else(|e| panic!("key {text:?} wrote {written:?}: {}", e.message)), value, "key {text:?} wrote {written:?}");
        }
    }

    /// Nested structures read back as the same values.
    #[test]
    fn nested_structures_read_back_the_same() {
        let doc = table(vec![
            ("name", s("demo")),
            ("ports", Value::Array(vec![Value::Int(80), Value::Int(443)])),
            ("servers", Value::Array(vec![table(vec![("host", s("a")), ("tags", Value::Array(vec![s("x"), s("y")]))]), table(vec![("host", s("b"))])])),
            ("matrix", Value::Array(vec![Value::Array(vec![Value::Int(1), Value::Int(2)]), Value::Array(vec![Value::Int(3)])])),
            ("deep", table(vec![("a", table(vec![("b", table(vec![("c", Value::Float(0.5))]))]))])),
            ("empty", Value::Array(vec![])),
            ("none", Value::Null),
        ]);
        let written = write(&doc).unwrap();
        assert_eq!(read::read(&written).unwrap(), doc, "{written}");
    }
}
