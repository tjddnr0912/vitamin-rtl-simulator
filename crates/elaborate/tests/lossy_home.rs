//! §5.2 Q-LOSSYSPELL: the lossy-spelling record has one home per side (ER §5.1) — the
//! parser's scan (`crates/hdl-parser/src/lossy.rs`) and the design's OR
//! (`crates/elaborate/src/lossy.rs`). This test pins, over the production sources
//! (`crates/*/src/**/*.rs` ; comments, every `#[cfg(test)]` item and the files of a module
//! declared under `#[cfg(test)]` blanked, string and char literals kept whole ; fix N4, agy
//! F6 (b) (c)), what the record rests on and who touches it:
//!
//! - H1 the parser's two lossy bools `lossy_sign_kw_param` and `lossy_let_any` are each
//!   declared and initialised in `lib.rs`, written only by `|=` in `lossy.rs` and read
//!   once in `api.rs`; neither is a field of `ScopeSnapshot`, so no scope restore can clear
//!   it (premise judge 2 PR13, L9 ; PLAN3 PR13 for the second, §5.2 Q-LOSSYSPELL U3).
//!   Read on tokens (fix N4, N5): no hdl-parser source writes the whole parser back
//!   (`* self =`, `mem :: replace | swap | take (` of `self` / `& mut self`), which would
//!   restore every field at once.
//! - H2 (a) `scan_lossy (` has one caller, `api.rs`'s `parse_lexed`, under
//!   `if facts.xz_literal()` (U3: the pass runs only on a unit whose lexer saw an x / z
//!   literal), passing the parser's mark list `& p . lossy_whole_rhs` (fix N4) ; (b) `is_let_head (` has one, the module-item dispatch, and the string
//!   `"let"` is spelled in no hdl-parser source but `lossy.rs` ; (b') `note_let_decl (`
//!   has one caller, the first statement of `parse_let_decl` (`sva_seq.rs`), and
//!   `LetDecl {` is built in that fn only ; (c) `new_param (` in hdl-parser = one call with
//!   `Signing :: from_written` (`params.rs`, `param_dtype`) + four with the literal
//!   `Signing :: Implicit`, and no third form (fix N4, agy F6 (a)) ; (e) `lex_facts (` and `parse_lexed (` each have one production
//!   caller outside their own crate, cli `frontend.rs` (inside it: `lex` and
//!   `parse_with_warnings`, the wrappers) ; `LexFacts {` is the definition and two
//!   constructions, in `lex_facts` and `LexFacts :: of_tokens`, both in hdl-lexer ; no
//!   `Default`, no `From`, no `new`, no `pub` field on `LexFacts`, every `impl LexFacts`
//!   block read and a derive found past other attribute lines (premise judge 3 L2 ; agy F6
//!   (d)) ;
//!   the extras bool is written by `xz_seen` only, the callback of the two x / z literal
//!   kinds and of nothing else ; `strip_attribute_instances` writes the token vector only
//!   by `out.push(src[i].clone())` at two sites (premise judge 3 L4: the fact is then a
//!   superset of the stripped vector's, E6). H2 (d), the SUPERSET test, is `hdl-parser`
//!   `src/tests/lossy_spell.rs` `superset` (each parsed pin) and, through the design
//!   record, `crates/cli/tests/lossy_spell_pins.rs` `design_superset_over_the_cells`.
//!   Its blind spot (premise judge 2 L6): a class specialization actual is substituted
//!   into the monomorphized class body by name and a class's own `#( )` default is kept
//!   out of the unit, so neither reaches a holder the walk lists: the class pins
//!   (b4_classcollide, the class default) are those paths' only guard.
//! - H3 `TopItem :: LossySpell` is constructed only in hdl-parser `api.rs` and matched in
//!   production only by the design's OR (elaborate `lossy.rs`, called at `run`'s head) and
//!   the three pass-through arms (`staged.rs`, `pipeline.rs`, `md_return.rs`). Read on
//!   tokens (fix N4, N5): the variant has one spelling, `TopItem :: LossySpell` — no
//!   unqualified `LossySpell (` outside hdl-ast's definition, no `TopItem :: *`,
//!   `TopItem :: {` or `TopItem as` import that would give it another.
//! - H4 the design record `lossy_spell` (a word, not `design_lossy_spell`) has ONE writer,
//!   `run`'s head in `driver.rs`, the field and its initialiser, the hand-off in `api.rs`
//!   and the `Sidecars` field in `tables.rs` — and 0 production readers while nothing
//!   reads it: no other line of any crate's source names it (the first reader edits this
//!   rule, as the write view's first reader will edit wext_home's H3) ; the writer is the
//!   second statement of `run`, right after `self.inside_name_use =` (agy F6 (e)). It
//!   cannot see a read through a `{:?}` of the whole `Sidecars`.
//! - H5 the `-G` test `cli_value_spells_unknown` and `cli_overrides_for` (`params.rs`)
//!   read one text through the same raw → value fns: both call
//!   `literal :: is_fill_literal (t , ast :: IntLitKind :: UnsizedBased)` and
//!   `parse_int_literal (t , ast :: IntLitKind :: Sized)`, the `-G` test calls no other
//!   value fn (its calls are exactly `is_fill_literal`, `fill_is_unknown`,
//!   `parse_int_literal` and std verbs), and its one caller is the design's OR ; the
//!   override fn reads `is_fill_literal` before `parse_int_literal` (agy F6 (e)).
//! - H6 (fix N4, the parser's mark list `lossy_whole_rhs`, form A): (a) the words
//!   `lossy_whole_rhs` and `inside_kw_at` appear only at their listed (file, fn) sites — the
//!   field, its initialiser, `rewind`'s two pops, `assign_rhs`'s push, `api.rs`'s one read
//!   and `inside_kw_at`'s own sites ; (b) `assign_rhs (` has exactly the eight callers S1–S8
//!   by (file, fn), and its body pushes only when the next token is an unsized x / z
//!   literal ; (c) on tokens, `<any receiver> . pos =` only inside `rewind`, `. pos` followed
//!   only by a comparison, `+`, `-`, `+=`, `..` or a closer (no `-=`, no method call), no
//!   `& mut … . pos`, and the bare word `pos` only at the field and in `Parser :: new` (no
//!   destructuring), so no restore of the cursor leaves a mark past it. It cannot see a write through an unsafe pointer, a
//!   `& mut (* self) . pos`, or `assign_rhs` taken as a fn value ; a needle spelled inside a
//!   string literal is over-reported (a false failure, never a silent pass).
//!
//! Each rule has a planted control that must fail.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// Every production `.rs` file under `crates/*/src`, as (path relative to `crates/`, text
/// with every comment and every `#[cfg(test)]` item cut out): `production_of` over the raw
/// files.
fn production_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let mut ents: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        ents.sort();
        for p in ents {
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|e| e == "rs") {
                out.push(p);
            }
        }
    }
    let crates = root().join("crates");
    let mut krates: Vec<_> = std::fs::read_dir(&crates)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.join("src").is_dir())
        .collect();
    krates.sort();
    let mut files = Vec::new();
    for k in krates {
        walk(&k.join("src"), &mut files);
    }
    production_of(
        files
            .into_iter()
            .map(|p| {
                let text = std::fs::read_to_string(&p).unwrap();
                let rel = p
                    .strip_prefix(&crates)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                (rel, text)
            })
            .collect(),
    )
}

/// The production part of a set of raw source files (agy F6 (b), (c), taught): every comment
/// cut (`//` to the end of its line, `/* … */` nested, outside string and char literals ;
/// line breaks kept), every item under `#[cfg(test)]` cut (an inline `mod t { … }`, a `fn`,
/// a `use`, … — the item alone, so production code after it stays read), and every file a
/// `#[cfg(test)] mod NAME;` declares dropped with its directory (`NAME.rs`, `NAME/`), whatever
/// its name (`tests.rs`, `kernel_tests.rs`, `src/tests/`). String literals stay: a needle
/// inside one is read as code (an over-report, never a miss).
fn production_of(raw: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut test_paths: Vec<String> = Vec::new();
    let mut kept = Vec::new();
    for (rel, text) in raw {
        let (code, test_mods) = strip_test_items(&strip_comments(&text));
        let dir = match rel.rsplit_once('/') {
            Some((d, f)) if ["lib.rs", "main.rs", "mod.rs"].contains(&f) => d.to_string(),
            Some((d, f)) => format!("{d}/{}", f.trim_end_matches(".rs")),
            None => String::new(),
        };
        for m in test_mods {
            test_paths.push(format!("{dir}/{m}.rs"));
            test_paths.push(format!("{dir}/{m}/"));
        }
        kept.push((rel, code));
    }
    kept.into_iter()
        .filter(|(rel, _)| {
            !test_paths
                .iter()
                .any(|t| rel == t || (t.ends_with('/') && rel.starts_with(t.as_str())))
        })
        .collect()
}

/// The length of the string or char literal starting at `i`, or `None` (a lifetime, or no
/// literal). Handles `"…"` with escapes, `r"…"` / `r#"…"#`, the `b` prefixes, `'x'`, `'\n'`.
fn literal_len(b: &[u8], i: usize) -> Option<usize> {
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let prev_ident = i > 0 && ident(b[i - 1]);
    match b[i] {
        b'r' if !prev_ident || (i > 0 && b[i - 1] == b'b' && (i < 2 || !ident(b[i - 2]))) => {
            let mut j = i + 1;
            while b.get(j) == Some(&b'#') {
                j += 1;
            }
            if b.get(j) != Some(&b'"') {
                return None;
            }
            let hashes = j - i - 1;
            let mut k = j + 1;
            while k < b.len() {
                if b[k] == b'"'
                    && b[k + 1..]
                        .iter()
                        .take(hashes)
                        .filter(|&&c| c == b'#')
                        .count()
                        == hashes
                {
                    return Some(k + 1 + hashes - i);
                }
                k += 1;
            }
            Some(b.len() - i)
        }
        b'"' => {
            let mut k = i + 1;
            while k < b.len() {
                match b[k] {
                    b'\\' => k += 2,
                    b'"' => return Some(k + 1 - i),
                    _ => k += 1,
                }
            }
            Some(b.len() - i)
        }
        b'\'' => {
            if b.get(i + 1) == Some(&b'\\') {
                // the escaped char itself is skipped: `'\''`, `'\\'`, `'\x41'`
                let mut k = i + 3;
                while k < b.len() && b[k] != b'\'' {
                    k += 1;
                }
                return Some(k + 1 - i);
            }
            // `'x'` (one char, maybe multi-byte) ; anything else is a lifetime
            let ch = std::str::from_utf8(&b[i + 1..])
                .ok()
                .and_then(|t| t.chars().next())
                .map_or(1, char::len_utf8);
            (b.get(i + 1 + ch) == Some(&b'\'')).then_some(ch + 2)
        }
        _ => None,
    }
}

/// `text` with every comment replaced by spaces (line breaks kept), literals untouched.
fn strip_comments(text: &str) -> String {
    let b = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if let Some(n) = literal_len(b, i) {
            out.extend_from_slice(&b[i..i + n]);
            i += n;
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            let mut depth = 0;
            while i < b.len() {
                if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                    depth += 1;
                    out.extend_from_slice(b"  ");
                    i += 2;
                } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                    depth -= 1;
                    out.extend_from_slice(b"  ");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(if b[i] == b'\n' { b'\n' } else { b' ' });
                    i += 1;
                }
            }
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).expect("utf8 kept")
}

/// The byte index just past the item starting at `at`: its `;` or its matching `}`, literals
/// skipped (a `{` before the first `;` opens the body).
fn item_end(b: &[u8], at: usize) -> usize {
    let mut depth = 0i32;
    let mut i = at;
    while i < b.len() {
        if let Some(n) = literal_len(b, i) {
            i += n;
            continue;
        }
        match b[i] {
            b'{' | b'(' | b'[' => depth += 1,
            b'}' | b')' | b']' => {
                depth -= 1;
                if depth == 0 && b[i] == b'}' {
                    return i + 1;
                }
            }
            b';' if depth == 0 => return i + 1,
            _ => {}
        }
        i += 1;
    }
    b.len()
}

/// The byte index just past the bracket matching the one at `open`, literals skipped.
fn group_end(b: &[u8], open: usize) -> usize {
    let mut depth = 0i32;
    let mut i = open;
    while i < b.len() {
        if let Some(n) = literal_len(b, i) {
            i += n;
            continue;
        }
        match b[i] {
            b'{' | b'(' | b'[' => depth += 1,
            b'}' | b')' | b']' => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    b.len()
}

/// `code` (comments already cut) with each `#[cfg(test)]` item blanked (line breaks kept),
/// and the names of the file modules a `#[cfg(test)] mod NAME;` declares.
fn strip_test_items(code: &str) -> (String, Vec<String>) {
    let mut b = code.as_bytes().to_vec();
    let mut mods = Vec::new();
    let mut from = 0;
    while let Some(at) = code[from..].find("#[cfg(test)]").map(|a| a + from) {
        // skip further attributes and blanks to the item itself
        let mut j = at + "#[cfg(test)]".len();
        loop {
            while j < b.len() && b[j].is_ascii_whitespace() {
                j += 1;
            }
            if b[j..].starts_with(b"#[") {
                j = group_end(&b, j + 1);
            } else {
                break;
            }
        }
        let end = item_end(&b, j);
        let item = String::from_utf8_lossy(&b[j..end]).into_owned();
        let words: Vec<&str> = item.split_whitespace().collect();
        if let Some(k) = words.iter().position(|w| *w == "mod") {
            if let Some(name) = words.get(k + 1).filter(|n| n.ends_with(';')) {
                mods.push(name.trim_end_matches(';').to_string());
            }
        }
        for c in &mut b[at..end] {
            if *c != b'\n' {
                *c = b' ';
            }
        }
        from = end;
    }
    (String::from_utf8(b).expect("utf8 kept"), mods)
}

/// Code lines only (comments are cut by `production_of`; a line left blank is skipped).
fn code_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with("//"))
}

/// Does `line` hold `word` as a whole identifier (no identifier character on either side)?
fn has_word(line: &str, word: &str) -> bool {
    let id = |c: char| c.is_ascii_alphanumeric() || c == '_';
    line.match_indices(word).any(|(at, _)| {
        !line[..at].chars().next_back().is_some_and(id)
            && !line[at + word.len()..].chars().next().is_some_and(id)
    })
}

/// (file, trimmed line) of every code line holding `word` as a whole identifier.
fn word_sites(files: &[(String, String)], word: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (f, t) in files {
        for l in code_lines(t) {
            if has_word(l, word) {
                out.push((f.clone(), l.trim().to_string()));
            }
        }
    }
    out
}

/// (file, trimmed line) of every code line holding `needle` (a call or a path, spelled
/// without spaces as rustfmt writes it).
fn sites(files: &[(String, String)], needle: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (f, t) in files {
        for l in code_lines(t) {
            if l.contains(needle) {
                out.push((f.clone(), l.trim().to_string()));
            }
        }
    }
    out
}

fn only(files: &[(String, String)], prefix: &str) -> Vec<(String, String)> {
    files
        .iter()
        .filter(|(f, _)| f.starts_with(prefix))
        .cloned()
        .collect()
}

/// The body (`{` … matching `}`) of the first `fn <name>(` in `text`.
fn fn_body<'a>(text: &'a str, name: &str) -> &'a str {
    let at = text
        .find(&format!("fn {name}("))
        .unwrap_or_else(|| panic!("fn {name} not found"));
    let open = at + text[at..].find('{').expect("fn body");
    let mut depth = 0i32;
    for (i, c) in text[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &text[open..open + i + 1];
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced body of fn {name}")
}

/// The identifiers a body calls (`ident(`, `ident::<..>(` not used here), comment lines
/// skipped, sorted and deduplicated.
fn calls(body: &str) -> Vec<String> {
    let mut out = std::collections::BTreeSet::new();
    for l in code_lines(body) {
        let b = l.as_bytes();
        for (i, _) in l.match_indices('(') {
            let mut s = i;
            while s > 0 && (b[s - 1].is_ascii_alphanumeric() || b[s - 1] == b'_') {
                s -= 1;
            }
            if s < i {
                out.insert(l[s..i].to_string());
            }
        }
    }
    out.into_iter().collect()
}

fn text_of<'a>(files: &'a [(String, String)], rel: &str) -> &'a str {
    &files
        .iter()
        .find(|(f, _)| f == rel)
        .unwrap_or_else(|| panic!("{rel} not found"))
        .1
}

// ─────────────────────────────── H1 ───────────────────────────────

/// The parser's lossy bools: (field, the `api.rs` read).
const H1_BOOLS: [(&str, &str); 2] = [
    (
        "lossy_sign_kw_param",
        "lossy.sign_kw_param = p.lossy_sign_kw_param;",
    ),
    ("lossy_let_any", "lossy.let_any = p.lossy_let_any;"),
];

/// H1's check over a set of files: each code line naming a parser bool is one of the four
/// allowed forms, at its file.
fn h1_violations(files: &[(String, String)]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (word, read) in H1_BOOLS {
        out.extend(word_sites(files, word).into_iter().filter(|(f, l)| {
            let ok = match f.as_str() {
                "hdl-parser/src/lib.rs" => {
                    *l == format!("{word}: bool,") || *l == format!("{word}: false,")
                }
                "hdl-parser/src/lossy.rs" => l.starts_with(&format!("self.{word} |= ")),
                "hdl-parser/src/api.rs" => l == read,
                _ => false,
            };
            !ok
        }));
    }
    out
}

/// The text of `struct ScopeSnapshot { … }`.
fn scope_snapshot(lib: &str) -> &str {
    fn_like_body(lib, "struct ScopeSnapshot")
}

fn fn_like_body<'a>(text: &'a str, head: &str) -> &'a str {
    let at = text.find(head).expect("head");
    let open = at + text[at..].find('{').expect("body");
    let close = open + text[open..].find("\n}").expect("close");
    &text[open..close]
}

#[test]
fn h1_parser_bools_are_or_written_and_in_no_snapshot() {
    let files = only(&production_sources(), "hdl-parser/src/");
    assert_eq!(h1_violations(&files), Vec::<(String, String)>::new());
    for (word, _) in H1_BOOLS {
        let all = word_sites(&files, word);
        assert_eq!(
            all.len(),
            4,
            "{word}: decl, init, the `|=` write, the read: {all:?}"
        );
    }
    let snap = scope_snapshot(text_of(&files, "hdl-parser/src/lib.rs"));
    assert!(snap.contains("type_names"), "the snapshot body was found");
    assert!(!snap.contains("lossy"), "a lossy bit in ScopeSnapshot");
}

#[test]
fn h1_planted_controls_fail() {
    for word in ["lossy_sign_kw_param", "lossy_let_any"] {
        let planted = vec![(
            "hdl-parser/src/scope.rs".to_string(),
            format!("fn restore(&mut self, s: Snap) {{\n    self.{word} = s.saved;\n}}\n"),
        )];
        assert_eq!(
            h1_violations(&planted).len(),
            1,
            "{word}: an assignment from saved state"
        );
        let planted2 = vec![(
            "hdl-parser/src/lossy.rs".to_string(),
            format!("    self.{word} = matches!(x, y);\n"),
        )];
        assert_eq!(
            h1_violations(&planted2).len(),
            1,
            "{word}: `=` where `|=` is the rule"
        );
    }
    let snap = "struct ScopeSnapshot {\n    type_names: X,\n    lossy_let_any: bool,\n}\n";
    assert!(scope_snapshot(snap).contains("lossy"));
}

// ─────────────────────────────── H2 ───────────────────────────────

/// The files of a call's callers, its definition line left out.
fn callers(files: &[(String, String)], needle: &str, def: &str) -> Vec<String> {
    sites(files, needle)
        .into_iter()
        .filter(|(_, l)| !l.contains(def))
        .map(|(f, _)| f)
        .collect()
}

/// `text` with every run of white space made one space.
fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// H2's counts: scan_lossy's callers, is_let_head's callers, the files spelling `"let"`, and
/// `DataType::new_param(` calls as (from_written, Implicit, any other argument list).
#[allow(clippy::type_complexity)]
fn h2_counts(
    files: &[(String, String)],
) -> (Vec<String>, Vec<String>, Vec<String>, usize, usize, usize) {
    let scan = callers(files, "scan_lossy(", "fn scan_lossy(");
    let let_head = callers(files, "is_let_head(", "fn is_let_head(");
    let let_str: Vec<String> = sites(files, "\"let\"")
        .into_iter()
        .map(|(f, _)| f)
        .collect();
    // `new_param(` spans lines: read each call's argument text up to its `)`.
    // agy F6 (a), taught: every call is one of the two forms, a third argument list (say
    // `Signing::Signed`) is counted apart and must be 0.
    let (mut written, mut implicit, mut other) = (0, 0, 0);
    for (f, t) in files {
        let _ = f;
        for (at, _) in t.match_indices("new_param(") {
            let args = &t[at..at + t[at..].find(')').unwrap_or(t.len() - at)];
            if args.contains("Signing::from_written(") {
                written += 1;
            } else if args.contains("Signing::Implicit") {
                implicit += 1;
            } else if !t[..at].ends_with("fn ") {
                other += 1;
            }
        }
    }
    (scan, let_head, let_str, written, implicit, other)
}

/// H2 (a)'s gate: `parse_lexed`'s body runs the scan only under the lexer's fact.
fn scan_is_gated(api: &str) -> bool {
    squash(fn_body(api, "parse_lexed")).contains(
        "let mut lossy = if facts.xz_literal() { lossy::scan_lossy(tokens, src, \
         &p.lossy_whole_rhs) } else { LossySpell::default() };",
    )
}

/// H2 (b'): `note_let_decl` is the first statement of `parse_let_decl`.
fn let_note_first(sva_seq: &str) -> bool {
    let body = fn_body(sva_seq, "parse_let_decl");
    code_lines(&body[1..])
        .map(str::trim)
        .find(|l| !l.is_empty())
        .is_some_and(|l| l == "self.note_let_decl();")
}

#[test]
fn h2_scan_let_and_signing_sites() {
    let files = only(&production_sources(), "hdl-parser/src/");
    let (scan, let_head, let_str, written, implicit, other) = h2_counts(&files);
    assert_eq!(scan, ["hdl-parser/src/api.rs"], "(a) scan_lossy's callers");
    assert!(
        scan_is_gated(text_of(&files, "hdl-parser/src/api.rs")),
        "(a) the scan's one call sits under the lexer's fact"
    );
    assert_eq!(
        let_head,
        ["hdl-parser/src/item_dispatch.rs"],
        "(b) is_let_head's callers"
    );
    assert_eq!(
        let_str,
        ["hdl-parser/src/lossy.rs"],
        "(b) the string \"let\""
    );
    assert_eq!(
        callers(&files, "note_let_decl(", "fn note_let_decl("),
        ["hdl-parser/src/sva_seq.rs"],
        "(b') note_let_decl's callers"
    );
    let sva = text_of(&files, "hdl-parser/src/sva_seq.rs");
    assert!(
        let_note_first(sva),
        "(b') the first statement of parse_let_decl"
    );
    assert_eq!(
        sites(&files, "LetDecl {"),
        [(
            "hdl-parser/src/sva_seq.rs".to_string(),
            "Some(ModuleItem::LetDecl(LetDecl {".to_string()
        )],
        "(b') the one LetDecl construction"
    );
    assert!(fn_body(sva, "parse_let_decl").contains("LetDecl {"));
    assert_eq!(
        (written, implicit, other),
        (1, 4, 0),
        "(c) new_param's calls"
    );
    let params = text_of(&files, "hdl-parser/src/params.rs");
    assert!(fn_body(params, "param_dtype").contains("Signing::from_written("));
}

#[test]
fn h2_planted_controls_fail() {
    let planted = vec![
        (
            "hdl-parser/src/api.rs".to_string(),
            "let mut lossy = lossy::scan_lossy(tokens, src);".to_string(),
        ),
        (
            "hdl-parser/src/stmt.rs".to_string(),
            "    let again = lossy::scan_lossy(toks, src);\n    if t == \"let\" {}\n    \
             let d = DataType::new_param(k, Signing::from_written(w), false, None);\n    \
             if lossy::is_let_head(t, s, i) { self.note_let_decl(); }"
                .to_string(),
        ),
    ];
    let (scan, let_head, let_str, written, _, other) = h2_counts(&planted);
    assert_eq!(scan.len(), 2, "a second caller of scan_lossy is seen");
    assert_eq!(let_head, ["hdl-parser/src/stmt.rs"]);
    assert_eq!(let_str, ["hdl-parser/src/stmt.rs"]);
    assert_eq!(written, 1);
    assert_eq!(other, 0);
    // agy F6 (a): a third signing form passes no count but `other`
    let signed = vec![(
        "hdl-parser/src/params.rs".to_string(),
        "let d = DataType::new_param(k, Signing::Signed, true, None);".to_string(),
    )];
    assert_eq!(h2_counts(&signed).5, 1, "a new_param call of neither form");
    assert_eq!(
        callers(&planted, "note_let_decl(", "fn note_let_decl("),
        ["hdl-parser/src/stmt.rs"]
    );
    // the scan called outside the gate
    let ungated = "pub fn parse_lexed(t: &[S], src: &str, facts: LexFacts) {\n    \
                   let mut lossy = lossy::scan_lossy(tokens, src);\n}\n";
    assert!(!scan_is_gated(ungated));
    // the note after the head's tokens are consumed
    let late = "fn parse_let_decl(&mut self) -> Option<ModuleItem> {\n    \
                let start = self.cur_span();\n    self.note_let_decl();\n}\n";
    assert!(!let_note_first(late));
    assert!(let_note_first(
        "fn parse_let_decl(&mut self) {\n    // c\n    self.note_let_decl();\n}\n"
    ));
}

// ─────────────────── H2 (e): the lexer's fact (§5.2 Q-LOSSYSPELL U3) ───────────────────

/// What H2 (e) reads, over a set of files.
#[derive(Debug, PartialEq)]
struct H2e {
    /// The files of each `lex_facts(` caller but the definition.
    lex_facts: Vec<String>,
    /// The files of each `parse_lexed(` caller but the definition.
    parse_lexed: Vec<String>,
    /// Code lines holding `LexFacts {`, but an `impl` header.
    facts_braces: Vec<String>,
    /// `Default`, `From<..>`, `fn new(` or a `pub` field on `LexFacts` (premise judge 3 L2).
    facts_doors: Vec<String>,
    /// Code lines writing the extras bool.
    extras_writes: Vec<String>,
    /// The variant each `xz_seen` callback sits on.
    callback_variants: Vec<String>,
    /// Lines of `strip_attribute_instances` that touch its output or the token vector in a
    /// way other than its two `out.push(src[i].clone())`, `out.last()`, the take, the
    /// final store and the pre-pass read (premise judge 3 L4).
    strip_writes: Vec<String>,
    /// `out.push(src[i].clone())` sites in `strip_attribute_instances`.
    strip_pushes: usize,
}

fn h2e(files: &[(String, String)]) -> H2e {
    let lexer = files
        .iter()
        .find(|(f, _)| f == "hdl-lexer/src/lib.rs")
        .map_or("", |(_, t)| t.as_str());
    let facts_braces: Vec<String> = sites(files, "LexFacts {")
        .into_iter()
        .filter(|(_, l)| !l.starts_with("impl"))
        .map(|(f, l)| format!("{f}: {l}"))
        .collect();
    let mut facts_doors = Vec::new();
    for (f, t) in files {
        let lines: Vec<&str> = code_lines(t).collect();
        for (i, l) in lines.iter().enumerate() {
            let l = l.trim();
            // agy F6 (d), taught: the derive anywhere in the attribute lines above the struct
            // (another attribute between them hides it no more ; comments are cut already)
            let default_derive = l.starts_with("#[derive(")
                && l.contains("Default")
                && lines[i + 1..]
                    .iter()
                    .map(|n| n.trim())
                    .find(|n| !n.starts_with("#["))
                    .is_some_and(|n| n.contains("struct LexFacts"));
            if default_derive
                || (l.starts_with("impl") && l.contains("for LexFacts"))
                || l.contains("Default for LexFacts")
            {
                facts_doors.push(format!("{f}: {l}"));
            }
        }
    }
    if !lexer.is_empty() && lexer.contains("pub struct LexFacts") {
        let def = fn_like_body(lexer, "pub struct LexFacts");
        for l in code_lines(def) {
            if l.trim_start().starts_with("pub") {
                facts_doors.push(format!("pub field: {}", l.trim()));
            }
        }
        // agy F6 (d), taught: every `impl LexFacts {` block, not the first only
        for (at, _) in lexer.match_indices("impl LexFacts {") {
            let imp = fn_like_body(&lexer[at..], "impl LexFacts");
            for l in code_lines(imp) {
                if l.contains("fn new(") || l.contains("bool) -> Self") {
                    facts_doors.push(format!("constructor: {}", l.trim()));
                }
            }
        }
    }
    let extras_writes: Vec<String> = sites(files, "extras.xz_literal =")
        .into_iter()
        .map(|(f, l)| format!("{f}: {l}"))
        .collect();
    // the callback: each `xz_seen` word but the fn itself, and the variant line after it
    let mut callback_variants = Vec::new();
    let lines: Vec<&str> = lexer.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        if has_word(l, "xz_seen") && !l.contains("fn xz_seen(") && !l.trim_start().starts_with("//")
        {
            let v = lines[i + 1..]
                .iter()
                .map(|n| n.trim())
                .find(|n| {
                    n.ends_with(',')
                        && n.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                        && !n.contains(' ')
                })
                .unwrap_or("NONE");
            callback_variants.push(v.trim_end_matches(',').to_string());
        }
    }
    let (mut strip_writes, mut strip_pushes) = (Vec::new(), 0);
    if lexer.contains("fn strip_attribute_instances(") {
        let body = fn_body(lexer, "strip_attribute_instances");
        for l in code_lines(body) {
            let l = l.trim();
            let mut rest = l.to_string();
            for ok in [
                "out.push(src[i].clone())",
                "out.last()",
                "std::mem::take(tokens)",
                "*tokens = out;",
                "tokens.windows(2)",
                "let mut out: Vec<Spanned> = Vec::with_capacity(src.len());",
            ] {
                if ok == "out.push(src[i].clone())" {
                    strip_pushes += rest.matches(ok).count();
                }
                rest = rest.replace(ok, "");
            }
            if has_word(&rest, "out") || has_word(&rest, "tokens") {
                strip_writes.push(l.to_string());
            }
        }
    }
    H2e {
        lex_facts: callers(files, "lex_facts(", "fn lex_facts("),
        parse_lexed: callers(files, "parse_lexed(", "fn parse_lexed("),
        facts_braces,
        facts_doors,
        extras_writes,
        callback_variants,
        strip_writes,
        strip_pushes,
    }
}

#[test]
fn h2e_lexer_fact_sites() {
    let files = production_sources();
    let got = h2e(&files);
    assert_eq!(
        got,
        H2e {
            lex_facts: vec!["cli/src/frontend.rs".into(), "hdl-lexer/src/lib.rs".into()],
            parse_lexed: vec!["cli/src/frontend.rs".into(), "hdl-parser/src/api.rs".into()],
            facts_braces: vec![
                "hdl-lexer/src/lib.rs: pub struct LexFacts {".into(),
                "hdl-lexer/src/lib.rs: LexFacts {".into(),
                "hdl-lexer/src/lib.rs: let facts = LexFacts {".into(),
            ],
            facts_doors: vec![],
            extras_writes: vec!["hdl-lexer/src/lib.rs: lex.extras.xz_literal = true;".into()],
            callback_variants: vec!["IntSizedXz".into(), "IntUnsizedBasedXz".into()],
            strip_writes: vec![],
            strip_pushes: 2,
        }
    );
    // Inside their crates the two callers are the wrappers: `lex` and `parse_with_warnings`.
    let lexer = text_of(&files, "hdl-lexer/src/lib.rs");
    assert!(fn_body(lexer, "lex").contains("lex_facts(src)"));
    let api = text_of(&files, "hdl-parser/src/api.rs");
    assert!(fn_body(api, "parse_with_warnings").contains("parse_lexed("));
    // The two constructions sit in `of_tokens` and `lex_facts`; the one write in `xz_seen`.
    assert!(fn_body(lexer, "of_tokens").contains("LexFacts {"));
    assert!(fn_body(lexer, "lex_facts").contains("LexFacts {"));
    assert!(fn_body(lexer, "xz_seen").contains("lex.extras.xz_literal = true;"));
}

#[test]
fn h2e_planted_controls_fail() {
    let base = production_sources();
    let lexer = text_of(&base, "hdl-lexer/src/lib.rs").to_string();
    let with = |edit: &dyn Fn(&str) -> String| -> H2e {
        let mut files = base.clone();
        for (f, t) in files.iter_mut() {
            if f == "hdl-lexer/src/lib.rs" {
                *t = edit(&lexer);
            }
        }
        h2e(&files)
    };
    // L2: `Default` derived on LexFacts
    let g = with(&|t| {
        t.replace(
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub struct LexFacts {",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\npub struct LexFacts {",
        )
    });
    assert_eq!(
        g.facts_doors.len(),
        1,
        "a derived Default: {:?}",
        g.facts_doors
    );
    // L2: a `From<bool>` / a `new(bool)` / a `pub` field
    let g = with(&|t| {
        format!(
            "{t}\nimpl From<bool> for LexFacts {{\n    fn from(b: bool) -> Self {{\n        \
             LexFacts {{ xz_literal: b }}\n    }}\n}}\n"
        )
    });
    assert_eq!(g.facts_doors.len(), 1, "a From<bool>");
    assert_eq!(g.facts_braces.len(), 4, "a third construction");
    let g = with(&|t| {
        t.replace(
            "impl LexFacts {\n",
            "impl LexFacts {\n    pub fn new(b: bool) -> Self { todo!() }\n",
        )
    });
    assert_eq!(g.facts_doors.len(), 1, "a new(bool)");
    let g = with(&|t| {
        t.replacen(
            "pub struct LexFacts {\n    xz_literal",
            "pub struct LexFacts {\n    pub xz_literal",
            1,
        )
    });
    assert_eq!(g.facts_doors.len(), 1, "a pub field");
    // agy F6 (d): an attribute between the derive and the struct ; a second impl block
    let g = with(&|t| {
        t.replace(
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub struct LexFacts {",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\n#[must_use]\npub struct LexFacts {",
        )
    });
    assert_eq!(
        g.facts_doors.len(),
        1,
        "a derived Default under another attribute"
    );
    let g = with(&|t| {
        format!(
            "{t}\nimpl LexFacts {{\n    pub(crate) fn from_bool(b: bool) -> Self {{\n        \
             Self {{ xz_literal: b }}\n    }}\n}}\n"
        )
    });
    assert_eq!(
        g.facts_doors.len(),
        1,
        "a constructor in a second impl block"
    );
    // the extras bool written elsewhere ; the callback on a third variant
    let g = with(&|t| {
        t.replacen(
            "fn lone_sigil(",
            "fn other(lex: &mut Lexer<TokenKind>) { lex.extras.xz_literal = true; }\nfn lone_sigil(",
            1,
        )
    });
    assert_eq!(
        g.extras_writes.len(),
        2,
        "a second writer of the extras bool"
    );
    let g = with(&|t| {
        t.replacen(
            "#[regex(r\"\\d[\\d_]*\")]\n    IntDecimal,",
            "#[regex(r\"\\d[\\d_]*\", xz_seen)]\n    IntDecimal,",
            1,
        )
    });
    assert_eq!(
        g.callback_variants,
        ["IntSizedXz", "IntUnsizedBasedXz", "IntDecimal"],
        "the callback on a third variant"
    );
    // L4: the strip writes a token it did not take from the input
    let g = with(&|t| {
        t.replacen(
            "                out.push(src[i].clone());\n                i += 1;\n            }",
            "                out.push(Spanned::new(TokenKind::IntSizedXz, 0..0));\n                i += 1;\n            }",
            1,
        )
    });
    assert_eq!(g.strip_pushes, 1, "one of the two pushes replaced");
    assert_eq!(
        g.strip_writes.len(),
        1,
        "a push of a new token: {:?}",
        g.strip_writes
    );
    let g = with(&|t| {
        t.replacen(
            "    *tokens = out;\n",
            "    out.insert(0, src[0].clone());\n    *tokens = out;\n",
            1,
        )
    });
    assert_eq!(g.strip_writes.len(), 1, "an insert");
    // a second production caller of the production entries
    let mut files = base.clone();
    files.push((
        "elaborate/src/x.rs".to_string(),
        "let (t, e, f) = hdl_lexer::lex_facts(s);\nlet u = hdl_parser::parse_lexed(&t, s, f);\n"
            .to_string(),
    ));
    let g = h2e(&files);
    assert_eq!(g.lex_facts.len(), 3);
    assert_eq!(g.parse_lexed.len(), 3);
}

// ─────────────────────────────── H3 ───────────────────────────────

fn h3_sites(files: &[(String, String)]) -> Vec<String> {
    sites(files, "TopItem::LossySpell")
        .into_iter()
        .map(|(f, l)| {
            let shape = if l.contains("TopItem::LossySpell(lossy)") {
                "construct"
            } else if l.contains("TopItem::LossySpell(ls) = it") {
                "or"
            } else if l.contains("TopItem::LossySpell(_) => None")
                || l.contains("TopItem::LossySpell(_) => {}")
            {
                "pass"
            } else {
                "OTHER"
            };
            format!("{f} {shape}")
        })
        .collect()
}

#[test]
fn h3_variant_built_once_matched_at_the_head_and_three_arms() {
    assert_eq!(
        h3_sites(&production_sources()),
        [
            "cli/src/pipeline.rs pass",
            "cli/src/staged.rs pass",
            "elaborate/src/lossy.rs or",
            "elaborate/src/md_return.rs pass",
            "hdl-parser/src/api.rs construct",
        ]
    );
    let lossy = text_of(&production_sources(), "elaborate/src/lossy.rs").to_string();
    assert!(fn_body(&lossy, "design_lossy_spell").contains("for it in &unit.items"));
}

#[test]
fn h3_planted_control_fails() {
    let planted = vec![(
        "elaborate/src/params.rs".to_string(),
        "        if let ast::TopItem::LossySpell(ls) = unit.items.first().unwrap() {}".to_string(),
    )];
    assert_eq!(h3_sites(&planted), ["elaborate/src/params.rs OTHER"]);
}

// ─────────────────────────────── H4 ───────────────────────────────

fn h4_sites(files: &[(String, String)]) -> Vec<String> {
    word_sites(files, "lossy_spell")
        .into_iter()
        .map(|(f, l)| format!("{f}: {l}"))
        .collect()
}

#[test]
fn h4_one_writer_no_reader_while_nothing_reads_it() {
    // While nothing reads it: the first reader edits this list.
    assert_eq!(
        h4_sites(&production_sources()),
        [
            "elaborate/src/api.rs: lossy_spell: el.lossy_spell,",
            "elaborate/src/driver.rs: lossy_spell: ast::LossySpell::default(),",
            "elaborate/src/driver.rs: self.lossy_spell = crate::lossy::design_lossy_spell(unit, &self.top_param_overrides);",
            "elaborate/src/lib.rs: lossy_spell: ast::LossySpell,",
            "elaborate/src/tables.rs: pub lossy_spell: hdl_ast::LossySpell,",
        ]
    );
    let driver = text_of(&production_sources(), "elaborate/src/driver.rs").to_string();
    assert!(
        writer_at_run_head(&driver),
        "the one writer is `run`'s second statement"
    );
}

/// agy F6 (e), taught: the design record is written at the HEAD of `run`, before anything
/// lowers — its second statement, right after the `inside` record's.
fn writer_at_run_head(driver: &str) -> bool {
    let body = fn_body(driver, "run");
    let b = body.as_bytes();
    let (mut stmts, mut start, mut i) = (Vec::new(), 1, 1);
    while i < b.len() - 1 && stmts.len() < 2 {
        if b[i].is_ascii_whitespace() && start == i {
            start += 1;
            i += 1;
            continue;
        }
        let end = item_end(b, i);
        stmts.push(squash(&body[start..end]));
        start = end;
        i = end;
    }
    stmts.len() == 2
        && stmts[0].starts_with("self.inside_name_use =")
        && stmts[1].starts_with("self.lossy_spell = crate::lossy::design_lossy_spell(")
}

#[test]
fn h4_planted_control_fails() {
    let planted = vec![(
        "elaborate/src/params.rs".to_string(),
        "        if self.lossy_spell.param_xz { return; }".to_string(),
    )];
    assert_eq!(h4_sites(&planted).len(), 1, "a reader is seen");
    let not_a_reader = vec![(
        "elaborate/src/lossy.rs".to_string(),
        "pub(crate) fn design_lossy_spell(".to_string(),
    )];
    assert!(
        h4_sites(&not_a_reader).is_empty(),
        "a longer identifier is not the word"
    );
    // agy F6 (e): the writer moved below the first lowering step
    let late = "fn run(&mut self, unit: &U) {\n    self.inside_name_use = None;\n    \
                let (map, order) = build_module_map(unit);\n    \
                self.lossy_spell = crate::lossy::design_lossy_spell(unit, &o);\n}\n";
    assert!(
        !writer_at_run_head(late),
        "the writer after a lowering step"
    );
    let head = "fn run(&mut self, unit: &U) {\n    self.inside_name_use = unit.items.iter().find_map(|it| match it {\n        _ => None,\n    });\n    \
                self.lossy_spell = crate::lossy::design_lossy_spell(unit, &o);\n    x();\n}\n";
    assert!(writer_at_run_head(head), "the writer second");
}

// ─────────────────────────────── H5 ───────────────────────────────

const H5_FILL: &str = "literal::is_fill_literal(t, ast::IntLitKind::UnsizedBased)";
const H5_SIZED: &str = "parse_int_literal(t, ast::IntLitKind::Sized)";

fn h5_check(params: &str) -> Result<(), String> {
    let ovr = fn_body(params, "cli_overrides_for");
    let g = fn_body(params, "cli_value_spells_unknown");
    for (what, body) in [("cli_overrides_for", ovr), ("cli_value_spells_unknown", g)] {
        for needle in [H5_FILL, H5_SIZED] {
            if !body.contains(needle) {
                return Err(format!("{what} does not call `{needle}`"));
            }
        }
        // agy F6 (e), taught: the fill channel is read before the sized parse, in both
        if body.find(H5_FILL) > body.find(H5_SIZED) {
            return Err(format!("{what} reads the sized parse before the fill"));
        }
    }
    let got = calls(g);
    let want = [
        "any",
        "ends_with",
        "fill_is_unknown",
        "is_fill_literal",
        "is_some_and",
        "iter",
        "len",
        "parse_int_literal",
        "starts_with",
        "trim",
    ];
    if got != want {
        return Err(format!("the -G test calls {got:?}"));
    }
    Ok(())
}

#[test]
fn h5_g_test_and_overrides_read_one_raw_to_value_fn() {
    let files = production_sources();
    h5_check(text_of(&files, "elaborate/src/params.rs")).unwrap();
    let callers: Vec<String> = sites(&files, "cli_value_spells_unknown(")
        .into_iter()
        .filter(|(_, l)| !l.contains("fn cli_value_spells_unknown("))
        .map(|(f, _)| f)
        .collect();
    assert_eq!(callers, ["elaborate/src/lossy.rs"], "its one caller");
}

#[test]
fn h5_planted_controls_fail() {
    let ovr = format!(
        "fn cli_overrides_for(x) {{ if {H5_FILL} {{}} let s = crate::literal::{H5_SIZED}; }}\n"
    );
    let other = format!(
        "{ovr}fn cli_value_spells_unknown(raw: &str) -> bool {{\n    let t = raw.trim();\n    \
         if {H5_FILL} {{ return literal::fill_is_unknown(t, k); }}\n    \
         if t.len() >= 2 && t.starts_with('\"') && t.ends_with('\"') {{ return false; }}\n    \
         crate::literal::parse_int_literal(t, ast::IntLitKind::UnsizedBased).is_some_and(|c| c.bits.unk.iter().any(|&w| w != 0))\n}}\n"
    );
    assert!(h5_check(&other).is_err(), "another kind of parse");
    let extra = format!(
        "{ovr}fn cli_value_spells_unknown(raw: &str) -> bool {{\n    let t = raw.trim();\n    \
         if {H5_FILL} {{ return literal::fill_is_unknown(t, k); }}\n    \
         if t.len() >= 2 && t.starts_with('\"') && t.ends_with('\"') {{ return false; }}\n    \
         fill_to_i64(k, t, 32); crate::literal::{H5_SIZED}.is_some_and(|c| c.bits.unk.iter().any(|&w| w != 0))\n}}\n"
    );
    assert!(h5_check(&extra).is_err(), "a second value fn");
    // agy F6 (e): the two reads swapped in the -G test
    let swapped = format!(
        "{ovr}fn cli_value_spells_unknown(raw: &str) -> bool {{\n    let t = raw.trim();\n    \
         if t.len() >= 2 && t.starts_with('\"') && t.ends_with('\"') {{ return false; }}\n    \
         if crate::literal::{H5_SIZED}.is_some_and(|c| c.bits.unk.iter().any(|&w| w != 0)) {{ return true; }}\n    \
         if {H5_FILL} {{ return literal::fill_is_unknown(t, k); }}\n    false\n}}\n"
    );
    assert!(h5_check(&swapped).is_err(), "the sized parse first");
}

// ──────────────── the token walker (H1's whole-Parser rule, H3's spellings, H6) ────────────────

/// One token of a production file: its text (a `Joint` run of punctuation joined, so `+=`,
/// `==`, `..`, `::`, `<=` are one token), its file and the innermost enclosing `fn` (`-`
/// outside any fn). Groups are written as their open and close tokens.
#[derive(Clone, Debug)]
struct Tok {
    t: String,
    file: String,
    fnk: String,
}

/// Rust's multi-character operators.
const OPS: [&str; 24] = [
    "<<=", ">>=", "...", "..=", "::", "->", "=>", "==", "!=", "<=", ">=", "&&", "||", "+=", "-=",
    "*=", "/=", "%=", "^=", "&=", "|=", "<<", ">>", "..",
];

fn tok_walk(ts: TokenStream, file: &str, cur: &str, out: &mut Vec<Tok>) {
    let tt: Vec<TokenTree> = ts.into_iter().collect();
    let mut pending: Option<String> = None;
    let mut i = 0;
    while i < tt.len() {
        let push = |out: &mut Vec<Tok>, t: String, k: &str| {
            out.push(Tok {
                t,
                file: file.to_string(),
                fnk: k.to_string(),
            })
        };
        match &tt[i] {
            TokenTree::Ident(id) if id == "fn" => {
                push(out, "fn".into(), cur);
                if let Some(TokenTree::Ident(name)) = tt.get(i + 1) {
                    pending = Some(name.to_string());
                    push(out, name.to_string(), cur);
                    i += 1;
                }
            }
            TokenTree::Punct(p) if p.as_char() == ';' => {
                pending = None;
                push(out, ";".into(), cur);
            }
            TokenTree::Group(g) => {
                let key = match (g.delimiter(), &pending) {
                    (Delimiter::Brace, Some(f)) => f.clone(),
                    _ => cur.to_string(),
                };
                if g.delimiter() == Delimiter::Brace {
                    pending = None;
                }
                let (o, c) = match g.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::None => ("", ""),
                };
                if !o.is_empty() {
                    push(out, o.into(), &key);
                }
                tok_walk(g.stream(), file, &key, out);
                if !c.is_empty() {
                    push(out, c.into(), &key);
                }
            }
            TokenTree::Punct(p) => {
                // a `Joint` run, split into Rust's operators by longest match (`::*` is `::`
                // then `*`, `..=` one token)
                let mut run = p.as_char().to_string();
                let mut joint = p.spacing() == Spacing::Joint;
                while joint {
                    match tt.get(i + 1) {
                        Some(TokenTree::Punct(q)) if q.as_char() != ';' => {
                            run.push(q.as_char());
                            joint = q.spacing() == Spacing::Joint;
                            i += 1;
                        }
                        _ => break,
                    }
                }
                let mut rest = run.as_str();
                while !rest.is_empty() {
                    let n = [3, 2]
                        .into_iter()
                        .find(|&n| rest.get(..n).is_some_and(|op| OPS.contains(&op)))
                        .unwrap_or(1);
                    push(out, rest[..n].to_string(), cur);
                    rest = &rest[n..];
                }
            }
            TokenTree::Ident(id) => push(out, id.to_string(), cur),
            TokenTree::Literal(l) => push(out, l.to_string(), cur),
        }
        i += 1;
    }
}

/// The tokens of a set of production files (comments and `#[cfg(test)]` items already cut).
fn toks_of(files: &[(String, String)]) -> Vec<Tok> {
    let mut out = Vec::new();
    for (f, t) in files {
        let ts = TokenStream::from_str(t).unwrap_or_else(|e| panic!("{f}: tokenize: {e:?}"));
        tok_walk(ts, f, "-", &mut out);
    }
    out
}

fn tt(t: &[Tok], i: usize) -> &str {
    t.get(i).map_or("", |x| x.t.as_str())
}

// ─────────────── H1 (N5): no whole-Parser restore in hdl-parser ───────────────

/// N5 of fix N4: no `* self =` and no `mem :: replace | swap | take` whose first argument is
/// `self` itself (`self`, `& mut * self`, `* self`) in hdl-parser's source: a restore of the
/// whole parser would bring back the two lossy bools' and the mark list's old values (0
/// sites today). A field of `self` as the argument (`& mut self . in_cont_assign`, the
/// crate's own idiom) is not a whole-Parser restore and passes ; `& mut self . pos` is H6 (c).
fn whole_parser_restores(t: &[Tok]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..t.len() {
        if tt(t, i) == "*" && tt(t, i + 1) == "self" && tt(t, i + 2) == "=" {
            out.push(format!("{} {}: * self =", t[i].file, t[i].fnk));
        }
        if tt(t, i) == "mem"
            && tt(t, i + 1) == "::"
            && ["replace", "swap", "take"].contains(&tt(t, i + 2))
            && tt(t, i + 3) == "("
        {
            let mut j = i + 4;
            while ["&", "mut", "*"].contains(&tt(t, j)) {
                j += 1;
            }
            if tt(t, j) == "self" && [",", ")"].contains(&tt(t, j + 1)) {
                out.push(format!(
                    "{} {}: mem::{}(self",
                    t[i].file,
                    t[i].fnk,
                    tt(t, i + 2)
                ));
            }
        }
    }
    out
}

#[test]
fn h1_no_whole_parser_restore() {
    let files = only(&production_sources(), "hdl-parser/src/");
    assert_eq!(
        whole_parser_restores(&toks_of(&files)),
        Vec::<String>::new()
    );
}

#[test]
fn h1_whole_parser_planted_controls_fail() {
    let planted = |src: &str| {
        whole_parser_restores(&toks_of(&[(
            "hdl-parser/src/x.rs".to_string(),
            src.to_string(),
        )]))
    };
    assert_eq!(planted("fn f(&mut self, s: Self) { *self = s; }").len(), 1);
    assert_eq!(
        planted("fn f(&mut self, s: Self) { let o = std::mem::replace(self, s); }").len(),
        1
    );
    assert_eq!(
        planted("fn f(&mut self, s: &mut Self) { core::mem::swap(&mut *self, s); }").len(),
        1
    );
    assert_eq!(
        planted("fn f(&mut self) { let o = std::mem::take(self); }").len(),
        1
    );
    // the crate's own idiom: a field, not the parser
    assert!(planted("fn f(&mut self) { let o = std::mem::take(&mut self.idx_scope); }").is_empty());
}

// ─────────────── H3 (N5): the variant's spellings ───────────────

/// N5 of fix N4: `LossySpell (` (the variant: the struct is built with `{` or `default`) is
/// spelled only as `TopItem :: LossySpell (` — but at the variant's definition in hdl-ast —
/// and no crate imports the variants (`TopItem :: *`, `TopItem :: {`, `TopItem as`), so the
/// path H3 counts is the only one a match or a construction can take.
fn variant_spellings(t: &[Tok]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..t.len() {
        if tt(t, i) == "LossySpell"
            && tt(t, i + 1) == "("
            && !(i >= 2 && tt(t, i - 2) == "TopItem" && tt(t, i - 1) == "::")
            && !(t[i].file == "hdl-ast/src/lib.rs" && tt(t, i + 2) == "LossySpell")
        {
            out.push(format!(
                "{} {}: unqualified LossySpell (",
                t[i].file, t[i].fnk
            ));
        }
        if tt(t, i) == "TopItem"
            && (tt(t, i + 1) == "as"
                || (tt(t, i + 1) == "::" && ["*", "{"].contains(&tt(t, i + 2))))
        {
            out.push(format!(
                "{} {}: TopItem {} {}",
                t[i].file,
                t[i].fnk,
                tt(t, i + 1),
                tt(t, i + 2)
            ));
        }
    }
    out
}

#[test]
fn h3_variant_has_one_spelling() {
    assert_eq!(
        variant_spellings(&toks_of(&production_sources())),
        Vec::<String>::new()
    );
}

#[test]
fn h3_spelling_planted_controls_fail() {
    let planted = |src: &str| {
        variant_spellings(&toks_of(&[(
            "elaborate/src/x.rs".to_string(),
            src.to_string(),
        )]))
    };
    assert_eq!(
        planted("use hdl_ast::TopItem::*; fn f(it: &T) { if let LossySpell(ls) = it {} }").len(),
        2
    );
    assert_eq!(
        planted("use hdl_ast::TopItem::{LossySpell, Module};").len(),
        1
    );
    assert_eq!(
        planted("use hdl_ast::TopItem as I; fn f(it: &T) { if let I::LossySpell(ls) = it {} }")
            .len(),
        2
    );
    assert!(planted("fn f(it: &T) { if let ast::TopItem::LossySpell(ls) = it {} }").is_empty());
}

// ─────────────── H6: the right-side marks (fix N4, §5.2 Q-LOSSYSPELL U4) ───────────────

/// H6 (a): every token naming one of the parser's two token-index lists, as `file fn word`.
/// `lossy_whole_rhs`: the field and its initialiser (`lib.rs`), `rewind`'s truncation, the
/// one push in `assign_rhs` (`lossy.rs`), the one read in `parse_lexed` (`api.rs`).
/// `inside_kw_at` (premise judge B-3, RULINGS L1-19: the same shape, the same stale-entry
/// class): the field, the initialiser, `rewind`, its two pushes, its one read.
fn h6a(t: &[Tok]) -> Vec<String> {
    t.iter()
        .filter(|x| x.t == "lossy_whole_rhs" || x.t == "inside_kw_at")
        .map(|x| format!("{} {} {}", x.file, x.fnk, x.t))
        .collect()
}

/// H6 (b): the callers of `assign_rhs` as `file fn` ; a token `assign_rhs` that is neither
/// the definition nor a call (a fn value) is listed as `VALUE`.
fn h6b(t: &[Tok]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..t.len() {
        if t[i].t != "assign_rhs" || (i > 0 && tt(t, i - 1) == "fn") {
            continue;
        }
        if tt(t, i + 1) == "(" {
            out.push(format!("{} {}", t[i].file, t[i].fnk));
        } else {
            out.push(format!("VALUE {} {}", t[i].file, t[i].fnk));
        }
    }
    out
}

/// What may follow `<recv> . pos` (premise judge L2 (ii)): every follower the census found.
const POS_FOLLOWERS: [&str; 14] = [
    ";", ")", ",", "]", "..", "==", "!=", "<", ">", "<=", ">=", "+", "-", "+=",
];

/// H6 (c), premise judge L2: (i) `. pos =` only inside `rewind` (`lib.rs`) ; (ii) after
/// `. pos` the next token is one of `POS_FOLLOWERS` (anything else — `-=`, `.method(`, `*=`
/// — is a hit) ; (iii) no `& mut … . pos` ; (iv) the bare word `pos` (not after `.`) only at
/// the field's declaration and in `Parser::new`. The receiver is any expression, not only
/// `self` (stricter than L2's letter: no other type of hdl-parser has a field `pos`, which
/// (iv) holds).
fn h6c(t: &[Tok]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..t.len() {
        if t[i].t != "pos" {
            continue;
        }
        let at = format!("{} {}", t[i].file, t[i].fnk);
        if i > 0 && tt(t, i - 1) == "." {
            let f = tt(t, i + 1);
            let in_rewind = t[i].file == "hdl-parser/src/lib.rs" && t[i].fnk == "rewind";
            if f == "=" {
                if !in_rewind {
                    out.push(format!("(i) {at}: . pos ="));
                }
            } else if !POS_FOLLOWERS.contains(&f) {
                out.push(format!("(ii) {at}: . pos {f}"));
            }
            // the receiver path (`a . b . pos`) back to its first word, then `& mut` before it
            let mut j = i.saturating_sub(2);
            while j >= 2 && tt(t, j - 1) == "." {
                j -= 2;
            }
            if j >= 2 && tt(t, j - 1) == "mut" && tt(t, j - 2) == "&" {
                out.push(format!("(iii) {at}: & mut .. . pos"));
            }
        } else {
            let site = format!("{at} pos {}", tt(t, i + 1));
            let field = "hdl-parser/src/lib.rs - pos :";
            let ctor = "hdl-parser/src/lib.rs new pos :";
            if site != field && site != ctor {
                out.push(format!("(iv) {site}"));
            }
        }
    }
    out
}

/// The mark sites of fix N4 (plan section 1.3 ; premise judge L4 names S7's fn): `assign_rhs`
/// is called here and nowhere else. A ninth caller fails until it is listed here with the
/// cell that measured its plain fill right or loud (PR16).
const H6_SITES: [&str; 8] = [
    "hdl-parser/src/blocks_timing.rs parse_force",
    "hdl-parser/src/decls.rs parse_cont_assign",
    "hdl-parser/src/decls.rs parse_decl_name_list",
    "hdl-parser/src/stmt.rs parse_assign_or_call",
    "hdl-parser/src/stmt.rs parse_assign_or_call",
    "hdl-parser/src/stmt.rs parse_proc_assign",
    "hdl-parser/src/stmt_ctl.rs build_for_typed_init",
    "hdl-parser/src/stmt_ctl.rs parse_for_assign",
];

#[test]
fn h6_marks_one_writer_eight_sites_one_rewind() {
    let files = only(&production_sources(), "hdl-parser/src/");
    let t = toks_of(&files);
    let mut a = h6a(&t);
    a.sort();
    let mut b = h6b(&t);
    b.sort();
    let a: Vec<&str> = a.iter().map(String::as_str).collect();
    let b: Vec<&str> = b.iter().map(String::as_str).collect();
    // the three read together, so a failure shows each
    assert_eq!(
        (a, b, h6c(&t)),
        (
            vec![
                "hdl-parser/src/api.rs parse_lexed inside_kw_at",
                "hdl-parser/src/api.rs parse_lexed lossy_whole_rhs",
                "hdl-parser/src/expr.rs expr_capped inside_kw_at",
                "hdl-parser/src/lib.rs - inside_kw_at",
                "hdl-parser/src/lib.rs - lossy_whole_rhs",
                "hdl-parser/src/lib.rs new inside_kw_at",
                "hdl-parser/src/lib.rs new lossy_whole_rhs",
                // `rewind`: the `.last()` test and the `.pop()`, each list
                "hdl-parser/src/lib.rs rewind inside_kw_at",
                "hdl-parser/src/lib.rs rewind inside_kw_at",
                "hdl-parser/src/lib.rs rewind lossy_whole_rhs",
                "hdl-parser/src/lib.rs rewind lossy_whole_rhs",
                "hdl-parser/src/lossy.rs assign_rhs lossy_whole_rhs",
                "hdl-parser/src/stmt_ctl.rs parse_case_inside inside_kw_at",
            ],
            H6_SITES.to_vec(),
            Vec::<String>::new()
        ),
        "(a) the two lists' sites, (b) assign_rhs's callers, (c) the cursor's writers"
    );
    // `rewind` truncates both lists to `save` and sets the cursor ; `assign_rhs` pushes the
    // cursor only on the fill kind, then parses the expression.
    let lib = text_of(&files, "hdl-parser/src/lib.rs");
    let rw = squash(fn_body(lib, "rewind"));
    for needle in [
        "self.pos = save;",
        "while self.lossy_whole_rhs.last().is_some_and(|&m| m >= save) {",
        "while self.inside_kw_at.last().is_some_and(|&m| m >= save) {",
    ] {
        assert!(rw.contains(needle), "rewind: `{needle}` in `{rw}`");
    }
    let lossy = text_of(&files, "hdl-parser/src/lossy.rs");
    let ar = squash(fn_body(lossy, "assign_rhs"));
    assert!(
        ar.contains(
            "if self.peek() == Some(TokenKind::IntUnsizedBasedXz) { \
             self.lossy_whole_rhs.push(self.pos); }"
        ),
        "assign_rhs: `{ar}`"
    );
}

#[test]
fn h6_planted_controls_fail() {
    let one = |file: &str, src: &str| toks_of(&[(file.to_string(), src.to_string())]);
    // (a) a third writer of a list
    let t = one(
        "hdl-parser/src/expr.rs",
        "fn f(&mut self) { self.lossy_whole_rhs.push(self.pos); }",
    );
    assert_eq!(h6a(&t), ["hdl-parser/src/expr.rs f lossy_whole_rhs"]);
    // (b) a ninth site, and the fn as a value
    let t = one(
        "hdl-parser/src/params.rs",
        "fn finish_param_assignment(&mut self) { let v = self.assign_rhs(); \
         let f = Self::assign_rhs; }",
    );
    assert_eq!(
        h6b(&t),
        [
            "hdl-parser/src/params.rs finish_param_assignment",
            "VALUE hdl-parser/src/params.rs finish_param_assignment"
        ]
    );
    // (c)(i) a restore outside `rewind`
    let t = one(
        "hdl-parser/src/type_params.rs",
        "fn g(&mut self) { let save = self.pos; self.pos = save; }",
    );
    assert_eq!(h6c(&t), ["(i) hdl-parser/src/type_params.rs g: . pos ="]);
    // (c)(i) the right name in the wrong file
    let t = one(
        "hdl-parser/src/expr.rs",
        "fn rewind(&mut self, save: usize) { self.pos = save; }",
    );
    assert_eq!(h6c(&t).len(), 1);
    // (c)(ii) `-=`, `*=`, a method on the cursor
    for src in [
        "fn g(&mut self) { self.pos -= 1; }",
        "fn g(&mut self) { self.pos *= 1; }",
        "fn g(&mut self) { let p = self.pos.saturating_sub(1); }",
    ] {
        let t = one("hdl-parser/src/x.rs", src);
        assert_eq!(h6c(&t).len(), 1, "`{src}`");
    }
    // (c)(iii) a `&mut` alias of the cursor, as the crate's `mem::replace` idiom spells it
    let t = one(
        "hdl-parser/src/x.rs",
        "fn g(&mut self, save: usize) { let o = std::mem::replace(&mut self.pos, save); }",
    );
    assert_eq!(h6c(&t), ["(iii) hdl-parser/src/x.rs g: & mut .. . pos"]);
    // (c)(iv) a destructured cursor
    let t = one(
        "hdl-parser/src/x.rs",
        "fn g(&mut self) { let Parser { pos, .. } = self; *pos = 0; }",
    );
    assert_eq!(
        h6c(&t),
        [
            "(iv) hdl-parser/src/x.rs g pos ,",
            "(iv) hdl-parser/src/x.rs g pos ="
        ]
    );
    // the followers the tree uses pass
    let t = one(
        "hdl-parser/src/x.rs",
        "fn g(&mut self) { let a = self.toks.get(self.pos); self.pos += 1; \
         let b = self.pos == 0 || self.pos >= self.toks.len(); let c = &self.toks[self.pos]; }",
    );
    assert_eq!(h6c(&t), Vec::<String>::new());
}

// ─────────────── the reader itself (agy F6 (b), (c)) ───────────────

#[test]
fn production_of_planted_controls_fail() {
    let read = |files: &[(&str, &str)]| {
        production_of(
            files
                .iter()
                .map(|(f, t)| (f.to_string(), t.to_string()))
                .collect(),
        )
    };
    let bad = "self.lossy_let_any = s.saved;";
    // (c) a comment is cut wherever it sits ; a string's `//` is no comment
    let got = read(&[(
        "hdl-parser/src/scope.rs",
        &format!(
            "fn f() {{ /* {bad} */ let s = \"a // b\"; }} // {bad}\n/* nested /* {bad} */ */\n"
        ),
    )]);
    assert_eq!(h1_violations(&got), Vec::<(String, String)>::new());
    assert!(
        got[0].1.contains("\"a // b\""),
        "the string kept: {}",
        got[0].1
    );
    // (b) production code after a `#[cfg(test)]` item is read ; the item is not
    let got = read(&[(
        "hdl-parser/src/scope.rs",
        &format!(
            "#[cfg(test)]\n#[allow(dead_code)]\nmod t {{\n    fn g() {{ let c = '}}'; {bad} }}\n}}\n\
             fn restore(&mut self) {{\n    {bad}\n}}\n"
        ),
    )]);
    assert_eq!(h1_violations(&got).len(), 1, "{:?}", got);
    // (b) a file module `#[cfg(test)] mod NAME;` declares is dropped, whatever its name ; a
    // file no `#[cfg(test)]` declares is read
    let body = format!("fn restore(&mut self) {{\n    {bad}\n}}\n");
    let got = read(&[
        (
            "sim-engine/src/native/mod.rs",
            "#[cfg(test)]\nmod kernel_tests;\nmod run;\n",
        ),
        ("sim-engine/src/native/kernel_tests.rs", &body),
        ("sim-engine/src/native/run.rs", &body),
        ("hdl-parser/src/lib.rs", "#[cfg(test)]\nmod tests;\n"),
        ("hdl-parser/src/tests/lossy_spell.rs", &body),
        ("hdl-parser/src/expr.rs", "#[cfg(test)]\nmod inner_tests;\n"),
        ("hdl-parser/src/expr/inner_tests.rs", &body),
        ("hdl-parser/src/scope_tests.rs", &body),
    ]);
    let names: Vec<&str> = got.iter().map(|(f, _)| f.as_str()).collect();
    assert_eq!(
        names,
        [
            "sim-engine/src/native/mod.rs",
            "sim-engine/src/native/run.rs",
            "hdl-parser/src/lib.rs",
            "hdl-parser/src/expr.rs",
            "hdl-parser/src/scope_tests.rs",
        ]
    );
    assert_eq!(h1_violations(&got).len(), 2, "run.rs and scope_tests.rs");
}
