//! The 64-bit operator / literal / width kernels have one home (ROADMAP §5.2 PARSE-1 (+ OPS),
//! ER §5.1): `crates/hdl-ast/src/value_rules.rs`; the parser's parse-time constant fold has
//! one home in `crates/hdl-parser/src/const_fold.rs`.
//!
//! This pin claims ONLY the exact token shapes below (T1 T2 T2b T3 T4 T5 T6 T7 T7b), counted
//! per (file, enclosing fn), each exception named with its reason. What no shape sees (an
//! operator arm of another spelling, a value computed before a match, a bit loop, an IR-node
//! extension, a helper of another name) is REVIEW-ONLY in `value_rules.rs`'s head note and is
//! not claimed here.
//!
//! The walker (the one `sysfn_home.rs` uses, extended to emit every token):
//! - every `crates/<crate>/src/**/*.rs` file is read whole with `proc_macro2` (test FILES
//!   under `src/` included); comments are dropped by the lexer, doc comments arrive as
//!   `#[doc]` / `#![doc]` groups;
//! - `#[..]` and `#![..]` attributes are skipped whole; the item after an attribute whose
//!   tokens are EXACTLY `cfg(test)` is skipped through its first brace body or its `;`
//!   (`#[cfg(all(test, ..))]` is NOT skipped: its item is read);
//! - the token tree is flattened depth-first with explicit open / close delimiters, and a
//!   `Joint` punct joins the next punct ONLY when the pair is one of
//!   `<< >> => :: <= >= == != || *=` (so `::<` is `::` then `<`, and `<<=` is `<<` then `=`),
//!   so a shape is the same tokens at any line layout or group nesting;
//! - a token's key is the innermost enclosing `fn` (`-` outside any fn); a definition shape
//!   (T2b T4 T5) is keyed by the defined name.
//!
//! Shapes (S3 = hdl-ast + hdl-parser + elaborate; S* = every crate):
//! - T1 (S3) K1's arm spellings: `=> [*]a.checked_add|sub|mul|div|rem([*]b)` and
//!   `=> Some((a <|<=|>|>=|==|!= b) as i64)`, `a` / `b` single identifiers.
//! - T2 (S*) the bit-length identifiers, whole: `leading_zeros trailing_zeros ilog2 ilog10
//!   ilog checked_ilog2 checked_ilog10 next_power_of_two checked_next_power_of_two log2`.
//! - T2b (S*) a `fn` whose name contains `clog2`.
//! - T3 (S3) literal reads: `parse::<iN|uN|isize|usize>`, `.parse()`, `from_str_radix`,
//!   `from_str`, `to_digit`, `checked_mul(10|8|16|2|radix|base`, `* 10|8|16 +`,
//!   `*= 10|radix|base`, `- b'0'` / `- '0'`.
//! - T4 (S*) `fn const_binop|const_pow|const_shl_i64|binop_in|ctx5_binop|clog2_u64|wrap_i128|
//!   decimal_i64|ctx5_mask` and `enum OpSet`: only in the home, each once; `ctx5_mask` nowhere.
//! - T5 (hdl-parser) the 21 parse-time fold fns defined only in `const_fold.rs`.
//! - T6 (hdl-parser) a VALUE read of the parser's constant table (`const_locals` /
//!   `pkg_const_scoped` followed by `. get (` or `[`) only in `const_fold.rs`.
//! - T7 (S3) K7's sign-extension tokens: `! 0<sfx> <<` ; `| ! <ident ending in mask>` ;
//!   `wrapping_shl (` ; a BINARY `- ( 1<sfx> << <ident> )` (binary = the token before `-` is
//!   an identifier, a number, `)` or `]`) ; the shift pair `<< s ) as iN ) >> s` ;
//!   `as i8|i16|i32|u8|u16|u32 as i64|i128|u64|u128` ; `as i8|i16|i32 ) as i64|i128` ;
//!   `MAX >>`.
//! - T7b (S3) a bare mask `( 1<sfx> << <w> ) - 1` with `<w>` an identifier, `a.b`, or
//!   `( a - 1 )`, and the trailing `1` unsuffixed.
//!
//! Every hit outside `value_rules.rs` is an EXCEPTION (T5 / T6: outside `const_fold.rs`),
//! its count exact; the homes' own counts are pinned too. A new spelling fails here: call the
//! home, or add the key with its reason (a reviewed edit).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{Delimiter, Spacing, Span, TokenStream, TokenTree};

pub const HOME: &str = "crates/hdl-ast/src/value_rules.rs";
pub const FOLD_HOME: &str = "crates/hdl-parser/src/const_fold.rs";
const S3: [&str; 3] = ["hdl-ast", "hdl-parser", "elaborate"];

/// The 21 parse-time fold fns (T5).
const FOLD21: [&str; 21] = [
    "try_const_index",
    "try_const_index_w",
    "try_const_index_wc",
    "cw_shape",
    "cw_eval",
    "const_bound",
    "const_lit",
    "const_lit_enum",
    "read_pattern",
    "based_lit_pattern",
    "lit_u32",
    "plain_int",
    "plain_dec",
    "small_unsized_lit",
    "decimal_literal",
    "index_literal",
    "literal_element",
    "written_decimal",
    "const_param_fits",
    "based_lit_param_value",
    "param_const_shape",
];

/// The kernel definitions (T4).
const KERNELS: [&str; 9] = [
    "const_binop",
    "const_pow",
    "const_shl_i64",
    "binop_in",
    "ctx5_binop",
    "clog2_u64",
    "wrap_i128",
    "decimal_i64",
    "ctx5_mask",
];

/// The home's own counts, per pin (T4: the 8 kernel fns + `enum OpSet`).
#[rustfmt::skip]
const HOME_COUNTS: &[(&str, usize, &str)] = &[
    ("T1", 10, "const_binop's 5 checked and 5 compare arms (its Eq arm is a block)"),
    ("T2", 1, "clog2_u64's leading_zeros"),
    ("T2b", 1, "fn clog2_u64"),
    ("T3", 1, "decimal_i64's parse::<i64>"),
    ("T4", 9, "the 8 kernel fns + enum OpSet, each once"),
    ("T7", 2, "wrap_i128's `!0u64 <<` and ctx5_binop's `p - (1i128 << w)`"),
    ("T7b", 1, "wrap_i128's mask"),
];

/// (pin, file, enclosing fn, hits, reason). Outside `value_rules.rs` (T5 / T6: outside
/// `const_fold.rs`).
#[rustfmt::skip]
const EXCEPTIONS: &[(&str, &str, &str, usize, &str)] = &[
    // T1: K1's arm spellings.
    ("T1", "crates/elaborate/src/block_local/proofs.rs", "fold_i32", 3, "O1 the i32 proof fold: its 11 computing arms equal `const_binop(op, l as i64, r as i64).and_then(|v| i32::try_from(v).ok())` over its op set; home VAL-1 ; VAL-2 lane census"),
    ("T1", "crates/elaborate/src/const_wide.rs", "fold_region", 2, "O9 a VAL core: wide / 4-state"),
    ("T1", "crates/elaborate/src/read_set.rs", "go", 3, "O2 an unsigned fold after lowering: u64 over ir::BinOp"),
    // T2: bit-length identifiers.
    ("T2", "crates/elaborate/src/const_eval.rs", "min_signed_bits", 1, "O4 bit length + 1 of a magnitude"),
    ("T2", "crates/elaborate/src/read_set.rs", "go", 1, "O2 a Shl guard"),
    ("T2", "crates/sim-engine/src/builtins/render.rs", "fmt_time_spec", 1, "ilog10: a $time digit count"),
    ("T2", "crates/sim-engine/src/eval/sysfunc.rs", "eval_sysfunc", 1, "O4 the engine's wide $clog2"),
    ("T2", "crates/sim-engine/src/native/dirty.rs", "collect", 2, "O4 dirty-set bit iteration"),
    ("T2", "crates/sim-engine/src/native/dirty.rs", "drain_with", 2, "O4 dirty-set bit iteration"),
    ("T2", "crates/sim-engine/src/native/dirty.rs", "retain_snapshot", 1, "O4 dirty-set bit iteration"),
    ("T2", "crates/sim-engine/src/native/tests.rs", "exact", 1, "a test file's hand oracle"),
    ("T2", "crates/sim-ir/src/mw.rs", "int_to_real", 1, "O4 crate edge"),
    ("T2", "crates/sim-ir/src/selfwidth.rs", "const_u32_of_expr_ctx", 1, "O4 crate edge"),
    ("T2", "crates/sim-ir/src/selfwidth/edge_value_tests.rs", "clog2_ref", 1, "a test file's hand oracle"),
    // T2b: fns named for clog2.
    ("T2b", "crates/elaborate/src/const_fn.rs", "const_clog2_selfdet", 1, "elaborate's $clog2 reader (L7): calls clog2_u64"),
    ("T2b", "crates/elaborate/src/const_wide.rs", "selfdet_clog2_wide", 1, "O4 wide: a top-bit search"),
    ("T2b", "crates/elaborate/src/const_wide_num.rs", "wide_clog2", 1, "O4 wide: a top-bit search"),
    ("T2b", "crates/sim-engine/src/width_tests.rs", "t12_clog2_time_fixed_widths", 1, "a test fn"),
    ("T2b", "crates/sim-ir/src/selfwidth/edge_value_tests.rs", "clog2_follows_the_engine_arm", 1, "a test fn"),
    ("T2b", "crates/sim-ir/src/selfwidth/edge_value_tests.rs", "clog2_ref", 1, "a test file's hand oracle"),
    // T3: literal reads.
    ("T3", "crates/elaborate/src/const_str.rs", "const_str_method_value", 3, "O5 IEEE 1800-2017 §6.16.9 string methods"),
    ("T3", "crates/elaborate/src/elab_task.rs", "elab_arg_shape", 1, "L13 the elaboration-task renderer's size read (no `_` removal)"),
    ("T3", "crates/elaborate/src/literal.rs", "decimal_bits", 2, "O3 parse_int_literal's value: 4-state, any width"),
    ("T3", "crates/elaborate/src/literal.rs", "digit_bits", 1, "O3 parse_int_literal's value: 4-state, any width"),
    ("T3", "crates/elaborate/src/literal.rs", "unescape_str_literal_reporting", 5, "O5 string-literal escapes (§5.9.1)"),
    ("T3", "crates/elaborate/src/params.rs", "cli_overrides_for", 1, "O5 the `-G` command-line grammar"),
    ("T3", "crates/elaborate/src/tests/mod.rs", "parse_range_expr", 1, "a test file's helper"),
    ("T3", "crates/hdl-parser/src/const_fold.rs", "based_lit_pattern", 2, "L10 the based literal's digit loop"),
    ("T3", "crates/hdl-parser/src/const_fold.rs", "small_unsized_lit", 1, "L10 the radix-generic unsized literal reader"),
    // T6: constant-table value reads outside const_fold.rs.
    ("T6", "crates/hdl-parser/src/imports.rs", "parse_import_term", 1, "O7 the import's table copy"),
    ("T6", "crates/hdl-parser/src/packed_md.rs", "respell_pkg_expr", 1, "O7 the respell builder"),
    // T7: sign-extension tokens.
    ("T7", "crates/elaborate/src/const_fn.rs", "const_eval_i64_lit", 1, "L12 the 64-bit bit-container read"),
    ("T7", "crates/elaborate/src/elab_task.rs", "elab_task_message", 1, "L13 the elaboration-task renderer's signed %d"),
    ("T7", "crates/elaborate/src/value/arms.rs", "signing_over_size_cast", 1, "L14 the cast's mask-and-fill, one body since VAL-1 (dev-profile w = 63 edge, E7)"),
    ("T7", "crates/hdl-parser/src/const_fold.rs", "read_pattern", 1, "L12 the 64-bit bit-container read"),
    // T7b: bare masks -- an identity; its edge convention (O10), a type range (O11) or a lane.
    ("T7b", "crates/elaborate/src/const_eval.rs", "array_param_init_pattern", 1, "O10 bare mask, G64"),
    ("T7b", "crates/elaborate/src/const_eval.rs", "const_delay_u64", 1, "O10 bare mask, G64+"),
    ("T7b", "crates/elaborate/src/const_fn.rs", "const_placement_wide", 1, "O10 bare mask, G64+"),
    ("T7b", "crates/elaborate/src/const_fn.rs", "const_seed", 1, "O10 bare mask, G0/64+"),
    ("T7b", "crates/elaborate/src/const_fn.rs", "const_unsigned_selfdet", 1, "O10 bare mask, R1-63"),
    ("T7b", "crates/elaborate/src/const_fn.rs", "exec_const_select_write", 1, "O10 bare mask, G64+"),
    ("T7b", "crates/elaborate/src/const_fn_width/fold.rs", "fold_ctx", 2, "O10 bare masks, G64 / U"),
    ("T7b", "crates/elaborate/src/const_fn_width/fold.rs", "ones", 1, "O10 bare mask, G64+"),
    ("T7b", "crates/elaborate/src/const_real.rs", "const_selfdet_real", 1, "O10 bare mask, G64"),
    ("T7b", "crates/elaborate/src/const_select.rs", "const_param_select_env", 1, "O10 bare mask, N63"),
    ("T7b", "crates/elaborate/src/const_select.rs", "select_base_at_declared", 1, "O10 bare mask, G64+"),
    ("T7b", "crates/elaborate/src/const_wide_num.rs", "bp_mask_to", 1, "O10 bare mask, U"),
    ("T7b", "crates/elaborate/src/crv.rs", "rand_type_range", 2, "O11 a type's range"),
    ("T7b", "crates/elaborate/src/dead_arm.rs", "known_leaf", 1, "O10 bare mask, G64"),
    ("T7b", "crates/elaborate/src/elab_task.rs", "elab_task_message", 1, "L13 the renderer's mask"),
    ("T7b", "crates/elaborate/src/instance_array.rs", "resolve_range_end", 2, "O11 a type's range"),
    ("T7b", "crates/elaborate/src/literal.rs", "make_const_i64", 1, "O10 bare mask, G<64"),
    ("T7b", "crates/elaborate/src/net_util.rs", "check_enum_label_fits", 1, "O11 a type's range"),
    ("T7b", "crates/elaborate/src/value/arms.rs", "signing_over_size_cast", 1, "L14 the cast's mask, one body since VAL-1"),
    ("T7b", "crates/elaborate/src/wildcard_eq.rs", "wildcard_cmp_ids", 1, "O10 bare mask, U"),
    ("T7b", "crates/hdl-parser/src/const_fold.rs", "based_lit_pattern", 1, "O10 bare mask, G64"),
    ("T7b", "crates/hdl-parser/src/const_fold.rs", "const_lit_enum", 1, "O10 bare mask, G64"),
    ("T7b", "crates/hdl-parser/src/const_fold.rs", "cw_eval", 1, "L4 the context-width mask"),
    ("T7b", "crates/hdl-parser/src/typedefs.rs", "parse_typedef_inner", 2, "O11 a type's range (L15)"),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Id,
    Num,
    Chr,
    Str,
    P,
}

/// One flattened token: kind, text, `Joint` spacing, the innermost enclosing fn, its span.
#[derive(Clone, Debug)]
pub struct Tok {
    pub kind: Kind,
    pub text: String,
    pub joint: bool,
    pub key: String,
    pub span: Span,
}

/// One hit: (pin, shape, key, the token index the shape starts at).
pub struct Hit {
    pub pin: &'static str,
    pub shape: &'static str,
    pub key: String,
    pub span: Span,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .map(|e| e.expect("dir entry").path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Every `crates/<crate>/src/**/*.rs`, relative to the workspace root, sorted: (rel, source).
pub fn src_files(root: &Path) -> Vec<(String, String)> {
    let mut crates: Vec<_> = std::fs::read_dir(root.join("crates"))
        .expect("crates/")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.join("src").is_dir())
        .collect();
    crates.sort();
    let mut files = Vec::new();
    for c in crates {
        rs_files(&c.join("src"), &mut files);
    }
    files
        .into_iter()
        .map(|p| {
            let rel = p
                .strip_prefix(root)
                .expect("under root")
                .to_string_lossy()
                .replace('\\', "/");
            let src = std::fs::read_to_string(&p).expect("read source");
            (rel, src)
        })
        .collect()
}

fn is_cfg_test(attr: &TokenStream) -> bool {
    let s: String = attr
        .to_string()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    s == "cfg(test)"
}

fn push(out: &mut Vec<Tok>, kind: Kind, text: String, joint: bool, key: &str, span: Span) {
    out.push(Tok {
        kind,
        text,
        joint,
        key: key.to_string(),
        span,
    });
}

fn delims(d: Delimiter) -> (&'static str, &'static str) {
    match d {
        Delimiter::Parenthesis => ("(", ")"),
        Delimiter::Brace => ("{", "}"),
        Delimiter::Bracket => ("[", "]"),
        Delimiter::None => ("", ""),
    }
}

fn walk(ts: TokenStream, cur: &str, out: &mut Vec<Tok>) {
    let tt: Vec<TokenTree> = ts.into_iter().collect();
    let mut pending_fn: Option<String> = None;
    let mut i = 0;
    while i < tt.len() {
        match &tt[i] {
            TokenTree::Punct(p) if p.as_char() == '#' => {
                // `#[..]` or `#![..]`: skipped whole (doc comments arrive here).
                let mut j = i + 1;
                if matches!(tt.get(j), Some(TokenTree::Punct(q)) if q.as_char() == '!') {
                    j += 1;
                }
                if let Some(TokenTree::Group(g)) = tt.get(j) {
                    if g.delimiter() == Delimiter::Bracket {
                        i = j + 1;
                        if is_cfg_test(&g.stream()) {
                            // Skip the item: through its first brace body or its `;`.
                            while i < tt.len() {
                                match &tt[i] {
                                    TokenTree::Group(b) if b.delimiter() == Delimiter::Brace => {
                                        i += 1;
                                        break;
                                    }
                                    TokenTree::Punct(q) if q.as_char() == ';' => {
                                        i += 1;
                                        break;
                                    }
                                    _ => i += 1,
                                }
                            }
                        }
                        continue;
                    }
                }
                push(
                    out,
                    Kind::P,
                    "#".into(),
                    p.spacing() == Spacing::Joint,
                    cur,
                    p.span(),
                );
                i += 1;
            }
            TokenTree::Ident(id) if id == "fn" => {
                push(out, Kind::Id, "fn".into(), false, cur, id.span());
                if let Some(TokenTree::Ident(name)) = tt.get(i + 1) {
                    pending_fn = Some(name.to_string());
                    push(out, Kind::Id, name.to_string(), false, cur, name.span());
                    i += 2;
                    continue;
                }
                i += 1;
            }
            TokenTree::Punct(p) if p.as_char() == ';' => {
                pending_fn = None;
                push(
                    out,
                    Kind::P,
                    ";".into(),
                    p.spacing() == Spacing::Joint,
                    cur,
                    p.span(),
                );
                i += 1;
            }
            TokenTree::Group(g) => {
                let mut key = cur.to_string();
                if g.delimiter() == Delimiter::Brace {
                    if let Some(f) = pending_fn.take() {
                        key = f;
                    }
                }
                let (o, c) = delims(g.delimiter());
                if !o.is_empty() {
                    push(out, Kind::P, o.into(), false, &key, g.span_open());
                }
                walk(g.stream(), &key, out);
                if !c.is_empty() {
                    push(out, Kind::P, c.into(), false, &key, g.span_close());
                }
                i += 1;
            }
            TokenTree::Ident(id) => {
                push(out, Kind::Id, id.to_string(), false, cur, id.span());
                i += 1;
            }
            TokenTree::Punct(p) => {
                push(
                    out,
                    Kind::P,
                    p.as_char().to_string(),
                    p.spacing() == Spacing::Joint,
                    cur,
                    p.span(),
                );
                i += 1;
            }
            TokenTree::Literal(l) => {
                let s = l.to_string();
                let kind = if s.starts_with('\'') || s.starts_with("b'") {
                    Kind::Chr
                } else if s.starts_with(|c: char| c.is_ascii_digit()) {
                    Kind::Num
                } else {
                    Kind::Str
                };
                push(out, kind, s, false, cur, l.span());
                i += 1;
            }
        }
    }
}

/// The pairs a `Joint` punct joins with the next punct.
const PAIRS: [&str; 10] = ["<<", ">>", "=>", "::", "<=", ">=", "==", "!=", "||", "*="];

fn join(seq: Vec<Tok>) -> Vec<Tok> {
    let mut res: Vec<Tok> = Vec::with_capacity(seq.len());
    let mut i = 0;
    while i < seq.len() {
        let t = &seq[i];
        if t.kind == Kind::P && t.joint && i + 1 < seq.len() && seq[i + 1].kind == Kind::P {
            let pair = format!("{}{}", t.text, seq[i + 1].text);
            if PAIRS.contains(&pair.as_str()) {
                let mut j = t.clone();
                j.text = pair;
                j.joint = seq[i + 1].joint;
                res.push(j);
                i += 2;
                continue;
            }
        }
        res.push(t.clone());
        i += 1;
    }
    res
}

/// The flattened, joined tokens of one source file.
pub fn tokens(src: &str) -> Vec<Tok> {
    let ts = TokenStream::from_str(src).unwrap_or_else(|e| panic!("tokenize: {e:?}"));
    let mut out = Vec::new();
    walk(ts, "-", &mut out);
    join(out)
}

const INTS: [&str; 12] = [
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
];
const CHK: [&str; 5] = [
    "checked_add",
    "checked_sub",
    "checked_mul",
    "checked_div",
    "checked_rem",
];
const CMP: [&str; 6] = ["<", "<=", ">", ">=", "==", "!="];
const BITLEN: [&str; 10] = [
    "leading_zeros",
    "trailing_zeros",
    "ilog2",
    "ilog10",
    "ilog",
    "checked_ilog2",
    "checked_ilog10",
    "next_power_of_two",
    "checked_next_power_of_two",
    "log2",
];

fn sfx_num(t: &str, digit: char) -> bool {
    let Some(rest) = t.strip_prefix(digit) else {
        return false;
    };
    match rest.strip_prefix('_') {
        Some(r) => INTS.contains(&r),
        None => rest.is_empty() || INTS.contains(&rest),
    }
}

fn tx(t: &[Tok], k: isize) -> &str {
    if k >= 0 && (k as usize) < t.len() {
        t[k as usize].text.as_str()
    } else {
        ""
    }
}

fn kd(t: &[Tok], k: isize) -> Option<Kind> {
    if k >= 0 && (k as usize) < t.len() {
        Some(t[k as usize].kind)
    } else {
        None
    }
}

/// The shapes starting at token `i`: (pin, shape).
pub fn shapes(t: &[Tok], i: usize) -> Vec<(&'static str, &'static str)> {
    let tx = |k: isize| tx(t, k);
    let kd = |k: isize| kd(t, k);
    let id = |k: isize| kd(k) == Some(Kind::Id);
    let one = |k: isize| kd(k) == Some(Kind::Num) && sfx_num(tx(k), '1');
    let zero = |k: isize| kd(k) == Some(Kind::Num) && sfx_num(tx(k), '0');
    let i = i as isize;
    let v = tx(i);
    let p = kd(i) == Some(Kind::P);
    let mut out = Vec::new();
    // T1: K1's checked arm and compare arm.
    if p && v == "=>" {
        let j = i + 1 + isize::from(tx(i + 1) == "*");
        if id(j) && tx(j + 1) == "." && CHK.contains(&tx(j + 2)) && tx(j + 3) == "(" {
            let k = j + 4 + isize::from(tx(j + 4) == "*");
            if id(k) && tx(k + 1) == ")" {
                out.push(("T1", "checked arm"));
            }
        }
        if tx(i + 1) == "Some"
            && tx(i + 2) == "("
            && tx(i + 3) == "("
            && id(i + 4)
            && CMP.contains(&tx(i + 5))
            && id(i + 6)
            && tx(i + 7) == ")"
            && tx(i + 8) == "as"
            && tx(i + 9) == "i64"
            && tx(i + 10) == ")"
        {
            out.push(("T1", "compare arm"));
        }
    }
    // T2: bit-length identifiers.
    if id(i) && BITLEN.contains(&v) {
        out.push(("T2", "bit length"));
    }
    // T3: literal reads.
    if v == "parse"
        && tx(i + 1) == "::"
        && tx(i + 2) == "<"
        && INTS.contains(&tx(i + 3))
        && tx(i + 4) == ">"
    {
        out.push(("T3", "parse::<int>"));
    }
    if v == "parse" && tx(i - 1) == "." && tx(i + 1) == "(" && tx(i + 2) == ")" {
        out.push(("T3", ".parse()"));
    }
    if id(i) && matches!(v, "from_str_radix" | "to_digit" | "from_str") {
        out.push(("T3", "radix / digit read"));
    }
    if v == "checked_mul"
        && tx(i + 1) == "("
        && matches!(tx(i + 2), "10" | "8" | "16" | "2" | "radix" | "base")
    {
        out.push(("T3", "checked_mul(radix)"));
    }
    if v == "*" && matches!(tx(i + 1), "10" | "8" | "16") && tx(i + 2) == "+" {
        out.push(("T3", "* 10 +"));
    }
    if v == "*=" && matches!(tx(i + 1), "10" | "radix" | "base") {
        out.push(("T3", "*= 10"));
    }
    if p && v == "-" && kd(i + 1) == Some(Kind::Chr) && matches!(tx(i + 1), "b'0'" | "'0'") {
        out.push(("T3", "- b'0'"));
    }
    // T7: K7's sign-extension tokens.
    if v == "!" && zero(i + 1) && tx(i + 2) == "<<" {
        out.push(("T7", "!0 <<"));
    }
    if p && v == "|" && tx(i + 1) == "!" && id(i + 2) && tx(i + 2).ends_with("mask") {
        out.push(("T7", "| !mask"));
    }
    if v == "wrapping_shl" && tx(i + 1) == "(" {
        out.push(("T7", "wrapping_shl("));
    }
    if p && v == "-"
        && (matches!(kd(i - 1), Some(Kind::Id) | Some(Kind::Num)) || matches!(tx(i - 1), ")" | "]"))
        && tx(i + 1) == "("
        && one(i + 2)
        && tx(i + 3) == "<<"
        && id(i + 4)
        && tx(i + 5) == ")"
    {
        out.push(("T7", "- (1 << w)"));
    }
    if v == "<<"
        && id(i + 1)
        && tx(i + 2) == ")"
        && tx(i + 3) == "as"
        && INTS.contains(&tx(i + 4))
        && tx(i + 5) == ")"
        && tx(i + 6) == ">>"
        && tx(i + 7) == tx(i + 1)
    {
        out.push(("T7", "shift pair"));
    }
    if v == "as"
        && matches!(tx(i + 1), "i8" | "i16" | "i32" | "u8" | "u16" | "u32")
        && tx(i + 2) == "as"
        && matches!(tx(i + 3), "i64" | "i128" | "u64" | "u128")
    {
        out.push(("T7", "as iN as i64"));
    }
    if v == "as"
        && matches!(tx(i + 1), "i8" | "i16" | "i32")
        && tx(i + 2) == ")"
        && tx(i + 3) == "as"
        && matches!(tx(i + 4), "i64" | "i128")
    {
        out.push(("T7", "(x as iN) as i64"));
    }
    if v == "MAX" && tx(i + 1) == ">>" {
        out.push(("T7", "MAX >>"));
    }
    // T7b: a bare mask `(1 << w) - 1`.
    if v == "(" && one(i + 1) && tx(i + 2) == "<<" {
        let j = i + 3;
        let e = if id(j) && tx(j + 1) == ")" {
            Some(j + 1)
        } else if id(j) && tx(j + 1) == "." && id(j + 2) && tx(j + 3) == ")" {
            Some(j + 3)
        } else if tx(j) == "("
            && id(j + 1)
            && tx(j + 2) == "-"
            && tx(j + 3) == "1"
            && tx(j + 4) == ")"
            && tx(j + 5) == ")"
        {
            Some(j + 5)
        } else {
            None
        };
        if let Some(e) = e {
            if tx(e + 1) == "-" && tx(e + 2) == "1" {
                out.push(("T7b", "(1 << w) - 1"));
            }
        }
    }
    // T4 / T2b / T5: definitions.
    if v == "fn" && id(i + 1) {
        let nm = tx(i + 1);
        if KERNELS.contains(&nm) {
            out.push(("T4", "fn"));
        }
        if nm.contains("clog2") {
            out.push(("T2b", "fn"));
        }
        if FOLD21.contains(&nm) {
            out.push(("T5", "fn"));
        }
    }
    if v == "enum" && tx(i + 1) == "OpSet" {
        out.push(("T4", "enum"));
    }
    // T6: a value read of the parser's constant table.
    if matches!(v, "const_locals" | "pkg_const_scoped")
        && ((tx(i + 1) == "." && tx(i + 2) == "get" && tx(i + 3) == "(") || tx(i + 1) == "[")
    {
        out.push(("T6", "table read"));
    }
    out
}

fn in_scope(pin: &str, krate: &str) -> bool {
    match pin {
        "T1" | "T3" | "T7" | "T7b" => S3.contains(&krate),
        "T5" | "T6" => krate == "hdl-parser",
        _ => true,
    }
}

/// Every hit of one file `rel` (`crates/<crate>/src/..`).
pub fn file_hits(rel: &str, src: &str) -> Vec<Hit> {
    let krate = rel.split('/').nth(1).unwrap_or("");
    let t = tokens(src);
    let mut out = Vec::new();
    for i in 0..t.len() {
        for (pin, shape) in shapes(&t, i) {
            if !in_scope(pin, krate) {
                continue;
            }
            let key = if matches!(pin, "T4" | "T2b" | "T5") {
                t[i + 1].text.clone()
            } else {
                t[i].key.clone()
            };
            out.push(Hit {
                pin,
                shape,
                key,
                span: t[i].span,
            });
        }
    }
    out
}

/// (pin, file, key) -> hits over every src file.
pub fn all_hits(files: &[(String, String)]) -> BTreeMap<(String, String, String), usize> {
    let mut m = BTreeMap::new();
    for (rel, src) in files {
        for h in file_hits(rel, src) {
            *m.entry((h.pin.to_string(), rel.clone(), h.key))
                .or_insert(0) += 1;
        }
    }
    m
}

fn home_of(pin: &str) -> &'static str {
    if matches!(pin, "T5" | "T6") {
        FOLD_HOME
    } else {
        HOME
    }
}

#[test]
fn value_rule_shapes_live_in_the_homes() {
    let root = workspace_root();
    let files = src_files(&root);
    let all = all_hits(&files);
    let mut got: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    let mut home: BTreeMap<String, usize> = BTreeMap::new();
    let mut home_keys: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for ((pin, file, key), n) in &all {
        if file == home_of(pin) {
            *home.entry(pin.clone()).or_insert(0) += n;
            *home_keys
                .entry(pin.clone())
                .or_default()
                .entry(key.clone())
                .or_insert(0) += n;
        } else {
            got.insert((pin.clone(), file.clone(), key.clone()), *n);
        }
    }
    let want: BTreeMap<(String, String, String), usize> = EXCEPTIONS
        .iter()
        .map(|(p, f, k, c, _)| ((p.to_string(), f.to_string(), k.to_string()), *c))
        .collect();
    assert_eq!(
        want.len(),
        EXCEPTIONS.len(),
        "an EXCEPTIONS key listed twice"
    );
    let mut msg = String::new();
    for k in got.keys().chain(want.keys()).collect::<BTreeSet<_>>() {
        let (g, w) = (got.get(k).copied(), want.get(k).copied());
        if g != w {
            msg += &format!(
                "  {} | {} | {} : measured {g:?} listed {w:?}\n",
                k.0, k.1, k.2
            );
        }
    }
    // The homes' own counts.
    for (pin, n, why) in HOME_COUNTS {
        let g = home.get(*pin).copied().unwrap_or(0);
        if g != *n {
            msg += &format!("  home {pin} ({why}): measured {g} listed {n}\n");
        }
    }
    // T4: each kernel defined once in the home; `ctx5_mask` nowhere.
    let t4 = home_keys.get("T4").cloned().unwrap_or_default();
    for k in KERNELS.iter().filter(|k| **k != "ctx5_mask") {
        if t4.get(*k).copied() != Some(1) {
            msg += &format!("  home T4 fn {k}: measured {:?} want Some(1)\n", t4.get(*k));
        }
    }
    if t4.get("OpSet").copied() != Some(1) {
        msg += &format!(
            "  home T4 enum OpSet: measured {:?} want Some(1)\n",
            t4.get("OpSet")
        );
    }
    if all.keys().any(|(p, _, k)| p == "T4" && k == "ctx5_mask") {
        msg += "  T4 `fn ctx5_mask` is defined somewhere\n";
    }
    // T5: the 21 folds each defined once in const_fold.rs.
    let t5 = home_keys.get("T5").cloned().unwrap_or_default();
    for k in FOLD21 {
        if t5.get(k).copied() != Some(1) {
            msg += &format!(
                "  T5 fn {k} in const_fold.rs: measured {:?} want Some(1)\n",
                t5.get(k)
            );
        }
    }
    assert!(msg.is_empty(), "measured != pinned:\n{msg}");
}

#[test]
fn scanner_matches_its_controls() {
    let count = |pin: &str, src: &str| {
        file_hits("crates/elaborate/src/x.rs", src)
            .into_iter()
            .filter(|h| h.pin == pin)
            .count()
    };
    let keys = |pin: &str, src: &str| {
        file_hits("crates/elaborate/src/x.rs", src)
            .into_iter()
            .filter(|h| h.pin == pin)
            .map(|h| h.key)
            .collect::<Vec<_>>()
    };
    // T1 positive: an arm split across lines; an arm in a fn reached through `use BinOp::*`.
    assert_eq!(
        count(
            "T1",
            "fn f(op: BinOp, a: i64, b: i64) -> Option<i64> { match op { BinOp::Add\n =>\n a\n .checked_add(\n b), _ => None } }"
        ),
        1
    );
    assert_eq!(
        count(
            "T1",
            "fn f(op: BinOp, a: i64, b: i64) -> Option<i64> { use BinOp::*; match op { Lt => Some((a < b) as i64), Mul => a.checked_mul(*b), _ => None } }"
        ),
        2
    );
    // T1 negative: a call to another fn; a checked op whose operand is not one identifier.
    assert_eq!(
        count(
            "T1",
            "fn f() { match op { Add => lower_add(a, b), _ => None } }"
        ),
        0
    );
    assert_eq!(
        count(
            "T1",
            "fn f() { match op { Add => a.checked_add(b.len()), _ => None } }"
        ),
        0
    );
    // T2 positive / negative (whole identifiers; a comment is not a token).
    assert_eq!(count("T2", "fn f(x: u64) -> u32 { x.ilog2() }"), 1);
    assert_eq!(
        count(
            "T2",
            "fn f() { wide_clog2(x); strip_trailing_zeros(y); // leading_zeros\n }"
        ),
        0
    );
    // T3 positive: a turbofish read split over lines (`::<` is `::` then `<`); negative: f64.
    assert_eq!(
        count("T3", "fn f(s: &str) { str::parse\n ::<\n u16>(s); }"),
        1
    );
    assert_eq!(count("T3", "fn f(s: &str) { s.parse::<f64>(); }"), 0);
    // T7 positive / negative.
    assert_eq!(count("T7", "fn f() { v | !mask }"), 1);
    assert_eq!(count("T7", "fn f() { x - (1i128 << w) }"), 1);
    assert_eq!(
        count(
            "T7",
            "fn f() { it.filter(|x| !seen(x)); a || !b; -(1i64 << bits) }"
        ),
        0
    );
    // T7b positive / negative (the trailing `1` unsuffixed).
    assert_eq!(count("T7b", "fn f() { (1u64 << (w - 1)) - 1 }"), 1);
    assert_eq!(count("T7b", "fn f() { 1 << k; (1u64 << w) - 1u64 }"), 0);
    // The walker: attributes and the item after an exact `#[cfg(test)]` are skipped; a
    // `#[cfg(all(test, ..))]` item is read; the key is the innermost fn.
    assert_eq!(
        count(
            "T2",
            "#[cfg(test)]\nmod t { fn g(x: u64) -> u32 { x.ilog2() } }"
        ),
        0
    );
    assert_eq!(
        count(
            "T2",
            "#[cfg(all(test, feature = \"o\"))]\nmod t { fn g(x: u64) -> u32 { x.ilog2() } }"
        ),
        1
    );
    assert_eq!(count("T2", "#[doc = \"x.ilog2()\"] fn g() {}"), 0);
    assert_eq!(
        keys(
            "T2",
            "fn outer() { fn inner(x: u64) -> u32 { x.ilog2() } x.ilog2(); }"
        ),
        vec!["inner".to_string(), "outer".to_string()]
    );
}
