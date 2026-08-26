//! R14 (ROADMAP §3 ⑭) — the PROFILE IDENTITY producers: how a schedulable body
//! (a process, a continuous assign) acquires the `kind` + `file:line:col` +
//! instance `scope` that `run.json`'s `processes` object reports.
//!
//! Split out of `stmt_flow` at 989 lines (the ≤1000-line module policy), and it
//! is a clean seam rather than a size-driven one: everything here answers "what
//! do we CALL this body", nothing here decides what it DOES. The one piece that
//! stays in `stmt_flow` is the `push_process` append itself, because the
//! lockstep invariant it maintains is the same one `proc_multipliers` and
//! `proc_scopes` have always ridden.

use super::*;

impl Elaborator<'_> {
    /// Resolve an AST span to a display `file:line:col`, independent of
    /// `cur_span`. ⚠️ `cur_span` during module elaboration is the MODULE HEADER
    /// (see `note_at`'s doc), so a profile row anchored on it would name the
    /// module for every process in it — the one thing this identity exists to
    /// avoid. Returns `(file, line, col)`; `("", 0, 0)` with no resolver
    /// installed, which is the AST-only unit-test path.
    pub(crate) fn span_file_line_col(&self, span: ast::Span) -> (String, u32, u32) {
        match self.span_resolver {
            Some(r) => {
                let loc = r.resolve(span.lo, span.hi);
                (loc.file, loc.line, loc.col)
            }
            None => (String::new(), 0, 0),
        }
    }

    /// R14: THE cont-assign append point. Pushes the `ContAssign` and its
    /// profile identity together so the two vectors cannot drift — five
    /// producers push continuous assigns (a user `assign`, a net-declaration
    /// initializer, three port-binding shapes, a `var_init` flush) and a
    /// per-site `ca_idents.push` next to each would be five chances to forget.
    pub(crate) fn push_cont_assign(
        &mut self,
        ca: ir::ContAssign,
        kind: &'static str,
        span: Option<ast::Span>,
    ) {
        let (file, line, col) = match span {
            Some(sp) => self.span_file_line_col(sp),
            None => (String::new(), 0, 0),
        };
        self.ca_idents.push(ProcIdent {
            kind,
            file,
            line,
            col,
            scope: self.cur_prefix.clone(),
        });
        self.cont_assigns.push(ca);
        debug_assert_eq!(
            self.ca_idents.len(),
            self.cont_assigns.len(),
            "R14 ca_idents must stay lockstep with cont_assigns"
        );
    }

    /// R14: lower a body vita SYNTHESIZED and relabel its profile identity.
    ///
    /// Several producers build an `ast::ProceduralBlock` that has no keyword in
    /// the source at all — the §6.8 declaration-initializer flush, an SVA
    /// desugar, a covergroup sampler, a clocking-block commit handler — and then
    /// hand it to [`Self::lower_proc_block`], which would honestly report the
    /// `ProcKind` field those producers had to fill in. Honest about the FIELD,
    /// misleading about the SOURCE: a profile row saying `always` at a line that
    /// contains a `covergroup` sends the reader hunting for a block that is not
    /// there. The relabel keeps the span (which IS useful — it points at the
    /// construct that caused the synthesis) and corrects the kind.
    pub(crate) fn lower_synth_proc(
        &mut self,
        pb: &ast::ProceduralBlock,
        kind: &'static str,
    ) -> ir::Process {
        let proc = self.lower_proc_block(pb);
        if let Some(id) = self.pending_proc_ident.as_mut() {
            id.kind = kind;
        }
        proc
    }

    /// R14: lower a USER module item's procedural block.
    ///
    /// ⚠️ Not every `ModuleItem::Proc` is a block the user wrote. A module-level
    /// `assert property(…)` / `cover property(…)` is WRAPPED by the parser in a
    /// synthetic `initial` (`module_items.rs`, "wrapped in a synthetic `initial`
    /// … so it flows through the same procedural collection"), so honouring
    /// `ProcKind` there would print `initial` at a line whose first keyword is
    /// `assert` — the same misdirection `lower_synth_proc` exists to prevent,
    /// arriving through a different door. Detected by the body, which is the
    /// fact rather than a proxy: the wrapper's body IS the concurrent
    /// assertion. (Fixing it in the parser would mean a new `ProcKind` variant,
    /// and `ProcKind` is SchemaHash-frozen — every `.vu` would go stale for a
    /// reporting label.)
    pub(crate) fn lower_user_proc(&mut self, p: &ast::ProceduralBlock) -> ir::Process {
        let wrapped = matches!(
            *p.body,
            ast::Stmt::ConcurrentAssert { .. } | ast::Stmt::CoverProperty { .. }
        );
        if wrapped {
            return self.lower_synth_proc(p, "sva");
        }
        self.lower_proc_block(p)
    }

    /// R14: [`Self::lower_synth_proc`] with the arguments the other way round,
    /// for the four call sites that build the `ProceduralBlock` INLINE as a
    /// multi-line literal — putting `kind` last there would bury it under the
    /// literal's closing brace.
    pub(crate) fn lower_synth_proc_inline(
        &mut self,
        kind: &'static str,
        pb: &ast::ProceduralBlock,
    ) -> ir::Process {
        self.lower_synth_proc(pb, kind)
    }

    /// R14: the port-binding spelling of [`Self::push_cont_assign`].
    ///
    /// ⚠️ This passed `None` until round-35 R3, on the claim that "a port hookup
    /// has no source span of its own that would help a reader — the useful half
    /// is the INSTANCE (`scope`)". MEASURED AND REFUTED by the reporter: on
    /// their design `kind:"port"` is 1,267 rows and 51% of all evals, and
    /// `scope` cannot resolve them because ONE instance carries dozens of ports
    /// (their top has a 39-connection instance). The largest category in the
    /// profile was the one category a reader could not act on.
    ///
    /// WHICH SPAN, when they differ. `span` is the PORT CONNECTION in the
    /// PARENT's instantiation — for a named connection the whole `.p(expr)` (or
    /// the `.p` shorthand), starting at the `.` so the COLUMN names the port;
    /// for a positional one the connection expression. That is the text an
    /// author would edit to change this hookup, and it is the only candidate
    /// that separates the rows of one instantiation from each other: several
    /// connections written on one line differ by column, not by line. Both
    /// alternatives collapse exactly that distinction — the child port's
    /// DECLARATION is one span for every instance of the module, and the
    /// instantiation header is one span for every port of one instance.
    ///
    /// `None` is reserved for a connection with NO source text of its own (the
    /// `.*` wildcard synthesizes one connection per port from thin air) and
    /// reports `("", 0, 0)` — the same honest "unlocated" a run with no span
    /// resolver gives. Aiming such a row at the nearest real token instead would
    /// be a location that does not survive being followed.
    pub(crate) fn push_cont_assign_port(&mut self, ca: ir::ContAssign, span: Option<ast::Span>) {
        self.push_cont_assign(ca, "port", span);
    }
}
