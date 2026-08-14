//! Selective truncation of archived records.
//!
//! Everything a person wrote or read is archived whole. What gets shortened or
//! dropped is machine ballast, and the rules below come from measuring a real
//! 219 MB corpus rather than from guessing:
//!
//! | Field                          | Measured | Action                     |
//! |--------------------------------|----------|----------------------------|
//! | `toolUseResult` (user rows)     | 44.8 MB  | head + hash                |
//! | `signature` (thinking blocks)   | 25.5 MB  | dropped — cryptographic    |
//! | `usage.iterations`              | ~9 MB    | dropped — duplicates usage |
//! | `image.source.data` (base64)    | 11.0 MB  | dropped — hash kept        |
//! | `Write`/`Edit` tool inputs      |  5.0 MB  | head + hash — source code  |
//!
//! Nothing is removed silently. A shortened string becomes
//! `{"_lore_trunc":true,"head":…,"bytes":N,"sha256":…}` and a removed one
//! becomes `{"_lore_dropped":true,"bytes":N}`, so a projector can always tell
//! short text from an abbreviated blob, and the record never pretends to be
//! complete.

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

/// Reasoning: enough to recognize the train of thought, not to replay it.
const THINKING_HEAD: usize = 1200;
/// Tool output: file dumps and grep results, kept only for identification.
const TOOL_RESULT_HEAD: usize = 800;
/// Tool inputs: keeps paths, commands and short arguments intact while cutting
/// the file contents that `Write` and `Edit` carry. lore does not archive source
/// code — git already stores it, far more efficiently.
const TOOL_INPUT_HEAD: usize = 2048;
/// Backstop for any string no rule above claimed. Set well above a realistic
/// prompt so pasted input stays searchable, and well below the point where one
/// strange field could bloat the archive.
const BACKSTOP_HEAD: usize = 32_768;

/// Line kinds dropped entirely: transient UI bookkeeping with no evidentiary
/// value, measured at ~10% of transcript bytes.
///
/// The two `file-history` kinds are Claude Code's undo bookkeeping. They record
/// no content — only a pointer into Claude's backup store, which is deleted on
/// the same cycle as everything else, so what survives is a reference to a file
/// that no longer exists. The snapshot restates the entire tracked-file list on
/// every message (10.7 MB across this machine's archive, 10% of it); the delta
/// carries no session id at all, so nothing can attribute it. What either might
/// have told us — which file was touched when — already comes from the `Write`
/// and `Edit` tool calls, which do carry a session and an absolute path.
///
/// `last-prompt` restates the prompt already archived as a `user` record and
/// again in `history.jsonl`. A third copy answers nothing the first two do not.
pub const DROPPED_KINDS: &[&str] = &[
    "attachment",
    "queue-operation",
    "file-history-snapshot",
    "file-history-delta",
    "last-prompt",
];

pub fn is_dropped_kind(kind: &str) -> bool {
    DROPPED_KINDS.contains(&kind)
}

/// Rewrites ballast in place. Returns true if anything was shortened or removed.
pub fn apply(value: &mut Value) -> bool {
    let mut hit = false;

    // Tool output arrives twice: inside `message.content` as a `tool_result`
    // block, and again as a top-level `toolUseResult`. Both are the same file
    // dumps and both need the same treatment.
    if let Some(v) = value.get_mut("toolUseResult") {
        hit |= shorten_deep(v, TOOL_RESULT_HEAD);
    }

    // The per-iteration token breakdown restates the totals already present on
    // `usage` itself.
    if let Some(usage) = value.pointer_mut("/message/usage").and_then(Value::as_object_mut) {
        hit |= drop_key(usage, "iterations", false);
    }

    if let Some(blocks) = value
        .pointer_mut("/message/content")
        .and_then(Value::as_array_mut)
    {
        for block in blocks {
            let Some(obj) = block.as_object_mut() else {
                continue;
            };
            match obj.get("type").and_then(Value::as_str) {
                Some("thinking") => {
                    if let Some(t) = obj.get_mut("thinking") {
                        hit |= shorten(t, THINKING_HEAD);
                    }
                    // A signature proves the reasoning was not tampered with in
                    // transit. It is worthless once the record is archived, and
                    // it was the single largest field in the corpus.
                    hit |= drop_key(obj, "signature", false);
                }
                Some("image") => {
                    // Pasted screenshots as base64. The hash is kept so a
                    // duplicate paste is still recognizable.
                    if let Some(source) = obj.get_mut("source").and_then(Value::as_object_mut) {
                        hit |= drop_key(source, "data", true);
                    }
                }
                Some("tool_use") => {
                    if let Some(input) = obj.get_mut("input") {
                        hit |= shorten_deep(input, TOOL_INPUT_HEAD);
                    }
                }
                Some("tool_result") => {
                    if let Some(c) = obj.get_mut("content") {
                        hit |= shorten_deep(c, TOOL_RESULT_HEAD);
                    }
                }
                _ => {}
            }
        }
    }

    hit |= shorten_deep(value, BACKSTOP_HEAD);
    hit
}

fn drop_key(map: &mut Map<String, Value>, key: &str, keep_hash: bool) -> bool {
    match map.get_mut(key) {
        Some(v) if !is_marker(v) => drop_value(v, keep_hash),
        _ => false,
    }
}

/// Replaces a value with a marker recording what was removed.
fn drop_value(value: &mut Value, keep_hash: bool) -> bool {
    if is_marker(value) {
        return false;
    }
    let bytes = match &*value {
        Value::String(s) => s.len(),
        other => other.to_string().len(),
    };
    let mut marker = json!({ "_lore_dropped": true, "bytes": bytes });
    if keep_hash {
        if let Value::String(s) = &*value {
            marker["sha256"] = json!(hex(&Sha256::digest(s.as_bytes())));
        }
    }
    *value = marker;
    true
}

/// Shortens every string at or below this node.
fn shorten_deep(value: &mut Value, head: usize) -> bool {
    if is_marker(value) {
        return false;
    }
    match value {
        Value::String(_) => shorten(value, head),
        Value::Array(items) => items.iter_mut().fold(false, |a, i| a | shorten_deep(i, head)),
        Value::Object(map) => map.iter_mut().fold(false, |a, (_, v)| a | shorten_deep(v, head)),
        _ => false,
    }
}

fn shorten(value: &mut Value, head: usize) -> bool {
    let Value::String(s) = value else {
        return false;
    };
    if s.len() <= head {
        return false;
    }

    let bytes = s.len();
    let sha = hex(&Sha256::digest(s.as_bytes()));
    // Never split a UTF-8 character.
    let mut cut = head;
    while cut > 0 && !s.is_char_boundary(cut) {
        cut -= 1;
    }
    let head_text = s[..cut].to_string();

    *value = json!({
        "_lore_trunc": true,
        "head": head_text,
        "bytes": bytes,
        "sha256": sha,
    });
    true
}

/// Markers are terminal: re-running truncation must never wrap one again.
fn is_marker(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|m| m.contains_key("_lore_trunc") || m.contains_key("_lore_dropped"))
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assistant(blocks: Value) -> Value {
        json!({ "message": { "content": blocks } })
    }

    #[test]
    fn keeps_short_strings_intact() {
        let mut v = assistant(json!([{ "type": "text", "text": "hello" }]));
        assert!(!apply(&mut v));
        assert_eq!(v["message"]["content"][0]["text"], "hello");
    }

    #[test]
    fn keeps_prompts_and_replies_whole() {
        let long = "p".repeat(20_000);
        let mut v = assistant(json!([{ "type": "text", "text": long.clone() }]));
        assert!(!apply(&mut v));
        assert_eq!(v["message"]["content"][0]["text"], long);
    }

    #[test]
    fn shortens_thinking_and_records_the_whole() {
        let long = "t".repeat(5_000);
        let mut v = assistant(json!([{ "type": "thinking", "thinking": long }]));
        assert!(apply(&mut v));
        let block = &v["message"]["content"][0]["thinking"];
        assert_eq!(block["bytes"], 5_000);
        assert_eq!(block["head"].as_str().unwrap().len(), THINKING_HEAD);
        assert_eq!(block["sha256"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn drops_thinking_signatures() {
        let sig = "s".repeat(101_156);
        let mut v = assistant(json!([{ "type": "thinking", "thinking": "ok", "signature": sig }]));
        assert!(apply(&mut v));
        let s = &v["message"]["content"][0]["signature"];
        assert_eq!(s["_lore_dropped"], true);
        assert_eq!(s["bytes"], 101_156);
        assert!(s.get("sha256").is_none());
    }

    #[test]
    fn drops_pasted_image_data_but_keeps_its_hash() {
        let data = "A".repeat(65_747);
        let mut v = assistant(json!([{
            "type": "image",
            "source": { "type": "base64", "media_type": "image/png", "data": data }
        }]));
        assert!(apply(&mut v));
        let src = &v["message"]["content"][0]["source"];
        assert_eq!(src["media_type"], "image/png");
        assert_eq!(src["data"]["_lore_dropped"], true);
        assert_eq!(src["data"]["bytes"], 65_747);
        assert_eq!(src["data"]["sha256"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn shortens_written_file_contents_but_keeps_the_path() {
        let code = "x".repeat(55_482);
        let mut v = assistant(json!([{
            "type": "tool_use",
            "name": "Write",
            "input": { "file_path": "/w/main.rs", "content": code }
        }]));
        assert!(apply(&mut v));
        let input = &v["message"]["content"][0]["input"];
        assert_eq!(input["file_path"], "/w/main.rs");
        assert_eq!(input["content"]["bytes"], 55_482);
        assert_eq!(input["content"]["head"].as_str().unwrap().len(), TOOL_INPUT_HEAD);
    }

    #[test]
    fn shortens_the_duplicated_top_level_tool_result() {
        let dump = "d".repeat(44_000);
        let mut v = json!({ "type": "user", "toolUseResult": { "stdout": dump } });
        assert!(apply(&mut v));
        assert_eq!(v["toolUseResult"]["stdout"]["bytes"], 44_000);
    }

    #[test]
    fn drops_the_per_iteration_token_breakdown_but_keeps_the_totals() {
        let mut v = json!({
            "message": { "usage": {
                "input_tokens": 2,
                "output_tokens": 491,
                "iterations": [{ "input_tokens": 2, "output_tokens": 491 }]
            }}
        });
        assert!(apply(&mut v));
        assert_eq!(v["message"]["usage"]["output_tokens"], 491);
        assert_eq!(v["message"]["usage"]["iterations"]["_lore_dropped"], true);
    }

    #[test]
    fn shortens_tool_results_through_nested_content() {
        let long = "r".repeat(9_000);
        let mut v = assistant(json!([{
            "type": "tool_result",
            "content": [{ "type": "text", "text": long }]
        }]));
        assert!(apply(&mut v));
        assert_eq!(v["message"]["content"][0]["content"][0]["text"]["bytes"], 9_000);
    }

    #[test]
    fn never_splits_a_utf8_character() {
        let long = "é".repeat(5_000);
        let mut v = assistant(json!([{ "type": "thinking", "thinking": long }]));
        assert!(apply(&mut v));
        let head = v["message"]["content"][0]["thinking"]["head"].as_str().unwrap();
        assert!(head.len() <= THINKING_HEAD);
        assert!(head.chars().all(|c| c == 'é'));
    }

    #[test]
    fn is_idempotent_so_reprocessing_never_nests_markers() {
        let mut v = assistant(json!([
            { "type": "thinking", "thinking": "t".repeat(5_000), "signature": "s".repeat(9_000) },
            { "type": "tool_use", "name": "Write", "input": { "content": "c".repeat(9_000) } },
        ]));
        assert!(apply(&mut v));
        let once = v.clone();
        assert!(!apply(&mut v));
        assert_eq!(once, v);
    }

    #[test]
    fn backstop_catches_a_blob_no_rule_claimed() {
        let mut v = json!({ "type": "system", "someFutureField": "z".repeat(40_000) });
        assert!(apply(&mut v));
        assert_eq!(v["someFutureField"]["bytes"], 40_000);
    }
}
