//! strings — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// r19: does `e` mention a HIERARCHICAL (cross-instance) name anywhere?
    ///
    /// Used as a REJECT gate, so under-detection would leave a silent-wrong: the match
    /// is `_`-free-exhaustive over `ExprKind` on purpose, which makes a future variant
    /// a compile error instead of a silently-widened gate.
    ///
    /// A 2-segment head is NOT automatically hierarchical. The parser encodes a method
    /// call `q.size()` / `p.substr(0,1)` as `Call{name: HierPath[q, size]}`, the same
    /// shape as a genuine cross-instance call `u.f()`, and a struct member read `r.f`
    /// is a 2-segment `Ident` like `u.p`. They are told apart the way the METHOD
    /// LOWERING tells them apart — whether the head resolves to a local net — so this
    /// classifier cannot disagree with the expression it is classifying. Resolving
    /// otherwise false-rejected `'{$sformatf("v%0d", q.size()), …}`, a common idiom.
    /// A `pkg::name` reference is likewise not hierarchical (elaboration-resolved).
    fn head_is_cross_instance(&self, segs: &[ast::Ident]) -> bool {
        segs.len() > 1 && self.lookup_net_scoped(&segs[0].name).is_none()
    }

    pub(crate) fn expr_mentions_hier_path(&self, e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        let any = |v: &[ast::Expr]| v.iter().any(|x| self.expr_mentions_hier_path(x));
        match &e.kind {
            K::Ident(p) => self.head_is_cross_instance(&p.segments),
            // leaves
            K::IntLit { .. }
            | K::RealLit { .. }
            | K::StrLit { .. }
            | K::Null
            | K::Dollar
            | K::Error => false,
            // `TimeLit` is NOT a leaf — it holds `num: Box<Expr>`. Recursing matches the
            // other `_`-free walkers in the tree (`expr_has_call`); grouping it with the
            // leaves would be an under-detection in a gate whose failure mode is silent.
            K::TimeLit { num, .. } => self.expr_mentions_hier_path(num),
            // a package-scoped constant is elaboration-resolved, not a child-instance read
            K::PkgScoped { .. } => false,
            K::Unary { operand, .. } => self.expr_mentions_hier_path(operand),
            K::Paren { inner } => self.expr_mentions_hier_path(inner),
            // The discarded `target` can hold `Size(Expr)` / `Named(HierPath)`, but both
            // are elaboration-time types/constants — never a t0 value read out of a child
            // instance — and a hierarchical one is loud-rejected downstream anyway.
            K::Cast { expr, .. } => self.expr_mentions_hier_path(expr),
            K::NamedArg { value, .. } => value
                .as_deref()
                .is_some_and(|v| self.expr_mentions_hier_path(v)),
            K::Binary { lhs, rhs, .. } => {
                self.expr_mentions_hier_path(lhs) || self.expr_mentions_hier_path(rhs)
            }
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.expr_mentions_hier_path(cond)
                    || self.expr_mentions_hier_path(then_e)
                    || self.expr_mentions_hier_path(else_e)
            }
            K::BitSelect { base, index } => {
                self.expr_mentions_hier_path(base) || self.expr_mentions_hier_path(index)
            }
            K::PartSelect { base, msb, lsb } => {
                self.expr_mentions_hier_path(base)
                    || self.expr_mentions_hier_path(msb)
                    || self.expr_mentions_hier_path(lsb)
            }
            K::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                self.expr_mentions_hier_path(base)
                    || self.expr_mentions_hier_path(offset)
                    || self.expr_mentions_hier_path(width)
            }
            K::Concat { parts } | K::AssignPattern(parts) => any(parts),
            K::Replicate { count, value } => self.expr_mentions_hier_path(count) || any(value),
            K::MinTypMax { min, typ, max } => {
                self.expr_mentions_hier_path(min)
                    || self.expr_mentions_hier_path(typ)
                    || self.expr_mentions_hier_path(max)
            }
            K::Call { name, args } => self.head_is_cross_instance(&name.segments) || any(args),
            K::MethodCall { recv, args, .. } => self.expr_mentions_hier_path(recv) || any(args),
            K::SysCall { args, .. } => any(args),
            // Conservative: these carry constraint/iterator bodies this walker does not
            // model, so treat them as possibly hierarchical rather than assume not.
            K::RandomizeWith { .. }
            | K::ArrayMethodWith { .. }
            | K::New { .. }
            | K::ClassNew { .. }
            | K::Dist { .. } => true,
        }
    }

    /// N6: resolve a string-array ELEMENT read/write `NAME[K]` — if `base` is an Ident
    /// naming a fixed string array in scope and `index` folds to a CONST in range,
    /// return the K-th element net. A runtime index / out-of-range / non-string-array
    /// base returns None (the caller falls through / loud-rejects). Shared by the read
    /// (`lower_expr`) and write (`lvalue`) paths so both resolve identically.
    pub(crate) fn string_array_elem_net(
        &mut self,
        base: &ast::Expr,
        index: &ast::Expr,
    ) -> Option<u32> {
        let ast::ExprKind::Ident(path) = &base.kind else {
            return None;
        };
        if path.segments.len() != 1 {
            return None;
        }
        let key = self.walk_scopes_key_shadowed(&path.segments[0].name, |k| {
            self.string_array_elems.contains_key(k)
        })?;
        let (min, max, elems) = self.string_array_elems.get(&key)?.clone();
        let k = self.const_eval_in_scope(index)?;
        if k < min || k > max {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("string-array index {k} is out of the declared range [{min}:{max}]"),
            );
            return None;
        }
        Some(elems[(k - min) as usize])
    }

    /// N6: true iff `base` is a fixed string-array Ident in scope (so a runtime-index
    /// element access can be loud-rejected rather than mis-routed).
    pub(crate) fn is_string_array_base(&self, base: &ast::Expr) -> bool {
        matches!(&base.kind, ast::ExprKind::Ident(p) if p.segments.len() == 1
            && self.walk_scopes_key_shadowed(&p.segments[0].name, |k| self.string_array_elems.contains_key(k)).is_some())
    }

    /// r19: true iff `base` names a string ARRAY — a FIXED one (`string s[N]`, whose
    /// elements are the per-element `<name>$sae$<i>` nets) or a DYNAMIC one (`string
    /// s[]`, a `DynArray` net marked in `string_elem_dyn_nets`). That makes `base[i]`
    /// a string ELEMENT (its declared element type is `string`, IEEE §7.4), NOT the
    /// §6.16.2 BYTE select that `base[i]` means on a SCALAR `string`.
    ///
    /// Not a `NetKind::String` handle either way — a fixed string array registers no
    /// net under the ARRAY's own name (only the element nets) and a dynamic one is a
    /// `DynArray` net — so neither can reach `string_handle` / `string_base_expr_net`
    /// and a scalar-string byte select is never captured here.
    ///
    /// **Each clause resolves the name with the SAME machinery its own lowering uses**,
    /// so this classifier cannot disagree with the expression it is classifying: the
    /// fixed clause shares `walk_scopes_key` with the element-net read/write paths, and
    /// the dynamic clause goes through `dyn_handle_read` — which consults the `dyn_subst`
    /// ALIAS first. Resolving the dynamic clause with a plain `lookup_net_scoped` instead
    /// was a silent-wrong: inside an R2-inlined body a dyn-array formal SHADOWS an outer
    /// same-named net, so a module-level `string b[]` made an inlined `int b[]` formal's
    /// `b[0] < b[1]` compare as text (256 < 255 answered 1, iverilog 0).
    pub(crate) fn is_string_array_elem_base(&self, base: &ast::Expr) -> bool {
        // Zero cost for every design that declares no string array at all: the arm can
        // only fire on a hit in one of these two containers, so the whole classifier
        // stays byte-identical there without walking any scope chain.
        if self.string_array_elems.is_empty() && self.string_elem_dyn_nets.is_empty() {
            return false;
        }
        if self.is_string_array_base(base) {
            return true;
        }
        matches!(&base.kind, ast::ExprKind::Ident(p) if p.segments.len() == 1
            && self
                .dyn_handle_read(&p.segments[0].name)
                .is_some_and(|(n, _)| self.string_elem_dyn_nets.contains(&n)))
    }

    /// N5: the raw string literal of a string-valued parameter initializer, or None.
    /// Unwraps a parenthesised value (`= ("abc")`). A non-StrLit value (numeric, an
    /// Ident, …) returns None → the numeric fold path (unchanged).
    pub(crate) fn param_str_literal(value: &ast::Expr) -> Option<String> {
        match &value.kind {
            ast::ExprKind::StrLit { raw } => Some(raw.clone()),
            ast::ExprKind::Paren { inner } => Self::param_str_literal(inner),
            _ => None,
        }
    }

    /// r19: the folded f64 of a REAL-valued parameter initializer, or None.
    ///
    /// A parameter is real when its DECLARED type says so (`parameter real R = 3;` is
    /// a real parameter even though its initializer is an integer literal — keying on
    /// the initializer's literal form instead made `R/2` integer-divide to 1.0 where
    /// IEEE gives 1.5), or when an untyped parameter is initialised with a real
    /// literal (IEEE §6.20.2: the type follows the value).
    ///
    /// Returns the VALUE. An earlier version returned raw text and negated by string
    /// concatenation, which produced `"--1.25"` for `-(-1.25)` — unparseable, and
    /// `parse_real_literal` maps that to a silent 0.0.
    ///
    /// LITERAL forms only (through parens and unary `+`/`-`). Real ARITHMETIC has no
    /// const-fold path, so it returns None and the caller stays loud — a wrong
    /// parameter value poisons everything downstream with no trace.
    pub(crate) fn param_real_value(
        &self,
        ty: &ast::ParamType,
        value: &ast::Expr,
    ) -> Option<(f64, Option<i64>)> {
        let declared_real = matches!(ty, ast::ParamType::Real | ast::ParamType::Realtime);
        if !declared_real && !Self::mentions_real_literal(value) {
            return None;
        }
        // A real LITERAL (through parens / unary sign) folds here. Otherwise, when the
        // DECLARED type is real, fall back to the integer const-fold — that is what
        // makes `parameter real R = 3;` (and `= 3+1`) bind as 3.0 / 4.0 rather than as
        // an i64 param whose `R/2` would integer-divide. Real ARITHMETIC reaches
        // neither and returns None ⇒ the caller stays loud. The i64 is handed back so
        // the caller can ALSO register it in `params`: `parameter real R = 4;` must keep
        // every integral capability an ordinary i64 param had (`logic [R-1:0]`,
        // `generate if (R > 2)`, `arr [R]`, integer overrides) — binding it real-only
        // took a byte-correct design loud, which is a descent down the accuracy ladder.
        if let Some(f) = Self::real_literal_value(value) {
            // NO i64 twin. Giving an exactly-integral real literal one buys spelling
            // parity (`real R = 4` can size `logic [R-1:0]`; `real R = 4.0` cannot),
            // and it was tried — but a twin makes the INTEGER const domain succeed on
            // an expression that mentions a real, and `param_real_value` is the only
            // site that applies the §11.8.1 "any real operand ⇒ real domain" ordering.
            // Every other const-evaluation context (generate-if / generate-for
            // control, a generate-scope `localparam real`, an integral-typed
            // localparam, any expression the real domain declines) then answered from
            // the truncated integer instead: `generate if (R/2 > 2)` with R = 5.0 took
            // the ELSE branch, and `localparam real X = R/2` inside a generate bound
            // 2.0. Parity is not worth five silent-wrongs; the asymmetry is recorded
            // in ROADMAP §3 and closing it means routing those sites through the real
            // domain, which is its own slice.
            return Some((f, None));
        }
        // §11.8.1: an operation with ANY real operand is evaluated in the REAL
        // domain. So the moment the initializer mentions a real — a literal or a
        // real-declared parameter — the real fold must run BEFORE the integer one.
        // Order matters and was measured: once an exactly-integral real parameter
        // gained its i64 twin, `localparam real HALF = CLK/2;` (CLK = 5.0) became
        // foldable by the INTEGER domain, which answered 2 instead of 2.5. The
        // integer fold is for a wholly integral initializer only.
        if self.expr_mentions_real(value) {
            // Real arithmetic binds its f64 and NO integer twin, for the reason
            // above: a twin here would let the integer domain answer this same
            // expression elsewhere, truncating it.
            let f = self.const_eval_real_in_scope(value)?;
            return Some((f, None));
        }
        // A wholly integral initializer of a DECLARED-real parameter converts to
        // real at its own self-determined size (§11.8.1 — the real target gives it
        // no integral context): `parameter real P = 4'd15 + 4'd1` binds 0.0 (the
        // 4-bit sum wraps; iverilog agrees), not 16.0. The i64 twin registers the
        // SAME self-determined value, so the parameter's integral capabilities
        // cannot disagree with its real reading.
        declared_real
            .then(|| self.const_int_selfdet(value))
            .flatten()
            .map(|v| (v as f64, Some(v)))
    }

    /// Does `e` mention a real value — a real literal, or a name bound in the real
    /// parameter map? Decides whether §11.8.1 puts the whole expression in the real
    /// domain. Structural and conservative: a shape it cannot see through answers
    /// false and the integer path stays in charge, exactly as before.
    pub(crate) fn expr_mentions_real(&self, e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        match &e.kind {
            K::RealLit { .. } => true,
            K::Ident(p) if p.segments.len() == 1 => self
                .walk_scopes(&p.segments[0].name, &self.real_param_val)
                .is_some(),
            K::Paren { inner } => self.expr_mentions_real(inner),
            K::Unary { operand, .. } => self.expr_mentions_real(operand),
            K::Binary { lhs, rhs, .. } => {
                self.expr_mentions_real(lhs) || self.expr_mentions_real(rhs)
            }
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.expr_mentions_real(cond)
                    || self.expr_mentions_real(then_e)
                    || self.expr_mentions_real(else_e)
            }
            K::SysCall { args, .. } | K::Call { args, .. } => {
                args.iter().any(|a| self.expr_mentions_real(a))
            }
            _ => false,
        }
    }

    /// r19: is `e` a REAL literal (through parens / unary sign)? Used by the override
    /// path, where there is no declared type to consult — only the expression.
    pub(crate) fn expr_is_real_literal(e: &ast::Expr) -> bool {
        Self::real_literal_value(e).is_some()
    }

    /// The value of a REAL literal initializer, through parens and unary `+`/`-`.
    /// Negation is applied to the parsed f64 — negating the raw TEXT produced
    /// `"--1.25"` for `-(-1.25)`, which parses to a silent 0.0.
    fn real_literal_value(e: &ast::Expr) -> Option<f64> {
        match &e.kind {
            ast::ExprKind::RealLit { raw, .. } => Some(parse_real_f64(raw)),
            ast::ExprKind::Paren { inner } => Self::real_literal_value(inner),
            ast::ExprKind::Unary { op, operand } => {
                let v = Self::real_literal_value(operand)?;
                match op {
                    ast::UnOp::Plus => Some(v),
                    ast::UnOp::Minus => Some(-v),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Does the initializer contain a REAL literal (so an UNTYPED parameter takes the
    /// real type from its value, IEEE §6.20.2)?
    fn mentions_real_literal(e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::RealLit { .. } => true,
            ast::ExprKind::Paren { inner } => Self::mentions_real_literal(inner),
            ast::ExprKind::Unary { operand, .. } => Self::mentions_real_literal(operand),
            _ => false,
        }
    }

    /// A fresh `String` net for hoisting a `$sformatf` operand to a temp
    /// (`$sfmt_tmp$<n>` — the leading `$` keeps it collision-proof against user
    /// identifiers, and a `String` net has no VCD `$var` form so the temp is
    /// invisible to dumps). Returns the name so a synthetic `Ident` lvalue/expr
    /// can reference it (module-scoped via `fq`, like `fresh_sva_reg`).
    pub(crate) fn fresh_string_temp(&mut self) -> String {
        let name = format!("$sfmt_tmp${}", self.nets.len());
        let nv = ir::NetVar {
            kind: ir::NetKind::String,
            width: 0,
            msb: 0,
            lsb: 0,
            signed: false,
            array_len: 0,
            dir: ir::PortDir::Internal,
            init: default_init(ast::NetVarKind::Reg, 1),
        };
        self.add_net(&name, nv);
        name
    }

    /// v7 P2-C: `name` (single segment, current scope) as a STRING net.
    pub(crate) fn string_handle(&self, name: &str) -> Option<u32> {
        let n = self.lookup_net_scoped(name)?;
        (self.nets.get(n as usize)?.kind == ir::NetKind::String).then_some(n)
    }

    /// The STRING net denoted by a single-segment ident expression (the base of a
    /// `s[i]` element select), or `None` for anything else (a packed vector, an
    /// array element, a hierarchical ref). Conservative on purpose — only a bare
    /// string variable carries the byte-sequence semantics §6.16.2/3 want.
    pub(crate) fn string_base_expr_net(&self, base: &ast::Expr) -> Option<u32> {
        match &base.kind {
            ast::ExprKind::Ident(p) => match p.segments.as_slice() {
                [seg] => self.string_handle(&seg.name),
                _ => None,
            },
            _ => None,
        }
    }

    /// String element READ `s[i]` (IEEE §6.16.2) → the `.getc(i)` byte primitive
    /// (front-indexed character, 0 if out of range, 8-bit), when `base` is a bare
    /// string variable. `None` ⇒ not a string element select → the caller falls
    /// through to the packed bit-select (byte-identical for non-string bases).
    pub(crate) fn string_index_read(&mut self, base: &ast::Expr, index: &ast::Expr) -> Option<u32> {
        let handle = if let Some(net) = self.string_base_expr_net(base) {
            self.push_expr(ir::Expr::Signal { net, word: None })
        } else if matches!(&base.kind, ast::ExprKind::Ident(p) if p.segments.len() == 1)
            // r19: …or a string-ARRAY ELEMENT base (`sa[i][j]` — element, then §6.16.2
            // byte). `string_base_expr_net` sees only a bare string NET, so this used to
            // fall through to a packed bit-select on a width-0 String signal and read a
            // silent 0 (`s[0]="world"; s[0][0]` gave 0, iverilog 119). The WRITE twin
            // (`s[0][0]="W"`) stays loud ("nested lvalue select"), so read and write do
            // not diverge here.
            || matches!(&base.kind, ast::ExprKind::BitSelect { .. })
        {
            // INLINE-function path: a string LOCAL or FORMAL `s` has no real string
            // NET (`string_base_expr_net` finds only module/scoped nets), so `s[i]`
            // used to fall through to a packed BIT-select on the subst-bound string
            // value — silently 0 (bit 0 of a handle) instead of the byte. `expr_is_
            // string_ast` recognizes an inline string (a declared-`string` formal or
            // local via `formal_str`, or a subst value that is a string) exactly as
            // the string compare/method paths do; lower the base and reuse the
            // `.getc(i)` byte primitive. Checked BEFORE lowering, so a non-string base
            // is never lowered here.
            if !self.expr_is_string_ast(base) {
                return None;
            }
            let h = self.lower_expr(base);
            // Route to `.getc` ONLY when the lowered base is a StrGetC-consumable
            // string HANDLE — a Signal-to-string-net (a runtime-sourced inline local)
            // or a string Const (a literal-bound local/formal) — exactly the forms the
            // engine `handle_str_bytes` reads. A FRAME string formal is a 1-bit WIRE
            // net (its literal actual lives in a heap slot via the str-arg mask, not
            // byte-readable here), and a string-SysFunc actual (`s.substr(..)`) is not
            // byte-readable either; both fall through to the unchanged bit-select
            // (byte-identical to the prior behavior — the frame-path string-select is
            // a separate deferred gap, NOT regressed here).
            if !self.handle_is_str_readable(h) {
                return None;
            }
            h
        } else {
            return None;
        };
        let idx = self.lower_index_expr(index);
        Some(self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::StrGetC,
            args: vec![handle, idx],
        }))
    }

    /// A `StrGetC`/`handle_str_bytes`-consumable string handle. The caller has
    /// already confirmed the BASE is string-domain (`expr_is_string_ast`), so any
    /// word-less `Signal` is a genuine string operand — either a real string net
    /// (read via `str_bytes`) OR a frame string formal, which lowers to a 1-bit wire
    /// whose frame slot holds the materialized string Value (the engine's
    /// `handle_str_bytes` reads that via an eval fallback). A string `Const` (a
    /// literal-bound inline local) is likewise readable. A string-producing `SysFunc`
    /// actual (`s.substr(..)`) is deliberately NOT accepted here — the engine byte
    /// path does not eval it, so it falls through to the unchanged bit-select (a
    /// separate follow-on), not a silent-X.
    ///
    /// r19: a WORD-INDEXED `Signal` on a dyn net whose ELEMENTS are strings
    /// (`string_elem_dyn_nets`) is readable too — the engine's `handle_str_bytes`
    /// falls back to `eval` for exactly this shape, the same path `%s` and a compare
    /// on `sa[i]` already take. Rejecting it made a byte select on a dynamic
    /// string-array element (`d[0][0]`) fall through to a packed bit-select and read a
    /// silent 0, while the byte-identical FIXED-array form (`s[0][0]`) correctly gave
    /// the iverilog-verified 119.
    ///
    /// Why the discriminator is sound: `string_elem_dyn_nets` is the SAME set that
    /// drives the engine's `dyn_str_elem` flag, which is what makes those elements be
    /// stored as byte-strings in the first place. So the gate cannot admit a net whose
    /// elements the engine does not hold as strings — a stronger argument than "the
    /// set happens to exclude `int d[]`". Being keyed on the ALREADY-LOWERED node's
    /// net id, it also cannot disagree with the lowering it is classifying.
    ///
    /// Load-bearing invariant: the element Value carries `is_str`, and
    /// `resize_keep_sign` returns early on `is_str`. A string dyn net has width 0, so
    /// its self-width would otherwise truncate the byte string to 1 bit. Anything that
    /// stores a dyn string element WITHOUT `Value::from_str_bytes` would silently
    /// collapse this byte select.
    ///
    /// The WRITE twin stays loud in every SELECT form: a bit select (`d[0][0] = "W"`)
    /// is "nested lvalue select", and a part select (`d[0][3:0] = …`, including a
    /// runtime offset) is rejected in `lval_part_base` — a string element has no
    /// bit-addressable storage, and letting it through wrote the wrong byte silently.
    /// A WHOLE element inside a concat lvalue (`{d[0], x} = …`) is caught by the
    /// `lower_lvalue` funnel guards, which key on the same
    /// `is_non_bit_addressable_target` predicate — all THREE guard sites share it, so
    /// the policy cannot drift between them.
    pub(crate) fn handle_is_str_readable(&self, eid: u32) -> bool {
        match self.exprs.get(eid as usize) {
            Some(ir::Expr::Signal { word: None, .. }) | Some(ir::Expr::Const { .. }) => true,
            Some(ir::Expr::Signal { net, word: Some(_) }) => {
                self.string_elem_dyn_nets.contains(net)
            }
            _ => false,
        }
    }

    /// v7 P2-C: does this AST expression denote a STRING-domain value?
    /// (a string variable read, or a string-producing method call). Literals
    /// stay packed — a string-vs-literal comparison routes via the string
    /// side. Conservative: anything else is not string-domain.
    pub(crate) fn expr_is_string_ast(&self, e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::Paren { inner } => self.expr_is_string_ast(inner),
            ast::ExprKind::Ident(p) => match p.segments.as_slice() {
                [seg] => {
                    // A formal DECLARED `string` is string-domain regardless of its
                    // actual: an inline LITERAL actual is a packed const (not a string
                    // net, so the `subst`/`ir_expr_is_string` path below misses it), and
                    // a FRAME formal is a scoped net not in `subst`. Keyed on the formal's
                    // declared type (NOT the actual's) so a string literal passed to a
                    // PACKED formal still compares packed. Checked FIRST (innermost-wins).
                    if self.formal_is_string(&seg.name) {
                        return true;
                    }
                    // review F2: an inlined FORMAL bound to a string actual
                    // must keep its string-domain-ness — resolve through the
                    // subst (the bypass lowered `a < b` as a packed compare,
                    // non-lexicographic for unequal lengths).
                    if let Some(eid) = self.subst_lookup(&seg.name) {
                        return self.ir_expr_is_string(eid);
                    }
                    if let Some(net) = self.out_subst_lookup(&seg.name) {
                        return self.is_string_net(net);
                    }
                    self.lookup_scoped(&seg.name).is_none()
                        && self.string_handle(&seg.name).is_some()
                }
                _ => false,
            },
            ast::ExprKind::Call { name, .. } => {
                // G4: a call to a user function declared `function string f(...)` is a
                // string-domain value, so `f(x) == "…"` lowers as a string compare.
                if name.segments.len() == 1 {
                    if let Some(f) = self.func_table.get(&name.segments[0].name) {
                        if f.ret_string {
                            return true;
                        }
                    }
                }
                name.segments.len() == 2
                    && self.string_handle(&name.segments[0].name).is_some()
                    && matches!(
                        name.segments[1].name.as_str(),
                        "substr" | "toupper" | "tolower"
                    )
            }
            // §6.16: `$sformatf(...)` RETURNS a string, so it is a string-domain operand
            // — a concat containing one must take the string path, and a comparison with
            // one must compare lexicographically. `ir_expr_is_string` already says so at
            // the IR level; the AST classifier did not, so `{"x", $sformatf(…)}` (no OTHER
            // string part) fell to the PACKED concat and rendered the wrong bytes wherever
            // the statement-level hoist could not rewrite it first.
            ast::ExprKind::SysCall { name, .. } => name.name == "$sformatf",
            // r19: a string-ARRAY ELEMENT (`sa[i]`) is itself a string-domain value.
            // Without this arm an element fell through to the PACKED lowering in every
            // context that gates on this classifier — concatenation, replication and the
            // relational compares — and its bytes were silently dropped: `{sa[0],"!"}`
            // rendered "!" (not "abc!"), `{2{sa[0]}}` rendered "", and `sa[0] < sa[1]`
            // compared MSB-extended bits instead of lexicographically (§6.16). A byte
            // select on a SCALAR string keeps the packed 8-bit meaning — see
            // `is_string_array_elem_base` for why the two can never overlap.
            ast::ExprKind::BitSelect { base, .. } => self.is_string_array_elem_base(base),
            _ => false,
        }
    }

    /// v7 P2-C: is `net` a string variable?
    pub(crate) fn is_string_net(&self, net: u32) -> bool {
        self.nets.get(net as usize).map(|n| n.kind) == Some(ir::NetKind::String)
    }

    /// r19: does a write to `net` land on storage with NO bit-addressable
    /// representation — a scalar `NetKind::String` net, a dyn STRING element, or a dyn
    /// REAL element? For all three, a partial or positional write cannot be expressed:
    /// the engine re-derives the whole value (from its byte string, or as an f64) and
    /// silently discards the offset/width, or writes an empty piece.
    ///
    /// ONE predicate behind EVERY guard that rejects such a write — the two
    /// `lower_lvalue` funnel guards and the earlier `lval_part_base` rejection — so the
    /// scalar / fixed / dynamic forms cannot drift apart. They did drift: keyed on
    /// `is_string_net` alone, the funnel guards were blind to a dyn element's
    /// `DynArray` net, so `{d[0], x} = …` silently emptied the element while the scalar
    /// and fixed twins were loud.
    /// Note this includes a scalar `NetKind::String` net even though `p[0] = "W"` IS
    /// supported: that plain byte write is intercepted earlier and routed to `.putc`,
    /// so it never reaches these guards.
    pub(crate) fn is_non_bit_addressable_target(&self, net: u32) -> bool {
        self.is_string_net(net)
            || self.string_elem_dyn_nets.contains(&net)
            || self.real_elem_dyn_nets.contains(&net)
    }

    /// v7 P2-C: does an already-LOWERED expr denote a string-domain value?
    /// (subst-bound formals resolve here — review F2.)
    pub(crate) fn ir_expr_is_string(&self, eid: u32) -> bool {
        match self.exprs.get(eid as usize) {
            Some(ir::Expr::Signal { net, word: None }) => self.is_string_net(*net),
            // r18: a string dyn-array/queue ELEMENT read `sa[i]` — a word-indexed `Signal`
            // on a dyn net whose ELEMENTS are strings (`string_elem_dyn_nets`; the container
            // net kind is Queue/DynArray, not `String`). The element is itself a string, so
            // a method chains on it (`sa[i].substr(a,b)`, and `rec.name.substr(..)` via the
            // SoA member net `$unp$arr$name[i]`). The engine's `handle_str_bytes` reads a
            // word-indexed string signal through its `eval` fallback (the same path that
            // renders `%s` / compares `sa[i]`).
            Some(ir::Expr::Signal { net, word: Some(_) }) => {
                self.string_elem_dyn_nets.contains(net)
            }
            Some(ir::Expr::SysFunc { which, .. }) => matches!(
                which,
                ir::SysFuncId::StrSubstr
                    | ir::SysFuncId::StrToUpper
                    | ir::SysFuncId::StrToLower
                    | ir::SysFuncId::Sformatf
            ),
            _ => false,
        }
    }

    /// v7 P2-C: method-call EXPRESSION on a string (`s.len()`, `s.substr(i,j)`,
    /// `s.toupper()`, `s.getc(i)`, `s.compare(t)`). `putc` is the statement
    /// mutator (`lower_string_method_stmt`); unknown methods are loud.
    pub(crate) fn lower_string_method_expr(
        &mut self,
        net: u32,
        method: &str,
        args: &[ast::Expr],
    ) -> u32 {
        let handle = self.push_expr(ir::Expr::Signal { net, word: None });
        self.lower_string_method_expr_handle(handle, method, args)
    }

    /// G1: [`lower_string_method_expr`] over a PRE-LOWERED string handle — a string-net
    /// `Signal`, a frame string-FORMAL slot `Signal` (a 1-bit wire whose frame slot
    /// holds the materialized string, read via the engine's `handle_str_bytes` eval
    /// fallback), or a literal string `Const`. Lets a string method on a formal / inline
    /// local dispatch even though its handle is not a bare `NetKind::String` net. The
    /// caller must have confirmed the handle is string-readable (`handle_is_str_readable`).
    pub(crate) fn lower_string_method_expr_handle(
        &mut self,
        handle: u32,
        method: &str,
        args: &[ast::Expr],
    ) -> u32 {
        let arity_ok = |n: usize| args.len() == n;
        let which = match method {
            "len" if arity_ok(0) => ir::SysFuncId::StrLen,
            "getc" if arity_ok(1) => ir::SysFuncId::StrGetC,
            "substr" if arity_ok(2) => ir::SysFuncId::StrSubstr,
            "toupper" if arity_ok(0) => ir::SysFuncId::StrToUpper,
            "tolower" if arity_ok(0) => ir::SysFuncId::StrToLower,
            "compare" if arity_ok(1) => ir::SysFuncId::StrCmp,
            // ⓑ-breadth (v18): string→number conversions (IEEE §6.16.9-13).
            "atoi" if arity_ok(0) => ir::SysFuncId::StrAtoi,
            "atohex" if arity_ok(0) => ir::SysFuncId::StrAtohex,
            "atooct" if arity_ok(0) => ir::SysFuncId::StrAtooct,
            "atobin" if arity_ok(0) => ir::SysFuncId::StrAtobin,
            "atoreal" if arity_ok(0) => ir::SysFuncId::StrAtoreal,
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "string method `{method}` (with this arity) is outside the scope \
                         (len/getc/substr/toupper/tolower/compare/putc/atoi/atohex/atooct/\
                         atobin/atoreal)"
                    ),
                );
                return self.placeholder_expr();
            }
        };
        let mut ids = vec![handle];
        ids.extend(args.iter().map(|a| self.lower_index_expr(a)));
        self.push_expr(ir::Expr::SysFunc { which, args: ids })
    }

    /// v7 P2-C: method-call STATEMENT on a string — `s.putc(i, c);`.
    pub(crate) fn lower_string_method_stmt(
        &mut self,
        b: &mut ProcessBuilder,
        net: u32,
        method: &str,
        args: &[ast::Expr],
    ) {
        // ⓑ-breadth (v18): number→string conversions (IEEE §6.16.14-18) — in-place
        // mutators taking the value to render.
        let which = match (method, args.len()) {
            ("putc", 2) => ir::SysTaskId::StrPutC,
            ("itoa", 1) => ir::SysTaskId::StrItoa,
            ("hextoa", 1) => ir::SysTaskId::StrHextoa,
            ("octtoa", 1) => ir::SysTaskId::StrOcttoa,
            ("bintoa", 1) => ir::SysTaskId::StrBintoa,
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "string method statement `{method}` is outside the scope \
                         (putc/itoa/hextoa/octtoa/bintoa)"
                    ),
                );
                return;
            }
        };
        let handle = self.push_expr(ir::Expr::Signal { net, word: None });
        let mut ids = vec![handle];
        ids.extend(args.iter().map(|a| self.lower_index_expr(a)));
        let sid = self.push_stmt(ir::Stmt::SysTask {
            which,
            fmt: None,
            args: ids,
        });
        b.push_stmt_id(sid);
    }

    /// String element WRITE `s[i] = c` (IEEE §6.16.3): a string variable is a
    /// dynamic byte SEQUENCE, so the assignment mutates the front-indexed CHARACTER
    /// (byte) at `i` via the `.putc(i, c)` primitive (an OOB index or a NUL byte is a
    /// silent no-op per spec), NOT a packed bit-write into the materialized vector.
    /// `false` ⇒ `lhs` is not a bare-string element select → the normal lvalue path
    /// runs (byte-identical for every non-string lvalue).
    /// The STRING net of a bare-string element lvalue `s[i]`, or `None` for any
    /// other lvalue shape (a packed-vector bit-select, an array element, a concat).
    /// Shared by the blocking write and the non-blocking honest-loud guard.
    pub(crate) fn string_index_lvalue_net(&self, lhs: &ast::Lvalue) -> Option<u32> {
        let ast::Lvalue::BitSelect { base, .. } = lhs else {
            return None;
        };
        let ast::Lvalue::Ident(p) = &**base else {
            return None;
        };
        match p.segments.as_slice() {
            [seg] => self.string_handle(&seg.name),
            _ => None,
        }
    }

    /// Is `lhs` a bare-string element select `s[i]`? (predicate twin of
    /// `string_index_lvalue_net`, gating both the blocking write route and the
    /// non-blocking honest-loud guard).
    pub(crate) fn is_string_index_lvalue(&self, lhs: &ast::Lvalue) -> bool {
        self.string_index_lvalue_net(lhs).is_some()
    }

    /// Emit the plain blocking string element write `s[i] = c` as the `.putc(i, c)`
    /// byte primitive. PRECONDITION: `is_string_index_lvalue(lhs)` holds and the
    /// caller has already excluded event/delay control — so the shape match is
    /// infallible here (a defensive `else` keeps it total).
    pub(crate) fn string_index_write_putc(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        rhs: &ast::Expr,
    ) {
        let (Some(net), ast::Lvalue::BitSelect { index, .. }) =
            (self.string_index_lvalue_net(lhs), lhs)
        else {
            return;
        };
        let handle = self.push_expr(ir::Expr::Signal { net, word: None });
        let idx = self.lower_index_expr(index);
        let c = self.lower_expr(rhs);
        let sid = self.push_stmt(ir::Stmt::SysTask {
            which: ir::SysTaskId::StrPutC,
            fmt: None,
            args: vec![handle, idx, c],
        });
        b.push_stmt_id(sid);
    }

    /// String concatenation `lhs = {a, b, …}` or replication `lhs = {N{a, …}}`
    /// (IEEE §6.16 / §11.4.12) where at least one part is a string — desugared to
    /// the existing `$sformatf("%s%s…", a, b, …)` render path (the static `Concat`
    /// node cannot carry a dynamic-width string result). Only the direct-rhs-of-a-
    /// blocking-assign placement is handled (the dominant string-building pattern);
    /// a string concat in any other context stays loud (the `lower_expr` reject).
    /// Each part renders through `%s` — string vars/literals and integral byte
    /// values alike (IEEE §6.16: an integral concat element is its character
    /// bytes), matching iverilog. A `real` part is loud (no string segment).
    pub(crate) fn string_concat_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        let mut inner = rhs;
        while let ast::ExprKind::Paren { inner: i } = &inner.kind {
            inner = i;
        }
        // The ordered list of part expressions (a replication flattens to its
        // value list repeated `count` times). `None` ⇒ not a string concat.
        let part_exprs: Vec<&ast::Expr> = match &inner.kind {
            ast::ExprKind::Concat { parts } => {
                if !parts.iter().any(|p| self.expr_is_string_ast(p)) {
                    return false; // no string part ⇒ ordinary bit-concat (byte-identical)
                }
                parts.iter().collect()
            }
            ast::ExprKind::Replicate { count, value } => {
                if !value.iter().any(|p| self.expr_is_string_ast(p)) {
                    return false; // ordinary bit-replicate (byte-identical)
                }
                let Some(n) = self.const_eval_in_scope(count) else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a non-constant replication count on a string is unsupported",
                    );
                    return true;
                };
                // §11.4.12.2 reaches this arm too, and it is the one the generic guard
                // below cannot see: a string VARIABLE operand returns from here. A
                // negative count used to render an empty string at exit 0 while BOTH
                // oracles reject it. ZERO keeps its own meaning (legal as a direct
                // concatenation operand, §11.4.12.1).
                if n < 0 {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a replication count may not be negative (IEEE §11.4.12.2)",
                    );
                }
                let n = n.max(0) as usize;
                let mut v = Vec::with_capacity(n.saturating_mul(value.len()));
                for _ in 0..n {
                    v.extend(value.iter());
                }
                v
            }
            _ => return false,
        };
        if delay.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "an intra-assignment delay on a string concatenation is unsupported",
            );
            return true;
        }
        // §4.5.248: render any `$sformatf` PART to a temp first (`s = {s,
        // $sformatf("%02x", b[i])}` — the hex-string accumulator every testbench has).
        // Done here, after the string-concat decision, so a rhs this helper declines
        // never gets a stray temp emitted for it. Every part is evaluated exactly once
        // by the concat, so hoisting preserves both the count and the order.
        let hoisted: Vec<ast::Expr> = part_exprs
            .iter()
            .map(|p| {
                self.hoist_nested_sformatf(b, p)
                    .unwrap_or_else(|| (*p).clone())
            })
            .collect();
        let part_exprs: Vec<&ast::Expr> = hoisted.iter().collect();
        // §4.5.252: this is the DIRECT rhs of a string blocking assign, whose value is
        // rendered by `format_args_str` — so a `$sformatf` part the hoist declined
        // (inside a frame FUNCTION body, where the temp would have to be a module net)
        // may lower as a plain expression node here instead of staying loud.
        let saved_ok = std::mem::replace(&mut self.sformatf_expr_ok, true);
        let rhs_id = self.lower_string_concat_parts(&part_exprs);
        self.sformatf_expr_ok = saved_ok;
        let lv = self.lower_lvalue(lhs);
        self.check_lvalue_kind(&lv, true);
        let sid = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: lv,
            rhs: rhs_id,
        });
        b.push_stmt_id(sid);
        true
    }

    /// Shared string-concat lowering (IEEE §6.16): the ordered `part_exprs`
    /// (a replication already flattened to its repeated value list) lower to a
    /// `$sformatf("%s%s…%s", part0, part1, …)` expression — the parts pass as
    /// `%s` args, so each renders as its string/char bytes and a literal `%` in a
    /// part value is NOT a conversion. Returns the resulting ExprId.
    ///
    /// Used by BOTH the statement-level concat-ASSIGN desugar
    /// (`string_concat_special`) and the general expression path
    /// (`lower_expr`'s `Concat`/`Replicate` arms) so display args, comparisons,
    /// function/task args, and nested concats share one oracle-proven impl.
    ///
    /// A real part stays LOUD (correct-or-loud): the loud diagnostic is emitted
    /// and the remaining parts still lower (so the dead `$sformatf` node is
    /// well-formed), but the surrounding elaboration is already poisoned.
    pub(crate) fn lower_string_concat_parts(&mut self, part_exprs: &[&ast::Expr]) -> u32 {
        // fmt = "%s"×N (the parts pass as %s args, so a `%` in any part value is
        // rendered literally, not as a conversion).
        let fmt = "%s".repeat(part_exprs.len());
        let fmt_cid = self.intern_const(literal::str_const_from_bytes(fmt.as_bytes()));
        let fmt_eid = self.push_expr(ir::Expr::Const { val: fmt_cid });
        let mut args = vec![fmt_eid];
        for p in part_exprs {
            let pid = self.lower_expr(p);
            if self.expr_is_real(pid) {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a real value may not be a string-concatenation element",
                );
            }
            args.push(pid);
        }
        self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::Sformatf,
            args,
        })
    }

    /// vita's canonical `$typename` spelling for a net (hand-IEEE — no oracle).
    /// Atom types (`integer`/`time`/`real`) print bare; vector/array data types
    /// print as `<base>[msb:lsb]…` for packed dims with unpacked dims appended as
    /// `$[lo:hi]` (IEEE 1800 §20.6.1 form). reg/logic/wire all canonicalise to
    /// "logic" (they are the same 4-state data type).
    pub(crate) fn typename_string(&self, net: u32) -> String {
        use ast::NetVarKind as K;
        // The `string` decl branch never populates `intro_kind`, so detect it first.
        if self.is_string_net(net) {
            return "string".to_string();
        }
        let kind = self.intro_kind.get(&net).copied().unwrap_or(K::Logic);
        // Atom types render bare regardless of any implicit packed dim.
        match kind {
            K::Integer => return "integer".to_string(),
            K::Time => return "time".to_string(),
            K::Real => return "real".to_string(),
            K::Realtime => return "realtime".to_string(),
            K::Event => return "event".to_string(),
            K::String => return "string".to_string(),
            K::Byte => return "byte".to_string(),
            K::Shortint => return "shortint".to_string(),
            K::Int => return "int".to_string(),
            K::Longint => return "longint".to_string(),
            _ => {}
        }
        // `bit` is the 2-state vector data type; reg/logic/wire/tri/… are 4-state.
        let base = if matches!(kind, K::Bit) {
            "bit"
        } else {
            "logic"
        };
        let (dims, unpacked_n) = self.net_dims_desc(net).unwrap_or((Vec::new(), 0));
        let mut s = base.to_string();
        // carry the signedness qualifier (IEEE canonical) when the net is signed.
        if self.nets.get(net as usize).is_some_and(|n| n.signed) {
            s.push_str(" signed");
        }
        // packed dims (after the unpacked prefix) directly follow the base.
        for &(l, r) in dims.iter().skip(unpacked_n) {
            s.push_str(&format!("[{l}:{r}]"));
        }
        // unpacked dims trail as `$[lo:hi]`.
        for &(l, r) in dims.iter().take(unpacked_n) {
            s.push_str(&format!("$[{l}:{r}]"));
        }
        s
    }
}
