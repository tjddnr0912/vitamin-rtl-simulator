//! System-function name facts: every fact of a system FUNCTION that its `$name` alone
//! decides, in one home.
//!
//! History (owner, 2026-11-09, ROADMAP §5.2 SYS-1, its first half SYS-1a): the name lists
//! that elaborate's constant domain, its edge gate, its hoisters and its replication-count
//! walk each spelled at their own site, the parser's copy of the dimension-query list and the
//! child walk's does-not-evaluate list were collected into this one file, so a new `$name`
//! is one entry here and an edit to one list is reviewed beside its siblings (ER §5.1). The
//! INVERSE facts, the id a name lowers to and the name an id prints, live beside
//! `SysFuncId` in sim-ir `names.rs` (`sysfunc_id` / `sysfunc_name`), where the id type lives
//! and the engine reads it. STRUCTURE ONLY: each column below is its PRE set copied
//! verbatim, in PRE's order and with PRE's comment, so moving it moved no value; the only
//! merge is the dimension-query list, whose two byte-equal copies (elaborate and the
//! parser) asked one question.
//!
//! The rule: IN is a property of the FUNCTION keyed on its name alone (membership in a
//! set a consumer asks about), spelled as a fact fn of the name. The consumer's own logic
//! (the arity test, the recursion into the arguments, an evaluation, the dynamic-handle
//! test, the complement) stays at the consumer, in PRE's operand order, so no column can
//! recurse, evaluate or cost (each is a `matches!` over a `&str`).
//!
//! | column | PRE set (n) | PRE site | asked by |
//! |---|---|---|---|
//! | [`folds_integer`] | `$clog2 $bits $rtoi` (3) | elaborate `const_fn_width.rs` `sys_fn_is_integer` | `const_fn_width.rs`, `params.rs`, `param_query.rs`, `const_eval.rs` |
//! | [`is_array_query_function`] | `$size $left $right $low $high $increment $dimensions $unpacked_dimensions` (8) | elaborate `const_array.rs` `is_dim_query_name` = hdl-parser `packed_md.rs` twin | `const_bound.rs`, `const_fn.rs`, `const_eval.rs`, hdl-parser `packed_md.rs` |
//! | [`width_neutral`] | `$clog2 $signed $unsigned` (3) | elaborate `edge_gate.rs` `is_other_sys_call` | `edge_gate.rs` (its complement) |
//! | [`real_conversion`] | `$clog2 $rtoi` (2) | elaborate `param_query.rs` `real_conversion_is_folded` | same |
//! | [`exact_i64`] | `$clog2` (1) | elaborate `expr_size_hier_exact.rs` `expr_exact` | same |
//! | [`inert`] | 20 names | elaborate `hoist/sformatf.rs` `expr_is_inert` | same |
//! | [`does_not_evaluate`] | 10 names | hdl-ast `walk.rs` `syscall_does_not_evaluate` | `walk.rs` (`ChildPos::Unevaluated`) |
//! | [`type_query`] | the 10 above + `$isunbounded` (11) | elaborate `ast_query.rs` `count_reads_runtime_net` | same |
//! | [`type_query_runtime_size`] | `$size $high $right $bits` (4) | elaborate `ast_query.rs` `count_reads_runtime_net` | same |
//!
//! The columns are distinct questions and are never merged: `type_query` minus
//! `does_not_evaluate` is `$isunbounded`; `inert` lacks `$unpacked_dimensions` and
//! `$isunbounded`; `folds_integer` minus `real_conversion` is `$bits`; `width_neutral`'s
//! `$clog2` is in no width rule's signing list. `elaborate/tests/sysfn_home.rs`
//! `columns_keep_pre` pins each column against its PRE set and these relations: a value
//! row that changes a column edits both.
//!
//! OUTSIDE this file, by rule (each spells a `$name` for a reason that is not a name fact):
//! * a name test inside a member of ANOTHER home, which moves with that home: the constant
//!   domain's fold cores and arms (VAL-1: elaborate `const_eval_in_scope`, `eval_const_env`,
//!   `fold_ctx`, `fold_region`, `env_fold`, `const_dim_query`), the lowering's §20.7
//!   introspection fold (`try_introspect_fold`, hand-off H1), the width / sign / kind
//!   walkers (TYPE-1: `ctx_signed_impl`, `size_ctx_self_width`, `bits_of_selfdet`,
//!   `expr_is_string_ast`), the §20.7 values (`range_rules.rs` `dim_query`, `md_query`) and
//!   the parser's parse-time fold and gate (PARSE-1: `expr_primary.rs`,
//!   `rewrite_packed_md_dim_query`, `names_an_overridable`);
//! * a DISPATCH KEY: a guard that selects the routine, fold or refusal written for that one
//!   name (the literal and the routine or message are one unit);
//! * a special form's SIGNATURE table (name -> id + arity), a producer that builds a
//!   `SysCall` node (hdl-parser `struct_sel.rs`), task lists and task labels;
//! * diagnostic text and minted names;
//! * the special-form facts (which arguments a call writes or reads, the direct-rhs-only
//!   calls, the statement route, the SVA sampled-value list): SYS-1b's, until it lands.
//!
//! ENFORCED by `crates/elaborate/tests/sysfn_home.rs`, over every crate's `src`: a string
//! literal whose whole value is a name spelled in this file or in sim-ir `names.rs`'s
//! `sysfunc_name` / `sysfunc_id` bodies is a hit, comments, attributes (doc comments
//! included) and `#[cfg(test)]` items skipped; each hit outside the two homes is an
//! EXCEPTION keyed by (file, enclosing fn) with its count exact and its reason there.
//!
//! REVIEW-ONLY (no test sees them): a prefix test (`starts_with("$dist_")`,
//! `starts_with("$f")`), a name built by `format!` or held in a const or a variable, a new
//! name in no column spelled only outside the homes, and a set over `SysFuncId` (ids, not
//! names).

/// Does this system function return a 32-bit SIGNED INTEGER *and* fold in the
/// constant domain?
///
/// An explicit list, never a blanket `SysCall`: the width rule
/// (`const_self_width`'s `Some(32)` and `const_signed_env`'s `true`) is a claim about
/// INTEGER-returning calls only, and a real-returning one (`$itor`, `$bitstoreal`,
/// `$realtime`) would be sized wrong by it the day the const domain learns to fold it.
/// These three are exactly the ones elaborate's `const_fn.rs` folds into this domain —
/// `$clog2` (`:421`), `$rtoi` (`:431`), `$bits` (`:452`). The dimension-query family
/// (`$size`/`$high`/…) is integer-returning too but is NOT in this list, so a
/// consumer that also needs the family must spell `|| is_array_query_function(..)` —
/// `const_eval_in_scope` DOES fold the family (`const_fn.rs:437`), which is why
/// `const_expr_signed` (`const_eval.rs`) asks both lists. Widening this list
/// itself is a different (loud→value) move for the consumers that gate on it —
/// `ctx_width_names_are_evident` and `param_decl_width_opt` — and belongs to its
/// own row.
#[inline]
pub fn folds_integer(name: &str) -> bool {
    matches!(name, "$clog2" | "$bits" | "$rtoi")
}

/// The §20.7 array query functions (IEEE 1800-2017 A.8.2 `array_query_function`: the
/// `array_dimension_function`s and the `array_dimensions_function`s): the names
/// elaborate's `const_dim_query` answers — shared with the diagnostic that explains a
/// declined fold, so the two lists cannot drift — and the names the parser's
/// `rewrite_packed_md_dim_query` answers over a multi-dimensional packed parameter. One
/// list for both crates (the parser's twin said "the two crates cannot share a list";
/// both reach hdl-ast).
#[inline]
pub fn is_array_query_function(name: &str) -> bool {
    matches!(
        name,
        "$size"
            | "$left"
            | "$right"
            | "$low"
            | "$high"
            | "$increment"
            | "$dimensions"
            | "$unpacked_dimensions"
    )
}

/// The system functions elaborate's edge gate (`edge_gate.rs` `width_leaves_bind_here`)
/// descends through: every OTHER system call stops the descent (its complement,
/// `is_other_sys_call`, is the gate's question).
#[inline]
pub fn width_neutral(name: &str) -> bool {
    matches!(name, "$clog2" | "$signed" | "$unsigned")
}

/// The system functions `real_conversion_is_folded` (elaborate `param_query.rs`) accepts
/// as a real-to-integer boundary, before its own arity test and its fold (`args.len() == 1`
/// first, `const_eval_in_scope` last). A `false` is not a proved conversion (stays loud).
#[inline]
pub fn real_conversion(name: &str) -> bool {
    matches!(name, "$clog2" | "$rtoi")
}

/// The system functions `expr_exact` (elaborate `expr_size_hier_exact.rs`) recurses into
/// as exact in i64, with its own arity test (`args.len() == 1`). PRE spelled it
/// `name.name == "$clog2"`.
#[inline]
pub fn exact_i64(name: &str) -> bool {
    matches!(name, "$clog2")
}

/// `$sformatf` itself renders and mutates nothing; a QUERY sysfunc is a pure
/// read. Everything else — `$random`/`$urandom`/`$fgetc`/… and every USER
/// call (which may carry an output formal) — is assumed to have an effect.
///
/// Asked by `expr_is_inert` (elaborate `hoist/sformatf.rs`), which also requires every
/// argument inert.
#[inline]
pub fn inert(name: &str) -> bool {
    matches!(
        name,
        "$sformatf"
            | "$time"
            | "$stime"
            | "$realtime"
            | "$bits"
            | "$clog2"
            | "$signed"
            | "$unsigned"
            | "$size"
            | "$left"
            | "$right"
            | "$high"
            | "$low"
            | "$dimensions"
            | "$increment"
            | "$typename"
            | "$countones"
            | "$onehot"
            | "$onehot0"
            | "$isunknown"
    )
}

/// A system FUNCTION that does not EVALUATE its operand: it reports a static property
/// of the operand's type (IEEE 1800 §20.5 `$bits`, §20.6 array queries). Its arguments
/// are [`crate::walk::ChildPos::Unevaluated`], so nothing in them is read at run time, and
/// moving a side effect out of one would perform an effect the source never performs.
///
/// Measured: iverilog leaves the side effect unperformed for `$bits`; `$clog2` and
/// `$isunknown` DO evaluate their operand and are deliberately absent.
#[inline]
pub fn does_not_evaluate(name: &str) -> bool {
    matches!(
        name,
        "$bits"
            | "$size"
            | "$high"
            | "$low"
            | "$left"
            | "$right"
            | "$increment"
            | "$dimensions"
            | "$unpacked_dimensions"
            | "$typename"
    )
}

/// The type / shape-query system functions `count_reads_runtime_net` (elaborate
/// `ast_query.rs`) reads as ELABORATION CONSTANTS regardless of a net operand (they read
/// the operand's TYPE / shape, not its runtime value). A `false` recurses into the
/// arguments.
#[inline]
pub fn type_query(name: &str) -> bool {
    matches!(
        name,
        "$bits"
            | "$size"
            | "$high"
            | "$low"
            | "$left"
            | "$right"
            | "$increment"
            | "$dimensions"
            | "$unpacked_dimensions"
            | "$isunbounded"
            | "$typename"
    )
}

/// The [`type_query`] names that ANSWER the runtime size over a dynamic-storage handle
/// (`try_introspect_fold`'s dyn arm), so `count_reads_runtime_net` reads them as a
/// runtime read when their first argument is such a handle (its own test, after this one).
#[inline]
pub fn type_query_runtime_size(name: &str) -> bool {
    matches!(name, "$size" | "$high" | "$right" | "$bits")
}
