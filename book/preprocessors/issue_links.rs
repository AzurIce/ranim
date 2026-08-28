#!/usr/bin/env -S cargo -Zscript
---
[package]
edition = "2024"
[dependencies]
regex = "1"
serde_json = "1"
---
// mdbook preprocessor: autolink bare `#N` references to GitHub PRs/issues.
//
// mdbook pipes a JSON `[PreprocessorContext, Book]` document to stdin; we
// walk every chapter and turn `#N` tokens into `[#N](<repo>/pull/N)`.
// `/pull/N` redirects to `/issues/N` for non-PR numbers, so one link form
// covers both (this is also what bevy.org does).
//
// Skipped: fenced code blocks, inline code spans, URL segments, and `#N`
// immediately preceded by `[`, `(`, `&` or a word character.
//
// A rust script (`cargo -Zscript`, nightly): run by mdbook from the book
// root via book.toml; needs a nightly toolchain (the repo pins one).
//
// Usage:
// - `cargo -Zscript issue_links.rs`               preprocessor mode (stdin/stdout)
// - `cargo -Zscript issue_links.rs supports <r>`  renderer probe, always exits 0

use regex::Regex;
use serde_json::Value;
use std::io::{Read, Write};
use std::sync::LazyLock;

const DEFAULT_REPO: &str = "AzurIce/ranim";

static CODE_SPAN: LazyLock<Regex> = LazyLock::new(|| Regex::new("(`+[^`]*`+)").unwrap());
static URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(https?://[^\s），。；！？)\]]*)").unwrap());
static REFERENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"#[0-9]+").unwrap());

fn main() {
    if std::env::args().nth(1).is_some() {
        // mdbook probes `<command> supports <renderer>`: we apply to all renderers.
        std::process::exit(0);
    }
    if let Err(err) = run() {
        eprintln!("issue_links: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut stdin = String::new();
    std::io::stdin()
        .read_to_string(&mut stdin)
        .map_err(|e| e.to_string())?;
    let parsed: Value = serde_json::from_str(&stdin).map_err(|e| e.to_string())?;
    let (context, mut book) = match parsed {
        Value::Array(items) if items.len() == 2 => (items[0].clone(), items[1].clone()),
        _ => return Err("unexpected input shape; expected [context, book]".into()),
    };

    let repo = context
        .pointer("/config/book/repository")
        .and_then(|v| v.as_str())
        .map(|url| url.trim_end_matches('/').to_string())
        .unwrap_or_else(|| format!("https://github.com/{DEFAULT_REPO}"));

    // mdbook 0.5 serializes the book as {"items": [...]} (older: "sections").
    let items_key = ["items", "sections"]
        .into_iter()
        .find(|k| book.get(k).is_some())
        .ok_or("book JSON has neither `items` nor `sections`")?;
    book[items_key] = walk(book[items_key].clone(), &repo);

    let out = serde_json::to_string(&book).map_err(|e| e.to_string())?;
    std::io::stdout()
        .write_all(out.as_bytes())
        .map_err(|e| e.to_string())
}

fn walk(items: Value, repo: &str) -> Value {
    match items {
        Value::Array(arr) => Value::Array(arr.into_iter().map(|it| walk(it, repo)).collect()),
        Value::Object(mut obj) => {
            if let Some(Value::Object(mut ch)) = obj.remove("Chapter") {
                if let Some(Value::String(content)) = ch.get("content") {
                    let new = process(content, repo);
                    ch.insert("content".into(), Value::String(new));
                }
                if let Some(sub) = ch.remove("sub_items") {
                    ch.insert("sub_items".into(), walk(sub, repo));
                }
                obj.insert("Chapter".into(), Value::Object(ch));
            }
            Value::Object(obj)
        }
        other => other, // unit variants (e.g. "Separator") are plain strings
    }
}

fn process(content: &str, repo: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut fenced = false;
    for line in content.split('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            fenced = !fenced;
            out.push_str(line);
        } else if fenced {
            out.push_str(line);
        } else {
            link_line(line, repo, &mut out);
        }
        out.push('\n');
    }
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

fn link_line(line: &str, repo: &str, out: &mut String) {
    // Inline code spans are protected verbatim; everything between them is
    // processed (with URL segments protected the same way).
    let mut last = 0;
    for m in CODE_SPAN.find_iter(line) {
        link_refs(&line[last..m.start()], repo, out);
        out.push_str(m.as_str());
        last = m.end();
    }
    link_refs(&line[last..], repo, out);
}

fn link_refs(text: &str, repo: &str, out: &mut String) {
    let mut last = 0;
    for m in URL.find_iter(text) {
        link_refs_in(&text[last..m.start()], repo, out);
        out.push_str(m.as_str());
        last = m.end();
    }
    link_refs_in(&text[last..], repo, out);
}

fn link_refs_in(text: &str, repo: &str, out: &mut String) {
    let mut last = 0;
    for m in REFERENCE.find_iter(text) {
        if !is_reference(text, m.start(), m.end()) {
            continue;
        }
        out.push_str(&text[last..m.start()]);
        let n = &text[m.start() + 1..m.end()];
        out.push_str(&format!("[#{n}]({repo}/pull/{n})"));
        last = m.end();
    }
    out.push_str(&text[last..]);
}

/// A match is a reference when the character before `#` and the character
/// after the digits are both non-word (and not `&`, `(`, `[`).
fn is_reference(s: &str, start: usize, end: usize) -> bool {
    let prefix_bad =
        |c: char| c.is_alphanumeric() || c == '_' || matches!(c, '&' | '(' | '[');
    let before = s[..start].chars().next_back();
    let after = s[end..].chars().next();
    !before.is_some_and(prefix_bad) && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
}
