//! Keeps secrets out of what the daemon stores and serves as conversation history.
//!
//! Two things happen here, both only to Maitty's copy; the agent itself still sees everything:
//!
//! - token- and key-shaped strings are masked by pattern ([`redact_text`], [`redact_input`]);
//! - the content of obviously secret files (a `.env`, a private key, a credentials file) is left
//!   out of a tool's input and of an edit's diff ([`is_secret_path`], [`OMITTED`]), keeping the
//!   path, the tool and the sizes so the chat still says "wrote .env".
//!
//! Pattern masking is best-effort. It catches well-known shapes, not every secret, so it is a
//! second line of defence and never a guarantee (see `docs/SECURITY.md`). It leans towards
//! missing a secret over wrecking ordinary text: prose, hashes, UUIDs and plain base64 stay.
//!
//! Every function is pure and idempotent (masking masked text changes nothing), because the same
//! entry passes the choke point again whenever it is updated, hydrated or read back from the cache.
//! Nothing here logs or returns what a pattern matched.

use super::history::{Content, Entry};
use serde_json::Value;
use std::borrow::Cow;
use std::sync::LazyLock;
use regex::{Captures, Regex};

/// What replaces a masked secret.
pub(super) const MASK: &str = "[redacted]";
/// What replaces the text of a secret file's content, diff or edit.
pub(super) const OMITTED: &str = "[content of a secret file omitted]";
const MASKED_KEY: &str = "[redacted private key]";
/// Added to a tool input whose text was omitted: how many bytes of text that was.
const OMITTED_BYTES_FIELD: &str = "omitted_bytes";

/// An unquoted value shorter than this is not treated as a secret (`password: no`).
const MIN_VALUE_CHARS: usize = 6;

/// Whole-match token shapes. Each is specific enough that ordinary text does not look like one.
static TOKEN_SHAPES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        // GitHub: classic and fine-grained personal access tokens, OAuth, user, server, refresh.
        r"\bgh[pousr]_[A-Za-z0-9]{36,255}",
        r"\bgithub_pat_[A-Za-z0-9_]{22,255}",
        // AWS access key ids.
        r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b",
        // Anthropic, then OpenAI (project keys and the older 48-character form).
        r"\bsk-ant-[A-Za-z0-9_-]{20,}",
        r"\bsk-(?:proj|svcacct|admin)-[A-Za-z0-9_-]{20,}",
        r"\bsk-[A-Za-z0-9]{32,}",
        // Slack bot, user and app tokens.
        r"\bxox[abeoprs]-[A-Za-z0-9-]{10,}",
        r"\bxapp-[0-9]-[A-Za-z0-9-]{10,}",
        // Google API keys.
        r"\bAIza[0-9A-Za-z_-]{35}",
        // Stripe live secret and restricted keys.
        r"\b(?:sk|rk)_live_[0-9A-Za-z]{16,}",
        // JSON Web Tokens: header and payload are base64url JSON, so both start with `eyJ`.
        r"\beyJ[A-Za-z0-9_-]{8,}\.eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]*",
    ]
    .iter()
    .map(|pattern| Regex::new(pattern).expect("token pattern is valid"))
    .collect()
});

/// A PEM private key block, to its END line, or to the end of the text while it is still being
/// streamed (or was cut), so a half-seen key never shows its body.
static PRIVATE_KEY_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?s)-----BEGIN [A-Z0-9 ]*PRIVATE KEY(?: BLOCK)?-----.*?(?:-----END [A-Z0-9 ]*PRIVATE KEY(?: BLOCK)?-----|\z)",
    )
    .expect("private key pattern is valid")
});

/// `Authorization: Bearer <credential>` and its `Basic` / `Token` / `Digest` siblings, bare or
/// inside a quoted header argument. Group 1 is the credential.
static AUTHORIZATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)\bauthorization["']?\s*[:=]\s*["']?(?:(?:bearer|basic|token|digest)\s+)?([A-Za-z0-9._~+/=-]{8,})"#,
    )
    .expect("authorization pattern is valid")
});

/// A bare `Bearer <token>`, as in a curl argument or a log line. Group 1 is the token. Long enough
/// that the word "bearer" in prose followed by an ordinary word does not match.
static BEARER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bbearer\s+([A-Za-z0-9._~+/=-]{20,})").expect("bearer pattern is valid")
});

/// `scheme://user:password@host`. Group 1 is everything up to the password, group 2 the password.
static URL_PASSWORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(\b[A-Za-z][A-Za-z0-9+.-]*://[^\s/:@"'<>]+:)([^\s/@"'<>]+)@"#)
        .expect("url pattern is valid")
});

/// `name = value`, `name: value`, `"name": "value"`, `export NAME=value`. The name is any
/// identifier-like run that [`is_secret_name`] then judges; groups: 1 name, 2 the separator
/// (with a closing quote of the name), 3 the value as written, 4 / 5 a quoted value's inside.
static ASSIGNMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"([A-Za-z_][A-Za-z0-9_.-]*)(\\?["']?[ \t]*[:=][ \t]*)(\\?"([^"\\\n]*)\\?"|'([^'\n]*)'|[^\s"',;&\\]+)"#,
    )
    .expect("assignment pattern is valid")
});

/// Words that make a name secret when they are a whole segment or end one (`githubtoken`).
/// `tokens`, `tokenizer`, `author` and `authority` therefore stay out.
const SECRET_SUFFIXES: [&str; 6] = [
    "secret", "token", "password", "passwd", "apikey", "privatekey",
];
/// Whole segments only: too short or too common inside other words to match as a suffix.
const SECRET_SEGMENTS: [&str; 5] = ["auth", "credential", "credentials", "authorization", "secrets"];
/// Values that are placeholders or non-secrets however the name reads.
const NON_SECRET_VALUES: [&str; 10] = [
    "true", "false", "null", "none", "nil", "undefined", "yes", "no", "required", "optional",
];

/// `text` with token- and key-shaped strings masked. Everything else is returned as written.
pub(super) fn redact_text(text: &str) -> Cow<'_, str> {
    let mut out = replace(&PRIVATE_KEY_BLOCK, Cow::Borrowed(text), MASKED_KEY);
    out = replace(&AUTHORIZATION, out, |caps: &Captures<'_>| mask_group(caps, 1));
    out = replace(&BEARER, out, |caps: &Captures<'_>| mask_group(caps, 1));
    out = replace(&URL_PASSWORD, out, |caps: &Captures<'_>| {
        if &caps[2] == MASK {
            caps[0].to_owned()
        } else {
            format!("{}{MASK}@", &caps[1])
        }
    });
    for shape in TOKEN_SHAPES.iter() {
        out = replace(shape, out, MASK);
    }
    replace(&ASSIGNMENT, out, |caps: &Captures<'_>| mask_assignment(caps))
}

/// `re` applied to `text`, allocating only when something matched.
fn replace<'a, R: regex::Replacer>(re: &Regex, text: Cow<'a, str>, with: R) -> Cow<'a, str> {
    match re.replace_all(&text, with) {
        Cow::Borrowed(_) => text,
        Cow::Owned(changed) => Cow::Owned(changed),
    }
}

/// The whole match with group `index` replaced by [`MASK`]; a match already masked is unchanged.
fn mask_group(caps: &Captures<'_>, index: usize) -> String {
    let whole = &caps[0];
    let Some(group) = caps.get(index) else {
        return whole.to_owned();
    };
    if group.as_str() == MASK {
        return whole.to_owned();
    }
    let start = group.start() - caps.get(0).map_or(0, |m| m.start());
    format!("{}{MASK}{}", &whole[..start], &whole[start + group.len()..])
}

fn mask_assignment(caps: &Captures<'_>) -> String {
    let (name, separator, value) = (&caps[1], &caps[2], &caps[3]);
    let quoted = caps.get(4).or_else(|| caps.get(5));
    let inner = quoted.map_or(value, |m| m.as_str());
    if !is_secret_name(name) || !is_secret_value(name, inner, quoted.is_some(), separator) {
        return caps[0].to_owned();
    }
    match quoted {
        Some(inner) => {
            // Keep the quotes (and a JSON escape's backslashes) as written around the mask.
            let start = inner.start() - caps.get(3).map_or(0, |whole| whole.start());
            format!(
                "{name}{separator}{}{MASK}{}",
                &value[..start],
                &value[start + inner.len()..]
            )
        }
        None => format!("{name}{separator}{MASK}"),
    }
}

/// Whether an identifier names a secret: one of its segments is (or ends with) a secret word.
/// Segments split on `_ - .` and on a lower-to-upper case change (`githubToken`).
fn is_secret_name(name: &str) -> bool {
    let mut segments: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut previous_lower = false;
    for c in name.chars() {
        if matches!(c, '_' | '-' | '.') {
            segments.push(std::mem::take(&mut current));
            previous_lower = false;
            continue;
        }
        if c.is_ascii_uppercase() && previous_lower {
            segments.push(std::mem::take(&mut current));
        }
        previous_lower = c.is_ascii_lowercase();
        current.push(c.to_ascii_lowercase());
    }
    segments.push(current);
    let pair = |a: &str, b: &str| segments.windows(2).any(|w| w[0] == a && w[1] == b);
    pair("api", "key")
        || pair("private", "key")
        || segments.iter().any(|segment| {
            SECRET_SEGMENTS.contains(&segment.as_str())
                || SECRET_SUFFIXES.iter().any(|word| segment.ends_with(word))
        })
}

/// Whether an assigned value looks like a secret rather than a placeholder, a reference, a flag
/// or a bit of code. `inner` is the value without its quotes.
fn is_secret_value(name: &str, inner: &str, quoted: bool, separator: &str) -> bool {
    if inner == MASK || inner.chars().count() < MIN_VALUE_CHARS {
        return false;
    }
    let lower = inner.to_ascii_lowercase();
    // A URL (its password is the URL pattern's business) or a path names a place, not a secret.
    if NON_SECRET_VALUES.contains(&lower.as_str())
        || lower.contains("...")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || inner.starts_with(['/', '~'])
        || inner.starts_with("./")
        || inner.starts_with("../")
    {
        return false;
    }
    // A reference or a template, not a value: `$TOKEN`, `${TOKEN}`, `<your-token>`, `%TOKEN%`,
    // `{{ token }}`, `process.env.TOKEN`, `get_token()`.
    if inner.starts_with(['$', '<', '{', '%', '(', '*'])
        || inner.contains(['(', ')'])
        || ["process.env", "os.environ", "env.", "getenv", "self.", "this.", "args.", "opts."]
            .iter()
            .any(|prefix| lower.starts_with(prefix))
    {
        return false;
    }
    // The same character over and over is a placeholder (`xxxxxxxx`, `********`).
    let first = inner.chars().next();
    if inner.chars().all(|c| Some(c) == first) {
        return false;
    }
    let env_style = name
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        && separator.trim() == "=";
    // Without the mark of a value (a digit or a symbol), a bare lowercase word after a name is
    // more likely code or prose (`token = refresh_token`, `password: required`) than a secret.
    let has_mark = inner
        .chars()
        .any(|c| c.is_ascii_digit() || !(c.is_ascii_alphabetic() || c == '_' || c == '.'));
    quoted || env_style || has_mark
}

/// `value` with every string in it masked and every value under a secret-named key masked, for a
/// tool input. Returns whether anything changed.
pub(super) fn redact_input(value: &mut Value) -> bool {
    match value {
        Value::String(text) => replace_if_changed(text),
        Value::Array(items) => items.iter_mut().fold(false, |any, item| redact_input(item) | any),
        Value::Object(fields) => {
            let mut changed = false;
            for (key, field) in fields.iter_mut() {
                if is_secret_name(key) && is_secret_json_value(field) {
                    *field = Value::String(MASK.into());
                    changed = true;
                } else {
                    changed |= redact_input(field);
                }
            }
            changed
        }
        _ => false,
    }
}

/// A JSON value under a secret-named key that holds a secret: a string that passes the value
/// test. Flags, numbers and nested objects are left to the generic walk.
fn is_secret_json_value(value: &Value) -> bool {
    value
        .as_str()
        .is_some_and(|text| is_secret_value("", text, true, ":"))
}

fn replace_if_changed(text: &mut String) -> bool {
    match redact_text(text) {
        Cow::Owned(masked) => {
            *text = masked;
            true
        }
        Cow::Borrowed(_) => false,
    }
}

/// Whether a file path names a file that holds credentials: `.env` (not the `.example`, `.sample`
/// or `.template` copies), keys and certificates, credential stores, and anything under `.aws`,
/// `.ssh` or `.gnupg`. Judged on the path alone; case-insensitive; `\` and `/` both separate.
pub(super) fn is_secret_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let mut components = lower.split(['/', '\\']).filter(|part| !part.is_empty());
    let Some(name) = components.next_back() else {
        return false;
    };
    if components.any(|dir| matches!(dir, ".aws" | ".ssh" | ".gnupg")) {
        return true;
    }
    if name == ".env" {
        return true;
    }
    if let Some(suffix) = name.strip_prefix(".env.") {
        return !matches!(suffix, "example" | "sample" | "template");
    }
    if matches!(
        name,
        "id_rsa"
            | "id_ed25519"
            | "id_ecdsa"
            | "id_dsa"
            | ".npmrc"
            | ".pypirc"
            | ".netrc"
            | "credentials"
            | ".git-credentials"
            | "secrets"
    ) || name.starts_with("secrets.")
    {
        return true;
    }
    name.rsplit_once('.').is_some_and(|(stem, extension)| {
        !stem.is_empty() && matches!(extension, "pem" | "key" | "p12" | "pfx" | "kdbx")
    })
}

/// Input fields that name the file a tool acts on.
const PATH_FIELDS: [&str; 3] = ["file_path", "notebook_path", "path"];
/// Input fields kept as written on a secret file: they say where and how, not what.
const KEPT_FIELDS: [&str; 8] = [
    "file_path",
    "notebook_path",
    "path",
    "kind",
    "type",
    "edit_mode",
    "cell_type",
    "cell_id",
];

/// Whether a tool input acts on a secret file.
pub(super) fn input_targets_secret_file(input: &Value) -> bool {
    PATH_FIELDS
        .iter()
        .filter_map(|field| input.get(field).and_then(Value::as_str))
        .any(is_secret_path)
}

/// Replaces the text of a tool input that acts on a secret file with [`OMITTED`], keeping the
/// path, flags, numbers and the shape of lists, and recording how many bytes were left out.
/// Returns whether anything was omitted.
pub(super) fn omit_secret_file_content(input: &mut Value) -> bool {
    if !input_targets_secret_file(input) {
        return false;
    }
    let mut omitted_bytes = 0;
    let changed = omit_strings(input, &mut omitted_bytes, true);
    if let Some(object) = input.as_object_mut()
        && omitted_bytes > 0
        && !object.contains_key(OMITTED_BYTES_FIELD)
    {
        object.insert(OMITTED_BYTES_FIELD.into(), omitted_bytes.into());
    }
    changed
}

fn omit_strings(value: &mut Value, omitted_bytes: &mut usize, top: bool) -> bool {
    match value {
        Value::String(text) => {
            if text == OMITTED || text == MASK {
                return false;
            }
            *omitted_bytes += text.len();
            *text = OMITTED.into();
            true
        }
        Value::Array(items) => items
            .iter_mut()
            .fold(false, |any, item| omit_strings(item, omitted_bytes, false) | any),
        Value::Object(fields) => {
            let mut changed = false;
            for (key, field) in fields.iter_mut() {
                if top && KEPT_FIELDS.contains(&key.as_str()) {
                    continue;
                }
                changed |= omit_strings(field, omitted_bytes, false);
            }
            changed
        }
        _ => false,
    }
}

/// A file change's diff as it may be stored: omitted when the file is a secret file, otherwise
/// masked by pattern.
pub(super) fn redact_diff(path: &str, diff: &str) -> String {
    if is_secret_path(path) {
        OMITTED.to_owned()
    } else {
        redact_text(diff).into_owned()
    }
}

/// Commands that print a file, so that naming a secret file after one means the output is it.
const READING_COMMANDS: [&str; 18] = [
    "cat", "less", "more", "head", "tail", "bat", "type", "get-content", "gc", "grep", "rg", "sed",
    "awk", "xxd", "base64", "strings", "nl", "tac",
];

/// Whether a shell command prints a secret file (`cat .env`, `tail -n5 ~/.ssh/id_rsa`). A
/// command that only names one (`git add .env`, `ls .ssh`) does not.
fn command_prints_secret_file(command: &str) -> bool {
    command.lines().any(|line| {
        let mut words = line
            .split_whitespace()
            .map(|word| word.trim_matches(['"', '\'', ';', '(', ')']));
        let reads = words.next().is_some_and(|word| {
            let program = word.rsplit(['/', '\\']).next().unwrap_or(word);
            READING_COMMANDS.contains(&program.to_ascii_lowercase().as_str())
        });
        reads && words.any(|word| !word.starts_with('-') && is_secret_path(word))
    })
}

/// Makes `entry` safe to store and serve: text masked, secret-file content omitted. The single
/// function every path into the history goes through.
pub(super) fn redact_entry(entry: &mut Entry) {
    match &mut entry.content {
        Content::Message { text, .. } => {
            replace_if_changed(text);
        }
        Content::Tool {
            output,
            label,
            input,
            ..
        } => {
            replace_if_changed(label);
            let mut prints_secret_file = false;
            if let Some(input) = input.as_mut() {
                prints_secret_file = input
                    .get("command")
                    .and_then(Value::as_str)
                    .is_some_and(command_prints_secret_file);
                omit_secret_file_content(input);
                redact_input(input);
            }
            if prints_secret_file || (input.is_none() && command_prints_secret_file(label)) {
                if !output.is_empty() {
                    *output = OMITTED.into();
                }
            } else {
                replace_if_changed(output);
            }
        }
        Content::TurnStatus | Content::Unsupported { .. } => {}
    }
}

#[cfg(test)]
mod tests;
