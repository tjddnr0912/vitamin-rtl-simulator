//! scoped name lookup — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    // ── scope helpers (FQ-name keying) ─────────────────────────────
    /// Fully-qualified key of a LOCAL name within the current instance scope.
    pub(crate) fn fq(&self, local: &str) -> String {
        if self.cur_prefix.is_empty() {
            local.to_string()
        } else {
            format!("{}.{}", self.cur_prefix, local)
        }
    }
    /// Resolve a bare param/genvar `name` to its value, searching the current
    /// scope then each enclosing GENERATE-block scope (strip one trailing
    /// `.segment` at a time). A genvar bound at the generate-for's scope (`top.i`)
    /// is visible inside the loop body's nested prefix (`top.g[0]`, `top.g[0].h`,
    /// …) — exactly Verilog's generate-scope visibility. The walk STOPS at an
    /// INSTANCE boundary (a plain-identifier segment) so a child instance never
    /// sees a parent module's param by bare name. Innermost binding wins.
    pub(crate) fn lookup_scoped(&self, name: &str) -> Option<i64> {
        self.walk_scopes(name, &self.params)
    }

    /// Shared outward scope walk over a FQ-keyed `BTreeMap`. Looks up `name` in
    /// the current scope, then each enclosing generate-block scope, stopping at
    /// the first instance boundary. Used for both params/genvars and the symbol
    /// (net) table so the visibility rule is identical for each.
    pub(crate) fn walk_scopes<T: Copy>(
        &self,
        name: &str,
        table: &BTreeMap<String, T>,
    ) -> Option<T> {
        self.walk_scopes_key(name, |k| table.contains_key(k))
            .and_then(|k| table.get(&k).copied())
    }

    /// The key-returning core of [`Self::walk_scopes`] — ONE source of truth
    /// for the visibility rule, so key-level consumers (the modport
    /// read-only check, the iface-instance lookup) can never drift from the
    /// value-level lookups.
    pub(crate) fn walk_scopes_key(&self, name: &str, hit: impl Fn(&str) -> bool) -> Option<String> {
        self.walk_scopes_key_inner(name, hit, false)
    }

    /// r19: [`Self::walk_scopes_key`], but the outward walk STOPS as soon as a scope
    /// level binds `name` as a NET without `hit` matching there.
    ///
    /// The plain walk is innermost-wins only WITHIN the single map `hit` probes. A
    /// function/task/block/generate local is a net (in `symbols`), so a consumer that
    /// probes a SIDE-map (`string_array_elems`) sailed straight past an inner local
    /// and resolved it to an OUTER side-map entry — silently, and for non-string
    /// locals too: a task-local `logic [7:0] sa[2]` write landed on a module `string
    /// sa[2]` element, and a generate-local `logic [15:0] sa` read was even
    /// range-checked against the outer array's declared `[0:1]`.
    ///
    /// Deliberately NOT the default. The shadow test keys on `symbols`, which is
    /// populated DURING elaboration, so it is only sound where the lookup cannot run
    /// before the shadowing net exists. `elaborate_gen_item` re-folds every generate
    /// control expression once per phase but reports a fold failure only in the Nets
    /// phase, so making the param lookup (`lookup_scoped`) shadow-aware let a nested
    /// generate fold in Nets and fail in Logic — silently deleting the whole generate
    /// body, exit 0. The `params`/`param_meta` consumers therefore keep the plain
    /// walk; that leaves an inner net failing to shadow an outer param (ROADMAP §2,
    /// its own slice — it needs an order-INDEPENDENT, AST-gathered name set).
    ///
    /// **Precondition for opting a consumer in:** both the probed map AND `symbols`
    /// must be fully populated before that consumer can run. The three string-array
    /// sites qualify because they are reachable only from expression/lvalue lowering,
    /// which starts after the Nets passes that fill both. **Never opt in a consumer
    /// reachable from `const_eval_in_scope`** — that runs during param folding and
    /// during every generate phase, so it would reproduce the silent generate-body
    /// deletion verbatim. `array_const_vals` (`const_eval.rs`) is exactly such a
    /// consumer: it sits INSIDE `const_eval_in_scope` and must keep the plain walk.
    pub(crate) fn walk_scopes_key_shadowed(
        &self,
        name: &str,
        hit: impl Fn(&str) -> bool,
    ) -> Option<String> {
        self.walk_scopes_key_inner(name, hit, true)
    }

    fn walk_scopes_key_inner(
        &self,
        name: &str,
        hit: impl Fn(&str) -> bool,
        stop_at_net_binding: bool,
    ) -> Option<String> {
        use std::fmt::Write;
        let mut prefix = self.cur_prefix.as_str();
        // GEN-3X-STR (part b): reuse ONE scratch String across the outward walk
        // instead of a fresh `format!` allocation per scope level (byte-identical
        // keys; on a hit the scratch is moved out as the owned return).
        let mut key = String::new();
        loop {
            key.clear();
            if prefix.is_empty() {
                key.push_str(name);
            } else {
                let _ = write!(key, "{prefix}.{name}");
            }
            if hit(&key) {
                return Some(key);
            }
            if stop_at_net_binding && self.symbols.contains_key(&key) {
                return None;
            }
            if prefix.is_empty() {
                return None;
            }
            // The innermost segment about to be stripped: only continue walking
            // outward if it is a generate-block scope (`label[idx]`), an inline-task
            // body-local scope (`$itask$…`), or a frame function/task scope
            // (`$func$…`) — all of which must see the enclosing module's nets (a
            // function/task body may read a module signal; SV §13.4 functions are
            // not pure). A formal/local is registered UNDER the `$func$…`/`$itask$…`
            // segment, so it is still found FIRST (innermost-wins) and correctly
            // shadows a same-named module net; only a NON-local name falls through.
            // Stopping at an instance-boundary segment preserves per-instance name
            // isolation. (A frame body WRITING a module net is still rejected by
            // `validate_frame_body` — the frame-call subset only writes its own
            // locals — so this widens reads without enabling an unsupported write.)
            let last_seg = match prefix.rfind('.') {
                Some(i) => &prefix[i + 1..],
                None => prefix,
            };
            if !Self::is_gen_scope_segment(last_seg)
                && !last_seg.starts_with("$itask$")
                && !last_seg.starts_with("$func$")
                // DUP (round-5): a per-block `$blk$<span>` scope is transparent for
                // outward reads exactly like `$func$`/`$itask$` — a scoped block-local
                // is found FIRST (innermost-wins) while a non-scoped name in the same
                // block still falls through to the enclosing module net.
                && !last_seg.starts_with("$blk$")
            {
                return None;
            }
            prefix = match prefix.rfind('.') {
                Some(i) => &prefix[..i],
                None => "",
            };
        }
    }

    /// True if `e` is a self-contained, scope-INDEPENDENT constant expression:
    /// literals, package-scoped constants, and compound expressions built only from
    /// those (plus system-function calls whose args are all scope-safe).
    ///
    /// A class-method/ctor DEFAULT argument value is lowered in the CALLER scope,
    /// but IEEE §13.5.3 resolves it in the method's CLASS scope. Any BARE NAME or
    /// user call is therefore scope-ambiguous: it may name a class-scope entity (a
    /// data member, method, class `parameter`/`localparam`, nested typedef/enum
    /// label, or a `.with()` reduction over a member) that would silently bind to a
    /// same-named CALLER-scope name instead. vita has no authoritative per-class
    /// scope-symbol table (parameterized classes are monomorphised), so rather than
    /// enumerate class-scope names and risk missing a category, callers loud-reject
    /// any filled default that is NOT scope-safe (correct-or-loud). This is an
    /// ALLOW-LIST: a variant not recognised as scope-independent (including any
    /// added later) is rejected. A literal default (`7`, `"x"`, `8'hFF`,
    /// `{4'ha,4'hb}`, `1<<3`, `pkg::K`) works; class-scope name resolution of a
    /// name default is a follow-on.
    pub(crate) fn default_is_scope_safe(e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        let r = Self::default_is_scope_safe;
        match &e.kind {
            // Leaves that are scope-independent.
            K::IntLit { .. } | K::RealLit { .. } | K::StrLit { .. } | K::Null => true,
            // `pkg::x` resolves in the PACKAGE scope regardless of caller/class scope.
            K::PkgScoped { .. } => true,
            // A system function (`$clog2`, `$bits`, …) is scope-independent; safe iff
            // every argument is scope-safe.
            K::SysCall { args, .. } => args.iter().all(r),
            // Compound expressions: safe iff EVERY operand is scope-safe.
            K::Unary { operand, .. } => r(operand),
            K::Binary { lhs, rhs, .. } => r(lhs) && r(rhs),
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => r(cond) && r(then_e) && r(else_e),
            K::BitSelect { base, index } => r(base) && r(index),
            K::PartSelect { base, msb, lsb } => r(base) && r(msb) && r(lsb),
            K::IndexedPart {
                base,
                offset,
                width,
                ..
            } => r(base) && r(offset) && r(width),
            K::Concat { parts } => parts.iter().all(r),
            K::Replicate { count, value } => r(count) && value.iter().all(r),
            K::Paren { inner } => r(inner),
            K::MinTypMax { min, typ, max } => r(min) && r(typ) && r(max),
            // The cast VALUE operand must be scope-safe; a `size'(…)` width too. A
            // type-position target (`Prim`/`Signing`/`Named`) is a TYPE, not a
            // scope-ambiguous value, so it is not recursed. (A `Named` typedef could
            // in theory resolve to a different width in the class vs caller scope, but
            // that is unconstructable today — class-scope typedefs, package-body
            // classes, and cross-module class sharing are all unsupported, so the
            // typedef always resolves in the single module the class and caller
            // share; the value operand is recursed regardless.)
            K::Cast { target, expr } => {
                r(expr)
                    && match target {
                        ast::CastTarget::Size(s) => r(s),
                        _ => true,
                    }
            }
            // Ident, Call, ArrayMethodWith, RandomizeWith, New, ClassNew, Dist,
            // Dollar, AssignPattern, Error, and any variant added later are NOT
            // provably scope-independent → reject (conservative; no escape possible).
            _ => false,
        }
    }

    /// R19-X1: every BARE NAME a default-argument expression can resolve, or `false`
    /// if it holds a form this walk does not model.
    ///
    /// [`Self::default_is_scope_safe`] with ONE variant added — a single-segment
    /// `Ident`, collected rather than rejected. Everything else keeps that function's
    /// allow-list polarity: an unrecognised node (a user call, a method, `new`, a
    /// multi-segment path, or any variant added later) answers `false`, and the caller
    /// then treats the default as scope-ambiguous. Sharing the shape with the
    /// scope-safe predicate is the point: a default this returns `true` for is
    /// composed only of literals, `pkg::` constants, system calls and names, so
    /// comparing the NAMES settles the whole expression.
    pub(crate) fn default_free_names(e: &ast::Expr, out: &mut Vec<String>) -> bool {
        use ast::ExprKind as K;
        let mut r = |x: &ast::Expr| Self::default_free_names(x, out);
        match &e.kind {
            K::Ident(p) => match p.segments.as_slice() {
                [one] => {
                    out.push(one.name.clone());
                    true
                }
                // A dotted path resolves through the joined key, whose scope walk this
                // comparison does not model → ambiguous.
                _ => false,
            },
            K::IntLit { .. } | K::RealLit { .. } | K::StrLit { .. } | K::Null => true,
            K::PkgScoped { .. } => true,
            K::SysCall { args, .. } => args.iter().all(r),
            K::Unary { operand, .. } => r(operand),
            K::Binary { lhs, rhs, .. } => r(lhs) && r(rhs),
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => r(cond) && r(then_e) && r(else_e),
            K::BitSelect { base, index } => r(base) && r(index),
            K::PartSelect { base, msb, lsb } => r(base) && r(msb) && r(lsb),
            K::IndexedPart {
                base,
                offset,
                width,
                ..
            } => r(base) && r(offset) && r(width),
            K::Concat { parts } => parts.iter().all(r),
            K::Replicate { count, value } => r(count) && value.iter().all(r),
            K::Paren { inner } => r(inner),
            K::MinTypMax { min, typ, max } => r(min) && r(typ) && r(max),
            K::Cast { target, expr } => {
                r(expr)
                    && match target {
                        ast::CastTarget::Size(s) => r(s),
                        _ => true,
                    }
            }
            _ => false,
        }
    }

    /// R19-X1: does this filled DEFAULT argument value mean the same thing HERE as it
    /// does where the subroutine is DECLARED?
    ///
    /// vita lowers a default in the CALLER's scope; IEEE 1800 §13.5.4 evaluates it in
    /// the subroutine's own. MEASURED divergence — a module task
    /// `tw(output int x, input int y = g)` called from a task body that declares its
    /// own `g` read the CALLER's `g`: vita printed `91` where iverilog prints `6`, at
    /// exit 0, no diagnostic. (The same hazard for a CLASS-method default was already
    /// closed by [`Self::default_is_scope_safe`]; the plain function/task twin was
    /// not — the pattern this codebase keeps re-learning.)
    ///
    /// Answers by COMPARING BINDINGS rather than by banning names, because banning
    /// them would loud-reject the common and correct case: a module-level default
    /// naming a module net, called from a generate block or a subroutine body, still
    /// resolves outward to the very same net. Only a name that actually binds
    /// elsewhere here is rejected.
    pub(crate) fn default_binding_matches_decl_scope(&mut self, def: &ast::Expr) -> bool {
        // Identical scope, no formal substitution in flight ⇒ the two lowerings are the
        // same lowering. Covers every call that is not inside another subroutine, a
        // generate block, or a scoped block — i.e. almost all of them, byte-identically.
        if self.cur_prefix == self.tf_decl_scope
            && self.subst.is_empty()
            && self.out_subst.is_empty()
        {
            return true;
        }
        if Self::default_is_scope_safe(def) {
            return true;
        }
        let mut names = Vec::new();
        if !Self::default_free_names(def, &mut names) {
            return false;
        }
        // An INLINE-path formal substitution binds a callee formal to a caller net by
        // bare name; a default resolved through one is reading the wrong object by
        // construction, whatever the prefixes say.
        if names
            .iter()
            .any(|n| self.subst_lookup(n).is_some() || self.out_subst_lookup(n).is_some())
        {
            return false;
        }
        let saved = std::mem::replace(&mut self.cur_prefix, self.tf_decl_scope.clone());
        let there: Vec<(Option<u32>, Option<i64>)> = names
            .iter()
            .map(|n| (self.lookup_net_scoped(n), self.lookup_scoped(n)))
            .collect();
        self.cur_prefix = saved;
        names
            .iter()
            .zip(there)
            .all(|(n, t)| (self.lookup_net_scoped(n), self.lookup_scoped(n)) == t)
    }

    // ── name resolution ────────────────────────────────────────────
    /// Resolve a HierPath → NetId. v1: single-segment (flat) names only. Unknown
    /// → emit + return [`POISON_NET`] (u32::MAX, NOT 0 — so a surviving poison
    /// edge is detectable, never a silent alias of net 0). The IR is discarded on
    /// `had_error` regardless. (COVERAGE verdict MEDIUM.)
    /// Resolve a MULTI-segment path as a dotted symbol — interface member access
    /// (`bus.sig`, `i.sig`), whose dotted name IS the symbol key (aliases inserted by
    /// interface port binding; a direct `i.sig` hits the instance's own nets).
    ///
    /// Extracted so a caller that wants to give its OWN diagnostic for an unresolved
    /// hierarchical name can ask the question with the exact predicate `resolve_net`
    /// uses, instead of re-deriving it. Re-deriving is how a classifier drifts from its
    /// lowering; the A2b-prereq F2 filter below (a dotted hit on a package-var import
    /// alias is not a known dotted symbol, §26.3) is precisely the sort of clause that
    /// gets forgotten in a copy.
    pub(crate) fn lookup_dotted_net(&self, path: &ast::HierPath) -> Option<u32> {
        let joined = path
            .segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        self.lookup_net_scoped(&joined)
            .filter(|_| !self.dotted_hit_is_pkg_alias(&joined))
    }

    pub(crate) fn resolve_net(&mut self, path: &ast::HierPath) -> u32 {
        if path.segments.len() != 1 {
            if let Some(id) = self.lookup_dotted_net(path) {
                return id;
            }
            // A hierarchical READ in an expression is deferred (N3) and a hierarchical
            // WHOLE-net WRITE is deferred (`collect_lval_chunks` → `defer_hier_write`),
            // so reaching `resolve_net` with an unresolved multi-segment path is a
            // hierarchical name used as a SELECT base (`tb.dut.x[3] = …`) or another
            // lvalue context the whole-net subset does not cover — a loud follow-on.
            self.error(
                MsgCode::ElabUnsupported,
                "a hierarchical name in this lvalue context is unsupported in this \
                 subset (a whole-net hierarchical write `tb.dut.x = …` is supported; a \
                 hierarchical element/part-select write is a follow-on)",
            );
            return POISON_NET;
        }
        // Resolve in the current scope, then each ENCLOSING scope. A net declared
        // in the module body (`top.d`) is visible from inside a generate block
        // (`top.g[0]`); a net declared inside the block (`top.g[0].t`) shadows it.
        let name = &path.segments[0].name;
        match self.lookup_net_scoped(name) {
            Some(id) => {
                // A2b-prereq guard: when the hit is a package-variable IMPORT
                // alias but a local CONSTANT (param/localparam/enum label, or
                // a declared genvar — S2) also binds this name, the constant
                // shadows the import for reads (the Ident arm resolves params
                // first), so a WRITE reaching this funnel must not silently
                // land in the package variable: loud. `pkg_var_aliases` empty
                // (every pre-existing design) short-circuits: zero change.
                if self.bare_hit_is_shadowed_pkg_alias(name) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "`{name}` here resolves to a local constant \
                             (parameter/genvar/enum label), which shadows \
                             the imported package variable — a constant is \
                             not assignable (rename one of them to \
                             disambiguate)"
                        ),
                    );
                    return POISON_NET;
                }
                id
            }
            None => {
                self.error(
                    MsgCode::ElabUnresolvedName,
                    &format!("undeclared net/variable `{}`", self.fq(name)),
                );
                POISON_NET
            }
        }
    }

    /// Resolve a bare net `name` to its NetId, searching the current scope then
    /// each enclosing GENERATE-block scope. Symmetric with [`Self::lookup_scoped`]
    /// for params/genvars; STOPS at an instance boundary (per-instance net
    /// isolation). Returns the innermost (most specific) binding.
    pub(crate) fn lookup_net_scoped(&self, name: &str) -> Option<u32> {
        self.walk_scopes(name, &self.symbols)
    }

    // ── user function/task inlining (SD2 inline path) ──────────────
    /// Innermost-wins lookup in the input-formal substitution stack. Empty in
    /// steady state.
    pub(crate) fn subst_lookup(&self, name: &str) -> Option<u32> {
        self.subst
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, e)| *e)
    }
    /// Innermost-wins lookup in the output/inout-formal → caller-net stack.
    pub(crate) fn out_subst_lookup(&self, name: &str) -> Option<u32> {
        self.out_subst
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, e)| *e)
    }
    /// Resolve the net targeted by an introspection query: a (parenthesized)
    /// single-segment net ident. Indexed/typed/expression args return `None`
    /// (the query stays loud — pre-existing behavior, not a regression).
    pub(crate) fn resolve_intro_net(&self, e: &ast::Expr) -> Option<u32> {
        match &e.kind {
            ast::ExprKind::Paren { inner } => self.resolve_intro_net(inner),
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.lookup_net_scoped(&p.segments[0].name)
            }
            _ => None,
        }
    }
}
