//! The constant evaluator's entry set, its binder chain, its declared-range check and its
//! live-scope REG ask are each written once (ROADMAP §5.2 VAL-2, VAL-3, ER §5.1):
//! `crates/elaborate/src/value/` -- `chains.rs` (`param_decl_fold`, the declaration chain the four
//! binders take ; `range_bounds_checked`, the range router's fold-then-check the four loud range
//! sites take) and `lanes.rs` (`fold_self_wide`, REG asked in the live scope), with the lane table
//! and the router's paths in `value/mod.rs`'s head note.
//!
//! This pin claims ONLY the exact token shapes below (E1-E7), counted per (file, enclosing fn)
//! over `crates/elaborate/src/**/*.rs`. What no shape sees (an entry reached through a fn value
//! or a macro, a chain spelled with `match` / `if let`, a decline read after a `let`, the
//! predicate readers, every chain not listed) is REVIEW-ONLY in `value/mod.rs`'s head note and
//! is not claimed here.
//!
//! The walker is `value_arms_home.rs`'s: every file read whole with `proc_macro2` (comments
//! dropped by the lexer, doc comments arrive as `#[doc]` groups), `#[..]` / `#![..]` skipped
//! whole, the item after an attribute whose tokens are EXACTLY `cfg(test)` skipped through its
//! first brace body or its `;`, the tree flattened with explicit delimiters, a `Joint` punct
//! joined to the next only for `=> == != ::`, a token keyed by its innermost enclosing `fn`
//! (`-` outside any fn).
//!
//! Shapes:
//! - E1 `fn <name>` for each name of the entry list: once, in its named file (`fold_shift_count`
//!   once in each of its two files), nowhere else.
//! - E2 calls (`<name> (`, the token before not `fn`) of the non-IN cores, the frame entries
//!   and the two VAL-2 fns, per (callee, file, fn). IN's calls are the ratchet's key c.
//! - E3 the binder chain: calls of its five members per (callee, file, fn) ; in
//!   `param_decl_fold` the five in order, step 1 inside `fill . then ( | |` and steps 2-5 each
//!   inside an `or_else ( | |` (lazy), and no `then_some` there.
//! - E4 a `fold_self_bits (` call whose argument tokens hold `wide_name_bits`.
//! - E5 `had_error` tokens: each reader asks "any error so far" (head note).
//! - E6 `unwrap_or ( ( 0 , false ) )` (a declined width read as width 0) ; a
//!   constant-function local's reading moved into its writer `ConstFrame::bind_shape`
//!   (BIND-2a).
//! - E7 (VAL-3) calls of the range router and its check (`const_range_bound_fold`,
//!   `check_const_range_bound`, `range_bounds_checked`, `range_bound_fold_ev`, `bound_step`) per
//!   (callee, file, fn) ; in `range_bounds_checked` exactly four calls, each its own statement
//!   (eager: the tokens from the statement's start), in PRE's order and with PRE's arguments:
//!   `let m = self . const_range_bound_fold ( & r . msb )`, `let l = self . const_range_bound_fold
//!   ( & r . lsb )`, `self . check_const_range_bound ( & r . msb , m )`, `self .
//!   check_const_range_bound ( & r . lsb , l )`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

const SRC: &str = "crates/elaborate/src";
const CHAINS: &str = "crates/elaborate/src/value/chains.rs";
const NEW_ROW: &str = "add its row to value/mod.rs's lane table and to this test";
const E7_MSG: &str = "a declared bound is folded and checked in value/chains.rs ; say in value/mod.rs why this site is not";

/// E1: the entry list (the VAL-2 grounding's 49 + the two VAL-2 fns + VAL-3's check chain), each
/// in its file.
#[rustfmt::skip]
const ENTRIES: &[(&str, &str)] = &[
    ("const_eval_in_scope", "crates/elaborate/src/const_fn.rs"),
    ("eval_const_env", "crates/elaborate/src/const_fn.rs"),
    ("fold_ctx", "crates/elaborate/src/const_fn_width/fold.rs"),
    ("fold_region", "crates/elaborate/src/const_wide.rs"),
    ("env_fold", "crates/elaborate/src/expr_size_hier.rs"),
    ("fold_self", "crates/elaborate/src/const_fn_width/fold.rs"),
    ("fold_assign", "crates/elaborate/src/const_fn_width/fold.rs"),
    ("fold_shift_count", "crates/elaborate/src/const_wide.rs"),
    ("fold_shift_count", "crates/elaborate/src/const_fn_width/fold.rs"),
    ("fold_bits_at", "crates/elaborate/src/const_wide.rs"),
    ("fold_bits_at0", "crates/elaborate/src/const_wide.rs"),
    ("fold_self_bits", "crates/elaborate/src/const_wide.rs"),
    ("fold_selfdet_operand", "crates/elaborate/src/const_wide.rs"),
    ("eval_const_env_at", "crates/elaborate/src/const_fn_width.rs"),
    ("eval_const_env_self", "crates/elaborate/src/const_fn_width.rs"),
    ("eval_const_shift_count", "crates/elaborate/src/const_fn_width.rs"),
    ("eval_const_assign", "crates/elaborate/src/const_fn_width.rs"),
    ("eval_const_call", "crates/elaborate/src/const_fn.rs"),
    ("eval_const_call_at", "crates/elaborate/src/const_fn.rs"),
    ("const_int_selfdet", "crates/elaborate/src/const_fn.rs"),
    ("const_unsigned_selfdet", "crates/elaborate/src/const_fn.rs"),
    ("const_clog2_selfdet", "crates/elaborate/src/const_fn.rs"),
    ("const_pow_exponent_selfdet", "crates/elaborate/src/const_fn.rs"),
    ("selfdet_bits_i64", "crates/elaborate/src/const_wide.rs"),
    ("selfdet_bits_unsigned", "crates/elaborate/src/const_wide.rs"),
    ("selfdet_clog2_wide", "crates/elaborate/src/const_wide.rs"),
    ("selfdet_truth", "crates/elaborate/src/const_wide.rs"),
    ("selfdet_truth_arm", "crates/elaborate/src/value/arms.rs"),
    ("wide_selfdet_width", "crates/elaborate/src/const_wide.rs"),
    ("const_eval_u32", "crates/elaborate/src/const_fn.rs"),
    ("const_eval_i64_lit", "crates/elaborate/src/const_fn.rs"),
    ("const_bound_u32", "crates/elaborate/src/const_bound.rs"),
    ("const_bound_signed", "crates/elaborate/src/const_bound.rs"),
    ("repeat_unroll_count", "crates/elaborate/src/const_bound.rs"),
    ("const_range_bound_fold", "crates/elaborate/src/const_select.rs"),
    ("const_range_bound_fold_at", "crates/elaborate/src/const_select.rs"),
    ("range_bound_fold_ev", "crates/elaborate/src/const_fn_width/fold_x.rs"),
    ("const_delay_ticks", "crates/elaborate/src/const_eval.rs"),
    ("delay_ticks_in_scope", "crates/elaborate/src/const_eval.rs"),
    ("ca_delay_value", "crates/elaborate/src/const_eval.rs"),
    ("param_i64_at_declared", "crates/elaborate/src/const_wide.rs"),
    ("param_bits_at_declared", "crates/elaborate/src/const_wide.rs"),
    ("override_bits", "crates/elaborate/src/const_wide.rs"),
    ("override_self_value", "crates/elaborate/src/param_query.rs"),
    ("eval_param_init", "crates/elaborate/src/params.rs"),
    ("param_init_x", "crates/elaborate/src/params.rs"),
    ("const_truth_in_scope", "crates/elaborate/src/const_real.rs"),
    ("const_int_via_real", "crates/elaborate/src/const_real.rs"),
    ("const_rtoi_via_real", "crates/elaborate/src/const_real.rs"),
    ("param_decl_fold", "crates/elaborate/src/value/chains.rs"),
    ("fold_self_wide", "crates/elaborate/src/value/lanes.rs"),
    ("range_bounds_checked", "crates/elaborate/src/value/chains.rs"),
];

/// E2's callees.
const E2_CALLEES: [&str; 17] = [
    "eval_const_env",
    "fold_ctx",
    "fold_self",
    "fold_assign",
    "fold_region",
    "fold_bits_at",
    "fold_bits_at0",
    "fold_self_bits",
    "fold_selfdet_operand",
    "env_fold",
    "eval_const_env_self",
    "eval_const_env_at",
    "eval_const_assign",
    "eval_const_call",
    "eval_const_call_at",
    "fold_self_wide",
    "param_decl_fold",
];

/// E2: (callee, file, fn, calls).
#[rustfmt::skip]
const E2: &[(&str, &str, &str, usize)] = &[
    ("env_fold", "crates/elaborate/src/expr_size_hier.rs", "child_env", 3),
    ("env_fold", "crates/elaborate/src/expr_size_hier.rs", "env_fold", 6),
    ("env_fold", "crates/elaborate/src/expr_size_hier.rs", "fold_width", 2),
    ("eval_const_assign", "crates/elaborate/src/const_fn.rs", "bind_const_decl", 1),
    ("eval_const_assign", "crates/elaborate/src/const_fn.rs", "eval_const_call_at", 2),
    ("eval_const_assign", "crates/elaborate/src/const_fn.rs", "exec_const_select_write", 1),
    ("eval_const_assign", "crates/elaborate/src/const_fn.rs", "exec_const_stmt", 2),
    ("eval_const_assign", "crates/elaborate/src/param_query.rs", "override_self_value", 1),
    ("eval_const_assign", "crates/elaborate/src/value/arms.rs", "const_size_cast", 1),
    ("eval_const_call", "crates/elaborate/src/const_fn.rs", "const_eval_in_scope", 2),
    ("eval_const_call", "crates/elaborate/src/const_fn.rs", "eval_const_env", 1),
    ("eval_const_call_at", "crates/elaborate/src/const_fn.rs", "eval_const_call", 2),
    ("eval_const_env", "crates/elaborate/src/const_fn.rs", "eval_const_env", 6),
    ("eval_const_env", "crates/elaborate/src/const_fn_width/fold.rs", "fold_assign", 1),
    ("eval_const_env", "crates/elaborate/src/const_fn_width/fold.rs", "fold_ctx", 1),
    ("eval_const_env_at", "crates/elaborate/src/const_fn_width/fold.rs", "fold_ctx", 2),
    ("eval_const_env_self", "crates/elaborate/src/const_fn.rs", "const_int_selfdet", 1),
    ("eval_const_env_self", "crates/elaborate/src/const_fn.rs", "const_pow_exponent_selfdet", 1),
    ("eval_const_env_self", "crates/elaborate/src/const_fn.rs", "const_unsigned_selfdet", 1),
    ("eval_const_env_self", "crates/elaborate/src/const_fn.rs", "eval_const_env", 2),
    ("eval_const_env_self", "crates/elaborate/src/const_fn.rs", "exec_const_stmt", 4),
    ("eval_const_env_self", "crates/elaborate/src/const_fn_width/fold.rs", "fold_ctx", 1),
    ("eval_const_env_self", "crates/elaborate/src/const_fn_width/fold.rs", "fold_shift_count", 1),
    ("eval_const_env_self", "crates/elaborate/src/const_select.rs", "select_span", 5),
    ("eval_const_env_self", "crates/elaborate/src/value/arms.rs", "selfdet_truth_arm", 1),
    ("fold_assign", "crates/elaborate/src/const_fn_width.rs", "eval_const_assign", 1),
    ("fold_assign", "crates/elaborate/src/params.rs", "param_init_x", 1),
    ("fold_bits_at", "crates/elaborate/src/const_eval.rs", "wide_param_const_in_scope", 2),
    ("fold_bits_at", "crates/elaborate/src/const_wide.rs", "fold_bits_at0", 1),
    ("fold_bits_at", "crates/elaborate/src/const_wide.rs", "fold_region", 1),
    ("fold_bits_at", "crates/elaborate/src/const_wide.rs", "fold_self_bits", 1),
    ("fold_bits_at", "crates/elaborate/src/const_wide.rs", "fold_selfdet_operand", 1),
    ("fold_bits_at", "crates/elaborate/src/const_wide.rs", "override_bits", 1),
    ("fold_bits_at", "crates/elaborate/src/const_wide.rs", "param_bits_at_declared", 1),
    ("fold_bits_at0", "crates/elaborate/src/const_wide.rs", "fold_region", 7),
    ("fold_bits_at0", "crates/elaborate/src/const_wide.rs", "fold_selfdet_operand", 1),
    ("fold_ctx", "crates/elaborate/src/const_fn_width.rs", "eval_const_env_at", 1),
    ("fold_ctx", "crates/elaborate/src/const_fn_width/fold.rs", "fold_assign", 1),
    ("fold_ctx", "crates/elaborate/src/const_fn_width/fold.rs", "fold_ctx", 7),
    ("fold_ctx", "crates/elaborate/src/const_fn_width/fold.rs", "fold_self", 1),
    ("fold_region", "crates/elaborate/src/const_wide.rs", "fold_bits_at", 2),
    ("fold_region", "crates/elaborate/src/const_wide.rs", "fold_region", 16),
    ("fold_self", "crates/elaborate/src/const_fn_width.rs", "eval_const_env_self", 1),
    ("fold_self", "crates/elaborate/src/const_fn_width/fold_x.rs", "range_bound_fold_ev", 1),
    ("fold_self_bits", "crates/elaborate/src/const_bound.rs", "wide_folds_reading", 1),
    ("fold_self_bits", "crates/elaborate/src/const_fn.rs", "const_placement_wide", 1),
    ("fold_self_bits", "crates/elaborate/src/const_real.rs", "selfdet_truth_reading_str", 1),
    ("fold_self_bits", "crates/elaborate/src/const_wide.rs", "fold_concat_parts", 1),
    ("fold_self_bits", "crates/elaborate/src/const_wide.rs", "fold_count", 1),
    ("fold_self_bits", "crates/elaborate/src/const_wide.rs", "fold_init", 1),
    ("fold_self_bits", "crates/elaborate/src/const_wide.rs", "fold_shift_count", 1),
    ("fold_self_bits", "crates/elaborate/src/const_wide.rs", "override_bits", 2),
    ("fold_self_bits", "crates/elaborate/src/param_query.rs", "override_elem_bits", 1),
    ("fold_self_bits", "crates/elaborate/src/value/lanes.rs", "fold_self_wide", 1),
    ("fold_self_wide", "crates/elaborate/src/const_bound.rs", "count_value_for_msg", 1),
    ("fold_self_wide", "crates/elaborate/src/const_bound.rs", "select_or_count_reason", 1),
    ("fold_self_wide", "crates/elaborate/src/const_real.rs", "const_selfdet_real", 1),
    ("fold_self_wide", "crates/elaborate/src/const_wide.rs", "selfdet_bits_i64", 1),
    ("fold_self_wide", "crates/elaborate/src/const_wide.rs", "selfdet_bits_unsigned", 1),
    ("fold_self_wide", "crates/elaborate/src/const_wide.rs", "selfdet_clog2_wide", 1),
    ("fold_self_wide", "crates/elaborate/src/const_wide.rs", "selfdet_truth", 1),
    ("fold_self_wide", "crates/elaborate/src/const_wide.rs", "wide_domain_folds", 1),
    ("fold_self_wide", "crates/elaborate/src/const_wide.rs", "wide_domain_is_zero", 1),
    ("fold_self_wide", "crates/elaborate/src/const_wide.rs", "wide_selfdet_width", 1),
    ("fold_self_wide", "crates/elaborate/src/edge_gate.rs", "wide_domain_has_unknown", 1),
    ("fold_selfdet_operand", "crates/elaborate/src/const_wide.rs", "fold_region", 5),
    ("param_decl_fold", "crates/elaborate/src/generate.rs", "lower_gen_module_item", 1),
    ("param_decl_fold", "crates/elaborate/src/instance.rs", "elaborate_instance", 1),
    ("param_decl_fold", "crates/elaborate/src/package.rs", "elaborate_package", 1),
    ("param_decl_fold", "crates/elaborate/src/params.rs", "bind_one_param", 1),
];

/// E3: the binder chain's members, in `param_decl_fold`'s order.
const E3_MEMBERS: [&str; 5] = [
    "untyped_fill_init",
    "param_init_at_declared_width",
    "eval_param_init",
    "param_value_via_real",
    "param_i64_at_declared",
];

/// E3: (member, file, fn, calls, reason).
#[rustfmt::skip]
const E3: &[(&str, &str, &str, usize, &str)] = &[
    ("eval_param_init", "crates/elaborate/src/value/chains.rs", "param_decl_fold", 1, "the chain, step 3"),
    ("param_i64_at_declared", "crates/elaborate/src/params.rs", "param_init_at_declared_width", 1, "step 2's own body: the declared-width read-back of a name / a wide operand"),
    ("param_i64_at_declared", "crates/elaborate/src/value/chains.rs", "param_decl_fold", 1, "the chain, step 5"),
    ("param_init_at_declared_width", "crates/elaborate/src/value/chains.rs", "param_decl_fold", 1, "the chain, step 2"),
    ("param_value_via_real", "crates/elaborate/src/value/chains.rs", "param_decl_fold", 1, "the chain, step 4"),
    ("untyped_fill_init", "crates/elaborate/src/params.rs", "param_decl_width_opt", 1, "another question: the declared width's fill meta (param_decl_width_opt, default_binds)"),
    ("untyped_fill_init", "crates/elaborate/src/value/chains.rs", "param_decl_fold", 1, "the chain, step 1"),
];

/// E4: (file, fn, hits).
#[rustfmt::skip]
const E4: &[(&str, &str, usize)] = &[
    ("crates/elaborate/src/value/lanes.rs", "fold_self_wide", 1),
];

/// E5: (file, fn, tokens, what it is).
#[rustfmt::skip]
const E5: &[(&str, &str, usize, &str)] = &[
    ("crates/elaborate/src/api.rs", "elaborate_located_params", 1, "read: any error so far"),
    ("crates/elaborate/src/driver.rs", "count_error_capped", 1, "write: an error was reported"),
    ("crates/elaborate/src/driver.rs", "new", 1, "write: false at construction"),
    ("crates/elaborate/src/edge_gate.rs", "edge_backstop", 1, "read: any error so far"),
    ("crates/elaborate/src/edge_gate.rs", "zero_width_claim_is_sound", 1, "read: any error so far"),
    ("crates/elaborate/src/lib.rs", "-", 1, "the field"),
    ("crates/elaborate/src/read_set.rs", "census_sys_func", 1, "read: any error so far (debug_assertions only)"),
    ("crates/elaborate/src/read_set.rs", "census_sys_task", 1, "read: any error so far (debug_assertions only)"),
    ("crates/elaborate/src/var_init.rs", "assert_block_local_inits_drained", 1, "read: any error so far"),
];

/// E6: (file, fn, hits). BIND-2a: a constant-function local's declined width is read as
/// width 0 in one place, the frame's writer `ConstFrame::bind_shape` (`bound/frame.rs`) ;
/// `bind_const_decl` and `eval_const_call_at` (`const_fn.rs`) hand it the `Option` and no
/// longer spell the reading.
#[rustfmt::skip]
const E6: &[(&str, &str, usize)] = &[
    ("crates/elaborate/src/bound/frame.rs", "bind_shape", 1),
    ("crates/elaborate/src/inline_fold.rs", "fold_straight_line", 1),
];

/// E7's callees: the range router, its check and its parts.
const E7_CALLEES: [&str; 5] = [
    "const_range_bound_fold",
    "check_const_range_bound",
    "range_bounds_checked",
    "range_bound_fold_ev",
    "bound_step",
];

/// E7: (callee, file, fn, calls, reason).
#[rustfmt::skip]
const E7: &[(&str, &str, &str, usize, &str)] = &[
    ("bound_step", "crates/elaborate/src/const_fn_width/fold_x.rs", "range_bound_fold_ev", 1, "path B's value rule"),
    ("check_const_range_bound", "crates/elaborate/src/array_geom.rs", "array_dim_extents", 1, "Dim::Size `[N]`: IN + check, outside the chain (head note item 5)"),
    ("check_const_range_bound", "crates/elaborate/src/value/chains.rs", "range_bounds_checked", 2, "the chain"),
    ("const_range_bound_fold", "crates/elaborate/src/array_geom.rs", "record_declared_bounds_for", 1, "msb only, no check (a .velab writer)"),
    ("const_range_bound_fold", "crates/elaborate/src/const_select.rs", "range_facts_if", 2, "RangeFacts::lazy"),
    ("const_range_bound_fold", "crates/elaborate/src/const_select.rs", "range_facts_pair", 2, "RangeFacts::pair"),
    ("const_range_bound_fold", "crates/elaborate/src/expr_special.rs", "bits_of_selfdet", 3, "a part-select's bounds, no check"),
    ("const_range_bound_fold", "crates/elaborate/src/netdecl.rs", "elaborate_netvar_decl_inner", 1, "msb only, no check (a .velab writer)"),
    ("const_range_bound_fold", "crates/elaborate/src/value/chains.rs", "range_bounds_checked", 2, "the chain"),
    ("range_bound_fold_ev", "crates/elaborate/src/const_fn_width/fold_x.rs", "bound_x_reason", 1, "route E: the evidence re-run"),
    ("range_bound_fold_ev", "crates/elaborate/src/const_select.rs", "const_range_bound_fold_at", 1, "the value route"),
    ("range_bounds_checked", "crates/elaborate/src/array_geom.rs", "array_dim_extents", 1, "S3 an unpacked `[m:l]`"),
    ("range_bounds_checked", "crates/elaborate/src/array_geom.rs", "range_to_dims_opt", 1, "S4"),
    ("range_bounds_checked", "crates/elaborate/src/packed.rs", "packed_extents", 1, "S1 each packed dim"),
    ("range_bounds_checked", "crates/elaborate/src/params.rs", "check_param_decl_range", 1, "S2"),
];

/// E7: the body of `range_bounds_checked`, call by call: (callee, the statement's tokens before
/// the callee, argument tokens).
const E7_CHAIN: [(&str, &str, &str); 4] = [
    ("const_range_bound_fold", "let m = self .", "& r . msb"),
    ("const_range_bound_fold", "let l = self .", "& r . lsb"),
    ("check_const_range_bound", "self .", "& r . msb , m"),
    ("check_const_range_bound", "self .", "& r . lsb , l"),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Id,
    Num,
    Lit,
    P,
}

#[derive(Clone, Debug)]
struct Tok {
    kind: Kind,
    text: String,
    joint: bool,
    key: String,
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

/// Every `crates/elaborate/src/**/*.rs`, relative to the workspace root, sorted.
fn src_files(root: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    rs_files(&root.join(SRC), &mut files);
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

fn push(out: &mut Vec<Tok>, kind: Kind, text: String, joint: bool, key: &str) {
    out.push(Tok {
        kind,
        text,
        joint,
        key: key.to_string(),
    });
}

fn walk(ts: TokenStream, cur: &str, out: &mut Vec<Tok>) {
    let tt: Vec<TokenTree> = ts.into_iter().collect();
    let mut pending_fn: Option<String> = None;
    let mut i = 0;
    while i < tt.len() {
        match &tt[i] {
            TokenTree::Punct(p) if p.as_char() == '#' => {
                let mut j = i + 1;
                if matches!(tt.get(j), Some(TokenTree::Punct(q)) if q.as_char() == '!') {
                    j += 1;
                }
                if let Some(TokenTree::Group(g)) = tt.get(j) {
                    if g.delimiter() == Delimiter::Bracket {
                        i = j + 1;
                        if is_cfg_test(&g.stream()) {
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
                push(out, Kind::P, "#".into(), p.spacing() == Spacing::Joint, cur);
                i += 1;
            }
            TokenTree::Ident(id) if id == "fn" => {
                push(out, Kind::Id, "fn".into(), false, cur);
                if let Some(TokenTree::Ident(name)) = tt.get(i + 1) {
                    pending_fn = Some(name.to_string());
                    push(out, Kind::Id, name.to_string(), false, cur);
                    i += 2;
                    continue;
                }
                i += 1;
            }
            TokenTree::Punct(p) if p.as_char() == ';' => {
                pending_fn = None;
                push(out, Kind::P, ";".into(), false, cur);
                i += 1;
            }
            TokenTree::Group(g) => {
                let mut key = cur.to_string();
                if g.delimiter() == Delimiter::Brace {
                    if let Some(f) = pending_fn.take() {
                        key = f;
                    }
                }
                let (o, c) = match g.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::None => ("", ""),
                };
                if !o.is_empty() {
                    push(out, Kind::P, o.into(), false, &key);
                }
                walk(g.stream(), &key, out);
                if !c.is_empty() {
                    push(out, Kind::P, c.into(), false, &key);
                }
                i += 1;
            }
            TokenTree::Ident(id) => {
                push(out, Kind::Id, id.to_string(), false, cur);
                i += 1;
            }
            TokenTree::Punct(p) => {
                let j = p.spacing() == Spacing::Joint;
                push(out, Kind::P, p.as_char().to_string(), j, cur);
                i += 1;
            }
            TokenTree::Literal(l) => {
                let s = l.to_string();
                let kind = if s.starts_with(|c: char| c.is_ascii_digit()) {
                    Kind::Num
                } else {
                    Kind::Lit
                };
                push(out, kind, s, false, cur);
                i += 1;
            }
        }
    }
}

/// The pairs a `Joint` punct joins with the next punct.
const PAIRS: [&str; 4] = ["=>", "==", "!=", "::"];

fn tokens(src: &str) -> Vec<Tok> {
    let ts = TokenStream::from_str(src).unwrap_or_else(|e| panic!("tokenize: {e:?}"));
    let mut seq = Vec::new();
    walk(ts, "-", &mut seq);
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

/// The index after the `)` closing the `(` at `i`.
fn after_parens(t: &[Tok], i: usize) -> usize {
    let mut d = 0usize;
    for (j, x) in t.iter().enumerate().skip(i) {
        if x.kind == Kind::P && x.text == "(" {
            d += 1;
        } else if x.kind == Kind::P && x.text == ")" {
            d -= 1;
            if d == 0 {
                return j + 1;
            }
        }
    }
    t.len()
}

/// The index of the innermost `(` enclosing the token at `i` (parentheses only).
fn enclosing_paren(t: &[Tok], i: usize) -> Option<usize> {
    let mut d = 0usize;
    for j in (0..i).rev() {
        if t[j].kind != Kind::P {
            continue;
        }
        match t[j].text.as_str() {
            ")" => d += 1,
            "(" if d == 0 => return Some(j),
            "(" => d -= 1,
            _ => {}
        }
    }
    None
}

/// One hit: (pin, callee or name, key) ; the file is added by the caller.
type Hit = (&'static str, String, String);

/// Every E1-E7 hit of one source, plus E3's order / laziness violations in `param_decl_fold`
/// and E7's order / argument violations in `range_bounds_checked`.
fn hits(src: &str) -> (Vec<Hit>, Vec<String>) {
    let t = tokens(src);
    let tx = |i: usize| t.get(i).map_or("", |x| x.text.as_str());
    let entry_names: Vec<&str> = ENTRIES.iter().map(|(n, _)| *n).collect();
    let mut out: Vec<Hit> = Vec::new();
    // E3 positions inside param_decl_fold: (member index, token index).
    let mut chain: Vec<(usize, usize)> = Vec::new();
    let mut bad: Vec<String> = Vec::new();
    // E7 calls inside range_bounds_checked: (callee, statement prefix, argument tokens).
    let mut rbc: Vec<(String, String, String)> = Vec::new();
    for i in 0..t.len() {
        let v = tx(i);
        let key = t[i].key.clone();
        let call = tx(i + 1) == "(" && (i == 0 || tx(i - 1) != "fn");
        if v == "fn" && entry_names.contains(&tx(i + 1)) {
            out.push(("E1", tx(i + 1).to_string(), tx(i + 1).to_string()));
        }
        if call && E2_CALLEES.contains(&v) {
            out.push(("E2", v.to_string(), key.clone()));
        }
        if call {
            if let Some(m) = E3_MEMBERS.iter().position(|x| *x == v) {
                out.push(("E3", v.to_string(), key.clone()));
                if key == "param_decl_fold" {
                    chain.push((m, i));
                }
            }
        }
        if key == "param_decl_fold" && v == "then_some" {
            bad.push("`then_some` in param_decl_fold: an eager step 1".to_string());
        }
        if call && v == "fold_self_bits" {
            let end = after_parens(&t, i + 1);
            if (i + 2..end).any(|j| tx(j) == "wide_name_bits") {
                out.push(("E4", v.to_string(), key.clone()));
            }
        }
        if v == "had_error" {
            out.push(("E5", v.to_string(), key.clone()));
        }
        if call && E7_CALLEES.contains(&v) {
            out.push(("E7", v.to_string(), key.clone()));
            if key == "range_bounds_checked" {
                let end = after_parens(&t, i + 1);
                let args: Vec<&str> = (i + 2..end.saturating_sub(1)).map(tx).collect();
                let mut s0 = i;
                while s0 > 0 && tx(s0 - 1) != ";" && tx(s0 - 1) != "{" {
                    s0 -= 1;
                }
                let prefix: Vec<&str> = (s0..i).map(tx).collect();
                rbc.push((v.to_string(), prefix.join(" "), args.join(" ")));
            }
        }
        let e6 = ["unwrap_or", "(", "(", "0", ",", "false", ")", ")"];
        if (0..e6.len()).all(|k| tx(i + k) == e6[k]) {
            out.push(("E6", v.to_string(), key.clone()));
        }
    }
    if !chain.is_empty() {
        let order: Vec<usize> = chain.iter().map(|(m, _)| *m).collect();
        if order != [0, 1, 2, 3, 4] {
            bad.push(format!(
                "param_decl_fold's members out of order or not one each: {order:?}"
            ));
        }
        for (m, i) in &chain {
            let o = enclosing_paren(&t, *i);
            let lazy = o.is_some_and(|o| {
                let head = tx(o + 1) == "|" && tx(o + 2) == "|";
                if *m == 0 {
                    head && o >= 3 && tx(o - 1) == "then" && tx(o - 2) == "." && tx(o - 3) == "fill"
                } else {
                    head && o >= 1 && tx(o - 1) == "or_else"
                }
            });
            if !lazy {
                bad.push(format!(
                    "step {} `{}` is not inside its lazy closure",
                    m + 1,
                    E3_MEMBERS[*m]
                ));
            }
        }
    }
    if !rbc.is_empty() {
        let want: Vec<(String, String, String)> = E7_CHAIN
            .iter()
            .map(|(c, p, a)| (c.to_string(), p.to_string(), a.to_string()))
            .collect();
        if rbc != want {
            bad.push(format!(
                "range_bounds_checked is not PRE's fold msb, fold lsb, check msb, check lsb: {rbc:?}"
            ));
        }
    }
    (out, bad)
}

type Table = BTreeMap<(String, String, String), usize>;

/// (pin -> (name, file, key) -> count) over the tree, and E3's violations.
fn census(root: &Path) -> (BTreeMap<&'static str, Table>, Vec<String>) {
    let mut got: BTreeMap<&'static str, Table> = BTreeMap::new();
    let mut bad = Vec::new();
    for (rel, src) in src_files(root) {
        let (h, b) = hits(&src);
        for (pin, name, key) in h {
            *got.entry(pin)
                .or_default()
                .entry((name, rel.clone(), key))
                .or_insert(0) += 1;
        }
        bad.extend(b.into_iter().map(|x| format!("{rel}: {x}")));
    }
    (got, bad)
}

fn table<'a>(rows: impl Iterator<Item = (&'a str, &'a str, &'a str, usize)>) -> Table {
    rows.map(|(n, f, k, c)| ((n.to_string(), f.to_string(), k.to_string()), c))
        .collect()
}

#[test]
fn value_entry_shapes_live_in_the_home() {
    let (got, bad) = census(&workspace_root());
    let get = |p: &str| got.get(p).cloned().unwrap_or_default();
    let want_e1 = table(ENTRIES.iter().map(|(n, f)| (*n, *f, *n, 1)));
    let want_e2 = table(E2.iter().map(|(n, f, k, c)| (*n, *f, *k, *c)));
    let want_e3 = table(E3.iter().map(|(n, f, k, c, _)| (*n, *f, *k, *c)));
    let want_e4 = table(E4.iter().map(|(f, k, c)| ("fold_self_bits", *f, *k, *c)));
    let want_e5 = table(E5.iter().map(|(f, k, c, _)| ("had_error", *f, *k, *c)));
    let want_e6 = table(E6.iter().map(|(f, k, c)| ("unwrap_or", *f, *k, *c)));
    let want_e7 = table(E7.iter().map(|(n, f, k, c, _)| (*n, *f, *k, *c)));
    assert_eq!(
        get("E1"),
        want_e1,
        "E1: an entry defined again or moved: {NEW_ROW}"
    );
    assert_eq!(
        get("E2"),
        want_e2,
        "E2: a new caller of a core or frame entry: {NEW_ROW}"
    );
    assert_eq!(
        get("E3"),
        want_e3,
        "E3: the binder chain is spelled once, in {CHAINS}"
    );
    assert!(bad.is_empty(), "E3 / E7: {bad:?}");
    assert_eq!(
        get("E4"),
        want_e4,
        "E4: REG asked in the live scope is value/lanes.rs's fold_self_wide"
    );
    assert_eq!(
        get("E5"),
        want_e5,
        "E5: a new `had_error` reader: say which question it asks"
    );
    assert_eq!(
        get("E6"),
        want_e6,
        "E6: a new width-0 reading of a declined width: {NEW_ROW}"
    );
    assert_eq!(get("E7"), want_e7, "E7: {E7_MSG}");
}

#[test]
fn walker_controls() {
    let count = |pin: &str, src: &str| hits(src).0.iter().filter(|(p, _, _)| *p == pin).count();
    let bad = |src: &str| hits(src).1;
    // E1: a definition counts ; a call, a string, a comment do not.
    assert_eq!(
        count(
            "E1",
            "fn fold_ctx() {} fn g() { self.fold_ctx(e); let s = \"fn fold_ctx\"; } // fn fold_ctx() {}"
        ),
        1
    );
    // E2: a multi-line method call and a path call count ; a definition, a string, a longer name
    // (`fold_self_bits` is not `fold_self`) do not.
    assert_eq!(
        count(
            "E2",
            "fn f() { self\n .fold_ctx(\n e, env, w, d, 8, false); Self::fold_ctx(e); fold_self_bits(e, n); let s = \"fold_ctx(\"; } fn fold_ctx() {}"
        ),
        3
    );
    assert_eq!(count("E2", "fn f() { fold_self_bits(e, n); }"), 1);
    assert_eq!(count("E2", "fn f() { self.fold_selfish(e); }"), 0);
    // E3: the home's shape is clean ; an eager step 1, a step outside its closure, a swapped
    // order are each refused.
    let home = "fn param_decl_fold(&self, p: &P, meta: M, fill: bool) -> Option<i64> { fill.then(|| self.untyped_fill_init(p)).flatten().map(|(v, _)| v).or_else(|| self.param_init_at_declared_width(p)).or_else(|| self.eval_param_init(&p.value, meta)).or_else(|| self.param_value_via_real(p, meta)).or_else(|| { let dm = self.param_decl_width_declared(p); self.param_i64_at_declared(&p.value, dm) }) }";
    assert!(bad(home).is_empty(), "{:?}", bad(home));
    assert_eq!(count("E3", home), 5);
    let eager = home.replace(
        "fill.then(|| self.untyped_fill_init(p)).flatten()",
        "fill.then_some(self.untyped_fill_init(p)).flatten()",
    );
    assert_eq!(bad(&eager).len(), 2, "{:?}", bad(&eager));
    let unclosed = home.replace(
        ".or_else(|| self.eval_param_init(&p.value, meta))",
        ".or(self.eval_param_init(&p.value, meta))",
    );
    assert_eq!(bad(&unclosed).len(), 1, "{:?}", bad(&unclosed));
    let swapped = home
        .replace("self.param_init_at_declared_width(p)", "self.XX(p)")
        .replace(
            "self.eval_param_init(&p.value, meta)",
            "self.param_init_at_declared_width(p)",
        )
        .replace("self.XX(p)", "self.eval_param_init(&p.value, meta)");
    assert_eq!(bad(&swapped).len(), 1, "{:?}", bad(&swapped));
    // the order check reads only param_decl_fold: a binder's own inline chain is counted by E3's
    // table, not refused here.
    assert!(bad("fn bind() { let x = self.eval_param_init(&p.value, meta); }").is_empty());
    // E4: the live-scope closure counts ; edge_gate.rs's `wide_name_bits` outside a
    // fold_self_bits call (premise C9), another resolver and the forwarding call do not.
    assert_eq!(
        count(
            "E4",
            "fn a(&self) { fold_self_bits(e, &|n, _| self.wide_name_bits(n)); }"
        ),
        1
    );
    assert_eq!(
        count(
            "E4",
            "fn b(&self) { unknown(self.fold_self_wide(e)) || Self::first_in(e, &mut |x| unknown(self.wide_name_bits(x)).then_some(())).is_some(); fold_self_bits(e, &|n, _| self.param_leaf_bits(n)); fold_bits_at(e, 0, &|n, _| self.wide_name_bits(n)); }"
        ),
        0
    );
    // E5: a field and a read count ; a comment and a string do not.
    assert_eq!(
        count(
            "E5",
            "struct E { had_error: bool } fn f(&self) { if self.had_error {} let s = \"had_error\"; } // had_error"
        ),
        2
    );
    // E6: only `(0, false)`.
    assert_eq!(
        count(
            "E6",
            "fn f() { m.unwrap_or((0, false)); m.unwrap_or((0, true)); m.unwrap_or((1, false)); m.unwrap_or_default(); }"
        ),
        1
    );
    // `#[cfg(test)]` items are skipped ; `#[cfg(not(test))]` items are read.
    assert_eq!(
        count(
            "E5",
            "#[cfg(test)] mod tests { fn t(&self) { self.had_error; } } #[cfg(not(test))] fn k(&self) { self.had_error; }"
        ),
        1
    );
    // E7: a call counts per callee, `const_range_bound_fold_at` is not `const_range_bound_fold`, a
    // definition does not count ; the home's body is clean ; a swap, a crossed value, a wrong
    // bound, a dropped check and a doubled fold are each refused ; outside the home nothing is
    // ordered (E7's table counts it) ; a lazy lsb fold is refused (PRE's eager pair).
    assert_eq!(
        count(
            "E7",
            "fn f(&self) { self.const_range_bound_fold(&r.msb); self.const_range_bound_fold_at(e); hdl_ast::range_rules::bound_step(v1); } fn const_range_bound_fold(&self) {}"
        ),
        2
    );
    let rbc = "fn range_bounds_checked(&mut self, r: &ast::Range) -> (Option<i64>, Option<i64>) { let m = self.const_range_bound_fold(&r.msb); let l = self.const_range_bound_fold(&r.lsb); self.check_const_range_bound(&r.msb, m); self.check_const_range_bound(&r.lsb, l); (m, l) }";
    assert!(bad(rbc).is_empty(), "{:?}", bad(rbc));
    assert_eq!(count("E7", rbc), 4);
    let swapped = rbc.replace(
        "self.check_const_range_bound(&r.msb, m); self.check_const_range_bound(&r.lsb, l);",
        "self.check_const_range_bound(&r.lsb, l); self.check_const_range_bound(&r.msb, m);",
    );
    assert_eq!(bad(&swapped).len(), 1, "{:?}", bad(&swapped));
    let crossed = rbc.replace(
        "self.check_const_range_bound(&r.msb, m); self.check_const_range_bound(&r.lsb, l);",
        "self.check_const_range_bound(&r.msb, l); self.check_const_range_bound(&r.lsb, m);",
    );
    assert_eq!(bad(&crossed).len(), 1, "{:?}", bad(&crossed));
    let wrong_bound = rbc.replace(
        "let l = self.const_range_bound_fold(&r.lsb);",
        "let l = self.const_range_bound_fold(&r.msb);",
    );
    assert_eq!(bad(&wrong_bound).len(), 1, "{:?}", bad(&wrong_bound));
    let dropped = rbc.replace(" self.check_const_range_bound(&r.lsb, l);", "");
    assert_eq!(bad(&dropped).len(), 1, "{:?}", bad(&dropped));
    let lazy = rbc.replace(
        "let l = self.const_range_bound_fold(&r.lsb);",
        "let l = m.and_then(|_| self.const_range_bound_fold(&r.lsb));",
    );
    assert_eq!(bad(&lazy).len(), 1, "{:?}", bad(&lazy));
    let doubled = rbc.replace(
        "let m = self.const_range_bound_fold(&r.msb);",
        "let m = self.const_range_bound_fold(&r.msb); let _ = self.const_range_bound_fold(&r.msb);",
    );
    assert_eq!(bad(&doubled).len(), 1, "{:?}", bad(&doubled));
    assert!(bad("fn packed_extents(&mut self) { let m = self.const_range_bound_fold(&r.lsb); self.check_const_range_bound(&r.msb, m); }").is_empty());
    // Keys: the innermost enclosing fn.
    let keys: Vec<Hit> = hits("fn outer() { fn inner(&self) { self.had_error; } }").0;
    assert_eq!(
        keys,
        vec![("E5", "had_error".to_string(), "inner".to_string())]
    );
}
