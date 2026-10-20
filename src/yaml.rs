//! A subset of YAML, read into a tree whose every value keeps its line (0063).
//!
//! Job files are the reason it exists. dbt-jobs-as-code files lean on anchors
//! and `<<` merges, and a value a job inherits through one is as much the
//! job's as a value written in place, which a line scanner such as
//! `src/project.rs` cannot follow (0018). Everything else here is the everyday
//! block YAML those files and Airflow's are written in. The config blocks of
//! `dbt_project.yml` are read with it too, since dbt follows their anchors
//! and merges as well (0103).
//!
//! Invariant: a document is read whole or refused whole, at a line, with a
//! fixed message that carries no text from the file. A construct this reader
//! does not know is refused rather than guessed, since a job read wrong points
//! at the wrong models without a word.

use crate::project::Unparsed;
use std::collections::HashMap;

/// How many values one document may hold once its aliases are expanded. Far
/// past any job file, and short of what a few nested aliases can reach.
const MAX_VALUES: usize = 200_000;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    /// Every scalar but a null. Booleans and numbers stay text, and the reader
    /// that wants one says which spellings it takes.
    Str(String),
    Seq(Vec<Node>),
    /// In document order. A key appears once: YAML forbids a repeat, and this
    /// refuses one rather than pick a side.
    Map(Vec<(String, Node)>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Where the value starts, from 1. A value reached through an alias keeps
    /// the line of its anchor.
    pub line: usize,
    /// The line of the key the value is written under, from 1, or 0 under
    /// none: the document, a list item. A mapping starts on the line after its
    /// key. A value an alias names, and everything inside it, list items
    /// too, take the line that names the alias: the keys are written at the
    /// anchor, under another path, and the alias is where a reader looks for
    /// this one.
    pub key_line: usize,
    pub value: Value,
}

impl Node {
    pub fn get(&self, key: &str) -> Option<&Node> {
        match &self.value {
            Value::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// A dotted path of keys, `pipeline.name`.
    pub fn at_path(&self, path: &str) -> Option<&Node> {
        path.split('.').try_fold(self, |node, key| node.get(key))
    }

    pub fn str(&self) -> Option<&str> {
        match &self.value {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn items(&self) -> Option<&[Node]> {
        match &self.value {
            Value::Seq(items) => Some(items),
            _ => None,
        }
    }

    pub fn entries(&self) -> Option<&[(String, Node)]> {
        match &self.value {
            Value::Map(entries) => Some(entries),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        self.value == Value::Null
    }

    /// Gives this value and everything inside it the line of the alias that
    /// names it.
    fn named_at(&mut self, line: usize) {
        self.key_line = line;
        match &mut self.value {
            Value::Seq(items) => items.iter_mut().for_each(|n| n.named_at(line)),
            Value::Map(entries) => entries.iter_mut().for_each(|(_, n)| n.named_at(line)),
            _ => {}
        }
    }

    fn size(&self) -> usize {
        1 + match &self.value {
            Value::Seq(items) => items.iter().map(Node::size).sum(),
            Value::Map(entries) => entries.iter().map(|(_, v)| v.size()).sum(),
            _ => 0,
        }
    }
}

const TAB: &str = "a tab in the indentation";
const INDENT: &str = "unexpected indentation";

type R<T> = Result<T, Unparsed>;

fn fail<T>(line: usize, message: &'static str) -> R<T> {
    Err(Unparsed { line, message })
}

struct Line {
    no: usize,
    indent: usize,
    /// After the indentation. Rewritten for the mapping that opens on a
    /// `- key: value` line, which then reads like any other.
    text: String,
    /// The line as written, for a block scalar's own indentation.
    raw: String,
}

impl Line {
    fn blank(&self) -> bool {
        let t = self.text.trim_start();
        t.is_empty() || t.starts_with('#')
    }
    fn tab(&self) -> bool {
        self.text.starts_with('\t')
    }
    fn marker(&self, m: &str) -> bool {
        self.indent == 0 && (self.text == m || self.text.starts_with(&format!("{m} ")))
    }
    fn end(&self) -> bool {
        self.marker("---") || self.marker("...")
    }
}

fn is_dash(text: &str) -> bool {
    text == "-" || text.starts_with("- ")
}

/// Parses one document. An empty one is a null at line 1.
pub fn parse(text: &str) -> Result<Node, Unparsed> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let lines = text
        .split('\n')
        .enumerate()
        .map(|(i, raw)| {
            let raw = raw.strip_suffix('\r').unwrap_or(raw);
            let indent = raw.len() - raw.trim_start_matches(' ').len();
            Line { no: i + 1, indent, text: raw[indent..].to_string(), raw: raw.to_string() }
        })
        .collect();
    Parser { lines, at: 0, anchors: HashMap::new(), values: 0 }.document()
}

struct Parser {
    lines: Vec<Line>,
    at: usize,
    anchors: HashMap<String, Node>,
    values: usize,
}

impl Parser {
    fn skip_blank(&mut self) {
        while self.lines.get(self.at).is_some_and(Line::blank) {
            self.at += 1;
        }
    }

    fn document(&mut self) -> R<Node> {
        self.skip_blank();
        if let Some(l) = self.lines.get(self.at) {
            if l.text.starts_with('%') {
                return fail(l.no, "a directive such as %YAML");
            }
            if l.marker("---") {
                let rest = l.text[3..].trim();
                if !rest.is_empty() && !rest.starts_with('#') {
                    return fail(l.no, "a value on the --- line");
                }
                self.at += 1;
                self.skip_blank();
            }
        }
        let root = match self.lines.get(self.at) {
            Some(l) if !l.end() => {
                let indent = l.indent;
                self.block(indent, -1)?
            }
            _ => Node { line: 1, key_line: 0, value: Value::Null },
        };
        self.skip_blank();
        if let Some(l) = self.lines.get(self.at) {
            let no = l.no;
            if l.marker("...") {
                self.at += 1;
                self.skip_blank();
                if let Some(l) = self.lines.get(self.at) {
                    return fail(l.no, "more than one document");
                }
            } else if l.marker("---") {
                return fail(no, "more than one document");
            } else {
                return fail(no, INDENT);
            }
        }
        Ok(root)
    }

    /// The value that starts on the current line, at `indent`. `parent` is
    /// the indentation a continuation line has to pass, -1 at the top.
    fn block(&mut self, indent: usize, parent: isize) -> R<Node> {
        let l = &self.lines[self.at];
        if l.tab() {
            return fail(l.no, TAB);
        }
        if is_dash(&l.text) {
            return self.seq(indent);
        }
        let (no, text) = (l.no, l.text.clone());
        if split_key(&text, no)?.is_some() {
            return self.map(indent);
        }
        self.at += 1;
        self.inline(&text, no, parent, false)
    }

    fn map(&mut self, indent: usize) -> R<Node> {
        let first = self.lines[self.at].no;
        let mut entries: Vec<(String, Node)> = Vec::new();
        let mut merges: Vec<(usize, Node)> = Vec::new();
        loop {
            self.skip_blank();
            let Some(l) = self.lines.get(self.at) else { break };
            if l.indent < indent || l.end() {
                break;
            }
            if l.tab() {
                return fail(l.no, TAB);
            }
            if l.indent > indent {
                return fail(l.no, INDENT);
            }
            let (no, text) = (l.no, l.text.clone());
            if text.starts_with("{%") {
                return fail(no, "a Jinja {% %} block, which only a vars file can render");
            }
            if is_dash(&text) {
                return fail(no, "a list item among the keys of a mapping");
            }
            let Some((key, rest)) = split_key(&text, no)? else {
                return fail(no, "a line that is neither a key nor a list item");
            };
            self.at += 1;
            let mut value = self.inline(rest, no, indent as isize, true)?;
            value.key_line = no;
            if key == "<<" {
                merges.push((no, value));
                continue;
            }
            if entries.iter().any(|(k, _)| *k == key) {
                return fail(no, "a key that appears twice in one mapping");
            }
            entries.push((key, value));
        }
        // An explicit key wins over a merged one, and an earlier merge over a
        // later one, which is what YAML's merge key says.
        for (no, merged) in merges {
            let maps: Vec<Node> = match merged.value {
                Value::Map(_) => vec![merged],
                Value::Seq(items) => items,
                _ => return fail(no, "a << merge of something that is not a mapping"),
            };
            for m in maps {
                let Value::Map(more) = m.value else {
                    return fail(no, "a << merge of something that is not a mapping");
                };
                for (k, v) in more {
                    if !entries.iter().any(|(e, _)| *e == k) {
                        entries.push((k, v));
                    }
                }
            }
        }
        Ok(Node { line: first, key_line: 0, value: Value::Map(entries) })
    }

    fn seq(&mut self, indent: usize) -> R<Node> {
        let first = self.lines[self.at].no;
        let mut items: Vec<Node> = Vec::new();
        loop {
            self.skip_blank();
            let Some(l) = self.lines.get(self.at) else { break };
            if l.indent < indent || l.end() {
                break;
            }
            if l.tab() {
                return fail(l.no, TAB);
            }
            if l.indent > indent {
                return fail(l.no, INDENT);
            }
            if l.text.starts_with("{%") {
                return fail(l.no, "a Jinja {% %} block, which only a vars file can render");
            }
            // A key beside a list at its own indentation closes the list:
            // `jobs:` then `- a` then `other: b`, all at one level.
            if !is_dash(&l.text) {
                break;
            }
            let no = l.no;
            let text = l.text.clone();
            let rest = text[1..].trim_start_matches(' ');
            if rest.is_empty() || rest.starts_with('#') {
                self.at += 1;
                items.push(self.inline("", no, indent as isize, false)?);
                continue;
            }
            let (anchor, body) = take_anchor(rest, no)?;
            let opens = !body.is_empty() && !body.starts_with('#') && (is_dash(body) || split_key(body, no)?.is_some());
            if opens {
                // `- key: value` opens a mapping whose keys line up under
                // `key`; `- - a` a list inside the list. The line is read
                // again from there, as if it began at that column.
                let column = indent + text.len() - body.len();
                let line = &mut self.lines[self.at];
                line.indent = column;
                line.text = body.to_string();
                let node = self.block(column, indent as isize)?;
                if let Some(a) = anchor {
                    self.anchors.insert(a, node.clone());
                }
                items.push(node);
            } else {
                self.at += 1;
                items.push(self.inline(rest, no, indent as isize, false)?);
            }
        }
        Ok(Node { line: first, key_line: 0, value: Value::Seq(items) })
    }

    /// A value written after `key:`, after `- `, or alone on its line, which
    /// is line `no`; the reader already stands on the line after it.
    fn inline(&mut self, text: &str, no: usize, parent: isize, in_map: bool) -> R<Node> {
        let (anchor, s) = take_anchor(text.trim_start(), no)?;
        let node = if s.is_empty() || s.starts_with('#') {
            self.nested(no, parent, in_map)?
        } else if let Some(r) = s.strip_prefix('*') {
            if anchor.is_some() {
                return fail(no, "an anchor on an alias");
            }
            let end = r.find([' ', '\t']).unwrap_or(r.len());
            let tail = r[end..].trim_start();
            if !tail.is_empty() && !tail.starts_with('#') {
                return fail(no, "text after an alias");
            }
            let Some(mut found) = self.anchors.get(&r[..end]).cloned() else {
                return fail(no, "an alias to an anchor not defined above it");
            };
            self.values += found.size();
            if self.values > MAX_VALUES {
                return fail(no, "aliases that expand past the size this reader takes");
            }
            found.named_at(no);
            return Ok(found);
        } else if s.starts_with('!') {
            return fail(no, "a tag such as !!str");
        } else if s.starts_with('|') || s.starts_with('>') {
            self.block_scalar(s, no, parent)?
        } else if s.starts_with("{{") {
            // A template, kept as text: jobs-as-code renders it from a vars
            // file before reading the YAML, and nothing here renders it.
            self.plain(s, no, parent)?
        } else if s.starts_with('[') || s.starts_with('{') {
            self.flow(s, no, parent)?
        } else if s.starts_with('"') || s.starts_with('\'') {
            match quoted(s) {
                Ok((value, rest)) => {
                    let rest = rest.trim_start();
                    if !rest.is_empty() && !rest.starts_with('#') {
                        return fail(no, "text after a closing quote");
                    }
                    Node { line: no, key_line: 0, value: Value::Str(value) }
                }
                Err(message) => return fail(no, message),
            }
        } else if s == "?" || s.starts_with("? ") {
            return fail(no, "a complex key written with ?");
        } else if s.starts_with('@') || s.starts_with('`') {
            return fail(no, "a value starting with @ or a backtick, which YAML reserves");
        } else {
            self.plain(s, no, parent)?
        };
        self.values += 1;
        if self.values > MAX_VALUES {
            return fail(no, "aliases that expand past the size this reader takes");
        }
        if let Some(a) = anchor {
            self.anchors.insert(a, node.clone());
        }
        Ok(node)
    }

    /// The value under a key or a dash that has nothing after it.
    fn nested(&mut self, no: usize, parent: isize, in_map: bool) -> R<Node> {
        self.skip_blank();
        match self.lines.get(self.at) {
            Some(l) if !l.end() && (l.indent as isize) > parent => {
                let indent = l.indent;
                self.block(indent, parent)
            }
            // `jobs:` with its `- a` at the same indentation, which YAML allows
            // under a key and nowhere else.
            Some(l) if in_map && !l.end() && (l.indent as isize) == parent && is_dash(&l.text) => {
                let indent = l.indent;
                self.seq(indent)
            }
            _ => Ok(Node { line: no, key_line: 0, value: Value::Null }),
        }
    }

    fn plain(&mut self, s: &str, no: usize, parent: isize) -> R<Node> {
        let first = strip_comment(s);
        check_plain(first, no)?;
        let mut out = first.to_string();
        let mut done = first.len() < s.trim_end().len();
        while !done {
            let mut j = self.at;
            let mut breaks = 0usize;
            while self.lines.get(j).is_some_and(|l| l.text.trim().is_empty()) {
                breaks += 1;
                j += 1;
            }
            let Some(l) = self.lines.get(j) else { break };
            if (l.indent as isize) <= parent || l.end() || l.text.trim_start().starts_with('#') {
                break;
            }
            if l.tab() {
                return fail(l.no, TAB);
            }
            let full = l.text.trim();
            let piece = strip_comment(full);
            check_plain(piece, l.no)?;
            if breaks == 0 {
                out.push(' ');
            } else {
                out.push_str(&"\n".repeat(breaks));
            }
            out.push_str(piece);
            self.at = j + 1;
            done = piece.len() < full.len();
        }
        let value = match out.as_str() {
            "" | "~" | "null" | "Null" | "NULL" => Value::Null,
            _ => Value::Str(out),
        };
        Ok(Node { line: no, key_line: 0, value })
    }

    fn block_scalar(&mut self, header: &str, no: usize, parent: isize) -> R<Node> {
        let folded = header.starts_with('>');
        let mut chomp = ' ';
        let mut explicit: Option<usize> = None;
        let indicators = strip_comment(&header[1..]);
        for c in indicators.chars() {
            match c {
                '-' | '+' if chomp == ' ' => chomp = c,
                '1'..='9' if explicit.is_none() => explicit = c.to_digit(10).map(|d| d as usize),
                _ => return fail(no, "a | or > followed by something other than - + or a digit"),
            }
        }
        let floor = (parent + 1).max(0) as usize;
        let mut base = explicit.map(|d| (parent.max(0) as usize) + d);
        let mut body: Vec<String> = Vec::new();
        while let Some(l) = self.lines.get(self.at) {
            let spaces = l.raw.len() - l.raw.trim_start_matches(' ').len();
            if l.raw.trim().is_empty() {
                body.push(String::new());
                self.at += 1;
                continue;
            }
            if spaces < floor || (l.end() && base.is_none()) {
                break;
            }
            let b = *base.get_or_insert(spaces);
            if spaces < b {
                break;
            }
            body.push(l.raw[b..].to_string());
            self.at += 1;
        }
        // Blank lines at the end belong to the chomping, never to the text.
        let trailing = body.iter().rev().take_while(|l| l.is_empty()).count();
        body.truncate(body.len() - trailing);
        let mut out = String::new();
        if folded {
            let mut prev_more = false;
            let mut breaks = 0usize;
            for (i, line) in body.iter().enumerate() {
                if line.is_empty() {
                    breaks += 1;
                    continue;
                }
                let more = line.starts_with(' ') || line.starts_with('\t');
                if i > breaks {
                    if more || prev_more {
                        out.push_str(&"\n".repeat(breaks + 1));
                    } else if breaks == 0 {
                        out.push(' ');
                    } else {
                        out.push_str(&"\n".repeat(breaks));
                    }
                } else {
                    out.push_str(&"\n".repeat(breaks));
                }
                out.push_str(line);
                prev_more = more;
                breaks = 0;
            }
        } else {
            out = body.join("\n");
        }
        if !body.is_empty() {
            match chomp {
                '-' => {}
                '+' => out.push_str(&"\n".repeat(trailing + 1)),
                _ => out.push('\n'),
            }
        }
        Ok(Node { line: no, key_line: 0, value: Value::Str(out) })
    }

    fn flow(&mut self, s: &str, no: usize, parent: isize) -> R<Node> {
        let mut text = s.to_string();
        // Where each line the collection runs on to starts in `text`, so a
        // key keeps its own line.
        let mut breaks: Vec<(usize, usize)> = Vec::new();
        let end = loop {
            match flow_end(&text) {
                Ok(Some(end)) => break end,
                Ok(None) => {}
                Err(message) => return fail(no, message),
            }
            match self.lines.get(self.at) {
                // The closing bracket may sit back at the key's own column.
                Some(l) if (l.indent as isize) >= parent && !l.end() => {
                    if l.tab() {
                        return fail(l.no, TAB);
                    }
                    text.push(' ');
                    breaks.push((text.len(), l.no));
                    text.push_str(strip_quoted_comment(l.text.trim()));
                    self.at += 1;
                }
                _ => return fail(no, "a [ ] or { } that never closes"),
            }
        };
        let rest = text[end..].trim_start();
        if !rest.is_empty() && !rest.starts_with('#') {
            return fail(no, "text after a closing ] or }");
        }
        let mut f = Flow { s: &text[..end], i: 0, line: no, breaks: &breaks, anchors: &self.anchors, values: &mut self.values };
        let node = f.value().or_else(|m| fail(no, m))?;
        self.values += 1;
        Ok(node)
    }
}

/// `&name` in front of a value, and what follows it.
fn take_anchor(s: &str, no: usize) -> R<(Option<String>, &str)> {
    let Some(r) = s.strip_prefix('&') else { return Ok((None, s)) };
    let end = r.find([' ', '\t']).unwrap_or(r.len());
    if end == 0 {
        return fail(no, "an anchor with no name");
    }
    Ok((Some(r[..end].to_string()), r[end..].trim_start()))
}

/// A plain scalar without its comment. Quotes are text inside one, so a
/// ` #` starts a comment wherever it falls.
fn strip_comment(s: &str) -> &str {
    let mut prev = ' ';
    for (i, c) in s.char_indices() {
        if c == '#' && (prev == ' ' || prev == '\t') {
            return s[..i].trim_end();
        }
        prev = c;
    }
    s.trim_end()
}

/// A line inside `[ ]` or `{ }`, without a comment outside its quotes.
fn strip_quoted_comment(s: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut prev = ' ';
    for (i, c) in s.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '#' && (prev == ' ' || prev == '\t') => return s[..i].trim_end(),
            None => {}
        }
        prev = c;
    }
    s
}

/// YAML refuses `: ` inside an unquoted value, as a key where none can be.
fn check_plain(s: &str, no: usize) -> R<()> {
    if s.contains(": ") || s.contains(":\t") || s.ends_with(':') {
        return fail(no, "a colon and a space inside an unquoted value");
    }
    Ok(())
}

/// The key of a `key: value` line, and what follows the colon. None when the
/// line is a value rather than a key.
fn split_key(text: &str, no: usize) -> R<Option<(String, &str)>> {
    if text.starts_with('"') || text.starts_with('\'') {
        let Ok((key, rest)) = quoted(text) else { return Ok(None) };
        let after = rest.trim_start_matches([' ', '\t']);
        return Ok(match after.strip_prefix(':') {
            Some(v) if v.is_empty() || v.starts_with([' ', '\t']) => Some((key, v)),
            _ => None,
        });
    }
    if text.starts_with(['[', '{', '#', '&', '*', '!', '|', '>']) || is_dash(text) {
        return Ok(None);
    }
    if text == "?" || text.starts_with("? ") {
        return fail(no, "a complex key written with ?");
    }
    let mut prev = ' ';
    let bytes = text.as_bytes();
    for (i, c) in text.char_indices() {
        if c == '#' && (prev == ' ' || prev == '\t') {
            return Ok(None);
        }
        if c == ':' {
            let next = bytes.get(i + 1).copied();
            if next.is_none() || next == Some(b' ') || next == Some(b'\t') {
                let key = text[..i].trim_end();
                if key.is_empty() {
                    return fail(no, "a colon with no key before it");
                }
                return Ok(Some((key.to_string(), &text[i + 1..])));
            }
        }
        prev = c;
    }
    Ok(None)
}

/// A quoted scalar at the start of `s`, and what follows its closing quote.
fn quoted(s: &str) -> Result<(String, &str), &'static str> {
    let mut chars = s.char_indices();
    let Some((_, q)) = chars.next() else { return Err("an empty quoted value") };
    let mut out = String::new();
    while let Some((i, c)) = chars.next() {
        if q == '\'' {
            if c == '\'' {
                if s[i + 1..].starts_with('\'') {
                    out.push('\'');
                    chars.next();
                    continue;
                }
                return Ok((out, &s[i + 1..]));
            }
            out.push(c);
            continue;
        }
        match c {
            '"' => return Ok((out, &s[i + 1..])),
            '\\' => {
                let Some((_, e)) = chars.next() else { return Err("a quoted value that does not close on its line") };
                let simple = match e {
                    '0' => Some('\0'),
                    'a' => Some('\x07'),
                    'b' => Some('\x08'),
                    't' | '\t' => Some('\t'),
                    'n' => Some('\n'),
                    'v' => Some('\x0b'),
                    'f' => Some('\x0c'),
                    'r' => Some('\r'),
                    'e' => Some('\x1b'),
                    ' ' => Some(' '),
                    '"' => Some('"'),
                    '/' => Some('/'),
                    '\\' => Some('\\'),
                    'N' => Some('\u{85}'),
                    '_' => Some('\u{a0}'),
                    'L' => Some('\u{2028}'),
                    'P' => Some('\u{2029}'),
                    _ => None,
                };
                if let Some(ch) = simple {
                    out.push(ch);
                    continue;
                }
                let width = match e {
                    'x' => 2,
                    'u' => 4,
                    'U' => 8,
                    _ => return Err("an escape YAML does not define, inside double quotes"),
                };
                let mut code = 0u32;
                for _ in 0..width {
                    let digit = chars.next().and_then(|(_, h)| h.to_digit(16)).ok_or("an escape YAML does not define, inside double quotes")?;
                    code = code * 16 + digit;
                }
                out.push(char::from_u32(code).ok_or("an escape YAML does not define, inside double quotes")?);
            }
            _ => out.push(c),
        }
    }
    Err("a quoted value that does not close on its line")
}

/// Where the `[ ]` or `{ }` that opens `s` closes, or None while it is still
/// open at the end of the text.
fn flow_end(s: &str) -> Result<Option<usize>, &'static str> {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match quote {
            Some('"') if c == '\\' => {
                chars.next();
            }
            Some('\'') if c == '\'' && chars.peek().map(|&(_, n)| n) == Some('\'') => {
                chars.next();
            }
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                '"' | '\'' => quote = Some(c),
                '[' | '{' => depth += 1,
                ']' | '}' => {
                    depth = depth.checked_sub(1).ok_or("a ] or } with nothing open")?;
                    if depth == 0 {
                        return Ok(Some(i + 1));
                    }
                }
                _ => {}
            },
        }
    }
    Ok(None)
}

struct Flow<'s> {
    s: &'s str,
    i: usize,
    /// The line the collection opens on, and where each later one starts.
    line: usize,
    breaks: &'s [(usize, usize)],
    /// `<<: [*a, *b]` is how a merge of several is usually written.
    anchors: &'s HashMap<String, Node>,
    values: &'s mut usize,
}

impl Flow<'_> {
    fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }
    fn ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.i += 1;
        }
    }

    fn line_here(&self) -> usize {
        self.breaks.iter().rev().find(|(at, _)| *at <= self.i).map_or(self.line, |(_, no)| *no)
    }

    fn value(&mut self) -> Result<Node, &'static str> {
        self.ws();
        let line = self.line_here();
        match self.peek() {
            Some('[') => {
                self.i += 1;
                let mut items = Vec::new();
                loop {
                    self.ws();
                    if self.peek() == Some(']') {
                        self.i += 1;
                        break;
                    }
                    items.push(self.value()?);
                    self.ws();
                    match self.peek() {
                        Some(',') => self.i += 1,
                        Some(']') => {
                            self.i += 1;
                            break;
                        }
                        Some(':') => return Err("a key: value pair inside [ ]"),
                        _ => return Err("a missing comma inside [ ]"),
                    }
                }
                Ok(Node { line, key_line: 0, value: Value::Seq(items) })
            }
            Some('{') => {
                self.i += 1;
                let mut entries: Vec<(String, Node)> = Vec::new();
                loop {
                    self.ws();
                    if self.peek() == Some('}') {
                        self.i += 1;
                        break;
                    }
                    let key_line = self.line_here();
                    let key = match self.scalar()? {
                        Value::Str(k) => k,
                        _ => return Err("a key with no name inside { }"),
                    };
                    self.ws();
                    if self.peek() != Some(':') {
                        return Err("a key without its colon inside { }");
                    }
                    self.i += 1;
                    self.ws();
                    let mut value = match self.peek() {
                        Some(',') | Some('}') => Node { line, key_line: 0, value: Value::Null },
                        _ => self.value()?,
                    };
                    value.key_line = key_line;
                    if entries.iter().any(|(k, _)| *k == key) {
                        return Err("a key that appears twice in one mapping");
                    }
                    entries.push((key, value));
                    self.ws();
                    match self.peek() {
                        Some(',') => self.i += 1,
                        Some('}') => {
                            self.i += 1;
                            break;
                        }
                        _ => return Err("a missing comma inside { }"),
                    }
                }
                Ok(Node { line, key_line: 0, value: Value::Map(entries) })
            }
            Some('*') => {
                let rest = &self.s[self.i + 1..];
                let end = rest.find(|c: char| c.is_whitespace() || matches!(c, ',' | ']' | '}')).unwrap_or(rest.len());
                let mut found = self.anchors.get(&rest[..end]).cloned().ok_or("an alias to an anchor not defined above it")?;
                self.i += 1 + end;
                *self.values += found.size();
                if *self.values > MAX_VALUES {
                    return Err("aliases that expand past the size this reader takes");
                }
                found.named_at(line);
                Ok(found)
            }
            _ => {
                *self.values += 1;
                Ok(Node { line, key_line: 0, value: self.scalar()? })
            }
        }
    }

    fn scalar(&mut self) -> Result<Value, &'static str> {
        match self.peek() {
            Some('"') | Some('\'') => {
                let (v, rest) = quoted(&self.s[self.i..])?;
                self.i = self.s.len() - rest.len();
                Ok(Value::Str(v))
            }
            Some('&') | Some('!') => Err("an anchor or a tag inside [ ] or { }"),
            Some('*') => Err("an alias where a key goes"),
            Some('[') | Some('{') => Err("a [ ] or { } where a key goes"),
            _ => {
                let rest = &self.s[self.i..];
                let mut end = rest.len();
                let mut chars = rest.char_indices().peekable();
                while let Some((j, c)) = chars.next() {
                    let next = chars.peek().map(|&(_, n)| n);
                    let colon = c == ':' && matches!(next, None | Some(' ') | Some('\t') | Some(',') | Some(']') | Some('}'));
                    if matches!(c, ',' | '[' | ']' | '{' | '}') || colon {
                        end = j;
                        break;
                    }
                }
                let text = rest[..end].trim();
                self.i += end;
                match text {
                    "" => Err("an empty value inside [ ] or { }"),
                    "~" | "null" | "Null" | "NULL" => Ok(Value::Null),
                    _ => Ok(Value::Str(text.to_string())),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Value {
        Value::Str(v.to_string())
    }
    fn ok(text: &str) -> Node {
        parse(text).unwrap_or_else(|e| panic!("line {}: {}", e.line, e.message))
    }
    fn refused(text: &str) -> Unparsed {
        parse(text).expect_err("should be refused")
    }

    #[test]
    fn reads_nested_mappings_and_lists() {
        let doc = ok("pipeline:\n  name: sales_daily\n  cron: \"0 6 * * *\"\n  tags: [a, b]\nsteps:\n  - task_id: one\n    command: run --select tag:x\n  - task_id: two\n    depends_on:\n      - one\n");
        assert_eq!(doc.at_path("pipeline.name").unwrap().value, s("sales_daily"));
        assert_eq!(doc.at_path("pipeline.cron").unwrap().value, s("0 6 * * *"));
        assert_eq!(doc.at_path("pipeline.tags").unwrap().items().unwrap().len(), 2);
        let jobs = doc.get("steps").unwrap().items().unwrap();
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].get("command").unwrap().value, s("run --select tag:x"));
        assert_eq!(jobs[0].get("command").unwrap().line, 7);
        assert_eq!(jobs[1].get("depends_on").unwrap().items().unwrap()[0].value, s("one"));
        assert_eq!(jobs[1].line, 8);
    }

    #[test]
    fn a_list_may_sit_at_its_keys_own_indentation() {
        let doc = ok("jobs:\n- a\n- b\nnext: c\n");
        assert_eq!(doc.get("jobs").unwrap().items().unwrap().len(), 2);
        assert_eq!(doc.get("next").unwrap().value, s("c"));
    }

    #[test]
    fn reads_quotes_comments_and_nulls() {
        let doc = ok("a: \"x # not a comment\"  # a comment\nb: 'it''s'\nc: plain # gone\nd:\ne: ~\nf: \"\\u00e9\\t\"\ng: 'null'\n");
        assert_eq!(doc.get("a").unwrap().value, s("x # not a comment"));
        assert_eq!(doc.get("b").unwrap().value, s("it's"));
        assert_eq!(doc.get("c").unwrap().value, s("plain"));
        assert!(doc.get("d").unwrap().is_null());
        assert!(doc.get("e").unwrap().is_null());
        assert_eq!(doc.get("f").unwrap().value, s("é\t"));
        assert_eq!(doc.get("g").unwrap().value, s("null"));
    }

    #[test]
    fn folds_and_keeps_block_scalars() {
        let doc = ok("a: >-\n  run -s\n  one+\n  --exclude two\nb: |\n  line one\n  line two\nc: |-\n  kept\n\nd: >\n  para one\n\n  para two\ne: |+\n  x\n\nf: end\n");
        assert_eq!(doc.get("a").unwrap().value, s("run -s one+ --exclude two"));
        assert_eq!(doc.get("b").unwrap().value, s("line one\nline two\n"));
        assert_eq!(doc.get("c").unwrap().value, s("kept"));
        assert_eq!(doc.get("d").unwrap().value, s("para one\npara two\n"));
        assert_eq!(doc.get("e").unwrap().value, s("x\n\n"));
        assert_eq!(doc.get("f").unwrap().value, s("end"));
    }

    #[test]
    fn a_folded_command_inside_a_list_item_keeps_its_line() {
        let doc = ok("tasks:\n  - task_id: big\n    run: >-\n      run -s\n      a+ b+\n    depends_on: [x]\n");
        let job = &doc.get("tasks").unwrap().items().unwrap()[0];
        assert_eq!(job.get("run").unwrap().value, s("run -s a+ b+"));
        assert_eq!(job.get("run").unwrap().line, 3);
        assert_eq!(job.get("depends_on").unwrap().items().unwrap().len(), 1);
    }

    #[test]
    fn continues_a_plain_value_on_the_next_lines() {
        let doc = ok("a: run --select one\n  two\n  three # done\nb: c\n");
        assert_eq!(doc.get("a").unwrap().value, s("run --select one two three"));
        assert_eq!(doc.get("b").unwrap().value, s("c"));
    }

    #[test]
    fn reads_flow_collections_over_several_lines() {
        let doc = ok("a: [x, \"y, z\", 'w']\nb: {cron: \"0 * * * *\", on: true}\nc: [\n  one,  # first\n  two\n]\nd: []\ne: {}\n");
        let a = doc.get("a").unwrap().items().unwrap();
        assert_eq!(a.iter().map(|n| n.value.clone()).collect::<Vec<_>>(), vec![s("x"), s("y, z"), s("w")]);
        assert_eq!(doc.at_path("b.cron").unwrap().value, s("0 * * * *"));
        assert_eq!(doc.at_path("b.on").unwrap().value, s("true"));
        assert_eq!(doc.get("c").unwrap().items().unwrap().len(), 2);
        assert_eq!(doc.get("d").unwrap().items().unwrap().len(), 0);
        assert_eq!(doc.get("e").unwrap().entries().unwrap().len(), 0);
        assert_eq!(doc.get("b").unwrap().line, 2);
    }

    #[test]
    fn follows_anchors_aliases_and_merges() {
        let doc = ok("defaults: &defaults\n  account_id: 1\n  settings:\n    target_name: prod\n  generate_docs: false\njobs:\n  one:\n    <<: *defaults\n    generate_docs: true\n    steps: &steps\n      - dbt build\n  two:\n    <<: [*defaults]\n    steps: *steps\n");
        let one = doc.at_path("jobs.one").unwrap();
        assert_eq!(one.get("account_id").unwrap().value, s("1"));
        assert_eq!(one.get("generate_docs").unwrap().value, s("true"), "an explicit key wins over a merged one");
        assert_eq!(one.at_path("settings.target_name").unwrap().value, s("prod"));
        let two = doc.at_path("jobs.two").unwrap();
        assert_eq!(two.get("generate_docs").unwrap().value, s("false"));
        assert_eq!(two.get("steps").unwrap().items().unwrap()[0].value, s("dbt build"));
        assert_eq!(two.get("steps").unwrap().line, 11, "an alias keeps its anchor's line");
    }

    #[test]
    fn keeps_a_template_as_text() {
        let doc = ok("environment_id: {{ env_id }}\nsteps:\n  - dbt run --target {{ target }}\n");
        assert_eq!(doc.get("environment_id").unwrap().value, s("{{ env_id }}"));
        assert_eq!(doc.get("steps").unwrap().items().unwrap()[0].value, s("dbt run --target {{ target }}"));
    }

    #[test]
    fn reads_crlf_a_bom_and_document_markers() {
        let doc = ok("\u{feff}# header\r\n---\r\na: 1\r\nb:\r\n  - x\r\n...\r\n");
        assert_eq!(doc.get("a").unwrap().value, s("1"));
        assert_eq!(doc.get("b").unwrap().items().unwrap()[0].value, s("x"));
        assert!(ok("").is_null());
        assert!(ok("# only a comment\n").is_null());
    }

    #[test]
    fn reads_lists_inside_lists_and_anchored_items() {
        let doc = ok("a:\n  - - x\n    - y\n  - &item\n    k: v\n    other: w\nb: *item\n");
        let a = doc.get("a").unwrap().items().unwrap();
        assert_eq!(a[0].items().unwrap().len(), 2);
        assert_eq!(a[1].get("other").unwrap().value, s("w"));
        assert_eq!(doc.at_path("b.k").unwrap().value, s("v"));
    }

    #[test]
    fn refuses_what_it_does_not_read_at_the_line() {
        let cases: &[(&str, usize, &str)] = &[
            ("a:\n\tb: 1\n", 2, TAB),
            ("a: 1\na: 2\n", 2, "a key that appears twice in one mapping"),
            ("a: \"open\n", 1, "a quoted value that does not close on its line"),
            ("a: *nowhere\n", 1, "an alias to an anchor not defined above it"),
            ("a: !!str 1\n", 1, "a tag such as !!str"),
            ("? a\n: b\n", 1, "a complex key written with ?"),
            ("a: 1\n---\nb: 2\n", 2, "more than one document"),
            ("jobs:\n{% if prod %}\n  a: 1\n", 2, "a Jinja {% %} block, which only a vars file can render"),
            ("a: run --vars {x: 1}\n", 1, "a colon and a space inside an unquoted value"),
            ("a: 1\n    b: 2\n", 2, "a colon and a space inside an unquoted value"),
            ("a:\n  b: 1\n c: 2\n", 3, INDENT),
            ("a: [x, y\nb: 1\n", 1, "a [ ] or { } that never closes"),
            ("a:\n  - x\n  b: 1\n", 3, INDENT),
            ("a: 1\n- b\n", 2, "a list item among the keys of a mapping"),
            ("<<: 1\n", 1, "a << merge of something that is not a mapping"),
            ("a: \"x\" y\n", 1, "text after a closing quote"),
        ];
        for (text, line, message) in cases {
            let e = refused(text);
            assert_eq!((e.line, e.message), (*line, *message), "for {text:?}");
        }
    }

    #[test]
    fn refuses_aliases_that_expand_past_the_limit() {
        // Ten of each level: 10^8 values written in a dozen lines, either way.
        let mut flow = String::from("a0: &a0 [x, x, x, x, x, x, x, x, x, x]\n");
        let mut block = String::from("a0: &a0\n");
        block.push_str(&"  - x\n".repeat(10));
        for i in 1..9 {
            let p = i - 1;
            flow.push_str(&format!("a{i}: &a{i} [{}]\n", vec![format!("*a{p}"); 10].join(", ")));
            block.push_str(&format!("a{i}: &a{i}\n"));
            block.push_str(&format!("  - *a{p}\n").repeat(10));
        }
        for text in [flow, block] {
            assert_eq!(refused(&text).message, "aliases that expand past the size this reader takes");
        }
    }

    /// A key is found at the line it is written on, or, reached through an
    /// alias, at the line that names the alias, however deep it sits (0103).
    #[test]
    fn each_value_knows_the_line_of_its_key() {
        let doc = ok("models:\n  shop:\n\n    with_anchor: &anc\n      inner:\n        +tags: [x]\n    aliased: *anc\n    merged:\n      <<: *anc\n      own: 1\n    flow: {a: 1,\n      b: {c: 2}}\n");
        let at = |path: &str| doc.at_path(path).unwrap().key_line;
        assert_eq!((at("models"), at("models.shop"), at("models.shop.with_anchor"), at("models.shop.with_anchor.inner")), (1, 2, 4, 5));
        assert_eq!((at("models.shop.aliased"), at("models.shop.aliased.inner"), at("models.shop.aliased.inner.+tags")), (7, 7, 7));
        assert_eq!((at("models.shop.merged.inner"), at("models.shop.merged.own")), (9, 10));
        assert_eq!((at("models.shop.flow.a"), at("models.shop.flow.b"), at("models.shop.flow.b.c")), (11, 12, 12));
        assert_eq!(doc.key_line, 0, "the document is under no key");
    }

    #[test]
    fn a_refusal_never_carries_text_from_the_file() {
        let e = refused("secret: \"hunter2\nnext: 1\n");
        assert!(!e.message.contains("hunter2"));
    }
}
