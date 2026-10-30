//! Macros, as the manifest lists them, and which one a call in the editor
//! reaches.
//!
//! A call is resolved by name, the way dbt resolves it for a node: `pkg.name`
//! is that package's macro, and a bare `name` is the calling package's own,
//! then the root project's. The order matters in practice, because a project
//! can define a macro under a package macro's name: a bare `hub` then reaches
//! the project's own, while `automate_dv.hub` still reaches the package's. An
//! installed package is never searched for a bare name, since dbt never
//! searches one either, and dbt's own macros come last in dbt's order but have
//! no file here, so they need no place in this one.
//!
//! Each macro keeps its body only when it bears on an incremental strategy,
//! which `src/incremental.rs` reads; nothing here looks at one.
//!
//! The invariant: a macro is linked only to a file that exists inside the
//! project. The manifest gives a package macro's path relative to the package,
//! so it is looked for where `dbt deps` puts packages, and a macro of dbt's own,
//! which lives in site-packages, is never found and stays plain text. A custom
//! `packages-install-path` is the known gap.

use crate::manifest::RawMacro;
use std::collections::HashMap;
use std::path::Path;

/// Where `dbt deps` installs a package, in a folder named after it: the name
/// dbt has used since 1.0, then the one before it.
pub(crate) const PACKAGE_DIRS: &[&str] = &["dbt_packages", "dbt_modules"];

/// How many macros a search may fail to place for each one it can list.
const TRIES_PER_HIT: usize = 4;

#[derive(Clone, Debug)]
pub struct Macro {
    pub id: String,
    pub name: String,
    pub package: String,
    /// `original_file_path`, slashed: relative to the project for the root
    /// project's macros, relative to the package for an installed one's.
    pub path: String,
    pub description: String,
    /// The arguments the YAML documents, which is all the manifest knows.
    pub args: Vec<String>,
    /// `macro_sql`, kept only for a macro that bears on an incremental
    /// strategy (`crate::manifest`), empty for every other.
    pub body: String,
}

#[derive(Clone, Default)]
pub struct Macros {
    list: Vec<Macro>,
    /// Package, then macro name, to a position in `list`. dbt refuses two
    /// macros of one name in one package, so the pair is a key.
    by_package: HashMap<String, HashMap<String, u32>>,
    /// The root project's name, which is the package its macros carry.
    root: String,
}

impl Macros {
    pub fn build(raw: HashMap<String, RawMacro>, root: &str) -> Macros {
        let mut list = Vec::with_capacity(raw.len());
        let mut by_package: HashMap<String, HashMap<String, u32>> = HashMap::new();
        for (id, m) in raw {
            let (Some(name), Some(package)) = (m.name, m.package_name) else { continue };
            if name.is_empty() {
                continue;
            }
            let slot = by_package.entry(package.clone()).or_default();
            if slot.contains_key(&name) {
                continue;
            }
            slot.insert(name.clone(), list.len() as u32);
            list.push(Macro {
                id,
                name,
                package,
                path: crate::graph::slashed(m.original_file_path.unwrap_or_default()),
                description: m.description.unwrap_or_default(),
                args: m
                    .arguments
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|a| a.name)
                    .filter(|a| !a.is_empty())
                    .collect(),
                body: m.macro_sql.unwrap_or_default(),
            });
        }
        Macros { list, by_package, root: root.to_string() }
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// How many macros one package defines: the project's own, among the
    /// hundreds dbt and its adapter bring, is what an overview tells apart.
    pub fn count_of(&self, package: &str) -> usize {
        self.by_package.get(package).map_or(0, HashMap::len)
    }

    /// Whether the manifest named the root project. dbt-core wrote the name
    /// from 1.6 on; before, `load_base` takes it from `dbt_project.yml`.
    pub fn has_root(&self) -> bool {
        !self.root.is_empty()
    }

    pub fn set_root(&mut self, name: String) {
        self.root = name;
    }

    /// The root project's name, or empty when neither the manifest nor
    /// `dbt_project.yml` gave one.
    pub fn root(&self) -> &str {
        &self.root
    }

    /// Where the macro of unique_id `id`, `macro.<package>.<name>`, sits in
    /// the list, which `at` reads back.
    pub(crate) fn position(&self, id: &str) -> Option<u32> {
        let (package, name) = id.strip_prefix("macro.")?.split_once('.')?;
        self.by_package.get(package)?.get(name).copied()
    }

    pub(crate) fn at(&self, i: u32) -> &Macro {
        &self.list[i as usize]
    }

    pub(crate) fn get(&self, package: &str, name: &str) -> Option<&Macro> {
        let &i = self.by_package.get(package)?.get(name)?;
        Some(&self.list[i as usize])
    }

    /// The macro `call` reaches from a file of package `local`. Anything with
    /// more than one dot is an attribute of something, never a macro.
    pub fn resolve(&self, call: &str, local: &str) -> Option<&Macro> {
        match call.split_once('.') {
            Some((package, name)) if !name.contains('.') => self.get(package, name),
            Some(_) => None,
            None => self.get(local, call).or_else(|| self.get(&self.root, call)),
        }
    }

    /// The macro a name stands for when it is asked for on its own, outside
    /// any file (`--describe`): a unique_id, `package.name` for that package's,
    /// and a bare name the way dbt resolves a call from the root project, its
    /// own macros first, then the adapter's, then dbt's. A package's macro is
    /// never found by a bare name, since dbt does not find it either; the
    /// caller offers `package.name` instead. Case counts, as in a call.
    pub(crate) fn lookup(&self, name: &str, adapter: &str) -> Option<u32> {
        if let Some(at) = self.position(name) {
            return Some(at);
        }
        let in_package = |package: &str, name: &str| self.by_package.get(package)?.get(name).copied();
        match name.split_once('.') {
            Some((package, name)) if !name.contains('.') => in_package(package, name),
            Some(_) => None,
            None => std::iter::once(self.root.clone())
                .chain(crate::macro_use::internal_packages(adapter))
                .find_map(|package| in_package(&package, name)),
        }
    }

    /// The package a project file belongs to: the installed one whose folder
    /// it sits in, or the root project.
    pub fn package_of<'a>(&'a self, file: &'a str) -> &'a str {
        let mut parts = file.split(['/', '\\']).filter(|p| !p.is_empty() && *p != ".");
        match (parts.next(), parts.next(), parts.next()) {
            (Some(dir), Some(package), Some(_)) if PACKAGE_DIRS.contains(&dir) => package,
            _ => &self.root,
        }
    }

    /// Where `m` is written inside the project, or None when it is not there:
    /// a macro of dbt's own, a package not installed, or a file the manifest
    /// remembers and a later commit moved.
    pub fn place(&self, project: &Path, m: &Macro) -> Option<String> {
        self.place_under(project, &project.canonicalize().ok()?, m)
    }

    /// `place`, with the project canonicalized once by a caller placing many.
    fn place_under(&self, project: &Path, canon: &Path, m: &Macro) -> Option<String> {
        if m.path.is_empty() {
            return None;
        }
        let candidates: Vec<String> = if m.package == self.root {
            vec![m.path.clone()]
        } else {
            PACKAGE_DIRS.iter().map(|dir| format!("{dir}/{}/{}", m.package, m.path)).collect()
        };
        // Through files::resolve, so a macro path climbing out with `..`, or a
        // package symlinked in from elsewhere, is refused like any other path.
        candidates
            .into_iter()
            .find(|rel| crate::files::resolve_under(project, canon, rel).is_ok_and(|p| p.is_file()))
    }

    /// Every macro of that name, whatever its package.
    pub(crate) fn named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Macro> + 'a {
        self.by_package.values().filter_map(move |names| names.get(name)).map(|&i| &self.list[i as usize])
    }

    /// Every package that defines a macro, in no particular order.
    pub(crate) fn packages(&self) -> impl Iterator<Item = &str> {
        self.by_package.keys().map(String::as_str)
    }

    pub(crate) fn has(&self, package: &str, name: &str) -> bool {
        self.get(package, name).is_some()
    }

    /// Every macro whose name holds `query`, best first: the name itself, then
    /// a name that starts with it, then one that holds it, the project's before
    /// a package's at each step and shorter names first. Ranked by step before
    /// package, unlike the file search, so `star` finds `dbt_utils.star` above
    /// a project macro that merely contains the word. A query with a dot is
    /// matched against `package.name`, the way a call names a package's macro.
    pub fn search(&self, query: &str) -> Vec<&Macro> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<(u8, bool, usize, &Macro)> = Vec::new();
        for m in &self.list {
            let key = if q.contains('.') { format!("{}.{}", m.package, m.name) } else { m.name.clone() }.to_lowercase();
            let step = if key == q {
                0
            } else if key.starts_with(&q) {
                1
            } else if key.contains(&q) {
                2
            } else {
                continue;
            };
            hits.push((step, m.package != self.root, m.name.len(), m));
        }
        // The pair closes every tie, so the order is the same on every start (0055).
        hits.sort_unstable_by(|a, b| {
            (a.0, a.1, a.2, &a.3.name, &a.3.package).cmp(&(b.0, b.1, b.2, &b.3.name, &b.3.package))
        });
        hits.into_iter().map(|h| h.3).collect()
    }

    /// The first `limit` macros `search` finds that are written inside the
    /// project, each with its file, since nothing else can be opened. A
    /// package with no folder where `dbt deps` puts one is passed over whole,
    /// after one look for that folder: dbt's own few hundred macros, and an
    /// adapter's, then cost that look and not one per macro. The rest are
    /// looked for until `TRIES_PER_HIT` times `limit` of them were not found: a
    /// manifest older than a move of `macros/` names files that are gone, and
    /// a query of one letter would otherwise look for every one on each key.
    pub fn find(&self, project: &Path, query: &str, limit: usize) -> Vec<(&Macro, String)> {
        let Ok(canon) = project.canonicalize() else { return Vec::new() };
        let mut installed: HashMap<&str, bool> = HashMap::new();
        let mut misses = limit.saturating_mul(TRIES_PER_HIT);
        let mut found = Vec::new();
        for m in self.search(query) {
            if found.len() >= limit || misses == 0 {
                break;
            }
            if m.package != self.root
                && !*installed.entry(&m.package).or_insert_with(|| {
                    PACKAGE_DIRS.iter().any(|dir| {
                        crate::files::resolve_under(project, &canon, &format!("{dir}/{}", m.package))
                            .is_ok_and(|p| p.is_dir())
                    })
                })
            {
                continue;
            }
            match self.place_under(project, &canon, m) {
                Some(file) => found.push((m, file)),
                None => misses -= 1,
            }
        }
        found
    }
}

/// The lines a macro's definition spans in its file, counted from 1 and both
/// ends included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Block {
    pub line: usize,
    pub end: usize,
}

/// Where the macro `name` is written in `text`: from its `{% macro name(` to
/// the `{% endmacro %}` closing it. A generic test and a materialization are
/// macros too, named from their own blocks, `{% test x(` being `test_x` and
/// `{% materialization x, adapter='y' %}` being `materialization_x_y`
/// (`_default` without an adapter). Comments are masked first, since a
/// commented-out copy is not the macro dbt reads, and so are raw blocks and
/// the strings of `{{ }}`, whose `{%` is text. A block of the same keyword
/// inside it is counted, so its end is not taken for the outer one's. A block
/// never closed runs to the end of the file, which dbt would not have parsed,
/// so only a stale manifest meets one.
///
/// The editor finds the same start with `macroDefLine` in `web/app.js` and
/// masks comments with `maskJinjaComments` there: a change to what names a
/// macro, or to what a comment is, wants the other changed with it. The
/// editor's still reads a `{%` inside a raw block or a string as a start.
pub(crate) fn block_in(text: &str, name: &str) -> Option<Block> {
    located(text, name).map(|(block, _)| block)
}

/// The names of the arguments in the tag that opens the macro `name`, in the
/// order written and without their defaults: `amount, scale` for
/// `{% macro cents(amount, scale=2) %}`. Empty for a materialization, whose
/// tag has no parentheses and which dbt gives no arguments; None for a macro
/// `block_in` does not find.
pub(crate) fn args_in(text: &str, name: &str) -> Option<Vec<String>> {
    let masked = mask_comments(text);
    let (_, open) = located(text, name)?;
    // A materialization takes no arguments: dbt gives it an empty list.
    let Some(open) = open else { return Some(Vec::new()) };
    let inner = &masked[open..];
    let mut names = Vec::new();
    let (mut start, mut quote, mut depth, mut escaped) = (0, 0u8, 0i32, false);
    let mut push = |arg: &str| {
        // A default is written after an `=`; the name is what stands before it.
        let name = arg.split('=').next().unwrap_or("").trim();
        if !name.is_empty() {
            names.push(name.to_string());
        }
    };
    for (i, c) in inner.bytes().enumerate() {
        if quote != 0 {
            // A backslash escapes the byte after it, a quote included.
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == quote {
                quote = 0;
            }
            continue;
        }
        match c {
            b'"' | b'\'' => quote = c,
            b'(' | b'[' | b'{' => depth += 1,
            b']' | b'}' => depth -= 1,
            // The parenthesis that closes the tag's own, outside any default's.
            b')' if depth == 0 => {
                push(&inner[start..i]);
                return Some(names);
            }
            b')' => depth -= 1,
            b',' if depth == 0 => {
                push(&inner[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    Some(names)
}

/// The block of `name`, and the offset just past the `(` of its tag in the
/// masked text, when it has one.
fn located(text: &str, name: &str) -> Option<(Block, Option<usize>)> {
    let masked = mask_comments(text);
    let starts = statement_starts(&masked);
    for (n, &start) in starts.iter().enumerate() {
        let (keyword, after) = tag_at(&masked, start);
        let (opener, closer, paren) = match keyword {
            "macro" if opens(after, name) => ("macro", "endmacro", opened_at(&masked, after, name)),
            "test" => match name.strip_prefix("test_").filter(|test| opens(after, test)) {
                Some(test) => ("test", "endtest", opened_at(&masked, after, test)),
                None => continue,
            },
            "materialization" if materialization_name(after) == name => ("materialization", "endmaterialization", None),
            _ => continue,
        };
        let (mut end, mut depth) = (masked.trim_end().len(), 1);
        for &tag in &starts[n + 1..] {
            match tag_at(&masked, tag).0 {
                k if k == opener => depth += 1,
                k if k == closer => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                end = tag_end(&masked, tag);
                break;
            }
        }
        let lines = |upto: usize| masked[..upto].matches('\n').count() + 1;
        return Some((Block { line: lines(start), end: lines(end.max(start)) }, paren));
    }
    None
}

/// The offset in `masked` just past the `(` after `name` in `after`, which is
/// a suffix of `masked`.
fn opened_at(masked: &str, after: &str, name: &str) -> Option<usize> {
    let rest = after.trim_start().strip_prefix(name)?.trim_start().strip_prefix('(')?;
    Some(masked.len() - rest.len())
}

/// Where each `{%` that Jinja's lexer reads as a tag opens, in text whose
/// comments are masked: not inside a `{{ }}` block, whose strings may hold one,
/// nor in a raw block.
fn statement_starts(text: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut i = 0;
    while let Some(found) = text[i..].find('{') {
        let open = i + found;
        i = open + 1;
        match text.as_bytes().get(open + 1) {
            Some(b'%') => {
                starts.push(open);
                i = if tag_at(text, open).0 == "raw" && closes_tag(tag_at(text, open).1) {
                    text[open..]
                        .match_indices("{%")
                        .map(|(at, _)| open + at)
                        .find(|&tag| tag_at(text, tag).0 == "endraw")
                        .map_or(text.len(), |tag| tag_end(text, tag))
                } else {
                    tag_end(text, open)
                };
            }
            Some(b'{') => i = block_end(text, open + 2, b'}').saturating_add(2).min(text.len()),
            _ => {}
        }
    }
    starts
}

/// `text` with every `{# #}` comment blanked, newlines and byte offsets kept.
/// Only where Jinja's lexer opens one, in the text between blocks: a `{#`
/// inside a string of a `{{ }}` or `{% %}` block, or in a raw block, is text.
fn mask_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let (mut at, mut i) = (0, 0);
    while let Some(found) = text[i..].find('{') {
        let open = i + found;
        i = open + 1;
        match text.as_bytes().get(open + 1) {
            Some(b'#') => {
                let Some(close) = text[open + 2..].find("#}") else { break };
                let end = open + 2 + close + 2;
                out.push_str(&text[at..open]);
                out.extend(text[open..end].chars().map(|c| if c == '\n' { "\n".to_string() } else { " ".repeat(c.len_utf8()) }));
                (at, i) = (end, end);
            }
            Some(b'%') if tag_at(text, open).0 == "raw" && closes_tag(tag_at(text, open).1) => {
                i = text[open..]
                    .match_indices("{%")
                    .map(|(at, _)| open + at)
                    .find(|&tag| tag_at(text, tag).0 == "endraw")
                    .map_or(text.len(), |tag| tag_end(text, tag));
            }
            Some(b'%') => i = tag_end(text, open),
            Some(b'{') => i = block_end(text, open + 2, b'}').saturating_add(2).min(text.len()),
            _ => {}
        }
    }
    out.push_str(&text[at..]);
    out
}

/// Whether `after`, what follows a tag's keyword, closes the tag.
fn closes_tag(after: &str) -> bool {
    let after = after.trim_start();
    after.strip_prefix(['-', '+']).unwrap_or(after).starts_with("%}")
}

/// The offset just past the `%}` of the tag opening at `start`.
fn tag_end(text: &str, start: usize) -> usize {
    block_end(text, start + 2, b'%').saturating_add(2).min(text.len())
}

/// Where a block opened before `from` closes: the first `}` after `close`
/// outside a string with every bracket shut, since Jinja's lexer reads the
/// `}}` ending `{'a': {'b': 1}}` as two braces of the dict. A block that never
/// balances ends at its first close, or at the end of the text.
fn block_end(text: &str, from: usize, close: u8) -> usize {
    let bytes = text.as_bytes();
    let (mut quote, mut depth, mut first) = (0u8, 0i32, None);
    let mut i = from;
    while i + 1 < bytes.len() {
        let c = bytes[i];
        if quote != 0 {
            if c == b'\\' {
                i += 1;
            } else if c == quote {
                quote = 0;
            }
        } else if c == b'"' || c == b'\'' {
            quote = c;
        } else {
            if c == close && bytes[i + 1] == b'}' {
                if depth <= 0 {
                    return i;
                }
                first.get_or_insert(i);
            }
            match c {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                _ => {}
            }
        }
        i += 1;
    }
    first.unwrap_or(bytes.len())
}

/// The keyword of the tag opening at `start` (`{%`), and what follows it.
fn tag_at(text: &str, start: usize) -> (&str, &str) {
    let inner = &text[start + 2..];
    let inner = inner.strip_prefix(['-', '+']).unwrap_or(inner).trim_start();
    let end = inner.find(|c: char| !c.is_ascii_alphabetic() && c != '_').unwrap_or(inner.len());
    inner.split_at(end)
}

/// Whether what follows a `macro` keyword names `name` and opens its
/// arguments.
fn opens(after: &str, name: &str) -> bool {
    after.starts_with(char::is_whitespace)
        && after.trim_start().strip_prefix(name).is_some_and(|rest| rest.trim_start().starts_with('('))
}

/// The name dbt gives the materialization a tag declares: its first argument
/// is the name, and `adapter` is looked for among the arguments after it, as a
/// keyword, never inside another word.
fn materialization_name(after: &str) -> String {
    let head = after.split("%}").next().unwrap_or(after);
    let word = |s: &str| s.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect::<String>();
    let mut args = head.split(',');
    let name = word(args.next().unwrap_or("").trim_start());
    let adapter = args
        .filter_map(|arg| arg.trim_start().strip_prefix("adapter"))
        .filter_map(|rest| rest.trim_start().strip_prefix('='))
        .map(|rest| word(rest.trim_start().trim_start_matches(['\'', '"'])))
        .find(|a| !a.is_empty())
        .unwrap_or_else(|| "default".to_string());
    format!("materialization_{name}_{adapter}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RawManifest;

    fn macros() -> Macros {
        let m = |name: &str, package: &str, path: &str| {
            serde_json::json!({
                "name": name,
                "package_name": package,
                "original_file_path": path,
                "macro_sql": "{% macro x() %}{% endmacro %}",
            })
        };
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "metadata": { "project_name": "shop" },
            "macros": {
                "macro.shop.hub": m("hub", "shop", "macros\\vault\\hub.sql"),
                "macro.shop.cents": m("cents", "shop", "macros/money.sql"),
                "macro.automate_dv.hub": m("hub", "automate_dv", "macros/tables/snowflake/hub.sql"),
                "macro.automate_dv.prefix": m("prefix", "automate_dv", "macros/internal/prefix.sql"),
                "macro.dbt_utils.star": m("star", "dbt_utils", "macros/sql/star.sql"),
                "macro.dbt.is_incremental": m("is_incremental", "dbt", "macros/materializations/is_incremental.sql"),
            },
        }))
        .unwrap();
        crate::graph::Graph::build(raw, Path::new("manifest.json"), 0, 0).macros
    }

    fn id(found: Option<&Macro>) -> &str {
        found.map(|m| m.id.as_str()).unwrap_or("none")
    }

    #[test]
    fn a_bare_name_reaches_the_project_before_a_package_of_the_same_name() {
        let m = macros();
        assert_eq!(id(m.resolve("hub", "shop")), "macro.shop.hub");
        assert_eq!(id(m.resolve("automate_dv.hub", "shop")), "macro.automate_dv.hub");
        // Qualifying with the project's own name is legal dbt, and means the same.
        assert_eq!(id(m.resolve("shop.hub", "shop")), "macro.shop.hub");
    }

    #[test]
    fn a_bare_name_never_reaches_an_installed_package() {
        let m = macros();
        // dbt answers `'star' is undefined` for this call, so a link would lie.
        assert_eq!(id(m.resolve("star", "shop")), "none");
        assert_eq!(id(m.resolve("dbt_utils.star", "shop")), "macro.dbt_utils.star");
        assert_eq!(id(m.resolve("dbt_utils.nothing", "shop")), "none");
        assert_eq!(id(m.resolve("adapter.dispatch", "shop")), "none");
        assert_eq!(id(m.resolve("a.b.c", "shop")), "none");
    }

    #[test]
    fn inside_a_package_its_own_macros_come_first() {
        let m = macros();
        let local = m.package_of("dbt_packages/automate_dv/macros/tables/snowflake/sat.sql");
        assert_eq!(local, "automate_dv");
        assert_eq!(id(m.resolve("hub", local)), "macro.automate_dv.hub");
        assert_eq!(id(m.resolve("prefix", local)), "macro.automate_dv.prefix");
        // And the root project still answers what the package does not define.
        assert_eq!(id(m.resolve("cents", local)), "macro.shop.cents");
    }

    #[test]
    fn a_file_belongs_to_the_root_project_unless_it_sits_in_an_installed_package() {
        let m = macros();
        assert_eq!(m.package_of("models/marts/orders.sql"), "shop");
        assert_eq!(m.package_of("./dbt_packages/dbt_utils/macros/sql/star.sql"), "dbt_utils");
        assert_eq!(m.package_of("dbt_modules\\dbt_utils\\macros\\star.sql"), "dbt_utils");
        // The folder itself, or a file loose in it, belongs to no package.
        assert_eq!(m.package_of("dbt_packages/README.md"), "shop");
        assert_eq!(m.package_of(""), "shop");
    }

    fn project(tag: &str, files: &[&str]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("dbt-edith-macros-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for f in files {
            let p = dir.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, "{% macro x() %}{% endmacro %}\n").unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    #[test]
    fn a_macro_is_placed_where_its_file_really_is() {
        let m = macros();
        let dir = project("place", &["macros/vault/hub.sql", "dbt_modules/dbt_utils/macros/sql/star.sql"]);
        let place = |call: &str| m.resolve(call, "shop").and_then(|found| m.place(&dir, found));
        // A manifest parsed on Windows still places a project macro.
        assert_eq!(place("hub").as_deref(), Some("macros/vault/hub.sql"));
        // A package is found under the older folder name too.
        assert_eq!(place("dbt_utils.star").as_deref(), Some("dbt_modules/dbt_utils/macros/sql/star.sql"));
        // Resolved, and still no link: not installed, or not in the project at all.
        assert_eq!(place("automate_dv.hub"), None);
        assert_eq!(place("dbt.is_incremental"), None);
        // A file the manifest remembers and the disk no longer has.
        assert_eq!(place("cents"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn names(found: Vec<&Macro>) -> Vec<String> {
        found.iter().map(|m| format!("{}.{}", m.package, m.name)).collect()
    }

    #[test]
    fn a_search_ranks_the_name_before_the_package() {
        let m = macros();
        // The project's `hub` first, then the package's of the same name.
        assert_eq!(names(m.search("HUB ")), vec!["shop.hub", "automate_dv.hub"]);
        // A package's name that starts with the query above the project's that
        // only holds it, and the project's first among those that hold it.
        assert_eq!(names(m.search("s")), vec!["dbt_utils.star", "shop.cents", "dbt.is_incremental"]);
        // A dot reads the package too, as a call is written.
        assert_eq!(names(m.search("automate_dv.h")), vec!["automate_dv.hub"]);
        assert_eq!(names(m.search("dv.")), vec!["automate_dv.hub", "automate_dv.prefix"]);
        // The file a macro is in is not its name.
        assert!(m.search("vault").is_empty());
        assert!(m.search("  ").is_empty());
    }

    #[test]
    fn a_search_finds_only_the_macros_written_in_the_project() {
        let m = macros();
        let dir = project(
            "find",
            &["macros/vault/hub.sql", "macros/money.sql", "dbt_packages/automate_dv/macros/internal/prefix.sql"],
        );
        let found = |q: &str, limit: usize| -> Vec<(String, String)> {
            m.find(&dir, q, limit).into_iter().map(|(m, file)| (m.name.clone(), file)).collect()
        };
        // automate_dv is installed without its `hub.sql`, and dbt's own macros
        // are never in the project.
        assert_eq!(found("hub", 10), vec![("hub".to_string(), "macros/vault/hub.sql".to_string())]);
        assert_eq!(found("e", 10), vec![
            ("cents".to_string(), "macros/money.sql".to_string()),
            ("prefix".to_string(), "dbt_packages/automate_dv/macros/internal/prefix.sql".to_string()),
        ]);
        // The limit counts what was found, not what was looked at.
        assert_eq!(found("e", 1).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_search_gives_up_on_a_manifest_whose_files_are_gone() {
        // Six macros the manifest places in files a later commit removed, and a
        // seventh, last in the ranking, whose file is still there.
        let entries: serde_json::Map<String, serde_json::Value> = (1..=7)
            .map(|i| {
                let name = format!("fx_{i}");
                let path = format!("macros/{name}.sql");
                let body = serde_json::json!({ "name": name, "package_name": "shop", "original_file_path": path });
                (format!("macro.shop.{name}"), body)
            })
            .collect();
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "metadata": { "project_name": "shop" },
            "macros": entries,
        }))
        .unwrap();
        let m = crate::graph::Graph::build(raw, Path::new("manifest.json"), 0, 0).macros;
        let dir = project("gone", &["macros/fx_7.sql"]);
        // One to list, so four misses: the seventh is never looked for.
        assert!(m.find(&dir, "fx", 1).is_empty());
        // Two to list, eight misses allowed, which reach it.
        let found: Vec<&str> = m.find(&dir, "fx", 2).into_iter().map(|(m, _)| m.name.as_str()).collect();
        assert_eq!(found, vec!["fx_7"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_macro_path_cannot_leave_the_project() {
        // The file exists, one level above the project, so only the refusal
        // keeps it out.
        let base = project("escape", &["shop/macros/a.sql", "elsewhere/x.sql"]);
        let outside = Macro {
            id: "macro.shop.x".into(),
            name: "x".into(),
            package: "shop".into(),
            path: "../elsewhere/x.sql".into(),
            description: String::new(),
            args: Vec::new(),
            body: String::new(),
        };
        assert!(base.join("shop").join(&outside.path).is_file());
        assert_eq!(macros().place(&base.join("shop"), &outside), None);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// dbt-core before 1.6 writes no `metadata.project_name`, only a hash of it,
    /// so the root package is named by `dbt_project.yml` instead.
    #[test]
    fn a_manifest_that_names_no_project_takes_the_name_from_its_project_file() {
        let dir = project("unnamed", &["macros/money.sql"]);
        std::fs::write(dir.join("dbt_project.yml"), "name: 'shop'\nversion: '1.0'\n").unwrap();
        std::fs::create_dir_all(dir.join("target")).unwrap();
        let manifest = dir.join("target").join("manifest.json");
        let json = serde_json::json!({
            "metadata": { "dbt_version": "1.5.0", "project_id": "0f3e3a5d" },
            "macros": {
                "macro.shop.cents": { "name": "cents", "package_name": "shop", "original_file_path": "macros/money.sql" },
            },
        });
        std::fs::write(&manifest, json.to_string()).unwrap();
        let none = dir.join("target").join("none.json");
        let g = crate::api::load_base(&dir, &manifest, &none).unwrap();
        let local = g.macros.package_of("models/orders.sql");
        let cents = g.macros.resolve("cents", local).expect("a bare call reaches the project");
        assert_eq!(g.macros.place(&dir, cents).as_deref(), Some("macros/money.sql"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_null_in_a_macro_costs_that_macro_and_nothing_else() {
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "metadata": { "project_name": "shop" },
            "nodes": { "model.shop.orders": { "name": "orders", "resource_type": "model" } },
            "macros": {
                "macro.shop.odd": { "name": null, "package_name": "shop" },
                "macro.shop.cents": {
                    "name": "cents", "package_name": "shop", "original_file_path": null,
                    "description": null, "arguments": null,
                },
                "macro.shop.fx": {
                    "name": "fx", "package_name": "shop", "original_file_path": "macros/fx.sql",
                    "description": "Converts to euros.", "arguments": [{ "name": "amount" }, { "name": null }],
                },
            },
        }))
        .unwrap();
        let g = crate::graph::Graph::build(raw, Path::new("manifest.json"), 0, 0);
        assert_eq!(g.nodes.len(), 1, "the lineage still loads");
        assert_eq!(g.macros.len(), 2);
        let fx = g.macros.resolve("fx", "shop").unwrap();
        assert_eq!(fx.description, "Converts to euros.");
        assert_eq!(fx.args, vec!["amount"]);
        assert_eq!(g.macros.resolve("cents", "shop").unwrap().path, "");
    }

    #[test]
    fn a_definition_is_found_between_its_opening_tag_and_its_closing_one() {
        let text = "{# {% macro cents(x) %} is gone #}\n\
                    {% macro other() %}1{% endmacro %}\n\
                    \n\
                    {%- macro cents(amount,\n\
                    \x20          scale=2) -%}\n\
                    \x20 {{ amount }}\n\
                    {%- endmacro -%}\n\
                    {% macro after() %}2{% endmacro %}\n";
        assert_eq!(block_in(text, "cents"), Some(Block { line: 4, end: 7 }));
        assert_eq!(block_in(text, "other"), Some(Block { line: 2, end: 2 }));
        // A longer name is not the one asked for, nor is a call.
        assert_eq!(block_in(text, "cent"), None);
        assert_eq!(block_in(text, "amount"), None);
        // Never closed: to the last line.
        assert_eq!(block_in("a\n{% macro open() %}\nbody\n", "open"), Some(Block { line: 2, end: 3 }));
    }

    #[test]
    fn a_generic_test_and_a_materialization_are_macros_under_the_names_dbt_gives_them() {
        let text = "{% test positive(model, column_name) %}\nselect 1\n{% endtest %}\n\
                    {% materialization fancy, adapter='duckdb' %}\nx\n{% endmaterialization %}\n\
                    {% materialization fancy, default %}\ny\n{% endmaterialization %}\n";
        assert_eq!(block_in(text, "test_positive"), Some(Block { line: 1, end: 3 }));
        assert_eq!(block_in(text, "materialization_fancy_duckdb"), Some(Block { line: 4, end: 6 }));
        assert_eq!(block_in(text, "materialization_fancy_default"), Some(Block { line: 7, end: 9 }));
        assert_eq!(block_in(text, "positive"), None);
    }

    #[test]
    fn what_looks_like_a_comment_or_a_block_inside_a_tag_or_a_raw_block_is_text() {
        // A `{#` in a string of a tag opens no comment, so the macro after it is found.
        let text = "{% macro a() %}{% set x = '{#' %}1{% endmacro %}\n{% macro b() %}2{% endmacro %}\n";
        assert_eq!(block_in(text, "a"), Some(Block { line: 1, end: 1 }));
        assert_eq!(block_in(text, "b"), Some(Block { line: 2, end: 2 }));
        // Nor does one in `{{ }}`, with a dict's braces inside it, or in a raw block.
        let text = "{% macro a() %}{{ {'k': '{#'} }}{% endmacro %}\n\
                    {% raw %}{# {% macro c() %} #}{% endraw %}\n\
                    {% macro b() %}2{% endmacro %}\n";
        assert_eq!(block_in(text, "b"), Some(Block { line: 3, end: 3 }));
        assert_eq!(block_in(text, "a"), Some(Block { line: 1, end: 1 }));
        // A macro written inside another ends at its own end, and the outer at the last.
        let text = "{% macro outer() %}\n{% macro inner() %}\n1\n{% endmacro %}\n{{ inner() }}\n{% endmacro %}\n";
        assert_eq!(block_in(text, "inner"), Some(Block { line: 2, end: 4 }));
        assert_eq!(block_in(text, "outer"), Some(Block { line: 1, end: 6 }));
        // `adapter` is a keyword of the arguments, not a piece of another word.
        let text = "{% materialization my_adapter_x, default %}\nx\n{% endmaterialization %}\n";
        assert_eq!(block_in(text, "materialization_my_adapter_x_default"), Some(Block { line: 1, end: 3 }));
        let text = "{% materialization fancy, adapter = \"duckdb\", supported_languages=['sql'] %}x{% endmaterialization %}\n";
        assert_eq!(block_in(text, "materialization_fancy_duckdb"), Some(Block { line: 1, end: 1 }));
    }

    #[test]
    fn a_raw_block_or_a_string_holds_no_tag_to_start_or_end_a_block() {
        // The end inside a raw block is text.
        let text = "{% macro a() %}{% raw %}{% endmacro %}{% endraw %}\n2\n{% endmacro %}\n";
        assert_eq!(block_in(text, "a"), Some(Block { line: 1, end: 3 }));
        // So is a copy of the macro before the real one, in a raw block or a string.
        let text = "{% raw %}{% macro d() %}{% endraw %}\n{{ \"{% macro d() %}\" }}\n{% macro d() %}\n1\n{% endmacro %}\n";
        assert_eq!(block_in(text, "d"), Some(Block { line: 3, end: 5 }));
    }

    #[test]
    fn the_arguments_are_the_names_before_any_default() {
        let text = "{# {% macro m(old) %} #}\n{% macro m(a, b = 'x,)', c=none,\n  d=[1, (2)], e) -%}\n{% endmacro %}\n\
                    {% macro none() %}{% endmacro %}\n{% test positive(model, column_name) %}{% endtest %}\n\
                    {% materialization fancy, default %}{% endmaterialization %}\n";
        assert_eq!(args_in(text, "m"), Some(["a", "b", "c", "d", "e"].map(String::from).to_vec()));
        assert_eq!(args_in(text, "none"), Some(Vec::new()));
        assert_eq!(args_in(text, "test_positive"), Some(["model", "column_name"].map(String::from).to_vec()));
        assert_eq!(args_in(text, "materialization_fancy_default"), Some(Vec::new()));
        assert_eq!(args_in(text, "absent"), None);
        // A quote escaped inside a default does not end it.
        let text = "{% macro e(a, b='it\\'s, ok', c=\"say \\\"hi\\\")\", d) %}{% endmacro %}\n";
        assert_eq!(args_in(text, "e"), Some(["a", "b", "c", "d"].map(String::from).to_vec()));
    }

    #[test]
    fn a_bare_name_is_the_root_projects_then_the_adapters_then_dbts() {
        let from_ids = |raw: &[(&str, &str)]| {
            let list = raw.iter().map(|(id, package)| {
                let name = id.rsplit('.').next().unwrap();
                (id.to_string(), serde_json::json!({ "name": name, "package_name": package, "original_file_path": "x.sql" }))
            });
            let raw: HashMap<String, RawMacro> = list.map(|(id, v)| (id, serde_json::from_value(v).unwrap())).collect();
            Macros::build(raw, "shop")
        };
        let both = from_ids(&[("macro.dbt.q", "dbt"), ("macro.dbt_duckdb.q", "dbt_duckdb"), ("macro.shop.q", "shop"), ("macro.pkg.r", "pkg")]);
        let package_of = |name: &str, adapter: &str| both.lookup(name, adapter).map(|at| both.at(at).package.clone());
        assert_eq!(package_of("q", "duckdb").as_deref(), Some("shop"));
        assert_eq!(package_of("pkg.r", "duckdb").as_deref(), Some("pkg"));
        // An installed package is never searched for a bare name.
        assert_eq!(package_of("r", "duckdb"), None);
        assert_eq!(package_of("dbt_duckdb.q", "postgres").as_deref(), Some("dbt_duckdb"));
        assert_eq!(package_of("a.b.c", "duckdb"), None);
        let no_root = from_ids(&[("macro.dbt.q", "dbt"), ("macro.dbt_duckdb.q", "dbt_duckdb")]);
        let package_of = |adapter: &str| no_root.lookup("q", adapter).map(|at| no_root.at(at).package.clone());
        assert_eq!(package_of("duckdb").as_deref(), Some("dbt_duckdb"));
        assert_eq!(package_of("postgres").as_deref(), Some("dbt"));
        assert_eq!(package_of("").as_deref(), Some("dbt"));
        // A parent adapter's after the adapter's own.
        let parent = from_ids(&[("macro.dbt.q", "dbt"), ("macro.dbt_postgres.q", "dbt_postgres")]);
        assert_eq!(parent.lookup("q", "redshift").map(|at| parent.at(at).package.as_str()), Some("dbt_postgres"));
    }
}
