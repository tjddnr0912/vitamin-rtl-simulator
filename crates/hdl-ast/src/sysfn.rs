//! System-function name facts: every fact of a system FUNCTION (and, for argument
//! direction, of the task form of the same name) that its `$name` alone decides, in one home.
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
//! parser) asked one question. Its second half, SYS-1b, moved the special-form facts here
//! the same way (the ten columns after [`type_query_runtime_size`]): which arguments a call
//! writes or reads, which calls advance file state and are evaluated once ahead of the
//! statement, which nested positions refuse them, which bare calls are routed, and the SVA
//! sampled-value list with its edge subset — out of five elaborate files, each read at its
//! PRE site in PRE's operand position.
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
//! | [`is_hoistable_rhs_only_sysfunc`] | `$fgetc $ungetc $fgets $fread $fscanf $sscanf $value$plusargs $fopen` (8) | elaborate `hoist/special.rs` | `hoist/special.rs` `rhs_only_call` |
//! | [`write_arg_range`] | `$fgets $fread` / `$value$plusargs` / `$fscanf $sscanf` ranges, clamped by argc | elaborate `hoist/special.rs` | `hoist/special.rs` `written_roots` |
//! | [`observes_fd_state`] | `$feof` (1) | elaborate `hoist/special.rs` | `hoist/special.rs` `read_roots` |
//! | [`syscall_writes_arg`] | 19 names by position | elaborate `systask.rs` | `da/writes.rs` (`SysCall` and `SysTaskCall` arms) |
//! | [`syscall_read_args`] | 11 names by sub-slice | elaborate `systask.rs` | `da/reads.rs` (`SysCall` and `SysTaskCall` arms) |
//! | [`bare_call_routed`] | 9 names | elaborate `stmt_main.rs` `lower_stmt_inner` (inline) | same |
//! | [`refused_nested_stream_read`] | `$fgetc $ungetc $fgets $fread $fscanf $sscanf` (6) | elaborate `expr_main.rs` `lower_expr_ungated` (inline) | same |
//! | [`refused_nested_seed_or_dest`] | 7 `$dist_*` + `$cast` (8) | elaborate `expr_main.rs` `lower_expr_ungated` (inline) | same |
//! | [`is_sva_sampled_fn`] | `$past $rose $fell $stable $changed $sampled` (6) | elaborate `sva_ast.rs` | `sva_decl.rs` `rewrite_sampled` |
//! | [`sva_lsb_edge`] | `$rose $fell` (2) | elaborate `sva_prop.rs` `sva_prev_for` (inline) | same |
//!
//! The columns are distinct questions and are never merged: `type_query` minus
//! `does_not_evaluate` is `$isunbounded`; `inert` lacks `$unpacked_dimensions` and
//! `$isunbounded`; `folds_integer` minus `real_conversion` is `$bits`; `width_neutral`'s
//! `$clog2` is in no width rule's signing list. Of the special-form columns:
//! [`write_arg_range`] equals [`syscall_writes_arg`] restricted to
//! [`is_hoistable_rhs_only_sysfunc`] and `i < argc` (`syscall_writes_arg` alone covers the
//! string-output tasks, `$cast`, `$random` and the `$dist_*` seed, and is unbounded);
//! [`syscall_read_args`] is its complement except the seed (read AND written) and
//! `$value$plusargs`'s arguments past the second (neither); [`refused_nested_stream_read`] is
//! [`is_hoistable_rhs_only_sysfunc`] minus `$value$plusargs` and `$fopen` (each refused by
//! its own dispatch key); [`bare_call_routed`] is it minus `$fopen` plus `$random` and
//! `$dist_uniform`; [`sva_lsb_edge`] is within [`is_sva_sampled_fn`].
//! `elaborate/tests/sysfn_home.rs` `columns_keep_pre` pins each column against its PRE set
//! and these relations: a value row that changes a column edits both.
//!
//! OUTSIDE this file, by rule (each spells a `$name` for a reason that is not a name fact):
//! * a name test inside a member of ANOTHER home, which moves with that home: the constant
//!   domain's fold cores and arms (VAL-1: elaborate `const_eval_in_scope`, `eval_const_env`,
//!   `fold_ctx`, `fold_region`, `env_fold`, `const_dim_query`), the lowering's §20.7
//!   introspection fold (`try_introspect_fold`, hand-off H1), the width / sign / kind
//!   walkers (TYPE-1: `ctx_signed_impl`, `size_ctx_self_width`, `bits_of_selfdet`,
//!   `expr_is_string_ast`), the §20.7 values (`range_rules.rs` `dim_query`, `md_query`) and
//!   the parser's parse-time fold and gate (PARSE-1: `const_fold.rs`, `expr_primary.rs`,
//!   `rewrite_packed_md_dim_query`, `names_an_overridable`);
//! * a DISPATCH KEY: a guard that selects the routine, fold or refusal written for that one
//!   name (the literal and the routine or message are one unit) — among them elaborate
//!   `expr_main.rs` `lower_expr_ungated`'s seeded `$random`, `$fopen` and `$value$plusargs`
//!   refusals and `sva_decl.rs` `rewrite_sampled`'s per-name rewrite arms;
//! * a special form's dispatch key + minted id: elaborate `crv.rs` `dist_uniform_special`,
//!   `sys_special.rs` `file_read_int_special` and `scanf_special` — each match is the
//!   routine's `_ => return false`, the `SysFuncId` it mints (recorded by site in sim-ir
//!   `names.rs` `NOT_FROM_SYSFUNC_ID`; hdl-ast has no sim-ir edge) and the arity / usage text
//!   its own refusal checks, one unit;
//! * a producer that builds a `SysCall` node (hdl-parser `struct_sel.rs`), task-form lists
//!   and task labels (elaborate `stmt_main.rs`'s display family, its format index and its
//!   `$sformatf`-argument list; `systask.rs` `radix_of_systask`, `map_systask`,
//!   `lower_systask`);
//! * diagnostic text and minted names.
//!
//! ENFORCED by `crates/elaborate/tests/sysfn_home.rs`, over every crate's `src`: a string
//! literal whose whole value is a name spelled in this file or in sim-ir `names.rs`'s
//! `sysfunc_name` / `sysfunc_id` bodies is a hit, comments, attributes (doc comments
//! included) and `#[cfg(test)]` items skipped; each hit outside the two homes is an
//! EXCEPTION keyed by (file, enclosing fn) with its count exact and its reason there.
//!
//! REVIEW-ONLY (no test sees them): a prefix test (`starts_with("$f")`, elaborate
//! `stmt_main.rs`'s display-family format index), a name built by `format!` or held in a
//! const or a variable, a new name in no column spelled only outside the homes, and a set
//! over `SysFuncId` (ids, not names).

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

// ─────────── SYS-1b: the special-form facts (ROADMAP §5.2 SYS-1b) ───────────
//
// Which arguments a call writes or reads, which calls advance file state and are evaluated
// once ahead of the statement, which nested positions refuse them, which bare calls are
// routed, and the SVA sampled-value list with its edge subset. Each column is its PRE body
// copied verbatim (PRE file and fn named on each), read at the PRE site in PRE's operand
// position.

/// The direct-rhs-only file/plusargs system functions the hoist (elaborate
/// `hoist/special.rs`, `rhs_only_call`) hoists — the v7/v9 families whose result is an INT
/// and whose statement form is already implemented in `sys_special.rs`. ONE spelling; it is
/// a SUBSET of the names `expr_main.rs` loud-rejects ([`refused_nested_stream_read`] plus
/// the `$value$plusargs` and `$fopen` dispatch keys), so a name dropped from here simply
/// keeps the old diagnostic (never a silent miss).
///
/// PRE: elaborate `hoist/special.rs` `is_hoistable_rhs_only_sysfunc`.
#[inline]
pub fn is_hoistable_rhs_only_sysfunc(name: &str) -> bool {
    matches!(
        name,
        "$fgetc"
            | "$ungetc"
            | "$fgets"
            | "$fread"
            | "$fscanf"
            | "$sscanf"
            | "$value$plusargs"
            | "$fopen"
    )
}

/// The argument positions a hoistable form WRITES (its ref/destination actuals), as a
/// half-open range over `args`. Hoisting moves that write earlier, so the caller (elaborate
/// `hoist/special.rs` `written_roots`) refuses when one of these roots is also read
/// elsewhere in the statement.
///
/// `$fgetc`/`$ungetc`/`$fopen` write no ARGUMENT — but "writes no argument" is not "has no
/// effect": they all move fd state, and `$feof(fd)` READS it. See `Reads::fd_observer`
/// (elaborate `hoist/special.rs`) and [`observes_fd_state`].
///
/// ⚠️ The destination is argument ZERO for `$fgets(str, fd)` (IEEE 1800 §21.3.4.3) and
/// `$fread(mem, fd[, start[, count]])` (§21.3.4.5) — only `$value$plusargs(fmt, var)` writes
/// argument one. Grouping the three together pointed the overlap gate at the `fd`, which no
/// surviving expression ever holds, so the gate passed everything: measured
/// `n = mem[0] + $fread(mem, fd);` at 73 against iverilog's 15, at exit 0. vita's own
/// recognizers say which it is — `sys_special.rs` lowers `args[0]` and calls
/// `deny_readonly_write(net, "$fgets into" / "$fread into")` on it.
///
/// Every bound is clamped by `argc`, so a short argument list (the arity refusal comes
/// later, in `sys_special.rs`) indexes no argument it does not have. Equal, on the
/// [`is_hoistable_rhs_only_sysfunc`] names, to the positions `i < argc` with
/// [`syscall_writes_arg`]`(name, i)`; the two are distinct questions and never merged.
///
/// PRE: elaborate `hoist/special.rs` `write_arg_range`.
#[inline]
pub fn write_arg_range(name: &str, argc: usize) -> std::ops::Range<usize> {
    match name {
        "$fgets" | "$fread" => 0..1.min(argc),
        "$value$plusargs" => 1.min(argc)..2.min(argc),
        "$fscanf" | "$sscanf" => 2.min(argc)..argc,
        _ => 0..0,
    }
}

/// Does `name` observe fd state (rather than an argument)? `$feof` is the whole list:
/// `ir::sysfunc_id` has no `$ftell`/`$ferror`, and every other fd-touching form is either a
/// task or a member of the hoisted family itself.
///
/// PRE: elaborate `hoist/special.rs` `observes_fd_state` (spelled `name == "$feof"`).
#[inline]
pub fn observes_fd_state(name: &str) -> bool {
    name == "$feof"
}

/// Does system function / task `name` WRITE its `i`-th argument back? The
/// destination-taking family is the complement of [`syscall_read_args`] (one
/// table, two views); on top of it `$random(seed)` and every `$dist_*(seed, …)`
/// write their SEED back (IEEE §20.15.1 / §20.15.2 — the engine's
/// `StmtEffect::SeededRandom` / `SeededDist`), and the seed is READ as well, which
/// is why it is not a read-table entry. `$urandom(seed)` takes its seed as an input
/// (§18.13.1) and writes nothing.
///
/// The consumer is the never-writes walk (elaborate `da/writes.rs`, both its `SysCall` and
/// its `SysTaskCall` arm), whose `SysCall` arm used
/// to count EVERY argument as a write: narrowing it to the read table alone let an
/// `automatic integer sd = 7; a = $random(sd);` under a `fork` be proven
/// "never reassigned" (`const_immune`) and flattened, so both activations drew
/// from one seed — round-1 soundness, §4.5.498.
///
/// PRE: elaborate `systask.rs` `syscall_writes_arg`.
#[inline]
pub fn syscall_writes_arg(name: &str, i: usize) -> bool {
    match name {
        "$sformat" | "$swrite" | "$swriteb" | "$swriteh" | "$swriteo" | "$fgets" | "$fread"
        | "$cast" => i == 0,
        "$sscanf" | "$fscanf" => i >= 2,
        "$value$plusargs" => i == 1,
        "$random" | "$dist_uniform" | "$dist_normal" | "$dist_exponential" | "$dist_poisson"
        | "$dist_chi_square" | "$dist_t" | "$dist_erlang" => i == 0,
        _ => false,
    }
}

/// The sub-slice of a system task/func's args that are READ inputs — the write
/// DEST args of the string-building / scanf / file-read / `$cast` / `$value$plusargs`
/// families are excluded (a string populated by e.g. `$sformat`/`$sscanf`/`$fgets`/
/// `$cast`/`$value$plusargs` and never otherwise read is write-only, so must not trip
/// the coalesce read-gate). Every other task reads all its args. This must cover EVERY
/// sysfunc vita supports as a direct-rhs writer, else its write-only dest is misread.
///
/// Read by elaborate `da/reads.rs` (`rvalue_reads_ident`'s `SysCall` arm and
/// `stmt_reads_ident`'s `SysTaskCall` arm). `$value$plusargs`'s arguments past the second
/// are neither read here nor written by [`syscall_writes_arg`] (PRE's split, kept).
///
/// PRE: elaborate `systask.rs` `syscall_read_args`.
#[inline]
pub fn syscall_read_args<'a>(task: &str, args: &'a [crate::Expr]) -> &'a [crate::Expr] {
    match task {
        // dest is arg 0; the remaining args are read inputs (`$cast(dst, src)` writes
        // dst=arg0, reads src=arg1)
        "$sformat" | "$swrite" | "$swriteb" | "$swriteh" | "$swriteo" | "$fgets" | "$fread"
        | "$cast" => args.get(1..).unwrap_or(&[]),
        // source + fmt (args 0,1) are the only reads; trailing args are write dests
        "$sscanf" | "$fscanf" => &args[..2.min(args.len())],
        // `$value$plusargs(fmt, dest)`: the format (arg 0) is read, dest (arg 1) written
        "$value$plusargs" => &args[..1.min(args.len())],
        _ => args,
    }
}

/// The system calls a BARE call statement (`$fgetc(fd);`, a `SysTaskCall`) routes to the
/// statement-form specials (elaborate `stmt_main.rs` `lower_stmt_inner`: re-built as a
/// `SysCall` and offered to `scanf_special`, `fgets_special`, `fread_special`,
/// `value_plusargs_special`, `file_read_int_special`, `random_seeded_special`,
/// `dist_uniform_special`), so the call's effect happens and the value is discarded. Every
/// other bare system call keeps the warn-and-skip (W3056). The consumer's own frame test
/// (`!self.in_frame_body || self.frame_task_lowering`) is evaluated first.
///
/// Of the `$dist_*` family only `$dist_uniform` is routed:
/// its integer LCG advance AND draw match iverilog exactly
/// for all inputs (the same verified `$random` LCG). The
/// NON-uniform siblings have a PRE-EXISTING vita-vs-iverilog
/// divergence — `$dist_chi_square`/`$dist_t` in the seed LCG
/// advance itself, `$dist_normal`/`$dist_exponential`/
/// `$dist_erlang` (and possibly `$dist_poisson`) in the draw
/// VALUE (vendored-libm rounding, off-by-one) — present in
/// the ASSIGNMENT form too. Routing them bare would turn
/// base's skip into a DIFFERENT wrong value; leave them
/// warn+skip (base==fix) until the algorithm is fixed
/// (a separate, deep follow-on).
///
/// PRE: elaborate `stmt_main.rs` `lower_stmt_inner`, inline `matches!`.
#[inline]
pub fn bare_call_routed(name: &str) -> bool {
    matches!(
        name,
        "$sscanf"
            | "$fscanf"
            | "$fgets"
            | "$fread"
            | "$value$plusargs"
            | "$fgetc"
            | "$ungetc"
            | "$random"
            | "$dist_uniform"
    )
}

/// v9 SYS-READ: the fd-ADVANCING file-read functions (and `$sscanf`, which reads its
/// source string the same way) mutate the read state (and some write ref/dest args), so
/// they are direct-rhs-only special forms (the legal placements intercept in lower_stmt, or
/// are hoisted, BEFORE lower_expr is reached). A call reaching elaborate `expr_main.rs`
/// `lower_expr_ungated` with one of these names is an illegal nested placement and is
/// refused there (E3009 "advances the file position"), before its arguments are lowered.
/// This list MUST stay in sync with `file_read_int_special` (`sys_special.rs`). Round-9
/// FIO: `$feof` is PURE (reads the EOF flag, no mutation) and is intentionally NOT in this
/// list — it maps through `ir::sysfunc_id` like any pure value sysfunc, so
/// `while (!$feof(fd))` and other expression/condition placements work (re-evaluation per
/// iteration is exactly correct).
///
/// PRE: elaborate `expr_main.rs` `lower_expr_ungated`, inline `matches!`.
#[inline]
pub fn refused_nested_stream_read(name: &str) -> bool {
    matches!(
        name,
        "$fgetc" | "$ungetc" | "$fgets" | "$fread" | "$fscanf" | "$sscanf"
    )
}

/// v9 rank 6: `$dist_*` advances the ref seed, `$cast` (func form) writes the dst ref — both
/// direct-rhs-only (the legal placements intercept in lower_stmt before lower_expr). A call
/// reaching elaborate `expr_main.rs` `lower_expr_ungated` with one of these names is an
/// illegal nested placement — loud, never a silent unseeded draw / dropped cast — refused
/// there before its arguments are lowered. MUST stay in sync with `dist_uniform_special`
/// (elaborate `crv.rs`) / `cast_special` (elaborate `expr_cast.rs`):
/// `elaborate/tests/sysfn_home.rs` pins the seven `$dist_*` names against
/// `dist_uniform_special`'s literals.
///
/// PRE: elaborate `expr_main.rs` `lower_expr_ungated`, inline `matches!`.
#[inline]
pub fn refused_nested_seed_or_dest(name: &str) -> bool {
    matches!(
        name,
        "$dist_uniform"
            | "$dist_normal"
            | "$dist_exponential"
            | "$dist_poisson"
            | "$dist_chi_square"
            | "$dist_t"
            | "$dist_erlang"
            | "$cast"
    )
}

/// Is `name` (incl. the leading `$`) an SVA sampled-value function we desugar? Asked by
/// elaborate `sva_decl.rs` `rewrite_sampled` (its match guard, before the arity test).
///
/// PRE: elaborate `sva_ast.rs` `is_sva_sampled_fn`.
#[inline]
pub fn is_sva_sampled_fn(name: &str) -> bool {
    matches!(
        name,
        "$past" | "$rose" | "$fell" | "$stable" | "$changed" | "$sampled"
    )
}

/// The [`is_sva_sampled_fn`] names that test the LSB (`$rose` / `$fell`, IEEE 1800-2017
/// §16.9.3), so elaborate `sva_prop.rs` `sva_prev_for` refuses them over a `real` or a
/// `string` (bit 0 is not a value of either type). `$stable` / `$changed` compare the
/// whole value and are not in it.
///
/// PRE: elaborate `sva_prop.rs` `sva_prev_for`, `matches!(func, "$rose" | "$fell")`.
#[inline]
pub fn sva_lsb_edge(name: &str) -> bool {
    matches!(name, "$rose" | "$fell")
}
