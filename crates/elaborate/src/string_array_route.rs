//! string_array_route — T1: route a ZERO-BASED ASCENDING fixed `string` array to the
//! DYNAMIC-array representation, so it gains a runtime index / `foreach` / runtime
//! element write / `.size()` that the per-element-net form cannot express.
//!
//! Why routing rather than extending the element-net path: a fixed string array is N
//! distinct `NetKind::String` nets, so a RUNTIME index would have to select among nets
//! — there is no such operation. The dynamic form is one `DynArray` handle whose
//! elements live in the engine heap, where an index is an ordinary runtime value.
//!
//! Why it is safe to unify (the ladder only goes UP): measured capability parity across
//! 23 shapes, fixed vs dyn, against iverilog — decl-init, const index, byte select,
//! element `.len()`/`.getc()`/`.toupper()`/`.substr()`, element-to-element copy, function
//! argument, ternary, `$sformatf`, `case`, compare, concat, empty read — agree on every
//! one, and dyn additionally answers runtime index, `foreach`, runtime write and
//! `.size()`. Nothing fixed could do is lost. (Before §4.5.220 that was NOT true: the
//! dyn element byte select `d[0][0]` answered a silent 0 where fixed answered 119, so
//! routing then would have been a trade of one silent-wrong for another.)

use super::*;

impl Elaborator<'_> {
    /// Route one `string <name> [n]` / `[0:n-1]` declaration to a `DynArray` handle.
    ///
    /// Returns `true` when the declaration is fully handled here (routed, or rejected
    /// with a diagnostic); `false` to DECLINE, leaving the caller on the unchanged
    /// per-element-net path. Declining is always safe — it is exactly today's behaviour.
    pub(crate) fn route_fixed_string_array(
        &mut self,
        decl: &ast::DeclName,
        n: i64,
        ports: &ast::PortList,
        body: &[ast::ModuleItem],
    ) -> bool {
        // Mirrors the `string s[]` branch: a string container cannot be a port.
        let dir = self.dir_for_name(&decl.name.name, ports, body);
        if dir != ir::PortDir::Internal {
            self.error(
                MsgCode::ElabUnsupported,
                "a string array cannot be a port (outside the v7 scope)",
            );
            return true;
        }
        // Validate the init HERE and let the collectors expand it, which is exactly the
        // division of labour the per-element-net path already uses. The element COUNT
        // check lives inside `fixed_string_array_init_pairs`: without it a
        // `string s[3] = '{"a","b"}` would silently produce a 2-element array (iverilog
        // rejects the mismatch), which is precisely the silent-wrong this routing must
        // not introduce.
        //
        // The pairs are deliberately NOT pushed here. The collectors route them to the
        // list their SCOPE requires — a block-local string init lands in the deferred
        // list so it runs after the module-scope string inits it may read — and pushing
        // from here would both duplicate the writes and flatten that ordering.
        if let Some(init) = &decl.init {
            if self
                .fixed_string_array_init_pairs(&decl.name, &decl.unpacked[0], init)
                .is_none()
            {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a string-array initializer is supported only as a \
                     `'{…}` pattern with one element per declared index, \
                     at module or block scope (else assign elements in an \
                     initial block)",
                );
                return true;
            }
        }

        // MANGLED, never the declared name — see `fixed_string_dyn_key`. The bare name
        // must stay free in the module namespace or a block-local of the same name
        // collides with this net instead of getting its own storage.
        let mangled = format!("{}$sad", decl.name.name);
        let next_id = self.nets.len() as u32;
        self.add_net(
            &mangled,
            ir::NetVar {
                kind: ir::NetKind::DynArray,
                width: 0,
                msb: 0,
                lsb: 0,
                signed: false,
                array_len: 0,
                dir: ir::PortDir::Internal,
                init: default_init(ast::NetVarKind::Reg, 1),
            },
        );
        if (self.nets.len() as u32) <= next_id {
            // The name was already taken (a re-declaration); no net was added, so there
            // is nothing to route. Decline rather than mark a net we do not own.
            return false;
        }
        // `string_elem_dyn_nets` is what makes the engine hold the elements as byte
        // strings and fill `new[]` with "" — the routed net MUST join it, or every
        // element would degrade to a bit-vector 0/X.
        self.string_elem_dyn_nets.insert(next_id);
        self.fixed_string_dyn.insert(next_id, n);
        let key = self.fq(&decl.name.name);
        self.fixed_string_dyn_key.insert(key, next_id);

        // Pre-size to the declared length. This rides `pending_var_inits` (the t0
        // var-init pre-sweep) rather than being synthesized at the net, because a
        // `DynArray` net carries no length. It is pushed HERE, at the declaration, so
        // it always precedes the element writes the collectors push later — the decl
        // pass runs before `collect_var_init_drivers`, and a block-local's writes are
        // appended after the module-scope list entirely.
        let span = decl.name.span;
        let path = ast::HierPath {
            segments: vec![decl.name.clone()],
            span,
        };
        self.pending_var_inits.push((
            ast::Lvalue::Ident(path),
            ast::Expr {
                kind: ast::ExprKind::New {
                    size: Box::new(ast::Expr {
                        kind: ast::ExprKind::IntLit {
                            kind: ast::IntLitKind::Decimal,
                            raw: n.to_string(),
                        },
                        span,
                    }),
                    src: None,
                },
                span,
            },
        ));
        true
    }

    /// T1: true iff `net` is a routed fixed string array — fixed-size storage that
    /// merely happens to be dyn-backed, so the resize operations stay LOUD.
    pub(crate) fn is_fixed_string_dyn(&self, net: u32) -> bool {
        self.fixed_string_dyn.contains_key(&net)
    }
}
