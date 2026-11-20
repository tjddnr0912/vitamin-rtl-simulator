//! §5.2 Q-LOSSYSPELL: the lossy-spelling record has one home per side (ER §5.1) — the
//! parser's scan (`crates/hdl-parser/src/lossy.rs`) and the design's OR
//! (`crates/elaborate/src/lossy.rs`). This test pins, over the production sources
//! (`crates/*/src/**/*.rs`, `src/tests/`, `src/tests.rs` and a trailing `#[cfg(test)]` item
//! skipped, comment lines skipped), what the record rests on and who touches it:
//!
//! - H1 the parser's two lossy bools `lossy_sign_kw_param` and `lossy_let_any` are each
//!   declared and initialised in `lib.rs`, written only by `|=` in `lossy.rs` and read
//!   once in `api.rs`; neither is a field of `ScopeSnapshot`, so no scope restore can clear
//!   it (premise judge 2 PR13, L9 ; PLAN3 PR13 for the second, §5.2 Q-LOSSYSPELL U3).
//! - H2 (a) `scan_lossy (` has one caller, `api.rs`'s `parse_lexed`, under
//!   `if facts.xz_literal()` (U3: the pass runs only on a unit whose lexer saw an x / z
//!   literal) ; (b) `is_let_head (` has one, the module-item dispatch, and the string
//!   `"let"` is spelled in no hdl-parser source but `lossy.rs` ; (b') `note_let_decl (`
//!   has one caller, the first statement of `parse_let_decl` (`sva_seq.rs`), and
//!   `LetDecl {` is built in that fn only ; (c) `new_param (` in hdl-parser = one call with
//!   `Signing :: from_written` (`params.rs`, `param_dtype`) + four with the literal
//!   `Signing :: Implicit` ; (e) `lex_facts (` and `parse_lexed (` each have one production
//!   caller outside their own crate, cli `frontend.rs` (inside it: `lex` and
//!   `parse_with_warnings`, the wrappers) ; `LexFacts {` is the definition and two
//!   constructions, in `lex_facts` and `LexFacts :: of_tokens`, both in hdl-lexer ; no
//!   `Default`, no `From`, no `new`, no `pub` field on `LexFacts` (premise judge 3 L2) ;
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
//!   the three pass-through arms (`staged.rs`, `pipeline.rs`, `md_return.rs`).
//! - H4 the design record `lossy_spell` (a word, not `design_lossy_spell`) has ONE writer,
//!   `run`'s head in `driver.rs`, the field and its initialiser, the hand-off in `api.rs`
//!   and the `Sidecars` field in `tables.rs` — and 0 production readers while nothing
//!   reads it: no other line of any crate's source names it (the first reader edits this
//!   rule, as the write view's first reader will edit wext_home's H3).
//! - H5 the `-G` test `cli_value_spells_unknown` and `cli_overrides_for` (`params.rs`)
//!   read one text through the same raw → value fns: both call
//!   `literal :: is_fill_literal (t , ast :: IntLitKind :: UnsizedBased)` and
//!   `parse_int_literal (t , ast :: IntLitKind :: Sized)`, the `-G` test calls no other
//!   value fn (its calls are exactly `is_fill_literal`, `fill_is_unknown`,
//!   `parse_int_literal` and std verbs), and its one caller is the design's OR.
//!
//! Each rule has a planted control that must fail.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// Every production `.rs` file under `crates/*/src`, as (path relative to `crates/`, text
/// with a trailing `#[cfg(test)]` item cut off).
fn production_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let mut ents: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        ents.sort();
        for p in ents {
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == "tests") {
                    continue;
                }
                walk(&p, out);
            } else if p.extension().is_some_and(|e| e == "rs")
                && p.file_name().is_some_and(|n| n != "tests.rs")
            {
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
    files
        .into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p).unwrap();
            let rel = p
                .strip_prefix(&crates)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            (rel, strip_test_tail(&text))
        })
        .collect()
}

/// The text up to a `#[cfg(test)]` line that is followed by `mod` (an inline test
/// module), so its pins do not count as production sites.
fn strip_test_tail(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        if l.trim() == "#[cfg(test)]"
            && lines
                .get(i + 1)
                .is_some_and(|n| n.trim_start().starts_with("mod "))
        {
            return lines[..i].join("\n");
        }
    }
    text.to_string()
}

/// Code lines only (a line whose first non-blank characters are `//` is a comment).
fn code_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter(|l| !l.trim_start().starts_with("//"))
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

fn h2_counts(files: &[(String, String)]) -> (Vec<String>, Vec<String>, Vec<String>, usize, usize) {
    let scan = callers(files, "scan_lossy(", "fn scan_lossy(");
    let let_head = callers(files, "is_let_head(", "fn is_let_head(");
    let let_str: Vec<String> = sites(files, "\"let\"")
        .into_iter()
        .map(|(f, _)| f)
        .collect();
    // `new_param(` spans lines: read each call's argument text up to its `)`.
    let (mut written, mut implicit) = (0, 0);
    for (f, t) in files {
        let _ = f;
        for (at, _) in t.match_indices("DataType::new_param(") {
            let args = &t[at..at + t[at..].find(')').unwrap_or(t.len() - at)];
            if args.contains("Signing::from_written(") {
                written += 1;
            } else if args.contains("Signing::Implicit") {
                implicit += 1;
            }
        }
    }
    (scan, let_head, let_str, written, implicit)
}

/// H2 (a)'s gate: `parse_lexed`'s body runs the scan only under the lexer's fact.
fn scan_is_gated(api: &str) -> bool {
    squash(fn_body(api, "parse_lexed")).contains(
        "let mut lossy = if facts.xz_literal() { lossy::scan_lossy(tokens, src) } else { \
         LossySpell::default() };",
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
    let (scan, let_head, let_str, written, implicit) = h2_counts(&files);
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
    assert_eq!((written, implicit), (1, 4), "(c) new_param's calls");
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
    let (scan, let_head, let_str, written, _) = h2_counts(&planted);
    assert_eq!(scan.len(), 2, "a second caller of scan_lossy is seen");
    assert_eq!(let_head, ["hdl-parser/src/stmt.rs"]);
    assert_eq!(let_str, ["hdl-parser/src/stmt.rs"]);
    assert_eq!(written, 1);
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
            let default_derive = l.starts_with("#[derive(")
                && l.contains("Default")
                && lines
                    .get(i + 1)
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
        if let Some(at) = lexer.find("impl LexFacts {") {
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
        fn_body(&driver, "run").contains("self.lossy_spell = crate::lossy::design_lossy_spell("),
        "the one writer sits in `run`"
    );
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
}
