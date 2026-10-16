//! The profile's secrets, kept on the server while the profile is edited.
//!
//! `profiles.yml` is editable from the page (0017), and it can hold a
//! warehouse password, a token or a private key. Those values never leave:
//! `hide` puts `HIDDEN` in their place before the profile is sent, and `restore`
//! puts the values on disk back where the saved text still says `HIDDEN`
//! (0054). Everything else in the file reaches the page as it is, so the
//! account, the role or a target's name can still be corrected there.
//!
//! A line scanner, not a YAML parser (0003, 0018), shared with `profiles`,
//! which reads the active target with it: a key is found by its indentation
//! and the keys above it, and a value runs to the next line that is not
//! indented further. That is every profile dbt's documentation shows.
//! A flow mapping, `{ password: x }` on one line, is beyond it: one holding a
//! secret key is hidden whole rather than taken apart.

use crate::profiles::{indent, key_line, lines};
use std::collections::HashMap;

/// What the page shows in place of a secret. Saved back unchanged, it means
/// "keep the value on disk"; anything else typed over it is the new value.
pub const HIDDEN: &str = "<hidden by dbt-edith>";

/// Whether a profile key holds a credential. The same vocabulary as the
/// environment variables the hover card guards (0019, 0090), plus `pass` on
/// its own, which dbt-postgres takes for `password` and which that vocabulary
/// takes only after a prefix, as in `db_pass`. A key naming where a secret is
/// kept, such as `private_key_path`, is not one, and neither is dbt-exasol's
/// `dsn`, a host and a port.
pub fn secret_key(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    if k.ends_with("_path") || k.ends_with("_file") {
        return false;
    }
    k == "pass" || crate::envs::sensitive_name(&k)
}

/// A secret found in a profile: where its value is in the text, and the path
/// of keys that leads to it, which is how a saved profile finds it again.
struct Found {
    path: Vec<String>,
    /// Line of the key, and the last line its value runs to.
    first: usize,
    last: usize,
    /// Byte in the key's line just past the colon.
    at: usize,
    /// The key's indentation, which the lines of its value all exceed.
    depth: usize,
}

/// Every secret value in a profile, in order.
fn secrets(all: &[(&str, &str)]) -> Vec<Found> {
    let mut out = Vec::new();
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut i = 0;
    while i < all.len() {
        let body = all[i].0;
        let Some((key, at, depth)) = key_line(body) else {
            i += 1;
            continue;
        };
        while stack.last().is_some_and(|(d, _)| *d >= depth) {
            stack.pop();
        }
        stack.push((depth, key.clone()));
        // The value runs on over every following line indented further, and
        // the blank lines between them.
        let mut last = i;
        let mut j = i + 1;
        while j < all.len() {
            let next = all[j].0;
            // A blank line, or a comment at any indentation, runs on: YAML
            // reads past both, so `password:` with `# rotated` at the key's
            // own depth and `hunter2` beneath is still the password.
            if next.trim().is_empty() || next.trim_start().starts_with('#') {
                j += 1;
                continue;
            }
            if indent(next) <= depth {
                break;
            }
            last = j;
            j += 1;
        }
        let inline = body[at..].trim();
        // YAML lets a value start on the line after its key, indented further:
        // `password:` with `hunter2` beneath it, a comment line or two between
        // them allowed. With nothing on the key's line but a comment, the
        // first line below that is not one is what says what the key holds,
        // so it is judged in the inline's place. A line that is itself a key
        // opens a mapping instead, whose keys are judged on their own lines.
        let below = (i + 1..=last)
            .map(|k| all[k].0)
            .find(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'));
        let judged = match below {
            Some(line) if (inline.is_empty() || inline.starts_with('#')) && !opens_mapping(line) => line.trim(),
            _ => inline,
        };
        // `password:` alone opens a mapping, whose keys are judged on their own
        // lines. An `env_var()` holds no secret, only where one is, and showing
        // it is the point of editing the file. An empty value hides nothing,
        // and neither does a switch such as `passcode_in_password: true`.
        let worth_hiding = flow_holds_secret(judged)
            || secret_key(&key)
            && !judged.is_empty()
            && !judged.starts_with('#')
            && !judged.contains("{{")
            && !matches!(judged, "''" | "\"\"")
            && !judged.eq_ignore_ascii_case("true")
            && !judged.eq_ignore_ascii_case("false");
        if worth_hiding {
            out.push(Found { path: stack.iter().map(|(_, k)| k.clone()).collect(), first: i, last, at, depth });
            i = last + 1;
        } else {
            i += 1;
        }
    }
    out
}

/// A flow mapping on the key's line that holds a secret key,
/// `{type: postgres, user: me, password: hunter2}`, nested or not: the scanner
/// does not take it apart (0054), so it is hidden whole rather than sent as
/// it is, and put back whole by the key it hangs from.
fn flow_holds_secret(inline: &str) -> bool {
    let Some(inner) = inline.strip_prefix('{') else { return false };
    inner.split(|c| c == ',' || c == '{').any(|pair| {
        pair.split_once(':').is_some_and(|(k, _)| secret_key(k.trim().trim_matches(|c| c == '\'' || c == '"')))
    })
}

/// Whether a line under a key is a key of its own rather than the value. A
/// quoted key is one when its closing quote is followed by a colon, `'a b': x`;
/// `'ab: cd'` is a value whatever it holds, which `key_line` alone would read
/// as a key named `ab`.
fn opens_mapping(line: &str) -> bool {
    let t = line.trim_start();
    match t.chars().next() {
        Some(q @ ('\'' | '"')) => t[1..].find(q).is_some_and(|end| t[end + 2..].trim_start().starts_with(':')),
        _ => key_line(line).is_some(),
    }
}

/// The text of a value as it stands in the file, from just past the colon to
/// the end of its last line, line endings between included.
fn value_text(all: &[(&str, &str)], f: &Found) -> String {
    let mut text = all[f.first].0[f.at..].to_string();
    for k in f.first..f.last {
        text.push_str(all[k].1);
        text.push_str(all[k + 1].0);
    }
    text
}

/// A value's text with the lines after its first moved from under a key at
/// depth `from` to one at depth `to`. A block put back from disk under a key
/// the saved profile re-indented has to stay deeper than that key: level
/// with it or above, YAML reads its lines as the key's siblings, and the next
/// showing would send them in clear.
fn shifted(value: &str, from: usize, to: usize) -> String {
    if from == to {
        return value.to_string();
    }
    let mut lines = value.split('\n');
    let mut out = lines.next().unwrap_or("").to_string();
    for line in lines {
        out.push('\n');
        let body = line.trim_start_matches(' ');
        if !body.trim_end_matches('\r').is_empty() {
            let had = line.len() - body.len();
            out.push_str(&" ".repeat((had + to).saturating_sub(from)));
        }
        out.push_str(body);
    }
    out
}

/// The profile as the page may see it, and how many values were hidden.
pub fn hide(profile: &str) -> (String, usize) {
    let all = lines(profile);
    let found = secrets(&all);
    let mut out = String::with_capacity(profile.len());
    let mut i = 0;
    for f in &found {
        for (body, end) in &all[i..f.first] {
            out.push_str(body);
            out.push_str(end);
        }
        out.push_str(&all[f.first].0[..f.at]);
        out.push(' ');
        out.push_str(HIDDEN);
        out.push_str(all[f.last].1);
        i = f.last + 1;
    }
    for (body, end) in &all[i..] {
        out.push_str(body);
        out.push_str(end);
    }
    (out, found.len())
}

/// The profile as it will be written: every value the page left as `HIDDEN`
/// is the one on disk again, found by the keys that lead to it. A hidden value
/// whose keys no longer lead anywhere on disk, a target renamed for instance,
/// is refused rather than written as the placeholder it is.
pub fn restore(saved: &str, on_disk: &str) -> Result<String, String> {
    let disk = lines(on_disk);
    let mut kept: HashMap<Vec<String>, (String, usize)> = HashMap::new();
    for f in secrets(&disk) {
        kept.entry(f.path.clone()).or_insert_with(|| (value_text(&disk, &f), f.depth));
    }
    let all = lines(saved);
    let mut out = String::with_capacity(saved.len() + 64);
    let mut stack: Vec<(usize, String)> = Vec::new();
    for (body, end) in &all {
        if let Some((key, at, depth)) = key_line(body) {
            while stack.last().is_some_and(|(d, _)| *d >= depth) {
                stack.pop();
            }
            stack.push((depth, key));
            if body[at..].trim().starts_with(HIDDEN) {
                let path: Vec<String> = stack.iter().map(|(_, k)| k.clone()).collect();
                let Some((value, was)) = kept.get(&path) else {
                    return Err(format!(
                        "{} was hidden, and the profile on disk has no value there to put back: type the value itself",
                        path.join(".")
                    ));
                };
                out.push_str(&body[..at]);
                out.push_str(&shifted(value, *was, depth));
                out.push_str(end);
                continue;
            }
        }
        out.push_str(body);
        out.push_str(end);
    }
    if out.contains(HIDDEN) {
        return Err(format!("{HIDDEN} stands where no hidden value was: type the value itself"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: &str = "\
shop:
  target: dev
  outputs:
    dev:
      type: snowflake
      account: ab12345.eu-west-1
      user: ANALYST
      password: 'hunter2'   # rotated in May
      role: TRANSFORMER
      private_key_path: /home/me/.ssh/rsa_key.p8
    prod:
      type: snowflake
      user: SVC_DBT
      password: \"{{ env_var('SF_PROD_PASSWORD') }}\"
      private_key: |
        -----BEGIN PRIVATE KEY-----
        MIIEvQIBADANBg
        -----END PRIVATE KEY-----
      private_key_passphrase: s3cret
      passcode_in_password: false
      token:
other:
  outputs:
    dev:
      type: postgres
      pass: letmein
      password: ''
";

    #[test]
    fn secrets_are_hidden_and_nothing_else_is() {
        let (shown, n) = hide(PROFILE);
        assert_eq!(n, 4, "{shown}");
        // The message names the value by its place in the list, never the value:
        // a failing test must not print what it was guarding.
        for (i, value) in ["hunter2", "rotated in May", "MIIEvQIBADANBg", "BEGIN PRIVATE KEY", "s3cret", "letmein"].iter().enumerate() {
            assert!(!shown.contains(value), "hidden value number {i} reached the page");
        }
        for kept in [
            "account: ab12345.eu-west-1",
            "user: ANALYST",
            "role: TRANSFORMER",
            "private_key_path: /home/me/.ssh/rsa_key.p8",
            "password: \"{{ env_var('SF_PROD_PASSWORD') }}\"",
            "      token:\n",
            "passcode_in_password: false",
            "password: ''",
        ] {
            assert!(shown.contains(kept), "{kept} should stay: {shown}");
        }
        assert!(shown.contains(&format!("      password: {HIDDEN}\n      role:")), "{shown}");
        assert!(shown.contains(&format!("      private_key: {HIDDEN}\n      private_key_passphrase: {HIDDEN}\n")), "{shown}");
        assert!(shown.contains(&format!("      pass: {HIDDEN}\n")), "{shown}");
    }

    #[test]
    fn saved_unchanged_the_profile_is_the_one_on_disk() {
        let (shown, _) = hide(PROFILE);
        assert_eq!(restore(&shown, PROFILE).unwrap(), PROFILE);
    }

    #[test]
    fn an_edit_keeps_the_hidden_values_and_a_value_typed_over_one_replaces_it() {
        let (shown, _) = hide(PROFILE);
        let edited = shown
            .replace("role: TRANSFORMER", "role: ANALYST_ROLE")
            .replace(&format!("pass: {HIDDEN}"), "pass: n3w-one");
        let saved = restore(&edited, PROFILE).unwrap();
        assert!(saved.contains("role: ANALYST_ROLE"));
        assert!(saved.contains("password: 'hunter2'   # rotated in May"), "{saved}");
        assert!(saved.contains("pass: n3w-one") && !saved.contains("letmein"), "{saved}");
        assert!(saved.contains("        MIIEvQIBADANBg\n"), "the key's block comes back whole: {saved}");
    }

    #[test]
    fn a_hidden_value_with_nowhere_to_go_is_refused() {
        let (shown, _) = hide(PROFILE);
        // dev renamed to staging: its password has no counterpart on disk.
        let renamed = shown.replace("    dev:\n      type: snowflake", "    staging:\n      type: snowflake");
        let err = restore(&renamed, PROFILE).unwrap_err();
        assert!(err.contains("shop.outputs.staging.password"), "{err}");
        // Pasted under a key that held no secret.
        let pasted = shown.replace("user: ANALYST", &format!("user: {HIDDEN}"));
        assert!(restore(&pasted, PROFILE).is_err());
        let anywhere = format!("{shown}# {HIDDEN}\n");
        assert!(restore(&anywhere, PROFILE).is_err());
    }

    /// YAML lets a value start on the line after its key, indented further,
    /// and a comment on the key's line changes nothing about that.
    #[test]
    fn a_value_on_the_line_after_its_key_is_hidden_too() {
        let profile = "shop:\n  outputs:\n    dev:\n      password:\n        hunter2\n      user: me\n      \
                       token: # set below\n        abc123\n      schema:\n        name: raw\n";
        let (shown, n) = hide(profile);
        assert_eq!(n, 2, "{shown}");
        assert!(!shown.contains("hunter2") && !shown.contains("abc123"), "{shown}");
        assert!(shown.contains(&format!("      password: {HIDDEN}\n      user: me\n")), "{shown}");
        assert!(shown.contains("      schema:\n        name: raw\n"), "a mapping under a key is not a value: {shown}");
        assert_eq!(restore(&shown, profile).unwrap(), profile);
    }

    /// A comment between the key and its value is not the value, and a quoted
    /// scalar with a colon inside is not a mapping: both were sent in clear.
    #[test]
    fn a_comment_or_a_quoted_colon_under_the_key_hides_the_value_still() {
        let profile = "shop:\n  outputs:\n    dev:\n      password:\n        # rotated weekly\n        hunter2\n      \
                       token:\n        'ab: cd'\n      user: me\n      schema:\n        'quoted key': raw\n";
        let (shown, n) = hide(profile);
        assert_eq!(n, 2, "{shown}");
        assert!(!shown.contains("hunter2") && !shown.contains("ab: cd"), "{shown}");
        assert!(shown.contains("      schema:\n        'quoted key': raw\n"), "a quoted key is a key: {shown}");
        assert_eq!(restore(&shown, profile).unwrap(), profile);
    }

    /// A comment at the key's own depth, or less, between the key and its
    /// value: YAML reads past it, and so does this.
    #[test]
    fn a_comment_at_the_keys_depth_between_key_and_value_hides_it_still() {
        let profile = "shop:\n  outputs:\n    dev:\n      password:\n      # rotated\n        hunter2\n      token:\n\
                       # column zero\n        abc123\n      user: me\n      empty:\n      # nothing under\n      host: db\n";
        let (shown, n) = hide(profile);
        assert_eq!(n, 2, "{shown}");
        assert!(!shown.contains("hunter2") && !shown.contains("abc123"), "{shown}");
        assert!(shown.contains("      user: me\n      empty:\n      # nothing under\n      host: db\n"), "{shown}");
        assert_eq!(restore(&shown, profile).unwrap(), profile);
    }

    /// A flow mapping holding a secret key is hidden whole: the scanner does
    /// not take it apart, and sending it as it was sent the password.
    #[test]
    fn a_flow_mapping_with_a_secret_key_is_hidden_whole() {
        let profile = "shop:\n  outputs:\n    dev: {type: postgres, user: me, password: hunter2}\n    ci:\n      \
                       creds: {nested: {\"password\": hunter3}}\n      opts: {threads: 4, retries: 2}\n      host: db\n";
        let (shown, n) = hide(profile);
        assert_eq!(n, 2, "{shown}");
        assert!(!shown.contains("hunter"), "{shown}");
        assert!(shown.contains(&format!("    dev: {HIDDEN}\n")), "{shown}");
        assert!(shown.contains("      opts: {threads: 4, retries: 2}\n      host: db\n"), "{shown}");
        assert_eq!(restore(&shown, profile).unwrap(), profile);
    }

    /// The hole this guards: the placeholder saved under its key moved to a
    /// deeper indentation, so the block put back from disk sat level with the
    /// key, where the next showing no longer read it as the key's value and
    /// sent it in clear. The block now moves with its key.
    #[test]
    fn a_block_put_back_under_a_moved_key_stays_its_value() {
        let (shown, _) = hide(PROFILE);
        let moved = shown
            .replace(&format!("      private_key: {HIDDEN}\n"), "")
            .replace("    prod:\n", &format!("    prod:\n              private_key: {HIDDEN}\n"));
        assert_ne!(moved, shown, "the fixture changed under the test");
        let saved = restore(&moved, PROFILE).unwrap();
        assert!(
            saved.contains("              private_key: |\n                -----BEGIN PRIVATE KEY-----\n                MIIEvQIBADANBg\n"),
            "{saved}"
        );
        let (again, n) = hide(&saved);
        assert_eq!(n, 4, "{again}");
        assert!(!again.contains("MIIEvQIBADANBg"), "the key reached the page: {again}");
    }

    #[test]
    fn windows_line_endings_survive_the_round_trip() {
        let crlf = PROFILE.replace('\n', "\r\n");
        let (shown, n) = hide(&crlf);
        assert_eq!(n, 4);
        assert!(!shown.contains("MIIEvQIBADANBg"));
        assert!(shown.contains(&format!("private_key: {HIDDEN}\r\n")), "{shown:?}");
        assert_eq!(restore(&shown, &crlf).unwrap(), crlf);
    }

    #[test]
    fn credential_keys_are_told_from_their_neighbours() {
        for key in ["password", "PASSWORD", "pass", "token", "refresh_token", "client_secret", "private_key",
                    "private_key_passphrase", "oauth_client_secret", "api_key"] {
            assert!(secret_key(key), "{key}");
        }
        for key in ["user", "account", "role", "private_key_path", "keyfile", "token_file", "authenticator",
                    "client_id", "warehouse"] {
            assert!(!secret_key(key), "{key}");
        }
    }
}
