//! Editing agents' settings.json in place: the syntax tree is changed rather than the file
//! re-serialised, so comments, key order, layout and a byte-order mark survive.
//!
//! Qwen Code and Gemini CLI strip comments before parsing settings.json (strip-json-comments),
//! and Qwen a byte-order mark; trailing commas they reject, so they're rejected here too.

use anyhow::{Result, anyhow};
use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{CstInputValue, CstObject, CstRootNode};
use serde_json::Value;

const BOM: char = '\u{feff}';

fn options() -> ParseOptions {
    ParseOptions {
        allow_comments: true,
        allow_loose_object_property_names: false,
        allow_trailing_commas: false,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
    }
}

/// A settings file being edited. An empty file reads as `{}`.
pub(super) struct Doc {
    bom: bool,
    root: CstRootNode,
}

impl Doc {
    pub(super) fn parse(text: &str) -> Result<Self> {
        let body = text.strip_prefix(BOM);
        let bom = body.is_some();
        let body = body.unwrap_or(text);
        let body = if body.trim().is_empty() { "{}\n" } else { body };
        let root = CstRootNode::parse(body, &options()).map_err(|e| anyhow!("{e}"))?;
        Ok(Self { bom, root })
    }

    /// The top-level object, or `None` when the file holds something else.
    pub(super) fn object(&self) -> Option<CstObject> {
        self.root.value().and_then(|v| v.as_object())
    }

    pub(super) fn to_value(&self) -> Option<Value> {
        self.root.to_serde_value()
    }

    pub(super) fn text(&self) -> String {
        format!("{}{}", if self.bom { BOM.to_string() } else { String::new() }, self.root)
    }
}

/// A settings file's contents as a plain value, comments dropped; `None` when it won't parse.
pub(super) fn parse_value(text: &str) -> Option<Value> {
    Doc::parse(text).ok()?.to_value()
}

/// Sets `key` on `object`, replacing its value in place or adding it at the end.
pub(super) fn set_prop(object: &CstObject, key: &str, value: CstInputValue) {
    match object.get(key) {
        Some(prop) => prop.set_value(value),
        None => {
            object.append(key, value);
        }
    }
}

pub(super) fn input(value: &Value) -> CstInputValue {
    match value {
        Value::Null => CstInputValue::Null,
        Value::Bool(b) => CstInputValue::Bool(*b),
        Value::Number(n) => CstInputValue::Number(n.to_string()),
        Value::String(s) => CstInputValue::String(s.clone()),
        Value::Array(items) => CstInputValue::Array(items.iter().map(input).collect()),
        Value::Object(map) => CstInputValue::Object(map.iter().map(|(k, v)| (k.clone(), input(v))).collect()),
    }
}
