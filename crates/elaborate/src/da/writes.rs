//! definite-assignment support: the WRITE-detection AST walkers.
//!
//! Split out of `da.rs` to hold it under the 1000-line module cap. These answer
//! "can this construct WRITE `name`?" — including a write smuggled through an
//! output / inout formal of a call. Read-side twins live in `da/reads.rs`.

use super::*;

/// EXHAUSTIVE companion to [`stmt_may_write_ident`]: true when expression `e`
/// contains a function / method / system-call / class-`new` whose ARGUMENT (or method
/// RECEIVER) references `name` — a position where a callee `output` / `inout` formal
/// could COPY BACK into `name` (IEEE §13.4.1; vita synthesizes the copy-out via
/// `emit_frame_func_out_call`, and such functions land in `inout_func_names`). da.rs
/// is pure AST analysis with NO view of the callee's port directions, so this is
/// CONSERVATIVE-FOR-ACCEPT: ANY call passing `name` is treated as a possible write (a
/// pure-input call passing `name` is harmlessly over-flagged → the block stays loud,
/// never silently accepted). A plain read of `name` in a NON-call position
/// (`name * 1ns`, `name - 5`) is NOT a call argument and is NOT flagged, so a constant
/// watchdog delay `#(timeout_ns * 1ns)` stays accepted.
///
/// Modeled on `expr_reads_ident` but recurses into EVERY sub-expression of EVERY
/// `ExprKind` variant — including the ones `expr_reads_ident` `_ => false`-swallows
/// (`MethodCall`, `ClassNew`, `RandomizeWith`, `ArrayMethodWith`, `Dist`, `NamedArg`,
/// `TimeLit`) — because a missed call here IS the silent-wrong: a mutated-under-fork
/// local wrongly proven "never written" is flattened to one shared net and aliases
/// across concurrent activations. Enumerates all variants (no `_` catch-all) so a
/// future `ExprKind` addition is a compile error, not a silent blind spot.
fn expr_call_may_write_ident(e: &ast::Expr, name: &str, out: Option<OutActualWrites>) -> bool {
    use ast::ExprKind as K;
    // A call / method / ctor whose direct ARG references `name` may copy it back; and
    // recurse each arg so a call BURIED in an arg (`f(x.m(name))`, past the
    // `expr_reads_ident` MethodCall blind spot) is still found.
    let arg_writes = |args: &[ast::Expr]| {
        args.iter()
            .any(|a| expr_reads_ident(a, name) || expr_call_may_write_ident(a, name, out))
    };
    match &e.kind {
        // Leaves with no sub-expression: cannot host a call.
        K::IntLit { .. }
        | K::RealLit { .. }
        | K::StrLit { .. }
        | K::PkgScoped { .. }
        | K::Ident(_)
        | K::Null
        | K::Dollar
        | K::Error => false,
        // Call-bearing variants: `name` in an arg / receiver may be copied back.
        // R17-X1: a two-or-more-segment call path is a METHOD on its head — and a
        // MUTATING container method (`name.push_back(v)`, `name.delete()`) WRITES that
        // head. Checking only the args made this walker answer "never written" for the
        // one shape it exists to catch. A pure QUERY (`name.size()`, `name.substr(…)`)
        // is a read, not a write; treating every method as mutating was measured to
        // keep `automatic byte e[]; $display(e.size());` loud where iverilog prints 0.
        K::Call { name: cn, args } => call_writes(
            out,
            cn,
            args,
            name,
            (cn.segments.len() >= 2
                && cn.segments[0].name == name
                && !container_method_is_pure(&cn.segments[1].name))
                || arg_writes(args),
        ),
        K::SysCall { args, .. } | K::ClassNew { args } => arg_writes(args),
        K::MethodCall { recv, args, .. } => {
            // `name.method(…)` (a mutating queue/array method) writes `name`, and an
            // arg may bind to an output/inout formal.
            expr_reads_ident(recv, name)
                || expr_call_may_write_ident(recv, name, out)
                || arg_writes(args)
        }
        K::RandomizeWith(b) => {
            arg_writes(&b.args)
                || b.constraints
                    .iter()
                    .any(|c| expr_call_may_write_ident(c, name, out))
        }
        K::ArrayMethodWith(b) => expr_call_may_write_ident(&b.with_expr, name, out),
        // Composites: recurse into every sub-expression.
        K::Unary { operand, .. } => expr_call_may_write_ident(operand, name, out),
        K::Binary { lhs, rhs, .. } => {
            expr_call_may_write_ident(lhs, name, out) || expr_call_may_write_ident(rhs, name, out)
        }
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            expr_call_may_write_ident(cond, name, out)
                || expr_call_may_write_ident(then_e, name, out)
                || expr_call_may_write_ident(else_e, name, out)
        }
        K::BitSelect { base, index } => {
            expr_call_may_write_ident(base, name, out)
                || expr_call_may_write_ident(index, name, out)
        }
        K::PartSelect { base, msb, lsb } => {
            expr_call_may_write_ident(base, name, out)
                || expr_call_may_write_ident(msb, name, out)
                || expr_call_may_write_ident(lsb, name, out)
        }
        K::IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            expr_call_may_write_ident(base, name, out)
                || expr_call_may_write_ident(offset, name, out)
                || expr_call_may_write_ident(width, name, out)
        }
        K::Concat { parts } => parts
            .iter()
            .any(|x| expr_call_may_write_ident(x, name, out)),
        K::Replicate { count, value } => {
            expr_call_may_write_ident(count, name, out)
                || value
                    .iter()
                    .any(|x| expr_call_may_write_ident(x, name, out))
        }
        K::Paren { inner } => expr_call_may_write_ident(inner, name, out),
        K::MinTypMax { min, typ, max } => {
            expr_call_may_write_ident(min, name, out)
                || expr_call_may_write_ident(typ, name, out)
                || expr_call_may_write_ident(max, name, out)
        }
        K::New { size, src } => {
            expr_call_may_write_ident(size, name, out)
                || src
                    .as_ref()
                    .is_some_and(|s| expr_call_may_write_ident(s, name, out))
        }
        // A time literal wraps a numeric operand (`#(f(name) ns)` is exotic but legal).
        K::TimeLit { num, .. } => expr_call_may_write_ident(num, name, out),
        // `.formal(value)` — the bound value can itself be an output/inout call.
        K::NamedArg { value, .. } => value
            .as_ref()
            .is_some_and(|v| expr_call_may_write_ident(v, name, out)),
        K::Dist { value, items } => {
            expr_call_may_write_ident(value, name, out)
                || items.iter().any(|it| {
                    expr_call_may_write_ident(&it.lo, name, out)
                        || it
                            .hi
                            .as_ref()
                            .is_some_and(|h| expr_call_may_write_ident(h, name, out))
                        || expr_call_may_write_ident(&it.weight, name, out)
                })
        }
        K::Cast { target, expr } => {
            expr_call_may_write_ident(expr, name, out)
                || matches!(target, ast::CastTarget::Size(s) if expr_call_may_write_ident(s, name, out))
        }
        K::AssignPattern(parts) => parts
            .iter()
            .any(|x| expr_call_may_write_ident(x, name, out)),
    }
}

/// True if an lvalue's INDEX sub-expressions host a copy-back call (`mem[f(name)] =
/// x` writes `name` via `f`'s output formal). The write-side twin of
/// [`lvalue_index_reads_ident`], routed through [`expr_call_may_write_ident`].
fn lvalue_index_call_may_write(lv: &ast::Lvalue, name: &str, out: Option<OutActualWrites>) -> bool {
    use ast::Lvalue as L;
    match lv {
        L::Ident(_) => false,
        L::BitSelect { base, index, .. } => {
            lvalue_index_call_may_write(base, name, out)
                || expr_call_may_write_ident(index, name, out)
        }
        L::PartSelect { base, msb, lsb, .. } => {
            lvalue_index_call_may_write(base, name, out)
                || expr_call_may_write_ident(msb, name, out)
                || expr_call_may_write_ident(lsb, name, out)
        }
        L::IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            lvalue_index_call_may_write(base, name, out)
                || expr_call_may_write_ident(offset, name, out)
                || expr_call_may_write_ident(width, name, out)
        }
        L::Concat { parts, .. } => parts
            .iter()
            .any(|p| lvalue_index_call_may_write(p, name, out)),
        L::Error(_) => false,
    }
}

/// Does an `@(…)` sensitivity host a copy-back call (`@(f(name)) …` — exotic but
/// legal)? Also inspects an `iff` guard. The write-side twin of
/// [`sensitivity_reads_ident`].
fn sensitivity_call_may_write(
    s: &ast::Sensitivity,
    name: &str,
    out: Option<OutActualWrites>,
) -> bool {
    match s {
        ast::Sensitivity::Star => false,
        ast::Sensitivity::List(evs) => evs.iter().any(|ev| {
            expr_call_may_write_ident(&ev.expr, name, out)
                || ev
                    .iff
                    .as_ref()
                    .is_some_and(|g| expr_call_may_write_ident(g, name, out))
        }),
    }
}

/// Does an intra-assignment event control `= @(ev) rhs` / `= repeat(n) @(ev) rhs`
/// host a copy-back call in its `repeat` count or event? Write-side twin of
/// [`intra_event_reads_ident`].
fn intra_event_call_may_write(
    e: &ast::IntraEvent,
    name: &str,
    out: Option<OutActualWrites>,
) -> bool {
    e.repeat
        .as_ref()
        .is_some_and(|r| expr_call_may_write_ident(r, name, out))
        || sensitivity_call_may_write(&e.ctrl, name, out)
}

/// BL1 (round-19): true when NO statement in `stmts` can WRITE `name`. A write is a
/// blocking / non-blocking / procedural-continuous (`assign`) / `force` / `deassign`
/// / `release` whose lvalue is rooted at `name` (whole-var, or through a select /
/// concat base — `++`/`--`/`+=` all desugar to a blocking assign, so they are
/// covered), a task call that could pass `name` as an OUTPUT / INOUT actual
/// (conservatively, ANY reference to `name` in a user-task or `randomize` actual), a
/// system task's WRITE-dest arg (`$sscanf(.., name)`, `$fgets(name, ..)` — its pure
/// READ args, isolated by `syscall_read_args`, do NOT count, so `$display(name)` of
/// the constant is not a write), a FUNCTION / METHOD / SYSTEM CALL that passes `name`
/// as an argument at ANY expression position (rhs / condition / scrutinee / loop
/// bound / return value / index / delay / intra-event / decl-init) — the callee may
/// bind it to an `output` / `inout` formal and copy back ([`expr_call_may_write_ident`])
/// — or a deferred / concurrent assertion ACTION block that writes `name`
/// (`assert #0 (x) else name = 0;`). Recurses into every nested block / control-flow /
/// timing body and every nested block's decl-initializers.
///
/// Used together with a constant-folding initializer to prove an `automatic`
/// block-local under a `fork` is CONCURRENCY-IMMUNE: a never-written constant reads
/// identically from one shared static (v1-flattened) net on every activation, so the
/// flatten is byte-identical to per-activation storage. SOUND FOR ACCEPT: every form
/// that MIGHT write `name` returns "writes", so `never_assigns` can never be fooled
/// into skipping the loud reject; an un-vetted statement that only READS is at worst a
/// harmless false "may-write" that keeps the case loud (correct-or-loud).
pub(crate) fn stmt_never_assigns_ident(stmts: &[ast::Stmt], name: &str) -> bool {
    stmt_never_writes_ident(stmts, name, None)
}

/// R17 §3.2: [`stmt_never_assigns_ident`] with an OPT-IN call resolver.
///
/// The signature-blind walker has to call every user-call actual that mentions `name`
/// a possible write, because an actual can bind to an `output`/`inout` formal. That
/// is the right default, but it also means a local passed to a pure `input` formal —
/// the entire content of the round-17 §3.2 report — reads as "may be written" and no
/// never-written argument can ever be made about it.
///
/// `out` is an OPT-IN parameter in the project's sense: passing the literal `None`
/// short-circuits every widened test back to the conservative answer, so
/// [`stmt_never_assigns_ident`] is mechanically what it was. When present it can only
/// ever REMOVE a write from consideration, and only on a PROOF — `CallEffect::Inert`
/// (touches `name` nowhere) or `CallEffect::Reads` (resolved callee, `name` only at
/// `input` formals, body proven unable to reach the flattened net). `Unknown` falls
/// straight back to the conservative walk.
pub(crate) fn stmt_never_writes_ident(
    stmts: &[ast::Stmt],
    name: &str,
    out: Option<OutActualWrites>,
) -> bool {
    !stmts.iter().any(|st| stmt_may_write_ident(st, name, out))
}

/// R17: is `method` a built-in container / string method that provably does NOT
/// MUTATE ITS RECEIVER?
///
/// A WHITELIST, and it must stay one: this feeds accept gates ("nothing writes this
/// local"), so an unrecognised name has to answer "may mutate". Anything absent —
/// `push_back`, `pop_front`, `insert`, `delete`, `putc`, `sort`, `rsort`, `reverse`,
/// `shuffle`, a user method, a typo — is treated as a write.
///
/// Only the RECEIVER is at issue. `q.first(idx)` writes its ARGUMENT (an inout ref)
/// and is still pure with respect to `q`; the argument side is answered separately by
/// the arg walk, which is why these two questions are not merged.
pub(crate) fn container_method_is_pure(method: &str) -> bool {
    matches!(
        method,
        // IEEE §6.16 string query methods.
        "len"
            | "getc"
            | "substr"
            | "toupper"
            | "tolower"
            | "compare"
            | "icompare"
            | "atoi"
            | "atohex"
            | "atooct"
            | "atobin"
            | "atoreal"
            // IEEE §7.9-7.12 container queries.
            | "size"
            | "num"
            | "exists"
            | "first"
            | "last"
            | "next"
            | "prev"
            // IEEE §7.12.3 array reductions.
            | "sum"
            | "product"
            | "and"
            | "or"
            | "xor"
            // IEEE §7.12.1 locators — all return a NEW queue, none reorder in place
            // (`sort`/`rsort`/`reverse`/`shuffle` do, and are deliberately absent).
            | "find"
            | "find_index"
            | "find_first"
            | "find_last"
            | "find_first_index"
            | "find_last_index"
            | "min"
            | "max"
            | "unique"
            | "unique_index"
    )
}

/// The one place the opt-in resolver is consulted: does this CALL write `name`?
///
/// With NO resolver the answer is `fallback`, the signature-blind guess, and the walk
/// is byte-identical to what it was. WITH a resolver the polarity flips for the
/// unresolved case: an `Unknown` verdict answers "may write" rather than falling back.
/// That is not symmetry for its own sake — the caller is about to claim the local is
/// NEVER written, and the signature-blind fallback answers "no write" for a plain
/// `poke();` whose body assigns the flattened net through a hierarchical self-path
/// (`task poke; t.a = 99; endtask` — measured in round-16, not hypothetical). The
/// resolver is the only thing that looks at callee bodies, so when it is present and
/// still cannot prove the call harmless, the honest answer is "may write".
fn call_writes(
    out: Option<OutActualWrites>,
    cn: &ast::HierPath,
    args: &[ast::Expr],
    name: &str,
    fallback: bool,
) -> bool {
    match out.map(|f| f(cn, args, name)) {
        Some(CallEffect::Writes) | Some(CallEffect::Unknown) => true,
        Some(CallEffect::Inert) | Some(CallEffect::Reads) => false,
        None => fallback,
    }
}

/// The recursive worker for [`stmt_never_assigns_ident`] — true if `s` (or any
/// nested sub-statement) can WRITE `name`. Enumerates EVERY `Stmt` variant (no `_`
/// catch-all) so a future statement form with a write position is a compile error,
/// not a silent blind spot: an lvalue rooted at `name`, a copy-back CALL at any
/// expression position ([`expr_call_may_write_ident`]), an assertion ACTION block, or
/// a nested block's decl-initializer.
fn stmt_may_write_ident(s: &ast::Stmt, name: &str, out: Option<OutActualWrites>) -> bool {
    use ast::Stmt::*;
    match s {
        // Direct lvalue write rooted at `name`, PLUS a copy-back call hidden in the
        // rhs / lvalue-index / delay / intra-event.
        Blocking {
            lhs,
            delay,
            event,
            rhs,
            ..
        }
        | NonBlocking {
            lhs,
            delay,
            event,
            rhs,
            ..
        } => {
            lvalue_root_is(lhs, name)
                || lvalue_index_call_may_write(lhs, name, out)
                || expr_call_may_write_ident(rhs, name, out)
                || delay.as_ref().is_some_and(|d| {
                    d.values
                        .iter()
                        .any(|e| expr_call_may_write_ident(e, name, out))
                })
                || event
                    .as_ref()
                    .is_some_and(|e| intra_event_call_may_write(e, name, out))
        }
        Assign { lhs, rhs, .. } | Force { lhs, rhs, .. } => {
            lvalue_root_is(lhs, name)
                || lvalue_index_call_may_write(lhs, name, out)
                || expr_call_may_write_ident(rhs, name, out)
        }
        Deassign { lhs, .. } | Release { lhs, .. } => {
            lvalue_root_is(lhs, name) || lvalue_index_call_may_write(lhs, name, out)
        }
        Block { stmts, decls, .. } | Fork { stmts, decls, .. } => {
            stmts.iter().any(|st| stmt_may_write_ident(st, name, out))
                // A nested-block decl-init `int z = f(name);` writes `name` via the
                // callee's output/inout copy-out — mirror `stmt_reads_ident`'s decl walk.
                || decls.iter().flat_map(|d| d.names.iter()).any(|n| {
                    n.init
                        .as_ref()
                        .is_some_and(|e| expr_call_may_write_ident(e, name, out))
                })
        }
        If {
            cond,
            then_s,
            else_s,
            ..
        } => {
            expr_call_may_write_ident(cond, name, out)
                || stmt_may_write_ident(then_s, name, out)
                || else_s
                    .as_ref()
                    .is_some_and(|e| stmt_may_write_ident(e, name, out))
        }
        Case {
            scrutinee, items, ..
        } => {
            expr_call_may_write_ident(scrutinee, name, out)
                || items.iter().any(|it| match it {
                    ast::CaseItem::Match { labels, body, .. } => {
                        labels
                            .iter()
                            .any(|e| expr_call_may_write_ident(e, name, out))
                            || stmt_may_write_ident(body, name, out)
                    }
                    ast::CaseItem::Default { body, .. } => stmt_may_write_ident(body, name, out),
                })
        }
        For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            stmt_may_write_ident(init, name, out)
                || expr_call_may_write_ident(cond, name, out)
                || stmt_may_write_ident(step, name, out)
                || stmt_may_write_ident(body, name, out)
        }
        While { cond, body, .. } => {
            expr_call_may_write_ident(cond, name, out) || stmt_may_write_ident(body, name, out)
        }
        Repeat { count, body, .. } => {
            expr_call_may_write_ident(count, name, out) || stmt_may_write_ident(body, name, out)
        }
        Forever { body, .. } => stmt_may_write_ident(body, name, out),
        Wait { cond, body, .. } => {
            expr_call_may_write_ident(cond, name, out)
                || body
                    .as_ref()
                    .is_some_and(|b| stmt_may_write_ident(b, name, out))
        }
        DelayCtrl { delay, body, .. } => {
            delay
                .values
                .iter()
                .any(|e| expr_call_may_write_ident(e, name, out))
                || body
                    .as_ref()
                    .is_some_and(|b| stmt_may_write_ident(b, name, out))
        }
        EventCtrl { ctrl, body, .. } => {
            sensitivity_call_may_write(ctrl, name, out)
                || body
                    .as_ref()
                    .is_some_and(|b| stmt_may_write_ident(b, name, out))
        }
        Return { value, .. } => value
            .as_ref()
            .is_some_and(|e| expr_call_may_write_ident(e, name, out)),
        // A user-task / randomize actual could be an OUTPUT / INOUT — any reference to
        // `name` is conservatively a possible write (keeps it loud; sound for accept).
        UserTaskCall { name: cn, args, .. } => call_writes(
            out,
            cn,
            args,
            name,
            args.iter().any(|a| expr_reads_ident(a, name)),
        ),
        RandomizeWith { args, .. } => args.iter().any(|a| expr_reads_ident(a, name)),
        // A system task's WRITE-dest arg (`$sscanf(.., name)`) writes `name`; its READ
        // args (isolated by `syscall_read_args`) do not — BUT a READ arg can still host
        // a nested output/inout CALL (`$display(f(name))`), so scan every arg for a
        // copy-back call too.
        SysTaskCall {
            name: task, args, ..
        } => {
            let reads = syscall_read_args(task.name.as_str(), args);
            args.iter().any(|a| {
                (!reads.iter().any(|r| std::ptr::eq(r, a)) && expr_reads_ident(a, name))
                    || expr_call_may_write_ident(a, name, out)
            })
        }
        // Deferred immediate assertion `assert #0 (c) [pass] else fail;` — the pass /
        // fail ACTION blocks can WRITE `name`, and the sampled cond can host a call.
        DeferredAssert {
            cond,
            then_s,
            else_s,
            ..
        } => {
            expr_call_may_write_ident(cond, name, out)
                || stmt_may_write_ident(then_s, name, out)
                || stmt_may_write_ident(else_s, name, out)
        }
        // Concurrent assertion `assert property(...) [pass] else fail;` — only the
        // pass / fail ACTION blocks carry a data write; the clock / disable_iff /
        // antecedent / consequent are SAMPLED value expressions (IEEE 1800 §16 —
        // assertion expressions are side-effect free), so they cannot write `name`.
        ConcurrentAssert { pass, fail, .. } => {
            pass.as_ref()
                .is_some_and(|p| stmt_may_write_ident(p, name, out))
                || fail
                    .as_ref()
                    .is_some_and(|f| stmt_may_write_ident(f, name, out))
        }
        // No data-variable write position and no callable that could copy back `name`:
        // `-> ev` triggers an event (not a data write); `disable` / `wait fork` carry
        // no expression; `cover property` only COUNTS matches (no action block; its
        // clock / disable_iff / seq are sampled, side-effect free); `;` / recovery
        // error are inert.
        EventTrigger { .. }
        | Disable { .. }
        | WaitFork { .. }
        | CoverProperty { .. }
        | Null(_)
        | Error(_) => false,
    }
}

/// Is the WRITTEN base of lvalue `lv` the identifier `name`? (`name = …`,
/// `name[i] = …`, `name[a:b] = …`, or `name` as one target of a concat write
/// `{name, …} = …`). A name appearing only in a select INDEX (`other[name] = …`) is
/// a READ, not a write, and does NOT match. The write-side counterpart of
/// [`lval_root_name`] extended to concat targets. Used by `stmt_may_write_ident`.
fn lvalue_root_is(lv: &ast::Lvalue, name: &str) -> bool {
    use ast::Lvalue as L;
    match lv {
        L::Ident(p) => p.segments.len() == 1 && p.segments[0].name == name,
        L::BitSelect { base, .. } | L::PartSelect { base, .. } | L::IndexedPart { base, .. } => {
            lvalue_root_is(base, name)
        }
        L::Concat { parts, .. } => parts.iter().any(|p| lvalue_root_is(p, name)),
        L::Error(_) => false,
    }
}
