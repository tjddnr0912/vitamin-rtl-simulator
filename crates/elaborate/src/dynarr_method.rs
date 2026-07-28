//! dyn-handle METHOD lowering — split out of `dynarr.rs` (module-size policy: that file
//! reached the 1000-line cap). Pure mechanical move: the `.size()`/`.exists()`/reduction/
//! ordering/push/pop/insert/delete dispatch for a dynamic-storage handle, in both
//! expression and statement position.

use super::*;

impl Elaborator<'_> {
    /// Method-call EXPRESSION on a dyn handle (`d.size()`, `a.exists(k)`…).
    /// Pops reaching HERE are NOT the direct rhs of a blocking assign (that
    /// shape is intercepted in `dyn_blocking_special`) — loud, per the engine
    /// contract (`StmtEffect::QPop` is statement-level).
    pub(crate) fn lower_dyn_method_expr(
        &mut self,
        net: u32,
        kind: ir::NetKind,
        method: &str,
        args: &[ast::Expr],
    ) -> u32 {
        use ir::NetKind as K;
        let handle = self.push_expr(ir::Expr::Signal { net, word: None });
        match (method, kind) {
            ("size", _) | ("num", K::Assoc | K::AssocStr) => {
                if !args.is_empty() {
                    self.error(MsgCode::ElabUnsupported, "size()/num() take no arguments");
                }
                let which = if method == "num" {
                    ir::SysFuncId::AssocNum
                } else {
                    ir::SysFuncId::DynSize
                };
                self.push_expr(ir::Expr::SysFunc {
                    which,
                    args: vec![handle],
                })
            }
            ("exists", K::Assoc | K::AssocStr) => {
                let Some(k) = args.first() else {
                    self.error(MsgCode::ElabUnsupported, "exists() takes the key argument");
                    return self.placeholder_expr();
                };
                // r19/S6: `.exists(key)` — its siblings `lower_dyn_index` and
                // `delete(key)` were gated; this one was not, so a real key
                // returned 0 with a bogus-X warning at exit 0.
                let key = self.lower_index_expr(k);
                self.push_expr(ir::Expr::SysFunc {
                    which: ir::SysFuncId::AssocExists,
                    args: vec![handle, key],
                })
            }
            // ⓑ-breadth (v15): array reduction methods (IEEE §7.12.3). Element-
            // typed scalar result; legal on the ordered/keyed element kinds
            // (dyn array / queue / assoc) — NOT on strings (a string is a byte
            // sequence, not a numeric array; `.sum()` there is a kind error).
            (
                "sum" | "product" | "and" | "or" | "xor",
                K::DynArray | K::Queue | K::Assoc | K::AssocStr,
            ) => {
                if !args.is_empty() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "array reduction methods take no arguments (with-clause is a separate slice)",
                    );
                }
                let which = match method {
                    "sum" => ir::SysFuncId::ArrSum,
                    "product" => ir::SysFuncId::ArrProduct,
                    "and" => ir::SysFuncId::ArrAnd,
                    "or" => ir::SysFuncId::ArrOr,
                    _ => ir::SysFuncId::ArrXor,
                };
                self.push_expr(ir::Expr::SysFunc {
                    which,
                    args: vec![handle],
                })
            }
            ("pop_back" | "pop_front", K::Queue) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a queue pop is only supported as the DIRECT rhs of a blocking assignment (`x = q.pop_back();`)",
                );
                self.placeholder_expr()
            }
            // v6: the iteration methods WRITE their ref key argument — same
            // direct-rhs-only contract as the pops.
            ("first" | "next" | "last" | "prev", _) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "first/next/last/prev are only supported as the DIRECT rhs of a blocking assignment (`st = a.first(k);`)",
                );
                self.placeholder_expr()
            }
            ("push_back" | "push_front" | "delete" | "insert", _) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "statement method used in expression position",
                );
                self.placeholder_expr()
            }
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!("unknown or kind-mismatched dynamic-storage method `.{method}()`"),
                );
                self.placeholder_expr()
            }
        }
    }

    /// Method-call STATEMENT on a dyn handle (`q.push_back(v);`, `a.delete(k);`).
    pub(crate) fn lower_dyn_method_stmt(
        &mut self,
        b: &mut ProcessBuilder,
        net: u32,
        kind: ir::NetKind,
        method: &str,
        args: &[ast::Expr],
    ) {
        use ir::NetKind as K;
        let handle = self.push_expr(ir::Expr::Signal { net, word: None });
        let task = match (method, kind, args.len()) {
            ("push_back", K::Queue, 1) | ("push_front", K::Queue, 1) => {
                let v = self.lower_expr(&args[0]);
                let which = if method == "push_back" {
                    ir::SysTaskId::QPushBack
                } else {
                    ir::SysTaskId::QPushFront
                };
                ir::Stmt::SysTask {
                    which,
                    fmt: None,
                    args: vec![handle, v],
                }
            }
            ("delete", _, 0) => ir::Stmt::SysTask {
                which: ir::SysTaskId::DynDelete,
                fmt: None,
                args: vec![handle],
            },
            ("delete", K::Assoc | K::AssocStr, 1) => {
                let k = self.lower_index_expr(&args[0]);
                ir::Stmt::SysTask {
                    which: ir::SysTaskId::AssocDeleteKey,
                    fmt: None,
                    args: vec![handle, k],
                }
            }
            // v6: queue positional delete(i) — IEEE §7.10.2.3 (OOB/X index =
            // engine warn + skip).
            ("delete", K::Queue, 1) => {
                let i = self.lower_index_expr(&args[0]);
                ir::Stmt::SysTask {
                    which: ir::SysTaskId::QDeleteIdx,
                    fmt: None,
                    args: vec![handle, i],
                }
            }
            ("delete", _, 1) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "indexed delete(i) is a queue/assoc method (a dyn array only supports delete())",
                );
                return;
            }
            // v6: queue positional insert(i, v) — IEEE §7.10.2.2 (i == size
            // appends; OOB/X index = engine warn + no-op).
            ("insert", K::Queue, 2) => {
                let i = self.lower_index_expr(&args[0]);
                let v = self.lower_expr(&args[1]);
                ir::Stmt::SysTask {
                    which: ir::SysTaskId::QInsert,
                    fmt: None,
                    args: vec![handle, i, v],
                }
            }
            ("insert", K::Queue, _) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "insert() takes exactly (index, value)",
                );
                return;
            }
            // ⓑ-breadth (v16): ordering methods — in-place mutators on an
            // ORDERED collection (dyn array / queue). Assoc has no positional
            // order to sort (`.sort()` there is a kind error → the catch-all).
            ("sort" | "rsort" | "reverse", K::DynArray | K::Queue, 0) => {
                let which = match method {
                    "sort" => ir::SysTaskId::ArrSort,
                    "rsort" => ir::SysTaskId::ArrRsort,
                    _ => ir::SysTaskId::ArrReverse,
                };
                ir::Stmt::SysTask {
                    which,
                    fmt: None,
                    args: vec![handle],
                }
            }
            ("sort" | "rsort" | "reverse", K::DynArray | K::Queue, _) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "array ordering methods take no arguments (with-clause is a separate slice)",
                );
                return;
            }
            ("pop_back" | "pop_front", K::Queue, 0) => {
                // §13.4.1: `void'(q.pop_front());` — pop for the SIDE EFFECT and discard
                // the value. The parser has already lowered the `void'()` wrapper to this
                // statement form, so reaching here means the result is genuinely unwanted;
                // the AXI-model idiom "drop the head" has no other spelling.
                //
                // The engine's pop is an expression op, so the removal only happens if the
                // value is consumed — hence a per-call sink net rather than a bare op. It
                // is written and never read, which is exactly "discarded".
                let (ew, esign) = self.handle_elem_type(net).unwrap_or((32, false));
                let sink = self.nets.len() as u32;
                self.add_net(
                    // `$`-fenced like every other elaborator temp: a legal SV identifier
                    // here both DUMPED into the VCD and collided with a user declaration
                    // of the same name (review F6).
                    &format!("$popsink${sink}"),
                    ir::NetVar {
                        kind: ir::NetKind::Reg,
                        width: ew,
                        msb: ew.max(1) - 1,
                        lsb: 0,
                        signed: esign,
                        array_len: 1,
                        dir: ir::PortDir::Internal,
                        init: default_init(ast::NetVarKind::Reg, ew.max(1)),
                    },
                );
                ir::Stmt::BlockingAssign {
                    lhs: ir::Lvalue {
                        chunks: vec![ir::LvalChunk {
                            net: sink,
                            word: None,
                            offset: None,
                            width: None,
                            kind: ir::SelKind::Bit,
                        }],
                    },
                    rhs: self.push_expr(ir::Expr::SysFunc {
                        which: if method == "pop_front" {
                            ir::SysFuncId::QPopFront
                        } else {
                            ir::SysFuncId::QPopBack
                        },
                        args: vec![handle],
                    }),
                }
            }
            ("pop_back" | "pop_front", K::Queue, _) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "pop_back()/pop_front() take no arguments",
                );
                return;
            }
            ("size" | "num" | "exists", _, _) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "value-returning method used as a statement",
                );
                return;
            }
            // v6: an iteration call whose status is discarded — loud (the
            // result drives the walk; dropping it is almost surely a bug).
            ("first" | "next" | "last" | "prev", _, _) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "first/next/last/prev results must be assigned (`st = a.first(k);`)",
                );
                return;
            }
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!("unknown or kind-mismatched dynamic-storage method `.{method}()`"),
                );
                return;
            }
        };
        let sid = self.push_stmt(task);
        b.push_stmt_id(sid);
    }
}
