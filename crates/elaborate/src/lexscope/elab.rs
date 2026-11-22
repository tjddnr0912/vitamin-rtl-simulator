//! The lexscope entries that take the Elaborator (tier E): the segment entry `with_scope`
//! and residue R's two routine-key entries (LEAVE L2 / L3, closing at Q-RTNSCOPE RS1), moved
//! here with their bodies (§5.2 Q-LEXSCOPE LS2a) ; and `report_at`, a refusal reported in the
//! carried scope it was lowered in, once per (span, scope) (LS3).

use super::{Mark, ReportedAt, ScopeRef};
use crate::Elaborator;
use diag::MsgCode;
use hdl_ast as ast;

impl Elaborator<'_> {
    /// Run `f` with `cur_prefix` temporarily extended by `seg` (a gen-block
    /// `label[idx]` segment). Restores the prefix on return. Genvar bindings in
    /// `self.params` are NOT touched here (the caller manages those).
    pub(crate) fn with_scope<R>(&mut self, seg: &str, f: impl FnOnce(&mut Self) -> R) -> R {
        let saved = self.lex.scope_enter(seg);
        let r = f(self);
        self.lex.scope_restore(saved);
        r
    }

    /// The routine-table key this generate scope gives `bare`, or `None` when the
    /// current position is not inside a generate scope of this module instance.
    pub(crate) fn cur_gen_rtn_key(&self, bare: &str) -> Option<String> {
        let inst = self.lex.mark_text(Mark::Inst);
        let cur = self.lex.cur_prefix_text();
        let rel = if inst.is_empty() {
            cur
        } else {
            cur.strip_prefix(inst)?.strip_prefix('.')?
        };
        Self::gen_rtn_key(rel, bare)
    }

    /// The routine table key a GENERATE scope gives a bare routine name, or `None`
    /// when `scope` names no generate scope at all (a plain module scope).
    ///
    /// `scope` is a prefix RELATIVE to the module instance (`g[0]`,
    /// `g[0].$func$g[0]$f`, `gl[1].$blk$12_40`). Only the generate-block segments
    /// contribute: a transparent `$func$…`/`$itask$…`/`$blk$…` segment is skipped,
    /// so a call made from inside a generate-scoped function body — or from a named
    /// block nested in the generate block — composes the same key the declaration
    /// registered. Segments are joined with `$`, never `.` (see `rtn_decl_scope`).
    fn gen_rtn_key(scope: &str, bare: &str) -> Option<String> {
        let mut key = String::new();
        for seg in scope.split('.') {
            // A `$…$` segment carries a `[` of its own when the routine it names is
            // itself generate-scoped (`$func$g[0]$f`), so the `$` test comes first.
            if seg.starts_with('$') || !Self::is_gen_scope_segment(seg) {
                continue;
            }
            key.push_str(seg);
            key.push('$');
        }
        if key.is_empty() {
            return None;
        }
        key.push_str(bare);
        Some(key)
    }

    /// Outward walk over the GENERATE scopes enclosing the current position,
    /// innermost first, returning the first composed routine key `hit` accepts.
    ///
    /// This is the routine-table twin of [`Self::walk_scopes_key`], and it is a
    /// separate walk for one reason: `func_table` is MODULE-local (taken and
    /// restored per instance), so its keys are module-relative while `cur_prefix`
    /// is absolute. The walk therefore runs over `cur_prefix` minus `inst_prefix`
    /// and never crosses an instance boundary — a child instance elaborates with
    /// its own tables, so there is nothing outside this module to reach.
    ///
    /// Innermost-wins is what makes a generate `f` shadow a module `f`, and running
    /// out of relative scope (returning `None`) is what hands the module-level key
    /// back to the caller.
    pub(crate) fn walk_rtn_scopes(&self, bare: &str, hit: impl Fn(&str) -> bool) -> Option<String> {
        let inst = self.lex.mark_text(Mark::Inst);
        let cur = self.lex.cur_prefix_text();
        let mut rel: &str = if inst.is_empty() {
            cur
        } else if cur == inst {
            return None;
        } else {
            // `None` = not under this instance's path at all (a table swapped in
            // for a parent expression — see the `wire_ports` swap). No generate
            // scope of THIS position can be claimed; fall back to the module key.
            cur.strip_prefix(inst).and_then(|r| r.strip_prefix('.'))?
        };
        loop {
            if rel.is_empty() {
                return None;
            }
            if let Some(k) = Self::gen_rtn_key(rel, bare) {
                if hit(&k) {
                    return Some(k);
                }
            }
            rel = match rel.rfind('.') {
                Some(i) => &rel[..i],
                None => "",
            };
        }
    }

    /// The use-before-declaration check's own-binding test (`check_decl_precedes_use`): is
    /// the outward walk's hit for `name` the key `name` forms under the scope `decl_pos`
    /// describes (mark [`Mark::DeclPos`])?
    pub(crate) fn walk_hits_decl_pos(&self, name: &str) -> bool {
        let own_key = self.decl_pos_key(name);
        self.walk_scopes_key(name, |k| self.symbols.contains_key(k))
            .as_deref()
            == Some(own_key.as_str())
    }

    /// The same check's block-local exemption: is the key `name` forms under the scope
    /// `decl_pos` describes a hoisted block-local (`hoisted_block_local`)?
    pub(crate) fn hoisted_at_decl_pos(&self, name: &str) -> bool {
        self.hoisted_block_local
            .contains_key(&self.decl_pos_key(name))
    }

    /// The key `name` forms under the scope `decl_pos` describes (mark [`Mark::DeclPos`]).
    fn decl_pos_key(&self, name: &str) -> String {
        let decl_pos_scope = self.lex.mark_text(Mark::DeclPos);
        if decl_pos_scope.is_empty() {
            name.to_string()
        } else {
            format!("{}.{}", decl_pos_scope, name)
        }
    }

    pub(crate) fn report_at(
        &mut self,
        span: Option<ast::Span>,
        prefix: &ScopeRef,
        msg: &str,
        reported: &mut ReportedAt,
    ) {
        let (lo, hi) = span.map_or((0, 0), |s| (s.lo, s.hi));
        // One bound lowered twice under one instance (a statement lowered for two routes)
        // is one defect.
        if !reported
            .0
            .insert((lo, hi, self.lex.scope_text(prefix).to_string()))
        {
            return;
        }
        let saved_prefix = self.lex.scope_swap(prefix);
        match span {
            Some(span) => self.error_at(MsgCode::ElabUnsupported, span, msg),
            None => self.error(MsgCode::ElabUnsupported, msg),
        }
        self.lex.scope_restore(saved_prefix);
    }
}
