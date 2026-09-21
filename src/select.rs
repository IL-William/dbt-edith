//! Parsing and resolving a dbt node-selection expression against the graph.
//!
//! This is dbt-core's `graph/selector_methods.py` re-implemented over the
//! manifest already in memory. Nothing here runs dbt (0002, 0024), so fidelity
//! against `dbt ls -s` is this module's whole contract, and every place it
//! knowingly differs is named in a comment beside the code that differs.
//!
//! Invariant: a disabled node is never returned, and never walked through.
//! dbt's graph does not contain them, so an answer here must not contain them
//! either, not even as a stepping stone between two enabled nodes.

use crate::graph::{Graph, Kind, Node};

/// What the expression may grow to before it is refused, in bytes. Well past
/// any selector a person types, and short enough that a pasted file is caught
/// before any of it is parsed.
const MAX_LEN: usize = 4096;
/// Terms across both halves. Each one is a full scan of the node vector.
const MAX_TERMS: usize = 64;

/// The methods this build understands, in the order the error message lists
/// them. `config.*` keys are spelled out because naming the key is the whole
/// difficulty of that method.
const KNOWN: &str = "fqn, tag, path, file, package, resource_type, source, exposure, \
                     config.materialized, config.schema, config.database, config.alias, \
                     config.incremental_strategy, config.unique_key";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConfigKey {
    Materialized,
    Schema,
    Database,
    Alias,
    Strategy,
    UniqueKey,
}

/// Which attribute a term matches on. `Fqn` is the implicit method.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Method {
    Fqn,
    Path,
    File,
    Tag,
    Package,
    ResourceType,
    Source,
    Exposure,
    Config(ConfigKey),
}

/// How far a `+` reaches. `None` is dbt's bare `+`: no limit.
pub type Depth = Option<u32>;

/// One `method:value` with its graph operators, e.g. `2+tag:nightly+`.
#[derive(Clone, Debug)]
pub struct Term {
    pub method: Method,
    pub value: String,
    /// `@`: the node, its descendants, and every ancestor of those descendants.
    pub at: bool,
    /// `+model` / `N+model`.
    pub parents: Option<Depth>,
    /// `model+` / `model+N`.
    pub children: Option<Depth>,
    /// The term as typed, for the warning that names it.
    pub raw: String,
}

/// A union of intersections: `a,b c` is `(a ∩ b) ∪ c`. dbt has no parentheses
/// on the command line and neither does this.
#[derive(Clone, Debug, Default)]
pub struct Union(pub Vec<Vec<Term>>);

/// A whole line out of the box, after any pasted command prefix was stripped.
#[derive(Clone, Debug)]
pub struct Expr {
    pub include: Union,
    pub exclude: Union,
    /// The include half as parsed, which is what the `dbt ls` button quotes.
    pub select: String,
    /// The exclude half as parsed, empty when there was none.
    pub excluded: String,
    /// True when a `dbt ls -s` or `--select` was stripped off the front.
    pub stripped: bool,
}

/// Whether tests take part at all.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tests {
    /// Tests are not in the universe: no term matches one, and none is added.
    Excluded,
    /// dbt's default indirect selection: a test joins when a parent is in.
    Eager,
}

/// What the resolver decided, and what it could not do.
#[derive(Debug, Default)]
pub struct Resolved {
    /// Node indices, ascending, deduped, never disabled.
    pub nodes: Vec<u32>,
    /// One line per term that matched nothing, in the order typed.
    pub warnings: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SelectError {
    Empty,
    TooLong { len: usize },
    TooManyTerms { count: usize },
    UnknownMethod { method: String, pos: usize },
    UnknownFlag { flag: String, pos: usize },
    EmptyTerm { pos: usize },
    BadDepth { term: String, pos: usize },
    BadSource { value: String, pos: usize },
}

impl SelectError {
    /// A stable token for the frontend, never a sentence it has to match on.
    pub fn code(&self) -> &'static str {
        match self {
            SelectError::Empty => "empty",
            SelectError::TooLong { .. } => "too_long",
            SelectError::TooManyTerms { .. } => "too_many_terms",
            SelectError::UnknownMethod { .. } => "unknown_method",
            SelectError::UnknownFlag { .. } => "unknown_flag",
            SelectError::EmptyTerm { .. } => "empty_term",
            SelectError::BadDepth { .. } => "bad_depth",
            SelectError::BadSource { .. } => "bad_source",
        }
    }

    /// Byte offset in the typed line, for a caret under the term at fault.
    pub fn pos(&self) -> Option<usize> {
        match self {
            SelectError::UnknownMethod { pos, .. }
            | SelectError::UnknownFlag { pos, .. }
            | SelectError::EmptyTerm { pos }
            | SelectError::BadDepth { pos, .. }
            | SelectError::BadSource { pos, .. } => Some(*pos),
            _ => None,
        }
    }
}

impl std::fmt::Display for SelectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectError::Empty => write!(f, "type a selector, for example my_model+"),
            SelectError::TooLong { len } => {
                write!(f, "the selector is {len} characters long, the limit is {MAX_LEN}")
            }
            SelectError::TooManyTerms { count } => {
                write!(f, "{count} terms, the limit is {MAX_TERMS}")
            }
            SelectError::UnknownMethod { method, .. } => {
                write!(f, "unknown selector method `{method}`. This build knows: {KNOWN}")
            }
            SelectError::UnknownFlag { flag, .. } => write!(
                f,
                "unknown option `{flag}`. Only --select and --exclude are understood here, \
                 after an optional dbt command"
            ),
            SelectError::EmptyTerm { .. } => write!(f, "a comma with no term beside it"),
            SelectError::BadDepth { term, .. } => {
                write!(f, "`{term}`: the depth beside a + must be a whole number")
            }
            SelectError::BadSource { value, .. } => write!(
                f,
                "`source:{value}`: expected source, source.table or package.source.table"
            ),
        }
    }
}

impl std::error::Error for SelectError {}

/* ------------------------------------------------------------- matching -- */

/// fnmatch with `*` and `?`, iterative so no pattern can grow the stack.
///
/// Two differences from Python's fnmatch, both deliberate: a `[seq]` class is
/// matched literally, and the comparison is case sensitive on every platform
/// where Python folds case on Windows. This binary's main target is Windows,
/// so that second one is a real difference, not a theoretical one.
fn glob(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    // `star` is the last `*` seen, `mark` how much of the text it has eaten.
    let (mut star, mut mark) = (usize::MAX, 0usize);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = pi;
            pi += 1;
            mark = ti;
        } else if star != usize::MAX {
            pi = star + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

fn has_glob(s: &str) -> bool {
    s.contains('*') || s.contains('?')
}

/// dbt's `SelectionCriteria.default_method`: a slash makes it a path, a source
/// extension makes it a file, everything else is an fqn.
///
/// Both separators count, where dbt only counts the backslash on Windows. A
/// node name with a backslash in it would be read as a path here; no dbt
/// project has one, and the binary does run on Windows.
fn default_method(value: &str) -> Method {
    let lower = value.to_lowercase();
    if value.contains('/') || value.contains('\\') {
        Method::Path
    } else if lower.ends_with(".sql") || lower.ends_with(".py") || lower.ends_with(".csv") {
        Method::File
    } else {
        Method::Fqn
    }
}

fn method_by_name(name: &str) -> Option<Method> {
    Some(match name {
        "fqn" => Method::Fqn,
        "path" => Method::Path,
        "file" => Method::File,
        "tag" => Method::Tag,
        "package" => Method::Package,
        "resource_type" => Method::ResourceType,
        "source" => Method::Source,
        "exposure" => Method::Exposure,
        "config.materialized" => Method::Config(ConfigKey::Materialized),
        "config.schema" => Method::Config(ConfigKey::Schema),
        "config.database" => Method::Config(ConfigKey::Database),
        "config.alias" => Method::Config(ConfigKey::Alias),
        "config.incremental_strategy" => Method::Config(ConfigKey::Strategy),
        "config.unique_key" => Method::Config(ConfigKey::UniqueKey),
        _ => return None,
    })
}

/// dbt's `is_selected_node`: the leaf on its own, or a dotted prefix of the
/// fqn, where the first part carrying a wildcard matches everything left.
///
/// The fqn is stored joined, so splitting it here is exactly dbt's "dots in
/// model names act as namespace separators" flattening, for free.
fn is_selected_node(fqn: &str, selector: &str) -> bool {
    let parts: Vec<&str> = fqn.split('.').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return false;
    }
    if parts[parts.len() - 1] == selector {
        return true;
    }
    let wanted: Vec<&str> = selector.split('.').collect();
    if parts.len() < wanted.len() {
        return false;
    }
    for (i, want) in wanted.iter().enumerate() {
        if has_glob(want) {
            return glob(&wanted[i..].join("."), &parts[i..].join("."));
        }
        if parts[i] != *want {
            return false;
        }
    }
    true
}

/// dbt's `QualifiedNameSelectorMethod`: the fqn as it stands, and again without
/// its package, so `staging.stg_customers` matches whatever package it sits in.
fn fqn_match(selector: &str, fqn: &str) -> bool {
    if is_selected_node(fqn, selector) {
        return true;
    }
    match fqn.split_once('.') {
        Some((_, unscoped)) => is_selected_node(unscoped, selector),
        None => false,
    }
}

/// Backslashes folded to slashes and any trailing slash dropped, so a selector
/// typed either way reaches a manifest written either way.
fn normalise(path: &str) -> String {
    let p = path.replace('\\', "/");
    p.trim_end_matches('/').to_string()
}

/// dbt globs the working directory and keeps a node whose file is in the
/// result or under a directory in it. We match the path the manifest recorded
/// instead: the same answer for every ordinary selector, and a different one
/// for a pattern that leans on what is actually on disk (0024).
fn path_match(selector: &str, file: &str) -> bool {
    let sel = normalise(selector);
    let f = normalise(file);
    if sel.is_empty() || f.is_empty() {
        return false;
    }
    if has_glob(&sel) {
        if glob(&sel, &f) {
            return true;
        }
        // A glob that names a directory takes everything under it.
        let mut cur = f.as_str();
        while let Some((parent, _)) = cur.rsplit_once('/') {
            if glob(&sel, parent) {
                return true;
            }
            cur = parent;
        }
        return false;
    }
    f == sel || f.starts_with(&format!("{sel}/"))
}

/// dbt matches the file name and, failing that, the name without its extension.
fn file_match(selector: &str, file: &str) -> bool {
    let f = normalise(file);
    let name = f.rsplit('/').next().unwrap_or(&f);
    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
    glob(selector, name) || glob(selector, stem)
}

fn kind_by_name(v: &str) -> Option<Kind> {
    Some(match v {
        "model" => Kind::Model,
        "source" => Kind::Source,
        "seed" => Kind::Seed,
        "snapshot" => Kind::Snapshot,
        "test" | "unit_test" => Kind::Test,
        "exposure" => Kind::Exposure,
        "analysis" => Kind::Analysis,
        "operation" => Kind::Operation,
        _ => return None,
    })
}

fn config_value<'a>(node: &'a Node, key: ConfigKey) -> &'a str {
    match key {
        ConfigKey::Materialized => &node.materialized,
        ConfigKey::Schema => &node.schema,
        ConfigKey::Database => &node.database,
        ConfigKey::Alias => &node.alias,
        ConfigKey::Strategy => &node.strategy,
        ConfigKey::UniqueKey => &node.unique_key,
    }
}

/// Does one node answer one term, the graph operators left out of it.
fn matches(term: &Term, node: &Node) -> bool {
    let v = term.value.as_str();
    match term.method {
        Method::Fqn => fqn_match(v, &node.fqn),
        Method::Path => path_match(v, &node.file),
        Method::File => file_match(v, &node.file),
        Method::Tag => node.tags.iter().any(|t| glob(v, t)),
        Method::Package => glob(v, &node.package),
        Method::ResourceType => kind_by_name(v) == Some(node.kind),
        Method::Config(key) => glob(v, config_value(node, key)),
        Method::Source => {
            if node.kind != Kind::Source {
                return false;
            }
            // A source's name is "<source>.<table>", built in Graph::build.
            let (source, table) = match node.name.split_once('.') {
                Some(pair) => pair,
                None => (node.name.as_str(), ""),
            };
            let parts: Vec<&str> = v.split('.').collect();
            let (want_package, want_source, want_table) = match parts.len() {
                1 => ("*", parts[0], "*"),
                2 => ("*", parts[0], parts[1]),
                3 => (parts[0], parts[1], parts[2]),
                _ => return false,
            };
            glob(want_package, &node.package) && glob(want_source, source) && glob(want_table, table)
        }
        Method::Exposure => {
            if node.kind != Kind::Exposure {
                return false;
            }
            match v.split_once('.') {
                Some((package, name)) => glob(package, &node.package) && glob(name, &node.name),
                None => glob(v, &node.name),
            }
        }
    }
}

/* -------------------------------------------------------------- parsing -- */

/// Whitespace splitting, with quotes dropped wherever they fall.
///
/// A quote is shell syntax: the shell would have removed it, and dbt splits
/// each `--select` value on whitespace anyway, so `-s "a b"` is the union of
/// `a` and `b` there and here alike. Each token carries the offset of its first
/// real character, which is what a caret in the box points at.
fn tokenize(line: &str) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut cur = String::new();
    let mut at = 0usize;
    for (i, c) in line.char_indices() {
        if c == '"' || c == '\'' {
            continue;
        }
        if c.is_whitespace() {
            if !cur.is_empty() {
                out.push((at, std::mem::take(&mut cur)));
            }
            continue;
        }
        if cur.is_empty() {
            at = i;
        }
        cur.push(c);
    }
    if !cur.is_empty() {
        out.push((at, cur));
    }
    out
}

/// dbt commands and the select flags, so a whole command line can be pasted.
const PREFIX: &[&str] = &[
    "dbt", "ls", "list", "run", "build", "test", "compile", "seed", "snapshot", "source",
    "freshness", "docs", "generate", "-s", "--select", "-m", "--models", "--model",
];

/// Eats a pasted `dbt ls -s` off the front. Returns whether anything went.
fn strip_command(tokens: &mut Vec<(usize, String)>) -> bool {
    let mut cut = 0usize;
    while cut < tokens.len() {
        let t = tokens[cut].1.as_str();
        if PREFIX.contains(&t) || t.starts_with("--select=") || t.starts_with("-s=") {
            cut += 1;
        } else {
            break;
        }
    }
    // A `--select=x` carries its value: put it back as a plain term.
    if cut > 0 {
        let last = tokens[cut - 1].1.clone();
        for head in ["--select=", "-s="] {
            if let Some(v) = last.strip_prefix(head) {
                if !v.is_empty() {
                    let at = tokens[cut - 1].0 + head.len();
                    tokens[cut - 1] = (at, v.to_string());
                    cut -= 1;
                }
                break;
            }
        }
    }
    tokens.drain(..cut);
    cut > 0
}

type Tokens = Vec<(usize, String)>;

/// Splits the include half from the exclude half. Any other option is an error
/// rather than a guess: silently eating a flag eats the selector beside it.
fn split_exclude(tokens: Tokens) -> Result<(Tokens, Tokens), SelectError> {
    let (mut include, mut exclude) = (Tokens::new(), Tokens::new());
    let mut excluding = false;
    for (at, tok) in tokens {
        if tok == "--exclude" || tok == "-e" {
            excluding = true;
            continue;
        }
        if let Some(v) = tok.strip_prefix("--exclude=") {
            excluding = true;
            if !v.is_empty() {
                exclude.push((at + "--exclude=".len(), v.to_string()));
            }
            continue;
        }
        if tok.starts_with('-') && tok.len() > 1 {
            return Err(SelectError::UnknownFlag { flag: tok, pos: at });
        }
        if excluding {
            exclude.push((at, tok));
        } else {
            include.push((at, tok));
        }
    }
    Ok((include, exclude))
}

fn parse_depth(digits: &str, raw: &str, pos: usize) -> Result<Depth, SelectError> {
    if digits.is_empty() {
        return Ok(None);
    }
    digits
        .parse::<u32>()
        .map(Some)
        .map_err(|_| SelectError::BadDepth { term: raw.to_string(), pos })
}

/// One term, in the order of dbt's `RAW_SELECTOR_PATTERN`: `@`, then `N+`, then
/// `method:`, then the value, then `+N`.
fn parse_term(raw: &str, pos: usize) -> Result<Term, SelectError> {
    let mut s = raw;
    let at = s.starts_with('@');
    if at {
        s = &s[1..];
    }

    // Digits are ASCII, so counting them gives a byte index either way.
    let mut parents = None;
    let lead = s.chars().take_while(|c| c.is_ascii_digit()).count();
    if s[lead..].starts_with('+') {
        parents = Some(parse_depth(&s[..lead], raw, pos)?);
        s = &s[lead + 1..];
    }

    let mut children = None;
    let trail = s.chars().rev().take_while(|c| c.is_ascii_digit()).count();
    let cut = s.len() - trail;
    if cut > 0 && s[..cut].ends_with('+') {
        children = Some(parse_depth(&s[cut..], raw, pos)?);
        s = &s[..cut - 1];
    }

    // Any colon at all means a method: dbt refuses an unknown one rather than
    // reading it as part of a name, and a quiet pass here would draw the whole
    // project for something like `state:modified`.
    let (method, value) = match s.split_once(':') {
        Some((name, rest)) => match method_by_name(name) {
            Some(m) => (m, rest),
            None => {
                return Err(SelectError::UnknownMethod { method: name.to_string(), pos });
            }
        },
        None => (default_method(s), s),
    };
    if value.is_empty() {
        return Err(SelectError::EmptyTerm { pos });
    }
    if method == Method::Source {
        let parts = value.split('.').count();
        if parts > 3 {
            return Err(SelectError::BadSource { value: value.to_string(), pos });
        }
    }
    Ok(Term {
        method,
        value: value.to_string(),
        at,
        parents,
        children,
        raw: raw.to_string(),
    })
}

fn parse_union(tokens: &Tokens) -> Result<(Union, String), SelectError> {
    let mut groups: Vec<Vec<Term>> = Vec::new();
    for (at, tok) in tokens {
        let mut group: Vec<Term> = Vec::new();
        let mut offset = 0usize;
        for piece in tok.split(',') {
            if piece.is_empty() {
                return Err(SelectError::EmptyTerm { pos: at + offset });
            }
            group.push(parse_term(piece, at + offset)?);
            offset += piece.len() + 1;
        }
        groups.push(group);
    }
    let text = tokens.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>().join(" ");
    Ok((Union(groups), text))
}

impl Expr {
    /// Parses one typed line. `extra_exclude` is the route's own `exclude`
    /// parameter, which joins whatever `--exclude` the line already carried.
    pub fn parse(line: &str, extra_exclude: &str) -> Result<Expr, SelectError> {
        if line.len() > MAX_LEN {
            return Err(SelectError::TooLong { len: line.len() });
        }
        if extra_exclude.len() > MAX_LEN {
            return Err(SelectError::TooLong { len: extra_exclude.len() });
        }
        let mut tokens = tokenize(line);
        let stripped = strip_command(&mut tokens);
        let (include, mut exclude) = split_exclude(tokens)?;
        if include.is_empty() {
            return Err(SelectError::Empty);
        }
        // The extra half is parsed on its own, so an offset from it can never
        // be mistaken for one in the typed line: it points at nothing there.
        let (extra, _) = split_exclude(tokenize(extra_exclude))?;
        exclude.extend(extra);

        let count = include.iter().chain(exclude.iter()).map(|(_, t)| t.split(',').count()).sum();
        if count > MAX_TERMS {
            return Err(SelectError::TooManyTerms { count });
        }

        let (include, select) = parse_union(&include)?;
        let (exclude, excluded) = parse_union(&exclude)?;
        Ok(Expr { include, exclude, select, excluded, stripped })
    }
}

/* ------------------------------------------------------------ resolving -- */

/// Walks parents or children from the seeds, marking what it reaches. Nodes
/// outside the universe are not entered and not crossed: a disabled ancestor
/// is a wall, because dbt's graph has no edge through one.
fn walk(graph: &Graph, seeds: &[u32], depth: Depth, up: bool, universe: &[bool], out: &mut [bool]) {
    let limit = depth.unwrap_or(u32::MAX);
    if limit == 0 {
        return;
    }
    let mut frontier: Vec<u32> = seeds.to_vec();
    let mut seen: Vec<bool> = vec![false; graph.nodes.len()];
    for step in 1..=limit {
        let mut next: Vec<u32> = Vec::new();
        for &cur in &frontier {
            let node = &graph.nodes[cur as usize];
            let side = if up { &node.parents } else { &node.children };
            for &nb in side {
                if !universe[nb as usize] || seen[nb as usize] {
                    continue;
                }
                seen[nb as usize] = true;
                out[nb as usize] = true;
                next.push(nb);
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
        if step == limit {
            break;
        }
    }
}

fn marked(mask: &[bool]) -> Vec<u32> {
    mask.iter().enumerate().filter(|(_, &m)| m).map(|(i, _)| i as u32).collect()
}

/// One term's own matches, then whatever its operators add around them.
fn expand(graph: &Graph, term: &Term, universe: &[bool]) -> Vec<bool> {
    let mut mask = vec![false; graph.nodes.len()];
    for (i, node) in graph.nodes.iter().enumerate() {
        if universe[i] && matches(term, node) {
            mask[i] = true;
        }
    }
    let direct = marked(&mask);
    if direct.is_empty() {
        return mask;
    }
    // `@` first, so the `+` depths below still count from the direct matches
    // and not from what `@` dragged in, which is how dbt combines them.
    if term.at {
        let mut down = vec![false; graph.nodes.len()];
        walk(graph, &direct, None, false, universe, &mut down);
        let mut seeds = direct.clone();
        seeds.extend(marked(&down));
        walk(graph, &seeds, None, true, universe, &mut mask);
        for (i, d) in down.iter().enumerate() {
            mask[i] |= d;
        }
    }
    if let Some(depth) = term.parents {
        walk(graph, &direct, depth, true, universe, &mut mask);
    }
    if let Some(depth) = term.children {
        walk(graph, &direct, depth, false, universe, &mut mask);
    }
    mask
}

/// Why a term found nothing, which is more use than dbt's flat warning.
fn why_empty(graph: &Graph, term: &Term, tests: Tests) -> String {
    let any = |pick: &dyn Fn(&Node) -> bool| graph.nodes.iter().any(|n| pick(n) && matches(term, n));
    if any(&|n: &Node| n.disabled) {
        return format!("`{}` matches only disabled nodes", term.raw);
    }
    if tests == Tests::Excluded && any(&|n: &Node| n.kind == Kind::Test && !n.disabled) {
        return format!("`{}` matches only tests; switch tests on to see them", term.raw);
    }
    format!("nothing matches `{}`", term.raw)
}

/// The union of intersections, with a warning for every term that found nothing.
fn evaluate(graph: &Graph, union: &Union, universe: &[bool], tests: Tests, warn: &mut Vec<String>) -> Vec<bool> {
    let mut out = vec![false; graph.nodes.len()];
    for group in &union.0 {
        let mut acc: Option<Vec<bool>> = None;
        for term in group {
            let mask = expand(graph, term, universe);
            if !mask.iter().any(|&m| m) {
                warn.push(why_empty(graph, term, tests));
            }
            // Intersecting the expanded sets, not the seeds: `+a,+b` is every
            // node feeding both, which is the whole point of writing it.
            acc = Some(match acc {
                None => mask,
                Some(mut a) => {
                    for (i, m) in mask.iter().enumerate() {
                        a[i] &= m;
                    }
                    a
                }
            });
        }
        if let Some(a) = acc {
            for (i, m) in a.iter().enumerate() {
                out[i] |= m;
            }
        }
    }
    out
}

/// Resolves a parsed expression against the graph.
pub fn resolve(graph: &Graph, expr: &Expr, tests: Tests) -> Resolved {
    let n = graph.nodes.len();
    // Hooks carry no edges and cannot be drawn, so they are not in the universe
    // at all; dbt would list them for `resource_type:operation` and we do not.
    let universe: Vec<bool> = graph
        .nodes
        .iter()
        .map(|node| {
            !node.disabled
                && node.kind != Kind::Operation
                && (node.kind != Kind::Test || tests == Tests::Eager)
        })
        .collect();

    let mut warnings: Vec<String> = Vec::new();
    let mut included = evaluate(graph, &expr.include, &universe, tests, &mut warnings);
    if !expr.exclude.0.is_empty() {
        let dropped = evaluate(graph, &expr.exclude, &universe, tests, &mut warnings);
        for (i, d) in dropped.iter().enumerate() {
            if *d {
                included[i] = false;
            }
        }
    }

    // Indirect selection, eager: a test joins when a parent survived. Graph
    // builds `Node.tests` on every parent, so that rule needs no extra walk.
    // dbt applies it per union component and then subtracts; doing it once at
    // the end differs only when one parent of a test is excluded and another
    // is not (0024).
    if tests == Tests::Eager {
        for i in 0..n {
            if !included[i] {
                continue;
            }
            for &t in &graph.nodes[i].tests {
                if universe[t as usize] {
                    included[t as usize] = true;
                }
            }
        }
    }

    warnings.dedup();
    Resolved { nodes: marked(&included), warnings }
}

/// The one call the route makes.
pub fn select(graph: &Graph, line: &str, exclude: &str, tests: Tests) -> Result<(Expr, Resolved), SelectError> {
    let expr = Expr::parse(line, exclude)?;
    let resolved = resolve(graph, &expr, tests);
    Ok((expr, resolved))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;

    /// Two packages, a source feeding a staging model, a mart that two models
    /// feed, an exposure on the end, a seed, a snapshot, one test and one
    /// disabled model. Small enough to assert whole sets against.
    fn graph() -> Graph {
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "nodes": {
                "model.shop.stg_customers": {
                    "name": "stg_customers", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/staging/crm/stg_customers.sql",
                    "fqn": ["shop", "staging", "crm", "stg_customers"],
                    "tags": ["nightly"], "schema": "staging",
                    "config": { "materialized": "view" },
                },
                "model.shop.stg_orders": {
                    "name": "stg_orders", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/staging/crm/stg_orders.sql",
                    "fqn": ["shop", "staging", "crm", "stg_orders"],
                    "config": { "materialized": "view" },
                },
                "model.shop.dim_customers": {
                    "name": "dim_customers", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/marts/dim_customers.sql",
                    "fqn": ["shop", "marts", "dim_customers"],
                    "config": { "materialized": "table" },
                },
                "model.shop.fct_orders": {
                    "name": "fct_orders", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/marts/fct_orders.sql",
                    "fqn": ["shop", "marts", "fct_orders"],
                    "config": { "materialized": "incremental", "incremental_strategy": "merge" },
                },
                "model.other.helper": {
                    "name": "helper", "resource_type": "model", "package_name": "other",
                    "original_file_path": "models/staging/helper.sql",
                    "fqn": ["other", "staging", "helper"],
                    "config": { "materialized": "view" },
                },
                "seed.shop.country_codes": {
                    "name": "country_codes", "resource_type": "seed", "package_name": "shop",
                    "original_file_path": "seeds/country_codes.csv",
                    "fqn": ["shop", "country_codes"],
                },
                "snapshot.shop.orders_snap": {
                    "name": "orders_snap", "resource_type": "snapshot", "package_name": "shop",
                    "original_file_path": "snapshots/orders_snap.sql",
                    "fqn": ["shop", "orders_snap"],
                },
                "test.shop.not_null_dim_customers_id": {
                    "name": "not_null_dim_customers_id", "resource_type": "test",
                    "package_name": "shop", "tags": ["nightly"],
                    "original_file_path": "models/marts/schema.yml",
                    "fqn": ["shop", "marts", "not_null_dim_customers_id"],
                    "column_name": "customer_id",
                    "attached_node": "model.shop.dim_customers",
                    "test_metadata": { "name": "not_null" },
                },
            },
            "sources": {
                "source.shop.crm.customers": {
                    "name": "customers", "resource_type": "source", "package_name": "shop",
                    "source_name": "crm", "identifier": "customers",
                    "original_file_path": "models/staging/crm/sources.yml",
                    "fqn": ["shop", "crm", "customers"],
                },
            },
            "exposures": {
                "exposure.shop.weekly_report": {
                    "name": "weekly_report", "resource_type": "exposure", "package_name": "shop",
                    "original_file_path": "models/exposures.yml",
                    "fqn": ["shop", "weekly_report"],
                },
            },
            "disabled": {
                "model.shop.legacy": [{
                    "name": "legacy", "resource_type": "model", "package_name": "shop",
                    "original_file_path": "models/marts/legacy.sql",
                    "fqn": ["shop", "marts", "legacy"], "tags": ["nightly"],
                    "config": { "enabled": false, "materialized": "table" },
                }],
            },
            "parent_map": {
                "model.shop.stg_customers": ["source.shop.crm.customers"],
                "model.shop.dim_customers": ["model.shop.stg_customers"],
                "model.shop.fct_orders": ["model.shop.stg_orders", "model.shop.dim_customers"],
                "test.shop.not_null_dim_customers_id": ["model.shop.dim_customers"],
                "exposure.shop.weekly_report": ["model.shop.fct_orders"],
            },
        }))
        .unwrap();
        Graph::build(raw, std::path::Path::new("manifest.json"), 0, 0)
    }

    /// The names a selector yields, sorted, so a test reads as a set.
    fn pick_with(line: &str, exclude: &str, tests: Tests) -> Vec<String> {
        let g = graph();
        let (_, res) = select(&g, line, exclude, tests).expect(line);
        let mut out: Vec<String> = res.nodes.iter().map(|&i| g.nodes[i as usize].name.clone()).collect();
        out.sort();
        out
    }

    fn pick(line: &str) -> Vec<String> {
        pick_with(line, "", Tests::Excluded)
    }

    fn warnings(line: &str, tests: Tests) -> Vec<String> {
        let g = graph();
        select(&g, line, "", tests).expect(line).1.warnings
    }

    fn err(line: &str) -> SelectError {
        Expr::parse(line, "").expect_err(line)
    }

    #[test]
    fn glob_matches_the_way_fnmatch_does() {
        assert!(glob("*", "anything"));
        assert!(glob("stg_*", "stg_orders"));
        assert!(glob("*_orders", "stg_orders"));
        assert!(glob("stg_?rders", "stg_orders"));
        assert!(!glob("stg_?rders", "stg_oorders"));
        // The case a naive matcher gets wrong: a star has to give text back.
        assert!(glob("a*a*b", "aaaab"));
        assert!(!glob("a*a*c", "aaaab"));
        assert!(glob("", ""));
        assert!(!glob("", "x"));
        assert!(!glob("toolong", "sh"));
        // A character class is literal here, where Python's fnmatch reads it.
        assert!(!glob("stg_[oc]*", "stg_orders"));
    }

    #[test]
    fn the_default_method_follows_dbts_inference() {
        for (value, want) in [
            ("stg_orders", Method::Fqn),
            ("shop.staging.stg_orders", Method::Fqn),
            ("models/staging", Method::Path),
            ("models\\staging", Method::Path),
            ("stg_orders.sql", Method::File),
            ("loader.PY", Method::File),
            ("countries.csv", Method::File),
        ] {
            assert_eq!(default_method(value), want, "{value}");
        }
    }

    #[test]
    fn an_fqn_matches_by_leaf_by_prefix_and_without_its_package() {
        let fqn = "shop.staging.crm.stg_customers";
        assert!(fqn_match("stg_customers", fqn), "the leaf on its own");
        assert!(fqn_match("shop.staging.crm.stg_customers", fqn), "the whole thing");
        assert!(fqn_match("shop.staging", fqn), "a prefix takes what is under it");
        assert!(fqn_match("shop.staging.*", fqn));
        assert!(fqn_match("*.staging.*", fqn));
        // dbt tries the fqn again without its package, so a path that skips the
        // package still matches. Dropping this would fail selectors that work.
        assert!(fqn_match("staging.crm.stg_customers", fqn), "unscoped by package");
        assert!(!fqn_match("crm", fqn), "a middle part alone is not a prefix");
        assert!(!fqn_match("stg_customer", fqn));
    }

    #[test]
    fn a_term_carries_its_graph_operators() {
        let t = |s: &str| parse_term(s, 0).unwrap();
        let bare = t("dim_customers");
        assert_eq!((bare.at, bare.parents, bare.children), (false, None, None));
        assert_eq!(t("+dim_customers").parents, Some(None));
        assert_eq!(t("2+dim_customers").parents, Some(Some(2)));
        assert_eq!(t("dim_customers+").children, Some(None));
        assert_eq!(t("dim_customers+3").children, Some(Some(3)));
        let both = t("+dim_customers+");
        assert_eq!((both.parents, both.children), (Some(None), Some(None)));
        let at = t("@dim_customers+");
        assert_eq!((at.at, at.children), (true, Some(None)));
        let deep = t("3+tag:nightly+2");
        assert_eq!((deep.method, deep.value.as_str()), (Method::Tag, "nightly"));
        assert_eq!((deep.parents, deep.children), (Some(Some(3)), Some(Some(2))));
        // A plus in the middle is part of the name, and finds nothing later.
        assert_eq!(t("dim+customers").value, "dim+customers");
    }

    #[test]
    fn space_is_union_and_comma_is_intersection() {
        assert_eq!(pick("stg_orders dim_customers"), ["dim_customers", "stg_orders"]);
        assert_eq!(pick("tag:nightly,config.materialized:view"), ["stg_customers"]);
        // (a ∩ b) ∪ c, evaluated per whitespace group.
        assert_eq!(
            pick("tag:nightly,config.materialized:view stg_orders"),
            ["stg_customers", "stg_orders"]
        );
        assert!(pick("tag:nightly,config.materialized:table").is_empty());
    }

    #[test]
    fn plus_walks_the_graph_to_the_depth_it_was_given() {
        assert_eq!(pick("dim_customers"), ["dim_customers"]);
        assert_eq!(
            pick("+dim_customers"),
            ["crm.customers", "dim_customers", "stg_customers"]
        );
        assert_eq!(pick("1+dim_customers"), ["dim_customers", "stg_customers"]);
        assert_eq!(pick("dim_customers+"), ["dim_customers", "fct_orders", "weekly_report"]);
        assert_eq!(pick("dim_customers+1"), ["dim_customers", "fct_orders"]);
        // A depth of zero is dbt's way of writing "no expansion at all".
        assert_eq!(pick("0+dim_customers"), ["dim_customers"]);
    }

    #[test]
    fn at_takes_the_children_and_every_parent_of_those() {
        assert_eq!(
            pick("@dim_customers"),
            [
                "crm.customers",
                "dim_customers",
                "fct_orders",
                "stg_customers",
                "stg_orders",
                "weekly_report"
            ]
        );
    }

    #[test]
    fn an_intersection_meets_on_the_expanded_sets_not_the_seeds() {
        // Everything feeding both marts, which is why anyone writes this.
        // Intersecting the seeds instead would answer nothing at all.
        assert_eq!(
            pick("+dim_customers,+fct_orders"),
            ["crm.customers", "dim_customers", "stg_customers"]
        );
    }

    #[test]
    fn a_disabled_node_is_never_returned_and_never_crossed() {
        assert!(pick("legacy").is_empty());
        assert!(pick("resource_type:model").iter().all(|n| n != "legacy"));
        assert!(pick("tag:nightly").iter().all(|n| n != "legacy"));
    }

    #[test]
    fn tests_take_part_only_when_they_are_switched_on() {
        assert_eq!(pick("tag:nightly"), ["stg_customers"]);
        assert_eq!(
            pick_with("tag:nightly", "", Tests::Eager),
            ["not_null_dim_customers_id", "stg_customers"]
        );
        // Eager indirect selection: the test joins because its parent did.
        assert_eq!(
            pick_with("dim_customers", "", Tests::Eager),
            ["dim_customers", "not_null_dim_customers_id"]
        );
        assert_eq!(pick("dim_customers"), ["dim_customers"]);
    }

    #[test]
    fn exclude_subtracts_from_whichever_half_asked_for_it() {
        assert_eq!(pick("+fct_orders --exclude stg_orders"), {
            let mut v = pick("+fct_orders");
            v.retain(|n| n != "stg_orders");
            v
        });
        // The route's own parameter joins whatever the line already carried.
        assert_eq!(
            pick_with("stg_customers stg_orders", "stg_orders", Tests::Excluded),
            ["stg_customers"]
        );
        assert_eq!(pick("resource_type:model --exclude package:shop"), ["helper"]);
    }

    #[test]
    fn a_term_that_finds_nothing_warns_and_says_why() {
        assert_eq!(warnings("no_such_model", Tests::Excluded), ["nothing matches `no_such_model`"]);
        assert_eq!(
            warnings("legacy", Tests::Excluded),
            ["`legacy` matches only disabled nodes"]
        );
        assert_eq!(
            warnings("not_null_dim_customers_id", Tests::Excluded),
            ["`not_null_dim_customers_id` matches only tests; switch tests on to see them"]
        );
        // The rest of the expression still resolves around the bad term.
        assert_eq!(pick("no_such_model dim_customers"), ["dim_customers"]);
        assert!(warnings("dim_customers", Tests::Excluded).is_empty());
    }

    #[test]
    fn every_method_matches_what_it_says() {
        // The source is in that directory too, in its sources.yml, and dbt
        // selects it for the same reason: the path method knows no kinds.
        assert_eq!(pick("path:models/staging/crm"), ["crm.customers", "stg_customers", "stg_orders"]);
        assert_eq!(pick("path:models/marts/dim_customers.sql"), ["dim_customers"]);
        assert_eq!(pick("path:models/marts/*"), ["dim_customers", "fct_orders"]);
        assert_eq!(pick("file:stg_orders.sql"), ["stg_orders"]);
        assert_eq!(pick("file:stg_orders"), ["stg_orders"], "the stem matches too");
        assert_eq!(pick("package:other"), ["helper"]);
        assert_eq!(pick("resource_type:seed"), ["country_codes"]);
        assert_eq!(pick("resource_type:snapshot"), ["orders_snap"]);
        assert_eq!(pick("config.materialized:incremental"), ["fct_orders"]);
        assert_eq!(pick("config.incremental_strategy:merge"), ["fct_orders"]);
        assert_eq!(pick("exposure:weekly_report"), ["weekly_report"]);
        assert_eq!(pick("source:crm"), ["crm.customers"]);
        assert_eq!(pick("source:crm.customers"), ["crm.customers"]);
        assert_eq!(pick("source:shop.crm.customers"), ["crm.customers"]);
        assert_eq!(pick("source:crm.*"), ["crm.customers"]);
        assert!(pick("source:other.crm.customers").is_empty());
        // An unknown resource type finds nothing rather than everything.
        assert!(pick("resource_type:macro").is_empty());
    }

    #[test]
    fn a_pasted_dbt_command_is_understood() {
        let expr = Expr::parse("dbt ls -s \"stg_customers dim_customers\" --exclude tag:nightly", "").unwrap();
        assert!(expr.stripped);
        assert_eq!(expr.select, "stg_customers dim_customers");
        assert_eq!(expr.excluded, "tag:nightly");
        assert_eq!(
            pick("dbt ls -s \"stg_customers dim_customers\" --exclude tag:nightly"),
            ["dim_customers"]
        );
        assert_eq!(Expr::parse("--select stg_orders", "").unwrap().select, "stg_orders");
        assert_eq!(Expr::parse("--select=stg_orders", "").unwrap().select, "stg_orders");
        assert_eq!(Expr::parse("dbt build --select stg_orders+", "").unwrap().select, "stg_orders+");
        // A plain expression is left exactly as it was typed.
        let plain = Expr::parse("stg_orders+", "").unwrap();
        assert!(!plain.stripped);
        assert_eq!(plain.select, "stg_orders+");
    }

    #[test]
    fn errors_are_named_and_placed() {
        assert_eq!(err(""), SelectError::Empty);
        assert_eq!(err("   "), SelectError::Empty);
        assert!(matches!(err("state:modified"), SelectError::UnknownMethod { .. }));
        assert_eq!(err("state:modified").pos(), Some(0));
        assert_eq!(err("dim_customers foo:bar").pos(), Some(14), "the caret sits on the bad term");
        assert!(matches!(err("a,,b"), SelectError::EmptyTerm { .. }));
        assert_eq!(err("a,,b").pos(), Some(2));
        assert!(matches!(err("+"), SelectError::EmptyTerm { .. }));
        assert!(matches!(err("--wat a"), SelectError::UnknownFlag { .. }));
        assert!(matches!(err("99999999999+a"), SelectError::BadDepth { .. }));
        assert!(matches!(err("source:a.b.c.d"), SelectError::BadSource { .. }));
        assert!(matches!(err(&"x".repeat(MAX_LEN + 1)), SelectError::TooLong { .. }));
        let many = (0..MAX_TERMS + 1).map(|i| format!("m{i}")).collect::<Vec<_>>().join(" ");
        assert!(matches!(err(&many), SelectError::TooManyTerms { .. }));
        // Every error has a token the frontend can branch on without parsing prose.
        assert_eq!(err("state:modified").code(), "unknown_method");
    }

    #[test]
    fn nothing_a_person_can_type_panics() {
        let g = graph();
        for line in [
            "@@@", "+++", "é", "tag:é*", "@", "----", "a,", ",a", ":", "x:", "a b,c+2 @d+",
            "models/../../etc/passwd", "\"", "'''", "*", "**", "?", "source:", "config.:x",
            &"a".repeat(MAX_LEN),
            &"+".repeat(200),
        ] {
            // Either answer is fine, a panic is not.
            let _ = select(&g, line, "tag:nightly", Tests::Eager);
        }
    }

    #[test]
    fn a_selector_reaches_everything_the_universe_holds() {
        // `*` is the cheapest proof that the universe is what it claims: every
        // enabled node but the hooks and, here, the tests.
        assert_eq!(
            pick("*"),
            [
                "country_codes",
                "crm.customers",
                "dim_customers",
                "fct_orders",
                "helper",
                "orders_snap",
                "stg_customers",
                "stg_orders",
                "weekly_report"
            ]
        );
    }
}
