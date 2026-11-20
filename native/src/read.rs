//! Reads YAML text into a value tree, following the JVM module's SnakeYAML behavior where the two readers differ:
//! one document only, duplicate keys are errors, `<<` merges mappings, a plain date or timestamp is a date, and
//! aliases are limited. Plain scalars resolve by the YAML 1.2 core schema, so `yes` and `on` are strings here.

use crate::model::{Failure, Kind, Result, Value};
use crate::timestamp;
use saphyr_parser::{Event, Parser, ScalarStyle};
use std::collections::HashMap;

/// SnakeYAML's default: aliases to mappings or sequences beyond this many are refused, which stops "billion laughs" input.
const MAX_COLLECTION_ALIASES: usize = 50;
const MAX_DEPTH: usize = 200;

fn fail(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Parse, message)
}

/// A node as the parser saw it, before scalars are given a type.
#[derive(Clone)]
enum Node {
    Scalar { text: String, style: ScalarStyle, tag: Option<String> },
    Sequence(Vec<Node>),
    Mapping(Vec<(Node, Node)>),
}

/// Reads one YAML document. An empty document is null.
pub fn read(text: &str) -> Result<Value> {
    let mut events = Vec::new();
    for event in Parser::new_from_str(text) {
        let (event, span) = event.map_err(|error| fail(error.to_string()))?;
        events.push((event, span.start.line()));
    }
    let mut builder = Builder { events, position: 0, anchors: HashMap::new(), collection_aliases: 0 };
    builder.document()
}

struct Builder<'a> {
    events: Vec<(Event<'a>, usize)>,
    position: usize,
    anchors: HashMap<usize, Node>,
    collection_aliases: usize,
}

impl<'a> Builder<'a> {
    fn next(&mut self) -> Result<Event<'a>> {
        let event = self.events.get(self.position).map(|(event, _)| event.clone()).ok_or_else(|| fail("Unexpected end of YAML"))?;
        self.position += 1;
        Ok(event)
    }

    fn peek_is_end(&self, end: &Event<'_>) -> bool {
        self.events.get(self.position).is_some_and(|(event, _)| event == end)
    }

    fn document(&mut self) -> Result<Value> {
        let mut documents = Vec::new();
        loop {
            match self.next()? {
                Event::StreamStart => {}
                Event::StreamEnd => break,
                Event::DocumentStart(_) => {
                    let node = self.node(0)?;
                    if self.next()? != Event::DocumentEnd {
                        return Err(fail("Malformed YAML document"));
                    }
                    documents.push(node);
                }
                other => return Err(fail(format!("Unexpected YAML event: {other:?}"))),
            }
        }
        match documents.len() {
            0 => Ok(Value::Null),
            1 => convert(&documents[0], 0),
            _ => Err(fail("expected a single document in the stream, but found another document")),
        }
    }

    fn node(&mut self, depth: usize) -> Result<Node> {
        if depth > MAX_DEPTH {
            return Err(fail("YAML is nested too deeply"));
        }
        match self.next()? {
            Event::Scalar(text, style, anchor, tag) => {
                let node = Node::Scalar { text: text.into_owned(), style, tag: tag.map(|t| tag_name(&t.handle, &t.suffix)) };
                self.remember(anchor, &node);
                Ok(node)
            }
            Event::SequenceStart(anchor, tag) => {
                let tag = tag.map(|t| tag_name(&t.handle, &t.suffix));
                if tag.as_deref().is_some_and(|t| t != "!!seq") {
                    return Err(fail(format!("Unsupported YAML tag on a sequence: {}", tag.unwrap_or_default())));
                }
                let mut items = Vec::new();
                while !self.peek_is_end(&Event::SequenceEnd) {
                    items.push(self.node(depth + 1)?);
                }
                self.next()?;
                let node = Node::Sequence(items);
                self.remember(anchor, &node);
                Ok(node)
            }
            Event::MappingStart(anchor, tag) => {
                let tag = tag.map(|t| tag_name(&t.handle, &t.suffix));
                if tag.as_deref().is_some_and(|t| t != "!!map") {
                    return Err(fail(format!("Unsupported YAML tag on a mapping: {}", tag.unwrap_or_default())));
                }
                let mut pairs = Vec::new();
                while !self.peek_is_end(&Event::MappingEnd) {
                    let key = self.node(depth + 1)?;
                    let value = self.node(depth + 1)?;
                    pairs.push((key, value));
                }
                self.next()?;
                let node = Node::Mapping(pairs);
                self.remember(anchor, &node);
                Ok(node)
            }
            Event::Alias(id) => {
                let node = self.anchors.get(&id).cloned().ok_or_else(|| fail("found undefined alias"))?;
                if !matches!(node, Node::Scalar { .. }) {
                    self.collection_aliases += 1;
                    if self.collection_aliases > MAX_COLLECTION_ALIASES {
                        return Err(fail(format!(
                            "Number of aliases for non-scalar nodes exceeds the specified max={MAX_COLLECTION_ALIASES}"
                        )));
                    }
                }
                Ok(node)
            }
            other => Err(fail(format!("Unexpected YAML event: {other:?}"))),
        }
    }

    fn remember(&mut self, anchor: usize, node: &Node) {
        if anchor != 0 {
            self.anchors.insert(anchor, node.clone());
        }
    }
}

/// `!!str`, or `!name` for a local tag.
fn tag_name(handle: &str, suffix: &str) -> String {
    if handle == "tag:yaml.org,2002:" || handle == "!!" { format!("!!{suffix}") } else { format!("{handle}{suffix}") }
}

fn convert(node: &Node, depth: usize) -> Result<Value> {
    if depth > MAX_DEPTH {
        return Err(fail("YAML is nested too deeply"));
    }
    match node {
        Node::Scalar { text, style, tag } => scalar(text, *style, tag.as_deref()),
        Node::Sequence(items) => Ok(Value::Array(items.iter().map(|item| convert(item, depth + 1)).collect::<Result<_>>()?)),
        Node::Mapping(pairs) => mapping(pairs, depth).map(Value::Table),
    }
}

fn is_merge_key(node: &Node) -> bool {
    matches!(node, Node::Scalar { text, style: ScalarStyle::Plain, tag: None } if text == "<<")
}

fn mapping(pairs: &[(Node, Node)], depth: usize) -> Result<Vec<(String, Value)>> {
    let mut entries: Vec<(String, Value)> = Vec::new();
    let mut merges = Vec::new();
    for (key, value) in pairs {
        if is_merge_key(key) {
            merges.push(value);
            continue;
        }
        let name = key_text(key)?;
        if entries.iter().any(|(existing, _)| *existing == name) {
            return Err(fail(format!("while constructing a mapping: found duplicate key {name}")));
        }
        entries.push((name, convert(value, depth + 1)?));
    }
    // `<<: *base` or `<<: [*a, *b]`: keys already written win, and so do earlier merges over later ones.
    for source in merges {
        let tables: Vec<&Node> = match source {
            Node::Mapping(_) => vec![source],
            Node::Sequence(items) if items.iter().all(|item| matches!(item, Node::Mapping(_))) => items.iter().collect(),
            _ => return Err(fail("while constructing a mapping: a merge key needs a mapping or a sequence of mappings")),
        };
        for table in tables {
            let Node::Mapping(inner) = table else { continue };
            for (name, value) in mapping(inner, depth + 1)? {
                if !entries.iter().any(|(existing, _)| *existing == name) {
                    entries.push((name, value));
                }
            }
        }
    }
    Ok(entries)
}

/// A mapping key as text. A number or boolean key reads as the text it was written with.
fn key_text(key: &Node) -> Result<String> {
    match key {
        Node::Scalar { text, style, tag } => {
            if matches!(scalar(text, *style, tag.as_deref())?, Value::Null) {
                return Err(fail("while constructing a mapping: a mapping key must not be null"));
            }
            Ok(text.clone())
        }
        _ => Err(fail("while constructing a mapping: a mapping key must be a scalar")),
    }
}

/// A scalar's value: quoted and block scalars are strings, a tag says what it is, and a plain one is resolved.
fn scalar(text: &str, style: ScalarStyle, tag: Option<&str>) -> Result<Value> {
    match tag {
        Some("!!str") => Ok(Value::Str(text.to_string())),
        Some("!!null") => Ok(Value::Null),
        Some("!!bool") => parse_bool(text).map(Value::Bool).ok_or_else(|| fail(format!("could not parse [{text}] as a boolean"))),
        Some("!!int") => parse_int(text).map(Value::Int).ok_or_else(|| fail(format!("could not parse [{text}] as an integer"))),
        Some("!!float") => parse_float(text).or_else(|| parse_int(text).map(|i| i as f64)).map(Value::Float).ok_or_else(|| fail(format!("could not parse [{text}] as a float"))),
        Some("!!timestamp") => timestamp::parse(text).map(Value::DateTime).ok_or_else(|| fail(format!("could not parse [{text}] as a timestamp"))),
        Some(other) => Err(fail(format!("Unsupported YAML tag: {other}"))),
        None if style != ScalarStyle::Plain => Ok(Value::Str(text.to_string())),
        None => Ok(resolve_plain(text)),
    }
}

fn resolve_plain(text: &str) -> Value {
    if matches!(text, "" | "~" | "null" | "Null" | "NULL") {
        return Value::Null;
    }
    if let Some(b) = parse_bool(text) {
        return Value::Bool(b);
    }
    if let Some(i) = parse_int(text) {
        return Value::Int(i);
    }
    if let Some(f) = parse_float(text) {
        return Value::Float(f);
    }
    if let Some(dt) = timestamp::parse(text) {
        return Value::DateTime(dt);
    }
    Value::Str(text.to_string())
}

fn parse_bool(text: &str) -> Option<bool> {
    match text {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

/// Decimal, `0o` octal and `0x` hex integers. Anything outside `i64` is not an integer here.
fn parse_int(text: &str) -> Option<i64> {
    if let Some(octal) = text.strip_prefix("0o") {
        return (!octal.is_empty() && octal.bytes().all(|b| (b'0'..=b'7').contains(&b))).then(|| i64::from_str_radix(octal, 8).ok()).flatten();
    }
    if let Some(hex) = text.strip_prefix("0x") {
        return (!hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit())).then(|| i64::from_str_radix(hex, 16).ok()).flatten();
    }
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn parse_float(text: &str) -> Option<f64> {
    match text {
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => return Some(f64::INFINITY),
        "-.inf" | "-.Inf" | "-.INF" => return Some(f64::NEG_INFINITY),
        ".nan" | ".NaN" | ".NAN" => return Some(f64::NAN),
        _ => {}
    }
    let body = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (mantissa, exponent) = match body.find(['e', 'E']) {
        Some(at) => (&body[..at], Some(&body[at + 1..])),
        None => (body, None),
    };
    let (whole, fraction) = match mantissa.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (mantissa, None),
    };
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    let mantissa_ok = match fraction {
        Some(fraction) => digits(whole) && digits(fraction) && !(whole.is_empty() && fraction.is_empty()),
        // Without a dot there must be an exponent, or it would have been an integer.
        None => !whole.is_empty() && digits(whole) && exponent.is_some(),
    };
    let exponent_ok = exponent.is_none_or(|e| {
        let digits_part = e.strip_prefix(['+', '-']).unwrap_or(e);
        !digits_part.is_empty() && digits_part.bytes().all(|b| b.is_ascii_digit())
    });
    if mantissa_ok && exponent_ok { text.parse().ok() } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(value: &Value) -> &Vec<(String, Value)> {
        match value {
            Value::Table(entries) => entries,
            other => panic!("not a table: {other:?}"),
        }
    }
    fn get<'a>(value: &'a Value, key: &str) -> &'a Value {
        &table(value).iter().find(|(k, _)| k == key).unwrap_or_else(|| panic!("no key {key}")).1
    }

    #[test]
    fn plain_scalars_resolve_by_the_core_schema() {
        let doc = read("i: 42\nneg: -7\nplus: +5\nhex: 0x1f\noct: 0o17\nlead: 017\nbig: 12345678901\nunder: 1_000\nf: 1.5\ne: 1e3\ndot: 3.\nhalf: .5\ninf: .inf\nninf: -.inf\nb: true\nB: True\nyes: yes\non: on\nn: ~\nnul: null\nempty:\ns: text\n").unwrap();
        assert_eq!(get(&doc, "i"), &Value::Int(42));
        assert_eq!(get(&doc, "neg"), &Value::Int(-7));
        assert_eq!(get(&doc, "plus"), &Value::Int(5));
        assert_eq!(get(&doc, "hex"), &Value::Int(31));
        assert_eq!(get(&doc, "oct"), &Value::Int(15));
        assert_eq!(get(&doc, "lead"), &Value::Int(17));
        assert_eq!(get(&doc, "big"), &Value::Int(12_345_678_901));
        assert_eq!(get(&doc, "under"), &Value::Str("1_000".into()));
        assert_eq!(get(&doc, "f"), &Value::Float(1.5));
        assert_eq!(get(&doc, "e"), &Value::Float(1000.0));
        assert_eq!(get(&doc, "dot"), &Value::Float(3.0));
        assert_eq!(get(&doc, "half"), &Value::Float(0.5));
        assert_eq!(get(&doc, "inf"), &Value::Float(f64::INFINITY));
        assert_eq!(get(&doc, "ninf"), &Value::Float(f64::NEG_INFINITY));
        assert_eq!(get(&doc, "b"), &Value::Bool(true));
        assert_eq!(get(&doc, "B"), &Value::Bool(true));
        assert_eq!(get(&doc, "yes"), &Value::Str("yes".into()));
        assert_eq!(get(&doc, "on"), &Value::Str("on".into()));
        for key in ["n", "nul", "empty"] {
            assert_eq!(get(&doc, key), &Value::Null, "{key}");
        }
        assert_eq!(get(&doc, "s"), &Value::Str("text".into()));
    }

    #[test]
    fn quoted_and_block_scalars_are_strings() {
        let doc = read("a: '123'\nb: \"true\"\nc: '2024-03-05'\nd: |\n  l1\n  l2\ne: >\n  f1\n  f2\nf: |-\n  x\ng: 'it''s'\nh: \"tab\\there\"\n").unwrap();
        assert_eq!(get(&doc, "a"), &Value::Str("123".into()));
        assert_eq!(get(&doc, "b"), &Value::Str("true".into()));
        assert_eq!(get(&doc, "c"), &Value::Str("2024-03-05".into()));
        assert_eq!(get(&doc, "d"), &Value::Str("l1\nl2\n".into()));
        assert_eq!(get(&doc, "e"), &Value::Str("f1 f2\n".into()));
        assert_eq!(get(&doc, "f"), &Value::Str("x".into()));
        assert_eq!(get(&doc, "g"), &Value::Str("it's".into()));
        assert_eq!(get(&doc, "h"), &Value::Str("tab\there".into()));
    }

    #[test]
    fn tags_choose_the_type() {
        let doc = read("a: !!str 123\nb: !!int '7'\nc: !!float 1\nd: !!bool 'true'\ne: !!null ''\n").unwrap();
        assert_eq!(get(&doc, "a"), &Value::Str("123".into()));
        assert_eq!(get(&doc, "b"), &Value::Int(7));
        assert_eq!(get(&doc, "c"), &Value::Float(1.0));
        assert_eq!(get(&doc, "d"), &Value::Bool(true));
        assert_eq!(get(&doc, "e"), &Value::Null);
        assert!(read("a: !custom 1\n").is_err());
        assert!(read("a: !!int nope\n").is_err());
    }

    #[test]
    fn plain_dates_are_dates_and_quoted_ones_are_not() {
        let doc = read("d: 2024-03-05\nt: 2024-03-05T06:07:08Z\nq: \"2024-03-05\"\n").unwrap();
        assert!(matches!(get(&doc, "d"), Value::DateTime(_)));
        assert!(matches!(get(&doc, "t"), Value::DateTime(dt) if dt.seconds == 1_709_618_828 && dt.zone == "UTC"));
        assert_eq!(get(&doc, "q"), &Value::Str("2024-03-05".into()));
    }

    #[test]
    fn documents_roots_and_emptiness() {
        assert_eq!(read("").unwrap(), Value::Null);
        assert_eq!(read("# only a comment\n").unwrap(), Value::Null);
        assert_eq!(read("just text").unwrap(), Value::Str("just text".into()));
        assert_eq!(read("- 1\n- two\n- [3, 4]\n- {a: b}\n").unwrap(), Value::Array(vec![
            Value::Int(1),
            Value::Str("two".into()),
            Value::Array(vec![Value::Int(3), Value::Int(4)]),
            Value::Table(vec![("a".into(), Value::Str("b".into()))]),
        ]));
        let error = read("a: 1\n---\nb: 2\n").unwrap_err();
        assert!(error.message.starts_with("expected a single document in the stream"), "{}", error.message);
        assert_eq!(error.kind, Kind::Parse);
    }

    #[test]
    fn keys_keep_order_and_scalar_keys_read_as_text() {
        let doc = read("b: 1\na: 2\n1: x\ntrue: y\n").unwrap();
        assert_eq!(table(&doc).iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(), ["b", "a", "1", "true"]);
        assert!(read("~: c\n").is_err());
        assert!(read("null: c\n").is_err());
        assert!(read("? [1, 2]\n: x\n").is_err());
    }

    #[test]
    fn duplicate_keys_are_errors() {
        let error = read("a: 1\na: 2\n").unwrap_err();
        assert!(error.message.contains("found duplicate key a"), "{}", error.message);
        assert!(read("a: {x: 1, x: 2}\n").is_err());
    }

    #[test]
    fn anchors_aliases_and_merge_keys() {
        let doc = read("base: &x\n  k: 1\n  j: 0\ncopy: *x\nc:\n  <<: *x\n  z: 2\n  k: 9\nmany:\n  <<: [*x, {k: 5, m: 6}]\n").unwrap();
        assert_eq!(get(get(&doc, "copy"), "k"), &Value::Int(1));
        let c = get(&doc, "c");
        assert_eq!((get(c, "k"), get(c, "j"), get(c, "z")), (&Value::Int(9), &Value::Int(0), &Value::Int(2)));
        let many = get(&doc, "many");
        assert_eq!((get(many, "k"), get(many, "j"), get(many, "m")), (&Value::Int(1), &Value::Int(0), &Value::Int(6)));
        assert!(read("a:\n  <<: 5\n").is_err());
        assert!(read("a: *missing\n").is_err());
    }

    #[test]
    fn too_many_aliases_to_collections_are_refused() {
        let mut text = String::from("a: &a [x, x, x]\n");
        for i in 0..60 {
            text.push_str(&format!("k{i}: *a\n"));
        }
        assert!(read(&text).unwrap_err().message.contains("exceeds the specified max=50"));
    }

    #[test]
    fn syntax_errors_say_where() {
        let error = read("a: [1, 2").unwrap_err();
        assert_eq!(error.kind, Kind::Parse);
        assert!(error.message.contains("line"), "{}", error.message);
        assert!(read("a:\n\tb: 1\n").is_err());
    }
}
