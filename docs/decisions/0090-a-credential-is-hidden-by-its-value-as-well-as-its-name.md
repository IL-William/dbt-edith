# 0090. A credential is hidden by its value as well as its name

Date: 2026-10-15 · Status: accepted · Amends 0019 and 0054

**Trigger:** read before changing what the hover card or the profile form
hides, or the vocabulary `sensitive_name` and `secret_key` share.

## Context

0019 let a resolved `env_var()` reach the hover card under two guards: a
`DBT_ENV_SECRET_*` is never substituted, and a name that reads as a credential
returns no value. 0054 made that vocabulary the profile form's. A review on
2026-10-15 found the vocabulary short of `DB_PASS`, `DATABASE_URL` and
`SNOWFLAKE_DSN`, found a password shown in a URL under a name that said
nothing, `REPORTS_URL`, and found the profile's redaction blind to a value
written on the line after its key.

## Decision

- **A third guard, on the value.** A value carrying a password, in the
  userinfo of a URL or as a `password=` parameter wherever a connection string
  puts one, is hidden whatever the variable or key is called: an environment
  variable, a resolved `var()`, a literal one and each item of a list one. A
  hidden var keeps nothing of its text, not even the expression, whose default
  may be the secret. The card says which of the name or the value gave it
  away.
- **The vocabulary grows by what a connection is called**: `PASS` and `DSN`
  as the last token after a prefix, and `URL`, `URI`, `STR` or `STRING` after
  `DATABASE`, `DB`, `CONNECTION` or `CONN`. Not alone: `PASS_RATE`, Fivetran's
  `_pass_through_columns` and dbt-exasol's `dsn` key stay in clear.
- **The profile's value is where the line scanner can follow YAML**: on the
  key's line, or on the first line below that is not a comment, comments at
  any depth between them allowed, a quoted scalar included. A flow mapping on
  one line is not taken apart: one holding a secret key is hidden whole, and
  put back whole. A hidden block saved under a key that moved moves with it.

## Rejected

- **Hiding every value with a URL in it.** A place is not a secret; only the
  password in it is.
- **Hiding by name alone, as before.** No list of names is complete, and the
  value says so itself.
