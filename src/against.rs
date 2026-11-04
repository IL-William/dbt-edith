//! The `--against` mode: one column compared with the same figure in another
//! dbt project, by the source columns the two share (0146).
//!
//! Invariants: a source column is `(schema, table, column)` read the way the
//! warehouse compares a name, and the database is never part of it, since each
//! project renders an `env_var` database as its own compile environment set
//! it. A column's fingerprint is the source columns it is built from, found by
//! one upstream walk over collin's edges, and it is kept per call: a column is
//! walked once however many times a candidate or a rule asks for it. The
//! search for the other project's column is a bounded walk downstream of the
//! shared source columns over value edges, never a pass over every column of
//! the other project, which can be very large. Nothing
//! is served, written or run, and the other project is read as dbt left it.

use crate::graph::{is_row_role, said_cut, ColRef, Gap, Graph, Kind};
use crate::keys::norm;
use crate::ls::{self, Output, ANSWERED, REFUSED};
use crate::ls_column::{self, Ask};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::{self, Write};
use std::rc::Rc;

/// How many of the other project's columns are fingerprinted and ranked: the
/// nearest by how many of this column's sources reach them.
const CANDIDATES: usize = 20;
/// How many of those the text card prints. JSON carries them all.
const PRINTED: usize = 5;
/// How many names a list of the text card holds before `+N` counts the rest.
const LISTED: usize = 12;

/// A source column: the identity two projects share when they read the same
/// table. Each part is lower case with its quotes off (`keys::norm`).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize)]
pub struct SourceCol {
    pub schema: String,
    pub table: String,
    pub column: String,
}

impl SourceCol {
    /// `SCHEMA.TABLE.COLUMN`, as a warehouse that folds to upper case prints it.
    fn said(&self) -> String {
        ls::field(&format!("{}.{}.{}", self.schema, self.table, self.column).to_uppercase())
    }

    fn table_said(&self) -> String {
        ls::field(&format!("{}.{}", self.schema, self.table).to_uppercase())
    }
}

/// How a source column reaches the column asked, the strongest role met on a
/// path: three classes are enough to tell a copied figure from a computed one.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Class {
    /// passthrough, rename, cast.
    Copy,
    /// transform, aggregate, window, and a role the cache did not name: not
    /// known to be a copy.
    Computed,
    /// hash.
    Hashed,
}

impl Class {
    fn word(self) -> &'static str {
        match self {
            Class::Copy => "copy",
            Class::Computed => "computed",
            Class::Hashed => "hashed",
        }
    }
}

/// What an edge says of the value it carries.
enum Role {
    Value(Class),
    /// filter, join_key, dedup_key: the column decided which rows exist.
    Row,
}

/// collin's role for an edge it guessed from names where the SQL gave it none.
/// Read as a copy, since the edge carries one column's value, and marked on the
/// fingerprint so a reader knows the set rests on a guess (0146).
const INFERRED: &str = "inferred";

fn role(kind: &str) -> Role {
    match kind.to_ascii_lowercase().as_str() {
        "passthrough" | "rename" | "cast" | INFERRED => Role::Value(Class::Copy),
        "hash" => Role::Value(Class::Hashed),
        k if is_row_role(k) => Role::Row,
        _ => Role::Value(Class::Computed),
    }
}

/// What a column is built from: the source columns, by how they reach it, and
/// the models the walk crossed.
#[derive(Default, Clone, Debug)]
pub struct Fingerprint {
    /// The source columns whose values it is built from.
    pub value: BTreeMap<SourceCol, Class>,
    /// The source columns reached only through indirect edges: they decide
    /// which rows it holds, not what it is.
    pub rows: BTreeSet<SourceCol>,
    /// Where the walk ended on something that is not a source table: a seed,
    /// a snapshot, a column the SQL generates or one read from a table no node
    /// owns. Never shared.
    pub not_shared: BTreeSet<ColRef>,
    /// Whether collin gave up on a name somewhere on the walk, so the set may
    /// be short.
    pub partial: bool,
    /// Whether an edge on the walk is one collin inferred from names.
    pub inferred: bool,
    /// The nodes of the columns the walk visited, for their rules.
    pub models: BTreeSet<u32>,
    /// The source columns of `value`, as the graph holds them, for the depth.
    pub terminals: BTreeSet<ColRef>,
    /// The fewest value edges from this column up to one of its source columns
    /// the other project reads: the measure a candidate's hops give downstream
    /// of them, so a column as deep as this one is the likelier twin. Set by
    /// `with_depth`, which knows the other project; 0 until then.
    pub depth: u32,
    /// The model and the column asked, case-folded: the last tie-break between
    /// candidates the lineage cannot tell apart.
    pub named: (String, String),
}

/// What reaches a column on the way up: the strongest class by a path of
/// value edges, if any, and whether some path crossed an indirect edge.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Reach {
    value: Option<Class>,
    row: bool,
}

/// The `SourceCol` of a column of a source node.
fn source_col(graph: &Graph, c: ColRef) -> SourceCol {
    let node = &graph.nodes[c.node as usize];
    let (schema, table) = node.place();
    SourceCol { schema, table, column: norm(&node.columns[c.col as usize].name) }
}

/// A gap that means a name was lost. A chain past collin's step limit loses
/// none: all its edges are there (0141).
fn lost(g: &Gap) -> bool {
    g.reason != "steps_capped" && !g.name.is_empty()
}

/// The fingerprint of `start`: an upstream walk over all levels, the indirect
/// edges crossed at every one. Each column carries the best it was reached by,
/// and is walked again only when a better one arrives, so the work is a few
/// visits a column and ends on a cycle.
pub fn fingerprint(graph: &Graph, start: ColRef) -> Fingerprint {
    let mut fp = Fingerprint::default();
    let Some(cll) = graph.cll.as_ref() else { return fp };
    let mut seen: HashMap<ColRef, Reach> = HashMap::new();
    seen.insert(start, Reach { value: Some(Class::Copy), row: false });
    let mut work = vec![start];
    while let Some(c) = work.pop() {
        let here = seen[&c];
        for e in cll.incoming(c).filter(|e| e.from != c) {
            // What goes on from `c` along this edge: a value edge carries the
            // value on, stronger for the role met; an indirect edge turns
            // everything above it into rows, once `c` is reached at all.
            let (value, row) = match role(&cll.kinds[e.kind as usize]) {
                Role::Row => (None, true),
                Role::Value(class) => (here.value.map(|v| v.max(class)), here.row),
            };
            let slot = seen.entry(e.from).or_default();
            let better = Reach { value: slot.value.max(value), row: slot.row || row };
            if better != *slot {
                *slot = better;
                work.push(e.from);
            }
        }
    }
    let mut rows: BTreeSet<SourceCol> = BTreeSet::new();
    for (&c, reach) in &seen {
        let node = &graph.nodes[c.node as usize];
        fp.models.insert(c.node);
        // The marks are those of the value: a gap or a guess met only on a
        // path through a filter or a join key says nothing of the sources.
        if reach.value.is_some() {
            fp.partial |= cll.partial(c).any(lost);
        }
        let (mut any, mut valued) = (false, false);
        for e in cll.incoming(c).filter(|e| e.from != c) {
            any = true;
            if reach.value.is_some() {
                fp.inferred |= cll.kinds[e.kind as usize].eq_ignore_ascii_case(INFERRED);
            }
            valued |= matches!(role(&cll.kinds[e.kind as usize]), Role::Value(_));
        }
        // A column no value edge enters is where its value starts, and so is
        // one the SQL sets or one read from a table no node owns.
        if let Some(class) = reach.value.filter(|_| !valued || cll.outside(c) || cll.root(c).is_some()) {
            if node.kind == Kind::Source {
                fp.terminals.insert(c);
                let slot = fp.value.entry(source_col(graph, c)).or_insert(class);
                *slot = (*slot).max(class);
            } else {
                fp.not_shared.insert(c);
            }
        }
        if reach.row && !any && node.kind == Kind::Source {
            rows.insert(source_col(graph, c));
        }
    }
    fp.rows = rows.into_iter().filter(|s| !fp.value.contains_key(s)).collect();
    fp.named = named(graph, start);
    fp
}

/// The model and the column names of `c`, case-folded.
fn named(graph: &Graph, c: ColRef) -> (String, String) {
    let n = &graph.nodes[c.node as usize];
    (n.name.to_lowercase(), n.columns[c.col as usize].name.to_lowercase())
}

/// What a candidate's names say of the column asked, to rank by after the
/// lineage has: the column's name equal, then the model's too. Only ever a
/// tie-break (0146): the more equal names the earlier, hence the `Reverse`.
fn same_name(own: &(String, String), graph: &Graph, c: ColRef) -> std::cmp::Reverse<(bool, bool)> {
    let (model, column) = named(graph, c);
    std::cmp::Reverse((column == own.1, column == own.1 && model == own.0))
}

impl Fingerprint {
    /// This fingerprint with its depth counted to the source columns the
    /// search starts from, those whose table and column the other project
    /// has: a nearer source only this project reads would shorten it, and
    /// no candidate's hops start there.
    fn with_depth(mut self, graph: &Graph, start: ColRef, there: &Graph, places: &HashMap<(String, String), Vec<u32>>) -> Self {
        let shared: HashSet<ColRef> = self
            .terminals
            .iter()
            .copied()
            .filter(|&t| {
                let s = source_col(graph, t);
                places.get(&(s.schema.clone(), s.table.clone())).into_iter().flatten().any(|&n| there.col_slot(n, &s.column).is_some())
            })
            .collect();
        if let Some(cll) = graph.cll.as_ref() {
            self.depth = depth_of(cll, start, &shared);
        }
        self
    }
}

/// The fewest value edges from `start` up to a column of `terminals`, by a
/// breadth-first walk over value edges; 0 when it reaches none.
fn depth_of(cll: &crate::graph::ColLineage, start: ColRef, terminals: &HashSet<ColRef>) -> u32 {
    let mut seen: HashSet<ColRef> = HashSet::from([start]);
    let mut queue = std::collections::VecDeque::from([(start, 0u32)]);
    while let Some((c, hops)) = queue.pop_front() {
        if terminals.contains(&c) {
            return hops;
        }
        for e in cll.incoming(c).filter(|e| e.from != c && matches!(role(&cll.kinds[e.kind as usize]), Role::Value(_))) {
            if seen.insert(e.from) {
                queue.push_back((e.from, hops + 1));
            }
        }
    }
    0
}

/// The fingerprints of one project, each walked once per call.
pub struct Prints<'a> {
    graph: &'a Graph,
    memo: HashMap<ColRef, Rc<Fingerprint>>,
    /// How many walks were made, for the test that pins the memo.
    pub walks: usize,
}

impl<'a> Prints<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Prints { graph, memo: HashMap::new(), walks: 0 }
    }

    pub fn get(&mut self, c: ColRef) -> Rc<Fingerprint> {
        if let Some(f) = self.memo.get(&c) {
            return f.clone();
        }
        self.walks += 1;
        let f = Rc::new(fingerprint(self.graph, c));
        self.memo.insert(c, f.clone());
        f
    }
}

/// The source columns the value of a column is built from, for every column a
/// rule reads: what `Fingerprint::value` holds, without its classes and without
/// the walk through filters and join keys that a fingerprint also makes for
/// its models. One memo serves all the columns of a side, each column resolved
/// once from the sets of the columns it is built from, so the cost is that of
/// the value edges above them, not of a walk a column (0146).
pub struct ValueSources<'a> {
    graph: &'a Graph,
    memo: HashMap<ColRef, Rc<Vec<ColRef>>>,
    /// How many resolutions walked, for the test that pins the memo.
    pub walks: usize,
}

/// One column met on a resolution, as Tarjan's walk keeps it.
struct Met {
    index: usize,
    low: usize,
    on_stack: bool,
    /// The columns it is built from, by value edges.
    from: Vec<ColRef>,
}

impl<'a> ValueSources<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        ValueSources { graph, memo: HashMap::new(), walks: 0 }
    }

    /// The columns of a source node that `c` is built from, sorted. A column
    /// is where its value starts when no value edge enters it, or when it is
    /// one the SQL sets or one read from a table no node owns (as in
    /// `fingerprint`). A cycle of value edges is one set for all its columns.
    pub fn get(&mut self, c: ColRef) -> Rc<Vec<ColRef>> {
        if let Some(set) = self.memo.get(&c) {
            return set.clone();
        }
        self.walks += 1;
        let graph = self.graph;
        let Some(cll) = graph.cll.as_ref() else { return Rc::default() };
        let met_at = |c: ColRef, index: usize| {
            let mut from: Vec<ColRef> = cll
                .incoming(c)
                .filter(|e| e.from != c && matches!(role(&cll.kinds[e.kind as usize]), Role::Value(_)))
                .map(|e| e.from)
                .collect();
            from.sort_unstable();
            from.dedup();
            Met { index, low: index, on_stack: true, from }
        };
        let mut met: HashMap<ColRef, Met> = HashMap::new();
        let mut stack: Vec<ColRef> = Vec::new();
        let mut calls: Vec<(ColRef, usize)> = vec![(c, 0)];
        met.insert(c, met_at(c, 0));
        stack.push(c);
        while let Some(&(at, i)) = calls.last() {
            if let Some(&up) = met[&at].from.get(i) {
                calls.last_mut().unwrap().1 += 1;
                if self.memo.contains_key(&up) {
                    continue;
                }
                match met.get(&up) {
                    None => {
                        met.insert(up, met_at(up, met.len()));
                        stack.push(up);
                        calls.push((up, 0));
                    }
                    Some(m) if m.on_stack => {
                        let index = m.index;
                        let low = &mut met.get_mut(&at).unwrap().low;
                        *low = (*low).min(index);
                    }
                    Some(_) => {}
                }
                continue;
            }
            calls.pop();
            let (index, low) = (met[&at].index, met[&at].low);
            if index == low {
                let mut members = Vec::new();
                while let Some(m) = stack.pop() {
                    met.get_mut(&m).unwrap().on_stack = false;
                    members.push(m);
                    if m == at {
                        break;
                    }
                }
                let mut own: Vec<ColRef> = Vec::new();
                let mut inputs: Vec<Rc<Vec<ColRef>>> = Vec::new();
                for &m in &members {
                    let valued = !met[&m].from.is_empty();
                    if (!valued || cll.outside(m) || cll.root(m).is_some()) && graph.nodes[m.node as usize].kind == Kind::Source {
                        own.push(m);
                    }
                    for up in &met[&m].from {
                        if let Some(set) = self.memo.get(up) {
                            if !inputs.iter().any(|i| Rc::ptr_eq(i, set)) {
                                inputs.push(set.clone());
                            }
                        }
                    }
                }
                let set = if own.is_empty() && inputs.len() == 1 {
                    inputs.pop().unwrap()
                } else {
                    let mut all = own;
                    for i in &inputs {
                        all.extend(i.iter().copied());
                    }
                    all.sort_unstable();
                    all.dedup();
                    Rc::new(all)
                };
                for m in members {
                    self.memo.insert(m, set.clone());
                }
            }
            if let Some(&(parent, _)) = calls.last() {
                let low = met[&at].low;
                let l = &mut met.get_mut(&parent).unwrap().low;
                *l = (*l).min(low);
            }
        }
        self.memo[&c].clone()
    }
}

/// A column of the other project reached downstream of the shared sources.
pub struct Candidate {
    pub at: ColRef,
    /// How many of this column's sources reach it.
    pub hits: u32,
    /// The fewest edges from any of those sources.
    pub hops: u32,
}

/// The columns of the other project built from the source columns of `here`:
/// from each shared source column one breadth-first walk downstream over value
/// edges, the columns it reaches counted by how many of the sources reach them
/// and by the fewest edges from one. The nearest `CANDIDATES` by hits, then
/// hops, then name, and how many columns were reached in all. Hops come before
/// the name because a column built straight from the sources is likelier the
/// match than the aggregates downstream of it, which an alphabet would put
/// first. A row edge fans out to every column of a model, so it is not walked.
pub fn candidates(here: &Fingerprint, there: &Graph, places: &HashMap<(String, String), Vec<u32>>) -> (Vec<Candidate>, usize) {
    let Some(cll) = there.cll.as_ref() else { return (Vec::new(), 0) };
    // One walk a shared source column, from every column of the other project
    // that reads it: two declarations of one table are one source, and a
    // column they both reach is reached from it once.
    let mut groups: Vec<Vec<ColRef>> = Vec::new();
    for s in here.value.keys() {
        let mut starts: Vec<ColRef> = Vec::new();
        for &n in places.get(&(s.schema.clone(), s.table.clone())).into_iter().flatten() {
            if let Some(col) = there.col_slot(n, &s.column) {
                starts.push(ColRef { node: n, col });
            }
        }
        starts.sort_unstable();
        starts.dedup();
        if !starts.is_empty() {
            groups.push(starts);
        }
    }
    let mut reach: HashMap<ColRef, (u32, u32)> = HashMap::new();
    // The walk that last reached a column, so one walk crosses it once.
    let mut last: HashMap<ColRef, u32> = HashMap::new();
    for (i, starts) in groups.iter().enumerate() {
        let id = i as u32 + 1;
        let mut queue = std::collections::VecDeque::new();
        for &s in starts {
            last.insert(s, id);
            queue.push_back((s, 0u32));
        }
        while let Some((c, hops)) = queue.pop_front() {
            for e in cll.outgoing(c).iter().filter(|e| e.to != c && !cll.is_indirect(e)) {
                if last.insert(e.to, id) == Some(id) {
                    continue;
                }
                let slot = reach.entry(e.to).or_insert((0, u32::MAX));
                slot.0 += 1;
                slot.1 = slot.1.min(hops + 1);
                queue.push_back((e.to, hops + 1));
            }
        }
    }
    let mut reached: Vec<Candidate> = reach
        .into_iter()
        .filter(|(c, _)| there.nodes[c.node as usize].kind != Kind::Source)
        .map(|(at, (hits, hops))| Candidate { at, hits, hops })
        .collect();
    let considered = reached.len();
    let by_name = |c: &ColRef| {
        let n = &there.nodes[c.node as usize];
        (n.name.as_str(), n.id.as_str(), n.columns[c.col as usize].name.as_str())
    };
    let apart = |c: &Candidate| c.hops.abs_diff(here.depth);
    // Keys made once each: the names are lower-cased and copied a candidate,
    // not a comparison.
    reached.sort_by_cached_key(|c| {
        let (model, id, column) = by_name(&c.at);
        (std::cmp::Reverse(c.hits), apart(c), c.hops, same_name(&here.named, there, c.at), model.to_string(), id.to_string(), column.to_string())
    });
    reached.truncate(CANDIDATES);
    (reached, considered)
}

/// A column of the other project set against this one's fingerprint.
struct Compared {
    at: ColRef,
    reached: Option<u32>,
    hops: Option<u32>,
    shares: usize,
    /// How many of the shared sources reach both by the same class.
    agree: usize,
    union: usize,
    missing: Vec<SourceCol>,
    extra: Vec<SourceCol>,
    print: Rc<Fingerprint>,
}

fn compare(here: &Fingerprint, found: (ColRef, Option<(u32, u32)>), print: Rc<Fingerprint>) -> Compared {
    let (at, reach) = found;
    let shares = here.value.keys().filter(|s| print.value.contains_key(*s)).count();
    Compared {
        at,
        reached: reach.map(|r| r.0),
        hops: reach.map(|r| r.1),
        shares,
        agree: here.value.iter().filter(|(s, c)| print.value.get(*s) == Some(*c)).count(),
        union: here.value.len() + print.value.len() - shares,
        missing: here.value.keys().filter(|s| !print.value.contains_key(*s)).cloned().collect(),
        extra: print.value.keys().filter(|s| !here.value.contains_key(*s)).cloned().collect(),
        print,
    }
}

/// A rule's text with the mark of a cut, as the other modes print it.
type Text = (String, bool);

/// The rules of the models one side crossed, by what they read.
#[derive(Default)]
struct SideRules {
    /// By clause and the source columns the rule reads.
    placed: BTreeMap<(String, BTreeSet<SourceCol>), BTreeSet<Text>>,
    /// The rules none of whose columns resolves to a source, by clause.
    unplaced: BTreeMap<String, BTreeSet<Text>>,
}

/// The rules of `models`, each resolved to source columns: a source's column
/// is its own, a model's is the value set of that column (`ValueSources`), and
/// a column of a table no node owns resolves to nothing.
fn side_rules(values: &mut ValueSources<'_>, models: &BTreeSet<u32>) -> SideRules {
    let graph = values.graph;
    let mut rules = SideRules::default();
    let Some(cll) = graph.cll.as_ref() else { return rules };
    for &m in models {
        for r in cll.rules_of(m) {
            let mut sources: BTreeSet<SourceCol> = BTreeSet::new();
            for c in &r.columns {
                let Some(&i) = graph.index.get(c.node.as_str()) else { continue };
                let node = &graph.nodes[i as usize];
                if node.kind == Kind::Source {
                    let (schema, table) = node.place();
                    sources.insert(SourceCol { schema, table, column: norm(&c.column) });
                } else if let Some(col) = graph.col_slot(i, &c.column) {
                    sources.extend(values.get(ColRef { node: i, col }).iter().map(|&s| source_col(graph, s)));
                }
            }
            let clause = r.clause.trim().to_string();
            let text = (r.text.clone(), r.cut);
            if sources.is_empty() {
                rules.unplaced.entry(clause).or_default().insert(text);
            } else {
                rules.placed.entry((clause, sources)).or_default().insert(text);
            }
        }
    }
    rules
}

#[derive(serde::Serialize)]
struct TextOut {
    text: String,
    cut: bool,
}

fn texts(set: &BTreeSet<Text>) -> Vec<TextOut> {
    set.iter().map(|(text, cut)| TextOut { text: text.clone(), cut: *cut }).collect()
}

/// A rule both sides apply, or one side only: `here` and `there` hold the
/// texts each side writes, one of them empty for a rule on one side.
#[derive(serde::Serialize)]
struct RuleOut {
    clause: String,
    sources: Vec<SourceCol>,
    here: Vec<TextOut>,
    there: Vec<TextOut>,
}

#[derive(serde::Serialize)]
struct UnplacedOut {
    clause: String,
    texts: Vec<TextOut>,
}

#[derive(serde::Serialize, Default)]
struct RulesOut {
    /// The column of the other project the rules were set against.
    against: String,
    /// Whether the cache of each side lists any rule at all. A side without
    /// any leaves the groups empty: a rule listed on one side only would read
    /// as one the other does not apply.
    here_in_cache: bool,
    there_in_cache: bool,
    both: Vec<RuleOut>,
    here_only: Vec<RuleOut>,
    there_only: Vec<RuleOut>,
    unplaced_here: Vec<UnplacedOut>,
    unplaced_there: Vec<UnplacedOut>,
}

fn rules_out(here: &SideRules, there: &SideRules) -> RulesOut {
    let mut out = RulesOut::default();
    let rule = |k: &(String, BTreeSet<SourceCol>), h: Vec<TextOut>, t: Vec<TextOut>| RuleOut {
        clause: k.0.clone(),
        sources: k.1.iter().cloned().collect(),
        here: h,
        there: t,
    };
    for (k, h) in &here.placed {
        match there.placed.get(k) {
            Some(t) => out.both.push(rule(k, texts(h), texts(t))),
            None => out.here_only.push(rule(k, texts(h), Vec::new())),
        }
    }
    for (k, t) in &there.placed {
        if !here.placed.contains_key(k) {
            out.there_only.push(rule(k, Vec::new(), texts(t)));
        }
    }
    let unplaced = |side: &SideRules| side.unplaced.iter().map(|(c, t)| UnplacedOut { clause: c.clone(), texts: texts(t) }).collect();
    out.unplaced_here = unplaced(here);
    out.unplaced_there = unplaced(there);
    out
}

#[derive(serde::Serialize)]
struct ValueOut {
    source: SourceCol,
    class: Class,
}

/// A fingerprint as the JSON copy carries it.
#[derive(serde::Serialize)]
struct PrintOut {
    value: Vec<ValueOut>,
    rows: Vec<SourceCol>,
    not_shared: Vec<String>,
    partial: bool,
    inferred: bool,
}

#[derive(serde::Serialize)]
struct MatchOut {
    /// `model.column`, the model by the name the other modes print.
    name: String,
    unique_id: String,
    /// How many of this column's sources reach it downstream; left out when
    /// `--with` named it and no search found it.
    #[serde(skip_serializing_if = "Option::is_none")]
    reached_from: Option<u32>,
    /// The fewest edges from one of those sources; left out likewise.
    #[serde(skip_serializing_if = "Option::is_none")]
    hops: Option<u32>,
    shares: usize,
    /// How many value sources this column has: the `b` of `shares a of b`.
    of: usize,
    /// How many sources the two have together, the ranking's denominator.
    union: usize,
    missing: Vec<SourceCol>,
    extra: Vec<SourceCol>,
    fingerprint: PrintOut,
}

/// A table both projects read, with the database each renders for it.
#[derive(serde::Serialize)]
struct TableOut {
    schema: String,
    table: String,
    here: String,
    there: String,
}

#[derive(serde::Serialize)]
struct Card {
    column: String,
    against: String,
    fingerprint: PrintOut,
    tables: Vec<TableOut>,
    candidates_considered: usize,
    matches: Vec<MatchOut>,
    rules: Option<RulesOut>,
    #[serde(skip)]
    none_because: &'static str,
}

/// `model.column` as the answer names it: the model as `ls::label` does, so a
/// name two nodes share is its unique_id.
fn name_of(graph: &Graph, shared: &HashSet<&str>, c: ColRef) -> String {
    let n = &graph.nodes[c.node as usize];
    ls::field(&format!("{}.{}", ls::label(n, shared), n.columns[c.col as usize].name))
}

fn print_out(graph: &Graph, shared: &HashSet<&str>, fp: &Fingerprint) -> PrintOut {
    PrintOut {
        value: fp.value.iter().map(|(s, c)| ValueOut { source: s.clone(), class: *c }).collect(),
        rows: fp.rows.iter().cloned().collect(),
        not_shared: fp.not_shared.iter().map(|&c| name_of(graph, shared, c)).collect(),
        partial: fp.partial,
        inferred: fp.inferred,
    }
}

/// The databases the two projects render for the tables a fingerprint reads.
fn tables_of(
    fp: &Fingerprint,
    here: &Graph,
    places_here: &HashMap<(String, String), Vec<u32>>,
    there: &Graph,
    places_there: &HashMap<(String, String), Vec<u32>>,
) -> Vec<TableOut> {
    let mut seen: BTreeSet<(String, String)> = fp.value.keys().chain(fp.rows.iter()).map(|s| (s.schema.clone(), s.table.clone())).collect();
    let database = |g: &Graph, places: &HashMap<(String, String), Vec<u32>>, k: &(String, String)| {
        places.get(k).and_then(|v| v.first()).map(|&i| g.nodes[i as usize].database.clone())
    };
    let mut out = Vec::new();
    while let Some(k) = seen.pop_first() {
        if let (Some(h), Some(t)) = (database(here, places_here, &k), database(there, places_there, &k)) {
            out.push(TableOut { schema: k.0, table: k.1, here: h, there: t });
        }
    }
    out
}

/// A list of the text card: the names joined, `none` for an empty one, and the
/// rest counted past `LISTED`. `trimmed` records that something was cut.
fn listed(names: &[String], trimmed: &mut bool) -> String {
    match names.len() {
        0 => "none".to_string(),
        n if n <= LISTED => names.join(", "),
        n => {
            *trimmed = true;
            format!("{}, +{}", names[..LISTED].join(", "), n - LISTED)
        }
    }
}

fn said_list(sources: &[SourceCol], trimmed: &mut bool) -> String {
    listed(&sources.iter().map(SourceCol::said).collect::<Vec<_>>(), trimmed)
}

/// A rule's texts on one line, each in backticks, a cut one marked.
fn quoted(texts: &[TextOut]) -> String {
    texts.iter().map(|t| format!("`{}`", ls::field(&said_cut(&t.text, t.cut)))).collect::<Vec<_>>().join(", ")
}

fn tables_word(n: usize) -> String {
    if n == 1 { "1 source table".to_string() } else { format!("{n} source tables") }
}

/// The text card: one fact a line, the word that tells it apart first, a list
/// that is empty said as `none` and a section with nothing to say left out.
fn render(card: &Card, trimmed: &mut bool, rules_cut: &mut bool) -> Vec<String> {
    let mut lines = vec![format!("{}  against {}", card.column, card.against)];
    let fp = &card.fingerprint;
    let value: Vec<String> = fp.value.iter().map(|v| format!("{} {}", v.source.said(), v.class.word())).collect();
    let marks = format!("{}{}", if fp.partial { " (partial)" } else { "" }, if fp.inferred { " (inferred)" } else { "" });
    lines.push(format!("  sources, value   {}{marks}", listed(&value, trimmed)));
    if !fp.rows.is_empty() {
        lines.push(format!("  sources, rows    {}", said_list(&fp.rows, trimmed)));
    }
    if !fp.not_shared.is_empty() {
        lines.push(format!("  not a shared source   {}", listed(&fp.not_shared, trimmed)));
    }
    // One line for the tables that share a pair of databases, which is most of
    // the time all of them: a project and its twin differ in the database alone.
    let mut databases: BTreeMap<(&str, &str), Vec<String>> = BTreeMap::new();
    for t in card.tables.iter().filter(|t| !t.here.is_empty() && !t.there.is_empty() && !t.here.eq_ignore_ascii_case(&t.there)) {
        let table = SourceCol { schema: t.schema.clone(), table: t.table.clone(), column: String::new() }.table_said();
        databases.entry((t.here.as_str(), t.there.as_str())).or_default().push(table);
    }
    for ((here, there), tables) in databases {
        lines.push(format!("  databases   {}   here {}   there {}", listed(&tables, trimmed), ls::field(here), ls::field(there)));
    }
    if card.matches.is_empty() {
        lines.push(format!("  matches   {}", card.none_because));
        return lines;
    }
    lines.push("  matches".to_string());
    for m in card.matches.iter().take(PRINTED) {
        lines.push(format!(
            "    {}   shares {} of {}   missing {}   extra {}",
            m.name,
            m.shares,
            m.of,
            said_list(&m.missing, trimmed),
            said_list(&m.extra, trimmed),
        ));
    }
    let Some(rules) = &card.rules else { return lines };
    for (side, in_cache) in [("here", rules.here_in_cache), ("there", rules.there_in_cache)] {
        if !in_cache {
            lines.push(format!("  rules, {side}   no rules in the cache"));
        }
    }
    if !(rules.here_in_cache && rules.there_in_cache) {
        return lines;
    }
    // A group of rules is cut at the length of a list, `+N` counting the rest.
    let more = |n: usize, lines: &mut Vec<String>, rules_cut: &mut bool| {
        if n > LISTED {
            *rules_cut = true;
            lines.push(format!("    +{}", n - LISTED));
        }
    };
    let grouped = [("both sides", &rules.both), ("here only", &rules.here_only), ("there only", &rules.there_only)];
    if grouped.iter().all(|(_, g)| g.is_empty()) && rules.unplaced_here.is_empty() && rules.unplaced_there.is_empty() {
        lines.push("  rules   none on either path".to_string());
    } else {
        // The groups below are those of the models this column's walk and
        // that column's walk crossed: the card says whose.
        lines.push(format!("  rules, against {}", rules.against));
    }
    for (heading, group) in grouped {
        if group.is_empty() {
            continue;
        }
        lines.push(format!("  rules, {heading}"));
        for r in group.iter().take(LISTED) {
            let on = said_list(&r.sources, trimmed);
            let both = !r.here.is_empty() && !r.there.is_empty();
            lines.push(if both {
                format!("    {} on {on}   here {}   there {}", ls::field(&r.clause), quoted(&r.here), quoted(&r.there))
            } else {
                format!("    {} on {on}   {}", ls::field(&r.clause), quoted(if r.here.is_empty() { &r.there } else { &r.here }))
            });
        }
        more(group.len(), &mut lines, rules_cut);
    }
    for (side, group) in [("here", &rules.unplaced_here), ("there", &rules.unplaced_there)] {
        if group.is_empty() {
            continue;
        }
        lines.push(format!("  rules, {side} unplaced"));
        for u in group.iter().take(LISTED) {
            lines.push(format!("    {}   {}", ls::field(&u.clause), quoted(&u.texts)));
        }
        more(group.len(), &mut lines, rules_cut);
    }
    lines
}

/// Compares the column `ask` names in `here` with the other project `there`,
/// and prints the card. The exit code, or the write that failed.
pub(crate) fn respond(here: &Graph, there: &Graph, ask: &Ask, output: Output, out: &mut impl Write, err: &mut impl Write) -> io::Result<i32> {
    let focus = match ls_column::focus(here, &ask.column) {
        Ok(at) => at,
        Err(why) => {
            writeln!(err, "error: {why}")?;
            return Ok(REFUSED);
        }
    };
    let with = match ask.with.as_deref().map(|spec| ls_column::focus(there, spec)) {
        None => None,
        Some(Ok(at)) => Some(at),
        Some(Err(why)) => {
            writeln!(err, "error: --with: {why}")?;
            return Ok(REFUSED);
        }
    };
    let places_here = here.sources_by_place();
    let places_there = there.sources_by_place();
    if !places_here.keys().any(|k| places_there.contains_key(k)) {
        writeln!(
            err,
            "no shared source table: this project has {}, the other {}, and no table is in both",
            tables_word(places_here.len()),
            tables_word(places_there.len())
        )?;
        // A script that parses the JSON never meets an empty stdout.
        if output != Output::Json {
            return Ok(ANSWERED);
        }
    }

    let (mut prints_here, mut prints_there) = (Prints::new(here), Prints::new(there));
    let fp = Rc::new((*prints_here.get(focus)).clone().with_depth(here, focus, there, &places_there));
    let (picked, considered): (Vec<(ColRef, Option<(u32, u32)>)>, usize) = match with {
        Some(at) => (vec![(at, None)], 1),
        None => {
            let (found, considered) = candidates(&fp, there, &places_there);
            (found.into_iter().map(|c| (c.at, Some((c.hits, c.hops)))).collect(), considered)
        }
    };
    let mut compared: Vec<Compared> = picked.into_iter().map(|found| compare(&fp, found, prints_there.get(found.0))).collect();
    if with.is_none() {
        compared.retain(|c| c.shares > 0);
    }
    // By the share of the sources the two have together (exactly, as a
    // fraction), then the fewer edges from the sources, then the more sources
    // reaching both by one class (a copy against a copy before a copy against
    // a computed), then by name: the same pair of projects ranks the same on
    // every start (0055).
    let by_name = |c: &Compared| {
        let n = &there.nodes[c.at.node as usize];
        (n.name.clone(), n.id.clone(), n.columns[c.at.col as usize].name.clone())
    };
    // A column as deep as this one, in edges from the shared sources, before
    // one nearer or farther: a copy chain's twin is the same step of it.
    let apart = |c: &Compared| c.hops.map_or(0, |h| h.abs_diff(fp.depth));
    compared.sort_by(|a, b| {
        (b.shares * a.union)
            .cmp(&(a.shares * b.union))
            .then(apart(a).cmp(&apart(b)))
            .then(a.hops.cmp(&b.hops))
            .then(b.agree.cmp(&a.agree))
            .then_with(|| same_name(&fp.named, there, a.at).cmp(&same_name(&fp.named, there, b.at)))
            .then_with(|| by_name(a).cmp(&by_name(b)))
    });

    let (shared_here, shared_there) = (ls::shared_names(here), ls::shared_names(there));
    let rules = compared.first().map(|best| {
        let (h_in, t_in) = (here.cll.as_ref().is_some_and(|c| c.has_rules()), there.cll.as_ref().is_some_and(|c| c.has_rules()));
        // Both sides are read only when both can be: one without rules would
        // turn every rule of the other into one that is not applied there.
        let (side, other) = if h_in && t_in {
            (side_rules(&mut ValueSources::new(here), &fp.models), side_rules(&mut ValueSources::new(there), &best.print.models))
        } else {
            (SideRules::default(), SideRules::default())
        };
        let mut r = rules_out(&side, &other);
        r.against = name_of(there, &shared_there, best.at);
        r.here_in_cache = h_in;
        r.there_in_cache = t_in;
        r
    });

    let because = if fp.value.is_empty() {
        "none: it reads no source column"
    } else if !fp.value.keys().any(|s| places_there.contains_key(&(s.schema.clone(), s.table.clone()))) {
        "none: no source table it reads is in the other project"
    } else {
        "none: no column of the other project is built from those sources"
    };
    let card = Card {
        column: ls::field(ask.column.trim()),
        against: ls::field(&ask.against.as_ref().map_or_else(|| "the other project".to_string(), |p| p.display().to_string())),
        fingerprint: print_out(here, &shared_here, &fp),
        tables: tables_of(&fp, here, &places_here, there, &places_there),
        candidates_considered: considered,
        matches: compared
            .iter()
            .map(|c| {
                let n = &there.nodes[c.at.node as usize];
                MatchOut {
                    name: name_of(there, &shared_there, c.at),
                    unique_id: n.id.clone(),
                    reached_from: c.reached,
                    hops: c.hops,
                    shares: c.shares,
                    of: fp.value.len(),
                    union: c.union,
                    missing: c.missing.clone(),
                    extra: c.extra.clone(),
                    fingerprint: print_out(there, &shared_there, &c.print),
                }
            })
            .collect(),
        rules,
        none_because: because,
    };
    if output == Output::Json {
        serde_json::to_writer_pretty(&mut *out, &card)?;
        writeln!(out)?;
        return Ok(ANSWERED);
    }
    let (mut trimmed, mut rules_cut) = (false, false);
    for line in render(&card, &mut trimmed, &mut rules_cut) {
        writeln!(out, "{line}")?;
    }
    if trimmed {
        writeln!(err, "note: a list is cut at {LISTED} names and +N counts the rest; --output json lists them all")?;
    }
    if rules_cut {
        writeln!(err, "note: a group of rules is cut at {LISTED} lines and +N counts the rest; --output json lists them all")?;
    }
    // Why a side lists no rule, which the card says in a few words.
    if let Some(rules) = card.rules.as_ref() {
        for (side, in_cache, graph) in [("here", rules.here_in_cache, here), ("there", rules.there_in_cache, there)] {
            if !in_cache {
                writeln!(err, "note: rules, {side}: {}", ls_column::no_rules(&graph.meta.cll_file, &graph.meta.cll_source))?;
            }
        }
    }
    Ok(ANSWERED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collin::{RawColEdge, RawColLineage, RawPartial, RawRule, RawRuleColumn};
    use crate::manifest::RawManifest;
    use serde_json::json;
    use std::path::{Path, PathBuf};

    /// A table a project reads: its unique_id, source, identifier, schema and
    /// database, as the manifest writes them.
    struct Src(&'static str, &'static str, &'static str, &'static str, &'static str);

    fn edge(from: &str, from_col: &str, to: &str, to_col: &str, kind: &str) -> RawColEdge {
        RawColEdge { from: from.into(), from_col: from_col.into(), to: to.into(), to_col: to_col.into(), kind: kind.into() }
    }

    fn rule(node: &str, clause: &str, text: &str, cut: bool, columns: &[(&str, &str)]) -> RawRule {
        RawRule {
            node: node.into(),
            clause: clause.into(),
            text: text.into(),
            cut,
            columns: columns.iter().map(|&(node, column)| RawRuleColumn { node: node.into(), column: column.into() }).collect(),
        }
    }

    fn gap(node: &str, column: &str, reason: &str, name: &str) -> RawPartial {
        RawPartial { node: node.into(), column: column.into(), reason: reason.into(), name: name.into() }
    }

    /// A project of sources, models and seeds named by unique_id, with collin's
    /// edges, rules and gaps merged in.
    fn project(sources: &[Src], models: &[String], seeds: &[&str], edges: Vec<RawColEdge>, rules: Vec<RawRule>, partial: Vec<RawPartial>) -> Graph {
        project_with(sources, models, seeds, edges, rules, partial, Vec::new())
    }

    /// `project`, with the columns collin says the SQL sets from no column.
    fn project_with(
        sources: &[Src],
        models: &[String],
        seeds: &[&str],
        edges: Vec<RawColEdge>,
        rules: Vec<RawRule>,
        partial: Vec<RawPartial>,
        roots: Vec<crate::collin::RawRoot>,
    ) -> Graph {
        let named = |id: &str| id.rsplit('.').next().unwrap().to_string();
        let package = |id: &str| id.split('.').nth(1).unwrap().to_string();
        let mut nodes = serde_json::Map::new();
        for (kind, id) in models.iter().map(|m| ("model", m.as_str())).chain(seeds.iter().map(|s| ("seed", *s))) {
            nodes.insert(
                id.to_string(),
                json!({ "name": named(id), "resource_type": kind, "package_name": package(id), "original_file_path": format!("{kind}s/{}.sql", named(id)) }),
            );
        }
        let mut table = serde_json::Map::new();
        for Src(id, source, identifier, schema, database) in sources {
            table.insert(
                id.to_string(),
                json!({ "name": identifier.to_lowercase(), "source_name": source, "resource_type": "source", "package_name": package(id),
                        "identifier": identifier, "schema": schema, "database": database, "original_file_path": "models/sources.yml" }),
            );
        }
        let raw: RawManifest = serde_json::from_value(json!({
            "metadata": { "dbt_version": "1.11.2", "adapter_type": "snowflake" },
            "nodes": nodes, "sources": table, "child_map": {}, "parent_map": {}
        }))
        .unwrap();
        let mut g = Graph::build(raw, Path::new("manifest.json"), 0, 0);
        g.merge_col_lineage(RawColLineage { version: 1, source: "collin".into(), edges, rules, partial, roots, ..Default::default() }, 1);
        g
    }

    fn names(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    const ORDERS: &str = "source.shop.raw.orders";
    const PAYMENTS: &str = "source.shop.raw.payments";

    /// raw.orders.amount (a transform) and raw.payments.amount (a copy) make
    /// `orders.amount`; raw.orders.status filters the rows; the seed alone
    /// makes `orders.region`. Three rules, one of them reading a table no node
    /// owns.
    fn here() -> Graph {
        project(
            &[
                Src(ORDERS, "raw", "orders", "RAW", "DEV_DB"),
                Src(PAYMENTS, "raw", "payments", "RAW", "DEV_DB"),
                Src("source.shop.raw.customers", "raw", "customers", "RAW", "DEV_DB"),
            ],
            &names(&["model.shop.stg_orders", "model.shop.stg_payments", "model.shop.orders"]),
            &["seed.shop.country_codes"],
            vec![
                edge(ORDERS, "amount", "model.shop.stg_orders", "amount", "transform"),
                edge(ORDERS, "status", "model.shop.stg_orders", "amount", "filter"),
                edge(PAYMENTS, "amount", "model.shop.stg_payments", "amount", "passthrough"),
                edge("model.shop.stg_orders", "amount", "model.shop.orders", "amount", "rename"),
                edge("model.shop.stg_payments", "amount", "model.shop.orders", "amount", "passthrough"),
                edge("seed.shop.country_codes", "code", "model.shop.orders", "region", "passthrough"),
            ],
            vec![
                rule("model.shop.stg_orders", "WHERE", "status <> 'cancelled'", false, &[(ORDERS, "status")]),
                rule("model.shop.orders", "QUALIFY", "row_number() over (partition by id order by updated_at desc) = 1", false, &[(ORDERS, "id"), (ORDERS, "updated_at")]),
                rule("model.shop.orders", "JOIN", "o.customer_id = c.id", false, &[("rel:RAW.PUBLIC.CUSTOMERS", "id")]),
            ],
            Vec::new(),
        )
    }

    const LEGACY_ORDERS: &str = "source.legacy.raw.orders";
    const LEGACY_PAYMENTS: &str = "source.legacy.raw.payments";

    /// The other project reads the same tables under another database and
    /// another case, and builds `fct_orders.amount` from one source column
    /// more than `orders.amount` has, and `fct_orders.gross` from one less.
    fn there() -> Graph {
        project(
            &[Src(LEGACY_ORDERS, "raw", "Orders", "raw", "PROD_DB"), Src(LEGACY_PAYMENTS, "raw", "PAYMENTS", "Raw", "PROD_DB")],
            &names(&["model.legacy.fct_orders"]),
            &[],
            vec![
                edge(LEGACY_ORDERS, "amount", "model.legacy.fct_orders", "amount", "transform"),
                edge(LEGACY_PAYMENTS, "amount", "model.legacy.fct_orders", "amount", "rename"),
                edge(LEGACY_ORDERS, "tax", "model.legacy.fct_orders", "amount", "transform"),
                edge(LEGACY_ORDERS, "amount", "model.legacy.fct_orders", "gross", "aggregate"),
            ],
            vec![
                rule("model.legacy.fct_orders", "WHERE", "o.status != 'cancelled'", false, &[(LEGACY_ORDERS, "status")]),
                rule("model.legacy.fct_orders", "WHERE", "deleted = 0", false, &[(LEGACY_ORDERS, "deleted")]),
            ],
            Vec::new(),
        )
    }

    fn ask(column: &str) -> Ask {
        Ask {
            column: column.into(),
            up: 0,
            down: 0,
            stop_at: Vec::new(),
            tool: None,
            cache: None,
            indirect: false,
            produce: None,
            rules: false,
            casts: false,
            against: Some(PathBuf::from("../legacy")),
            with: None,
        }
    }

    fn run(here: &Graph, there: &Graph, a: &Ask, output: Output) -> (String, String, i32) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = respond(here, there, a, output, &mut out, &mut err).unwrap();
        (String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap(), code)
    }

    fn at(g: &Graph, node: &str, column: &str) -> ColRef {
        let n = g.index[node];
        ColRef { node: n, col: g.col_slot(n, column).unwrap() }
    }

    fn source(schema: &str, table: &str, column: &str) -> SourceCol {
        SourceCol { schema: schema.into(), table: table.into(), column: column.into() }
    }

    /// The whole card for the pair above: both values and the row source, the
    /// databases that differ, the two matches ranked, and every group of rules.
    #[test]
    fn a_column_is_set_beside_the_column_built_from_the_same_sources() {
        let (out, err, code) = run(&here(), &there(), &ask("orders.amount"), Output::Name);
        assert_eq!(code, ANSWERED, "{err}");
        assert_eq!(
            out,
            "orders.amount  against ../legacy\n\
             \x20 sources, value   RAW.ORDERS.AMOUNT computed, RAW.PAYMENTS.AMOUNT copy\n\
             \x20 sources, rows    RAW.ORDERS.STATUS\n\
             \x20 databases   RAW.ORDERS, RAW.PAYMENTS   here DEV_DB   there PROD_DB\n\
             \x20 matches\n\
             \x20   fct_orders.amount   shares 2 of 2   missing none   extra RAW.ORDERS.TAX\n\
             \x20   fct_orders.gross   shares 1 of 2   missing RAW.PAYMENTS.AMOUNT   extra none\n\
             \x20 rules, against fct_orders.amount\n\
             \x20 rules, both sides\n\
             \x20   WHERE on RAW.ORDERS.STATUS   here `status <> 'cancelled'`   there `o.status != 'cancelled'`\n\
             \x20 rules, here only\n\
             \x20   QUALIFY on RAW.ORDERS.ID, RAW.ORDERS.UPDATED_AT   `row_number() over (partition by id order by updated_at desc) = 1`\n\
             \x20 rules, there only\n\
             \x20   WHERE on RAW.ORDERS.DELETED   `deleted = 0`\n\
             \x20 rules, here unplaced\n\
             \x20   JOIN   `o.customer_id = c.id`\n"
        );
        assert_eq!(err, "");
        // Name and compact print the same card.
        assert_eq!(run(&here(), &there(), &ask("orders.amount"), Output::Compact).0, out);
    }

    #[test]
    fn a_fingerprint_keeps_the_strongest_role_and_sets_row_sources_apart() {
        let g = here();
        let fp = fingerprint(&g, at(&g, "model.shop.orders", "amount"));
        assert_eq!(
            fp.value.iter().map(|(s, c)| (s.said(), *c)).collect::<Vec<_>>(),
            [("RAW.ORDERS.AMOUNT".to_string(), Class::Computed), ("RAW.PAYMENTS.AMOUNT".to_string(), Class::Copy)]
        );
        assert_eq!(fp.rows.iter().map(SourceCol::said).collect::<Vec<_>>(), ["RAW.ORDERS.STATUS"]);
        assert!(fp.not_shared.is_empty() && !fp.partial);
        // The nodes it crossed, for their rules.
        assert!(fp.models.contains(&g.index["model.shop.stg_orders"]) && !fp.models.contains(&g.index["seed.shop.country_codes"]));

        // A hash beats a transform on another path to the same source column,
        // a column read both ways is a value and not a row, and a cycle ends.
        let g = project(
            &[Src(ORDERS, "raw", "orders", "RAW", "DEV_DB")],
            &names(&["model.shop.a", "model.shop.b", "model.shop.c"]),
            &[],
            vec![
                edge(ORDERS, "id", "model.shop.a", "id", "transform"),
                edge(ORDERS, "id", "model.shop.b", "id", "hash"),
                edge("model.shop.a", "id", "model.shop.c", "key", "passthrough"),
                edge("model.shop.b", "id", "model.shop.c", "key", "cast"),
                edge(ORDERS, "id", "model.shop.c", "key", "filter"),
                edge("model.shop.c", "key", "model.shop.a", "id", "passthrough"),
            ],
            Vec::new(),
            Vec::new(),
        );
        let fp = fingerprint(&g, at(&g, "model.shop.c", "key"));
        assert_eq!(fp.value.get(&source("raw", "orders", "id")), Some(&Class::Hashed));
        assert!(fp.rows.is_empty(), "read as a value too: {:?}", fp.rows);
    }

    #[test]
    fn the_database_and_the_case_of_a_table_do_not_tell_two_tables_apart() {
        let (h, t) = (here(), there());
        assert_eq!(h.nodes[h.index[ORDERS] as usize].place(), t.nodes[t.index[LEGACY_ORDERS] as usize].place());
        assert_eq!(h.nodes[h.index[PAYMENTS] as usize].place(), ("raw".to_string(), "payments".to_string()));
        assert_eq!(t.nodes[t.index[LEGACY_PAYMENTS] as usize].place(), ("raw".to_string(), "payments".to_string()));
        let (out, _, _) = run(&h, &t, &ask("orders.amount"), Output::Name);
        assert!(out.contains("fct_orders.amount   shares 2 of 2"), "{out}");
    }

    #[test]
    fn with_names_the_pair_and_a_name_the_other_project_lacks_is_refused_with_its_nearest() {
        let a = Ask { with: Some("fct_orders.gross".into()), ..ask("orders.amount") };
        let (out, _, code) = run(&here(), &there(), &a, Output::Name);
        assert_eq!(code, ANSWERED);
        assert!(out.contains("  matches\n    fct_orders.gross   shares 1 of 2   missing RAW.PAYMENTS.AMOUNT   extra none\n"), "{out}");
        assert!(!out.contains("fct_orders.amount"), "{out}");
        // The rules are said to be set against the column named.
        assert!(out.contains("  rules, against fct_orders.gross\n"), "{out}");
        // A pair that shares nothing is still the pair asked for.
        let h = project(&[Src(ORDERS, "raw", "orders", "RAW", "D")], &names(&["model.shop.o"]), &[], vec![edge(ORDERS, "x", "model.shop.o", "x", "rename")], Vec::new(), Vec::new());
        let a = Ask { with: Some("fct_orders.gross".into()), ..ask("o.x") };
        let (out, _, _) = run(&h, &there(), &a, Output::Name);
        assert!(out.contains("fct_orders.gross   shares 0 of 1   missing RAW.ORDERS.X   extra RAW.ORDERS.AMOUNT"), "{out}");

        let a = Ask { with: Some("fct_order.amount".into()), ..ask("orders.amount") };
        let (out, err, code) = run(&here(), &there(), &a, Output::Name);
        assert_eq!((out.as_str(), code), ("", REFUSED));
        assert!(err.starts_with("error: --with: ") && err.contains("did you mean fct_orders"), "{err}");
        let a = Ask { with: Some("fct_orders.nope".into()), ..ask("orders.amount") };
        let (_, err, code) = run(&here(), &there(), &a, Output::Name);
        assert_eq!(code, REFUSED);
        assert!(err.contains("fct_orders has no column nope: those with an edge are amount, gross"), "{err}");
        // This project's own column is refused as `--column` refuses.
        let (_, err, code) = run(&here(), &there(), &ask("orders.nope"), Output::Name);
        assert_eq!(code, REFUSED);
        assert!(err.starts_with("error: orders has no column nope"), "{err}");
    }

    #[test]
    fn a_side_without_rules_says_so_and_nothing_is_set_against_it() {
        let no_rules = project(
            &[Src(LEGACY_ORDERS, "raw", "orders", "raw", "PROD_DB")],
            &names(&["model.legacy.fct_orders"]),
            &[],
            vec![edge(LEGACY_ORDERS, "amount", "model.legacy.fct_orders", "amount", "transform")],
            Vec::new(),
            Vec::new(),
        );
        let (out, err, _) = run(&here(), &no_rules, &ask("orders.amount"), Output::Name);
        assert!(out.ends_with("  rules, there   no rules in the cache\n"), "{out}");
        assert!(err.contains("note: rules, there: no rules in this cache: it was written by a collin before 0.11.0, or without --indirect"), "{err}");
        assert!(!out.contains("rules, here only") && !out.contains("rules, both"), "{out}");
        let (out, _, _) = run(&no_rules, &no_rules, &ask("fct_orders.amount"), Output::Name);
        assert!(out.ends_with("  rules, here   no rules in the cache\n  rules, there   no rules in the cache\n"), "{out}");
        // Rules on both sides, none on the path of this column.
        let quiet = |id: &'static str| {
            project(
                &[Src(id, "raw", "orders", "raw", "D")],
                &names(&["model.x.m", "model.x.other"]),
                &[],
                vec![edge(id, "amount", "model.x.m", "amount", "rename")],
                vec![rule("model.x.other", "WHERE", "1 = 1", false, &[])],
                Vec::new(),
            )
        };
        let (out, _, _) = run(&quiet("source.x.raw.orders"), &quiet("source.y.raw.orders"), &ask("m.amount"), Output::Name);
        assert!(out.ends_with("  rules   none on either path\n"), "{out}");
    }

    #[test]
    fn a_rule_reading_a_models_column_is_set_by_that_columns_sources_and_a_cut_text_says_so() {
        let h = project(
            &[Src(ORDERS, "raw", "orders", "RAW", "D")],
            &names(&["model.shop.stg", "model.shop.orders"]),
            &[],
            vec![
                edge(ORDERS, "amount", "model.shop.stg", "amount", "rename"),
                edge("model.shop.stg", "amount", "model.shop.orders", "amount", "rename"),
            ],
            vec![rule("model.shop.orders", "HAVING", "sum(stg.amount) > 0 and many more terms", true, &[("model.shop.stg", "amount")])],
            Vec::new(),
        );
        let t = project(
            &[Src(LEGACY_ORDERS, "raw", "orders", "RAW", "D2")],
            &names(&["model.legacy.fct"]),
            &[],
            vec![edge(LEGACY_ORDERS, "amount", "model.legacy.fct", "amount", "rename")],
            vec![rule("model.legacy.fct", "HAVING", "sum(amount) > 0", false, &[(LEGACY_ORDERS, "amount")])],
            Vec::new(),
        );
        let (out, _, _) = run(&h, &t, &ask("orders.amount"), Output::Name);
        assert!(
            out.ends_with("  rules, both sides\n    HAVING on RAW.ORDERS.AMOUNT   here `sum(stg.amount) > 0 and many more terms (cut)`   there `sum(amount) > 0`\n"),
            "{out}"
        );
    }

    #[test]
    fn a_seed_is_not_a_shared_source_and_a_column_with_only_those_lists_no_match() {
        let (out, _, code) = run(&here(), &there(), &ask("orders.region"), Output::Name);
        assert_eq!(code, ANSWERED);
        assert_eq!(
            out,
            "orders.region  against ../legacy\n  sources, value   none\n  not a shared source   country_codes.code\n  matches   none: it reads no source column\n"
        );
        // Sources the other project does not read: a different reason.
        let other = project(&[Src("source.legacy.erp.invoices", "erp", "invoices", "ERP", "D")], &names(&["model.legacy.m"]), &[], vec![edge("source.legacy.erp.invoices", "a", "model.legacy.m", "a", "rename")], Vec::new(), Vec::new());
        let places = other.sources_by_place();
        let (_, considered) = candidates(&fingerprint(&here(), at(&here(), "model.shop.orders", "amount")), &other, &places);
        assert_eq!(considered, 0);
    }

    #[test]
    fn no_shared_table_ends_on_one_line_of_stderr_and_exit_zero() {
        let other = project(&[Src("source.legacy.erp.invoices", "erp", "invoices", "ERP", "D")], &names(&["model.legacy.m"]), &[], vec![edge("source.legacy.erp.invoices", "a", "model.legacy.m", "a", "rename")], Vec::new(), Vec::new());
        let (out, err, code) = run(&here(), &other, &ask("orders.amount"), Output::Name);
        assert_eq!((out.as_str(), code), ("", ANSWERED));
        assert_eq!(err, "no shared source table: this project has 3 source tables, the other 1 source table, and no table is in both\n");
        // JSON is an object either way.
        let (out, _, code) = run(&here(), &other, &ask("orders.amount"), Output::Json);
        assert_eq!(code, ANSWERED);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["matches"], json!([]));
    }

    #[test]
    fn a_column_collin_traced_in_part_is_marked_and_a_capped_chain_is_not() {
        let with = |gaps: Vec<RawPartial>| {
            project(
                &[Src(ORDERS, "raw", "orders", "RAW", "D")],
                &names(&["model.shop.stg", "model.shop.orders"]),
                &[],
                vec![
                    edge(ORDERS, "amount", "model.shop.stg", "amount", "rename"),
                    edge("model.shop.stg", "amount", "model.shop.orders", "amount", "rename"),
                ],
                Vec::new(),
                gaps,
            )
        };
        let g = with(vec![gap("model.shop.stg", "amount", "name_unresolved", "tax")]);
        let (out, _, _) = run(&g, &there(), &ask("orders.amount"), Output::Name);
        assert!(out.contains("  sources, value   RAW.ORDERS.AMOUNT copy (partial)\n"), "{out}");
        let g = with(vec![gap("model.shop.stg", "amount", "steps_capped", "amount")]);
        let (out, _, _) = run(&g, &there(), &ask("orders.amount"), Output::Name);
        assert!(out.contains("  sources, value   RAW.ORDERS.AMOUNT copy\n"), "{out}");
    }

    /// Twenty-five columns of the other project are built from the source, and
    /// the same share: twenty are fingerprinted, by name, and all are counted.
    #[test]
    fn past_twenty_candidates_twenty_are_fingerprinted_and_all_are_counted() {
        let models: Vec<String> = (0..25).map(|i| format!("model.legacy.m{i:02}")).collect();
        let edges = models.iter().map(|m| edge(LEGACY_ORDERS, "amount", m, "amount", "rename")).collect();
        let t = project(&[Src(LEGACY_ORDERS, "raw", "orders", "raw", "D")], &models, &[], edges, Vec::new(), Vec::new());
        let h = project(&[Src(ORDERS, "raw", "orders", "RAW", "D")], &names(&["model.shop.orders"]), &[], vec![edge(ORDERS, "amount", "model.shop.orders", "amount", "rename")], Vec::new(), Vec::new());
        let (out, _, _) = run(&h, &t, &ask("orders.amount"), Output::Json);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["candidates_considered"], 25);
        let found: Vec<&str> = v["matches"].as_array().unwrap().iter().map(|m| m["name"].as_str().unwrap()).collect();
        assert_eq!(found.len(), 20);
        assert_eq!((found[0], found[19]), ("m00.amount", "m19.amount"));
        // The card prints five.
        let (text, _, _) = run(&h, &t, &ask("orders.amount"), Output::Name);
        assert_eq!(text.lines().filter(|l| l.contains("shares 1 of 1")).count(), PRINTED);
    }

    #[test]
    fn a_fingerprint_is_walked_once_however_often_it_is_asked() {
        let g = here();
        let mut prints = Prints::new(&g);
        let c = at(&g, "model.shop.orders", "amount");
        let (a, b) = (prints.get(c), prints.get(c));
        assert!(Rc::ptr_eq(&a, &b));
        assert_eq!(prints.walks, 1);
        // Two rules reading one column of a model share its walk.
        let h = project(
            &[Src(ORDERS, "raw", "orders", "RAW", "D")],
            &names(&["model.shop.stg", "model.shop.orders"]),
            &[],
            vec![edge(ORDERS, "amount", "model.shop.stg", "amount", "rename")],
            vec![
                rule("model.shop.orders", "WHERE", "stg.amount > 0", false, &[("model.shop.stg", "amount")]),
                rule("model.shop.orders", "HAVING", "sum(stg.amount) > 0", false, &[("model.shop.stg", "amount")]),
            ],
            Vec::new(),
        );
        let mut values = ValueSources::new(&h);
        let only: BTreeSet<u32> = [h.index["model.shop.orders"]].into_iter().collect();
        let rules = side_rules(&mut values, &only);
        assert_eq!(rules.placed.len(), 2);
        assert_eq!(values.walks, 1);
    }

    /// The value sources of every column of `g` are those of its fingerprint,
    /// on a graph with a cycle, a row edge, a generated column and a column
    /// shared by two paths.
    #[test]
    fn the_memoised_value_sources_are_the_fingerprints_value_sources() {
        let mut g = here();
        let mut values = ValueSources::new(&g);
        let mut checked = 0;
        for (n, node) in g.nodes.iter().enumerate() {
            for col in 0..node.columns.len() {
                let c = ColRef { node: n as u32, col: col as u32 };
                let want: BTreeSet<SourceCol> = fingerprint(&g, c).value.keys().cloned().collect();
                let got: BTreeSet<SourceCol> = values.get(c).iter().map(|&s| source_col(&g, s)).collect();
                assert_eq!(got, want, "{}.{}", node.id, node.columns[col].name);
                checked += 1;
            }
        }
        assert!(checked > 5);
        // A cycle of value edges, one column read twice, a filter above one
        // of them, and a source read by two declarations of one table.
        let id = "source.shop.raw.orders";
        g = project(
            &[Src(id, "raw", "orders", "RAW", "D"), Src("source.shop.raw.orders2", "raw", "orders", "RAW", "D")],
            &names(&["model.shop.a", "model.shop.b", "model.shop.c"]),
            &[],
            vec![
                edge(id, "x", "model.shop.a", "x", "rename"),
                edge("source.shop.raw.orders2", "y", "model.shop.a", "x", "transform"),
                edge("model.shop.b", "x", "model.shop.a", "x", "rename"),
                edge("model.shop.a", "x", "model.shop.b", "x", "rename"),
                edge("model.shop.a", "x", "model.shop.c", "x", "rename"),
                edge("model.shop.b", "x", "model.shop.c", "x", "rename"),
                edge(id, "z", "model.shop.c", "x", "filter"),
            ],
            Vec::new(),
            Vec::new(),
        );
        let mut values = ValueSources::new(&g);
        for node in ["model.shop.c", "model.shop.b", "model.shop.a"] {
            let c = at(&g, node, "x");
            let want: BTreeSet<SourceCol> = fingerprint(&g, c).value.keys().cloned().collect();
            let got: BTreeSet<SourceCol> = values.get(c).iter().map(|&s| source_col(&g, s)).collect();
            assert_eq!(got, want, "{node}");
            assert_eq!(got.len(), 2, "{node}: {got:?}");
        }
        // The filter's column is no value source.
        assert!(!values.get(at(&g, "model.shop.c", "x")).iter().any(|&s| source_col(&g, s).column == "z"));
    }

    /// The groups hold fifteen rules each, so twelve are printed and `+3` counts
    /// the rest; JSON keeps every one.
    #[test]
    fn a_group_of_rules_is_cut_at_twelve_with_a_count_and_stderr_says_so() {
        let rules = |node: &'static str, id: &'static str| -> Vec<RawRule> {
            (0..15).map(|i| rule(node, "WHERE", &format!("c{i:02} > 0"), false, &[(id, &format!("c{i:02}"))])).collect()
        };
        let h = project(
            &[Src(ORDERS, "raw", "orders", "RAW", "D")],
            &names(&["model.shop.orders"]),
            &[],
            vec![edge(ORDERS, "amount", "model.shop.orders", "amount", "rename")],
            rules("model.shop.orders", ORDERS),
            Vec::new(),
        );
        let t = project(
            &[Src(LEGACY_ORDERS, "raw", "orders", "raw", "D")],
            &names(&["model.legacy.fct"]),
            &[],
            vec![edge(LEGACY_ORDERS, "amount", "model.legacy.fct", "amount", "rename")],
            rules("model.legacy.fct", LEGACY_ORDERS),
            Vec::new(),
        );
        let (out, err, _) = run(&h, &t, &ask("orders.amount"), Output::Name);
        let rule_lines = out.lines().skip_while(|l| !l.starts_with("  rules, both sides")).skip(1).take_while(|l| l.starts_with("    ")).count();
        assert_eq!(rule_lines, LISTED + 1, "{out}");
        assert!(out.contains("\n    +3\n"), "{out}");
        assert!(err.contains("a group of rules is cut at 12 lines") && err.contains("--output json"), "{err}");
        let (out, err, _) = run(&h, &t, &ask("orders.amount"), Output::Json);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["rules"]["both"].as_array().unwrap().len(), 15);
        assert!(!err.contains("a group of rules"), "{err}");
        // Fifteen on one side only are cut the same way.
        let (out, _, _) = run(&h, &there(), &ask("orders.amount"), Output::Name);
        assert!(out.contains("  rules, here only\n") && out.contains("\n    +3\n"), "{out}");
    }

    /// A gap met only above a filter says nothing of the sources the value is
    /// built from, so the value line does not carry it; one on the value path does.
    #[test]
    fn a_gap_met_only_through_a_filter_does_not_mark_the_value_line() {
        let build = |gap_at: &str, column: &str| {
            project(
                &[Src(ORDERS, "raw", "orders", "RAW", "D")],
                &names(&["model.shop.flt", "model.shop.orders"]),
                &[],
                vec![
                    edge(ORDERS, "amount", "model.shop.orders", "amount", "rename"),
                    edge("model.shop.flt", "status", "model.shop.orders", "amount", "filter"),
                    edge(ORDERS, "status", "model.shop.flt", "status", "inferred"),
                ],
                Vec::new(),
                vec![gap(gap_at, column, "name_unresolved", "tax")],
            )
        };
        let (out, _, _) = run(&build("model.shop.flt", "status"), &there(), &ask("orders.amount"), Output::Name);
        assert!(out.contains("  sources, value   RAW.ORDERS.AMOUNT copy\n"), "{out}");
        let g = build("model.shop.flt", "status");
        let fp = fingerprint(&g, at(&g, "model.shop.orders", "amount"));
        assert!(!fp.partial && !fp.inferred && fp.models.contains(&g.index["model.shop.flt"]));
        // A gap on the value path still marks it.
        let g = build("model.shop.orders", "amount");
        let fp = fingerprint(&g, at(&g, "model.shop.orders", "amount"));
        assert!(fp.partial);
    }

    /// Two declarations of one table are one source: a column they both reach
    /// is reached from it once.
    #[test]
    fn two_declarations_of_one_source_table_count_once_in_the_hits() {
        let t = project(
            &[Src(LEGACY_ORDERS, "raw", "orders", "raw", "D"), Src("source.legacy.other.orders", "other", "orders", "raw", "D")],
            &names(&["model.legacy.m"]),
            &[],
            vec![
                edge(LEGACY_ORDERS, "amount", "model.legacy.m", "amount", "rename"),
                edge("source.legacy.other.orders", "amount", "model.legacy.m", "amount", "rename"),
            ],
            Vec::new(),
            Vec::new(),
        );
        let h = project(&[Src(ORDERS, "raw", "orders", "RAW", "D")], &names(&["model.shop.orders"]), &[], vec![edge(ORDERS, "amount", "model.shop.orders", "amount", "rename")], Vec::new(), Vec::new());
        let fp = fingerprint(&h, at(&h, "model.shop.orders", "amount"));
        let (found, _) = candidates(&fp, &t, &t.sources_by_place());
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].hits, found[0].hops), (1, 1));
    }

    /// In a copy chain on both sides each column's twin is the same step of
    /// the other chain, not the step nearest the sources.
    #[test]
    fn in_a_copy_chain_a_column_is_set_beside_the_same_step_of_the_other_chain() {
        let chain = |pkg: &str, id: &'static str, table: &'static str| {
            let m = |n: &str| format!("model.{pkg}.{n}");
            project(
                &[Src(id, "raw", table, "RAW", "D")],
                &[m("stg"), m("int"), m("fct")],
                &[],
                vec![
                    edge(id, "amount", &m("stg"), "amount", "rename"),
                    edge(&m("stg"), "amount", &m("int"), "amount", "rename"),
                    edge(&m("int"), "amount", &m("fct"), "amount", "rename"),
                ],
                vec![
                    rule(&m("stg"), "WHERE", "amount > 0", false, &[(id, "amount")]),
                    rule(&m("int"), "WHERE", "amount < 100", false, &[(&m("stg"), "amount")]),
                    rule(&m("fct"), "WHERE", "amount <> 7", false, &[(&m("int"), "amount")]),
                ],
                Vec::new(),
            )
        };
        let (h, t) = (chain("shop", ORDERS, "orders"), chain("legacy", LEGACY_ORDERS, "orders"));
        for column in ["stg.amount", "int.amount", "fct.amount"] {
            let (out, _, _) = run(&h, &t, &ask(column), Output::Json);
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert_eq!(v["matches"][0]["name"], column, "{out}");
            // Every rule the twin also applies is on both sides.
            assert_eq!(v["rules"]["here_only"], json!([]), "{column}: {out}");
        }
        // The shortlist keeps the twin when twenty columns sit nearer the sources.
        let (out, _, _) = run(&h, &t, &ask("fct.amount"), Output::Name);
        assert!(out.contains("\n    fct.amount   shares 1 of 1"), "{out}");
    }

    /// A source only this project reads, nearer the column than the shared
    /// one, does not shorten the depth the twin is looked for at.
    #[test]
    fn a_nearer_source_the_other_project_lacks_keeps_the_twin_in_a_copy_chain() {
        let chain = |pkg: &str, id: &'static str, extra: bool| {
            let m = |n: &str| format!("model.{pkg}.{n}");
            let mut srcs = vec![Src(id, "raw", "orders", "RAW", "D")];
            let mut edges = vec![
                edge(id, "amount", &m("stg"), "amount", "rename"),
                edge(&m("stg"), "amount", &m("int"), "amount", "rename"),
                edge(&m("int"), "amount", &m("fct"), "amount", "rename"),
            ];
            if extra {
                srcs.push(Src("source.shop.raw.fx", "raw", "fx", "RAW", "D"));
                edges.push(edge("source.shop.raw.fx", "rate", &m("fct"), "amount", "transform"));
            }
            project(&srcs, &[m("stg"), m("int"), m("fct")], &[], edges, Vec::new(), Vec::new())
        };
        let (h, t) = (chain("shop", ORDERS, true), chain("legacy", LEGACY_ORDERS, false));
        let (out, _, _) = run(&h, &t, &ask("fct.amount"), Output::Name);
        assert!(out.contains("  matches\n    fct.amount "), "{out}");
        let focus = at(&h, "model.shop.fct", "amount");
        let depth = fingerprint(&h, focus).with_depth(&h, focus, &t, &t.sources_by_place()).depth;
        assert_eq!(depth, 3);
    }

    /// Candidates the lineage cannot tell apart go by name: the column named as
    /// this one, then the model too, though the alphabet would put another first.
    #[test]
    fn candidates_the_lineage_cannot_tell_apart_go_to_the_one_named_alike() {
        let h = project(&[Src(ORDERS, "raw", "orders", "RAW", "D")], &names(&["model.shop.orders"]), &[], vec![edge(ORDERS, "amount", "model.shop.orders", "amount", "rename")], Vec::new(), Vec::new());
        let mut models = names(&["model.legacy.a_orders", "model.legacy.orders", "model.legacy.b_orders"]);
        let mut edges: Vec<RawColEdge> = Vec::new();
        for m in &models {
            for column in ["a_amount", "amount"] {
                edges.push(edge(LEGACY_ORDERS, "amount", m, column, "rename"));
            }
        }
        models.push("model.legacy.c".into());
        let t = project(&[Src(LEGACY_ORDERS, "raw", "orders", "raw", "D")], &models, &[], edges, Vec::new(), Vec::new());
        let (out, _, _) = run(&h, &t, &ask("orders.amount"), Output::Json);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let found: Vec<&str> = v["matches"].as_array().unwrap().iter().map(|m| m["name"].as_str().unwrap()).collect();
        // The same column in the same model first, then the same column
        // elsewhere, then the rest by the alphabet.
        assert_eq!(found[..4], ["orders.amount", "a_orders.amount", "b_orders.amount", "a_orders.a_amount"], "{found:?}");
    }

    #[test]
    fn with_naming_a_pair_with_no_shared_table_says_so_on_stderr() {
        let other = project(&[Src("source.legacy.erp.invoices", "erp", "invoices", "ERP", "D")], &names(&["model.legacy.m"]), &[], vec![edge("source.legacy.erp.invoices", "a", "model.legacy.m", "a", "rename")], Vec::new(), Vec::new());
        let a = Ask { with: Some("m.a".into()), ..ask("orders.amount") };
        let (out, err, code) = run(&here(), &other, &a, Output::Name);
        assert_eq!((out.as_str(), code), ("", ANSWERED));
        assert!(err.starts_with("no shared source table: "), "{err}");
    }

    #[test]
    fn the_json_is_the_raw_copy_with_one_shape() {
        let (out, _, code) = run(&here(), &there(), &ask("orders.amount"), Output::Json);
        assert_eq!(code, ANSWERED);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["against", "candidates_considered", "column", "fingerprint", "matches", "rules", "tables"]);
        assert_eq!(v["column"], "orders.amount");
        assert_eq!(v["against"], "../legacy");
        assert_eq!(
            v["fingerprint"],
            json!({
                "value": [
                    { "source": { "schema": "raw", "table": "orders", "column": "amount" }, "class": "computed" },
                    { "source": { "schema": "raw", "table": "payments", "column": "amount" }, "class": "copy" },
                ],
                "rows": [{ "schema": "raw", "table": "orders", "column": "status" }],
                "not_shared": [],
                "partial": false,
                "inferred": false,
            })
        );
        assert_eq!(v["tables"][0], json!({ "schema": "raw", "table": "orders", "here": "DEV_DB", "there": "PROD_DB" }));
        assert_eq!(v["candidates_considered"], 2);
        let best = &v["matches"][0];
        assert_eq!((best["name"].as_str(), best["unique_id"].as_str()), (Some("fct_orders.amount"), Some("model.legacy.fct_orders")));
        assert_eq!((&best["shares"], &best["of"], &best["union"], &best["reached_from"]), (&json!(2), &json!(2), &json!(3), &json!(2)));
        assert_eq!(best["extra"], json!([{ "schema": "raw", "table": "orders", "column": "tax" }]));
        assert_eq!(best["fingerprint"]["value"][0]["class"], "computed");
        let rules = &v["rules"];
        assert_eq!((rules["against"].as_str(), &rules["here_in_cache"], &rules["there_in_cache"]), (Some("fct_orders.amount"), &json!(true), &json!(true)));
        assert_eq!(rules["both"][0]["clause"], "WHERE");
        assert_eq!(rules["both"][0]["here"], json!([{ "text": "status <> 'cancelled'", "cut": false }]));
        assert_eq!(rules["there_only"][0]["sources"], json!([{ "schema": "raw", "table": "orders", "column": "deleted" }]));
        assert_eq!(rules["unplaced_here"][0]["clause"], "JOIN");
        // `--with` leaves the search's count out.
        let a = Ask { with: Some("fct_orders.gross".into()), ..ask("orders.amount") };
        let v: serde_json::Value = serde_json::from_str(&run(&here(), &there(), &a, Output::Json).0).unwrap();
        assert!(v["matches"][0].get("reached_from").is_none());
        assert_eq!(v["candidates_considered"], 1);
    }

    #[test]
    fn a_long_list_is_cut_with_a_count_and_stderr_says_where_the_rest_is() {
        let models = names(&["model.shop.wide"]);
        let edges = (0..15).map(|i| edge(ORDERS, &format!("c{i:02}"), "model.shop.wide", "total", "transform")).collect();
        let h = project(&[Src(ORDERS, "raw", "orders", "RAW", "D")], &models, &[], edges, Vec::new(), Vec::new());
        let t = project(&[Src(LEGACY_ORDERS, "raw", "orders", "raw", "D")], &names(&["model.legacy.m"]), &[], vec![edge(LEGACY_ORDERS, "c00", "model.legacy.m", "x", "rename")], Vec::new(), Vec::new());
        let (out, err, _) = run(&h, &t, &ask("wide.total"), Output::Name);
        assert!(out.contains("RAW.ORDERS.C11 computed, +3\n"), "{out}");
        assert!(err.contains("a list is cut at 12 names") && err.contains("--output json"), "{err}");
    }

    /// Twenty-five aggregates sit downstream of the column built straight from
    /// the source, and all of them are reached by the same one source: the
    /// alphabet must not decide which twenty are fingerprinted.
    #[test]
    fn a_column_built_straight_from_the_sources_outranks_the_aggregates_downstream_of_it() {
        let mut models = names(&["model.legacy.stg_orders"]);
        models.extend((0..25).map(|i| format!("model.legacy.agg_{i:02}")));
        let mut edges = vec![edge(LEGACY_ORDERS, "amount", "model.legacy.stg_orders", "amount", "rename")];
        edges.extend((0..25).map(|i| edge("model.legacy.stg_orders", "amount", &format!("model.legacy.agg_{i:02}"), "amount", "aggregate")));
        let t = project(&[Src(LEGACY_ORDERS, "raw", "orders", "raw", "D")], &models, &[], edges, Vec::new(), Vec::new());
        let h = project(&[Src(ORDERS, "raw", "orders", "RAW", "D")], &names(&["model.shop.orders"]), &[], vec![edge(ORDERS, "amount", "model.shop.orders", "amount", "rename")], Vec::new(), Vec::new());
        let (out, _, _) = run(&h, &t, &ask("orders.amount"), Output::Json);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["candidates_considered"], 26);
        let first = &v["matches"][0];
        assert_eq!((first["name"].as_str(), &first["hops"]), (Some("stg_orders.amount"), &json!(1)));
        assert_eq!(v["matches"][1]["hops"], 2);
        let (text, _, _) = run(&h, &t, &ask("orders.amount"), Output::Name);
        assert!(text.contains("  matches\n    stg_orders.amount   shares 1 of 1"), "{text}");
    }

    /// Two columns the same number of edges from the source and the same share
    /// of it: the one that copies, as this one does, comes before the one that
    /// computes, though its name is later.
    #[test]
    fn a_tie_goes_to_the_column_whose_class_agrees_before_its_name() {
        let t = project(
            &[Src(LEGACY_ORDERS, "raw", "orders", "raw", "D")],
            &names(&["model.legacy.a_calc", "model.legacy.z_copy"]),
            &[],
            vec![
                edge(LEGACY_ORDERS, "amount", "model.legacy.a_calc", "amount", "transform"),
                edge(LEGACY_ORDERS, "amount", "model.legacy.z_copy", "amount", "rename"),
            ],
            Vec::new(),
            Vec::new(),
        );
        let h = project(&[Src(ORDERS, "raw", "orders", "RAW", "D")], &names(&["model.shop.orders"]), &[], vec![edge(ORDERS, "amount", "model.shop.orders", "amount", "rename")], Vec::new(), Vec::new());
        let (out, _, _) = run(&h, &t, &ask("orders.amount"), Output::Name);
        let order: Vec<&str> = out.lines().filter(|l| l.contains("shares")).map(|l| l.trim().split("   ").next().unwrap()).collect();
        assert_eq!(order, ["z_copy.amount", "a_calc.amount"], "{out}");
    }

    /// An edge collin inferred is a copy, and the line says the set rests on a
    /// guess, beside the mark for a name collin lost.
    #[test]
    fn an_inferred_edge_is_a_copy_and_marks_the_fingerprint() {
        let g = project(
            &[Src(ORDERS, "raw", "orders", "RAW", "D")],
            &names(&["model.shop.stg", "model.shop.orders"]),
            &[],
            vec![
                edge(ORDERS, "amount", "model.shop.stg", "amount", "inferred"),
                edge("model.shop.stg", "amount", "model.shop.orders", "amount", "rename"),
            ],
            Vec::new(),
            vec![gap("model.shop.stg", "amount", "name_unresolved", "tax")],
        );
        let fp = fingerprint(&g, at(&g, "model.shop.orders", "amount"));
        assert_eq!(fp.value.get(&source("raw", "orders", "amount")), Some(&Class::Copy));
        assert!(fp.inferred && fp.partial);
        let (out, _, _) = run(&g, &there(), &ask("orders.amount"), Output::Name);
        assert!(out.contains("  sources, value   RAW.ORDERS.AMOUNT copy (partial) (inferred)\n"), "{out}");
        // Without one, no mark; the JSON copy says so either way.
        let (out, _, _) = run(&here(), &there(), &ask("orders.amount"), Output::Name);
        assert!(!out.contains("(inferred)"), "{out}");
        let v: serde_json::Value = serde_json::from_str(&run(&g, &there(), &ask("orders.amount"), Output::Json).0).unwrap();
        assert_eq!(v["fingerprint"]["inferred"], true);
    }

    /// A column the SQL sets, and one read from a table no node owns, are
    /// recorded as not shared, and the walk goes on through their other edges.
    #[test]
    fn a_generated_column_and_an_outside_one_are_not_shared_yet_their_other_edges_are_walked() {
        let g = project_with(
            &[Src(ORDERS, "raw", "orders", "RAW", "D")],
            &names(&["model.shop.stg", "model.shop.orders"]),
            &[],
            vec![
                edge(ORDERS, "amount", "model.shop.stg", "amount", "rename"),
                edge("rel:RAW.PUBLIC.TAXES", "rate", "model.shop.stg", "amount", "transform"),
                edge("model.shop.stg", "amount", "model.shop.orders", "amount", "rename"),
                edge(ORDERS, "tax", "model.shop.orders", "channel", "rename"),
            ],
            Vec::new(),
            Vec::new(),
            vec![crate::collin::RawRoot { node: "model.shop.orders".into(), column: "channel".into(), expressions: vec!["'web'".into()] }],
        );
        let outside = fingerprint(&g, at(&g, "model.shop.orders", "amount"));
        assert_eq!(outside.value.keys().map(SourceCol::said).collect::<Vec<_>>(), ["RAW.ORDERS.AMOUNT"]);
        assert_eq!(outside.not_shared, [at(&g, "model.shop.stg", "amount")].into_iter().collect());
        let generated = fingerprint(&g, at(&g, "model.shop.orders", "channel"));
        assert_eq!(generated.value.keys().map(SourceCol::said).collect::<Vec<_>>(), ["RAW.ORDERS.TAX"]);
        assert_eq!(generated.not_shared, [at(&g, "model.shop.orders", "channel")].into_iter().collect());
    }

    /// A layered project of invented models, each built from three of the layer
    /// below by value, filter and join-key edges, with rules on every model.
    fn layered(pkg: &str, layers: usize, width: usize, cols: usize) -> Graph {
        let src_ids: Vec<String> = (0..width).map(|t| format!("source.{pkg}.raw.t{t}")).collect();
        let srcs: Vec<Src> = src_ids.iter().enumerate().map(|(t, id)| Src(Box::leak(id.clone().into_boxed_str()), "raw", Box::leak(format!("t{t}").into_boxed_str()), "RAW", "D")).collect();
        let mut models = Vec::new();
        let mut edges = Vec::new();
        let mut rules = Vec::new();
        let mut prev: Vec<String> = src_ids.clone();
        for l in 0..layers {
            let mut cur = Vec::new();
            for i in 0..width {
                let m = format!("model.{pkg}.l{l}_m{i}");
                let p1 = prev[i % prev.len()].clone();
                let p2 = prev[(i * 7 + 3 + l) % prev.len()].clone();
                let p3 = prev[(i * 13 + 5 + 2 * l) % prev.len()].clone();
                for k in 0..cols {
                    let c = format!("c{k}");
                    edges.push(edge(&p1, &c, &m, &c, "rename"));
                    edges.push(edge(&p2, &c, &m, &c, "transform"));
                    edges.push(edge(&p1, "c0", &m, &c, "join_key"));
                    edges.push(edge(&p2, "c0", &m, &c, "join_key"));
                    edges.push(edge(&p1, "c1", &m, &c, "filter"));
                    edges.push(edge(&p3, &c, &m, &c, "transform"));
                    edges.push(edge(&p3, "c0", &m, &c, "join_key"));
                }
                rules.push(rule(&m, "JOIN", "a.c0 = b.c0", false, &[(p1.as_str(), "c0"), (p2.as_str(), "c0")]));
                rules.push(rule(&m, "WHERE", "a.c1 > 0", false, &[(p1.as_str(), "c1")]));
                rules.push(rule(&m, "WHERE", "b.c2 > 0", false, &[(p2.as_str(), "c2"), (p1.as_str(), "c3")]));
                rules.push(rule(&m, "JOIN", "c.c0 = a.c0", false, &[(p3.as_str(), "c0"), (p3.as_str(), &format!("c{}", 4 + i % 30))]));
                cur.push(m.clone());
                models.push(m);
            }
            prev = cur;
        }
        project(&srcs, &models, &[], edges, rules, Vec::new())
    }

    /// Run by hand: `L=15 W=200 C=40 cargo test --release -- --ignored --nocapture bench_layered`.
    #[test]
    #[ignore]
    fn bench_layered() {
        let layers: usize = std::env::var("L").ok().and_then(|v| v.parse().ok()).unwrap_or(12);
        let width: usize = std::env::var("W").ok().and_then(|v| v.parse().ok()).unwrap_or(60);
        let cols: usize = std::env::var("C").ok().and_then(|v| v.parse().ok()).unwrap_or(40);
        let t0 = std::time::Instant::now();
        let h = layered("shop", layers, width, cols);
        let t = layered("legacy", layers, width, cols);
        eprintln!("built in {:?}", t0.elapsed());
        let t1 = std::time::Instant::now();
        let fp = fingerprint(&h, at(&h, &format!("model.shop.l{}_m0", layers - 1), "c5"));
        eprintln!("one fingerprint {:?}: value {} rows {} models {}", t1.elapsed(), fp.value.len(), fp.rows.len(), fp.models.len());
        for top in [format!("l{}_m0.c5", layers - 1), format!("l{}_m17.c9", layers / 2)] {
            let t2 = std::time::Instant::now();
            let (out, _, code) = run(&h, &t, &ask(&top), Output::Name);
            let best = out.lines().skip_while(|l| *l != "  matches").nth(1).unwrap_or("none").trim().to_string();
            eprintln!("respond {top} {:?} code {code} lines {} bytes {} best {best}", t2.elapsed(), out.lines().count(), out.len());
        }
        let mut values = ValueSources::new(&h);
        let t3 = std::time::Instant::now();
        let _ = side_rules(&mut values, &fp.models);
        eprintln!("side_rules here {:?} walks {}", t3.elapsed(), values.walks);
    }
}
