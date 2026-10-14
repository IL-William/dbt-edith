//! `dbt_project.yml`: the `vars:` block, the project's `name:`, and the
//! `dispatch:` order an incremental model's default strategy depends on (0076).
//!
//! The manifest holds the merged YAML for everything else, but it does not hold
//! project vars at all, so this is the one place they can be read (0018). The
//! name it does hold, from dbt-core 1.6 on; before that only a hash of it, and
//! the macro links need the name itself (`src/macros.rs`).
//!
//! The scanner covers the shapes that actually appear in a `vars:` block and
//! reports every line it could not read, with a line number and a fixed
//! message. It never guesses: a value that is silently wrong is worse than a
//! value that is visibly absent. Like `envs::Warning`, an `Unparsed` carries no
//! text from the file, so nothing can escape through an error path.
//!
//! `files::is_dbt_project` is the other place that knows this file exists; it
//! only checks for it at startup.

use std::path::Path;

/// A project file larger than this is not one anyone maintains by hand.
pub(crate) const MAX_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ProjectVar {
    pub name: String,
    pub line: usize,
    /// The value as written, quotes removed and any trailing comment stripped.
    /// Jinja is kept as text: nothing here evaluates it.
    pub raw: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list: Option<Vec<String>>,
    /// `raw` holds Jinja, so it needs the env scanner to mean anything (0009).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub jinja: bool,
    /// Written with no value at all. That is a real dbt var whose value is null,
    /// not a line that could not be read, and projects use it on purpose to mean
    /// "unset".
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub null: bool,
}

/// A package-scoped block, `vars:` -> `<package>:` -> its own vars. Reported
/// rather than flattened: a package scope shadows the global one for that
/// package, and pretending otherwise would show the wrong value.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PackageVars {
    pub package: String,
    pub line: usize,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Unparsed {
    pub line: usize,
    pub message: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct ProjectVars {
    /// Whether a `vars:` block exists at all, which is not the same as an empty one.
    pub found: bool,
    pub vars: Vec<ProjectVar>,
    pub packages: Vec<PackageVars>,
    pub unparsed: Vec<Unparsed>,
}

/// Leading spaces, or None when the indentation contains a tab. YAML forbids a
/// tab there, and misreading the nesting is worse than refusing the line.
fn indent_of(line: &str) -> Option<usize> {
    let n = line.len() - line.trim_start_matches(' ').len();
    if line[n..].starts_with('\t') {
        return None;
    }
    Some(n)
}

fn skippable(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with('#')
}

/// Splits `key: value` at the first `:` outside quotes. YAML only starts a
/// mapping when the colon is followed by a space or ends the line, so `a:b` is
/// a scalar and is refused rather than silently split.
fn split_key(s: &str) -> Option<(String, &str)> {
    let b = s.as_bytes();
    let mut quote: Option<u8> = None;
    for i in 0..b.len() {
        let c = b[i];
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                }
            }
            None => {
                if c == b'\'' || c == b'"' {
                    quote = Some(c);
                } else if c == b':' && (i + 1 == b.len() || b[i + 1] == b' ' || b[i + 1] == b'\t') {
                    let key = s[..i].trim();
                    let key = key.strip_prefix('"').and_then(|k| k.strip_suffix('"'))
                        .or_else(|| key.strip_prefix('\'').and_then(|k| k.strip_suffix('\'')))
                        .unwrap_or(key);
                    return Some((key.to_string(), &s[i + 1..]));
                }
            }
        }
    }
    None
}

enum Value {
    Scalar(String),
    List(Vec<String>),
    Bad(&'static str),
}

/// A quoted scalar, up to its closing quote. `''` inside single quotes and the
/// usual backslash escapes inside double quotes are honoured; anything after
/// the closing quote other than a comment is refused.
fn quoted(v: &str) -> Value {
    let q = v.as_bytes()[0] as char;
    let mut out = String::new();
    let mut it = v[1..].char_indices();
    while let Some((i, c)) = it.next() {
        if q == '"' && c == '\\' {
            match it.next() {
                Some((_, n)) => {
                    out.push(match n {
                        'n' => '\n',
                        't' => '\t',
                        other => other,
                    });
                    continue;
                }
                None => return Value::Bad("unterminated quote"),
            }
        }
        if c == q {
            let after_at = 1 + i + 1;
            if q == '\'' && v[after_at..].starts_with('\'') {
                out.push('\'');
                it.next();
                continue;
            }
            let after = v[after_at..].trim();
            if after.is_empty() || after.starts_with('#') {
                return Value::Scalar(out);
            }
            return Value::Bad("text after the closing quote is not read");
        }
        out.push(c);
    }
    Value::Bad("unterminated quote")
}

/// A flow sequence, `['a', "b", c]`. One level only: a nested collection is
/// refused rather than flattened.
fn flow_list(v: &str) -> Value {
    let Some(close) = v.rfind(']') else {
        return Value::Bad("a flow sequence with no closing bracket");
    };
    let after = v[close + 1..].trim();
    if !after.is_empty() && !after.starts_with('#') {
        return Value::Bad("text after the closing bracket is not read");
    }
    let inner = &v[1..close];
    let b = inner.as_bytes();
    let mut items = Vec::new();
    let mut quote: Option<u8> = None;
    let mut start = 0;
    for i in 0..b.len() {
        let c = b[i];
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                }
            }
            None => match c {
                b'\'' | b'"' => quote = Some(c),
                b'[' | b'{' => return Value::Bad("a nested flow collection is not read"),
                b',' => {
                    items.push(&inner[start..i]);
                    start = i + 1;
                }
                _ => {}
            },
        }
    }
    items.push(&inner[start..]);

    let mut out = Vec::new();
    for item in items {
        let t = item.trim();
        if t.is_empty() {
            continue;
        }
        match t.as_bytes()[0] {
            b'\'' | b'"' => match quoted(t) {
                Value::Scalar(s) => out.push(s),
                _ => return Value::Bad("a quoted item in the sequence is not read"),
            },
            _ => out.push(t.to_string()),
        }
    }
    Value::List(out)
}

/// The value half of a `key: value` line.
fn parse_value(v: &str) -> Value {
    let v = v.trim();
    match v.as_bytes()[0] {
        b'[' => flow_list(v),
        b'{' => Value::Bad("a flow mapping is not read"),
        b'|' | b'>' => Value::Bad("a block scalar is not read"),
        b'&' => Value::Bad("an anchor is not read"),
        b'*' => Value::Bad("an alias is not read"),
        b'\'' | b'"' => quoted(v),
        _ => {
            // A `#` only ends a plain scalar when a space or tab precedes it,
            // the same rule `envs::unquote` uses, so `EXT_`, `Europe/London`
            // and `current_timestamp()` survive while `1  # widen` does not.
            let end = [v.find(" #"), v.find("\t#")].into_iter().flatten().min().unwrap_or(v.len());
            Value::Scalar(v[..end].trim_end().to_string())
        }
    }
}

/// The project's `vars:` block. Only a `vars:` at indentation 0 counts: one
/// nested under `models:` is a per-model config, not a project var.
pub fn scan(text: &str) -> ProjectVars {
    let mut out = ProjectVars::default();
    let lines: Vec<&str> = text.lines().collect();

    let Some(head) = lines.iter().position(|l| l.trim() == "vars:" && indent_of(l) == Some(0)) else {
        return out;
    };
    out.found = true;

    // The block's indentation is whatever its first real line uses, never a
    // hardcoded two: packages in the wild indent by four.
    let first = lines[head + 1..].iter().position(|l| !skippable(l)).map(|i| head + 1 + i);
    let Some(first) = first else { return out };
    let Some(block) = indent_of(lines[first]) else {
        out.unparsed.push(Unparsed { line: first + 1, message: "tabs are not valid YAML indentation" });
        return out;
    };
    if block == 0 {
        return out;
    }

    let mut j = first;
    while j < lines.len() {
        let line = lines[j];
        // A blank line, or a comment at any indentation, continues the block.
        if skippable(line) {
            j += 1;
            continue;
        }
        let Some(ind) = indent_of(line) else {
            out.unparsed.push(Unparsed { line: j + 1, message: "tabs are not valid YAML indentation" });
            j += 1;
            continue;
        };
        if ind == 0 {
            break;
        }
        if ind != block {
            out.unparsed.push(Unparsed { line: j + 1, message: "indented differently from the rest of the block" });
            j += 1;
            continue;
        }
        let Some((name, rest)) = split_key(&line[ind..]) else {
            out.unparsed.push(Unparsed { line: j + 1, message: "not a key and a value" });
            j += 1;
            continue;
        };
        if name == "<<" {
            out.unparsed.push(Unparsed { line: j + 1, message: "a merge key is not read" });
            j += 1;
            continue;
        }

        if rest.trim().is_empty() {
            // A block sequence, a package scope, or a null. Collect the nested
            // lines first: which of the three it is depends on their shape.
            let mut k = j + 1;
            let mut nested: Vec<&str> = Vec::new();
            while k < lines.len() {
                if skippable(lines[k]) {
                    k += 1;
                    continue;
                }
                match indent_of(lines[k]) {
                    Some(i) if i > block => {
                        nested.push(lines[k].trim());
                        k += 1;
                    }
                    _ => break,
                }
            }
            let count = nested.len();

            // A single `-` makes it a sequence, not a package scope: only
            // `key: value` lines all the way down are a scope. A list is an
            // ordinary var value and the commonest way to write one, so it is
            // read; anything richer than plain items is reported rather than
            // guessed at.
            if nested.iter().any(|l| l.starts_with('-')) {
                let mut items = Vec::new();
                let mut bad = !nested.iter().all(|l| l.starts_with('-'));
                for line in nested.iter().take_while(|_| !bad) {
                    let item = line[1..].trim();
                    if item.is_empty() {
                        bad = true;
                        break;
                    }
                    match parse_value(item) {
                        Value::Scalar(v) => items.push(v),
                        _ => {
                            bad = true;
                            break;
                        }
                    }
                }
                if bad {
                    out.unparsed.push(Unparsed { line: j + 1, message: "a sequence of anything but plain items is not read" });
                } else {
                    push_var(&mut out, ProjectVar {
                        name,
                        line: j + 1,
                        raw: items.join(", "),
                        list: Some(items),
                        jinja: false,
                        null: false,
                    });
                }
                j = k;
                continue;
            }

            if count == 0 {
                push_var(&mut out, ProjectVar {
                    name,
                    line: j + 1,
                    raw: String::new(),
                    list: None,
                    jinja: false,
                    null: true,
                });
            } else {
                out.packages.push(PackageVars { package: name, line: j + 1, count });
            }
            j = k;
            continue;
        }

        match parse_value(rest) {
            Value::Bad(message) => out.unparsed.push(Unparsed { line: j + 1, message }),
            Value::List(items) => push_var(&mut out, ProjectVar {
                name,
                line: j + 1,
                raw: items.join(", "),
                list: Some(items),
                jinja: false,
                null: false,
            }),
            Value::Scalar(s) => {
                let jinja = s.contains("{{") || s.contains("{%");
                push_var(&mut out, ProjectVar { name, line: j + 1, raw: s, list: None, jinja, null: false });
            }
        }
        j += 1;
    }
    out
}

/// dbt would refuse a duplicate key outright. Being lenient is more useful than
/// refusing the whole block, but it is never silent.
fn push_var(out: &mut ProjectVars, var: ProjectVar) {
    if let Some(prev) = out.vars.iter().position(|v| v.name == var.name) {
        out.unparsed.push(Unparsed { line: var.line, message: "a duplicate key, the last one wins" });
        out.vars.remove(prev);
    }
    out.vars.push(var);
}

/// The order `adapter.dispatch` searches packages in for dbt's own macros,
/// which decides an incremental model's default strategy (`src/macros.rs`).
#[derive(Debug, Clone, Default, PartialEq)]
pub enum DispatchOrder {
    /// No `dispatch:` entry for the `dbt` namespace: dbt searches the root
    /// project, then its own packages.
    #[default]
    Default,
    /// The entry's `search_order`, as written.
    Given(Vec<String>),
    /// A `dispatch:` block that could not be read, which may or may not
    /// reorder dbt's own macros.
    Unreadable,
}

/// `dispatch:` at indentation 0, a sequence of mappings:
///
/// ```yaml
/// dispatch:
///   - macro_namespace: dbt
///     search_order: ['my_project', 'dbt']
/// ```
///
/// `search_order` may also be a block list on the lines below. Any other
/// shape makes the whole block unreadable rather than half read, since the
/// entry misread could be the one for `dbt`. An entry for another namespace
/// is skipped, whatever its order looks like.
pub fn dispatch_in(text: &str) -> DispatchOrder {
    use DispatchOrder::{Default, Given, Unreadable};
    let lines: Vec<&str> = text.lines().collect();
    let head = lines.iter().position(|l| {
        indent_of(l) == Some(0) && !skippable(l) && split_key(l).is_some_and(|(k, _)| k == "dispatch")
    });
    let Some(head) = head else { return Default };
    let rest = split_key(lines[head]).map(|(_, r)| r.trim()).unwrap_or("");
    if !rest.is_empty() && !rest.starts_with('#') {
        return Unreadable;
    }
    // Each line of the block, by its indentation. A sequence may sit at the
    // key's own indentation, so a `-` at 0 still belongs to it.
    let mut block: Vec<(usize, &str)> = Vec::new();
    for line in &lines[head + 1..] {
        if skippable(line) {
            continue;
        }
        let Some(ind) = indent_of(line) else { return Unreadable };
        if ind == 0 && !line.starts_with('-') {
            break;
        }
        block.push((ind, line[ind..].trim_end()));
    }

    let mut dbt: Option<DispatchOrder> = None;
    let mut i = 0;
    while i < block.len() {
        let (dash, text) = block[i];
        let Some(after) = text.strip_prefix('-') else { return Unreadable };
        if !(after.is_empty() || after.starts_with(' ')) {
            return Unreadable;
        }
        // The item's keys, each with its value on its own line and the lines
        // nested under it: the first key may share the dash's line.
        let mut keys: Vec<(String, &str, Vec<&str>)> = Vec::new();
        let first = after.trim_start();
        let mut key_ind = None;
        if !first.is_empty() {
            let Some((k, v)) = split_key(first) else { return Unreadable };
            keys.push((k, v, Vec::new()));
            key_ind = Some(dash + 1 + after.len() - first.len());
        }
        i += 1;
        while i < block.len() && block[i].0 > dash {
            let (ind, text) = block[i];
            let k_ind = *key_ind.get_or_insert(ind);
            if ind > k_ind || (ind == k_ind && text.starts_with('-')) {
                let Some(last) = keys.last_mut() else { return Unreadable };
                last.2.push(text);
            } else if ind == k_ind {
                let Some((k, v)) = split_key(text) else { return Unreadable };
                keys.push((k, v, Vec::new()));
            } else {
                return Unreadable;
            }
            i += 1;
        }

        let mut namespace = None;
        let mut order = None;
        for (k, v, nested) in &keys {
            match k.as_str() {
                "macro_namespace" if nested.is_empty() => {
                    if let Value::Scalar(s) = parse_value_or_empty(v) {
                        namespace = Some(s);
                    }
                }
                "search_order" => order = search_order(v, nested),
                _ => {}
            }
        }
        match namespace.as_deref() {
            None => return Unreadable,
            Some("dbt") if dbt.is_some() => return Unreadable,
            Some("dbt") => dbt = Some(order.map(Given).unwrap_or(Unreadable)),
            Some(_) => {}
        }
    }
    dbt.unwrap_or(Default)
}

fn parse_value_or_empty(v: &str) -> Value {
    if v.trim().is_empty() { Value::Bad("no value") } else { parse_value(v) }
}

/// A `search_order`, as a flow sequence on its own line or a block one below
/// it, of plain package names.
fn search_order(value: &str, nested: &[&str]) -> Option<Vec<String>> {
    let items = if value.trim().is_empty() {
        let mut items = Vec::new();
        for line in nested {
            let item = line.strip_prefix('-')?.trim();
            match parse_value_or_empty(item) {
                Value::Scalar(s) => items.push(s),
                _ => return None,
            }
        }
        items
    } else {
        if !nested.is_empty() {
            return None;
        }
        match parse_value(value) {
            Value::List(items) => items,
            _ => return None,
        }
    };
    let plain = |s: &String| !s.is_empty() && !s.contains('{');
    (!items.is_empty() && items.iter().all(plain)).then_some(items)
}

/// `dispatch:` in the project's own `dbt_project.yml`. A file too large or
/// unreadable may hold any order; without a file, dbt has no project at all.
pub fn dispatch(root: &Path) -> DispatchOrder {
    match project_text(root) {
        Some(text) => dispatch_in(&text),
        None if root.join("dbt_project.yml").exists() => DispatchOrder::Unreadable,
        None => DispatchOrder::Default,
    }
}

/// The top-level `name:`, as a plain or quoted scalar and nothing else.
pub fn name(root: &Path) -> Option<String> {
    top_scalar(&project_text(root)?, "name")
}

/// The top-level `profile:`, the profile dbt reads the target from. No
/// fallback to `name:`: dbt has none either.
pub fn profile(root: &Path) -> Option<String> {
    top_scalar(&project_text(root)?, "profile")
}

fn project_text(root: &Path) -> Option<String> {
    let path = root.join("dbt_project.yml");
    let meta = std::fs::metadata(&path).ok()?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return None;
    }
    Some(crate::envs::decode(&std::fs::read(&path).ok()?))
}

/// The value half of a `key: value` line as a plain or quoted scalar, or None
/// for anything else: a list, a mapping, a block scalar, an anchor.
pub(crate) fn scalar(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        return Some(String::new());
    }
    match parse_value(value) {
        Value::Scalar(s) => Some(s),
        _ => None,
    }
}

#[cfg(test)]
fn name_in(text: &str) -> Option<String> {
    top_scalar(text, "name")
}

fn top_scalar(text: &str, wanted: &str) -> Option<String> {
    for line in text.lines() {
        if indent_of(line) != Some(0) || skippable(line) {
            continue;
        }
        let Some((key, rest)) = split_key(line) else { continue };
        if key != wanted {
            continue;
        }
        if rest.trim().is_empty() {
            return None;
        }
        return match parse_value(rest) {
            Value::Scalar(s) if !s.is_empty() && !s.contains('{') => Some(s),
            _ => None,
        };
    }
    None
}

/// The directories a config block's keys are relative to, one list per block.
/// dbt's defaults when the key is absent, and nothing at all when the key is
/// there and unreadable: a guessed root would point a config at the wrong
/// folder (0036).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ProjectPaths {
    pub models: Vec<String>,
    pub seeds: Vec<String>,
    pub snapshots: Vec<String>,
    pub analyses: Vec<String>,
    pub macros: Vec<String>,
    pub tests: Vec<String>,
}

impl Default for ProjectPaths {
    fn default() -> Self {
        let one = |d: &str| vec![d.to_string()];
        ProjectPaths {
            models: one("models"),
            seeds: one("seeds"),
            snapshots: one("snapshots"),
            analyses: one("analyses"),
            macros: one("macros"),
            tests: one("tests"),
        }
    }
}

/// The extensions a leaf key may carry in each block, for the case where the
/// last key of a chain names one resource rather than a directory. Per block,
/// since a seed is never a `.sql`. `data_tests:` is `tests:` renamed in
/// dbt-core 1.8, and both spellings are in the wild.
const BLOCK_EXTS: &[(&str, &[&str])] = &[
    ("models", &["sql", "py"]),
    ("seeds", &["csv"]),
    ("snapshots", &["sql"]),
    ("analyses", &["sql"]),
    ("macros", &["sql"]),
    ("tests", &["sql"]),
    ("data_tests", &["sql"]),
];

impl ProjectPaths {
    /// The roots a block's keys are looked for under, and the extensions a
    /// leaf may carry there, or None when the block is not one whose keys are
    /// paths.
    ///
    /// A test block takes the model roots as well, and that is not a hedge: a
    /// singular test lives under `test-paths`, while a generic test's fqn
    /// mirrors the path of the model it hangs on, so one key under
    /// `data_tests:` names a folder in the test tree and the next names one in
    /// the model tree. The project this was built for writes both.
    pub fn block(&self, block: &str) -> Option<(Vec<String>, &'static [&'static str])> {
        let exts = BLOCK_EXTS.iter().find(|(b, _)| *b == block).map(|(_, e)| *e)?;
        let roots = match block {
            "models" => self.models.clone(),
            "seeds" => self.seeds.clone(),
            "snapshots" => self.snapshots.clone(),
            "analyses" => self.analyses.clone(),
            "macros" => self.macros.clone(),
            "tests" | "data_tests" => [self.tests.clone(), self.models.clone()].concat(),
            _ => return None,
        };
        Some((roots, exts))
    }

    /// Every block whose keys are paths, with the roots it searches: what the
    /// editor needs to say where a key was looked for when it found nothing.
    pub fn searched(&self) -> std::collections::BTreeMap<&'static str, Vec<String>> {
        BLOCK_EXTS.iter().filter_map(|(b, _)| self.block(b).map(|(roots, _)| (*b, roots))).collect()
    }
}

/// One root as a project-relative path, or None when it cannot be one: empty,
/// or Jinja, which nothing here evaluates (0009).
fn clean_root(s: String) -> Option<String> {
    let s = crate::graph::slashed(s);
    let s = s.trim().trim_start_matches("./").trim_end_matches('/').to_string();
    if s.is_empty() || s.contains('{') {
        return None;
    }
    Some(s)
}

/// The value of a `*-paths` key, in the three shapes a project writes it:
/// `["models"]`, `models`, and a block sequence on the lines below. Anything
/// else gives no roots at all, rather than a guess.
fn path_list(lines: &[&str], at: &mut usize, rest: &str) -> Vec<String> {
    let rest = rest.trim();
    if !rest.is_empty() {
        return match parse_value(rest) {
            Value::List(items) => {
                let cleaned: Option<Vec<String>> = items.into_iter().map(clean_root).collect();
                cleaned.unwrap_or_default()
            }
            Value::Scalar(s) => clean_root(s).into_iter().collect(),
            Value::Bad(_) => Vec::new(),
        };
    }
    let mut out = Vec::new();
    while *at < lines.len() {
        let line = lines[*at];
        if skippable(line) {
            *at += 1;
            continue;
        }
        match indent_of(line) {
            Some(i) if i > 0 => {}
            _ => break,
        }
        let Some(item) = line.trim().strip_prefix('-') else { return Vec::new() };
        let item = item.trim();
        if item.is_empty() {
            return Vec::new();
        }
        match parse_value(item) {
            Value::Scalar(s) => match clean_root(s) {
                Some(root) => out.push(root),
                None => return Vec::new(),
            },
            _ => return Vec::new(),
        }
        *at += 1;
    }
    out
}

/// The six resource path keys, read with the same scanner the `vars:` block
/// uses. `source-paths` and `data-paths` are what they were called before
/// dbt 1.0, and projects on the VM still carry them.
pub fn paths_in(text: &str) -> ProjectPaths {
    let mut out = ProjectPaths::default();
    let lines: Vec<&str> = text.lines().collect();
    let mut j = 0;
    while j < lines.len() {
        let line = lines[j];
        j += 1;
        if skippable(line) || indent_of(line) != Some(0) {
            continue;
        }
        let Some((key, rest)) = split_key(line) else { continue };
        // A key written twice: the last one wins, as dbt reads it.
        let value = |out: &mut Vec<String>, j: &mut usize| *out = path_list(&lines, j, rest);
        match key.as_str() {
            "model-paths" | "source-paths" => value(&mut out.models, &mut j),
            "seed-paths" | "data-paths" => value(&mut out.seeds, &mut j),
            "snapshot-paths" => value(&mut out.snapshots, &mut j),
            "analysis-paths" => value(&mut out.analyses, &mut j),
            "macro-paths" => value(&mut out.macros, &mut j),
            "test-paths" => value(&mut out.tests, &mut j),
            _ => {}
        }
    }
    out
}

pub fn read_paths(root: &Path) -> ProjectPaths {
    let path = root.join("dbt_project.yml");
    let Ok(meta) = std::fs::metadata(&path) else {
        return ProjectPaths::default();
    };
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return ProjectPaths::default();
    }
    match std::fs::read(&path) {
        Ok(bytes) => paths_in(&crate::envs::decode(&bytes)),
        Err(_) => ProjectPaths::default(),
    }
}

/// Where a chain of config keys points inside the project.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Placed {
    pub path: String,
    /// False when the chain's last key named one resource file rather than a
    /// directory, which decides whether the click opens a tab or the tree.
    pub dir: bool,
    /// The other roots the same chain exists under, when a block has several.
    /// The click goes to `path`; the card says there is more to it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub also: Vec<String>,
}

/// Where `dbt deps` put a package, when it put one there at all. Asked of the
/// disk rather than of the manifest, because a package of macros alone owns no
/// node to be named by and would read as a folder that is not there.
fn installed_package(root: &Path, name: &str) -> Option<String> {
    crate::macros::PACKAGE_DIRS
        .iter()
        .map(|dir| format!("{dir}/{name}"))
        .find(|rel| crate::files::resolve(root, rel).is_ok_and(|p| p.is_dir()))
}

/// Where a chain of config keys under `block` points, or None when nothing is
/// there. `name` is the project's own name, so the level dbt requires under
/// each block is dropped; a first key naming an installed package is looked
/// for where `dbt deps` puts it, under that package's conventional roots,
/// since reading its own `dbt_project.yml` is refused (0036).
pub fn place(
    root: &Path,
    paths: &ProjectPaths,
    name: Option<&str>,
    block: &str,
    chain: &[String],
) -> Option<Placed> {
    let (own_roots, exts) = paths.block(block)?;
    let own_roots = own_roots.as_slice();
    // A key holding a separator is one name to dbt and matches no resource, so
    // a link built from it would point at the one folder it does not reach.
    if chain.iter().any(|s| s.trim().is_empty() || s.contains('/') || s.contains('\\') || s.starts_with('+')) {
        return None;
    }

    let default_roots = ProjectPaths::default().block(block).map(|(r, _)| r);
    let first = chain.first().map(String::as_str);
    // The project's own level first, since dbt requires it; then a package,
    // which is the only other thing that level can be.
    let package = match first {
        Some(f) if Some(f) != name => installed_package(root, f),
        _ => None,
    };
    let (prefixes, roots, rest): (Vec<String>, &[String], &[String]) = match (first, &package) {
        (Some(f), _) if Some(f) == name => (vec![String::new()], own_roots, &chain[1..]),
        (Some(_), Some(dir)) => (
            vec![format!("{dir}/")],
            default_roots.as_deref().unwrap_or(own_roots),
            &chain[1..],
        ),
        _ => (vec![String::new()], own_roots, chain),
    };

    let tail = rest.join("/");
    let mut candidates = Vec::new();
    for prefix in &prefixes {
        for base in roots {
            candidates.push(if tail.is_empty() { format!("{prefix}{base}") } else { format!("{prefix}{base}/{tail}") });
        }
    }

    // Through files::resolve, so a chain climbing out with `..`, a drive letter
    // or a folder symlinked in from elsewhere is refused like any other path.
    let on_disk = |rel: &String, dir: bool| {
        crate::files::resolve(root, rel).is_ok_and(|p| if dir { p.is_dir() } else { p.is_file() })
    };
    let mut dirs = candidates.iter().filter(|rel| on_disk(rel, true));
    if let Some(first) = dirs.next() {
        return Some(Placed { path: first.clone(), dir: true, also: dirs.cloned().collect() });
    }
    if tail.is_empty() {
        return package.filter(|_| rest.is_empty()).map(|dir| Placed { path: dir, dir: true, also: Vec::new() });
    }
    let mut files = candidates
        .iter()
        .flat_map(|rel| exts.iter().map(move |ext| format!("{rel}.{ext}")))
        .filter(|rel| on_disk(rel, false));
    if let Some(first) = files.next() {
        return Some(Placed { path: first, dir: false, also: files.collect() });
    }
    // A package naming no folder of its own still has a folder: the package.
    // Its roots are in its own project file, which is not read (0036), so this
    // is as far as a key that is only a package name goes.
    match package {
        Some(dir) if rest.is_empty() => Some(Placed { path: dir, dir: true, also: Vec::new() }),
        _ => None,
    }
}

pub fn read(root: &Path) -> ProjectVars {
    let path = root.join("dbt_project.yml");
    let Ok(meta) = std::fs::metadata(&path) else {
        return ProjectVars::default();
    };
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return ProjectVars::default();
    }
    match std::fs::read(&path) {
        Ok(bytes) => scan(&crate::envs::decode(&bytes)),
        Err(_) => ProjectVars::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_is_the_top_level_key_and_only_that() {
        let text = "# shop\nmodels:\n  name: not_this\nname: 'shop'  # the project\nname: later\n";
        assert_eq!(name_in(text).as_deref(), Some("shop"));
        assert_eq!(name_in("version: 2\nname: shop_2\r\n").as_deref(), Some("shop_2"));
        // Nothing, or something this does not read, is no name at all.
        assert_eq!(name_in("name:\n"), None);
        assert_eq!(name_in("name: \"{{ env_var('P') }}\"\n"), None);
        assert_eq!(name_in("models:\n  shop:\n    +schema: x\n"), None);
    }

    #[test]
    fn the_profile_is_its_own_top_level_key() {
        let text = "name: shop\nprofile: \"shop_profile\"  # dbt reads its target here\nmodels:\n  profile: no\n";
        assert_eq!(top_scalar(text, "profile").as_deref(), Some("shop_profile"));
        // No profile, no fallback to the name: dbt has none.
        assert_eq!(top_scalar("name: shop\n", "profile"), None);
        assert_eq!(scalar(" 'dev'  ").as_deref(), Some("dev"));
        assert_eq!(scalar("*alias"), None);
        assert_eq!(scalar("").as_deref(), Some(""));
    }

    fn names(p: &ProjectVars) -> Vec<&str> {
        p.vars.iter().map(|v| v.name.as_str()).collect()
    }
    fn raw<'a>(p: &'a ProjectVars, name: &str) -> &'a str {
        p.vars.iter().find(|v| v.name == name).map(|v| v.raw.as_str()).unwrap_or("<missing>")
    }

    #[test]
    fn flat_keys_at_two_and_four_space_indent() {
        let two = scan("vars:\n  alpha: 1\n  beta: two\n");
        assert_eq!(names(&two), ["alpha", "beta"]);
        let four = scan("vars:\n    alpha: 1\n    beta: two\n");
        assert_eq!(names(&four), ["alpha", "beta"]);
        assert!(four.unparsed.is_empty());
    }

    #[test]
    fn unquoted_scalars_keep_their_shape() {
        let p = scan(concat!(
            "vars:\n",
            "  zone: Europe/London\n",
            "  lag: 60 minutes\n",
            "  stamp: current_timestamp()\n",
            "  prefix: EXT_\n",
        ));
        assert_eq!(raw(&p, "zone"), "Europe/London");
        assert_eq!(raw(&p, "lag"), "60 minutes");
        assert_eq!(raw(&p, "stamp"), "current_timestamp()");
        assert_eq!(raw(&p, "prefix"), "EXT_");
    }

    #[test]
    fn an_inline_comment_needs_a_space_before_the_hash() {
        let p = scan("vars:\n  days: 1   # widen as needed\n  tag: a#b\n");
        assert_eq!(raw(&p, "days"), "1");
        assert_eq!(raw(&p, "tag"), "a#b");
    }

    #[test]
    fn a_hash_inside_quotes_is_not_a_comment() {
        let p = scan("vars:\n  label: 'lot # 4'\n");
        assert_eq!(raw(&p, "label"), "lot # 4");
    }

    #[test]
    fn a_comment_line_does_not_end_the_block() {
        let p = scan(concat!(
            "vars:\n",
            "  alpha: 1\n",
            "  ## a section heading\n",
            " # a comment indented by one, as people write them\n",
            "  beta: 2\n",
        ));
        assert_eq!(names(&p), ["alpha", "beta"]);
        assert!(p.unparsed.is_empty());
    }

    #[test]
    fn a_blank_line_does_not_end_the_block() {
        let p = scan("vars:\n  alpha: 1\n\n  beta: 2\n");
        assert_eq!(names(&p), ["alpha", "beta"]);
    }

    #[test]
    fn the_block_ends_at_the_next_top_level_key() {
        let p = scan("vars:\n  alpha: 1\nquoting:\n  database: false\n");
        assert_eq!(names(&p), ["alpha"]);
        assert!(p.unparsed.is_empty());
    }

    #[test]
    fn the_block_ends_at_end_of_file() {
        let p = scan("vars:\n  alpha: 1");
        assert_eq!(names(&p), ["alpha"]);
    }

    #[test]
    fn a_flow_sequence_becomes_a_list() {
        let p = scan("vars:\n  codes: ['GBP', \"USD\" , EUR]\n");
        let v = &p.vars[0];
        assert_eq!(v.list.as_deref(), Some(["GBP".to_string(), "USD".to_string(), "EUR".to_string()].as_slice()));
        assert_eq!(v.raw, "GBP, USD, EUR");
    }

    #[test]
    fn a_block_sequence_is_a_list_not_a_package() {
        // The commonest way to write a list var, and it must not be mistaken
        // for a package scope, which would make the var vanish silently.
        let p = scan("vars:\n  regions:\n    - EU\n    - 'US'\n  alpha: 1\n");
        assert_eq!(names(&p), ["regions", "alpha"]);
        assert_eq!(
            p.vars[0].list.as_deref(),
            Some(["EU".to_string(), "US".to_string()].as_slice())
        );
        assert_eq!(raw(&p, "regions"), "EU, US");
        assert!(p.packages.is_empty());
        assert!(p.unparsed.is_empty());
    }

    #[test]
    fn a_block_sequence_with_an_inline_comment_keeps_the_value() {
        let p = scan("vars:\n  regions:\n    - EU   # primary\n");
        assert_eq!(p.vars[0].list.as_deref(), Some(["EU".to_string()].as_slice()));
    }

    #[test]
    fn a_sequence_of_mappings_is_reported_not_guessed() {
        let p = scan("vars:\n  rules:\n    - name: a\n      to: b\n");
        assert!(p.vars.is_empty());
        assert_eq!(p.unparsed.len(), 1);
    }

    #[test]
    fn a_nested_flow_collection_is_unparsed() {
        let p = scan("vars:\n  nested: [[1, 2], 3]\n");
        assert!(p.vars.is_empty());
        assert_eq!(p.unparsed.len(), 1);
    }

    #[test]
    fn a_jinja_value_is_kept_as_text() {
        let p = scan("vars:\n  cutoff: \"{{ env_var('DBT_CUTOFF', '1900-01-01') }}\"\n");
        assert_eq!(raw(&p, "cutoff"), "{{ env_var('DBT_CUTOFF', '1900-01-01') }}");
        assert!(p.vars[0].jinja);
    }

    #[test]
    fn a_plain_value_is_not_flagged_as_jinja() {
        let p = scan("vars:\n  prefix: EXT_\n");
        assert!(!p.vars[0].jinja);
    }

    #[test]
    fn a_quoted_key_may_contain_a_colon() {
        let p = scan("vars:\n  \"pkg:zone\": 'America/Los_Angeles'\n");
        assert_eq!(names(&p), ["pkg:zone"]);
        assert_eq!(raw(&p, "pkg:zone"), "America/Los_Angeles");
    }

    #[test]
    fn a_key_with_no_value_is_a_null_var_not_an_error() {
        // Projects write this on purpose to mean "unset", and var() then returns
        // None, so it is a var rather than a line that could not be read.
        let p = scan("vars:\n  lookback_days:\n  alpha: 1\n");
        assert_eq!(names(&p), ["lookback_days", "alpha"]);
        assert!(p.vars[0].null);
        assert_eq!(raw(&p, "lookback_days"), "");
        assert!(p.unparsed.is_empty());
    }

    #[test]
    fn a_key_with_no_space_after_the_colon_is_not_a_mapping() {
        let p = scan("vars:\n  alpha:1\n");
        assert!(p.vars.is_empty());
        assert_eq!(p.unparsed[0].message, "not a key and a value");
    }

    #[test]
    fn package_scoped_vars_are_reported_not_flattened() {
        let p = scan("vars:\n  alpha: 1\n  my_pkg:\n    inner: 2\n    other: 3\n  beta: 4\n");
        assert_eq!(names(&p), ["alpha", "beta"]);
        assert_eq!(p.packages.len(), 1);
        assert_eq!(p.packages[0].package, "my_pkg");
        assert_eq!(p.packages[0].count, 2);
    }

    #[test]
    fn an_indented_vars_key_is_not_the_project_block() {
        let p = scan("models:\n  my_project:\n    vars:\n      alpha: 1\n");
        assert!(!p.found);
        assert!(p.vars.is_empty());
    }

    #[test]
    fn no_vars_block_at_all() {
        let p = scan("name: demo\nversion: '1.0'\n");
        assert!(!p.found);
    }

    #[test]
    fn an_empty_block_is_found_but_has_nothing() {
        let p = scan("vars:\nquoting:\n  database: false\n");
        assert!(p.found);
        assert!(p.vars.is_empty());
    }

    #[test]
    fn a_tab_indent_is_unparsed() {
        let p = scan("vars:\n  alpha: 1\n\tbeta: 2\n");
        assert_eq!(names(&p), ["alpha"]);
        assert_eq!(p.unparsed[0].message, "tabs are not valid YAML indentation");
    }

    #[test]
    fn a_merge_key_is_unparsed() {
        let p = scan("vars:\n  <<: *defaults\n  alpha: 1\n");
        assert_eq!(names(&p), ["alpha"]);
        assert_eq!(p.unparsed[0].message, "a merge key is not read");
    }

    #[test]
    fn a_block_scalar_and_a_flow_mapping_are_unparsed() {
        let p = scan("vars:\n  text: |\n    a line\n  map: {a: 1}\n");
        assert!(p.vars.is_empty());
        assert_eq!(p.unparsed.len(), 3, "block scalar, its indented body, then the flow mapping");
    }

    #[test]
    fn a_duplicate_key_keeps_the_last_and_warns() {
        let p = scan("vars:\n  alpha: 1\n  alpha: 2\n");
        assert_eq!(names(&p), ["alpha"]);
        assert_eq!(raw(&p, "alpha"), "2");
        assert_eq!(p.unparsed[0].message, "a duplicate key, the last one wins");
    }

    #[test]
    fn crlf_line_endings_read_the_same() {
        let unix = scan("vars:\n  alpha: 1\n  beta: two\n");
        let dos = scan("vars:\r\n  alpha: 1\r\n  beta: two\r\n");
        assert_eq!(unix, dos);
    }

    #[test]
    fn a_utf8_bom_does_not_rename_the_first_key() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"vars:\n  alpha: 1\n");
        let p = scan(&crate::envs::decode(&bytes));
        assert_eq!(names(&p), ["alpha"]);
    }

    #[test]
    fn line_numbers_are_one_based() {
        let p = scan("name: demo\n\nvars:\n  alpha: 1\n");
        assert_eq!(p.vars[0].line, 4);
    }

    // ------------------------------------------------------- resource paths --

    /// A throwaway project tree, named after the test so two can run at once.
    fn tree(tag: &str, dirs: &[&str], files: &[&str]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("dbt-edith-paths-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in dirs {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        for file in files {
            let at = root.join(file);
            std::fs::create_dir_all(at.parent().unwrap()).unwrap();
            std::fs::write(at, "").unwrap();
        }
        root
    }

    #[test]
    fn the_defaults_are_dbts_own_when_no_key_says_otherwise() {
        let p = paths_in("name: demo\n");
        assert_eq!(p, ProjectPaths::default());
        assert_eq!(p.models, ["models"]);
        assert_eq!(p.analyses, ["analyses"]);
    }

    #[test]
    fn a_flow_sequence_a_scalar_and_a_block_sequence_all_read() {
        assert_eq!(paths_in("model-paths: [\"models\", 'extra']\n").models, ["models", "extra"]);
        assert_eq!(paths_in("model-paths: models\n").models, ["models"]);
        assert_eq!(paths_in("model-paths:\n  - models\n  - extra\nseed-paths: [seeds]\n").models, ["models", "extra"]);
    }

    #[test]
    fn a_block_sequence_does_not_swallow_the_keys_after_it() {
        let p = paths_in("model-paths:\n  - models\nmacro-paths: [mac]\n");
        assert_eq!(p.models, ["models"]);
        assert_eq!(p.macros, ["mac"]);
    }

    #[test]
    fn the_pre_1_0_names_are_read_too() {
        assert_eq!(paths_in("source-paths: [app]\n").models, ["app"]);
        assert_eq!(paths_in("data-paths: [raw]\n").seeds, ["raw"]);
    }

    #[test]
    fn a_value_that_cannot_be_read_gives_no_roots_at_all() {
        // No root rather than a guess: a wrong root links a config to the
        // wrong folder, which is worse than no link (0036).
        assert!(paths_in("model-paths: \"{{ var('where') }}\"\n").models.is_empty());
        assert!(paths_in("model-paths: [[a]]\n").models.is_empty());
        assert!(paths_in("model-paths:\n  - models\n  - [a]\n").models.is_empty());
        assert!(paths_in("model-paths:\n").models.is_empty());
    }

    #[test]
    fn a_root_is_tidied_the_way_every_other_path_here_is() {
        assert_eq!(paths_in("model-paths: [\"./models/\"]\n").models, ["models"]);
        assert_eq!(paths_in("model-paths: [\"models\\\\staging\"]\n").models, ["models/staging"]);
    }

    #[test]
    fn a_tab_in_the_indentation_drops_the_line() {
        assert_eq!(paths_in(" \tmodel-paths: [app]\n").models, ["models"]);
    }

    #[test]
    fn a_chain_places_a_folder_under_the_root_the_project_names() {
        let root = tree("dir", &["app/staging/crm"], &[]);
        let paths = paths_in("model-paths: [app]\n");
        let p = place(&root, &paths, Some("demo"), "models", &str_chain(&["demo", "staging", "crm"])).unwrap();
        assert_eq!(p.path, "app/staging/crm");
        assert!(p.dir);
        assert!(p.also.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_block_key_itself_places_its_root() {
        let root = tree("block", &["models"], &[]);
        let p = place(&root, &ProjectPaths::default(), Some("demo"), "models", &[]).unwrap();
        assert_eq!(p.path, "models");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_leaf_naming_one_resource_places_its_file() {
        let root = tree("leaf", &["models/staging"], &["models/staging/stg_orders.sql", "seeds/countries.csv"]);
        let d = ProjectPaths::default();
        let model = place(&root, &d, Some("demo"), "models", &str_chain(&["demo", "staging", "stg_orders"])).unwrap();
        assert_eq!((model.path.as_str(), model.dir), ("models/staging/stg_orders.sql", false));
        // Per block: a seed is never a .sql.
        let seed = place(&root, &d, Some("demo"), "seeds", &str_chain(&["demo", "countries"])).unwrap();
        assert_eq!(seed.path, "seeds/countries.csv");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_second_root_answers_when_the_first_has_nothing_and_the_card_hears_about_both() {
        let root = tree("also", &["models/staging", "extra/staging"], &[]);
        let paths = paths_in("model-paths: [other, models, extra]\n");
        let p = place(&root, &paths, Some("demo"), "models", &str_chain(&["staging"])).unwrap();
        assert_eq!(p.path, "models/staging");
        assert_eq!(p.also, ["extra/staging"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_package_key_is_looked_for_where_dbt_deps_puts_it() {
        let root = tree("pkg", &["dbt_packages/automate_dv/macros/hub"], &[]);
        // The root project moved its own models; a package keeps the
        // conventional roots, since its own project file is not read (0036).
        let paths = paths_in("model-paths: [app]\n");
        let p = place(&root, &paths, Some("demo"), "macros", &str_chain(&["automate_dv", "hub"])).unwrap();
        assert_eq!(p.path, "dbt_packages/automate_dv/macros/hub");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_package_key_alone_falls_back_to_the_package_folder() {
        // A package that keeps its models somewhere this project cannot guess
        // still has one folder worth opening: its own.
        let root = tree("pkgroot", &["dbt_packages/dbt_artifacts/elsewhere"], &[]);
        let d = ProjectPaths::default();
        let p = place(&root, &d, Some("demo"), "models", &str_chain(&["dbt_artifacts"])).unwrap();
        assert_eq!(p.path, "dbt_packages/dbt_artifacts");
        // One level deeper there is nothing honest to point at.
        assert!(place(&root, &d, Some("demo"), "models", &str_chain(&["dbt_artifacts", "staging"])).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_package_is_known_by_its_folder_not_by_a_node() {
        // dbt_utils ships macros and no node at all, so the manifest never
        // names it; the folder dbt deps wrote is what says it is a package.
        let root = tree("pkgmacro", &["dbt_modules/dbt_utils/macros/sql"], &[]);
        let p = place(&root, &ProjectPaths::default(), Some("demo"), "macros", &str_chain(&["dbt_utils", "sql"])).unwrap();
        assert_eq!(p.path, "dbt_modules/dbt_utils/macros/sql");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn nothing_on_disk_is_no_answer() {
        let root = tree("none", &["models"], &[]);
        let d = ProjectPaths::default();
        assert!(place(&root, &d, Some("demo"), "models", &str_chain(&["demo", "stagin"])).is_none());
        // A block whose keys are not paths never had an answer to give.
        assert!(place(&root, &d, Some("demo"), "sources", &str_chain(&["demo"])).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_key_holding_a_separator_or_a_plus_is_refused() {
        let root = tree("odd", &["models/staging"], &[]);
        let d = ProjectPaths::default();
        // To dbt `staging/crm` is one name matching no resource, so a link
        // from it would point at the folder the config does not reach.
        assert!(place(&root, &d, Some("demo"), "models", &str_chain(&["staging/crm"])).is_none());
        assert!(place(&root, &d, Some("demo"), "models", &str_chain(&["staging\\crm"])).is_none());
        assert!(place(&root, &d, Some("demo"), "models", &str_chain(&["+tags"])).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_chain_cannot_climb_out_of_the_project() {
        let root = tree("escape", &["models"], &[]);
        let paths = paths_in("model-paths: [\"../elsewhere\"]\n");
        assert!(place(&root, &paths, Some("demo"), "models", &[]).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn both_spellings_of_the_test_block_share_one_root() {
        let root = tree("tests", &["checks/generic"], &[]);
        let paths = paths_in("test-paths: [checks]\n");
        for block in ["tests", "data_tests"] {
            let p = place(&root, &paths, Some("demo"), block, &str_chain(&["generic"])).unwrap();
            assert_eq!(p.path, "checks/generic");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_test_key_may_name_a_folder_in_either_tree() {
        // A singular test sits under test-paths; a generic test's fqn mirrors
        // the path of the model it hangs on, and real projects write both.
        let root = tree("twotrees", &["tests/singular", "models/staging"], &[]);
        let d = ProjectPaths::default();
        let singular = place(&root, &d, Some("demo"), "data_tests", &str_chain(&["demo", "singular"])).unwrap();
        assert_eq!(singular.path, "tests/singular");
        let generic = place(&root, &d, Some("demo"), "data_tests", &str_chain(&["demo", "staging"])).unwrap();
        assert_eq!(generic.path, "models/staging");
        // The model tree is not searched for anything else.
        assert!(place(&root, &d, Some("demo"), "seeds", &str_chain(&["demo", "staging"])).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    fn str_chain(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_dispatch_order_for_dbt_is_read_in_either_list_form() {
        let order = |s: &[&str]| DispatchOrder::Given(s.iter().map(|x| x.to_string()).collect());
        let flow = "name: shop\ndispatch:\n  - macro_namespace: dbt_utils\n    search_order: ['shop', 'dbt_utils']\n  - macro_namespace: dbt\n    search_order: ['shop', \"dbt\"]  # ours first\nmodels:\n  shop: {}\n";
        assert_eq!(dispatch_in(flow), order(&["shop", "dbt"]));
        let block = "dispatch:\n- search_order:\n  - dbt\n  - shop\n  macro_namespace: 'dbt'\n";
        assert_eq!(dispatch_in(block), order(&["dbt", "shop"]));
        let under = "dispatch:\n    -   macro_namespace: dbt\n        search_order:\n        - shop\n        - dbt\n";
        assert_eq!(dispatch_in(under), order(&["shop", "dbt"]));
    }

    #[test]
    fn no_entry_for_dbt_leaves_dbt_s_own_order() {
        assert_eq!(dispatch_in("name: shop\n"), DispatchOrder::Default);
        assert_eq!(dispatch_in("dispatch:\n  - macro_namespace: dbt_utils\n    search_order: ['shop', 'dbt_utils']\n"), DispatchOrder::Default);
        // Another namespace's order is not read, so its shape does not matter.
        assert_eq!(dispatch_in("dispatch:\n  - macro_namespace: dbt_utils\n    search_order: \"{{ var('x') }}\"\n"), DispatchOrder::Default);
        // Nested under a model folder, it is a folder name, not the block.
        assert_eq!(dispatch_in("models:\n  dispatch:\n    - macro_namespace: dbt\n"), DispatchOrder::Default);
    }

    #[test]
    fn a_dispatch_block_it_cannot_read_says_so() {
        let unreadable = |text: &str| assert_eq!(dispatch_in(text), DispatchOrder::Unreadable, "{text}");
        unreadable("dispatch: [{macro_namespace: dbt, search_order: [shop, dbt]}]\n");
        unreadable("dispatch:\n  - {macro_namespace: dbt, search_order: [shop, dbt]}\n");
        unreadable("dispatch:\n  - macro_namespace: dbt\n    search_order: \"{{ var('order') }}\"\n");
        unreadable("dispatch:\n  - macro_namespace: dbt\n    search_order: [\"{{ project_name }}\", dbt]\n");
        unreadable("dispatch:\n  - macro_namespace: dbt\n");
        unreadable("dispatch:\n  - search_order: [shop, dbt]\n");
        unreadable("dispatch:\n  - macro_namespace: dbt\n    search_order: [shop, dbt]\n  - macro_namespace: dbt\n    search_order: [dbt]\n");
        unreadable("dispatch:\n  macro_namespace: dbt\n");
        unreadable("dispatch:\n\t- macro_namespace: dbt\n");
    }

    #[test]
    fn a_project_file_too_large_to_read_may_hold_any_order() {
        let dir = std::env::temp_dir().join(format!("dbt-edith-dispatch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(dispatch(&dir), DispatchOrder::Default, "no file, no project");
        let mut big = "name: shop\n".to_string();
        big.push_str(&"# padding\n".repeat((MAX_BYTES as usize) / 10 + 1));
        std::fs::write(dir.join("dbt_project.yml"), big).unwrap();
        assert_eq!(dispatch(&dir), DispatchOrder::Unreadable);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
