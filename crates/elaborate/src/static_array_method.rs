//! V34-4: fixed-size unpacked arrays as receivers of the IEEE 1800 §7.12 array
//! manipulation methods — the §7.12.3 REDUCTIONS (`.sum()`/`.product()`/
//! `.and()`/`.or()`/`.xor()`, with or without a `with` clause) and the §7.12.2
//! ORDERING methods (`.sort()`/`.rsort()`/`.reverse()`).
//!
//! §7.12 applies those to fixed-size unpacked arrays as well as to the dynamic
//! kinds. vita resolved the receiver through `dyn_handle` ALONE, so a fixed
//! array got `E3009 … applies to a dynamic array / queue / assoc handle` (the
//! `with` form) or the flatly wrong "unsupported hierarchical function/task call
//! `a.sum`" (the bare and ordering forms), while the queue twin ran the
//! identical machinery. Nothing new is built here: both lowerings emit exactly
//! the IR the dyn-handle path emits, so the frozen sim-ir shape and
//! `format_version` are untouched.
//!
//! ## Which tool is the oracle here (measured, 2026-08-25)
//!
//! The item's premise — "iverilog supports these on fixed arrays" — is FALSE and
//! was measured before anything was built:
//!
//! ```text
//! int a[4]; s = a.sum();
//!   iverilog 13   error: Object tb.a has no method "sum(...)".
//!   iverilog 13   `a.sum() with (item)` does not even PARSE (nor does the
//!                 QUEUE spelling `q.sum() with (item)`) — iverilog has no
//!                 `with` clause at all, and `.and()/.or()/.xor()` collide
//!                 with the operator keywords.
//!   verilator 5.050  SUM=10  PROD=24  XOR=4  AND=0  OR=7   ✅
//! ```
//!
//! So verilator 5.050 is the SOLE oracle for this axis, and it is a legitimate
//! one: `int` is 2-state, and the item under test is width/sign arithmetic, not
//! x-propagation. Everything verilator cannot answer stays loud — see
//! [`StaticArrayRecv`].

use super::*;

/// What [`Elaborator::static_array_recv`] made of a receiver name.
pub(crate) enum StaticArrayRecv {
    /// A 1-D fixed-size unpacked array of integral elements: `(net, declared low
    /// index)`. The low index is `item.index`'s base — the engine iterates the
    /// FLAT slots, while §7.12.3's `index` is the DECLARED one (`int a[-1:1]`
    /// must yield -1, 0, 1 — never 0, 1, 2).
    Integral(u32, i64),
    /// A fixed-size unpacked array whose reduction vita must NOT guess at.
    Unsupported(&'static str),
    /// Not a fixed-size unpacked array at all — the caller keeps its own message.
    No,
}

impl Elaborator<'_> {
    /// Resolve a bare method-receiver name to a fixed-size unpacked array the
    /// §7.12 machinery can iterate. Shared by the reduction and the ordering
    /// callers so both refuse the same set for the same stated reasons.
    ///
    /// MUST-STAY-LOUD set, each measured rather than assumed:
    ///
    ///  * **2-D and up.** verilator 5.050 — the only tool that runs the 1-D form
    ///    — does not compile a 2-D one: it emits C++ that does not build
    ///    (`assigning to 'IData' from incompatible type WithFuncReturnType<…>
    ///    (aka 'VlUnpacked<unsigned int, 3>')`, measured on `int a[2][3]`), and
    ///    iverilog has no fixed-array reduction at all. No oracle on either side
    ///    ⇒ vita does not guess. §7.12.3 does not say whether the fold is over
    ///    the ROWS or over the leaf elements, which is precisely the question.
    ///  * **`real` / `string` / class-handle elements.** The fold is 4-state
    ///    INTEGER arithmetic; running it over an f64 net would answer with the
    ///    IEEE-754 bit pattern at exit 0. That is a silent-wrong, not a gap.
    ///  * **Packed vectors.** `logic [3:0] v; v.sum()` is not an array method at
    ///    all (verilator refuses it). `net_is_static_array` is false for a net
    ///    with no declared unpacked dims, so this returns `No` and the caller's
    ///    own diagnostic stands.
    pub(crate) fn static_array_recv(&self, name: &str) -> StaticArrayRecv {
        // Resolution priority mirrors `expr_array_view`: an inline-subst formal,
        // an out-formal or a constant shadows a net of the same name, and a
        // shadowed import alias must never silently reach the package storage.
        if self.subst_lookup(name).is_some()
            || self.out_subst_lookup(name).is_some()
            || self.lookup_scoped(name).is_some()
            || self.bare_hit_is_shadowed_pkg_alias(name)
        {
            return StaticArrayRecv::No;
        }
        let Some(net) = self.lookup_net_scoped(name) else {
            return StaticArrayRecv::No;
        };
        // Declared array-ness, not `array_len > 1`: `int a[1]` is still an array
        // and its `.sum()` is its single element (the `[0:0]` trap of find #5).
        if !self.net_is_static_array(net) {
            return StaticArrayRecv::No;
        }
        let Some(nv) = self.nets.get(net as usize) else {
            return StaticArrayRecv::No;
        };
        if !matches!(
            nv.kind,
            ir::NetKind::Wire | ir::NetKind::Reg | ir::NetKind::Logic | ir::NetKind::Integer
        ) || self.class_handle_nets.contains(&net)
        {
            return StaticArrayRecv::Unsupported(
                "an IEEE 1800 §7.12 array method needs integral elements — a real / \
                 string / class-handle array has no integer fold",
            );
        }
        // `net_dim_extents` is exact here: every array-declaring site records
        // `array_dims` when `dim_extents.len() >= 2` (or some `lo != 0`), so an
        // ABSENT entry proves the net is 1-D and 0-based.
        match self.net_dim_extents(net).as_slice() {
            // `item.index` is a 32-bit signed value (§7.12.3), and the rebase
            // constant is built at that width. A declared bound outside i32
            // could only come from a fold that is already wrong, so refuse
            // rather than truncate it silently.
            [(lo, _)] if i32::try_from(*lo).is_ok() => StaticArrayRecv::Integral(net, *lo),
            [_] => StaticArrayRecv::Unsupported(
                "an IEEE 1800 §7.12 array method needs a declared low index that fits 32 \
                 bits (`item.index` is a 32-bit signed value)",
            ),
            _ => StaticArrayRecv::Unsupported(
                "an IEEE 1800 §7.12 array method on a MULTI-dimensional fixed-size array \
                 is refused: iverilog 13 has no fixed-array method at all and verilator \
                 5.050 does not compile the multidimensional form, so there is no \
                 oracle to match",
            ),
        }
    }

    /// The BARE (`with`-less) §7.12.3 reduction on a fixed-size unpacked array —
    /// the exact IR the dyn-handle twin in `lower_dyn_method_expr` emits, and
    /// then the same engine arm: `SysFunc{which, args:[Signal{net,word:None}]}`.
    pub(crate) fn lower_static_array_reduction(
        &mut self,
        net: u32,
        method: &str,
        args: &[ast::Expr],
    ) -> u32 {
        if !args.is_empty() {
            self.error(
                MsgCode::ElabUnsupported,
                "array reduction methods take no arguments (use the `with` clause form)",
            );
        }
        let which = match method {
            "sum" => ir::SysFuncId::ArrSum,
            "product" => ir::SysFuncId::ArrProduct,
            "and" => ir::SysFuncId::ArrAnd,
            "or" => ir::SysFuncId::ArrOr,
            _ => ir::SysFuncId::ArrXor,
        };
        let handle = self.push_expr(ir::Expr::Signal { net, word: None });
        self.push_expr(ir::Expr::SysFunc {
            which,
            args: vec![handle],
        })
    }

    /// The §7.12.2 ORDERING methods (`.sort()`/`.rsort()`/`.reverse()`) on a
    /// fixed-size unpacked array — again the exact `Stmt::SysTask` the queue twin
    /// in `lower_dyn_method_stmt` emits, so the engine arm is shared.
    ///
    /// Verified against verilator 5.050 (`int a[4]` = 3,1,4,2 → sort `1 2 3 4`,
    /// rsort `4 3 2 1`, reverse of the sorted array `4 3 2 1`); iverilog 13 has
    /// no fixed-array ordering method (`Enable of unknown task ``a.sort''`).
    ///
    /// The `with`-clause form (`a.sort() with (item.x)`) is NOT routed here: the
    /// queue twin refuses it too, and a comparison key is a different mechanism.
    pub(crate) fn lower_static_array_order(
        &mut self,
        b: &mut ProcessBuilder,
        net: u32,
        method: &str,
        args: &[ast::Expr],
    ) {
        if !args.is_empty() {
            self.error(
                MsgCode::ElabUnsupported,
                "array ordering methods take no arguments (the with-clause comparison \
                 key is not supported on any receiver kind)",
            );
            return;
        }
        // An ordering method WRITES its receiver, so it is a procedural
        // assignment and a `wire` array is illegal (§6.5). Measured: without
        // this, `wire [7:0] w[3]; w.sort();` was accepted at exit 0 and the
        // sort silently vanished under the continuous drivers, while verilator
        // refuses the design outright (`%Error-CONTASSINIT`) and vita's own
        // `w[0] = 8'd9;` is `E3018`. Ask the SAME rule rather than spelling a
        // second copy of it — a lone whole-net chunk is all `check_lvalue_kind`
        // reads.
        let probe = ir::Lvalue {
            chunks: vec![ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: ir::SelKind::Bit,
            }],
        };
        let before = self.error_count;
        self.check_lvalue_kind(&probe, true);
        if self.error_count != before {
            return;
        }
        let which = match method {
            "sort" => ir::SysTaskId::ArrSort,
            "rsort" => ir::SysTaskId::ArrRsort,
            _ => ir::SysTaskId::ArrReverse,
        };
        let handle = self.push_expr(ir::Expr::Signal { net, word: None });
        let sid = self.push_stmt(ir::Stmt::SysTask {
            which,
            fmt: None,
            args: vec![handle],
        });
        b.push_stmt_id(sid);
    }
}
