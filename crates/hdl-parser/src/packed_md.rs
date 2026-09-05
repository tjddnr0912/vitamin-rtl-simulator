//! §3 ⑤ ⓐ: a parameter with MORE THAN ONE packed dimension — `parameter logic
//! [N-1:0][M-1:0] P = …`, `localparam lfsr_perm_t X = {160'h…}` where the typedef
//! is `logic [W-1:0][$clog2(W)-1:0]`.
//!
//! A `ParamDecl` carries ONE range, and the override channel (`#(.P(v))` on an
//! instance, a header default `= pkg::Dflt`) moves one flat value. So the parameter
//! is declared FLAT — `[total-1:0]`, exactly the bits the multi-dimensional vector
//! packs — and every SELECT on its name (`P[i]`, `P[i][j]`, `P[i][a:b]`,
//! `P[i][o+:w]`) is rewritten here, in the parser, to the flat part-select those
//! bits occupy. That is the packed-struct member precedent (`s.f` → `s[hi:lo]`),
//! keyed on the name the same way (`packed_md_params`, module-scoped like
//! `var_struct`; `packed_md_scoped` for `pkg::P`, unit-scoped like `struct_layouts`).
//!
//! Layout is IEEE §7.4.1: the LEFTMOST dimension varies slowest; within a
//! dimension `[left:right]` the element at the RIGHT bound is the lowest bits
//! (descending `[3:0]`: index 0; ascending `[0:2]`: index 2).
//! Every offset/width is an `Expr` (dims may name parameters — prim_lfsr's
//! `[LfsrDw-1:0][LfsrIdxDw-1:0]` is overridden per instance), folded to a literal
//! only when every operand is one; elaborate folds the rest.
//!
//! What is NOT rewritten stays exactly as loud as the scalar parameter twin: a
//! write (`P[i] = …`, E3010), a hierarchical read (`u.P[i]`, E3010), `$size`/
//! `$left`/`$dimensions` (no fold arm), an assignment pattern value (`'{…}`,
//! E3009), and `foreach`. A chain with more selects than dimensions, or a
//! non-final RANGE select, is a loud parse error here (the flat twin would
//! silently answer a bit or `x`).

use super::*;

/// One select of a chain, innermost (nearest the name) first after collection.
enum Sel {
    Bit(Expr),
    Part(Expr, Expr),
    Idx(Expr, Expr, PartDir),
}

impl Parser<'_, '_> {
    /// A non-negative literal or `None`.
    fn lit_u32(e: &Expr) -> Option<u32> {
        Self::const_lit(e).and_then(|v| u32::try_from(v).ok())
    }

    fn lit(v: u32, span: Span) -> Expr {
        Self::dec_lit(v, span)
    }

    fn bin(op: BinOp, l: Expr, r: Expr, span: Span) -> Expr {
        Expr {
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(l),
                rhs: Box::new(r),
            },
            span,
        }
    }

    /// `a - b`, folded when both are literals (and the result is non-negative);
    /// `a - 0` is `a`.
    fn sub(a: Expr, b: Expr, span: Span) -> Expr {
        match (Self::lit_u32(&a), Self::lit_u32(&b)) {
            (Some(x), Some(y)) if x >= y => Self::lit(x - y, span),
            (_, Some(0)) => a,
            _ => Self::bin(BinOp::Sub, a, b, span),
        }
    }

    /// `a + b`, folded when both are literals; `0 + b` is `b`, `a + 0` is `a`.
    fn add(a: Expr, b: Expr, span: Span) -> Expr {
        match (Self::lit_u32(&a), Self::lit_u32(&b)) {
            (Some(x), Some(y)) => match x.checked_add(y) {
                Some(s) => Self::lit(s, span),
                None => Self::bin(BinOp::Add, a, b, span),
            },
            (Some(0), _) => b,
            (_, Some(0)) => a,
            _ => Self::bin(BinOp::Add, a, b, span),
        }
    }

    /// `a * b`, folded when both are literals; `a * 1` / `1 * b` collapse. A
    /// literal ZERO factor is NOT folded away (`0 * x` is `x` in 4-state — the
    /// flat twin must keep that arithmetic).
    fn mul(a: Expr, b: Expr, span: Span) -> Expr {
        match (Self::lit_u32(&a), Self::lit_u32(&b)) {
            (Some(x), Some(y)) => match x.checked_mul(y) {
                Some(p) => Self::lit(p, span),
                None => Self::bin(BinOp::Mul, a, b, span),
            },
            (Some(1), _) => b,
            (_, Some(1)) => a,
            _ => Self::bin(BinOp::Mul, a, b, span),
        }
    }

    /// `(r.msb >= r.lsb) ? desc : asc`, or just one of them when the dimension's
    /// direction is known from literal bounds.
    fn by_dir(r: &Range, desc: Expr, asc: Expr, span: Span) -> Expr {
        match (Self::lit_u32(&r.msb), Self::lit_u32(&r.lsb)) {
            (Some(m), Some(l)) => {
                if m >= l {
                    desc
                } else {
                    asc
                }
            }
            _ => Expr {
                kind: ExprKind::Ternary {
                    cond: Box::new(Self::bin(BinOp::Ge, r.msb.clone(), r.lsb.clone(), span)),
                    then_e: Box::new(desc),
                    else_e: Box::new(asc),
                },
                span,
            },
        }
    }

    /// The element count of one packed dimension: `|msb - lsb| + 1`.
    pub(crate) fn packed_dim_width(r: &Range, span: Span) -> Expr {
        let desc = Self::sub(r.msb.clone(), r.lsb.clone(), span);
        let asc = Self::sub(r.lsb.clone(), r.msb.clone(), span);
        Self::add(Self::by_dir(r, desc, asc, span), Self::lit(1, span), span)
    }

    /// The bit width of `dims[k..]` — the size of one element of `dims[k-1]`.
    fn packed_stride(dims: &[Range], k: usize, span: Span) -> Expr {
        let mut w = Self::lit(1, span);
        for r in &dims[k..] {
            w = Self::mul(w, Self::packed_dim_width(r, span), span);
        }
        w
    }

    /// The flat `[total-1:0]` range a multi-dimensional packed parameter is
    /// declared with.
    pub(crate) fn packed_md_flat_range(dims: &[Range], span: Span) -> Range {
        let total = Self::packed_stride(dims, 0, span);
        Range {
            msb: Self::sub(total, Self::lit(1, span), span),
            lsb: Self::lit(0, span),
            span,
        }
    }

    /// Element offset (in elements from the lowest bits) of index `i` within
    /// dimension `r` = `[left:right]`: the element at the RIGHT bound is the lowest
    /// bits either way, so `i - right` descending and `right - i` ascending
    /// (`Range::lsb` is the right bound, whichever is numerically smaller).
    fn packed_elem_off(r: &Range, i: &Expr, span: Span) -> Expr {
        let desc = Self::sub(i.clone(), r.lsb.clone(), span);
        let asc = Self::sub(r.lsb.clone(), i.clone(), span);
        Self::by_dir(r, desc, asc, span)
    }

    /// Re-spell every bare name a package-level dimension expression uses
    /// (`[W-1:0][$clog2(W)-1:0]` with `parameter int W` in the same package) as
    /// `pkg::W`, so the dims survive into an importing module that did NOT
    /// wildcard-import the package (`p::P[i]`, `import p::P`). Only a name this
    /// package itself declared is re-spelled (`local_decl_names`); anything else
    /// stays bare (and stays as loud as the typedef twin when unresolvable).
    /// Shapes a dimension can carry are walked; an unknown shape is left alone.
    pub(crate) fn respell_pkg_dims(&self, pkg: &str, dims: &[Range]) -> Vec<Range> {
        dims.iter()
            .map(|r| Range {
                msb: self.respell_pkg_expr(pkg, r.msb.clone()),
                lsb: self.respell_pkg_expr(pkg, r.lsb.clone()),
                span: r.span,
            })
            .collect()
    }

    fn respell_pkg_expr(&self, pkg: &str, e: Expr) -> Expr {
        let span = e.span;
        let kind = match e.kind {
            ExprKind::Ident(p)
                if p.segments.len() == 1 && self.local_decl_names.contains(&p.segments[0].name) =>
            {
                ExprKind::PkgScoped {
                    pkg: Ident {
                        name: pkg.to_string(),
                        span,
                    },
                    name: p.segments[0].clone(),
                }
            }
            // §4.5.415 (review B F3): a name the package IMPORTED (not its own
            // declaration) with a parser-known literal value is fixed at the package
            // — a package constant is never overridden (§6.20.1) — so the twin carries
            // the VALUE; left bare it bound in the importer's scope (`QW` read the
            // importer's own local, both oracles the package's). A name with no
            // parser-time value stays as written.
            ExprKind::Ident(p)
                if p.segments.len() == 1 && self.const_locals.contains_key(&p.segments[0].name) =>
            {
                let v = self.const_locals[&p.segments[0].name].v;
                if v < 0 && v.unsigned_abs() <= u32::MAX as u64 {
                    ExprKind::Unary {
                        op: UnOp::Minus,
                        operand: Box::new(Self::dec_lit(v.unsigned_abs() as u32, span)),
                    }
                } else if v <= u32::MAX as i64 {
                    return Self::dec_lit(v as u32, span);
                } else {
                    ExprKind::Ident(p)
                }
            }
            ExprKind::Binary { op, lhs, rhs } => ExprKind::Binary {
                op,
                lhs: Box::new(self.respell_pkg_expr(pkg, *lhs)),
                rhs: Box::new(self.respell_pkg_expr(pkg, *rhs)),
            },
            ExprKind::Unary { op, operand } => ExprKind::Unary {
                op,
                operand: Box::new(self.respell_pkg_expr(pkg, *operand)),
            },
            ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => ExprKind::Ternary {
                cond: Box::new(self.respell_pkg_expr(pkg, *cond)),
                then_e: Box::new(self.respell_pkg_expr(pkg, *then_e)),
                else_e: Box::new(self.respell_pkg_expr(pkg, *else_e)),
            },
            ExprKind::Paren { inner } => ExprKind::Paren {
                inner: Box::new(self.respell_pkg_expr(pkg, *inner)),
            },
            ExprKind::SysCall { name, args } => ExprKind::SysCall {
                name,
                args: args
                    .into_iter()
                    .map(|a| self.respell_pkg_expr(pkg, a))
                    .collect(),
            },
            ExprKind::Cast { target, expr } => ExprKind::Cast {
                target,
                expr: Box::new(self.respell_pkg_expr(pkg, *expr)),
            },
            other => other,
        };
        Expr { kind, span }
    }

    /// The bare or `pkg::`-scoped name a select chain begins at, when that name is
    /// a multi-dimensional packed parameter; `None` for every other chain (byte-
    /// identical path).
    fn packed_md_chain_dims(&self, e: &Expr) -> Option<Vec<Range>> {
        if self.packed_md_params.is_empty() && self.packed_md_scoped.is_empty() {
            return None;
        }
        let mut cur = e;
        let mut depth = 0usize;
        while let ExprKind::BitSelect { base, .. }
        | ExprKind::PartSelect { base, .. }
        | ExprKind::IndexedPart { base, .. } = &cur.kind
        {
            depth += 1;
            cur = base;
        }
        if depth == 0 {
            return None;
        }
        match &cur.kind {
            ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.packed_md_params.get(&p.segments[0].name).cloned()
            }
            ExprKind::PkgScoped { pkg, name } => self
                .packed_md_scoped
                .get(&format!("{}::{}", pkg.name, name.name))
                .cloned(),
            _ => None,
        }
    }

    /// Rewrite a select chain on a multi-dimensional packed parameter (see the
    /// module doc) into the flat bit/part-select. Every other expression is
    /// returned untouched.
    pub(crate) fn rewrite_packed_md_select(&mut self, e: Expr) -> Expr {
        let Some(dims) = self.packed_md_chain_dims(&e) else {
            return e;
        };
        let span = e.span;
        // Peel the chain (innermost-first), then put it in source order.
        let mut sels: Vec<Sel> = Vec::new();
        let mut cur = e;
        let base = loop {
            match cur.kind {
                ExprKind::BitSelect { base, index } => {
                    sels.push(Sel::Bit(*index));
                    cur = *base;
                }
                ExprKind::PartSelect { base, msb, lsb } => {
                    sels.push(Sel::Part(*msb, *lsb));
                    cur = *base;
                }
                ExprKind::IndexedPart {
                    base,
                    offset,
                    width,
                    dir,
                } => {
                    sels.push(Sel::Idx(*offset, *width, dir));
                    cur = *base;
                }
                _ => break cur,
            }
        };
        sels.reverse();
        let n = dims.len();
        let m = sels.len();
        if m > n {
            self.error(
                "at most one select per packed dimension of a multi-dimensional packed parameter or formal (this chain selects deeper than the declared dimensions)",
            );
            return base;
        }
        if sels[..m - 1].iter().any(|s| !matches!(s, Sel::Bit(_))) {
            self.error(
                "an index in every select but the last on a multi-dimensional packed parameter (a range select must be the final one)",
            );
            return base;
        }
        // Bits below the selected element(s) contributed by the leading indexes.
        let mut off = Self::lit(0, span);
        for (k, s) in sels.iter().enumerate().take(m - 1) {
            let Sel::Bit(i) = s else { unreachable!() };
            let e_off = Self::packed_elem_off(&dims[k], i, span);
            off = Self::add(
                off,
                Self::mul(e_off, Self::packed_stride(&dims, k + 1, span), span),
                span,
            );
        }
        let k = m - 1;
        let r = &dims[k];
        let stride = Self::packed_stride(&dims, k + 1, span);
        // Element-relative (lowest element offset, element count) of the LAST select.
        let (lo_el, count) = match sels.pop().unwrap() {
            Sel::Bit(i) => {
                let lo = Self::packed_elem_off(r, &i, span);
                if k + 1 == n {
                    // The last dimension's elements are single bits.
                    return Expr {
                        kind: ExprKind::BitSelect {
                            base: Box::new(base),
                            index: Box::new(Self::add(off, lo, span)),
                        },
                        span,
                    };
                }
                (lo, Self::lit(1, span))
            }
            Sel::Part(a, b) => {
                // `[a:b]` spans elements b..=a descending / a..=b ascending. A REVERSED
                // select (`P[1:2]` on `[2:0]`, `P[3:2]` on `[1:3]`) is refused HERE when
                // the direction is decidable — both bounds literal, or one literal bound
                // (`[N-1:0]` descending, `[0:N-1]` ascending) — because the flat `+:`
                // below would answer a 0-width value in silence (review B1; the flat
                // and variable twins are loud "out of order"). A range select after a
                // RUNTIME index (`P[i][4:1]`) must stay `+:`: a flat `[hi:lo]` with a
                // runtime bound is a pre-existing silent 0 on any parameter.
                if let (Some(av), Some(bv)) = (Self::const_lit(&a), Self::const_lit(&b)) {
                    let desc = match (Self::const_lit(&r.msb), Self::const_lit(&r.lsb)) {
                        (Some(m), Some(l)) => Some(m >= l),
                        (None, Some(_)) => Some(true),
                        (Some(_), None) => Some(false),
                        (None, None) => None,
                    };
                    if let Some(desc) = desc {
                        if (desc && av < bv) || (!desc && av > bv) {
                            self.error(
                                "a range select in the dimension's own direction on a multi-dimensional packed parameter or formal (this select is out of order: `[a:b]` must descend on a `[hi:lo]` dimension and ascend on a `[lo:hi]` one)",
                            );
                            return base;
                        }
                    }
                }
                let lo_desc = Self::sub(b.clone(), r.lsb.clone(), span);
                let lo_asc = Self::sub(r.lsb.clone(), b.clone(), span);
                let n_desc = Self::sub(a.clone(), b.clone(), span);
                let n_asc = Self::sub(b, a, span);
                (
                    Self::by_dir(r, lo_desc, lo_asc, span),
                    Self::add(
                        Self::by_dir(r, n_desc, n_asc, span),
                        Self::lit(1, span),
                        span,
                    ),
                )
            }
            Sel::Idx(o, w, dir) => {
                // `[o+:w]` spans elements o..=o+w-1, `[o-:w]` spans o-w+1..=o —
                // in INDEX order regardless of the dimension's direction.
                let wm1 = Self::sub(w.clone(), Self::lit(1, span), span);
                let (idx_lo, idx_hi) = match dir {
                    PartDir::PlusColon => (o.clone(), Self::add(o, wm1, span)),
                    PartDir::MinusColon => (Self::sub(o.clone(), wm1, span), o),
                };
                let lo_desc = Self::sub(idx_lo, r.lsb.clone(), span);
                let lo_asc = Self::sub(r.lsb.clone(), idx_hi, span);
                (Self::by_dir(r, lo_desc, lo_asc, span), w)
            }
        };
        let offset = Self::add(off, Self::mul(lo_el, stride.clone(), span), span);
        let width = Self::mul(count, stride, span);
        Expr {
            kind: ExprKind::IndexedPart {
                base: Box::new(base),
                offset: Box::new(offset),
                width: Box::new(width),
                dir: PartDir::PlusColon,
            },
            span,
        }
    }
}

/// The §20.7 array query functions [`Parser::rewrite_packed_md_dim_query`] answers.
/// Twin of `elaborate::const_array::is_dim_query_name` — the two crates cannot share
/// a list, so a name added to one must be added to the other.
fn is_dim_query_name(name: &str) -> bool {
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

impl Parser<'_, '_> {
    /// `$size(P)` / `$left(P, d)` / … on a multi-dimensional packed PARAMETER.
    ///
    /// The declaration was flattened to one range (`packed_md_flat_range`) and only
    /// this parser still knows the dimensions, so an elaborate-side query would read
    /// the FLAT range — `$size(P)` 8 for `logic [1:0][3:0] P`, where both oracles say 2
    /// (§3 ⑤ ⓔ census, the scalar control twin). The answer is built from the recorded
    /// dimension the way a select's offset is: `packed_dim_width` for `$size`, the
    /// bound expressions for `$left` / `$right`, `by_dir` for `$high` / `$low` /
    /// `$increment`, literals for the dimension counts. A dimension index that is not
    /// a literal, or names a dimension the parameter does not declare, is a parse
    /// error rather than a guess (the runtime prints `x` for the latter; a constant
    /// cannot). Every call on any other operand is returned untouched.
    pub(crate) fn rewrite_packed_md_dim_query(&mut self, e: Expr) -> Expr {
        let ExprKind::SysCall { name, args } = &e.kind else {
            return e;
        };
        if !is_dim_query_name(&name.name) || args.is_empty() || args.len() > 2 {
            return e;
        }
        let dims = match &args[0].kind {
            ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.packed_md_params.get(&p.segments[0].name).cloned()
            }
            ExprKind::PkgScoped { pkg, name } => self
                .packed_md_scoped
                .get(&format!("{}::{}", pkg.name, name.name))
                .cloned(),
            _ => None,
        };
        let Some(dims) = dims else {
            return e;
        };
        let span = e.span;
        match name.name.as_str() {
            "$dimensions" | "$unpacked_dimensions" if args.len() == 2 => {
                self.error("`$dimensions` / `$unpacked_dimensions` take one argument");
                return e;
            }
            "$dimensions" => return Self::lit(dims.len() as u32, span),
            "$unpacked_dimensions" => return Self::lit(0, span),
            _ => {}
        }
        let d = if args.len() == 2 {
            match Self::lit_u32(&args[1]) {
                Some(d) => d,
                None => {
                    self.error(
                        "the dimension index of an array query on a multi-dimensional packed parameter must be a literal",
                    );
                    return e;
                }
            }
        } else {
            1
        };
        if d < 1 || d as usize > dims.len() {
            self.error(
                "an array query names a dimension a multi-dimensional packed parameter does not declare",
            );
            return e;
        }
        let r = &dims[d as usize - 1];
        match name.name.as_str() {
            "$size" => Self::packed_dim_width(r, span),
            "$left" => r.msb.clone(),
            "$right" => r.lsb.clone(),
            "$high" => Self::by_dir(r, r.msb.clone(), r.lsb.clone(), span),
            "$low" => Self::by_dir(r, r.lsb.clone(), r.msb.clone(), span),
            "$increment" => {
                let neg = Expr {
                    kind: ExprKind::Unary {
                        op: UnOp::Minus,
                        operand: Box::new(Self::lit(1, span)),
                    },
                    span,
                };
                Self::by_dir(r, Self::lit(1, span), neg, span)
            }
            _ => e,
        }
    }
}
