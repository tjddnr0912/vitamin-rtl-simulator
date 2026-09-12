//! §2 "Index sealing" — a constant's DECLARED WIDTH, asked without its bit layout.
//!
//! [`Elaborator::narrow_param_bits`] and its package twin
//! [`Elaborator::pkg_const_narrow_bits`] (`const_wide.rs`) answer one question with two
//! halves welded together: *"is this name's width a declared fact, and may I read its
//! bits positionally?"* The second half makes them decline a non-zero declared LSB
//! (`[39:4]`) or an ascending declaration (`[0:35]`), because the wide bit domain
//! indexes from 0 and carries no direction, so reading those bits through it would read
//! the declared range backwards (§4.5.363).
//!
//! Two of their three consumers never look at the bits. Census of every call site of
//! the two bits resolvers, `grep -rn 'self\.narrow_param_bits(\|self\.pkg_const_narrow_bits('
//! crates/*/src` (6 hits, 3 per lane) at the slice that added this module:
//!
//! | call site | what it keeps | needs the layout? |
//! |---|---|---|
//! | `const_wide.rs:1097` / `:1113` (`wide_name_bits`) | the whole `(bits, w, signed)` | YES |
//! | `params.rs` (`param_decl_width_opt`, `declared_only` bare-alias + `pkg::X` arms) | `.map(\|(_, w, s)\| (w, s))` — bits DROPPED | NO |
//! | `param_query.rs` (`declared_override_widths`, the two `Leaf` arms) | `let (_, w, signed) = …` — bits DROPPED | NO |
//!
//! For those four sites the layout decline throws away a width that is a declared fact.
//! `DeclRange = (i64, u32, bool)` (`lib.rs`) is filled at one place,
//! `params.rs::param_decl_range_opt`: `Some((lo, m.abs_diff(l) as u32 + 1, m < l))`. The
//! width term is `|msb − lsb| + 1`, so `[35:0]`, `[0:35]`, `[39:4]` and `[4:39]` all
//! record 36 — the width is direction- and offset-INDEPENDENT by construction, and the
//! `lo != 0 || ascending` test answers a question about bit POSITIONS that a caller
//! holding only `(w, signed)` never asks.
//!
//! Measured before this module existed, `localparam logic [0:35] PA = 36'hABC;` used as
//! `#(.P(pk::PA))` over an untyped `parameter P = 8` bound 32 bits where both iverilog
//! 13 and verilator 5.052 bind 36 — and with `36'hFEDCBA987` the VALUE went with it:
//! `edcba987` / `-305419897` against both oracles' `fedcba987` / `68414056839`.
//!
//! ⚠️ These are SEPARATE functions from the bits twins, not a `layout_needed: bool` on a
//! shared body: the two questions have different soundness arguments (a width needs the
//! provenance proof only; bits need the provenance proof AND a 0-based descending
//! layout), and ENGINEERING_RULES §3 keeps two such predicates apart so widening one can
//! never widen the other. Nothing here is reachable from `wide_name_bits`, so
//! §4.5.363's positional read stays declined where it is real.

use super::*;

impl Elaborator<'_> {
    /// A narrow module parameter's DECLARED `(width, signed)`, layout-independent.
    ///
    /// Every guard of [`Self::narrow_param_bits`] is reproduced verbatim EXCEPT the
    /// `lo != 0 || ascending` layout decline, which is the only one that is about bit
    /// positions rather than about whether the width is declared:
    ///
    /// * the frame-local shadow guard — an inline-function formal or a task output
    ///   formal SHADOWS the param, so the VALUE resolves to it first and the width must
    ///   not come from the param either;
    /// * `walk_scopes_key` over the same combined set, so a shadowing net wins here
    ///   exactly as it does in the value lookup;
    /// * a `param_range` entry must EXIST — that map is the one that answers "is this
    ///   width a DECLARED fact?" (`param_decl_range` records only a declared range or a
    ///   declared type/literal), unlike `param_meta`, where widths INFERRED from a
    ///   folded value sit next to declared ones;
    /// * `param_meta`'s width must AGREE with it, which is what proves the SIGN comes
    ///   from the same declaration as the width. A disagreement means one of the two
    ///   was inferred, and this resolver refuses to mix them.
    ///
    /// Fail-closed: a name with no `param_range` entry (a `$clog2` initializer) or none
    /// in `param_meta` (a >64-bit parent) still declines, so this can only move a leaf
    /// from the value-inferred 32 to a declared width, never from a declared width to a
    /// guess.
    pub(crate) fn narrow_param_decl_width(&self, path: &ast::HierPath) -> Option<(u32, bool)> {
        let [seg] = path.segments.as_slice() else {
            return None;
        };
        let n = &seg.name;
        if self.subst_lookup(n).is_some() || self.out_subst_lookup(n).is_some() {
            return None;
        }
        let key = self.walk_scopes_key(n, |k| {
            self.params.contains_key(k) || self.symbols.contains_key(k)
        })?;
        let (_lo, w, _ascending) = self.param_range.get(&key).copied()?;
        // The VALUE must be bound at the same key. `narrow_param_bits` reads it to
        // build the bits; this resolver does not return it, but a name with a declared
        // range and no bound value is not a constant at all, and answering its width
        // would certify a leaf the fold behind the caller cannot evaluate.
        self.params.get(&key)?;
        let (mw, signed) = self.param_meta.get(&key).copied()?;
        if mw != w {
            return None;
        }
        Some((w, signed))
    }

    /// A narrow PACKAGE constant's DECLARED `(width, signed)` — the package twin of
    /// [`Self::narrow_param_decl_width`], standing to [`Self::pkg_const_narrow_bits`]
    /// exactly as that one stands to [`Self::narrow_param_bits`].
    ///
    /// `pkg_const_range` is the provenance-filtered map (filled by the same
    /// `param_decl_range_opt`), `pkg_const_meta` the one that also carries inferred
    /// widths; requiring the two to agree is the same proof, in the package scope.
    /// There is no frame-local shadow guard here because a `pkg::NAME` spelling cannot
    /// be shadowed by a frame local — the resolution is qualified.
    pub(crate) fn pkg_const_decl_width(&self, pkg: &str, name: &str) -> Option<(u32, bool)> {
        let (_lo, w, _ascending) = self.pkg_const_range.get(pkg)?.get(name).copied()?;
        self.pkg_consts.get(pkg)?.get(name)?;
        let (mw, signed) = self.pkg_const_meta.get(pkg)?.get(name).copied()?;
        if mw != w {
            return None;
        }
        Some((w, signed))
    }

    /// A whole OVERRIDE expression that is one declared NAME: its `(width, signed)`.
    ///
    /// §6.20.2 gives an untyped, unranged parameter the range of its FINAL override
    /// value. When that value is written as a bare name the wide channel
    /// (`override_bits` → `fold_self_bits` → `wide_name_bits`) is what supplies it, and
    /// that channel declines an ascending / non-zero-LSB source because its bits are
    /// indexable (`narrow_param_bits`). Measured on `localparam logic [0:35] PA =
    /// 36'hABC` in a package, `#(.P(pk::PA))` over `parameter P = 8`:
    ///
    /// ```text
    /// iverilog 13  bits=36 val=000000abc
    /// verilator    bits=36 val=000000abc
    /// vita PRE     bits=32 val=00000abc
    /// ```
    ///
    /// and with `36'hFEDCBA987` the value itself was destroyed — `edcba987` /
    /// `-305419897` against both oracles' `fedcba987` / `68414056839`, because the
    /// binder coerces the (correct, full) i64 value to the meta width it was given.
    ///
    /// ⚠️ WHY THIS IS NOT THE LAYOUT QUESTION. The whole override is the name; the
    /// consumer is the child's parameter binding, which takes a WIDTH and a SIGN and
    /// coerces the already-folded i64 VALUE to them. No bit POSITION of the source is
    /// read anywhere on this path — which is the only thing the declared direction and
    /// offset change (§4.5.363). A select (`pk::PA[0:3]`, correct today) is a different
    /// expression and never reaches here: this arm matches a bare `Ident` / `PkgScoped`
    /// top only, and `wide_name_bits`' select arms are untouched.
    ///
    /// ⚠️ WHY IT IS A SEPARATE FUNCTION from [`Self::override_self_meta`]'s operator
    /// gate rather than a name added to that gate's accept set. That gate declines every
    /// NAME on purpose: it sizes through `const_self_width`, whose `Ident` arm falls back
    /// to `param_meta` and GUESSES 32 on a miss, so admitting names there would open the
    /// §4.5.363 laundering door it exists to keep shut. This resolver never consults
    /// `const_self_width`; it reads the provenance-filtered `param_range` /
    /// `pkg_const_range` pair and refuses anything they do not certify (ENGINEERING_RULES
    /// §3: a widening must be an opt-in of the CONSUMER, not of the shared machinery).
    ///
    /// Fail-closed and ordered AFTER the wide channel in `bind_one_param`'s meta chain,
    /// so every override that folds today keeps the width it has: this can only replace
    /// a value-inferred width with a declared one.
    pub(crate) fn override_decl_name_meta(&self, e: &ast::Expr) -> Option<(u32, bool)> {
        let mut top = e;
        while let ast::ExprKind::Paren { inner } = &top.kind {
            top = inner;
        }
        // A SIZE CAST of such a name, `40'(pk::PA)`. The width is the cast's size and
        // the sign is the operand's — which is not a new rule but the one the wide
        // channel already applies to every cast it folds; it is unreachable here only
        // because its operand declines. Measured, same design, `logic signed [35:0] SN`
        // vs `logic signed [0:35] SA`, `#(.P(40'(pk::S?)))`:
        //
        // ```text
        //             iverilog      verilator     vita PRE
        // c40_SN      40 fffffffffb 40 fffffffffb 40 fffffffffb   (descending, folds)
        // c40_SA      40 fffffffffb 40 fffffffffb 32 fffffffb     (ascending, declined)
        // c40_UA      40 0000000abc 40 0000000abc 32 00000abc
        // ```
        //
        // Scoped to a certified-name operand: this is NOT a general "a size cast's
        // width is its size" arm, which would answer for operands no map certifies.
        if let ast::ExprKind::Cast {
            target: ast::CastTarget::Size(n),
            expr,
        } = &top.kind
        {
            let (_, signed) = self.override_decl_name_meta(expr)?;
            let w = self.const_eval_in_scope(n)?;
            if w < 1 || w > i64::from(u16::MAX) {
                return None;
            }
            return Some((w as u32, signed));
        }
        match &top.kind {
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => self.narrow_param_decl_width(p),
            ast::ExprKind::PkgScoped { pkg, name } => {
                self.pkg_const_decl_width(&pkg.name, &name.name)
            }
            _ => None,
        }
    }
}
