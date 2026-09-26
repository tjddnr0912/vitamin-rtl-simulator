//! casts — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// `(width, signed, two_state)` of a primitive casting type — the ONE table shared
/// by the lowering ([`Elaborator::lower_prim_cast`]) and the integer const domain
/// (`const_eval_cast`), so a cast can never fold to one type at elaborate time and
/// lower as another. `Real` has no integral shape and yields None.
pub(crate) fn cast_prim_wsign(p: ast::CastPrim) -> Option<(u32, bool, bool)> {
    use ast::CastPrim as P;
    Some(match p {
        P::Int => (32, true, true),
        P::Integer => (32, true, false),
        P::Byte => (8, true, true),
        P::Shortint => (16, true, true),
        P::Longint => (64, true, true),
        P::Bit => (1, false, true),
        P::Logic | P::Reg => (1, false, false),
        P::Time => (64, false, false),
        P::Real => return None,
    })
}

impl Elaborator<'_> {
    // ── SV static cast `casting_type'(expr)` (IEEE 1800 §6.24) ──────────────
    // Numeric, size, and signing casts are iverilog-pinned and lower entirely to
    // EXISTING IR (IR-0); class/typedef-name casts have no oracle yet → loud-reject
    // (correct-or-loud, never silent-wrong). `string'(e)` is the one arm that is NOT
    // IR-0: it emits `SysFuncId::StrCast` (format_version 33) — see
    // `lower_string_cast` for why no composition of existing nodes expresses it.
    pub(crate) fn lower_cast(&mut self, target: &ast::CastTarget, operand: &ast::Expr) -> u32 {
        match target {
            // signed'(e) / unsigned'(e): PRESERVE width, flip the sign attribute.
            ast::CastTarget::Signing { signed } => {
                let e = self.lower_expr(operand);
                if self.cast_operand_is_real(operand, e) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "signed'/unsigned' cast is not defined on a real operand",
                    );
                    return self.placeholder_expr();
                }
                let which = if *signed {
                    ir::SysFuncId::Signed
                } else {
                    ir::SysFuncId::Unsigned
                };
                self.push_expr(ir::Expr::SysFunc {
                    which,
                    args: vec![e],
                })
            }
            // §3 ⑤ⓕ: the same cast whose SIGN is this INSTANCE's, not the parse
            // site's — `T'(e)` / a whole-member read of a `T` struct member. Same
            // body as the arm above with the folded bit in place of the literal;
            // an unresolvable `T$s` is loud, never a guessed default.
            ast::CastTarget::SigningParam { shape_param } => {
                let Some(signed) = self.cast_shape_signed(shape_param) else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a cast to a type parameter whose per-instance shape does not \
                         resolve in this scope",
                    );
                    return self.placeholder_expr();
                };
                let e = self.lower_expr(operand);
                if self.cast_operand_is_real(operand, e) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "signed'/unsigned' cast is not defined on a real operand",
                    );
                    return self.placeholder_expr();
                }
                let which = if signed {
                    ir::SysFuncId::Signed
                } else {
                    ir::SysFuncId::Unsigned
                };
                self.push_expr(ir::Expr::SysFunc {
                    which,
                    args: vec![e],
                })
            }
            // N'(e): result is N bits; signedness INHERITED from the operand.
            ast::CastTarget::Size(_) => {
                // `cast_size_bits` owns the size fold (SELF-determined — the size
                // expression has no outer context); a private `const_eval_in_scope`
                // here was a second spelling that widened `(4'd9+4'd8)'(2)` to 17
                // bits while the const domain (post-§2-fix) says 1.
                let n = match self.cast_size_bits(target) {
                    Some(n) if n >= 1 && (n as u64) <= MAX_NET_WIDTH => n as u32,
                    _ => {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "size cast width must be a positive constant expression",
                        );
                        return self.placeholder_expr();
                    }
                };
                // §11.6: the cast width N is the operand's context, so a fill
                // literal (`'1`/`'x`/`'z`) grows to N bits (a bare `lower_expr` sizes
                // it to 1 bit, then `lower_size_cast` zero-extends it = silent-wrong:
                // `8'('1)` gave `01` instead of `ff`). Byte-identical for a non-fill.
                // §4.5.212: when the operand is a context-determined OPERATION
                // (`8'(a*b)`, `6'(a<<1)`), N is the context for the WHOLE operation, so
                // the arithmetic must run at N bits (`lower_size_ctx`), not at the
                // operands' self-width then resize (which lost the carry). The sign of
                // every leaf follows the operand's overall sign (`ast_ctx_signed`); if
                // that can't be resolved here the fill-only path is kept (no regression).
                let e = match (
                    Self::is_size_ctx_operation(operand),
                    self.size_ctx_route(operand),
                ) {
                    (true, Some((ext, w))) => self.lower_size_ctx_entry(operand, n, ext, w),
                    _ => self.lower_ctx_or_plain(operand, n),
                };
                if self.cast_operand_is_real(operand, e) {
                    self.error(MsgCode::ElabUnsupported, REAL_SIZE_CAST_MSG);
                    return self.placeholder_expr();
                }
                self.lower_size_cast(e, n)
            }
            ast::CastTarget::Prim(p) => self.lower_prim_cast(*p, operand),
            // `name'(e)`: a bare single identifier that folds to a constant
            // parameter/localparam is a legal SIZE cast `W'(e)` (§6.24 casting_type
            // = constant_primary). Otherwise it is a typedef/class NAME cast, which
            // has no oracle yet → loud. `const_eval_in_scope` folds ONLY genuine
            // constants (a net/typedef/class name yields None), so correct-or-loud
            // is preserved.
            ast::CastTarget::Named(path) => {
                // §6.16/§6.24.1 `string'(e)` — the reserved spelling the parser
                // gives the `string` KEYWORD (`ast::STRING_CAST_NAME`). Checked
                // FIRST because every arm below resolves the path through a scope,
                // and a keyword resolves to nothing there: without this the cast
                // would fall out of the chain as "typedef/class cast … outside the
                // v1 cast scope", which is what it did before this slice.
                if target.is_string_cast() {
                    return self.lower_string_cast(operand);
                }
                if path.segments.len() == 1 {
                    let id_expr = ast::Expr {
                        kind: ast::ExprKind::Ident(path.clone()),
                        span: path.span,
                    };
                    if let Some(n) = self.const_eval_in_scope(&id_expr) {
                        if n >= 1 && (n as u64) <= MAX_NET_WIDTH {
                            // fill literal grows to the cast width N (see Size arm).
                            // §4.5.212: a context-determined operation runs at N bits.
                            let e = match (
                                Self::is_size_ctx_operation(operand),
                                self.size_ctx_route(operand),
                            ) {
                                (true, Some((ext, w))) => {
                                    self.lower_size_ctx_entry(operand, n as u32, ext, w)
                                }
                                _ => self.lower_ctx_or_plain(operand, n as u32),
                            };
                            if self.cast_operand_is_real(operand, e) {
                                self.error(MsgCode::ElabUnsupported, REAL_SIZE_CAST_MSG);
                                return self.placeholder_expr();
                            }
                            return self.lower_size_cast(e, n as u32);
                        }
                    }
                }
                // A.5: a single-seg path naming a CLASS is a class cast `Base'(d)`
                // (§6.24.2). v1 supports the UP-cast (and identity): the target is
                // the operand's class or an ANCESTOR. An up-cast is a pure IDENTITY
                // on the handle value — the heap object keeps its concrete class_id,
                // virtual dispatch reads the dynamic type (the static cast type is
                // irrelevant), and the static type for later member access rides on
                // the DESTINATION net's own `net_class`, not on this lowered value.
                // So we just return the operand handle UNCHANGED. A down-cast /
                // unrelated cast / unresolvable operand is loud (correct-or-loud:
                // a cast we cannot validate must not silently pass).
                if path.segments.len() == 1 && self.class_table.contains_key(&path.segments[0].name)
                {
                    let target = path.segments[0].name.clone();
                    match self.operand_static_class(operand) {
                        Some(op_class)
                            if target == op_class || self.class_is_ancestor(&target, &op_class) =>
                        {
                            // Legal up-cast / identity → handle value unchanged.
                            return self.lower_expr(operand);
                        }
                        Some(op_class) if self.class_is_ancestor(&op_class, &target) => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "a static class DOWN-cast `Derived'(base)` is outside the v1 \
                                 cast scope (only up-casts to a base class are supported)",
                            );
                            return self.placeholder_expr();
                        }
                        Some(_) => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "an UNRELATED class cast (target is neither the operand's class \
                                 nor a base of it) is illegal (IEEE §6.24.2)",
                            );
                            return self.placeholder_expr();
                        }
                        None => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "a class cast `Base'(expr)` requires an operand whose static \
                                 class is resolvable (a handle variable, `this`, or a handle \
                                 field) so the cast relationship can be validated",
                            );
                            return self.placeholder_expr();
                        }
                    }
                }
                self.error(
                    MsgCode::ElabUnsupported,
                    "typedef/class cast `name'(expr)` is outside the v1 cast scope \
                     (int'/byte'/…/N'/signed' casts are supported)",
                );
                self.placeholder_expr()
            }
        }
    }

    /// `string'(e)` (IEEE §6.16 / §6.24.1) — the integral→string conversion as an
    /// expression.
    ///
    /// Three routes, in this order:
    /// * A `real` operand is LOUD. iverilog 13 refuses it ("sorry: This cast
    ///   operation is not yet supported") and verilator 5.052 renders the raw f64
    ///   bytes (`3.5` -> `@\x0c`), so there is no oracle for a value — correct-or-loud.
    /// * A STRING operand is the IDENTITY (§6.16 converts an INTEGRAL operand; a
    ///   string one is already the result). `string'(s)` on `string s = "hi"` is
    ///   `hi`/2 on verilator; iverilog is no oracle for the probe (it has no
    ///   `.getc` method, so the design does not elaborate).
    /// * Everything else becomes `SysFuncId::StrCast` over the operand at its OWN
    ///   self-determined width — the cast names a type with no width of its own, so
    ///   there is no context to lend and `lower_expr` is the whole rule. The engine
    ///   then applies `Value::to_sv_string_bytes`, the one funnel the implicit
    ///   `string s = <integral>` assignment also uses.
    ///
    /// The result is a string-domain VALUE, so `ir_expr_is_string` claims `StrCast`
    /// and `{string'(x), "!"}`, `%s`, a string formal, a `case` scrutinee and a
    /// string compare all see it as a string. A METHOD on the cast
    /// (`string'(x).len()`) is NOT supported and must stay loud: both oracles refuse
    /// it (verilator "Not expecting CVTPACKSTRING under a DOT in dotted expression";
    /// iverilog "syntax error"), and the parser's `'(`-guarded arm never produces a
    /// method receiver, so the shape is refused where it is written.
    fn lower_string_cast(&mut self, operand: &ast::Expr) -> u32 {
        // A string LITERAL operand is the identity at the AST level — its lowered
        // form is a packed `Const`, which `ir_expr_is_string` cannot claim.
        if matches!(operand.kind, ast::ExprKind::StrLit { .. }) {
            return self.lower_expr(operand);
        }
        let e = self.lower_expr(operand);
        if self.cast_operand_is_real(operand, e) {
            self.error(
                MsgCode::ElabUnsupported,
                "`string'(expr)` is not defined on a real operand (IEEE §6.16 converts \
                 an INTEGRAL value); iverilog rejects it too",
            );
            return self.placeholder_expr();
        }
        if self.ir_expr_is_string(e) {
            return e;
        }
        self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::StrCast,
            args: vec![e],
        })
    }

    /// The signedness a cast INHERITS from its operand (§6.24.1) — the
    /// CANONICAL §5.4.1/§5.5 rule, i.e. the same answer the engine's width
    /// table will give the very node being wrapped. `None` = NOT YET KNOWABLE.
    ///
    /// It cannot be `expr_self_signed`: that is elaborate's own hand-written
    /// mirror, and it disagrees with the canonical rule at leaves whose sign is a
    /// SIDECAR rather than a net flag (the class-field sidecar is the one it
    /// reads). It reads UNSIGNED where the rule says
    /// signed at a user function's declared return and the whole
    /// int-returning system-function family it simply does not list (`$clog2`,
    /// `$countones`, `$random`, `$fopen`, the file-read family, `$dist_*`, the
    /// string→int methods). The cast then extended by the wrong sign, silently:
    /// `16'(f()+4'sd0)` with `function signed [3:0] f` printed `000d` where both
    /// oracles print `fffd`, `int'(f())` printed `0000000d` for `fffffffd`, and
    /// `4'($clog2(300))` printed `0009` for `fff9`.
    ///
    /// It reads SIGNED where the rule says unsigned at exactly one id, `$stime`
    /// (enumerated over every `SysFuncId`) — see `lower_size_cast` for why the
    /// rule wins there. `$urandom` is NOT in either list: both call it unsigned.
    /// An element-typed `pop` cannot reach a cast (`8'(q.pop_front())` is E3009)
    /// but an array REDUCTION can: `8'(q.sum())` on a `byte signed q[$]` goes
    /// `000000fd` → `fffffffd`.
    fn cast_operand_signed(&mut self, e: u32) -> Option<bool> {
        self.canonical_self_width(e).map(|sw| sw.signed)
    }

    /// The sign to EXTEND a cast operand by, and the sign the cast INHERITS.
    ///
    /// The canonical rule answers wherever it can. A widening of a signed operand
    /// that may not be repeated (`16'(fp(0))`, `16'(fur(0))` over a `$urandom`
    /// wrapper, `int'(f())`) goes through `extend_signed_once`, which names the
    /// operand once, so adopting the canonical "signed" there costs no second
    /// evaluation. `extend_to`'s `Select{Bit}` sign fill is a second mention and
    /// is only built over a repeatable operand.
    ///
    /// ⚠️ `fabricated_widening`: the cast widens and the operand's width is NOT a
    /// declared fact (`ir_bits_of` answered `None`, and the caller fabricated 32).
    /// The ternary may not be used there — it evaluates at max(runtime width, n)
    /// and never cuts to `n` (`40'(q48.sum())` gave `ffff800000000001` for
    /// `0000800000000001`) — so the extension is `extend_to`, whose sign fill
    /// names the operand twice. A non-repeatable operand therefore keeps the
    /// MIRROR's sign there, the pre-existing answer: `longint'(q.sum())` on a
    /// `byte signed q[$]` stays `00000000000000fd` (verilator `fffffffffffffffd`,
    /// ROADMAP §2) rather than drawing twice.
    fn cast_extend_signed(&mut self, e: u32, fabricated_widening: bool) -> bool {
        match self.cast_operand_signed(e) {
            Some(true) if fabricated_widening && !self.expr_is_repeatable(e) => {
                self.expr_self_signed(e)
            }
            Some(s) => s,
            // NOT YET KNOWABLE (a deferred hierarchical reference is still a
            // placeholder): fall back to the mirror, which is what those
            // operands got before this slice.
            None => self.expr_self_signed(e),
        }
    }

    /// `N'(e)` — width N, INHERITING the operand's signedness (§6.24.1).
    ///
    /// The `$signed`/`$unsigned` tail is not decoration: it is the SEAL. A cast
    /// is a self-determined boundary (§11.8.1) — `8'(a*a)` multiplies two 8-bit
    /// operands no matter how wide the expression around it is — but a bare
    /// `Binary`/`Unary`/`Ternary` node is CONTEXT-determined to the engine, so
    /// an enclosing width propagates straight through it and re-runs the whole
    /// operation wider. The `Equal` arm used to return that node raw and every
    /// consumer wider than N then read a different computation:
    /// `logic [15:0] r; r = 8'(a*a)` with `a = 8'hff` printed `fe01` where
    /// iverilog prints `0001` (measured: 634 of 4032 cells, every one of them
    /// with the destination wider than N — assignment, NBA, continuous assign,
    /// port connection, `case` scrutinee, comparison, wider binary operand and
    /// function argument all leak; a concat part or a `$display` argument does
    /// not, because those positions are self-determined already).
    ///
    /// `$signed`/`$unsigned` is the repo's seal primitive (the index seal in
    /// `packed.rs` uses it for the same reason): the engine evaluates its
    /// operand SELF-determined and only then resizes to the context. The other
    /// two arms were already sealed — `extend_to` builds a `Concat` and
    /// `select_low` a `Select`, both self-determined — so ADDING the tail to them
    /// is a no-op (measured: dropping the added `$unsigned` again moves 0 of
    /// 23,667 cells across the three backends). ⚠️ That is not the same claim as
    /// "the tail does nothing there": dropping it ENTIRELY, `$signed` included,
    /// moves 442 of those cells, because `extend_to`/`select_low` are unsigned and
    /// the `$signed` restamp is what the cast inherits.
    ///
    /// ⚠️ THE SEAL NEEDS THE OPERAND'S OWN WIDTH, AND IT IS NOT ALWAYS KNOWN.
    /// `ir_bits_of` answers `None` for a deferred hierarchical placeholder, a
    /// `string` net, the string-producing system functions, and the element-typed
    /// `pop`/array-reduction family — and the caller then FABRICATES 32. Sealing
    /// on that is a rung DOWN, both directions measured:
    ///   - `32'(u1.s)` with `logic signed [15:0] s` — `w` defaults to 32, so
    ///     only `N == 32` reaches the `Equal` arm, and there the old `return e`
    ///     was LOAD-BEARING: the engine's post-resolve width table sign-extended
    ///     it. Sealing stamped the mirror's `$unsigned` → `0000fffd` for the
    ///     oracles' `fffffffd` (28 cells of a 792-cell hierarchical matrix).
    ///   - `40'(s)` with `string s = "abcd"` — a string net's table width is 0,
    ///     so the `Concat` the widening arm builds has a CANONICAL self-width of
    ///     `N−31`, and `$unsigned` evaluates its operand at exactly that width:
    ///     `"abcd"` became `164` at `N = 40`, banded over `33 ≤ N ≤ 61` (the
    ///     mechanism caps the damage at `N ≤ 62` for any string). The bare
    ///     `Concat` escaped it because `eval_concat` reads the parts' real values.
    ///
    /// So the seal is DECLINED exactly when `ir_bits_of` declines, reproducing
    /// the pre-slice shape. Those operands keep the pre-existing leak (ROADMAP
    /// §2 — the same `ir_bits_of` gap the width probe has). The SIGN being
    /// unknown is NOT a reason to decline: `cast_extend_signed` already falls
    /// back to the mirror there, which is the sign the pre-slice code stamped,
    /// so the seal only materializes a decision that was already being made —
    /// and declining on it as well cost `8'(ua ** u1.k)` (a placeholder EXPONENT
    /// with a known 4-bit base) its fix, `02d9` for iverilog's `00d9`.
    ///
    /// With the width known, this function returns a self-determined N-bit node;
    /// with it fabricated it returns exactly what it returned before the seal.
    pub(crate) fn lower_size_cast(&mut self, e: u32, n: u32) -> u32 {
        self.release_hier_call_for_widening_cast(e, n);
        let known_w = self.ir_bits_of(e);
        let canon_w = self.canonical_self_width(e).map(|s| s.width);
        let w = known_w.unwrap_or(32);
        // The width is TRUSTWORTHY when `ir_bits_of` answered and the canonical
        // rule does not contradict it. Both halves are load-bearing: `None` is a
        // fabricated 32 (below), and a `Some` can be fabricated too — a class
        // field lowers to `Signal{net: 32-bit HANDLE net}` with its real width in
        // the `class_field_widths` sidecar, so `ir_bits_of` reads the handle's 32
        // and `32'(c.u8 + ua)` landed on the same-width arm and sealed there:
        // `0000` for the oracles' `0100`, and `32'(c.s8 + sp)` even flipped sign.
        // `is_none_or` keeps the case where only the canonical rule declines:
        // `ir_bits_of` answers without recursing into the whole tree wherever
        // IEEE gives the width from one side — `**` and the four shifts (Table
        // 11-21: the LEFT operand) and every comparison / logical op / reduction
        // (a fixed 1) — so a placeholder in the OTHER operand stops the canonical
        // walk while the width stays sound. `8'(ua ** u1.k)` is the measured
        // witness; declining on the sign as well cost that cell its fix once.
        let trusted_w = known_w.is_some() && canon_w.is_none_or(|c| c == w);
        let signed_op = self.cast_extend_signed(e, n > w && known_w.is_none());
        let resized = match n.cmp(&w) {
            // Same width: no resize node — the seal below is the whole job.
            std::cmp::Ordering::Equal => e,
            // Extend: sign-extend iff the operand is signed (§6.24.1), 4-state-
            // preserving (a bitwise `| 0` would corrupt Z→X). An operand that may
            // not be repeated is sign-extended by the single-mention ternary, but
            // only over a DECLARED width (see `cast_extend_signed`).
            std::cmp::Ordering::Greater => {
                if signed_op && known_w.is_some() && !self.expr_is_repeatable(e) {
                    self.extend_signed_once(e, n)
                } else {
                    self.extend_to(e, w, n, signed_op)
                }
            }
            // Truncate to the low N bits (Select is unsigned).
            std::cmp::Ordering::Less => self.select_low(e, n),
        };
        if !trusted_w {
            // Width fabricated ⇒ no seal. The tail SHAPE is the pre-slice one (the
            // `Equal` arm returned `e` with no stamp at all); the sign INPUT is
            // still the canonical one, which is a fix in its own right here —
            // `8'(q.sum())` on a `byte signed q[$]` goes `000000fd` → `fffffffd`.
            return if signed_op && n != w {
                self.push_expr(ir::Expr::SysFunc {
                    which: ir::SysFuncId::Signed,
                    args: vec![resized],
                })
            } else {
                resized
            };
        }
        let which = if signed_op {
            ir::SysFuncId::Signed
        } else {
            ir::SysFuncId::Unsigned
        };
        self.push_expr(ir::Expr::SysFunc {
            which,
            args: vec![resized],
        })
    }

    /// `keyword'(e)` — a primitive-type cast. Width/sign/state come from the named
    /// type; the EXTEND direction follows the OPERAND's sign (engine-resized),
    /// while the RESULT sign is the target's. 2-state targets coerce X/Z→0.
    pub(crate) fn lower_prim_cast(&mut self, p: ast::CastPrim, operand: &ast::Expr) -> u32 {
        use ast::CastPrim as P;
        // real target: real'(real)=identity; real'(integral)=$itor.
        if matches!(p, P::Real) {
            let e = self.lower_expr(operand);
            if self.cast_operand_is_real(operand, e) {
                return e;
            }
            return self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Itor,
                args: vec![e],
            });
        }
        let (tw, tsigned, t2state) = cast_prim_wsign(p)
            .expect("`Real` is handled above; every other prim has a table entry");
        // The target width `tw` is the operand's context, so a fill literal grows
        // to `tw` bits (a bare `lower_expr` sizes it to 1 bit, then the resize below
        // zero-extends it = silent-wrong: `int'('1)` gave `00000001`, not all-ones).
        // Byte-identical for a non-fill operand.
        let e = self.lower_ctx_or_plain(operand, tw);
        // real operand → integral target: round half away from zero, then narrow.
        if self.cast_operand_is_real(operand, e) {
            return self.lower_real_to_int_cast(e, tw, tsigned, t2state);
        }
        // integral operand: resize to the target width (sign-extend per the
        // OPERAND's sign), coerce X/Z for 2-state, then stamp the target sign.
        let w_known = self.ir_bits_of(e);
        let w = w_known.unwrap_or(32);
        // The 2-state coercion is `TwoState` (x/z→0, the operand's width and sign),
        // which names its operand ONCE — the engine walks the DAG as a TREE, so a
        // per-bit coercion re-evaluated the operand per result bit (`int'(f())`
        // called `f` 32 times, `int'($random)` drew 32 times). It is built only
        // where the operand can CARRY an x or z: `expr_may_be_unknown` is
        // conservative in the safe direction, and it answers false for `TwoState`,
        // so a nested coercion is not coerced again. It is asked about `e`, not the
        // resized value: every resize node below forwards the question to `e`.
        //
        // A WIDENING cast coerces the operand first and then extends it. The two
        // orders agree: an unsigned extension adds literal zeros, and a signed one
        // replicates the sign bit, so coercing that bit and then replicating it
        // equals replicating it and coercing each copy (an x or z sign bit becomes
        // zeros either way — the `X=-3 Y=3` / `A=ffffffffffffff8a` lines pinned in
        // `cli/tests/two_state_cast_fanout.rs`). The signed extension is
        // `extend_signed_once`, one mention; `extend_to`'s `Select{Bit}` fill would
        // be a second one. NARROWING selects first and coerces at the target width.
        //
        // ⚠️ Every arm builds the per-bit `coerce_two_state` (and `extend_to`) when
        // `w` is FABRICATED (`ir_bits_of` answered `None`, or 0): that `Concat` has
        // the declared width `tw`, while `TwoState` passes the operand's unknown
        // width through — `$bits(int'(q.sum()))` became E3009, `int'(qu8.sum())`
        // `000000fd` → `fffffffd`, and `48'(int'(s))` over a string lost its top
        // half. The operand is then named once per bit, as before (ROADMAP §2).
        let width_known = w_known.is_some() && w > 0;
        let needs_coerce = t2state && self.expr_may_be_unknown(e);
        let coerced = match tw.cmp(&w) {
            std::cmp::Ordering::Equal => {
                if !needs_coerce {
                    e
                } else if width_known {
                    self.two_state_once(e)
                } else {
                    self.coerce_two_state(e, tw)
                }
            }
            // Sign-extend iff the operand is signed (§6.24/§11.6.1); 4-state-
            // preserving (a `| 0` would zero-extend a signed operand AND corrupt
            // Z→X). The OPERAND's sign comes from the canonical rule for the same
            // reason the size cast's does (`cast_extend_signed`) — the mirror called
            // a signed function return unsigned and `int'(f())` zero-extended −3.
            // ⚠️ It is asked about `e` and not about the coerced value, so the
            // answer does not depend on how the coercion is spelled.
            std::cmp::Ordering::Greater => {
                let signed_op = self.cast_extend_signed(e, !width_known);
                // ⚠️ Two shapes keep the resize-then-coerce order, and the second
                // is a VALUE guard, not a tidiness one:
                //   - `w == 0` (no operand bits to coerce first).
                //   - `ir_bits_of` answered `None` and `w` is a FABRICATED 32
                //     (a deferred hierarchical placeholder, a `string` net, the
                //     string-producing system functions, the element-typed
                //     `pop`/array-reduction family — the list `lower_size_cast`'s doc
                //     enumerates). Coerce-first would freeze that guess into the low
                //     half; the equivalence above rests on `w` being the operand's
                //     ACTUAL width, so keep the old order where it is not.
                if needs_coerce && width_known {
                    let low = self.two_state_once(e);
                    if signed_op {
                        self.extend_signed_once(low, tw)
                    } else {
                        let zero = self.const_u32_expr(0, 1);
                        self.extend_with_fill(low, zero, tw - w)
                    }
                } else {
                    let resized = if signed_op && width_known && !self.expr_is_repeatable(e) {
                        self.extend_signed_once(e, tw)
                    } else {
                        self.extend_to(e, w, tw, signed_op)
                    };
                    if needs_coerce {
                        self.coerce_two_state(resized, tw)
                    } else {
                        resized
                    }
                }
            }
            std::cmp::Ordering::Less => {
                let resized = self.select_low(e, tw);
                if !needs_coerce {
                    resized
                } else if width_known {
                    self.two_state_once(resized)
                } else {
                    self.coerce_two_state(resized, tw)
                }
            }
        };
        let which = if tsigned {
            ir::SysFuncId::Signed
        } else {
            ir::SysFuncId::Unsigned
        };
        self.push_expr(ir::Expr::SysFunc {
            which,
            args: vec![coerced],
        })
    }

    /// §13.5.3: a subroutine call is an ASSIGNMENT to a variable of the formal's
    /// DECLARED type, so a REAL actual bound to an INTEGRAL formal ROUNDS
    /// (§6.24.1, half away from zero) and then NARROWS to the formal's width —
    /// the same rule `value::coerce_assign` applies at a net store.
    ///
    /// The inline TASK path already gets this for free: it copies each input actual
    /// into a formal-WIDTH local net, so the store coerces. The two FUNCTION paths
    /// do not — the inline one substitutes the actual's ExprId for the formal's
    /// name (no net exists to coerce at) and the frame one hands the raw value to
    /// the slot in-bind — so both read the real's rounded value at the BODY's width
    /// instead of the formal's. Measured against both oracles:
    /// `function integer f(input byte k); f = k;` called `f(300.0)` gave 300 where
    /// iverilog and verilator both give 44 (300 truncated into a signed byte), and
    /// the same held for every narrow formal type and for a real variable, a real
    /// parameter, a real-returning call and a negated real. ⚠️ The INTEGER-actual
    /// twins were all already correct, which is why this reads as a real-domain
    /// gap rather than a formal-binding one.
    ///
    /// Returns `eid` unchanged for every shape it may not touch:
    ///
    ///   * a non-real actual (nothing to convert — the integral rules already ran),
    ///   * a width outside the cast's own scope (0, or > 64 where
    ///     `lower_real_to_int_cast` reports; converting there would trade a wrong
    ///     value for a NEW loud), and
    ///   * an actual that may not be REPEATED keeps the pre-slice answer here (the
    ///     inline bind routes it to `real_to_int_store` before calling this).
    pub(crate) fn coerce_real_actual_to_formal(
        &mut self,
        eid: u32,
        w: u32,
        formal_signed: bool,
    ) -> u32 {
        if w == 0 || w > 64 {
            return eid;
        }
        // ⚠️ VALUE-based realness, not `cast_operand_is_real`. That predicate's AST
        // half looks a BARE single-segment name up in `func_table`, which is a
        // different resolver than the one that picks the callee: inside a package
        // body a bare `g()` resolves to `P::g`, so a module-level real-returning
        // `g` SHADOWS it and an INTEGRAL actual is declared real. Round-tripping it
        // through f64 then loses everything past 2^53 — measured, `h(g())` with
        // `g` returning `64'd9007199254740993` gave …992 where both oracles give
        // …993, a correct→silent-wrong of this slice's own making. (The
        // `classifier-must-match-its-lowering-resolver` rule, and the sibling
        // bind's own comment already called this predicate "recognized by
        // spelling, not by value".)
        //
        // ⭐ Nothing is lost by asking the IR instead: the one shape
        // `cast_operand_is_real` sees that `expr_is_real` does not is a real-
        // returning `Expr::Call`, and `expr_is_repeatable` refuses a `Call`
        // anyway — so that shape never reached the cast. A real function whose
        // body INLINES to a real `Const` is seen by both.
        if !self.expr_is_real(eid) {
            return eid;
        }
        // An actual that may only be evaluated once keeps the pre-slice answer.
        if !self.expr_is_repeatable(eid) {
            return eid;
        }
        self.lower_real_to_int_cast(eid, w, formal_signed, false)
    }

    /// real → integral cast (IEEE 1800 §6.24.1 / §6.12.2): ROUND HALF AWAY FROM
    /// ZERO, then the low `tw` bits under the target's sign — `RealToInt`, which
    /// names the operand ONCE and is the engine's exact conversion (the low 128
    /// bits of the rounded integer), then `select_low` and the sign stamp.
    ///
    /// This replaced an IR-0 composition (`$floor`/`$ceil`/`$rtoi` over an
    /// `e >= 0.0 ? … : …` bump, a two-word split for a 33..=64-bit target) that
    /// named the operand 2 to 5 times and rounded through `$rtoi`, which
    /// SATURATES: `int'(rv)` with `rv = 1.0e40` was `ffffffff` (both oracles
    /// `00000000`), `longint'(rv)` was `ffffffff00000000`, and a real-returning
    /// frame call in the operand took the same path. Only the non-repeatable
    /// ≤32-bit operand had the single-mention node; now every operand has it. A
    /// >64-bit target cannot arise from a primitive cast.
    pub(crate) fn lower_real_to_int_cast(
        &mut self,
        e: u32,
        tw: u32,
        tsigned: bool,
        _t2state: bool,
    ) -> u32 {
        if tw > 64 {
            self.error(
                MsgCode::ElabUnsupported,
                "real→integer cast wider than 64 bits is outside the cast scope",
            );
            return self.placeholder_expr();
        }
        let rti = self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::RealToInt,
            args: vec![e],
        });
        let low = self.select_low(rti, tw);
        let which = if tsigned {
            ir::SysFuncId::Signed
        } else {
            ir::SysFuncId::Unsigned
        };
        self.push_expr(ir::Expr::SysFunc {
            which,
            args: vec![low],
        })
    }

    /// True when the cast operand is real-valued. Reuses the IR `expr_is_real` for
    /// the common shapes, and additionally recognizes a user real-returning function
    /// call REACHABLE through the real-propagating positions (unary `+/-`, real
    /// arithmetic, ternary, parens) — the IR `Call` node carries no real flag, so
    /// `expr_is_real` alone mis-treats `int'(real_fn())` / `int'(-real_fn())` as
    /// integral (a bit-reinterpret) instead of routing through the real→int round
    /// path. The engine itself evaluates the real arithmetic correctly (the return
    /// net is `NetKind::Real`); only this elaborate routing decision needs the call.
    pub(crate) fn cast_operand_is_real(&self, operand: &ast::Expr, eid: u32) -> bool {
        self.expr_is_real(eid) || self.ast_has_real_call(operand)
    }

    /// Static real-ness of an already-lowered ExprId (for §6.2 illegality gates
    /// and the §4.1a format-string check).
    ///
    /// The RULE lives in `sim_ir::realness` because the ENGINE needs the same
    /// answer — §11.8.1 makes the integral-to-real conversion boundary
    /// self-determined, so the engine has to know a binary is REAL before it
    /// evaluates either operand, and a second spelling of "what is real" is how
    /// the two crates drift. This is the elaborate-side driver: it recurses,
    /// because its arena is still growing while it lowers.
    pub(crate) fn expr_is_real(&self, eid: u32) -> bool {
        if self.hier_shape(eid).is_some_and(|h| h.real) {
            return true;
        }
        let cx = ir::realness::RealnessCtx {
            exprs: &self.exprs,
            consts: &self.consts,
            nets: &self.nets,
            real_elem_dyn_nets: &self.real_elem_dyn_nets,
            func_ret_is_real: &|f: u32| {
                self.func_metas
                    .get(f as usize)
                    .and_then(|m| self.nets.get((m.base_net + m.return_slot) as usize))
                    .is_some_and(|n| matches!(n.kind, ir::NetKind::Real))
            },
        };
        ir::realness::expr_is_real_node(&cx, &|id| self.expr_is_real(id), eid)
    }

    /// §4.1a STATIC gate: walk the literal format string, pair each conversion
    /// specifier with its positional value-arg, and reject a `%b/%h/%o/%x` (radix)
    /// specifier on a real-typed argument. `%f/%g/%e/%d` on a real are legal.
    pub(crate) fn check_format_real_radix(&mut self, fmt: &str, arg_ids: &[u32]) {
        let mut chars = fmt.chars().peekable();
        let mut argi = 0usize;
        while let Some(c) = chars.next() {
            if c != '%' {
                continue;
            }
            // skip width/precision modifiers (digits and a single '.').
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() || d == '.' {
                    chars.next();
                } else {
                    break;
                }
            }
            let spec = match chars.next() {
                Some(s) => s,
                None => break,
            };
            match spec {
                '%' | 'm' => {} // literal '%' / scope name — consume no arg
                'b' | 'B' | 'h' | 'H' | 'x' | 'X' | 'o' | 'O' => {
                    if arg_ids
                        .get(argi)
                        .copied()
                        .is_some_and(|e| self.expr_is_real(e))
                    {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "binary/hex/octal format not defined on a real argument",
                        );
                    }
                    argi += 1;
                }
                // every other conversion consumes one positional argument.
                _ => {
                    argi += 1;
                }
            }
        }
    }

    /// A real (f64) literal Const expr — e.g. `"100.0"` (N5 coverage real %).
    pub(crate) fn real_const_expr(&mut self, raw: &str) -> u32 {
        let cid = self.intern_const(parse_real_literal(raw));
        self.push_expr(ir::Expr::Const { val: cid })
    }

    /// v9 rank 6: `ok = $cast(dst, src)` (function form) — the `$value$plusargs`
    /// family (the engine writes the `dst` ref arg in the WRITE phase, returns 1).
    /// `dst` must lower to a plain whole-net Signal; `src` is any expression.
    /// iverilog 13.0 does not support `$cast` (no oracle): hand-IEEE §6.24.2,
    /// integral assignment always succeeds. (The task form is in `map_systask`.)
    pub(crate) fn cast_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        let ast::ExprKind::SysCall { name, args } = &rhs.kind else {
            return false;
        };
        if name.name != "$cast" {
            return false;
        }
        if args.len() != 2 {
            self.error(MsgCode::ElabUnsupported, "$cast takes (dest, source)");
            return true;
        }
        let dst_id = self.lower_expr(&args[0]);
        // ⭐ The destination is WRITTEN, so it owes the read-only funnel like every
        // other write position — review measured `$cast(cb.s, 8'hAA)` moving a
        // clocking holding net at exit 0 where verilator refuses it, and this is the
        // guard `systask.rs` cites as the model its own `$sformat` check mirrors.
        if let Some(ir::Expr::Signal { net, word: None }) = self.exprs.get(dst_id as usize) {
            let net = *net;
            if self.deny_readonly_write_at(net, dst_id, "$cast into") {
                return true;
            }
        }
        if !matches!(
            self.exprs.get(dst_id as usize),
            Some(ir::Expr::Signal { word: None, .. })
        ) {
            self.error(
                MsgCode::ElabUnsupported,
                "$cast destination must be a plain integral variable (v9 subset)",
            );
            return true;
        }
        let src_id = self.lower_expr(&args[1]);
        let rhs_id = self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::Cast,
            args: vec![dst_id, src_id],
        });
        self.emit_blocking_intercept(b, lhs, delay, rhs_id);
        true
    }
}
