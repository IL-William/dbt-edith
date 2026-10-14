//! Which incremental strategy dbt runs for a model, and whether it inserts
//! every row, read from the macros the manifest carries (0076).
//!
//! The answer lives in macros a project or a package can override by name, so
//! it is read the way dbt reaches them rather than from a table per adapter,
//! and only the bodies of the few macros involved are kept
//! (`crate::manifest`). The invariant: anything these steps cannot reach
//! answers None, and the box keeps `incremental`. No name is better than a
//! wrong one, and the insert is claimed only on macros known by their own
//! test of the key.

use crate::macros::{Macro, Macros};
use crate::project::DispatchOrder;

/// A unique key as the manifest holds it, which is what the merge macros test:
/// Snowflake's whether it `is none`, dbt's own whether it is falsy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Key {
    /// Absent, or null.
    #[default]
    None,
    /// Falsy to Jinja, and still not none: `''`, `[]`, `{}`, `false`, `0`.
    Falsy,
    Given,
}

impl Key {
    pub fn of(value: Option<&serde_json::Value>) -> Key {
        use serde_json::Value;
        match value {
            None | Some(Value::Null) => Key::None,
            Some(Value::String(s)) if s.is_empty() => Key::Falsy,
            Some(Value::Array(a)) if a.is_empty() => Key::Falsy,
            Some(Value::Object(o)) if o.is_empty() => Key::Falsy,
            Some(Value::Bool(false)) => Key::Falsy,
            Some(Value::Number(n)) if n.as_f64() == Some(0.0) => Key::Falsy,
            Some(_) => Key::Given,
        }
    }
}

/// What an incremental model's box shows once its strategy is read: the
/// strategy, or `append` for one that inserts every row.
#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    pub shown: String,
    /// The strategy dbt was given or picked, when `shown` is `append` for it.
    pub named: String,
    /// The adapter's default, rather than a strategy configured.
    pub default: bool,
    /// The macro that inserts every row.
    pub insert: &'static str,
}

/// Which incremental strategy dbt runs for a model, and whether it inserts
/// every row, read from the macros the manifest carries rather than from a
/// table per adapter, which a project's or a package's override would make
/// wrong (0076). Anything these steps cannot reach answers None: no name is
/// better than a wrong one.
///
/// Three lookups, each the one dbt makes:
/// - by name, from a model: the model's package, the root project, then dbt's
///   own packages, the adapter's before dbt's (`MacroNamespace`);
/// - by dispatch, for dbt's own macros: the packages `dispatch:` gives the
///   `dbt` namespace, else the root project then dbt's own, and in each the
///   adapter's prefix, its parents', then `default__`;
/// - the materialization: the adapter's, else its parents', else the default,
///   a project's or a package's before dbt's own.
pub struct Incremental<'a> {
    macros: &'a Macros,
    /// dbt's own packages, in the order a bare name reaches them: the
    /// adapter's, its parents', then `dbt`.
    internal: Vec<String>,
    /// The adapter, its parents, then `default`, or None when two parents or
    /// more leave their order unknown.
    prefixes: Option<Vec<String>>,
    dispatch: &'a DispatchOrder,
}

/// The macros that make a merge or a delete+insert an insert of every row
/// when the key is missing, each by its own test of the key. The claim rests
/// on their bodies as dbt writes them today, so each is checked for the test
/// it is known by, and a body that changed under the same name claims nothing.
const SNOWFLAKE_MERGE: &str = "snowflake__get_merge_sql";
const DEFAULT_MERGE: &str = "default__get_merge_sql";
const DEFAULT_DELETE_INSERT: &str = "default__get_delete_insert_merge_sql";

impl<'a> Incremental<'a> {
    pub fn new(macros: &'a Macros, adapter: &str, dispatch: &'a DispatchOrder) -> Self {
        if adapter.is_empty() {
            return Incremental { macros, internal: Vec::new(), prefixes: None, dispatch };
        }
        // The manifest does not say which adapter another inherits from, but
        // it carries the parent's macros: `dbt_postgres`'s under Redshift. An
        // adapter is told from a package of the same naming by the macro every
        // adapter defines, and the project itself is never one.
        let mut parents: Vec<&str> = macros
            .packages()
            .filter(|package| *package != macros.root())
            .filter_map(|package| {
                let x = package.strip_prefix("dbt_").filter(|x| !x.is_empty() && *x != adapter)?;
                macros.has(package, &format!("{x}__create_table_as")).then_some(x)
            })
            .collect();
        parents.sort_unstable();
        let mut internal = vec![format!("dbt_{adapter}")];
        internal.extend(parents.iter().map(|p| format!("dbt_{p}")));
        internal.push("dbt".to_string());
        let prefixes = (parents.len() < 2).then(|| {
            let mut p = vec![adapter.to_string()];
            p.extend(parents.iter().map(|x| x.to_string()));
            p.push("default".to_string());
            p
        });
        Incremental { macros, internal, prefixes, dispatch }
    }

    fn is_internal(&self, package: &str) -> bool {
        self.internal.iter().any(|p| p == package)
    }

    /// dbt's own packages flattened into one namespace, the adapter's first.
    fn internal_macro(&self, name: &str) -> Option<&'a Macro> {
        self.internal.iter().find_map(|p| self.macros.get(p, name))
    }

    /// A bare name called while a model of package `local` runs.
    fn by_name(&self, name: &str, local: &str) -> Option<&'a Macro> {
        self.macros
            .get(local, name)
            .or_else(|| self.macros.get(self.macros.root(), name))
            .or_else(|| self.internal_macro(name))
    }

    /// `adapter.dispatch(name, 'dbt')`.
    fn dispatched(&self, name: &str) -> Option<&'a Macro> {
        let prefixes = self.prefixes.as_ref()?;
        let root = self.macros.root();
        let default = [root.to_string(), "dbt".to_string()];
        let order: &[String] = match self.dispatch {
            DispatchOrder::Given(order) => order,
            DispatchOrder::Default => &default,
            // An order that cannot be read matters only if a package besides
            // dbt's own, the project included, has a version it could put
            // before dbt's or after it.
            DispatchOrder::Unreadable => {
                let other = self.macros.packages().any(|package| {
                    !self.is_internal(package) && prefixes.iter().any(|p| self.macros.has(package, &format!("{p}__{name}")))
                });
                if other {
                    return None;
                }
                &default
            }
        };
        order.iter().find_map(|package| {
            // dbt finds no macro of an adapter's package by its name: only `dbt`
            // stands for its own packages there, and the others are passed over.
            if package != "dbt" && self.is_internal(package) {
                return None;
            }
            prefixes.iter().find_map(|p| {
                let wanted = format!("{p}__{name}");
                if package == "dbt" { self.internal_macro(&wanted) } else { self.macros.get(package, &wanted) }
            })
        })
    }

    /// The macro a call of `name` ends in: by name, and through the dispatch
    /// when the one found is dbt's own and only dispatches.
    fn reach(&self, name: &str, local: &str) -> Option<&'a Macro> {
        let m = self.by_name(name, local)?;
        if self.is_internal(&m.package) && dispatches(&m.body, name) {
            return self.dispatched(name);
        }
        Some(m)
    }

    /// The strategy dbt picks for an incremental model of package `local`
    /// that configures none.
    pub fn default_strategy(&self, local: &str) -> Option<String> {
        let picked = self.materialization_default(local)?;
        if picked != "default" {
            return Some(picked);
        }
        self.named_default(local)
    }

    /// What `get_incremental_default_sql` names: where `'default'` leads,
    /// fallen back to or configured outright, whatever the materialization
    /// would fall back to.
    pub fn named_default(&self, local: &str) -> Option<String> {
        self.materialization()?;
        returned_strategy(&self.reach("get_incremental_default_sql", local)?.body)
    }

    /// The incremental materialization dbt picks, when it is dbt's own: the
    /// adapter's, else its parents', else the default. A project's or a
    /// package's comes before dbt's of the same rank, and may do anything at
    /// all with the strategy and the key.
    fn materialization(&self) -> Option<&'a Macro> {
        for prefix in self.prefixes.as_ref()? {
            let name = format!("materialization_incremental_{prefix}");
            let mut found = self.macros.named(&name).peekable();
            if found.peek().is_none() {
                continue;
            }
            if found.any(|m| !self.is_internal(&m.package)) {
                return None;
            }
            return self.internal_macro(&name);
        }
        None
    }

    /// What the incremental materialization falls back to,
    /// `config.get('incremental_strategy') or '<x>'`, in its body or in a
    /// macro it calls, one call deep.
    fn materialization_default(&self, local: &str) -> Option<String> {
        let body = &self.materialization()?.body;
        fallback_in(body).or_else(|| calls(body).into_iter().find_map(|c| fallback_in(&self.by_name(c, local)?.body)))
    }

    /// The macro that turns `strategy` into an insert of every row for a
    /// model of package `local` with key `key`, or None.
    pub fn inserts(&self, strategy: &str, local: &str, key: Key) -> Option<&'static str> {
        if key == Key::Given {
            return None;
        }
        // Only dbt's own materialization is known to hand the strategy and
        // the key on as they are.
        self.materialization()?;
        let (own, called) = match strategy {
            "merge" => ("get_incremental_merge_sql", "get_merge_sql"),
            "delete+insert" => ("get_incremental_delete_insert_sql", "get_delete_insert_merge_sql"),
            _ => return None,
        };
        // dbt-core's own version, which hands the key on as it is.
        let m = self.reach(own, local)?;
        if m.package != "dbt" || m.name != format!("default__{own}") || !hands_key_on(&m.body, called) {
            return None;
        }
        self.known_insert(self.reach(called, local)?, local, key)
    }

    fn known_insert(&self, m: &Macro, local: &str, key: Key) -> Option<&'static str> {
        let b = m.body.as_str();
        match (m.package.as_str(), m.name.as_str()) {
            // Snowflake refuses a merge on a constant false, so its macro writes
            // a plain insert when the key is none.
            ("dbt_snowflake", SNOWFLAKE_MERGE) if tests_key(b, "is none") && b.contains("insert into") => {
                (key == Key::None).then_some(SNOWFLAKE_MERGE)
            }
            ("dbt", DEFAULT_MERGE) if tests_key(b, "") && b.contains("'FALSE'") => Some(DEFAULT_MERGE),
            // Snowflake's wraps dbt's in a transaction.
            ("dbt_snowflake", "snowflake__get_delete_insert_merge_sql") if b.contains(&format!("{DEFAULT_DELETE_INSERT}(")) => {
                let inner = self.by_name(DEFAULT_DELETE_INSERT, local)?;
                if inner.package != "dbt" {
                    return None;
                }
                self.known_insert(inner, local, key)
            }
            ("dbt", DEFAULT_DELETE_INSERT) if tests_key(b, "") && b.contains("delete from") && b.contains("insert into") => {
                Some(DEFAULT_DELETE_INSERT)
            }
            _ => None,
        }
    }
}

/// A body without its `{# #}` comments, which mention names freely.
fn uncommented(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(at) = rest.find("{#") {
        out.push_str(&rest[..at]);
        match rest[at..].find("#}") {
            Some(end) => rest = &rest[at + end + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

fn skip_ws(s: &str) -> &str {
    s.trim_start_matches([' ', '\t', '\n', '\r'])
}

/// A quoted name at the start of `s`, in either quote, and what follows it.
fn quoted_at(s: &str) -> Option<(&str, &str)> {
    let q = s.chars().next().filter(|c| *c == '\'' || *c == '"')?;
    let end = s[1..].find(q)?;
    Some((&s[1..1 + end], &s[end + 2..]))
}

/// The `<x>` of `config.get('incremental_strategy') or '<x>'`.
fn fallback_in(body: &str) -> Option<String> {
    let body = uncommented(body);
    let mut rest = body.as_str();
    while let Some(at) = rest.find("config.get(") {
        rest = &rest[at + "config.get(".len()..];
        let found = (|| {
            let (key, after) = quoted_at(skip_ws(rest))?;
            if key != "incremental_strategy" {
                return None;
            }
            let after = skip_ws(after).strip_prefix(')')?;
            let after = skip_ws(after).strip_prefix("or")?;
            if !after.starts_with([' ', '\t', '\n', '\r']) {
                return None;
            }
            let (x, _) = quoted_at(skip_ws(after))?;
            (!x.is_empty()).then(|| x.to_string())
        })();
        if found.is_some() {
            return found;
        }
    }
    None
}

/// The names a body calls, each once, in the order they first appear.
fn calls(body: &str) -> Vec<&str> {
    let b = body.as_bytes();
    let mut out: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_alphabetic() || b[i] == b'_' {
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            // An attribute, `adapter.dispatch(`, is no macro of its own.
            let attribute = start > 0 && b[start - 1] == b'.';
            if !attribute && b.get(i) == Some(&b'(') && !out.contains(&&body[start..i]) {
                out.push(&body[start..i]);
            }
        } else {
            i += 1;
        }
    }
    out
}

/// Whether dbt's version of `name` does nothing but dispatch it.
fn dispatches(body: &str, name: &str) -> bool {
    body.contains(&format!("adapter.dispatch('{name}'")) || body.contains(&format!("adapter.dispatch(\"{name}\""))
}

/// Whether a strategy macro hands the model's key on to `called` untouched.
fn hands_key_on(body: &str, called: &str) -> bool {
    body.contains(&format!("{called}(")) && (body.contains("arg_dict[\"unique_key\"]") || body.contains("arg_dict['unique_key']"))
}

/// Whether a body tests the key with `{% if unique_key <test> %}`, `test`
/// being empty for a plain truth test.
fn tests_key(body: &str, test: &str) -> bool {
    let wanted = if test.is_empty() { "if unique_key".to_string() } else { format!("if unique_key {test}") };
    let mut rest = body;
    while let Some(at) = rest.find(&wanted) {
        rest = &rest[at + wanted.len()..];
        let after = skip_ws(rest);
        if after.starts_with("%}") || after.starts_with("-%}") {
            return true;
        }
    }
    false
}

/// The strategy a default macro names, when its whole body is one
/// `return(get_incremental_<name>_sql(...))`, as `{{ }}` or `{% do %}`. Any
/// other body, a condition on the key included, names none.
fn returned_strategy(body: &str) -> Option<String> {
    let body = uncommented(body);
    let open = body.find("%}")? + 2;
    let close = body.rfind("endmacro")?;
    let close = body[..close].rfind("{%")?;
    let inner = body.get(open..close)?.trim();
    let (stmt, tag) = if let Some(s) = inner.strip_prefix("{{") {
        (s.strip_suffix("}}")?, false)
    } else {
        (inner.strip_prefix("{%")?.strip_suffix("%}")?, true)
    };
    let stmt = stmt.trim_start_matches('-').trim_end_matches('-').trim();
    if stmt.contains("{{") || stmt.contains("{%") || stmt.contains("}}") || stmt.contains("%}") {
        return None;
    }
    let stmt = if tag { stmt.strip_prefix("do")?.trim_start() } else { stmt };
    let call = stmt.strip_prefix("return(")?.strip_suffix(')')?.trim();
    let name = call.strip_prefix("get_incremental_")?;
    let (name, args) = name.split_once("_sql(")?;
    if name.is_empty() || !name.bytes().all(|c| c.is_ascii_lowercase() || c == b'_') {
        return None;
    }
    // The arguments close the call and nothing follows.
    let mut depth = 1;
    for (i, c) in args.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && i + 1 != args.len() {
                    return None;
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    // dbt names the macro from the strategy with `+` made `_`.
    Some(if name == "delete_insert" { "delete+insert".to_string() } else { name.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{RawMacro, RawManifest};
    use std::collections::HashMap;
    use std::path::Path;

    // The incremental macros, shortened from the bodies a Snowflake manifest
    // and a DuckDB one carry: every line a test below reads is as dbt writes it.
    const CORE: &[(&str, &str, &str)] = &[
        ("dbt", "materialization_incremental_default", "{% materialization incremental, default -%}
  {% set incremental_strategy = config.get('incremental_strategy') or 'default' %}
  {% set strategy_sql_macro_func = adapter.get_incremental_strategy_macro(context, incremental_strategy) %}
{%- endmaterialization %}"),
        ("dbt", "get_incremental_default_sql", "{% macro get_incremental_default_sql(arg_dict) %}
  {{ return(adapter.dispatch('get_incremental_default_sql', 'dbt')(arg_dict)) }}
{% endmacro %}"),
        ("dbt", "default__get_incremental_default_sql", "{% macro default__get_incremental_default_sql(arg_dict) %}

  {% do return(get_incremental_append_sql(arg_dict)) %}

{% endmacro %}"),
        ("dbt", "get_incremental_merge_sql", "{% macro get_incremental_merge_sql(arg_dict) %}
  {{ return(adapter.dispatch('get_incremental_merge_sql', 'dbt')(arg_dict)) }}
{% endmacro %}"),
        ("dbt", "default__get_incremental_merge_sql", "{% macro default__get_incremental_merge_sql(arg_dict) %}
  {% do return(get_merge_sql(arg_dict[\"target_relation\"], arg_dict[\"temp_relation\"], arg_dict[\"unique_key\"], arg_dict[\"dest_columns\"], arg_dict[\"incremental_predicates\"])) %}
{% endmacro %}"),
        ("dbt", "get_merge_sql", "{% macro get_merge_sql(target, source, unique_key, dest_columns, incremental_predicates=none) -%}
  {{ adapter.dispatch('get_merge_sql', 'dbt')(target, source, unique_key, dest_columns, incremental_predicates) }}
{%- endmacro %}"),
        ("dbt", "default__get_merge_sql", "{% macro default__get_merge_sql(target, source, unique_key, dest_columns, incremental_predicates=none) -%}
    {% if unique_key %}
        {% do predicates.append(unique_key_match) %}
    {% else %}
        {% do predicates.append('FALSE') %}
    {% endif %}
    merge into {{ target }} as DBT_INTERNAL_DEST
{% endmacro %}"),
        ("dbt", "get_incremental_delete_insert_sql", "{% macro get_incremental_delete_insert_sql(arg_dict) %}
  {{ return(adapter.dispatch('get_incremental_delete_insert_sql', 'dbt')(arg_dict)) }}
{% endmacro %}"),
        ("dbt", "default__get_incremental_delete_insert_sql", "{% macro default__get_incremental_delete_insert_sql(arg_dict) %}
  {% do return(get_delete_insert_merge_sql(arg_dict[\"target_relation\"], arg_dict[\"temp_relation\"], arg_dict[\"unique_key\"], arg_dict[\"dest_columns\"], arg_dict[\"incremental_predicates\"])) %}
{% endmacro %}"),
        ("dbt", "get_delete_insert_merge_sql", "{% macro get_delete_insert_merge_sql(target, source, unique_key, dest_columns, incremental_predicates) -%}
  {{ adapter.dispatch('get_delete_insert_merge_sql', 'dbt')(target, source, unique_key, dest_columns, incremental_predicates) }}
{%- endmacro %}"),
        ("dbt", "default__get_delete_insert_merge_sql", "{% macro default__get_delete_insert_merge_sql(target, source, unique_key, dest_columns, incremental_predicates) -%}
    {% if unique_key %}
        delete from {{ target }} as DBT_INTERNAL_DEST
        where ({{ unique_key_str }}) in (select distinct {{ unique_key_str }} from {{ source }})
    {% endif %}
    insert into {{ target }} ({{ dest_cols_csv }})
{%- endmacro %}"),
    ];

    const SNOWFLAKE: &[(&str, &str, &str)] = &[
        ("dbt_snowflake", "materialization_incremental_snowflake", "{% materialization incremental, adapter='snowflake', supported_languages=['sql', 'python'] -%}
  {#-- The temp relation will be a view, temp table, or transient table, depending on strategy and config --#}
  {%- set unique_key = config.get('unique_key') -%}
  {% set incremental_strategy = config.get('incremental_strategy') or 'default' %}
{%- endmaterialization %}"),
        ("dbt_snowflake", "snowflake__get_incremental_default_sql", "{% macro snowflake__get_incremental_default_sql(arg_dict) %}
  {{ return(get_incremental_merge_sql(arg_dict)) }}
{% endmacro %}"),
        ("dbt_snowflake", "snowflake__get_merge_sql", "{% macro snowflake__get_merge_sql(target, source_sql, unique_key, dest_columns, incremental_predicates) -%}
    {%- set dml -%}
    {%- if unique_key is none -%}
        insert into {{ target }} ({{ dest_cols_csv }})
    {%- else -%}
        {{ default__get_merge_sql(target, source_sql, unique_key, dest_columns, incremental_predicates) }}
    {%- endif -%}
    {%- endset -%}
    {% do return(snowflake_dml_explicit_transaction(dml)) %}
{% endmacro %}"),
        ("dbt_snowflake", "snowflake__get_delete_insert_merge_sql", "{% macro snowflake__get_delete_insert_merge_sql(target, source, unique_key, dest_columns, incremental_predicates) %}
    {% set dml = default__get_delete_insert_merge_sql(target, source, unique_key, dest_columns, incremental_predicates) %}
    {% do return(snowflake_dml_explicit_transaction(dml)) %}
{% endmacro %}"),
    ];

    const DUCKDB: &[(&str, &str, &str)] = &[
        ("dbt_duckdb", "materialization_incremental_duckdb", "{% materialization incremental, adapter=\"duckdb\", supported_languages=['sql', 'python'] -%}
    {% set incremental_strategy = config.get('incremental_strategy') or 'default' %}
{%- endmaterialization %}"),
        ("dbt_duckdb", "duckdb__get_incremental_default_sql", "{% macro duckdb__get_incremental_default_sql(arg_dict) %}
  {% do return(get_incremental_delete_insert_sql(arg_dict)) %}
{% endmacro %}"),
        ("dbt_duckdb", "duckdb__get_delete_insert_merge_sql", "{% macro duckdb__get_delete_insert_merge_sql(target, source, unique_key, dest_columns, incremental_predicates) -%}
    {% if unique_key %}
        delete from {{ target }}
    {% endif %}
    insert into {{ target }} ({{ dest_cols_csv }})
{%- endmacro %}"),
    ];

    /// dbt's own macros and `more`, each replacing any of the same package and name.
    fn kit(more: &[&[(&str, &str, &str)]]) -> Macros {
        let mut all = serde_json::Map::new();
        for (package, name, body) in CORE.iter().chain(more.iter().flat_map(|m| m.iter())) {
            all.insert(
                format!("macro.{package}.{name}"),
                serde_json::json!({ "name": name, "package_name": package, "macro_sql": body }),
            );
        }
        let raw: HashMap<String, RawMacro> = serde_json::from_value(serde_json::Value::Object(all)).unwrap();
        Macros::build(raw, "shop")
    }

    fn default_of(m: &Macros, adapter: &str, dispatch: &DispatchOrder) -> Option<String> {
        Incremental::new(m, adapter, dispatch).default_strategy("shop")
    }

    #[test]
    fn a_key_is_none_falsy_or_given_as_jinja_reads_it() {
        use serde_json::json;
        let of = |v: serde_json::Value| Key::of(Some(&v));
        assert_eq!(Key::of(None), Key::None);
        assert_eq!(of(json!(null)), Key::None);
        for falsy in [json!(""), json!([]), json!({}), json!(false), json!(0), json!(0.0)] {
            assert_eq!(of(falsy.clone()), Key::Falsy, "{falsy}");
        }
        for given in [json!("id"), json!(["id"]), json!([[]]), json!({"a": 1}), json!(true), json!(1)] {
            assert_eq!(of(given.clone()), Key::Given, "{given}");
        }
    }

    #[test]
    fn only_the_bodies_that_bear_on_a_strategy_are_kept() {
        let m = kit(&[SNOWFLAKE, &[("shop", "cents", "{% macro cents(x) %}{{ x }} / 100{% endmacro %}")]]);
        assert!(m.resolve("cents", "shop").unwrap().body.is_empty());
        assert!(m.resolve("dbt_snowflake.snowflake__get_merge_sql", "shop").unwrap().body.contains("is none"));
    }

    #[test]
    fn the_default_is_the_one_the_adapter_s_macros_name() {
        let none = DispatchOrder::Default;
        assert_eq!(default_of(&kit(&[SNOWFLAKE]), "snowflake", &none).as_deref(), Some("merge"));
        assert_eq!(default_of(&kit(&[DUCKDB]), "duckdb", &none).as_deref(), Some("delete+insert"));
        // An adapter with no version of its own reaches dbt's `default__`.
        assert_eq!(default_of(&kit(&[]), "sqlite", &none).as_deref(), Some("append"));
        // A manifest that names no adapter names no default either.
        assert_eq!(default_of(&kit(&[SNOWFLAKE]), "", &none), None);
    }

    #[test]
    fn a_materialization_may_name_the_default_outright_or_one_call_deep() {
        let none = DispatchOrder::Default;
        let bigquery: &[(&str, &str, &str)] = &[
            ("dbt_bigquery", "materialization_incremental_bigquery", "{% materialization incremental, adapter='bigquery' -%}
  {%- set strategy = dbt_bigquery_validate_get_incremental_strategy(config) -%}
{%- endmaterialization %}"),
            ("dbt_bigquery", "dbt_bigquery_validate_get_incremental_strategy", "{% macro dbt_bigquery_validate_get_incremental_strategy(config) %}
  {#-- Find and validate the incremental strategy --#}
  {%- set strategy = config.get(\"incremental_strategy\") or 'merge' -%}
  {% do return(strategy) %}
{% endmacro %}"),
        ];
        assert_eq!(default_of(&kit(&[bigquery]), "bigquery", &none).as_deref(), Some("merge"));
        let outright: &[(&str, &str, &str)] = &[("dbt_spark", "materialization_incremental_spark", "{% materialization incremental, adapter='spark' -%}
  {%- set raw_strategy = config.get('incremental_strategy') or 'append' -%}
{%- endmaterialization %}")];
        assert_eq!(default_of(&kit(&[outright]), "spark", &none).as_deref(), Some("append"));
    }

    #[test]
    fn a_materialization_of_the_project_s_own_leaves_the_default_unnamed() {
        let none = DispatchOrder::Default;
        let own: &[(&str, &str, &str)] = &[("shop", "materialization_incremental_snowflake", "{% materialization incremental, adapter='snowflake' -%}
  {% set incremental_strategy = config.get('incremental_strategy') or 'default' %}
{%- endmaterialization %}")];
        assert_eq!(default_of(&kit(&[SNOWFLAKE, own]), "snowflake", &none), None);
        // Nor does a configured merge claim anything under it, which may do
        // anything with the key.
        assert_eq!(inserts(&kit(&[SNOWFLAKE, own]), "snowflake", &none, "merge", Key::None), None);
        // dbt ranks an adapter's own above any default one, wherever it is
        // written, so a project's `default` materialization changes nothing.
        let fallback: &[(&str, &str, &str)] = &[("shop", "materialization_incremental_default", "{% materialization incremental, default -%}
{%- endmaterialization %}")];
        assert_eq!(default_of(&kit(&[SNOWFLAKE, fallback]), "snowflake", &none).as_deref(), Some("merge"));
        // A materialization that reads the configured strategy some other way.
        let other: &[(&str, &str, &str)] = &[("dbt_snowflake", "materialization_incremental_snowflake", "{% materialization incremental, adapter='snowflake' -%}
  {% set incremental_strategy = config.get('incremental_strategy', 'default') %}
{%- endmaterialization %}")];
        assert_eq!(default_of(&kit(&[SNOWFLAKE, other]), "snowflake", &none), None);
    }

    #[test]
    fn a_root_project_macro_named_like_dbt_s_wins() {
        let own: &[(&str, &str, &str)] = &[("shop", "get_incremental_default_sql", "{% macro get_incremental_default_sql(arg_dict) %}
  {% do return(get_incremental_append_sql(arg_dict)) %}
{% endmacro %}")];
        assert_eq!(default_of(&kit(&[SNOWFLAKE, own]), "snowflake", &DispatchOrder::Default).as_deref(), Some("append"));
    }

    #[test]
    fn the_dispatch_searches_each_package_by_every_prefix_before_the_next() {
        // The root project's `default__` comes before Snowflake's own version,
        // since dbt walks the packages first and the prefixes within each.
        let own: &[(&str, &str, &str)] = &[("shop", "default__get_incremental_default_sql", "{% macro default__get_incremental_default_sql(arg_dict) %}
  {% do return(get_incremental_delete_insert_sql(arg_dict)) %}
{% endmacro %}")];
        let m = kit(&[SNOWFLAKE, own]);
        assert_eq!(default_of(&m, "snowflake", &DispatchOrder::Default).as_deref(), Some("delete+insert"));
        let dbt_first = DispatchOrder::Given(vec!["dbt".into(), "shop".into()]);
        assert_eq!(default_of(&m, "snowflake", &dbt_first).as_deref(), Some("merge"));
    }

    #[test]
    fn a_parent_adapter_is_searched_after_the_adapter_and_two_leave_it_unnamed() {
        let none = DispatchOrder::Default;
        let redshift: &[(&str, &str, &str)] = &[
            ("dbt_redshift", "redshift__create_table_as", "{% macro redshift__create_table_as() %}{% endmacro %}"),
            ("dbt_postgres", "postgres__create_table_as", "{% macro postgres__create_table_as() %}{% endmacro %}"),
            ("dbt_postgres", "postgres__get_incremental_default_sql", "{% macro postgres__get_incremental_default_sql(arg_dict) %}
  {% do return(get_incremental_delete_insert_sql(arg_dict)) %}
{% endmacro %}"),
            // Named like an adapter's package, with a macro by that prefix,
            // and no adapter all the same.
            ("dbt_utils", "utils__star", "{% macro utils__star() %}{% endmacro %}"),
        ];
        assert_eq!(default_of(&kit(&[redshift]), "redshift", &none).as_deref(), Some("delete+insert"));
        let second: &[(&str, &str, &str)] = &[("dbt_spark", "spark__create_table_as", "{% macro spark__create_table_as() %}{% endmacro %}")];
        assert_eq!(default_of(&kit(&[redshift, second]), "redshift", &none), None);
    }

    #[test]
    fn a_project_named_like_an_adapter_s_package_is_no_parent() {
        // Taken for a parent, its materialization would pass for dbt's own.
        let mut all = serde_json::Map::new();
        let own = [
            ("dbt_vault", "vault__create_table_as", "{% macro vault__create_table_as() %}{% endmacro %}"),
            ("dbt_vault", "materialization_incremental_snowflake", "{% materialization incremental, adapter='snowflake' -%}
  {% set incremental_strategy = config.get('incremental_strategy') or 'append' %}
{%- endmaterialization %}"),
        ];
        for (package, name, body) in CORE.iter().chain(SNOWFLAKE).chain(own.iter()) {
            all.insert(format!("macro.{package}.{name}"), serde_json::json!({ "name": name, "package_name": package, "macro_sql": body }));
        }
        let raw: HashMap<String, RawMacro> = serde_json::from_value(serde_json::Value::Object(all)).unwrap();
        let m = Macros::build(raw, "dbt_vault");
        let d = DispatchOrder::Default;
        assert_eq!(Incremental::new(&m, "snowflake", &d).default_strategy("dbt_vault"), None);
        assert_eq!(Incremental::new(&m, "snowflake", &d).inserts("merge", "dbt_vault", Key::None), None);
    }

    #[test]
    fn an_unreadable_dispatch_matters_only_when_a_package_has_a_version() {
        let unreadable = DispatchOrder::Unreadable;
        assert_eq!(default_of(&kit(&[SNOWFLAKE]), "snowflake", &unreadable).as_deref(), Some("merge"));
        let package: &[(&str, &str, &str)] = &[("vault", "snowflake__get_incremental_default_sql", "{% macro snowflake__get_incremental_default_sql(arg_dict) %}
  {{ return(get_incremental_append_sql(arg_dict)) }}
{% endmacro %}")];
        let m = kit(&[SNOWFLAKE, package]);
        assert_eq!(default_of(&m, "snowflake", &unreadable), None);
        // Read, the order says where the package goes.
        assert_eq!(default_of(&m, "snowflake", &DispatchOrder::Default).as_deref(), Some("merge"));
        let package_first = DispatchOrder::Given(vec!["vault".into(), "dbt".into()]);
        assert_eq!(default_of(&m, "snowflake", &package_first).as_deref(), Some("append"));
        // The project's own version counts too: an order read wrong could
        // put dbt's before it.
        let own: &[(&str, &str, &str)] = &[("shop", "snowflake__get_incremental_default_sql", "{% macro snowflake__get_incremental_default_sql(arg_dict) %}
  {{ return(get_incremental_append_sql(arg_dict)) }}
{% endmacro %}")];
        assert_eq!(default_of(&kit(&[SNOWFLAKE, own]), "snowflake", &unreadable), None);
    }

    #[test]
    fn an_adapter_s_package_named_in_the_order_is_passed_over_as_dbt_does() {
        let own: &[(&str, &str, &str)] = &[("shop", "default__get_merge_sql", "{% macro default__get_merge_sql(target, source, unique_key, dest_columns) %}
    {% if unique_key %}{% else %}{% do predicates.append('FALSE') %}{% endif %}
{% endmacro %}")];
        let order = DispatchOrder::Given(vec!["dbt_snowflake".into(), "shop".into(), "dbt".into()]);
        // The project's version comes first, and claims nothing.
        assert_eq!(inserts(&kit(&[SNOWFLAKE, own]), "snowflake", &order, "merge", Key::None), None);
    }

    #[test]
    fn a_default_is_named_by_a_body_that_is_one_return_only() {
        let body = |inner: &str| format!("{{% macro x__get_incremental_default_sql(arg_dict) %}}\n  {inner}\n{{% endmacro %}}");
        let named = |inner: &str| returned_strategy(&body(inner));
        assert_eq!(named("{{ return(get_incremental_merge_sql(arg_dict)) }}").as_deref(), Some("merge"));
        assert_eq!(named("{%- do return(get_incremental_delete_insert_sql(arg_dict)) -%}").as_deref(), Some("delete+insert"));
        assert_eq!(named("{# the adapter's choice #}{{ return(get_incremental_append_sql(arg_dict)) }}").as_deref(), Some("append"));
        // Postgres's form: a condition on the key picks one or the other.
        assert_eq!(named("{% if arg_dict[\"unique_key\"] %}
    {% do return(get_incremental_delete_insert_sql(arg_dict)) %}
  {% else %}
    {% do return(get_incremental_append_sql(arg_dict)) %}
  {% endif %}"), None);
        assert_eq!(named("{{ log('x') }}{{ return(get_incremental_merge_sql(arg_dict)) }}"), None);
        assert_eq!(named("{{ return(get_incremental_merge_sql(arg_dict) ~ ';') }}"), None);
        assert_eq!(named("{{ return(get_merge_sql(arg_dict)) }}"), None);
        assert_eq!(named("{{ return(get_incremental_merge_sql(arg_dict)) | trim }}"), None);
    }

    fn inserts(m: &Macros, adapter: &str, dispatch: &DispatchOrder, strategy: &str, key: Key) -> Option<&'static str> {
        Incremental::new(m, adapter, dispatch).inserts(strategy, "shop", key)
    }

    #[test]
    fn snowflake_s_merge_inserts_when_the_key_is_none_and_only_then() {
        let (m, d) = (kit(&[SNOWFLAKE]), DispatchOrder::Default);
        assert_eq!(inserts(&m, "snowflake", &d, "merge", Key::None), Some(SNOWFLAKE_MERGE));
        // `[]` and `''` are not none: Snowflake's macro hands them to dbt's merge.
        assert_eq!(inserts(&m, "snowflake", &d, "merge", Key::Falsy), None);
        assert_eq!(inserts(&m, "snowflake", &d, "merge", Key::Given), None);
        // Its delete+insert wraps dbt's, which deletes nothing for a falsy key.
        assert_eq!(inserts(&m, "snowflake", &d, "delete+insert", Key::None), Some(DEFAULT_DELETE_INSERT));
        assert_eq!(inserts(&m, "snowflake", &d, "delete+insert", Key::Falsy), Some(DEFAULT_DELETE_INSERT));
        assert_eq!(inserts(&m, "snowflake", &d, "delete+insert", Key::Given), None);
        assert_eq!(inserts(&m, "snowflake", &d, "append", Key::None), None);
    }

    #[test]
    fn dbt_s_own_merge_inserts_when_the_key_is_falsy() {
        let (m, d) = (kit(&[]), DispatchOrder::Default);
        assert_eq!(inserts(&m, "sqlite", &d, "merge", Key::None), Some(DEFAULT_MERGE));
        assert_eq!(inserts(&m, "sqlite", &d, "merge", Key::Falsy), Some(DEFAULT_MERGE));
        assert_eq!(inserts(&m, "sqlite", &d, "merge", Key::Given), None);
    }

    #[test]
    fn any_other_macro_on_the_way_makes_no_claim() {
        let d = DispatchOrder::Default;
        let none = |more: &[(&str, &str, &str)], dispatch: &DispatchOrder| {
            inserts(&kit(&[SNOWFLAKE, more]), "snowflake", dispatch, "merge", Key::None)
        };
        let own = |name: &str| format!("{{% macro {name}(target, source, unique_key, dest_columns) %}}merge into {{{{ target }}}}{{% endmacro %}}");
        let (a, b, c) = (own("snowflake__get_merge_sql"), own("get_merge_sql"), own("get_incremental_merge_sql"));
        assert_eq!(none(&[("shop", "snowflake__get_merge_sql", &a)], &d), None);
        assert_eq!(none(&[("shop", "get_merge_sql", &b)], &d), None);
        assert_eq!(none(&[("shop", "get_incremental_merge_sql", &c)], &d), None);
        // A package's is reached only where `dispatch:` puts it.
        let package = [("vault", "snowflake__get_merge_sql", a.as_str())];
        assert_eq!(none(&package, &d), Some(SNOWFLAKE_MERGE));
        assert_eq!(none(&package, &DispatchOrder::Given(vec!["shop".into(), "vault".into(), "dbt".into()])), None);
        // Snowflake's, changed under the same name, no longer tests for none.
        let changed = "{% macro snowflake__get_merge_sql(target, source_sql, unique_key, dest_columns) -%}
    {{ default__get_merge_sql(target, source_sql, unique_key, dest_columns) }}
{% endmacro %}";
        assert_eq!(none(&[("dbt_snowflake", "snowflake__get_merge_sql", changed)], &d), None);
        // DuckDB's own macros are not among the known ones.
        let duck = kit(&[DUCKDB]);
        assert_eq!(inserts(&duck, "duckdb", &d, "delete+insert", Key::None), None);
    }

    #[test]
    fn each_incremental_box_shows_what_dbt_runs() {
        let mut macros = serde_json::Map::new();
        for (package, name, body) in CORE.iter().chain(SNOWFLAKE) {
            macros.insert(
                format!("macro.{package}.{name}"),
                serde_json::json!({ "name": name, "package_name": package, "macro_sql": body }),
            );
        }
        let model = |name: &str, config: serde_json::Value| {
            serde_json::json!({ "name": name, "resource_type": "model", "package_name": "shop", "config": config })
        };
        let raw: RawManifest = serde_json::from_value(serde_json::json!({
            "metadata": { "project_name": "shop", "adapter_type": "snowflake" },
            "nodes": {
                "model.shop.keyless": model("keyless", serde_json::json!({ "materialized": "incremental" })),
                "model.shop.keyed": model("keyed", serde_json::json!({
                    "materialized": "incremental", "unique_key": ["order_id", "line_number"],
                })),
                "model.shop.deletes": model("deletes", serde_json::json!({
                    "materialized": "incremental", "incremental_strategy": "delete+insert", "unique_key": "order_id",
                })),
                "model.shop.appends": model("appends", serde_json::json!({
                    "materialized": "incremental", "incremental_strategy": "append",
                })),
                "model.shop.empty_list": model("empty_list", serde_json::json!({
                    "materialized": "incremental", "incremental_strategy": "merge", "unique_key": [],
                })),
                "model.shop.inherits": model("inherits", serde_json::json!({
                    "materialized": "view", "incremental_strategy": "merge", "unique_key": "order_id",
                })),
                "model.shop.said_default": model("said_default", serde_json::json!({
                    "materialized": "incremental", "incremental_strategy": "default", "unique_key": false,
                })),
                "model.shop.nested": model("nested", serde_json::json!({
                    "materialized": "incremental", "unique_key": [["order_id", "line_number"], "day"],
                })),
            },
            "disabled": {
                "model.shop.off": [model("off", serde_json::json!({ "materialized": "incremental", "enabled": false }))],
            },
            "macros": macros,
        }))
        .unwrap();
        let mut g = crate::graph::Graph::build(raw, Path::new("manifest.json"), 0, 0);
        g.resolve_incremental(&crate::project::DispatchOrder::Default);
        let at = |name: &str| g.nodes.iter().find(|n| n.name == name).unwrap();
        let shown = |name: &str| {
            let f = at(name).strategy_facts();
            (f.strategy_shown, f.strategy_named, f.strategy_default, f.strategy_insert)
        };
        assert_eq!(shown("keyless"), ("append", "merge", true, SNOWFLAKE_MERGE));
        assert_eq!(shown("keyed"), ("merge", "", true, ""));
        assert_eq!(at("keyed").unique_key, ["order_id", "line_number"]);
        assert_eq!(shown("deletes"), ("delete+insert", "", false, ""));
        assert_eq!(shown("appends"), ("append", "", false, ""));
        // Not none, so Snowflake merges, on FALSE.
        assert_eq!(shown("empty_list"), ("merge", "", false, ""));
        assert_eq!(at("empty_list").key, Key::Falsy);
        // `'default'` configured goes where the fallback goes; `false` is falsy
        // to Jinja but not none, so Snowflake's merge makes no claim on it.
        assert_eq!(shown("said_default"), ("merge", "", true, ""));
        assert_eq!(at("said_default").key, Key::Falsy);
        // An entry that is no string reads as its JSON, not as no key at all.
        assert_eq!(at("nested").unique_key, [r#"["order_id","line_number"]"#, "day"]);
        assert_eq!(shown("nested"), ("merge", "", true, ""));
        // A disabled model is never run.
        assert_eq!(shown("off"), ("", "", false, ""));
        // A view keeps what it inherited, for the selector, and shows nothing.
        assert_eq!(shown("inherits"), ("", "", false, ""));
        assert_eq!((at("inherits").strategy.as_str(), &at("inherits").unique_key[..]), ("merge", &["order_id".to_string()][..]));

        // The box carries all of it, and a view nothing.
        let focus = g.index["model.shop.keyless"];
        let lineage = serde_json::to_value(g.lineage(focus, 0, 0, false, 10)).unwrap();
        let node = &lineage["nodes"][0];
        assert_eq!(node["strategy_shown"], "append");
        assert_eq!(node["strategy_named"], "merge");
        assert_eq!(node["strategy_default"], true);
        assert_eq!(node["strategy_insert"], SNOWFLAKE_MERGE);
        assert!(node.get("unique_key").is_none());
        let view = serde_json::to_value(g.lineage(g.index["model.shop.inherits"], 0, 0, false, 10)).unwrap();
        for field in ["strategy_shown", "strategy_named", "strategy_default", "strategy_insert", "unique_key"] {
            assert!(view["nodes"][0].get(field).is_none(), "{field}");
        }
        let keyed = serde_json::to_value(g.lineage(g.index["model.shop.keyed"], 0, 0, false, 10)).unwrap();
        assert_eq!(keyed["nodes"][0]["unique_key"], serde_json::json!(["order_id", "line_number"]));
    }
}
