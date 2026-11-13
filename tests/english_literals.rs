//! Guard: no English text hard-coded in the plugin's code. Everything a person can read must come
//! from `locales/*.json` through `i18n::t`/`tf`.
//!
//! It reads the production code of every `src/*.rs` (up to the first `#[cfg(test)]`; `i18n.rs` is the
//! dictionary and is not read), lexes the string literals (comments, char literals, raw strings and
//! escapes handled) and fails on a literal that looks like English text and is not in
//! `tests/literals-allow.txt`.
//!
//! The heuristic (a net, not a proof):
//! 1. TWO WORDS: two words of letters, the first or the second with 3 letters or more, separated by
//!    spaces ("unknown command", "alias loop or broken"), after the `{placeholders}` are taken out.
//! 2. CAPITALISED WORD: a word with a capital first letter, 3 letters or more and a lower-case rest
//!    ("Frame", "Drop"). ALL-CAPS host names (`BOOLEAN_OPERATION`) and camelCase fields don't match.
//! 3. WATCHED WORD: the whole literal is a one-word text that the inventory found in the code before it
//!    moved to the dictionary ("none", "auto", "effect"…). These words are also host values and field
//!    names, so each place that really needs one is in the allow-list, with the reason.
//!
//! What it cannot see: text built from pieces (`"a".to_string() + "b"`), one lower-case word that is
//! not in the dictionary, and text the host returns. Allow-list lines are `file | literal | reason`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

const FILES: &[&str] = &["audit", "contrast", "delta", "direct", "gerar", "glyphs", "lib", "panel", "store", "theme", "variants"];

/// One-word texts that used to be hard-coded in English (now `ValueNone`, `LineHeightAuto`, `EffectDefault`…).
const WATCHED: &[&str] = &["none", "auto", "effect", "underline", "strikethrough", "blur", "spread"];

/// A string literal and the line it starts on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Lit {
    file: String,
    line: usize,
    text: String,
}

/// The code before the first `#[cfg(test)]`.
fn production(src: &str) -> &str {
    src.split("#[cfg(test)]").next().unwrap_or(src)
}

/// The string literals of `src`, skipping comments and char literals.
fn literals(file: &str, src: &str) -> Vec<Lit> {
    let c: Vec<char> = src.chars().collect();
    let (mut i, mut line, mut out) = (0, 1, Vec::new());
    while i < c.len() {
        match c[i] {
            '\n' => {
                line += 1;
                i += 1;
            }
            '/' if c.get(i + 1) == Some(&'/') => {
                while i < c.len() && c[i] != '\n' {
                    i += 1;
                }
            }
            '/' if c.get(i + 1) == Some(&'*') => {
                i += 2;
                while i + 1 < c.len() && !(c[i] == '*' && c[i + 1] == '/') {
                    if c[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
                i += 2;
            }
            // A raw string: r"…", r#"…"#, br"…"
            'r' | 'b' if is_raw_start(&c, i) => {
                let mut j = i + usize::from(c[i] == 'b') + 1;
                let mut hashes = 0;
                while c.get(j) == Some(&'#') {
                    hashes += 1;
                    j += 1;
                }
                j += 1; // the opening quote
                let start_line = line;
                let mut text = String::new();
                while j < c.len() {
                    if c[j] == '"' && (0..hashes).all(|k| c.get(j + 1 + k) == Some(&'#')) {
                        break;
                    }
                    if c[j] == '\n' {
                        line += 1;
                    }
                    text.push(c[j]);
                    j += 1;
                }
                out.push(Lit { file: file.into(), line: start_line, text });
                i = j + 1 + hashes;
            }
            '"' => {
                let start_line = line;
                let mut text = String::new();
                i += 1;
                while i < c.len() && c[i] != '"' {
                    if c[i] == '\\' {
                        i += 1;
                        match c.get(i) {
                            Some('n') => text.push('\n'),
                            Some('t') => text.push('\t'),
                            Some('"') => text.push('"'),
                            Some('\\') => text.push('\\'),
                            Some('\n') => {
                                // A line continuation: the next line's indentation is skipped.
                                line += 1;
                                while c.get(i + 1).is_some_and(|x| x.is_whitespace()) {
                                    if c[i + 1] == '\n' {
                                        line += 1;
                                    }
                                    i += 1;
                                }
                            }
                            Some(other) => {
                                text.push('\\');
                                text.push(*other);
                            }
                            None => {}
                        }
                    } else {
                        if c[i] == '\n' {
                            line += 1;
                        }
                        text.push(c[i]);
                    }
                    i += 1;
                }
                out.push(Lit { file: file.into(), line: start_line, text });
                i += 1;
            }
            // A char literal ('x', '\n', '"') or a lifetime ('a): only the first is skipped over.
            '\'' => {
                if c.get(i + 1) == Some(&'\\') {
                    i += 2;
                    while i < c.len() && c[i] != '\'' {
                        i += 1;
                    }
                    i += 1;
                } else if c.get(i + 2) == Some(&'\'') {
                    i += 3;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out
}

fn is_raw_start(c: &[char], i: usize) -> bool {
    // `r"`, `r#`, `br"`, `br#`, not inside an identifier.
    if i > 0 && (c[i - 1].is_alphanumeric() || c[i - 1] == '_') {
        return false;
    }
    let j = i + usize::from(c[i] == 'b');
    c.get(j) == Some(&'r') && matches!(c.get(j + 1), Some('"' | '#'))
}

/// The text without its `{placeholders}`.
fn without_placeholders(s: &str) -> String {
    let (mut out, mut depth) = (String::new(), 0);
    for ch in s.chars() {
        match ch {
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

/// Why `text` looks like English, or none.
fn looks_english(text: &str) -> Option<&'static str> {
    if WATCHED.contains(&text.trim()) {
        return Some("watched word");
    }
    let plain = without_placeholders(text);
    let words: Vec<&str> = plain.split(|ch: char| !ch.is_ascii_alphabetic() && ch != ' ').flat_map(|chunk| chunk.split(' ')).collect();
    // Two words side by side: look at the original spacing, not the split.
    let spaced = plain.split(|ch: char| !ch.is_ascii_alphabetic() && ch != ' ').any(|chunk| {
        let w: Vec<&str> = chunk.split(' ').filter(|w| !w.is_empty()).collect();
        w.windows(2).any(|p| p[0].len().max(p[1].len()) >= 3)
    });
    if spaced {
        return Some("two words");
    }
    let capitalised = words.iter().any(|w| {
        let mut ch = w.chars();
        w.len() >= 3 && ch.next().is_some_and(|f| f.is_ascii_uppercase()) && ch.all(|x| x.is_ascii_lowercase())
    });
    capitalised.then_some("capitalised word")
}

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{}/{path}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

/// `(file, literal) -> reason` from `tests/literals-allow.txt`.
fn allowed() -> Vec<(String, String, String)> {
    read("tests/literals-allow.txt")
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .map(|l| {
            let parts: Vec<&str> = l.splitn(3, " | ").collect();
            assert_eq!(parts.len(), 3, "bad line in literals-allow.txt (file | literal | reason): {l}");
            (parts[0].trim().to_string(), parts[1].replace("\\n", "\n"), parts[2].trim().to_string())
        })
        .collect()
}

fn found() -> Vec<(Lit, &'static str)> {
    let mut out = Vec::new();
    for f in FILES {
        let src = read(&format!("src/{f}.rs"));
        for lit in literals(&format!("{f}.rs"), production(&src)) {
            if let Some(why) = looks_english(&lit.text) {
                out.push((lit, why));
            }
        }
    }
    out
}

#[test]
fn no_english_text_outside_the_dictionary() {
    let allow = allowed();
    let bad: Vec<String> = found()
        .into_iter()
        .filter(|(lit, _)| !allow.iter().any(|(f, t, _)| *f == lit.file && *t == lit.text))
        .map(|(lit, why)| format!("  {}:{}  {:?}  ({why})", lit.file, lit.line, lit.text))
        .collect();
    assert!(
        bad.is_empty(),
        "English text in the code, outside the dictionary. Move it to locales/en.json and locales/pt-BR.json (use t/tf), or, if it is not text a person reads, add it to tests/literals-allow.txt with a reason:\n{}",
        bad.join("\n")
    );
}

#[test]
fn every_allowed_literal_is_still_in_the_code() {
    let seen: Vec<(String, String)> = found().into_iter().map(|(l, _)| (l.file, l.text)).collect();
    let stale: Vec<String> = allowed().into_iter().filter(|(f, t, _)| !seen.iter().any(|(sf, st)| sf == f && st == t)).map(|(f, t, _)| format!("  {f} | {t:?}")).collect();
    assert!(stale.is_empty(), "literals-allow.txt lists literals that are gone (or no longer look English); remove them:\n{}", stale.join("\n"));
}

#[test]
fn the_heuristic_sees_what_the_inventory_found() {
    for english in ["unknown command: {cmd}", "alias loop or broken", "Drop shadow", "Frame", "Boolean Operation", "none", "auto", "effect", ", letter spacing {text}", "Select a component"] {
        assert!(looks_english(english).is_some(), "{english:?} should be flagged");
    }
    for fine in ["node.get", "absoluteBounds", "BOOLEAN_OPERATION", "—", "×{n}", "{label}: {value}", "#FF4D6D", "x", "", "1x", "png", "horizontal", "mainComponentName", "{n}px", "{}%", "12 / 34"] {
        assert!(looks_english(fine).is_none(), "{fine:?} should pass");
    }
}

#[test]
fn the_lexer_skips_comments_and_char_literals_and_reads_raw_strings() {
    let src = "// \"in a comment\"\nlet a = 'x'; let b = '\"'; let c = \"real one\"; /* \"block\" */ let d = r#\"raw \"quoted\" text\"#; fn f<'a>(s: &'a str) {}\nlet e = \"line\\\n        continues\";";
    let lits: Vec<String> = literals("t.rs", src).into_iter().map(|l| l.text).collect();
    assert_eq!(lits, vec!["real one", "raw \"quoted\" text", "linecontinues"]);
}
