//! The profile's secrets, kept on the server while the profile is edited.
//!
//! `profiles.yml` is editable from the page (0017), and it can hold a
//! warehouse password, a token or a private key. Those values never leave:
//! `hide` puts `HIDDEN` in their place before the profile is sent, and `restore`
//! puts the values on disk back where the saved text still says `HIDDEN`
//! (0054). Everything else in the file reaches the page as it is, so the
//! account, the role or a target's name can still be corrected there.
//!
//! A line scanner, not a YAML parser (0003, 0018): a key is found by its
//! indentation and the keys above it, and a value runs to the next line that
//! is not indented further. That is every profile dbt's documentation shows.
//! Only a flow mapping, `{ password: x }` on one line, is beyond it, and is
//! sent as it is.

use std::collections::HashMap;

/// What the page shows in place of a secret. Saved back unchanged, it means
/// "keep the value on disk"; anything else typed over it is the new value.
pub const HIDDEN: &str = "<hidden by dbt-edith>";

/// Whether a profile key holds a credential. The same vocabulary as the
/// environment variables the hover card guards (0019), plus `pass`, which
/// dbt-postgres takes for `password`. A key naming where a secret is kept,
/// such as `private_key_path`, is not one.
pub fn secret_key(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    if k.ends_with("_path") || k.ends_with("_file") {
        return false;
    }
    k == "pass" || crate::envs::sensitive_name(&k)
}

/// One line, without its ending, and the ending it had: a profile written on
/// Windows keeps its `\r\n`.
fn lines(text: &str) -> Vec<(&str, &str)> {
    text.split_inclusive('\n')
        .map(|l| {
            let body = l.trim_end_matches(['\n', '\r']);
            (body, &l[body.len()..])
        })
        .collect()
}

fn indent(body: &str) -> usize {
    body.len() - body.trim_start_matches(' ').len()
}

/// The key a line opens, where its value starts, and its depth: `password: x`
/// gives `password`, the byte just past the colon, and its indentation. A list
/// item, `- name: x`, counts as indented to its key. None for a line that is
/// blank, a comment, or the continuation of a value.
fn key_line(body: &str) -> Option<(String, usize, usize)> {
    let mut depth = indent(body);
    let mut rest = &body[depth..];
    if rest.is_empty() || rest.starts_with('#') {
        return None;
    }
    while let Some(after) = rest.strip_prefix("- ") {
        let gap = after.len() - after.trim_start_matches(' ').len();
        depth += 2 + gap;
        rest = &after[gap..];
    }
    let colon = rest.find(": ").or_else(|| rest.ends_with(':').then(|| rest.len() - 1))?;
    let key = rest[..colon].trim().trim_matches(|c| c == '"' || c == '\'');
    if key.is_empty() || key.contains(['{', '[', '#']) {
        return None;
    }
    Some((key.to_string(), body.len() - rest.len() + colon + 1, depth))
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
            if next.trim().is_empty() {
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
        // `password:` alone opens a mapping, whose keys are judged on their own
        // lines. An `env_var()` holds no secret, only where one is, and showing
        // it is the point of editing the file. An empty value hides nothing,
        // and neither does a switch such as `passcode_in_password: true`.
        let worth_hiding = secret_key(&key)
            && !inline.is_empty()
            && !inline.starts_with('#')
            && !inline.contains("{{")
            && !matches!(inline, "''" | "\"\"")
            && !inline.eq_ignore_ascii_case("true")
            && !inline.eq_ignore_ascii_case("false");
        if worth_hiding {
            out.push(Found { path: stack.iter().map(|(_, k)| k.clone()).collect(), first: i, last, at });
            i = last + 1;
        } else {
            i += 1;
        }
    }
    out
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
    let mut kept: HashMap<Vec<String>, String> = HashMap::new();
    for f in secrets(&disk) {
        kept.entry(f.path.clone()).or_insert_with(|| value_text(&disk, &f));
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
                let Some(value) = kept.get(&path) else {
                    return Err(format!(
                        "{} was hidden, and the profile on disk has no value there to put back: type the value itself",
                        path.join(".")
                    ));
                };
                out.push_str(&body[..at]);
                out.push_str(value);
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
        for secret in ["hunter2", "rotated in May", "MIIEvQIBADANBg", "BEGIN PRIVATE KEY", "s3cret", "letmein"] {
            assert!(!shown.contains(secret), "{secret} reached the page: {shown}");
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
