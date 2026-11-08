//! Q-VALRULES (d) (owner 2026-11-06): every rule that derives a count, a width, a low / high
//! bound, a direction, an offset or an index position from a pair of bounds lives in
//! `crates/hdl-ast/src/range_rules.rs`. This test fails when one of the SHAPES below is
//! spelled in any workspace crate's `src` outside that file, except the named sites. A new
//! site is either routed through a `range_rules` rule or added here with its reason; a
//! removed site must leave the list (the count is exact, so the list cannot go stale).
//!
//! The shapes (matched on tokens, string literals and `//` comments skipped):
//! * `abs_diff(` and `unsigned_abs(`;
//! * `.abs() + 1` (also `.abs() as T + 1`);
//! * a span `- B + 1` (also `- B) + 1`, `- B as T + 1`) with `B` a low-bound name
//!   ([`LOW_NAMES`]);
//! * `.min(` / `.max(` over two bound names ([`PAIR_NAMES`]);
//! * a direction test `A < B` / `A > B` / `A <= B` / `A >= B` over a bound pair
//!   ([`DIR_PAIRS`], either order, a `x.` prefix and a `*` allowed);
//! * a dimension product: `.product()`, `fold(1u32`, `fold(1u64`;
//! * a checked span (lens2 CD1 Ma): `checked_add(1)` / `saturating_add(1)` within
//!   [`STEP_WINDOW`] tokens of a `checked_sub(` / `saturating_sub(` on the same line;
//! * a span with its 1 first (lens2 CD1 Mh): `1 + A - B` and `+ 1 - B`, `B` in [`LOW_NAMES`];
//! * an indexed part-select's low / high index (lens2 CD1 Mb): `- W + 1` (also `- W as T + 1`)
//!   and `+ 1 - W`, `W` in [`WIDTH_NAMES`];
//! * (lens2 Md / Mf / Mg) a span over a low accessor `- X.B() + 1` (`B` in [`LOW_NAMES`]),
//!   a free `min(A, B)` / `max(A, B)` over two [`PAIR_NAMES`], and a product `fold(1,`.
//!
//! What no token test sees (review only, listed in the file's history note): a bare `*` of a
//! width by a count, an index offset `i - lo`, a bound pair under names outside the lists
//! below, a direction test over a selected pair named `a` / `b` (the evaluators' comparison
//! arms share that spelling, so `a` / `b` are not in [`DIR_PAIRS`]), a rule spelled through a
//! helper defined outside the file, a span over a HIGH accessor alone, a product written as a
//! loop or a bare `*=`, a `checked_add(1)` more than [`STEP_WINDOW`] tokens from its
//! `checked_sub(` or on another line, and a select's offset arithmetic over names outside
//! [`WIDTH_NAMES`]. The history note lists these lists and the exceptions
//! verbatim; an edit here edits the note.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const HOME: &str = "crates/hdl-ast/src/range_rules.rs";

/// Names a span's low bound is spelled with.
const LOW_NAMES: [&str; 8] = ["lo", "l", "lsb", "lo_n", "c_lo", "right", "low", "min"];
/// Names either bound of a pair is spelled with (`.min(` / `.max(`); `a` / `b` are a
/// SELECTED pair's bounds (`x[a:b]`, Q-VALRULES (d) design review 3, D3-1).
const PAIR_NAMES: [&str; 10] = [
    "m", "l", "msb", "lsb", "left", "right", "ia", "ib", "a", "b",
];
/// Names an indexed part-select's width is spelled with (`[o -: w]`'s low index `o - w + 1`).
const WIDTH_NAMES: [&str; 3] = ["w", "width", "ww"];
/// How many tokens around a `checked_add(1)` a `checked_sub(` may sit (one chained span).
const STEP_WINDOW: usize = 8;
/// Bound pairs a direction test reads (either order).
const DIR_PAIRS: [(&str, &str); 3] = [("m", "l"), ("msb", "lsb"), ("left", "right")];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Shape {
    AbsDiff,
    UnsignedAbs,
    AbsPlusOne,
    SpanPlusOne,
    MinMaxPair,
    DirTest,
    DimProduct,
    CheckedSpan,
    OnePlusSpan,
    IndexedLow,
}

/// (file, shape, count, why it is not a range rule here or cannot reach the file).
const EXCEPTIONS: &[(&str, Shape, usize, &str)] = &[
    (
        "crates/elaborate/src/packed.rs",
        Shape::UnsignedAbs,
        1,
        "`raw - k` for an already-derived negative k spelled as `raw + |k|`: arithmetic on an offset, not a derivation from a range",
    ),
    (
        "crates/elaborate/src/string_array_route.rs",
        Shape::UnsignedAbs,
        1,
        "the text of an index literal's magnitude, not a range derivation",
    ),
    (
        "crates/hdl-parser/src/expr_primary.rs",
        Shape::UnsignedAbs,
        1,
        "`i64_lit`: the text of a literal's magnitude, not a range derivation",
    ),
    (
        "crates/sim-engine/src/builtins/crv_draw.rs",
        Shape::AbsDiff,
        1,
        "a constraint's `[a:b]` value range at run time, not a declared range; sim-engine has no hdl-ast edge",
    ),
    (
        "crates/sim-engine/src/state/task_frames.rs",
        Shape::UnsignedAbs,
        2,
        "the VCD header re-derives a declared span from sim-ir's (msb, lsb) at run time; sim-engine has no hdl-ast edge, so it copies `Lane::PartSelectSub` -- owner Q-SIDETABLES",
    ),
    (
        "crates/sim-engine/src/builtins/crv_draw.rs",
        Shape::SpanPlusOne,
        3,
        "a constraint's value range `[lo:hi]` drawn at run time (not a declared or selected range); no hdl-ast edge",
    ),
    (
        "crates/sim-engine/src/builtins/pattern.rs",
        Shape::DimProduct,
        1,
        "a runtime array pattern's stride over sim-ir's recorded dims; sim-engine has no hdl-ast edge -- owner Q-SIDETABLES",
    ),
    (
        "crates/sim-engine/src/builtins/queues_io.rs",
        Shape::SpanPlusOne,
        1,
        "a queue slice `q[lo:hi]` taken at run time over queue positions; no hdl-ast edge -- owner Q-EXECSEM",
    ),
    (
        "crates/sim-engine/src/eval/sysfunc.rs",
        Shape::SpanPlusOne,
        1,
        "`$urandom_range(hi, lo)`'s value range at run time, not a declared range; no hdl-ast edge",
    ),
    (
        "crates/sim-ir/src/selfwidth/edge_value_tests.rs",
        Shape::SpanPlusOne,
        1,
        "a unit test's hand oracle for a width (sim-ir has no hdl-ast edge)",
    ),
    (
        "crates/sim-ir/src/selfwidth/edge_value_tests.rs",
        Shape::DirTest,
        2,
        "a unit test's hand oracle for a direction (sim-ir has no hdl-ast edge)",
    ),
    (
        "crates/elaborate/src/cover_bins.rs",
        Shape::SpanPlusOne,
        1,
        "a coverage bin's value range `[l:h]` (count of values), not a declared or selected range",
    ),
    (
        "crates/elaborate/src/cover_synth.rs",
        Shape::SpanPlusOne,
        1,
        "a coverage bin's value range `[lo:hi]`, not a declared or selected range",
    ),
    (
        "crates/elaborate/src/crv.rs",
        Shape::SpanPlusOne,
        1,
        "a constraint's value range `[lo:hi]`, not a declared or selected range",
    ),
    (
        "crates/elaborate/src/const_fn_width/fold.rs",
        Shape::MinMaxPair,
        1,
        "`(Some(a), Some(b)) => a.max(b)`: the larger of two operand WIDTHS (a self-width rule, IEEE 1800-2017 §11.6.1), not a pair of bounds",
    ),
    (
        "crates/sim-ir/src/selfwidth.rs",
        Shape::MinMaxPair,
        1,
        "`Ok((a.max(b), sa && sb))`: the larger of two operand widths (a self-width rule, §11.6.1), not a pair of bounds; sim-ir has no hdl-ast edge",
    ),
    (
        "crates/cli/src/pipeline.rs",
        Shape::MinMaxPair,
        1,
        "`(Some(a), Some(b)) => a.min(b)`: the earlier of two text offsets in a diagnostic, not a pair of bounds",
    ),
    (
        "crates/elaborate/src/sva_seq.rs",
        Shape::SpanPlusOne,
        1,
        "an SVA delay window `##[min:n]` (cycles), not a declared or selected range",
    ),
    (
        "crates/sim-ir/src/lib.rs",
        Shape::IndexedLow,
        1,
        "CRATE-EDGE (lens2 CD1): `sel_written_bits` re-derives `[off -: w]`'s low index (`range_rules::indexed_last`'s rule) over a select sim-ir records; sim-ir has no hdl-ast edge -- owner Q-SIDETABLES",
    ),
    (
        "crates/sim-ir/src/selfwidth.rs",
        Shape::CheckedSpan,
        1,
        "CRATE-EDGE (lens2 CD1): `part_width` re-derives a part-select's `|m - l| + 1` in its declared direction for the self-width rule; sim-ir has no hdl-ast edge -- owner Q-SIDETABLES",
    ),
    (
        "crates/sim-engine/src/state/changes.rs",
        Shape::IndexedLow,
        1,
        "CRATE-EDGE (lens2 CD1): a runtime `[off -: w]` change record's low index (`range_rules::indexed_last`'s rule); sim-engine has no hdl-ast edge -- owner Q-SIDETABLES",
    ),
    (
        "crates/sim-engine/src/state/frame_eval.rs",
        Shape::IndexedLow,
        1,
        "CRATE-EDGE (lens2 CD1): a frame write's `[off -: w]` low index at run time (`range_rules::indexed_last`'s rule); sim-engine has no hdl-ast edge -- owner Q-SIDETABLES",
    ),
];

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// The code tokens of one line: identifiers / numbers, and single punctuation characters;
/// a `//` comment and the inside of a `"…"` string are dropped.
fn tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    let flush = |cur: &mut String, out: &mut Vec<String>| {
        if !cur.is_empty() {
            out.push(std::mem::take(cur));
        }
    };
    while let Some(c) = chars.next() {
        if c == '"' {
            flush(&mut cur, &mut out);
            while let Some(s) = chars.next() {
                match s {
                    '\\' => {
                        chars.next();
                    }
                    '"' => break,
                    _ => {}
                }
            }
            out.push("\"\"".to_string());
        } else if c == '/' && chars.peek() == Some(&'/') {
            break;
        } else if c.is_alphanumeric() || c == '_' {
            cur.push(c);
        } else {
            flush(&mut cur, &mut out);
            if !c.is_whitespace() {
                out.push(c.to_string());
            }
        }
    }
    flush(&mut cur, &mut out);
    out
}

/// Every shape the token line holds, once per occurrence.
fn shapes(t: &[String]) -> Vec<Shape> {
    let is = |i: usize, s: &str| t.get(i).is_some_and(|x| x == s);
    let ident_in = |i: usize, set: &[&str]| t.get(i).is_some_and(|x| set.contains(&x.as_str()));
    let mut out = Vec::new();
    for i in 0..t.len() {
        if is(i, "abs_diff") && is(i + 1, "(") {
            out.push(Shape::AbsDiff);
        }
        if is(i, "unsigned_abs") && is(i + 1, "(") {
            out.push(Shape::UnsignedAbs);
        }
        // `. abs ( ) [as T] + 1`
        if is(i, ".") && is(i + 1, "abs") && is(i + 2, "(") && is(i + 3, ")") {
            let k = if is(i + 4, "as") { i + 6 } else { i + 4 };
            if is(k, "+") && is(k + 1, "1") {
                out.push(Shape::AbsPlusOne);
            }
        }
        // `- [*] B [)] [as T] + 1`
        if is(i, "-") {
            let mut k = i + 1;
            if is(k, "*") {
                k += 1;
            }
            if ident_in(k, &LOW_NAMES) && !is(k + 1, ".") && !is(k + 1, "(") {
                k += 1;
                if is(k, ")") {
                    k += 1;
                }
                if is(k, "as") {
                    k += 2;
                }
                if is(k, "+") && is(k + 1, "1") && !is(k + 2, ".") {
                    out.push(Shape::SpanPlusOne);
                }
            }
        }
        // `- X . B ( ) [as T] + 1`: a span over a record's low accessor (lens2 Md)
        if is(i, "-")
            && is(i + 2, ".")
            && ident_in(i + 3, &LOW_NAMES)
            && is(i + 4, "(")
            && is(i + 5, ")")
        {
            let k = if is(i + 6, "as") { i + 8 } else { i + 6 };
            if is(k, "+") && is(k + 1, "1") && !is(k + 2, ".") {
                out.push(Shape::SpanPlusOne);
            }
        }
        // `min|max ( A , B )` (`std::cmp::min(m, l)`, lens2 Mf), not a method call
        if (is(i, "min") || is(i, "max"))
            && !(i > 0 && is(i - 1, "."))
            && is(i + 1, "(")
            && ident_in(i + 2, &PAIR_NAMES)
            && is(i + 3, ",")
            && ident_in(i + 4, &PAIR_NAMES)
            && is(i + 5, ")")
        {
            out.push(Shape::MinMaxPair);
        }
        // `A . min|max ( [*] B )`
        if ident_in(i, &PAIR_NAMES)
            && is(i + 1, ".")
            && (is(i + 2, "min") || is(i + 2, "max"))
            && is(i + 3, "(")
        {
            let k = if is(i + 4, "*") { i + 5 } else { i + 4 };
            if ident_in(k, &PAIR_NAMES) && is(k + 1, ")") {
                out.push(Shape::MinMaxPair);
            }
        }
        // `A <|>|<=|>= [*] [x .] B`, (A, B) a bound pair either order
        for (a, b) in DIR_PAIRS {
            for (x, y) in [(a, b), (b, a)] {
                if !is(i, x) {
                    continue;
                }
                let mut k = i + 1;
                if !(is(k, "<") || is(k, ">")) {
                    continue;
                }
                k += 1;
                if is(k, "=") {
                    k += 1;
                }
                if is(k, "*") {
                    k += 1;
                }
                if is(k + 1, ".") && !is(k, y) {
                    k += 2;
                }
                if is(k, y) && !is(k + 1, "(") && !is(k + 1, "<") && !is(k + 1, ":") {
                    out.push(Shape::DirTest);
                }
            }
        }
        if is(i, ".") && is(i + 1, "product") && is(i + 2, "(") && is(i + 3, ")") {
            out.push(Shape::DimProduct);
        }
        if is(i, "fold")
            && is(i + 1, "(")
            && (is(i + 2, "1u32") || is(i + 2, "1u64") || (is(i + 2, "1") && is(i + 3, ",")))
        {
            out.push(Shape::DimProduct);
        }
        // `checked_add|saturating_add ( 1 )` near a `checked_sub|saturating_sub (`
        if (is(i, "checked_add") || is(i, "saturating_add"))
            && is(i + 1, "(")
            && is(i + 2, "1")
            && is(i + 3, ")")
        {
            let lo = i.saturating_sub(STEP_WINDOW);
            let hi = (i + 4 + STEP_WINDOW).min(t.len());
            if (lo..hi).any(|k| (is(k, "checked_sub") || is(k, "saturating_sub")) && is(k + 1, "("))
            {
                out.push(Shape::CheckedSpan);
            }
        }
        let bound_at = |k: usize, set: &[&str]| {
            ident_in(k, set) && !is(k + 1, ".") && !is(k + 1, "(") && !is(k + 1, "::")
        };
        // `1 + A - B` and `+ 1 - B`, B a low-bound name
        if is(i, "1")
            && is(i + 1, "+")
            && t.get(i + 2)
                .is_some_and(|x| x.chars().all(|c| c.is_alphanumeric() || c == '_'))
            && is(i + 3, "-")
            && bound_at(i + 4, &LOW_NAMES)
        {
            out.push(Shape::OnePlusSpan);
        }
        if is(i, "+") && is(i + 1, "1") && is(i + 2, "-") && bound_at(i + 3, &LOW_NAMES) {
            out.push(Shape::OnePlusSpan);
        }
        // `- W [as T] + 1` and `+ 1 - W`, W a width name
        if is(i, "-") && bound_at(i + 1, &WIDTH_NAMES) {
            let k = if is(i + 2, "as") { i + 4 } else { i + 2 };
            if is(k, "+") && is(k + 1, "1") && !is(k + 2, ".") {
                out.push(Shape::IndexedLow);
            }
        }
        if is(i, "+") && is(i + 1, "1") && is(i + 2, "-") && bound_at(i + 3, &WIDTH_NAMES) {
            out.push(Shape::IndexedLow);
        }
    }
    out
}

/// `(file relative to the workspace, shape) -> count` over every crate's `src`, comment
/// lines skipped, the home file skipped.
fn census(root: &Path) -> BTreeMap<(String, Shape), usize> {
    let mut files = Vec::new();
    for c in std::fs::read_dir(root.join("crates"))
        .expect("crates/")
        .flatten()
    {
        rs_files(&c.path().join("src"), &mut files);
    }
    let mut hits = BTreeMap::new();
    for f in files {
        let rel = f
            .strip_prefix(root)
            .expect("under the workspace")
            .to_string_lossy()
            .replace('\\', "/");
        if rel == HOME {
            continue;
        }
        let text = std::fs::read_to_string(&f).expect("readable source");
        for line in text.lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for s in shapes(&tokens(line)) {
                *hits.entry((rel.clone(), s)).or_insert(0) += 1;
            }
        }
    }
    hits
}

#[test]
fn range_derivations_live_in_range_rules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.canonicalize().expect("workspace root");
    assert!(root.join(HOME).is_file(), "the home file {HOME} is missing");
    let hits = census(&root);
    let mut want = BTreeMap::new();
    for &(f, s, n, _) in EXCEPTIONS {
        want.insert((f.to_string(), s), n);
    }
    let mut bad = Vec::new();
    for (k, n) in &hits {
        match want.get(k) {
            Some(w) if w == n => {}
            Some(w) => bad.push(format!("{} {:?}: {n} sites, the exception names {w}", k.0, k.1)),
            None => bad.push(format!(
                "{} {:?}: {n} site(s) outside {HOME} -- route it through a range_rules rule, or name it in EXCEPTIONS",
                k.0, k.1
            )),
        }
    }
    for (k, w) in &want {
        if !hits.contains_key(k) {
            bad.push(format!(
                "{} {:?}: the exception names {w} sites, there are none -- remove it",
                k.0, k.1
            ));
        }
    }
    assert!(
        bad.is_empty(),
        "range derivations outside {HOME}:\n{}",
        bad.join("\n")
    );
}

/// The consumers that fold a range in PRE's PAIR order (`range_facts_pair`, both bounds before
/// either is looked at), keyed by file AND enclosing function, the count exact: each was PRE's
/// `match (fold(msb), fold(lsb))` at that site. Every other consumer takes the lazy order
/// (`range_facts` / `range_facts_if`: the lsb only once the msb folded), so the eager order
/// cannot spread unseen (premise6 B1 / B2: an eager lsb recursed without end and cost up to
/// 48x). The function key (premise7 C-a): array_geom.rs, net_util.rs and params.rs hold both
/// orders, so a per-file count misses a lazy <-> pair swap inside one of them; no function
/// holds both (asserted), so every swap moves a key here.
const PAIR_SITES: &[(&str, &str, usize)] = &[
    (
        "crates/elaborate/src/array_formal.rs",
        "classify_unpacked_array",
        2,
    ),
    ("crates/elaborate/src/array_geom.rs", "compute_dim_desc", 2),
    ("crates/elaborate/src/assoc_key.rs", "assoc_key_shape_of", 1),
    (
        "crates/elaborate/src/inline_task_locals.rs",
        "hoist_one_inline_local",
        1,
    ),
    ("crates/elaborate/src/instance.rs", "elaborate_instance", 1),
    (
        "crates/elaborate/src/instance_array.rs",
        "elaborate_instance_array",
        3,
    ),
    ("crates/elaborate/src/net_util.rs", "enum_base_width", 1),
    ("crates/elaborate/src/net_util.rs", "prescan_net_bits", 1),
    (
        "crates/elaborate/src/netdecl.rs",
        "elaborate_netvar_decl_inner",
        1,
    ),
    ("crates/elaborate/src/params.rs", "param_decl_width_opt", 1),
    ("crates/elaborate/src/ports.rs", "elaborate_ports", 1),
];

/// The evaluation order a range-record accessor call takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Order {
    /// `range_facts(` / `range_facts_if(`: the lsb only once the msb folded.
    Lazy,
    /// `range_facts_pair(`: both bounds folded first.
    Pair,
}

/// `(enclosing function, order) -> calls` in one source text, on the code tokens (`//`
/// comments and string insides dropped, comment lines skipped). The enclosing function is the
/// last `fn NAME` above the call; an accessor's own `fn` line is not a call.
fn order_sites(text: &str) -> BTreeMap<(String, Order), usize> {
    let mut out = BTreeMap::new();
    let mut func = String::new();
    for line in text.lines() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        let t = tokens(line);
        for (i, tok) in t.iter().enumerate() {
            let prev = if i > 0 { t[i - 1].as_str() } else { "" };
            if prev == "fn" {
                func = tok.clone();
                continue;
            }
            if t.get(i + 1).map(String::as_str) != Some("(") {
                continue;
            }
            let order = match tok.as_str() {
                "range_facts" | "range_facts_if" => Order::Lazy,
                "range_facts_pair" => Order::Pair,
                _ => continue,
            };
            *out.entry((func.clone(), order)).or_insert(0) += 1;
        }
    }
    out
}

/// A pair site's key: (file relative to the workspace, enclosing function).
type SiteKey = (String, String);

/// `(file, function) -> pair calls`, and the functions that also hold a lazy call.
fn pair_map(files: &[(String, String)]) -> (BTreeMap<SiteKey, usize>, Vec<SiteKey>) {
    let mut pairs = BTreeMap::new();
    let mut mixed = Vec::new();
    for (rel, text) in files {
        let sites = order_sites(text);
        for ((func, order), n) in &sites {
            if *order != Order::Pair {
                continue;
            }
            pairs.insert((rel.clone(), func.clone()), *n);
            if sites.contains_key(&(func.clone(), Order::Lazy)) {
                mixed.push((rel.clone(), func.clone()));
            }
        }
    }
    (pairs, mixed)
}

/// The file holding the three accessors (`range_facts` / `range_facts_if` / `range_facts_pair`),
/// the only callers of the record's two constructors outside [`HOME`].
const ACCESSORS: &str = "crates/elaborate/src/const_select.rs";

/// `file:line: code` of each `RangeFacts::pair(` / `RangeFacts::lazy(` call (any path prefix)
/// in a crate's `src` outside [`ACCESSORS`] and [`HOME`], on the code tokens (`//` comments and
/// string insides dropped, comment lines skipped).
fn ctor_calls_outside(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    for c in std::fs::read_dir(root.join("crates"))
        .expect("crates/")
        .flatten()
    {
        rs_files(&c.path().join("src"), &mut files);
    }
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let rel = f
            .strip_prefix(root)
            .expect("under the workspace")
            .to_string_lossy()
            .replace('\\', "/");
        if rel == HOME || rel == ACCESSORS {
            continue;
        }
        let text = std::fs::read_to_string(&f).expect("readable source");
        for (n, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            let t = tokens(line);
            let is = |i: usize, s: &str| t.get(i).is_some_and(|x| x == s);
            for i in 0..t.len() {
                if is(i, "RangeFacts")
                    && is(i + 1, ":")
                    && is(i + 2, ":")
                    && (is(i + 3, "pair") || is(i + 3, "lazy"))
                    && is(i + 4, "(")
                {
                    out.push(format!("{rel}:{}: {}", n + 1, line.trim()));
                }
            }
        }
    }
    out
}

/// One lazy <-> pair swap inside one file: its first pair call takes the lazy accessor and its
/// first lazy call the pair accessor (the per-file pair count is unchanged).
fn swap_one_order(text: &str) -> String {
    text.replacen(".range_facts_pair(", ".range_facts_SWAP(", 1)
        .replacen(".range_facts(", ".range_facts_pair(", 1)
        .replacen(".range_facts_SWAP(", ".range_facts(", 1)
}

#[test]
fn range_record_fold_order() {
    use hdl_ast::range_rules::{RangeFacts, RangeUnfolded};
    // lazy: the lsb is not called after a failed msb (a called lsb panics here).
    assert_eq!(
        RangeFacts::lazy(|| None, || panic!("lsb folded after a failed msb")),
        Err(RangeUnfolded)
    );
    assert_eq!(RangeFacts::lazy(|| Some(3), || None), Err(RangeUnfolded));
    assert_eq!(
        RangeFacts::lazy(|| Some(3), || Some(0)),
        Ok(RangeFacts::new(3, 0))
    );
    // pair: the record of two already-folded bounds, Err when either is missing.
    assert_eq!(RangeFacts::pair(None, Some(0)), Err(RangeUnfolded));
    assert_eq!(RangeFacts::pair(Some(3), None), Err(RangeUnfolded));
    assert_eq!(
        RangeFacts::pair(Some(-2), Some(3)),
        Ok(RangeFacts::new(-2, 3))
    );
    // the pair order's sites: exactly PAIR_SITES, by file and function.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.canonicalize().expect("workspace root");
    let mut paths = Vec::new();
    rs_files(&root.join("crates/elaborate/src"), &mut paths);
    let mut files: Vec<(String, String)> = paths
        .iter()
        .map(|f| {
            let rel = f
                .strip_prefix(&root)
                .expect("under the workspace")
                .to_string_lossy()
                .replace('\\', "/");
            (rel, std::fs::read_to_string(f).expect("readable source"))
        })
        .collect();
    files.sort();
    let (hits, mixed) = pair_map(&files);
    let want: BTreeMap<(String, String), usize> = PAIR_SITES
        .iter()
        .map(|&(f, func, n)| ((f.to_string(), func.to_string()), n))
        .collect();
    assert_eq!(
        hits, want,
        "range_facts_pair sites differ from PAIR_SITES: a new consumer takes range_facts (lazy)"
    );
    assert!(
        mixed.is_empty(),
        "a function holds both orders, so a swap inside it moves no key: {mixed:?}"
    );
    // the record's constructors (lens4 N-1): called only inside the three accessors, so a
    // consumer that folds a range in either order reaches PAIR_SITES' count through them.
    // REVIEW-ONLY: PRE's raw `match (fold(msb), fold(lsb))` in a new consumer (= R's sites).
    let ctor = ctor_calls_outside(&root);
    assert!(
        ctor.is_empty(),
        "EVALUATION-ORDER: a constructor call outside the three accessors escapes PAIR_SITES \
         -- call range_facts / range_facts_if / range_facts_pair instead:\n{}",
        ctor.join("\n")
    );
    // control (premise7 C-a): one swap inside each file holding both orders keeps that file's
    // pair count and moves a (file, function) key.
    for name in ["array_geom.rs", "net_util.rs", "params.rs"] {
        let rel = format!("crates/elaborate/src/{name}");
        let swapped: Vec<(String, String)> = files
            .iter()
            .map(|(r, t)| {
                let t = if *r == rel {
                    swap_one_order(t)
                } else {
                    t.clone()
                };
                (r.clone(), t)
            })
            .collect();
        let (moved, _) = pair_map(&swapped);
        let per_file = |m: &BTreeMap<(String, String), usize>| -> usize {
            m.iter()
                .filter(|((f, _), _)| *f == rel)
                .map(|(_, n)| *n)
                .sum()
        };
        assert_eq!(
            per_file(&moved),
            per_file(&hits),
            "{name}: the swap keeps the per-file count"
        );
        assert_ne!(moved, hits, "{name}: the swap moves a (file, function) key");
    }
}

/// B2d (plan8 B2(c), premise6 C3): each lane's edge, where PRE's carrier caps, wraps or declines
/// -- no cell reaches these, so a lane edit that moves an edge dies here. Each `pre_*` closure is
/// PRE's spelling copied verbatim from R cfc17636 (the file:line beside it) and is asserted to
/// the value first, then the lane. A debug build's overflow panic (PRE's own) is pinned as a
/// panic on both spellings, a release build's wrap as the value.
#[test]
fn lane_edges_keep_pre() {
    use hdl_ast::range_rules::{bound_step, BoundStep, Lane, RangeFacts};
    use std::hint::black_box as bb;
    // e32: the abs_diff whose `|m - l| + 1` is 2^32.
    let e32 = (1i64 << 32) - 1;
    // the record: the count is exact, `|msb - lsb| + 1`, up to 2^64.
    assert_eq!(RangeFacts::new(3, 0).count, 4);
    assert_eq!(RangeFacts::new(0, 3).count, 4);
    assert_eq!(RangeFacts::new(5, 5).count, 1);
    assert_eq!(RangeFacts::new(i64::MIN, i64::MAX).count, 1u128 << 64);
    // the bound value rule F1a: elaborate const_fn_width/fold_x.rs:136 `if v <= i64::from(i32::MAX)`.
    let pre_value = |v: i64| v <= i64::from(i32::MAX);
    let top = i64::from(i32::MAX);
    for (v, want) in [
        (top, BoundStep::Value(top)),
        (top + 1, BoundStep::Wide),
        (i64::MIN, BoundStep::Value(i64::MIN)),
    ] {
        assert_eq!(pre_value(v), matches!(want, BoundStep::Value(_)));
        assert_eq!(bound_step(Some(v)), want);
    }
    assert_eq!(bound_step(None), BoundStep::Wide);
    // FormalDims: elaborate array_formal.rs:351 `(m.abs_diff(l) + 1) as u32` -- 2^32 wraps to 0.
    let pre_formal_dims = |m: i64, l: i64| (m.abs_diff(l) + 1) as u32;
    for (m, l, want) in [
        (e32, 0, 0u32),
        (0, e32, 0),
        (e32 - 1, 0, u32::MAX),
        (3, 0, 4),
    ] {
        assert_eq!(pre_formal_dims(m, l), want);
        assert_eq!(Lane::FormalDims.width_u32(m, l), want);
    }
    // ParamRange: elaborate params.rs:871 `m.abs_diff(l) as u32 + 1` -- at abs_diff 2^32 the
    // truncation gives 1 ; at abs_diff 2^32 - 1 `u32::MAX + 1` overflows.
    let pre_param_range = |m: i64, l: i64| m.abs_diff(l) as u32 + 1;
    for (m, l, want) in [(e32 + 1, 0, 1u32), (e32 - 1, 0, u32::MAX), (7, 0, 8)] {
        assert_eq!(pre_param_range(m, l), want);
        assert_eq!(Lane::ParamRange.width_u32(m, l), want);
    }
    overflow_edge(
        move || pre_param_range(bb(e32), bb(0)),
        move || Lane::ParamRange.width_u32(bb(e32), bb(0)),
        0,
    );
    // AssocKey: elaborate assoc_key.rs:88 `m.abs_diff(l).saturating_add(1)` -- saturates at u64::MAX.
    let pre_assoc_key = |m: i64, l: i64| m.abs_diff(l).saturating_add(1);
    for (m, l, want) in [
        (i64::MIN, i64::MAX, u64::MAX),
        (i64::MIN + 1, i64::MAX, u64::MAX),
        (0, i64::MAX, 1u64 << 63),
    ] {
        assert_eq!(pre_assoc_key(m, l), want);
        assert_eq!(Lane::AssocKey.width_u64(m, l), want);
    }
    // FnWidth: elaborate const_fn_width.rs:623 `u32::try_from(hi.abs_diff(lo).checked_add(1)?).ok()?`.
    let pre_fn_width = |m: i64, l: i64| u32::try_from(m.abs_diff(l).checked_add(1)?).ok();
    for (m, l, want) in [
        (i64::MIN, i64::MAX, None),
        (e32, 0, None),
        (e32 - 1, 0, Some(u32::MAX)),
    ] {
        assert_eq!(pre_fn_width(m, l), want);
        assert_eq!(Lane::FnWidth.count_u32(m, l), want);
    }
    // FnReturn: elaborate inline_fn.rs:131 `u32::try_from((hi - lo).unsigned_abs() + 1).ok()?` --
    // where `m - l` overflows i64 PRE overflows (a release build wraps: 2^63 + 1 -> None).
    let pre_fn_return = |m: i64, l: i64| u32::try_from((m - l).unsigned_abs() + 1).ok();
    for (m, l, want) in [
        (e32 - 1, 0, Some(u32::MAX)),
        (0, e32 - 1, Some(u32::MAX)),
        (e32, 0, None),
        (i64::MAX, 0, None),
    ] {
        assert_eq!(pre_fn_return(m, l), want);
        assert_eq!(Lane::FnReturn.count_u32(m, l), want);
    }
    overflow_edge(
        move || pre_fn_return(bb(i64::MAX), bb(-1)),
        move || Lane::FnReturn.count_u32(bb(i64::MAX), bb(-1)),
        None,
    );
}

/// A PRE overflow edge: in a debug build (overflow checks on) both spellings panic; in a release
/// build both answer `release`.
fn overflow_edge<T: PartialEq + std::fmt::Debug>(
    pre: impl FnOnce() -> T + std::panic::UnwindSafe,
    lane: impl FnOnce() -> T + std::panic::UnwindSafe,
    release: T,
) {
    if cfg!(debug_assertions) {
        assert!(
            std::panic::catch_unwind(pre).is_err(),
            "PRE's spelling overflows here in a debug build"
        );
        assert!(
            std::panic::catch_unwind(lane).is_err(),
            "the lane must overflow where PRE's spelling did"
        );
    } else {
        assert_eq!(pre(), release);
        assert_eq!(lane(), release);
    }
}

/// The matcher's own controls: each shape found on a planted line, none on its near miss.
#[test]
fn shapes_match_their_controls() {
    let one = |s: &str| shapes(&tokens(s));
    assert_eq!(one("let n = msb.abs_diff(lsb) + 1;"), [Shape::AbsDiff]);
    assert_eq!(
        one("let n = (m - l).unsigned_abs() as u32 + 1;"),
        [Shape::UnsignedAbs]
    );
    assert_eq!(
        one("let w = (msb - lsb).abs() as u32 + 1;"),
        [Shape::AbsPlusOne]
    );
    assert_eq!(
        one("let size = ((m - l).abs() + 1) as u32;"),
        [Shape::AbsPlusOne]
    );
    assert_eq!(
        one("let n = (hi_n - lo_n + 1) as u32;"),
        [Shape::SpanPlusOne]
    );
    assert_eq!(one("let s = (max - min) as u64 + 1;"), [Shape::SpanPlusOne]);
    assert_eq!(one("(msb.min(lsb), size.max(1))"), [Shape::MinMaxPair]);
    assert_eq!(
        one("Some(self.packed_outer_range(net, a.min(b), a.max(b)))"),
        [Shape::MinMaxPair, Shape::MinMaxPair]
    );
    assert_eq!(one("return if m >= l {"), [Shape::DirTest]);
    assert_eq!(one("if f.msb < f.lsb {"), [Shape::DirTest]);
    assert_eq!(one("if left <= *right {"), [Shape::DirTest]);
    assert_eq!(one("let n: u64 = d.iter().product();"), [Shape::DimProduct]);
    assert_eq!(one("x.iter().fold(1u32, |a, w| a)"), [Shape::DimProduct]);
    // lens2 CD1 Ma / Mh / Mb (the lens's slips, each a positive control here)
    assert_eq!(
        one("let n = u32::try_from(hi_n.checked_sub(lo_n)?.checked_add(1)?).ok()?;"),
        [Shape::CheckedSpan]
    );
    assert_eq!(
        one("Some((o.checked_add(1)?.checked_sub(w)?, o.checked_add(1)?))"),
        [Shape::CheckedSpan, Shape::CheckedSpan]
    );
    assert_eq!(
        one("let w = hi.saturating_sub(lo).saturating_add(1);"),
        [Shape::CheckedSpan]
    );
    assert_eq!(
        one("let n = u32::try_from(1 + hi_n - lo_n).ok()?;"),
        [Shape::OnePlusSpan]
    );
    assert_eq!(one("let n = (hi + 1 - lo) as u32;"), [Shape::OnePlusSpan]);
    assert_eq!(
        one("(c as u64 + 1 - w as u64, c as u64)"),
        [Shape::IndexedLow]
    );
    assert_eq!(one("PartIdxDown => off - w + 1,"), [Shape::IndexedLow]);
    // lens2 Md / Mf / Mg
    assert_eq!(
        one("let n = (f.hi() - f.lo() + 1) as u32;"),
        [Shape::SpanPlusOne]
    );
    assert_eq!(
        one("let lo = std::cmp::min(m, l).max(0) as u32;"),
        [Shape::MinMaxPair]
    );
    assert_eq!(
        one("let n = d.iter().fold(1, |acc: u32, &n| acc.saturating_mul(n.max(1)));"),
        [Shape::DimProduct]
    );
    assert_eq!(one("let lo = off_i - w as i64 + 1;"), [Shape::IndexedLow]);
    // near misses
    assert!(one("let s = \"m < l\";").is_empty());
    assert!(one("x.iter().try_fold(1u32, |a, w| a)").is_empty());
    assert!(one("let v = hi - lo + 1.0;").is_empty());
    assert!(one("let v = Vec::<l>::new(); size.max(1)").is_empty());
    assert!(one("let w = a.max(n);").is_empty());
    assert!(one("// m < l").is_empty());
    assert!(one("let n = self.next.checked_add(1)?;").is_empty());
    assert!(one("let k = 1 + n - lo.len();").is_empty());
    assert!(one("let v = x - w.len() + 1;").is_empty());
    assert!(one("let v = x + 1 - widths;").is_empty());
    assert!(one("let v = a.min(n).max(b);").is_empty());
    assert!(one("let v = x.fold(10, |a, b| a);").is_empty());
    assert!(one("let v = s.len() - f.lo() + 10;").is_empty());
}
