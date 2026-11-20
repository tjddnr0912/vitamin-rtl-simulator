//! §5.2 Q-LOSSYSPELL fix H: THE FILL RULE has one home (ER §5.1) — a fill on an
//! assignment's right side is lowered ONCE, at the assignment, under the one value
//! `fill_context` (`edge_gate.rs`) gives for the whole target, matched by the assignment
//! funnel `resize_rhs_for_lvalue` (`expr_ctx.rs`) with no `_` arm, and every fill lowered
//! under an undecided target leaves a record the one end-phase decision
//! (`verify_use_recs`) ends: its slot re-sized in place, or an error. This test pins, over
//! elaborate's production sources (comments and every `#[cfg(test)]` item cut, string
//! literals kept: the reader `lossy_home.rs` uses), what that rests on:
//!
//! - HF2 `resize_rhs_for_lvalue` holds exactly one `match self.fill_context(`, with the four
//!   arms `TargetW::Bits`, `TargetW::Late { visible, f2, resizable }`, `TargetW::NoBits`,
//!   `TargetW::Unprovable` and no `_ =>` arm, no `..` ; and `fill_context(` has no other
//!   caller (one value, one consumer).
//! - HF3 the fill record `record_fill(` is called at exactly two sites, both inside
//!   `lower_expr_ctx_inner`'s fill-literal arm (the all-ones / all-zeros spelling and the
//!   `'x` / `'z` constant) ; `fill_literal_const(` is called at the pinned (file, count)
//!   table: three IR sites (`expr_main.rs` self-determined, `expr_ctx.rs` in a context,
//!   `edge_gate.rs` the decision's re-size) and the constant-domain folds.
//! - HF4 the deleted second lowering does not come back under its names: none of
//!   `pending_fill_width`, `PendingFillWidth`, `scope_free`, `scope_free_fill_expr`,
//!   `every_name_resolves`, `decided_lvalue_width`, `resolve_pending_fill_widths` is an
//!   identifier anywhere in elaborate's production sources.
//! - HF5 `ir_lvalue_width(` (whose `.unwrap_or(1)` answers 1 for a width nobody
//!   established) is called only by the two non-fill wrappers `ia_capture_width` and
//!   `stream_target_width`, and `fill_context` does not call it.
//! - HF6 in `verify_use_recs` the `Fill` branch's first statement is the unmeasured test
//!   and its first branch the `had_error` test (the order (j1) … (j5) in the fn's doc: the
//!   real-piece test, then the need, then the re-size, then the deferred-piece refusal) ;
//!   the re-size is the one `self.patch_expr(slot.eid,` and the decision appends no
//!   expression (no `push_expr(`, no `const_u32_expr(`: the Resolve phase forbids an
//!   append outside a site).
//! - HF7 a target not provable is judged once, at the end: the funnel's `Unprovable` arm
//!   is exactly `self.record_unprovable(lv, site); rhs_id` (it reports nothing: a width
//!   that failed to fold for a reason already reported reaches it too), and
//!   `verify_use_recs` opens with `let quiet = self.had_error;` read once, before any
//!   record, and its first branch speaks for an `Unprovable` record only under `!quiet`
//!   (so the order of the records cannot silence one).
//! - HF8 every error the fill rule adds is one constant that prints no value (RULINGS
//!   L2-53: the reasons and numbers the rule's first builds printed were read from bounds
//!   folded for another purpose, four times): `refuse_fill` holds no `format!`, no string
//!   literal and no `{`, and names only the listed constant(s) ; `verify_use_recs` calls
//!   it at its four refusals and names no `FILL_` constant itself ; nothing else calls it ;
//!   every `const FILL_` of `edge_gate.rs` is listed, and its text holds no digit and no
//!   `IEEE` (no number, no clause).
//!
//! WHAT THESE RULES CANNOT SEE (they read text, not types): a fill lowered by a caller
//! that skips the funnel (HF1, a census, holds the assignment family ; the structural
//! closure is §5.2 Q-FILLCTX) ; a new left-side form (it reaches `fill_context` as chunks:
//! the exhaustive per-piece fold refuses what it cannot measure) ; a reader of a late
//! fill's slot between its lowering and the decision (the plan's P5 census, REVIEW-ONLY).
//! The values are `crates/cli/tests/fill_matrix.rs`'s.
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

fn text_of<'a>(files: &'a [(String, String)], rel: &str) -> &'a str {
    &files
        .iter()
        .find(|(f, _)| f == rel)
        .unwrap_or_else(|| panic!("{rel} not found"))
        .1
}

/// Every whitespace run as one space: rustfmt's layout does not move a rule.
fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn elab() -> Vec<(String, String)> {
    only(&production_sources(), "elaborate/src/")
}

/// `files` with `rel`'s text replaced by `f(text)` (a planted control).
fn planted(
    files: &[(String, String)],
    rel: &str,
    f: impl Fn(&str) -> String,
) -> Vec<(String, String)> {
    files
        .iter()
        .map(|(r, t)| (r.clone(), if r == rel { f(t) } else { t.clone() }))
        .collect()
}

/// `text` with `old` (present exactly once) replaced by `new`.
fn swap(text: &str, old: &str, new: &str) -> String {
    assert_eq!(text.matches(old).count(), 1, "planted anchor `{old}`");
    text.replacen(old, new, 1)
}

/// Call sites of `name(`, the definition `fn name(` left out.
fn call_sites(files: &[(String, String)], name: &str) -> Vec<(String, String)> {
    sites(files, &format!("{name}("))
        .into_iter()
        .filter(|(_, l)| !l.contains(&format!("fn {name}(")))
        .collect()
}

const EXPR_CTX: &str = "elaborate/src/expr_ctx.rs";
const EDGE_GATE: &str = "elaborate/src/edge_gate.rs";

// ─────────────────────────────── HF2 ───────────────────────────────

const HF2_ARMS: [&str; 4] = [
    "TargetW::Bits(w) =>",
    "TargetW::Late { visible, f2, resizable, } =>",
    "TargetW::NoBits =>",
    "TargetW::Unprovable =>",
];

fn hf2_violations(files: &[(String, String)]) -> Vec<String> {
    let body = squash(fn_body(text_of(files, EXPR_CTX), "resize_rhs_for_lvalue"));
    let needle = "match self.fill_context(";
    let n = body.matches(needle).count();
    if n != 1 {
        return vec![format!("{n} `{needle}` in the funnel")];
    }
    let at = body.find(needle).unwrap();
    let open = at + body[at..].find('{').expect("match block");
    let blk = &body[open..group_end(body.as_bytes(), open)];
    let mut v = Vec::new();
    if blk.contains("_ =>") {
        v.push("a `_ =>` arm".to_string());
    }
    if blk.contains("..") {
        v.push("a `..` in the match".to_string());
    }
    for arm in HF2_ARMS {
        if blk.matches(arm).count() != 1 {
            v.push(format!("arm `{arm}` not there once"));
        }
    }
    if blk.matches("=>").count() != HF2_ARMS.len() {
        v.push(format!(
            "{} arms, want {}",
            blk.matches("=>").count(),
            HF2_ARMS.len()
        ));
    }
    let callers = call_sites(files, "fill_context");
    if callers.len() != 1 || callers[0].0 != EXPR_CTX {
        v.push(format!("fill_context callers {callers:?}"));
    }
    v
}

#[test]
fn hf2_the_funnel_matches_the_one_value_exhaustively() {
    let v = hf2_violations(&elab());
    assert!(v.is_empty(), "{v:?}");
}

#[test]
fn hf2_planted_controls_fail() {
    let files = elab();
    let wild = planted(&files, EXPR_CTX, |t| {
        swap(t, "TargetW::NoBits => rhs_id,", "_ => rhs_id,")
    });
    assert!(!hf2_violations(&wild).is_empty(), "a `_` arm passed");
    let rest = planted(&files, EXPR_CTX, |t| {
        swap(
            t,
            "                resizable,\n            } =>",
            "                ..\n            } =>",
        )
    });
    assert!(!hf2_violations(&rest).is_empty(), "a `..` passed");
    let second = planted(&files, EDGE_GATE, |t| {
        format!("{t}\nfn planted(e: &Elaborator) {{ let _ = e.fill_context(lv); }}\n")
    });
    assert!(
        !hf2_violations(&second).is_empty(),
        "a second fill_context caller passed"
    );
}

// ─────────────────────────────── HF3 ───────────────────────────────

/// `fill_literal_const(` per file (the definition in `literal.rs` left out): the three IR
/// sites and the constant-domain folds.
const HF3_FILL_CONST: [(&str, usize, &str); 7] = [
    (
        "elaborate/src/const_bound.rs",
        2,
        "constant domain: a bound's fold",
    ),
    (
        "elaborate/src/const_eval.rs",
        1,
        "constant domain: the evaluator's literal",
    ),
    (
        "elaborate/src/const_wide.rs",
        2,
        "constant domain: the wide fold",
    ),
    (
        "elaborate/src/edge_gate.rs",
        1,
        "IR: the decision's re-size of a late fill's slot (j4)",
    ),
    (
        "elaborate/src/expr_ctx.rs",
        1,
        "IR: a fill in a context (lower_expr_ctx_inner)",
    ),
    (
        "elaborate/src/expr_main.rs",
        1,
        "IR: a self-determined fill (lower_expr)",
    ),
    (
        "elaborate/src/params.rs",
        1,
        "constant domain: a parameter value",
    ),
];

fn hf3_violations(files: &[(String, String)]) -> Vec<String> {
    let mut v = Vec::new();
    let rec = call_sites(files, "record_fill");
    if rec.len() != 2 || rec.iter().any(|(f, _)| f != EXPR_CTX) {
        v.push(format!("record_fill sites {rec:?}"));
    }
    let body = fn_body(text_of(files, EXPR_CTX), "lower_expr_ctx_inner");
    let arm = body
        .find("} if literal::is_fill_literal(raw, *kind) => {")
        .map(|a| {
            let open = a + body[a..].find("=> {").unwrap() + 3;
            &body[open..group_end(body.as_bytes(), open)]
        })
        .unwrap_or("");
    if arm.matches("self.record_fill(").count() != 2 {
        v.push("the two record_fill calls are not in the fill-literal arm".to_string());
    }
    let mut got = std::collections::BTreeMap::new();
    for (f, _) in call_sites(files, "fill_literal_const") {
        *got.entry(f).or_insert(0usize) += 1;
    }
    let want: std::collections::BTreeMap<String, usize> = HF3_FILL_CONST
        .iter()
        .map(|&(f, n, _)| (f.to_string(), n))
        .collect();
    if got != want {
        v.push(format!("fill_literal_const sites {got:?}"));
    }
    v
}

#[test]
fn hf3_the_fill_record_and_the_fill_constant_sites() {
    let v = hf3_violations(&elab());
    assert!(v.is_empty(), "{v:?}");
}

#[test]
fn hf3_planted_controls_fail() {
    let files = elab();
    let third = planted(&files, "elaborate/src/stmt_main.rs", |t| {
        format!("{t}\nfn planted(e: &mut Elaborator) {{ e.record_fill(f2, 1, None); }}\n")
    });
    assert!(
        !hf3_violations(&third).is_empty(),
        "a third record_fill passed"
    );
    let ir = planted(&files, "elaborate/src/events.rs", |t| {
        format!("{t}\nfn planted() {{ let _ = literal::fill_literal_const(raw, k, 8); }}\n")
    });
    assert!(
        !hf3_violations(&ir).is_empty(),
        "a fourth fill constant site passed"
    );
}

// ─────────────────────────────── HF4 ───────────────────────────────

const HF4_GONE: [&str; 7] = [
    "pending_fill_width",
    "PendingFillWidth",
    "scope_free",
    "scope_free_fill_expr",
    "every_name_resolves",
    "decided_lvalue_width",
    "resolve_pending_fill_widths",
];

fn hf4_violations(files: &[(String, String)]) -> Vec<(String, String)> {
    HF4_GONE.iter().flat_map(|w| word_sites(files, w)).collect()
}

#[test]
fn hf4_the_deleted_second_lowering_stays_deleted() {
    let v = hf4_violations(&elab());
    assert!(v.is_empty(), "{v:?}");
}

#[test]
fn hf4_planted_control_fails() {
    let files = planted(&elab(), "elaborate/src/hier_defer/write.rs", |t| {
        format!("{t}\nfn resolve_pending_fill_widths() {{}}\n")
    });
    assert!(!hf4_violations(&files).is_empty());
}

// ─────────────────────────────── HF5 ───────────────────────────────

const HF5_WRAPPERS: [&str; 2] = ["ia_capture_width", "stream_target_width"];

fn hf5_violations(files: &[(String, String)]) -> Vec<String> {
    let mut v = Vec::new();
    let gate = text_of(files, EDGE_GATE);
    let all = call_sites(files, "ir_lvalue_width");
    let in_wrappers: usize = HF5_WRAPPERS
        .iter()
        .map(|w| fn_body(gate, w).matches("ir_lvalue_width(").count())
        .sum();
    if all.len() != HF5_WRAPPERS.len() || in_wrappers != HF5_WRAPPERS.len() {
        v.push(format!("ir_lvalue_width callers {all:?}"));
    }
    if fn_body(gate, "fill_context").contains("ir_lvalue_width(") {
        v.push("fill_context calls ir_lvalue_width".to_string());
    }
    v
}

#[test]
fn hf5_the_defaulted_width_is_not_the_fill_s() {
    let v = hf5_violations(&elab());
    assert!(v.is_empty(), "{v:?}");
}

#[test]
fn hf5_planted_control_fails() {
    let files = planted(&elab(), EDGE_GATE, |t| {
        swap(
            t,
            "        let mut visible: u32 = 0;",
            "        let mut visible: u32 = 0 * self.ir_lvalue_width(lv);",
        )
    });
    assert!(!hf5_violations(&files).is_empty());
}

// ─────────────────────────────── HF6 ───────────────────────────────

const HF6_HEAD: &str =
    "let measured = self.measure_width(&u.measure); if u.kind == UseKind::Fill { \
     let Some(f) = measured else { if !self.had_error {";

fn hf6_violations(files: &[(String, String)]) -> Vec<String> {
    let body = squash(fn_body(text_of(files, EDGE_GATE), "verify_use_recs"));
    let mut v = Vec::new();
    if body.matches(HF6_HEAD).count() != 1 {
        v.push(
            "the Fill branch does not open with the unmeasured test and its had_error test".into(),
        );
    }
    let order = [
        "if !self.had_error { self.refuse_fill(u, reported); }",
        "if lv.chunks.len() > 1 { self.refuse_fill(u, reported); }",
        "let holds =",
        "self.patch_expr(slot.eid,",
        "contains(&c.net)) { self.refuse_fill(u, reported); continue; }",
    ];
    let at: Vec<Option<usize>> = order.iter().map(|n| body.find(n)).collect();
    if at.iter().any(Option::is_none) || at.windows(2).any(|w| w[0] >= w[1]) {
        v.push(format!("(j1)..(j5) out of order: {at:?}"));
    }
    if body.matches("self.patch_expr(").count() != 1 {
        v.push("not one re-size".into());
    }
    for append in ["push_expr(", "const_u32_expr("] {
        if body.contains(append) {
            v.push(format!("the decision appends: `{append}`"));
        }
    }
    v
}

#[test]
fn hf6_the_decision_s_order() {
    let v = hf6_violations(&elab());
    assert!(v.is_empty(), "{v:?}");
}

#[test]
fn hf6_planted_controls_fail() {
    let files = elab();
    let moved = planted(&files, EDGE_GATE, |t| {
        swap(
            t,
            "            if u.kind == UseKind::Fill {\n",
            "            if u.kind == UseKind::Fill {\n                let _early = matches!(u.need, Need::Assumed(_));\n",
        )
    });
    assert!(
        !hf6_violations(&moved).is_empty(),
        "a statement before the None test passed"
    );
    let append = planted(&files, EDGE_GATE, |t| {
        swap(
            t,
            "                        self.patch_expr(slot.eid, ir::Expr::Const { val });",
            "                        let val = self.push_expr(ir::Expr::Const { val });",
        )
    });
    assert!(
        !hf6_violations(&append).is_empty(),
        "an append in the decision passed"
    );
}

// ─────────────────────────────── HF7 ───────────────────────────────

fn hf7_violations(files: &[(String, String)]) -> Vec<String> {
    let mut v = Vec::new();
    let funnel = squash(fn_body(text_of(files, EXPR_CTX), "resize_rhs_for_lvalue"));
    let arm = "TargetW::Unprovable => {";
    match funnel.find(arm) {
        None => v.push("no Unprovable arm block".into()),
        Some(at) => {
            let open = at + arm.len() - 1;
            let blk = &funnel[open..group_end(funnel.as_bytes(), open)];
            if blk != "{ self.record_unprovable(lv, site); rhs_id }" {
                v.push(format!("the Unprovable arm is `{blk}`"));
            }
        }
    }
    let judge = squash(fn_body(text_of(files, EDGE_GATE), "verify_use_recs"));
    let head = "{ let quiet = self.had_error; let recs = std::mem::take(&mut self.use_recs); \
                for u in &recs { if u.kind == UseKind::Unprovable { \
                if !quiet { self.refuse_fill(u, reported); } continue; }";
    if !judge.starts_with(head) {
        v.push("verify_use_recs does not open with `quiet` and the Unprovable branch".into());
    }
    if judge.matches("quiet").count() != 2 {
        v.push("`quiet` is not written once and read once".into());
    }
    if judge.matches("UseKind::Unprovable").count() != 2 {
        v.push("an Unprovable record is read outside the loop's head".into());
    }
    v
}

#[test]
fn hf7_an_unprovable_target_is_judged_once_at_the_end() {
    let v = hf7_violations(&elab());
    assert!(v.is_empty(), "{v:?}");
}

#[test]
fn hf7_planted_controls_fail() {
    let files = elab();
    let speaks = planted(&files, EXPR_CTX, |t| {
        swap(
            t,
            "                self.record_unprovable(lv, site);\n",
            "                self.record_unprovable(lv, site);\n                self.error(MsgCode::ElabUnsupported, \"planted\");\n",
        )
    });
    assert!(
        !hf7_violations(&speaks).is_empty(),
        "a funnel that speaks passed"
    );
    let live = planted(&files, EDGE_GATE, |t| {
        swap(
            t,
            "                if !quiet {\n",
            "                if !self.had_error {\n",
        )
    });
    assert!(!hf7_violations(&live).is_empty(), "the live flag passed");
}

// ─────────────────────────────── HF8 ───────────────────────────────

/// HF8: the fill rule's refusal constants (RULINGS L2-53: one sentence, no value).
const HF8_CONSTS: [&str; 1] = ["FILL_UNESTABLISHED"];

/// Every `const FILL_…: &str = "…";` of `text`: (name, the string's value, a `\` line
/// continuation folded the way rustc folds it).
fn fill_consts(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (at, _) in text.match_indices("const FILL_") {
        let rest = &text[at + "const ".len()..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        let Some(q) = rest.find('"') else {
            out.push((name, String::new()));
            continue;
        };
        let mut val = String::new();
        let mut it = rest[q + 1..].chars().peekable();
        while let Some(c) = it.next() {
            match c {
                '"' => break,
                '\\' => match it.next() {
                    Some('\n') => {
                        while it.peek().is_some_and(|c| c.is_whitespace()) {
                            it.next();
                        }
                    }
                    Some(e) => {
                        val.push('\\');
                        val.push(e);
                    }
                    None => {}
                },
                c => val.push(c),
            }
        }
        out.push((name, val));
    }
    out
}

fn hf8_violations(files: &[(String, String)]) -> Vec<String> {
    let mut v = Vec::new();
    let gate = text_of(files, EDGE_GATE);
    let refuse = squash(fn_body(gate, "refuse_fill"));
    for bad in ["format!", "\"", "{}"] {
        if refuse.contains(bad) {
            v.push(format!("refuse_fill holds `{bad}`"));
        }
    }
    if refuse[1..].contains('{') {
        v.push("refuse_fill holds a `{`".into());
    }
    let named: Vec<&str> = refuse
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|w| w.starts_with("FILL_"))
        .collect();
    if named.is_empty() || named.iter().any(|w| !HF8_CONSTS.contains(w)) {
        v.push(format!("refuse_fill names {named:?}"));
    }
    let judge = squash(fn_body(gate, "verify_use_recs"));
    if judge.matches("self.refuse_fill(u, reported);").count() != 4 {
        v.push("verify_use_recs does not refuse through refuse_fill at its four refusals".into());
    }
    if judge.contains("FILL_") {
        v.push("verify_use_recs names a FILL_ constant".into());
    }
    let callers = call_sites(files, "refuse_fill");
    if callers.len() != 4 || callers.iter().any(|(f, _)| f != EDGE_GATE) {
        v.push(format!("refuse_fill callers {callers:?}"));
    }
    let consts = fill_consts(gate);
    if consts.is_empty() {
        v.push("no FILL_ constant".into());
    }
    for (name, val) in consts {
        if !HF8_CONSTS.contains(&name.as_str()) {
            v.push(format!("an unlisted constant {name}"));
        }
        if val.is_empty() || val.chars().any(|c| c.is_ascii_digit()) || val.contains("IEEE") {
            v.push(format!("{name} holds a number or a clause: `{val}`"));
        }
        if val.contains('{') {
            v.push(format!("{name} holds a `{{`"));
        }
    }
    v
}

#[test]
fn hf8_every_fill_refusal_is_one_constant_with_no_value() {
    let v = hf8_violations(&elab());
    assert!(v.is_empty(), "{v:?}");
}

#[test]
fn hf8_planted_controls_fail() {
    let files = elab();
    let formats = planted(&files, EDGE_GATE, |t| {
        swap(
            t,
            "self.report_at(u.span, &u.prefix, FILL_UNESTABLISHED, reported);",
            "self.report_at(u.span, &u.prefix, &format!(\"{} bits\", 8), reported);",
        )
    });
    assert!(
        !hf8_violations(&formats).is_empty(),
        "a formatted refusal passed"
    );
    let digit = planted(&files, EDGE_GATE, |t| {
        swap(t, "side is refused\";", "side is refused at 8 bits\";")
    });
    assert!(
        !hf8_violations(&digit).is_empty(),
        "a constant with a digit passed"
    );
    let clause = planted(&files, EDGE_GATE, |t| {
        swap(t, "side is refused\";", "side is refused, as IEEE says\";")
    });
    assert!(
        !hf8_violations(&clause).is_empty(),
        "a constant citing IEEE passed"
    );
    let second = planted(&files, EDGE_GATE, |t| {
        format!("{t}\nconst FILL_PLANTED: &str = \"a second sentence\";\n")
    });
    assert!(
        !hf8_violations(&second).is_empty(),
        "an unlisted constant passed"
    );
}
