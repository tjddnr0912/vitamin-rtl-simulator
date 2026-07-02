//! elaborate v1 tests — build a small hdl-ast by hand, elaborate, assert SimIr.

use std::cell::RefCell;

use diag::{LogEvent, LogSink};
use hdl_ast as ast;

use super::*;
use crate::literal::parse_int_literal;

// ── a collecting LogSink (interior mutability: emit takes &self) ──
#[derive(Default)]
struct CollectSink {
    events: RefCell<Vec<LogEvent>>,
}
impl LogSink for CollectSink {
    fn emit(&self, event: LogEvent) {
        self.events.borrow_mut().push(event);
    }
}
// ── tiny AST builders ──
const SP: ast::Span = ast::Span { lo: 0, hi: 0 };

fn ident(name: &str) -> ast::Ident {
    ast::Ident {
        name: name.to_string(),
        span: SP,
    }
}
fn hpath(name: &str) -> ast::HierPath {
    ast::HierPath {
        segments: vec![ident(name)],
        span: SP,
    }
}
fn ex(kind: ast::ExprKind) -> ast::Expr {
    ast::Expr { kind, span: SP }
}
fn id_expr(name: &str) -> ast::Expr {
    ex(ast::ExprKind::Ident(hpath(name)))
}
fn lit(raw: &str, kind: ast::IntLitKind) -> ast::Expr {
    ex(ast::ExprKind::IntLit {
        kind,
        raw: raw.to_string(),
    })
}
fn dec(n: &str) -> ast::Expr {
    lit(n, ast::IntLitKind::Decimal)
}
fn binop(op: ast::BinOp, l: ast::Expr, r: ast::Expr) -> ast::Expr {
    ex(ast::ExprKind::Binary {
        op,
        lhs: Box::new(l),
        rhs: Box::new(r),
    })
}

/// `wire [msb:lsb] names...;` (msb/lsb are decimal literals)
fn wire_vec(msb: u32, lsb: u32, names: &[&str]) -> ast::ModuleItem {
    netvar(ast::NetVarKind::Wire, Some((msb, lsb)), false, names)
}
fn netvar(
    kind: ast::NetVarKind,
    range: Option<(u32, u32)>,
    signed: bool,
    names: &[&str],
) -> ast::ModuleItem {
    let range = range.map(|(m, l)| ast::Range {
        msb: dec(&m.to_string()),
        lsb: dec(&l.to_string()),
        span: SP,
    });
    ast::ModuleItem::NetVar(ast::NetVarDecl {
        kind,
        signed,
        range,
        packed: Vec::new(),
        delay: None,
        names: names
            .iter()
            .map(|n| ast::DeclName {
                name: ident(n),
                unpacked: Vec::new(),
                init: None,
                span: SP,
            })
            .collect(),
        lifetime: None,
        class_type: None,
        class_args: Vec::new(),
        const_param: false,
        span: SP,
    })
}

/// `assign <lhs> = <rhs>;`
fn cont_assign(lhs: ast::Lvalue, rhs: ast::Expr) -> ast::ModuleItem {
    ast::ModuleItem::ContAssign(ast::ContinuousAssign {
        delay: None,
        assigns: vec![(lhs, rhs)],
        span: SP,
    })
}
fn lv_id(name: &str) -> ast::Lvalue {
    ast::Lvalue::Ident(hpath(name))
}

fn module(name: &str, body: Vec<ast::ModuleItem>) -> ast::SourceUnit {
    ast::SourceUnit {
        items: vec![ast::TopItem::Module(ast::ModuleDecl {
            is_macromodule: false,
            name: ident(name),
            params: Vec::new(),
            ports: ast::PortList::None,
            body,
            span: SP,
        })],
        span: SP,
    }
}

fn elab_ok(unit: &ast::SourceUnit) -> ir::SimIr {
    let sink = CollectSink::default();
    let ir = elaborate(unit, &sink);
    let diags: Vec<String> = sink
        .events
        .borrow()
        .iter()
        .filter_map(|e| match e {
            LogEvent::Diagnostic(d) => Some(format!("{}: {}", d.code.code_num(), d.message)),
            _ => None,
        })
        .collect();
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    ir.expect("elaborate returned None on clean input")
}

// ───────────────────────── 1. driver / nets ─────────────────────────
#[test]
fn t1_nets_in_decl_order_and_self_instance() {
    // module m; wire [7:0] a,b,y; assign y = a & b | 8'hF0;
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "b", "y"]),
            cont_assign(
                lv_id("y"),
                binop(
                    ast::BinOp::BitOr,
                    binop(ast::BinOp::BitAnd, id_expr("a"), id_expr("b")),
                    lit("8'hF0", ast::IntLitKind::Sized),
                ),
            ),
        ],
    );
    let s = elab_ok(&unit);

    // nets a,b,y in order, all width 8.
    assert_eq!(s.nets.len(), 3);
    for n in &s.nets {
        assert_eq!(n.width, 8);
        assert_eq!(n.msb, 7);
        assert_eq!(n.lsb, 0);
        assert_eq!(n.kind, ir::NetKind::Wire);
    }
    // one self-instance covering all 3 nets.
    assert_eq!(s.instances.len(), 1);
    assert_eq!(s.instances[0].first_net, 0);
    assert_eq!(s.instances[0].net_count, 3);
    assert!(s.instances[0].parent.is_none());

    // exactly one cont_assign onto net y (=2).
    assert_eq!(s.cont_assigns.len(), 1);
    let ca = &s.cont_assigns[0];
    assert_eq!(ca.lhs.chunks.len(), 1);
    assert_eq!(ca.lhs.chunks[0].net, 2); // y is the 3rd net
    assert!(ca.delay.is_none());

    // rhs is the top Binary(BitOr,...). Walk the arena root.
    let root = &s.exprs[ca.rhs as usize];
    match root {
        ir::Expr::Binary {
            op: ir::BinOp::BitOr,
            lhs,
            rhs,
        } => {
            // lhs = Binary(BitAnd, Signal a, Signal b)
            match &s.exprs[*lhs as usize] {
                ir::Expr::Binary {
                    op: ir::BinOp::BitAnd,
                    lhs: l2,
                    rhs: r2,
                } => {
                    assert!(matches!(
                        s.exprs[*l2 as usize],
                        ir::Expr::Signal { net: 0, word: None }
                    ));
                    assert!(matches!(
                        s.exprs[*r2 as usize],
                        ir::Expr::Signal { net: 1, word: None }
                    ));
                }
                other => panic!("expected BitAnd, got {other:?}"),
            }
            // rhs = Const 8'hF0
            match &s.exprs[*rhs as usize] {
                ir::Expr::Const { val } => {
                    let cv = &s.consts[*val as usize];
                    assert_eq!(cv.width, 8);
                    assert_eq!(cv.bits.val[0], 0xF0);
                    assert_eq!(cv.bits.unk[0], 0x00);
                }
                other => panic!("expected Const, got {other:?}"),
            }
        }
        other => panic!("expected BitOr root, got {other:?}"),
    }
}

// ───────────────────────── 2. post-order is fixed ─────────────────────────
#[test]
fn t2_postorder_indices_children_before_parent() {
    // y = a + b  → arena: [Signal a, Signal b, Binary] (root index 2).
    let unit = module(
        "m",
        vec![
            wire_vec(0, 0, &["a", "b", "y"]),
            cont_assign(
                lv_id("y"),
                binop(ast::BinOp::Add, id_expr("a"), id_expr("b")),
            ),
        ],
    );
    let s = elab_ok(&unit);
    assert_eq!(s.exprs.len(), 3);
    assert!(matches!(s.exprs[0], ir::Expr::Signal { net: 0, .. }));
    assert!(matches!(s.exprs[1], ir::Expr::Signal { net: 1, .. }));
    let root = s.cont_assigns[0].rhs;
    assert_eq!(root, 2);
    assert!(matches!(
        s.exprs[2],
        ir::Expr::Binary {
            op: ir::BinOp::Add,
            lhs: 0,
            rhs: 1
        }
    ));
}

// ───────────────────────── 3. reg default init = all-X ─────────────────────────
#[test]
fn t3_reg_default_init_is_x_wire_is_z() {
    let unit = module(
        "m",
        vec![
            netvar(ast::NetVarKind::Reg, Some((3, 0)), false, &["r"]),
            wire_vec(3, 0, &["w"]),
        ],
    );
    let s = elab_ok(&unit);
    // reg r: all-X → val 0, unk 0xF (4 bits)
    let r = &s.nets[0];
    assert_eq!(r.kind, ir::NetKind::Reg);
    assert_eq!(r.init.val[0], 0x0);
    assert_eq!(r.init.unk[0], 0xF);
    // wire w: all-Z → val 0xF, unk 0xF
    let w = &s.nets[1];
    assert_eq!(w.init.val[0], 0xF);
    assert_eq!(w.init.unk[0], 0xF);
}

// ───────────────────────── 4. integer is fixed 32-bit signed ─────────────────────────
#[test]
fn t4_integer_is_32bit_signed() {
    // `integer` defaults SIGNED — the parser resolves the default via `signed_eff`,
    // so a faithful decl carries signed=true (range_to_dims now honors the flag
    // rather than hardcoding it, so `integer unsigned` can be unsigned).
    let unit = module(
        "m",
        vec![netvar(ast::NetVarKind::Integer, None, true, &["i"])],
    );
    let s = elab_ok(&unit);
    let i = &s.nets[0];
    assert_eq!(i.kind, ir::NetKind::Integer);
    assert_eq!(i.width, 32);
    assert_eq!(i.msb, 31);
    assert_eq!(i.lsb, 0);
    assert!(i.signed);
}

// ───────────────────────── 5. const dedup ─────────────────────────
#[test]
fn t5_const_dedup() {
    // y = a & 8'hFF | 8'hFF  → the two 8'hFF literals share ONE const slot.
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "y"]),
            cont_assign(
                lv_id("y"),
                binop(
                    ast::BinOp::BitOr,
                    binop(
                        ast::BinOp::BitAnd,
                        id_expr("a"),
                        lit("8'hFF", ast::IntLitKind::Sized),
                    ),
                    lit("8'hFF", ast::IntLitKind::Sized),
                ),
            ),
        ],
    );
    let s = elab_ok(&unit);
    // exactly one ConstVal in the pool (8'hFF), even though two Const exprs.
    assert_eq!(s.consts.len(), 1);
    assert_eq!(s.consts[0].bits.val[0], 0xFF);
    let n_const_exprs = s
        .exprs
        .iter()
        .filter(|e| matches!(e, ir::Expr::Const { .. }))
        .count();
    assert_eq!(n_const_exprs, 2);
}

// ───────────────────────── 6. part-select RHS ─────────────────────────
#[test]
fn t6_part_select_rhs() {
    // y = a[5:2]
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a"]),
            wire_vec(3, 0, &["y"]),
            cont_assign(
                lv_id("y"),
                ex(ast::ExprKind::PartSelect {
                    base: Box::new(id_expr("a")),
                    msb: Box::new(dec("5")),
                    lsb: Box::new(dec("2")),
                }),
            ),
        ],
    );
    let s = elab_ok(&unit);
    let root = &s.exprs[s.cont_assigns[0].rhs as usize];
    match root {
        ir::Expr::Select {
            base,
            offset,
            width,
            kind: ir::SelKind::PartConst,
        } => {
            // base is Signal a (net 0)
            assert!(matches!(
                s.exprs[*base as usize],
                ir::Expr::Signal { net: 0, .. }
            ));
            // offset is Const 2
            assert!(matches!(s.exprs[*offset as usize], ir::Expr::Const { .. }));
            // width is a (msb - lsb) + 1 Binary(Add) tree
            assert!(matches!(
                s.exprs[*width as usize],
                ir::Expr::Binary {
                    op: ir::BinOp::Add,
                    ..
                }
            ));
        }
        other => panic!("expected PartConst Select, got {other:?}"),
    }
}

// ───────────────────────── 7. concat LHS contassign ─────────────────────────
#[test]
fn t7_concat_lhs() {
    // {cout, sum} = a  → two LvalChunks (cout MSB-first, then sum).
    let unit = module(
        "m",
        vec![
            wire_vec(0, 0, &["cout"]),
            wire_vec(7, 0, &["sum", "a"]),
            cont_assign(
                ast::Lvalue::Concat {
                    parts: vec![lv_id("cout"), lv_id("sum")],
                    span: SP,
                },
                id_expr("a"),
            ),
        ],
    );
    let s = elab_ok(&unit);
    let lhs = &s.cont_assigns[0].lhs;
    assert_eq!(lhs.chunks.len(), 2);
    assert_eq!(lhs.chunks[0].net, 0); // cout
    assert_eq!(lhs.chunks[1].net, 1); // sum
                                      // both are whole-net chunks (offset/width None).
    assert!(lhs.chunks[0].offset.is_none());
    assert!(lhs.chunks[1].offset.is_none());
}

// ───────────────────────── 8. concat RHS + replicate ─────────────────────────
#[test]
fn t8_concat_and_replicate_rhs() {
    // y = {2{a}, b}  → Concat[ Replicate{2,Concat[a]}, b ]
    let unit = module(
        "m",
        vec![
            wire_vec(0, 0, &["a", "b"]),
            wire_vec(2, 0, &["y"]),
            cont_assign(
                lv_id("y"),
                ex(ast::ExprKind::Concat {
                    parts: vec![
                        ex(ast::ExprKind::Replicate {
                            count: Box::new(dec("2")),
                            value: vec![id_expr("a")],
                        }),
                        id_expr("b"),
                    ],
                }),
            ),
        ],
    );
    let s = elab_ok(&unit);
    let root = &s.exprs[s.cont_assigns[0].rhs as usize];
    match root {
        ir::Expr::Concat { parts } => {
            assert_eq!(parts.len(), 2);
            // part 0 is a Replicate whose value is a 1-part Concat
            match &s.exprs[parts[0] as usize] {
                ir::Expr::Replicate { count, value } => {
                    assert!(matches!(s.exprs[*count as usize], ir::Expr::Const { .. }));
                    match &s.exprs[*value as usize] {
                        ir::Expr::Concat { parts: rp } => {
                            assert_eq!(rp.len(), 1);
                            assert!(matches!(
                                s.exprs[rp[0] as usize],
                                ir::Expr::Signal { net: 0, .. }
                            ));
                        }
                        other => panic!("replicate value not Concat: {other:?}"),
                    }
                }
                other => panic!("part0 not Replicate: {other:?}"),
            }
            // part 1 is Signal b
            assert!(matches!(
                s.exprs[parts[1] as usize],
                ir::Expr::Signal { net: 1, .. }
            ));
        }
        other => panic!("expected Concat root, got {other:?}"),
    }
}

// ───────────────────────── 9. unresolved name → error + None ─────────────────────────
#[test]
fn t9_unresolved_name_errors() {
    // y = z  (z undeclared)
    let unit = module(
        "m",
        vec![
            wire_vec(0, 0, &["y"]),
            cont_assign(lv_id("y"), id_expr("z")),
        ],
    );
    let sink = CollectSink::default();
    let out = elaborate(&unit, &sink);
    assert!(out.is_none(), "should fail on unresolved name");
    // exactly one diagnostic, code ElabUnresolvedName.
    let events = sink.events.borrow();
    let diags: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            LogEvent::Diagnostic(d) => Some(d),
            _ => None,
        })
        .collect();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, MsgCode::ElabUnresolvedName);
}

// ───────────────────────── 10. procedural block → unsupported ─────────────────────────
#[test]
fn t10_procedural_block_unsupported() {
    let proc = ast::ModuleItem::Proc(ast::ProceduralBlock {
        kind: ast::ProcKind::Always,
        sensitivity: None,
        body: Box::new(ast::Stmt::Null(SP)),
        span: SP,
    });
    let unit = module("m", vec![wire_vec(0, 0, &["a"]), proc]);
    let sink = CollectSink::default();
    let out = elaborate(&unit, &sink);
    assert!(
        out.is_some(),
        "bare always (no timing) is now a non-fatal warning, not fatal"
    );
    assert!(sink.events.borrow().iter().any(|e| matches!(
        e, LogEvent::Diagnostic(d) if d.severity == diag::Severity::Warning)));
}

// ───────────────────────── 11. literal-parse planes ─────────────────────────
#[test]
fn t11_literal_4state_planes() {
    // 4'b10xz : bit0=z=(1,1) bit1=x=(0,1) bit2=0=(0,0) bit3=1=(1,0)
    //   val: b0=1,b1=0,b2=0,b3=1 → 0b1001 = 0x9
    //   unk: b0=1,b1=1,b2=0,b3=0 → 0b0011 = 0x3
    let cv = parse_int_literal("4'b10xz", ast::IntLitKind::Sized).unwrap();
    assert_eq!(cv.width, 4);
    assert!(!cv.signed);
    assert_eq!(cv.bits.val[0], 0x9);
    assert_eq!(cv.bits.unk[0], 0x3);

    // 8'hF0 : clean 2-state
    let cv = parse_int_literal("8'hF0", ast::IntLitKind::Sized).unwrap();
    assert_eq!(cv.bits.val[0], 0xF0);
    assert_eq!(cv.bits.unk[0], 0x00);

    // 4'sd5 : signed decimal, width 4
    let cv = parse_int_literal("4'sd5", ast::IntLitKind::Sized).unwrap();
    assert!(cv.signed);
    assert_eq!(cv.bits.val[0], 0x5);
    assert_eq!(cv.bits.unk[0], 0x0);

    // 4'bx : x-extends to 4 bits → val 0, unk 0xF
    let cv = parse_int_literal("4'bx", ast::IntLitKind::Sized).unwrap();
    assert_eq!(cv.bits.val[0], 0x0);
    assert_eq!(cv.bits.unk[0], 0xF);

    // 8'hzz : all-Z → val 0xFF, unk 0xFF
    let cv = parse_int_literal("8'hzz", ast::IntLitKind::Sized).unwrap();
    assert_eq!(cv.bits.val[0], 0xFF);
    assert_eq!(cv.bits.unk[0], 0xFF);

    // 4'bz0 : §3.5.1 z-extension. b0=0=(0,0), b1=z=(1,1), extend b2,b3 = z=(1,1)
    //   val: b0=0,b1=1,b2=1,b3=1 → 0xE ; unk: 0,1,1,1 → 0xE
    let cv = parse_int_literal("4'bz0", ast::IntLitKind::Sized).unwrap();
    assert_eq!(cv.bits.val[0], 0xE);
    assert_eq!(cv.bits.unk[0], 0xE);

    // plain decimal 42 → 32-bit signed, val 0x2A
    let cv = parse_int_literal("42", ast::IntLitKind::Decimal).unwrap();
    assert_eq!(cv.width, 32);
    assert!(cv.signed);
    assert_eq!(cv.bits.val[0], 0x2A);
    assert_eq!(cv.bits.unk[0], 0x0);

    // unsized 'hFF → 32-bit unsigned, zero-extended
    let cv = parse_int_literal("'hFF", ast::IntLitKind::UnsizedBased).unwrap();
    assert_eq!(cv.width, 32);
    assert!(!cv.signed);
    assert_eq!(cv.bits.val[0], 0xFF);

    // 32'hDEAD_BEEF → underscore stripped
    let cv = parse_int_literal("32'hDEAD_BEEF", ast::IntLitKind::Sized).unwrap();
    assert_eq!(cv.bits.val[0], 0xDEAD_BEEF);
    assert_eq!(cv.bits.unk[0], 0x0);

    // SV single-char fill 'x → all-X over 32 bits
    let cv = parse_int_literal("'x", ast::IntLitKind::UnsizedBased).unwrap();
    assert_eq!(cv.bits.val[0], 0x0);
    assert_eq!(cv.bits.unk[0], 0xFFFF_FFFF);
}

// ─────────── 11b. unsized literal width grows to hold the value (P0-10) ───────────
// IEEE §3.5.1: an unsized literal is "at least 32 bits", grown to hold its value.
// Every width below is pinned LIVE against iverilog 13.0 ($bits): plain decimal &
// 'sd are SIGNED (a positive value needs a sign bit → msb+2); 'd is UNSIGNED
// (msb+1); based h/b/o use the DIGIT span (not the value MSB). Pre-P0-10 these all
// truncated to a fixed 32 bits (2^31 → -2^31, 2^32 → 0).
#[test]
fn t11b_unsized_literal_width_grows() {
    let dec = |s: &str| parse_int_literal(s, ast::IntLitKind::Decimal).unwrap();
    let based = |s: &str| parse_int_literal(s, ast::IntLitKind::UnsizedBased).unwrap();

    // plain decimal (signed): width = max(32, msb+2)
    assert_eq!(dec("2147483647").width, 32); // 2^31-1, msb=30 → 32
    assert_eq!(dec("2147483648").width, 33); // 2^31,   msb=31 → 33
    assert_eq!(dec("4294967295").width, 33); // 2^32-1, msb=31 → 33
    assert_eq!(dec("4294967296").width, 34); // 2^32,   msb=32 → 34
    assert_eq!(dec("8589934592").width, 35); // 2^33,   msb=33 → 35
    assert_eq!(dec("42").width, 32); // small stays 32 (byte-identical)

    // values are PRESERVED (positive sign bit = 0), signed flag stays true
    let cv = dec("2147483648");
    assert!(cv.signed);
    assert_eq!(cv.bits.val[0], 0x8000_0000); // bit31 set, sign bit (32) clear
    assert_eq!(cv.bits.unk[0], 0x0);
    let cv = dec("4294967296");
    assert_eq!(cv.bits.val[0], 0x1_0000_0000); // bit32 set
    let cv = dec("8589934592");
    assert_eq!(cv.bits.val[0], 0x2_0000_0000); // bit33 set

    // 'd (unsigned decimal): width = max(32, msb+1)
    assert_eq!(based("'d2147483648").width, 32); // 2^31 fits in 32 unsigned
    assert_eq!(based("'d4294967296").width, 33); // 2^32 → 33
    assert!(!based("'d2147483648").signed);

    // 'sd (signed decimal): like plain decimal → msb+2
    assert_eq!(based("'sd2147483648").width, 33);
    assert!(based("'sd2147483648").signed);

    // based h/b/o: DIGIT span, not value MSB
    assert_eq!(based("'hFFFFFFFF").width, 32); // 8 hex digits = 32 bits
    assert_eq!(based("'h1FFFFFFFF").width, 36); // 9 hex digits = 36 bits (iverilog)
    assert_eq!(based("'hFF").width, 32); // 2 digits, min 32
    assert_eq!(based("'sh1FFFFFFFF").width, 36); // signed h: still digit span (no +1)
    let cv = based("'h1FFFFFFFF");
    assert_eq!(cv.bits.val[0], 0x1_FFFF_FFFF);
    assert_eq!(cv.bits.unk[0], 0);

    // sized literals are UNCHANGED (explicit width wins)
    assert_eq!(
        parse_int_literal("8'hFF", ast::IntLitKind::Sized)
            .unwrap()
            .width,
        8
    );
    assert_eq!(
        parse_int_literal("64'd1099511627776", ast::IntLitKind::Sized)
            .unwrap()
            .width,
        64
    );
}

// ─────────── 11c. decimal → multi-WORD bit image (P-perf refactor guard) ───────────
// The decimal magnitude conversion is the only path that crosses 64-bit word
// boundaries with a carry, yet every other literal test is single-word. These
// pin the exact (val plane) image of decimals whose MSB lands in word 1 / word 2,
// so a faster base-conversion (Horner limbs) that mis-propagates a limb carry is
// caught. Widths follow §3.5.1 (plain decimal signed → msb+2). Values verified
// against iverilog 13.0 ($bits + the exact magnitude).
#[test]
fn t11c_decimal_multiword_bits() {
    let dec = |s: &str| parse_int_literal(s, ast::IntLitKind::Decimal).unwrap();

    // 2^64 = 18446744073709551616 : bit64 set → bits.len()=65, width=66, val=[0,1]
    let cv = dec("18446744073709551616");
    assert_eq!(cv.width, 66);
    assert_eq!(cv.bits.val, vec![0, 1]);
    assert_eq!(cv.bits.unk, vec![0, 0]);

    // 2^64 + 1 : both word0 bit0 and word1 bit0 set → val=[1,1]
    let cv = dec("18446744073709551617");
    assert_eq!(cv.width, 66);
    assert_eq!(cv.bits.val, vec![1, 1]);

    // 2^64 - 1 = 18446744073709551615 : 64 ones in word0, msb=63 → width 65, two words
    let cv = dec("18446744073709551615");
    assert_eq!(cv.width, 65);
    assert_eq!(cv.bits.val, vec![u64::MAX, 0]);

    // a value spanning words 0 AND 1 with a non-trivial high word (0xDEADBEEF):
    // 0xDEADBEEF * 2^64 + 1 = 68915718005535514953299001345 (python-verified).
    let cv = dec("68915718005535514953299001345");
    assert_eq!(cv.bits.val, vec![1, 0xDEAD_BEEF]);
    // MSB of 0xDEADBEEF is bit 31 → absolute bit 64+31 = 95 → bits.len()=96, width 97
    assert_eq!(cv.width, 97);

    // 2^128 = 340282366920938463463374607431768211456 : bit128 → 3 words, width 130
    let cv = dec("340282366920938463463374607431768211456");
    assert_eq!(cv.width, 130);
    assert_eq!(cv.bits.val, vec![0, 0, 1]);
    assert_eq!(cv.bits.unk, vec![0, 0, 0]);

    // a true 3-word value, every word non-trivial (carry must thread word0→1→2):
    // 0xCAFE*2^128 + 0xBEEF*2^64 + 0x1234 = 17683113479413488193239383253378116049965620
    let cv = dec("17683113479413488193239383253378116049965620");
    assert_eq!(cv.bits.val, vec![0x1234, 0xBEEF, 0xCAFE]);
    assert_eq!(cv.width, 145); // MSB of 0xCAFE = bit15 → 128+15=143 → bits.len()=144 → +1
}

// ───────────────────────── 12. determinism: identical input → identical IR ─────────────────────────
#[test]
fn t12_determinism_repeatable() {
    let build = || {
        module(
            "m",
            vec![
                wire_vec(7, 0, &["a", "b", "y"]),
                cont_assign(
                    lv_id("y"),
                    binop(
                        ast::BinOp::BitOr,
                        binop(ast::BinOp::BitAnd, id_expr("a"), id_expr("b")),
                        lit("8'hF0", ast::IntLitKind::Sized),
                    ),
                ),
            ],
        )
    };
    let s1 = elab_ok(&build());
    let s2 = elab_ok(&build());
    // structural equality (sim-ir derives PartialEq) — same arena, same order.
    assert_eq!(s1, s2);
}

// ───────────────────────── 13. bit-select LHS ─────────────────────────
#[test]
fn t13_bit_select_lhs() {
    // a[3] = b
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a"]),
            wire_vec(0, 0, &["b"]),
            cont_assign(
                ast::Lvalue::BitSelect {
                    base: Box::new(lv_id("a")),
                    index: Box::new(dec("3")),
                    span: SP,
                },
                id_expr("b"),
            ),
        ],
    );
    let s = elab_ok(&unit);
    let chunk = &s.cont_assigns[0].lhs.chunks[0];
    assert_eq!(chunk.net, 0); // a
    assert_eq!(chunk.kind, ir::SelKind::Bit);
    assert!(chunk.word.is_none()); // a is scalar array (len 1) → bit select
    assert!(chunk.offset.is_some());
    assert!(chunk.width.is_some());
}

// ── array (memory) builder: `reg [bw:0] name [0:depth-1];` ──
fn logic_mem(bit_msb: u32, depth_msb: u32, name: &str) -> ast::ModuleItem {
    let ast::ModuleItem::NetVar(mut d) = reg_mem(bit_msb, depth_msb, name) else {
        unreachable!()
    };
    d.kind = ast::NetVarKind::Logic;
    ast::ModuleItem::NetVar(d)
}

fn reg_mem(bit_msb: u32, depth_msb: u32, name: &str) -> ast::ModuleItem {
    ast::ModuleItem::NetVar(ast::NetVarDecl {
        kind: ast::NetVarKind::Reg,
        signed: false,
        range: Some(ast::Range {
            msb: dec(&bit_msb.to_string()),
            lsb: dec("0"),
            span: SP,
        }),
        packed: Vec::new(),
        delay: None,
        names: vec![ast::DeclName {
            name: ident(name),
            unpacked: vec![ast::Dim::Range(ast::Range {
                msb: dec(&depth_msb.to_string()),
                lsb: dec("0"),
                span: SP,
            })],
            init: None,
            span: SP,
        }],
        lifetime: None,
        class_type: None,
        class_args: Vec::new(),
        const_param: false,
        span: SP,
    })
}

// ───────────────────────── 14. RHS memory word-select → Signal{word} ─────────────────────────
#[test]
fn t14_rhs_memory_word_select_is_signal_word() {
    // reg [7:0] mem [0:3]; wire [7:0] y; assign y = mem[2];
    // mem[2] on the RHS MUST lower to Signal{net, word:Some(2)} — symmetric with
    // the LHS — NOT Select{kind:Bit} (which would read bit 2 of the whole memory).
    let unit = module(
        "m",
        vec![
            reg_mem(7, 3, "mem"),
            wire_vec(7, 0, &["y"]),
            cont_assign(
                lv_id("y"),
                ex(ast::ExprKind::BitSelect {
                    base: Box::new(id_expr("mem")),
                    index: Box::new(dec("2")),
                }),
            ),
        ],
    );
    let s = elab_ok(&unit);
    // mem is net 0 with array_len 4.
    assert_eq!(s.nets[0].array_len, 4);
    let root = &s.exprs[s.cont_assigns[0].rhs as usize];
    // `word` is now an ExprId (the index expression), so `mem[k]` with runtime `k`
    // works. For the const `mem[2]` it points at a Const whose value is 2.
    let word_eid = match root {
        ir::Expr::Signal {
            net: 0,
            word: Some(w),
        } => *w,
        _ => panic!("RHS mem[2] must be Signal{{net:0, word:Some(exprid)}}, got {root:?}"),
    };
    let word_const = match &s.exprs[word_eid as usize] {
        ir::Expr::Const { val } => s.consts[*val as usize]
            .bits
            .val
            .first()
            .copied()
            .unwrap_or(0),
        other => panic!("word index must be a Const, got {other:?}"),
    };
    assert_eq!(word_const, 2, "mem[2] word index must evaluate to 2");
    // and there is NO Select{kind:Bit} in the arena for this read.
    assert!(
        !s.exprs.iter().any(|e| matches!(
            e,
            ir::Expr::Select {
                kind: ir::SelKind::Bit,
                ..
            }
        )),
        "memory word read must not emit a bit Select"
    );

    // LHS symmetry: `mem[1] = y` → LvalChunk{word:Some(1)}. The array is a SV
    // `logic` (one continuous driver is legal — E3018 rejects `assign` to a reg,
    // so the old reg fixture became an illegal-code fixture).
    let unit2 = module(
        "m",
        vec![
            logic_mem(7, 3, "mem"),
            wire_vec(7, 0, &["y"]),
            cont_assign(
                ast::Lvalue::BitSelect {
                    base: Box::new(lv_id("mem")),
                    index: Box::new(dec("1")),
                    span: SP,
                },
                id_expr("y"),
            ),
        ],
    );
    let s2 = elab_ok(&unit2);
    let chunk = &s2.cont_assigns[0].lhs.chunks[0];
    assert_eq!(chunk.net, 0);
    // `word` is an ExprId (the index expr) — for `mem[1]` it points at a Const 1.
    let w_eid = chunk
        .word
        .expect("mem[1] LHS must carry a word index ExprId");
    let w_const = match &s2.exprs[w_eid as usize] {
        ir::Expr::Const { val } => s2.consts[*val as usize]
            .bits
            .val
            .first()
            .copied()
            .unwrap_or(0),
        other => panic!("LHS word index must be a Const, got {other:?}"),
    };
    assert_eq!(w_const, 1, "mem[1] LHS word index must evaluate to 1");
    assert!(chunk.offset.is_none() && chunk.width.is_none());
}

// ───────────────────────── 15. duplicate net name → error ─────────────────────────
#[test]
fn t15_duplicate_net_name_errors() {
    // wire a; wire [7:0] a;  → second `a` is a duplicate decl.
    let unit = module("m", vec![wire_vec(0, 0, &["a"]), wire_vec(7, 0, &["a"])]);
    let sink = CollectSink::default();
    let out = elaborate(&unit, &sink);
    assert!(out.is_none(), "duplicate decl must fail elaboration");
    // exactly one net survives (the orphan is NOT pushed → net_count stays 1).
    let events = sink.events.borrow();
    let diags: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            LogEvent::Diagnostic(d) => Some(d),
            _ => None,
        })
        .collect();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, MsgCode::ElabUnsupported);
}

// ───────────────────────── 16. multidriver: whole-net legal, partial-overlap error ─────────────────────────
fn multidriver_codes(unit: &ast::SourceUnit) -> (bool, Vec<MsgCode>) {
    let sink = CollectSink::default();
    let out = elaborate(unit, &sink);
    let codes = sink
        .events
        .borrow()
        .iter()
        .filter_map(|e| match e {
            LogEvent::Diagnostic(d) => Some(d.code),
            _ => None,
        })
        .collect();
    (out.is_some(), codes)
}

#[test]
fn t16a_whole_net_multidriver_now_legal() {
    // `wire a,b,y; assign y = a; assign y = b;` — two WHOLE-NET non-delayed
    // drivers are a tristate/bus pattern, RESOLVED by the engine (4-state wire
    // resolution), so elaboration must SUCCEED with no ElabMultidriver. (Was a
    // hard E3001 reject before the multi-driver feature.)
    let unit = module(
        "m",
        vec![
            wire_vec(0, 0, &["a", "b", "y"]),
            cont_assign(lv_id("y"), id_expr("a")),
            cont_assign(lv_id("y"), id_expr("b")),
        ],
    );
    let (ok, codes) = multidriver_codes(&unit);
    assert!(ok, "whole-net multidriver must now elaborate");
    assert!(
        !codes.contains(&MsgCode::ElabMultidriver),
        "no E3001 for an all-whole-net multidriver"
    );
}

#[test]
fn t16b_partial_overlap_still_errors() {
    // `wire [7:0] a,y; wire b; assign y = a; assign y[3] = b;` — a whole-net
    // driver overlapping a PARTIAL (bit-select) driver is OUT OF SCOPE (the
    // engine resolves only all-whole-net nets), so it must still loud-reject.
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "y"]),
            wire_vec(0, 0, &["b"]),
            cont_assign(lv_id("y"), id_expr("a")),
            cont_assign(
                ast::Lvalue::BitSelect {
                    base: Box::new(lv_id("y")),
                    index: Box::new(dec("3")),
                    span: SP,
                },
                id_expr("b"),
            ),
        ],
    );
    let (ok, codes) = multidriver_codes(&unit);
    assert!(!ok, "partial-overlap multidriver must still fail");
    assert!(codes.contains(&MsgCode::ElabMultidriver));
}

// ───────────────────────── 17. hostile declared width → no panic, ElabUnsupported ─────────────────────────
#[test]
fn t17_huge_width_no_panic() {
    // wire [4294967295:0] big;  → width = u32::MAX + 1 would overflow/OOM.
    // Must be rejected with ElabUnsupported, NOT panic.
    let unit = module(
        "m",
        vec![netvar(
            ast::NetVarKind::Wire,
            Some((u32::MAX, 0)),
            false,
            &["big"],
        )],
    );
    let sink = CollectSink::default();
    let out = elaborate(&unit, &sink); // must return (not panic)
    assert!(out.is_none());
    let events = sink.events.borrow();
    let codes: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            LogEvent::Diagnostic(d) => Some(d.code),
            _ => None,
        })
        .collect();
    assert!(codes.contains(&MsgCode::ElabUnsupported));
}

// ───────────────────────── 18. descending-range part-select guard ─────────────────────────
#[test]
fn t18_ascending_part_select_unsupported() {
    // wire [7:0] a; wire [3:0] y; assign y = a[2:5];  (msb<lsb → ascending)
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a"]),
            wire_vec(3, 0, &["y"]),
            cont_assign(
                lv_id("y"),
                ex(ast::ExprKind::PartSelect {
                    base: Box::new(id_expr("a")),
                    msb: Box::new(dec("2")), // msb < lsb
                    lsb: Box::new(dec("5")),
                }),
            ),
        ],
    );
    let sink = CollectSink::default();
    let out = elaborate(&unit, &sink);
    assert!(out.is_none());
    let events = sink.events.borrow();
    let codes: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            LogEvent::Diagnostic(d) => Some(d.code),
            _ => None,
        })
        .collect();
    assert!(codes.contains(&MsgCode::ElabUnsupported));
}

// ════════════════════════════════════════════════════════════════════
//  v2 — procedural-block lowering tests
// ════════════════════════════════════════════════════════════════════

impl CollectSink {
    /// Count WARNING-severity diagnostics (non-fatal degrade channel).
    fn n_warnings(&self) -> usize {
        self.events
            .borrow()
            .iter()
            .filter(|e| {
                matches!(
                    e, LogEvent::Diagnostic(d) if d.severity == diag::Severity::Warning
                )
            })
            .count()
    }
}

/// Elaborate, allowing warnings but no errors → returns the SimIr.
fn elab_with_warnings(unit: &ast::SourceUnit) -> (ir::SimIr, usize) {
    let sink = CollectSink::default();
    let ir = elaborate(unit, &sink).expect("non-fatal lowering must yield Some(SimIr)");
    let warns = sink.n_warnings();
    (ir, warns)
}

// ── CFG validators (process-LOCAL block space) ──
fn assert_cfg_valid(p: &ir::Process) {
    let n = p.body.len() as u32;
    assert!(p.entry < n, "entry {} out of bounds ({})", p.entry, n);
    let chk = |t: u32| assert!(t < n, "terminator target {t} out of bounds ({n})");
    for bb in &p.body {
        match &bb.term {
            ir::Terminator::Goto { target } => chk(*target),
            ir::Terminator::Branch {
                then_bb, else_bb, ..
            } => {
                chk(*then_bb);
                chk(*else_bb);
            }
            ir::Terminator::Delay { resume, .. } | ir::Terminator::Wait { resume, .. } => {
                chk(*resume)
            }
            ir::Terminator::Fork {
                children,
                join,
                resume_bb,
            } => {
                for c in children {
                    chk(*c);
                }
                chk(*join);
                chk(*resume_bb);
            }
            ir::Terminator::Call { target, ret_bb } => {
                chk(*target);
                chk(*ret_bb);
            }
            ir::Terminator::Return => {}
        }
    }
}

/// Every block reachable from entry must reach a Return (no infinite-non-loop
/// dangling). Loops (back-edges) are allowed; we only require Return-reachability
/// for ACYCLIC paths, so a `forever` is exempted by the caller.
fn assert_all_paths_return(p: &ir::Process) {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    let mut reaches_return = false;
    fn walk(p: &ir::Process, b: u32, seen: &mut std::collections::HashSet<u32>, hit: &mut bool) {
        if !seen.insert(b) {
            return;
        }
        match &p.body[b as usize].term {
            ir::Terminator::Return => *hit = true,
            ir::Terminator::Goto { target } => walk(p, *target, seen, hit),
            ir::Terminator::Branch {
                then_bb, else_bb, ..
            } => {
                walk(p, *then_bb, seen, hit);
                walk(p, *else_bb, seen, hit);
            }
            ir::Terminator::Delay { resume, .. } | ir::Terminator::Wait { resume, .. } => {
                walk(p, *resume, seen, hit)
            }
            ir::Terminator::Fork { resume_bb, .. } => walk(p, *resume_bb, seen, hit),
            ir::Terminator::Call { ret_bb, .. } => walk(p, *ret_bb, seen, hit),
        }
    }
    walk(p, p.entry, &mut seen, &mut reaches_return);
    assert!(reaches_return, "no path from entry reaches Return");
    let _ = &mut seen;
}

fn proc_item(
    kind: ast::ProcKind,
    sens: Option<ast::Sensitivity>,
    body: ast::Stmt,
) -> ast::ModuleItem {
    ast::ModuleItem::Proc(ast::ProceduralBlock {
        kind,
        sensitivity: sens,
        body: Box::new(body),
        span: SP,
    })
}
fn blk(stmts: Vec<ast::Stmt>) -> ast::Stmt {
    ast::Stmt::Block {
        label: None,
        decls: Vec::new(),
        stmts,
        span: SP,
    }
}
fn nb(lhs: &str, rhs: ast::Expr) -> ast::Stmt {
    ast::Stmt::NonBlocking {
        lhs: lv_id(lhs),
        delay: None,
        event: None,
        rhs,
        span: SP,
    }
}
fn bassign(lhs: &str, rhs: ast::Expr) -> ast::Stmt {
    ast::Stmt::Blocking {
        lhs: lv_id(lhs),
        delay: None,
        event: None,
        rhs,
        span: SP,
    }
}
fn delay_stmt(n: u32, body: Option<ast::Stmt>) -> ast::Stmt {
    ast::Stmt::DelayCtrl {
        delay: ast::Delay {
            values: vec![dec(&n.to_string())],
            span: SP,
        },
        body: body.map(Box::new),
        span: SP,
    }
}
fn systask(name: &str, args: Vec<ast::Expr>) -> ast::Stmt {
    ast::Stmt::SysTaskCall {
        name: ident(name),
        args,
        span: SP,
    }
}
fn str_e(s: &str) -> ast::Expr {
    ex(ast::ExprKind::StrLit {
        raw: format!("\"{s}\""),
    })
}
fn ev_list(terms: Vec<(ast::Edge, &str)>) -> ast::Sensitivity {
    ast::Sensitivity::List(
        terms
            .into_iter()
            .map(|(edge, n)| ast::EventExpr {
                edge,
                expr: id_expr(n),
                span: SP,
            })
            .collect(),
    )
}

// v2-1: initial testbench — $dumpfile/$dumpvars + a=0 + #5 + a=1 + #5 + $display + $finish.
#[test]
fn v2_1_initial_testbench_structure() {
    let body = blk(vec![
        systask("$dumpfile", vec![str_e("dump.vcd")]),
        systask("$dumpvars", vec![dec("0"), id_expr("a")]),
        bassign("a", dec("0")),
        delay_stmt(5, None),
        bassign("a", dec("1")),
        delay_stmt(5, None),
        systask("$display", vec![str_e("a=%d"), id_expr("a")]),
        systask("$finish", vec![]),
    ]);
    let unit = module(
        "tb",
        vec![
            netvar(ast::NetVarKind::Reg, Some((0, 0)), false, &["a"]),
            proc_item(ast::ProcKind::Initial, None, body),
        ],
    );
    let (ir, warns) = elab_with_warnings(&unit);
    assert_eq!(warns, 0, "clean testbench must not warn");
    assert_eq!(ir.processes.len(), 1);
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Initial);
    assert!(p.sensitivity.edges.is_empty());
    assert_cfg_valid(p);
    assert_all_paths_return(p);
    // two #5 delays → two Delay terminators with Active region. Since
    // format_version 4 `amount` is an ExprId — resolve each to its folded
    // Const value (raw module units; the engine scales at suspension time).
    let delays: Vec<_> = p
        .body
        .iter()
        .filter_map(|bb| match bb.term {
            ir::Terminator::Delay { amount, region, .. } => {
                let v = match &ir.exprs[amount as usize] {
                    ir::Expr::Const { val } => ir.consts[*val as usize]
                        .bits
                        .val
                        .first()
                        .copied()
                        .unwrap_or(u64::MAX),
                    other => panic!("const #5 must lower to a Const expr, got {other:?}"),
                };
                Some((v, region))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        delays,
        vec![(5, ir::DelayRegion::Active), (5, ir::DelayRegion::Active)]
    );
}

// v2-2: always_ff @(posedge clk) q <= d → SensKind::Edge / Posedge.
#[test]
fn v2_2_always_ff_edge() {
    let body = nb("q", id_expr("d"));
    let unit = module(
        "ff",
        vec![
            netvar(
                ast::NetVarKind::Reg,
                Some((0, 0)),
                false,
                &["q", "clk", "d"],
            ),
            proc_item(
                ast::ProcKind::AlwaysFf,
                Some(ev_list(vec![(ast::Edge::Posedge, "clk")])),
                body,
            ),
        ],
    );
    let (ir, _) = elab_with_warnings(&unit);
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Edge);
    assert_eq!(p.sensitivity.edges.len(), 1);
    assert_eq!(p.sensitivity.edges[0].kind, ir::EdgeKind::Posedge);
    assert_cfg_valid(p);
    assert_all_paths_return(p);
}

// v2-3: bare always @(a or b) → SensKind::Level, both AnyEdge terms.
#[test]
fn v2_3_level_sensitivity() {
    let body = bassign("y", id_expr("a"));
    let unit = module(
        "lvl",
        vec![
            netvar(ast::NetVarKind::Reg, Some((0, 0)), false, &["y", "a", "b"]),
            proc_item(
                ast::ProcKind::Always,
                Some(ev_list(vec![
                    (ast::Edge::NoEdge, "a"),
                    (ast::Edge::NoEdge, "b"),
                ])),
                body,
            ),
        ],
    );
    let (ir, _) = elab_with_warnings(&unit);
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Level);
    assert_eq!(p.sensitivity.edges.len(), 2);
    assert_cfg_valid(p);
    assert_all_paths_return(p);
}

// v2-4 (M-C): bare `always #5 clk = ~clk;` clock generator → NON-FATAL, Comb,
// forever-wrapped (no Return-reachable continuation; back-edge cycle).
#[test]
fn v2_4_clock_generator_self_timed() {
    let invert = ex(ast::ExprKind::Unary {
        op: ast::UnOp::BitNot,
        operand: Box::new(id_expr("clk")),
    });
    let body = delay_stmt(5, Some(bassign("clk", invert)));
    let unit = module(
        "clkgen",
        vec![
            netvar(ast::NetVarKind::Reg, Some((0, 0)), false, &["clk"]),
            proc_item(ast::ProcKind::Always, None, body), // <-- no header @, in-body #5
        ],
    );
    let (ir, warns) = elab_with_warnings(&unit);
    assert_eq!(
        warns, 0,
        "a self-timed clock generator is legal, must not warn"
    );
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Comb);
    assert_cfg_valid(p); // forever is exempt from assert_all_paths_return
                         // there is a Delay terminator and a back-edge Goto (the forever cycle).
    assert!(p
        .body
        .iter()
        .any(|bb| matches!(bb.term, ir::Terminator::Delay { .. })));
}

// v2-5 (M-C): truly inert `always` (no @ no timing) → WARN, still Some + valid.
#[test]
fn v2_5_bare_always_no_timing_warns_not_fatal() {
    let unit = module(
        "m",
        vec![
            netvar(ast::NetVarKind::Reg, Some((0, 0)), false, &["a"]),
            proc_item(ast::ProcKind::Always, None, bassign("a", dec("0"))),
        ],
    );
    let sink = CollectSink::default();
    let out = elaborate(&unit, &sink);
    assert!(out.is_some(), "bare always is now non-fatal");
    assert_eq!(sink.n_warnings(), 1);
    assert_cfg_valid(&out.unwrap().processes[0]);
}

// v2-6: if/else → Branch + shared merge; every path Returns.
#[test]
fn v2_6_if_else_merge() {
    let body = ast::Stmt::If {
        cond: id_expr("c"),
        then_s: Box::new(bassign("y", dec("1"))),
        else_s: Some(Box::new(bassign("y", dec("0")))),
        span: SP,
    };
    let unit = module(
        "m",
        vec![
            netvar(ast::NetVarKind::Reg, Some((0, 0)), false, &["y", "c"]),
            proc_item(ast::ProcKind::Initial, None, body),
        ],
    );
    let (ir, _) = elab_with_warnings(&unit);
    let p = &ir.processes[0];
    assert!(p
        .body
        .iter()
        .any(|bb| matches!(bb.term, ir::Terminator::Branch { .. })));
    assert_cfg_valid(p);
    assert_all_paths_return(p);
}

// v2-7 (M-B): casez lowers (NON-FATAL, warning) into a CaseEq Branch chain.
#[test]
fn v2_7_casez_wildcard_free_lowers_cleanly() {
    let items = vec![
        ast::CaseItem::Match {
            labels: vec![lit("2'b10", ast::IntLitKind::Sized)],
            body: Box::new(bassign("y", dec("1"))),
            span: SP,
        },
        ast::CaseItem::Default {
            body: Box::new(bassign("y", dec("0"))),
            span: SP,
        },
    ];
    let body = ast::Stmt::Case {
        kind: ast::CaseKind::Casez,
        scrutinee: id_expr("s"),
        items,
        span: SP,
    };
    let unit = module(
        "m",
        vec![
            netvar(ast::NetVarKind::Reg, Some((1, 0)), false, &["s", "y"]),
            proc_item(ast::ProcKind::Initial, None, body),
        ],
    );
    let (ir, warns) = elab_with_warnings(&unit);
    // v7: casez/casex lower to ONE dedicated match op per label (z-only /
    // x-or-z don't-care on EITHER side, computed at runtime), with NO warning.
    assert_eq!(warns, 0, "casez label lowers cleanly, no warning");
    let p = &ir.processes[0];
    let has_casez_eq = ir.exprs.iter().any(|e| {
        matches!(
            e,
            ir::Expr::Binary {
                op: ir::BinOp::CasezEq,
                ..
            }
        )
    });
    assert!(has_casez_eq, "casez must lower via BinOp::CasezEq (v7)");
    assert!(p
        .body
        .iter()
        .any(|bb| matches!(bb.term, ir::Terminator::Branch { .. })));
    assert_cfg_valid(p);
    assert_all_paths_return(p);
}

// v2-8: in-body @(posedge clk) → Wait{Edge,Posedge}, NOT process sensitivity.
#[test]
fn v2_8_in_body_event_wait() {
    let body = blk(vec![
        ast::Stmt::EventCtrl {
            ctrl: ev_list(vec![(ast::Edge::Posedge, "clk")]),
            body: None,
            span: SP,
        },
        nb("q", id_expr("d")),
    ]);
    let unit = module(
        "m",
        vec![
            netvar(
                ast::NetVarKind::Reg,
                Some((0, 0)),
                false,
                &["q", "d", "clk"],
            ),
            proc_item(ast::ProcKind::Initial, None, body),
        ],
    );
    let (ir, _) = elab_with_warnings(&unit);
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Initial); // block-level stays Initial
    let waits: Vec<_> = p
        .body
        .iter()
        .filter_map(|bb| match &bb.term {
            ir::Terminator::Wait {
                cond: ir::WaitCause::Edge { kind, .. },
                ..
            } => Some(*kind),
            _ => None,
        })
        .collect();
    assert_eq!(waits, vec![ir::EdgeKind::Posedge]);
    assert_cfg_valid(p);
    assert_all_paths_return(p);
}

// v2-9 (M-D): unknown $task ($printtimescale — still unmapped) → WARN + skip,
// IR survives, no Stmt. ($timeformat, the old example here, is now SUPPORTED —
// §21.3.2 — so it emits a real no-op Display stmt instead of warn-skipping.)
#[test]
fn v2_9_unknown_systask_nonfatal() {
    let body = blk(vec![
        systask("$printtimescale", vec![]),
        bassign("a", dec("0")),
        systask("$finish", vec![]),
    ]);
    let unit = module(
        "tb",
        vec![
            netvar(ast::NetVarKind::Reg, Some((0, 0)), false, &["a"]),
            proc_item(ast::ProcKind::Initial, None, body),
        ],
    );
    let (ir, warns) = elab_with_warnings(&unit);
    assert_eq!(warns, 1, "$printtimescale must warn-skip, not kill the IR");
    // exactly one SysTask stmt survives ($finish); $printtimescale emitted nothing.
    let n_systask = ir
        .stmts
        .iter()
        .filter(|s| matches!(s, ir::Stmt::SysTask { .. }))
        .count();
    assert_eq!(n_systask, 1);
    assert_cfg_valid(&ir.processes[0]);
    assert_all_paths_return(&ir.processes[0]);
}

// v2-10: full multi-process testbench (initial stimulus + always_ff DUT) +
//        whole-SimIr determinism (same AST → byte-identical SimIr).
#[test]
fn v2_10_multiprocess_and_determinism() {
    let mk = || {
        let dut = nb("q", id_expr("d"));
        let stim = blk(vec![
            bassign("d", dec("1")),
            delay_stmt(10, None),
            systask("$finish", vec![]),
        ]);
        module(
            "tb",
            vec![
                netvar(
                    ast::NetVarKind::Reg,
                    Some((0, 0)),
                    false,
                    &["q", "d", "clk"],
                ),
                proc_item(
                    ast::ProcKind::AlwaysFf,
                    Some(ev_list(vec![(ast::Edge::Posedge, "clk")])),
                    dut,
                ),
                proc_item(ast::ProcKind::Initial, None, stim),
            ],
        )
    };
    let (ir1, _) = elab_with_warnings(&mk());
    let (ir2, _) = elab_with_warnings(&mk());
    assert_eq!(ir1, ir2, "same AST must produce byte-identical SimIr");
    assert_eq!(ir1.processes.len(), 2);
    assert_eq!(ir1.processes[0].sensitivity.kind, ir::SensKind::Edge); // DUT
    assert_eq!(ir1.processes[1].sensitivity.kind, ir::SensKind::Initial); // stimulus
    for p in &ir1.processes {
        assert_cfg_valid(p);
    }
    assert_all_paths_return(&ir1.processes[1]); // initial terminates
}

// ════════════════════════════════════════════════════════════════════
//  v3 — module instantiation + hierarchy flattening tests
// ════════════════════════════════════════════════════════════════════

impl CollectSink {
    /// Count ERROR-severity diagnostics.
    fn n_errors(&self) -> usize {
        self.events
            .borrow()
            .iter()
            .filter(|e| matches!(e, LogEvent::Diagnostic(d) if d.severity == diag::Severity::Error))
            .count()
    }
}

// ── v3 builders ──
fn ansi_port(dir: ast::PortDir, range: Option<(&str, &str)>, name: &str) -> ast::AnsiPort {
    ast::AnsiPort {
        dir,
        net_or_var: None,
        signed: false,
        range: range.map(|(m, l)| ast::Range {
            // bounds are exprs (may reference a param like `W-1`)
            msb: parse_range_expr(m),
            lsb: parse_range_expr(l),
            span: SP,
        }),
        packed: Vec::new(),
        name: ident(name),
        iface: None,
        default: None,
        span: SP,
    }
}
/// Parse a tiny range-bound source string into an Expr: either a decimal literal
/// (`"7"`) or `NAME-1` (`"W-1"`) — enough for the width tests.
fn parse_range_expr(s: &str) -> ast::Expr {
    if let Some(lhs) = s.strip_suffix("-1") {
        binop(ast::BinOp::Sub, id_expr(lhs), dec("1"))
    } else if s.parse::<u32>().is_ok() {
        dec(s)
    } else {
        id_expr(s)
    }
}
fn param(name: &str, value: u32) -> ast::ParamDecl {
    ast::ParamDecl {
        kind: ast::ParamKind::Parameter,
        signed: false,
        ty: ast::ParamType::Implicit,
        range: None,
        name: ident(name),
        value: dec(&value.to_string()),
        span: SP,
    }
}
/// A module with ANSI ports + params + a body.
fn module_p(
    name: &str,
    params: Vec<ast::ParamDecl>,
    ports: Vec<ast::AnsiPort>,
    body: Vec<ast::ModuleItem>,
) -> ast::ModuleDecl {
    ast::ModuleDecl {
        is_macromodule: false,
        name: ident(name),
        params,
        ports: if ports.is_empty() {
            ast::PortList::None
        } else {
            ast::PortList::Ansi(ports)
        },
        body,
        span: SP,
    }
}
/// A SourceUnit from a list of ModuleDecls (declaration order).
fn unit_of(modules: Vec<ast::ModuleDecl>) -> ast::SourceUnit {
    ast::SourceUnit {
        items: modules.into_iter().map(ast::TopItem::Module).collect(),
        span: SP,
    }
}
/// A `child u(.p(expr), …)` named-connection instance item.
fn inst_named(module: &str, inst: &str, conns: Vec<(&str, ast::Expr)>) -> ast::ModuleItem {
    ast::ModuleItem::Instance(ast::ModuleInstance {
        module_name: ident(module),
        param_overrides: Vec::new(),
        instances: vec![ast::InstanceItem {
            name: ident(inst),
            unpacked: Vec::new(),
            conns: ast::PortConnList::Named(
                conns
                    .into_iter()
                    .map(|(p, e)| ast::PortConn {
                        name: ident(p),
                        value: Some(e),
                        span: SP,
                    })
                    .collect(),
                false,
            ),
            span: SP,
        }],
        span: SP,
    })
}
/// Like `inst_named` but with a `#(.P(v))` named param override.
fn inst_named_param(
    module: &str,
    inst: &str,
    overrides: Vec<(&str, u32)>,
    conns: Vec<(&str, ast::Expr)>,
) -> ast::ModuleItem {
    ast::ModuleItem::Instance(ast::ModuleInstance {
        module_name: ident(module),
        param_overrides: overrides
            .into_iter()
            .map(|(p, v)| ast::ParamConn::Named {
                name: ident(p),
                value: Some(dec(&v.to_string())),
                span: SP,
            })
            .collect(),
        instances: vec![ast::InstanceItem {
            name: ident(inst),
            unpacked: Vec::new(),
            conns: ast::PortConnList::Named(
                conns
                    .into_iter()
                    .map(|(p, e)| ast::PortConn {
                        name: ident(p),
                        value: Some(e),
                        span: SP,
                    })
                    .collect(),
                false,
            ),
            span: SP,
        }],
        span: SP,
    })
}
// v3-1: top `tb` instantiating a `dff` submodule → flat SimIr with tb nets +
//       namespaced dff nets + port cont-assigns + 2 Instance records.
#[test]
fn v3_1_top_instantiates_dff_flat_ir() {
    // module dff(input clk, input d, output q); endmodule  (no body logic)
    let dff = module_p(
        "dff",
        vec![],
        vec![
            ansi_port(ast::PortDir::Input, None, "clk"),
            ansi_port(ast::PortDir::Input, None, "d"),
            ansi_port(ast::PortDir::Output, None, "q"),
        ],
        vec![],
    );
    // module tb; reg clk,d,q; dff u_dff(.clk(clk),.d(d),.q(q)); endmodule
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["clk", "d", "q"]),
            inst_named(
                "dff",
                "u_dff",
                vec![
                    ("clk", id_expr("clk")),
                    ("d", id_expr("d")),
                    ("q", id_expr("q")),
                ],
            ),
        ],
    );
    // declaration order: dff then tb → tb is the never-instantiated top.
    let unit = unit_of(vec![dff, tb]);
    let (s, _w) = elab_with_warnings(&unit);

    // 2 instances: tb (parent None) then dff child (parent = 0).
    assert_eq!(s.instances.len(), 2);
    assert!(s.instances[0].parent.is_none());
    assert_eq!(s.instances[0].first_net, 0);
    assert_eq!(s.instances[0].net_count, 3); // tb: clk,d,q
    assert_eq!(s.instances[1].parent, Some(0));
    assert_eq!(s.instances[1].first_net, 3);
    assert_eq!(s.instances[1].net_count, 3); // dff: clk,d,q (namespaced)

    // 6 nets total: tb.clk(0) tb.d(1) tb.q(2) dff.clk(3) dff.d(4) dff.q(5).
    assert_eq!(s.nets.len(), 6);
    // dff ports carry their declared directions.
    assert_eq!(s.nets[3].dir, ir::PortDir::Input); // dff.clk
    assert_eq!(s.nets[4].dir, ir::PortDir::Input); // dff.d
    assert_eq!(s.nets[5].dir, ir::PortDir::Output); // dff.q

    // 3 port cont-assigns. Inputs drive the child port (child = parent_expr);
    // the output drives the parent lvalue (parent = child).
    assert_eq!(s.cont_assigns.len(), 3);
    // walk in child-header order: clk (in), d (in), q (out).
    let ca_clk = &s.cont_assigns[0];
    assert_eq!(ca_clk.lhs.chunks[0].net, 3); // lhs = dff.clk (child input)
    assert!(matches!(
        s.exprs[ca_clk.rhs as usize],
        ir::Expr::Signal { net: 0, .. }
    )); // rhs = tb.clk
    let ca_d = &s.cont_assigns[1];
    assert_eq!(ca_d.lhs.chunks[0].net, 4); // lhs = dff.d
    assert!(matches!(
        s.exprs[ca_d.rhs as usize],
        ir::Expr::Signal { net: 1, .. }
    )); // tb.d
    let ca_q = &s.cont_assigns[2];
    assert_eq!(ca_q.lhs.chunks[0].net, 2); // lhs = tb.q (parent, output port)
    assert!(matches!(
        s.exprs[ca_q.rhs as usize],
        ir::Expr::Signal { net: 5, .. }
    )); // rhs = dff.q
}

// v3-2: top selection — a module never instantiated is the top; the instantiated
//       leaf is NOT a separate top instance at the root.
#[test]
fn v3_2_top_selection_picks_uninstantiated() {
    let leaf = module_p(
        "leaf",
        vec![],
        vec![ansi_port(ast::PortDir::Output, None, "o")],
        vec![],
    );
    let top = module_p(
        "top",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, None, false, &["w"]),
            inst_named("leaf", "u", vec![("o", id_expr("w"))]),
        ],
    );
    let unit = unit_of(vec![leaf, top]);
    let (s, _w) = elab_with_warnings(&unit);
    // root instance is `top` (parent None), and there are exactly 2 instances.
    assert_eq!(s.instances.len(), 2);
    assert!(s.instances[0].parent.is_none());
    assert_eq!(s.instances[0].net_count, 1); // top: w
    assert_eq!(s.instances[1].net_count, 1); // leaf: o
}

// v3-2b: MULTIPLE uninstantiated top modules — IEEE 1364 / iverilog elaborate
//        EVERY uninstantiated module as an independent root (not just the
//        last-declared). Two modules, neither instantiating the other, must BOTH
//        become root instances (parent None) and BOTH bodies be lowered.
#[test]
fn v3_2b_multiple_uninstantiated_tops_all_become_roots() {
    let a = module_p(
        "mod_a",
        vec![],
        vec![],
        vec![proc_item(
            ast::ProcKind::Initial,
            None,
            blk(vec![systask("$display", vec![str_e("a")])]),
        )],
    );
    let b = module_p(
        "mod_b",
        vec![],
        vec![],
        vec![proc_item(
            ast::ProcKind::Initial,
            None,
            blk(vec![systask("$display", vec![str_e("b")])]),
        )],
    );
    let unit = unit_of(vec![a, b]);
    let (s, _w) = elab_with_warnings(&unit);
    // BOTH modules are roots: 2 instances, both parent None.
    assert_eq!(
        s.instances.len(),
        2,
        "both uninstantiated modules must be roots"
    );
    assert!(s.instances[0].parent.is_none());
    assert!(s.instances[1].parent.is_none());
    // BOTH bodies lowered → 2 processes (one initial each).
    assert_eq!(s.processes.len(), 2, "both top bodies must be lowered");
}

// v3-2c: a module instantiated ONLY inside a generate block is NOT a spurious
//        extra root. `holder` instantiates `leaf` inside a `generate` block, so
//        `leaf` is reachable and must appear EXACTLY once (under holder), never as
//        its own root. Guards the multi-root scan against missing generate-nested
//        instantiations (which would double-elaborate `leaf`).
#[test]
fn v3_2c_generate_nested_instance_is_not_a_root() {
    let leaf = module_p(
        "leaf",
        vec![],
        vec![ansi_port(ast::PortDir::Output, None, "o")],
        vec![],
    );
    let holder = module_p(
        "holder",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, None, false, &["w"]),
            ast::ModuleItem::Generate(ast::GenerateConstruct {
                items: vec![ast::GenItem::Block {
                    label: Some(ident("g")),
                    items: vec![ast::GenItem::Item(Box::new(inst_named(
                        "leaf",
                        "u",
                        vec![("o", id_expr("w"))],
                    )))],
                    span: SP,
                }],
                span: SP,
            }),
        ],
    );
    let unit = unit_of(vec![leaf, holder]);
    let (s, _w) = elab_with_warnings(&unit);
    // exactly 2 instances: holder (root) + leaf (under holder via generate).
    assert_eq!(
        s.instances.len(),
        2,
        "leaf instantiated in a generate must not also be a root"
    );
    assert!(s.instances[0].parent.is_none()); // holder
    assert_eq!(s.instances[1].parent, Some(0)); // leaf under holder
}

// v3-3: 2-deep hierarchy tb → mid → leaf. 3 instances, parent chain 0←1←2,
//       contiguous net slices.
#[test]
fn v3_3_two_deep_hierarchy() {
    let leaf = module_p(
        "leaf",
        vec![],
        vec![ansi_port(ast::PortDir::Input, None, "a")],
        vec![],
    );
    let mid = module_p(
        "mid",
        vec![],
        vec![ansi_port(ast::PortDir::Input, None, "x")],
        vec![inst_named("leaf", "u_leaf", vec![("a", id_expr("x"))])],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["sig"]),
            inst_named("mid", "u_mid", vec![("x", id_expr("sig"))]),
        ],
    );
    let unit = unit_of(vec![leaf, mid, tb]);
    let (s, _w) = elab_with_warnings(&unit);

    assert_eq!(s.instances.len(), 3);
    // depth-first preorder: tb(0), mid(1), leaf(2).
    assert!(s.instances[0].parent.is_none()); // tb
    assert_eq!(s.instances[1].parent, Some(0)); // mid under tb
    assert_eq!(s.instances[2].parent, Some(1)); // leaf under mid
                                                // contiguous slices: tb[0..1] mid[1..2] leaf[2..3].
    assert_eq!(s.instances[0].first_net, 0);
    assert_eq!(s.instances[1].first_net, 1);
    assert_eq!(s.instances[2].first_net, 2);
    assert_eq!(s.nets.len(), 3); // sig, mid.x, leaf.a
                                 // two input-port cont-assigns (mid.x = tb.sig ; leaf.a = mid.x).
    assert_eq!(s.cont_assigns.len(), 2);
    assert_eq!(s.cont_assigns[0].lhs.chunks[0].net, 1); // mid.x
    assert_eq!(s.cont_assigns[1].lhs.chunks[0].net, 2); // leaf.a
}

// v3-4: param override changes a submodule net width. `reg8 #(.W(8))` →
//       child output `[W-1:0] q` is 8 bits.
#[test]
fn v3_4_param_override_changes_width() {
    // module reg8 #(parameter W=1)(output [W-1:0] q); endmodule
    let reg8 = module_p(
        "reg8",
        vec![param("W", 1)],
        vec![ansi_port(ast::PortDir::Output, Some(("W-1", "0")), "q")],
        vec![],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, Some((7, 0)), false, &["bus"]),
            inst_named_param("reg8", "u", vec![("W", 8)], vec![("q", id_expr("bus"))]),
        ],
    );
    let unit = unit_of(vec![reg8, tb]);
    let (s, _w) = elab_with_warnings(&unit);

    // child q net (index 1: after tb.bus at 0) is width 8 thanks to W=8.
    assert_eq!(s.nets.len(), 2);
    let q = &s.nets[1];
    assert_eq!(q.width, 8, "W override must fold [W-1:0] to width 8");
    assert_eq!(q.msb, 7);
    assert_eq!(q.lsb, 0);
    assert_eq!(q.dir, ir::PortDir::Output);
}

// v3-5: default param (no override) → child width uses the declared default.
#[test]
fn v3_5_param_default_width() {
    let reg_w = module_p(
        "regw",
        vec![param("W", 4)],
        vec![ansi_port(ast::PortDir::Output, Some(("W-1", "0")), "q")],
        vec![],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, Some((3, 0)), false, &["bus"]),
            inst_named("regw", "u", vec![("q", id_expr("bus"))]),
        ],
    );
    let unit = unit_of(vec![reg_w, tb]);
    let (s, _w) = elab_with_warnings(&unit);
    // child q uses default W=4 → width 4.
    assert_eq!(s.nets[1].width, 4);
}

// v3-6: unconnected input port floats (no cont-assign); unconnected output warns.
#[test]
fn v3_6_unconnected_ports() {
    let leaf = module_p(
        "leaf",
        vec![],
        vec![
            ansi_port(ast::PortDir::Input, None, "a"),
            ansi_port(ast::PortDir::Output, None, "y"),
        ],
        vec![],
    );
    // connect only `a`; leave `y` unconnected.
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["sig"]),
            inst_named("leaf", "u", vec![("a", id_expr("sig"))]),
        ],
    );
    let unit = unit_of(vec![leaf, tb]);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("non-fatal");
    // exactly ONE cont-assign (input a); output y emitted none (floats).
    // net layout: tb.sig=0, leaf.a=1, leaf.y=2.
    assert_eq!(s.cont_assigns.len(), 1);
    assert_eq!(s.cont_assigns[0].lhs.chunks[0].net, 1); // leaf.a (driven input)
                                                        // and exactly one WARNING for the unconnected output port.
    assert_eq!(sink.n_warnings(), 1);
    assert_eq!(sink.n_errors(), 0);
}

// v3-7: unknown module instantiation → ElabUnresolvedInstance error.
#[test]
fn v3_7_unknown_module_errors() {
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, None, false, &["w"]),
            inst_named("nonexistent", "u", vec![("p", id_expr("w"))]),
        ],
    );
    let unit = unit_of(vec![tb]);
    let sink = CollectSink::default();
    let out = elaborate(&unit, &sink);
    assert!(out.is_none(), "unknown module must be a hard error");
    let codes: Vec<_> = sink
        .events
        .borrow()
        .iter()
        .filter_map(|e| match e {
            LogEvent::Diagnostic(d) => Some(d.code),
            _ => None,
        })
        .collect();
    assert!(codes.contains(&MsgCode::ElabUnresolvedInstance));
}

// v3-8: two instances of the SAME leaf under one parent (diamond reuse) is fine,
//       each namespaced separately — NOT flagged as a cycle.
#[test]
fn v3_8_repeated_leaf_instances_not_a_cycle() {
    let leaf = module_p(
        "leaf",
        vec![],
        vec![ansi_port(ast::PortDir::Input, None, "a")],
        vec![],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["s0", "s1"]),
            inst_named("leaf", "u0", vec![("a", id_expr("s0"))]),
            inst_named("leaf", "u1", vec![("a", id_expr("s1"))]),
        ],
    );
    let unit = unit_of(vec![leaf, tb]);
    let (s, _w) = elab_with_warnings(&unit);
    // tb + 2 leaf instances = 3.
    assert_eq!(s.instances.len(), 3);
    assert_eq!(s.instances[1].parent, Some(0));
    assert_eq!(s.instances[2].parent, Some(0));
    // 2 input cont-assigns: tb.u0.a = tb.s0 ; tb.u1.a = tb.s1.
    assert_eq!(s.cont_assigns.len(), 2);
    assert_eq!(s.cont_assigns[0].lhs.chunks[0].net, 2); // u0.a (s0=0,s1=1,u0.a=2)
    assert_eq!(s.cont_assigns[1].lhs.chunks[0].net, 3); // u1.a
}

// v3-9: determinism — same multi-module AST → byte-identical SimIr.
#[test]
fn v3_9_determinism() {
    let mk = || {
        let dff = module_p(
            "dff",
            vec![],
            vec![
                ansi_port(ast::PortDir::Input, None, "clk"),
                ansi_port(ast::PortDir::Output, None, "q"),
            ],
            vec![],
        );
        let tb = module_p(
            "tb",
            vec![],
            vec![],
            vec![
                netvar(ast::NetVarKind::Reg, None, false, &["clk", "q"]),
                inst_named(
                    "dff",
                    "u",
                    vec![("clk", id_expr("clk")), ("q", id_expr("q"))],
                ),
            ],
        );
        unit_of(vec![dff, tb])
    };
    let (s1, _) = elab_with_warnings(&mk());
    let (s2, _) = elab_with_warnings(&mk());
    assert_eq!(
        s1, s2,
        "same multi-module AST must produce byte-identical SimIr"
    );
}

// v3-10: the v1 single-module path still produces one self-instance covering all
//        nets (top special case: prefix = module name, no children).
#[test]
fn v3_10_single_module_self_instance_preserved() {
    let m = module_p("m", vec![], vec![], vec![wire_vec(7, 0, &["a", "b", "y"])]);
    let unit = unit_of(vec![m]);
    let (s, _w) = elab_with_warnings(&unit);
    assert_eq!(s.instances.len(), 1);
    assert!(s.instances[0].parent.is_none());
    assert_eq!(s.instances[0].first_net, 0);
    assert_eq!(s.instances[0].net_count, 3);
    assert_eq!(s.nets.len(), 3);
}

// ── v3 fix-set builders + diag extractors (PART D) ──
/// Like `inst_named` but with a `#(.P(expr))` named param override whose value is
/// an arbitrary Expr — lets the override reference a PARENT param (e.g. `id_expr("P")`).
fn inst_named_param_expr(
    module: &str,
    inst: &str,
    overrides: Vec<(&str, ast::Expr)>,
    conns: Vec<(&str, ast::Expr)>,
) -> ast::ModuleItem {
    ast::ModuleItem::Instance(ast::ModuleInstance {
        module_name: ident(module),
        param_overrides: overrides
            .into_iter()
            .map(|(p, e)| ast::ParamConn::Named {
                name: ident(p),
                value: Some(e),
                span: SP,
            })
            .collect(),
        instances: vec![ast::InstanceItem {
            name: ident(inst),
            unpacked: Vec::new(),
            conns: ast::PortConnList::Named(
                conns
                    .into_iter()
                    .map(|(p, e)| ast::PortConn {
                        name: ident(p),
                        value: Some(e),
                        span: SP,
                    })
                    .collect(),
                false,
            ),
            span: SP,
        }],
        span: SP,
    })
}
/// A `child u(expr, expr, …)` POSITIONAL-connection instance item (skip slots = None).
fn inst_positional(module: &str, inst: &str, conns: Vec<Option<ast::Expr>>) -> ast::ModuleItem {
    ast::ModuleItem::Instance(ast::ModuleInstance {
        module_name: ident(module),
        param_overrides: Vec::new(),
        instances: vec![ast::InstanceItem {
            name: ident(inst),
            unpacked: Vec::new(),
            conns: ast::PortConnList::Positional(conns),
            span: SP,
        }],
        span: SP,
    })
}
/// All diagnostic MsgCodes emitted, in order.
fn diag_codes(sink: &CollectSink) -> Vec<MsgCode> {
    sink.events
        .borrow()
        .iter()
        .filter_map(|e| match e {
            LogEvent::Diagnostic(d) => Some(d.code),
            _ => None,
        })
        .collect()
}
/// All WARNING-severity diagnostic messages, in order.
fn warn_messages(sink: &CollectSink) -> Vec<String> {
    sink.events
        .borrow()
        .iter()
        .filter_map(|e| match e {
            LogEvent::Diagnostic(d) if d.severity == diag::Severity::Warning => {
                Some(d.message.clone())
            }
            _ => None,
        })
        .collect()
}

// v3-11 [Fix 1]: a param override that references the PARENT's param resolves in
//        the parent scope (not folding to 0). top has P=8; child #(.W(P)) → q is 8 bits.
#[test]
fn v3_11_override_uses_parent_param() {
    // module regw #(parameter W=1)(output [W-1:0] q); endmodule
    let regw = module_p(
        "regw",
        vec![param("W", 1)],
        vec![ansi_port(ast::PortDir::Output, Some(("W-1", "0")), "q")],
        vec![],
    );
    // module top #(parameter P=8); wire [P-1:0] bus; regw #(.W(P)) u(.q(bus)); endmodule
    let top = module_p(
        "top",
        vec![param("P", 8)],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, Some((7, 0)), false, &["bus"]),
            // override value is the PARENT param `P` — must resolve to 8 in parent scope.
            inst_named_param_expr(
                "regw",
                "u",
                vec![("W", id_expr("P"))],
                vec![("q", id_expr("bus"))],
            ),
        ],
    );
    let unit = unit_of(vec![regw, top]);
    let (s, _w) = elab_with_warnings(&unit);
    // child q (net 1, after top.bus) must be width 8 — proves P resolved in parent.
    assert_eq!(
        s.nets[1].width, 8,
        "override .W(P) must see parent P=8, not fold to 0"
    );
}

// v3-12 [Fix 1 defensive]: an override of W to 0 → child [W-1:0] underflows →
//        clamped to width 1 + warn, NOT a fatal MAX_NET_WIDTH error.
#[test]
fn v3_12_zero_width_param_does_not_explode() {
    let regw = module_p(
        "regw",
        vec![param("W", 1)],
        vec![ansi_port(ast::PortDir::Output, Some(("W-1", "0")), "q")],
        vec![],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, None, false, &["bus"]),
            inst_named_param("regw", "u", vec![("W", 0)], vec![("q", id_expr("bus"))]),
        ],
    );
    let unit = unit_of(vec![regw, tb]);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("W=0 must NOT discard the whole IR");
    // child q clamped to width 1, no fatal error.
    assert_eq!(s.nets[1].width, 1);
    assert_eq!(sink.n_errors(), 0);
    assert!(
        sink.n_warnings() >= 1,
        "expected the underflow-clamp warning"
    );
}

// v3-13 [Fix 2]: surplus positional connection → ElabPortMismatch error.
#[test]
fn v3_13_surplus_positional_connection_errors() {
    // dff has 2 ports; connect 3 positionally.
    let dff = module_p(
        "dff",
        vec![],
        vec![
            ansi_port(ast::PortDir::Input, None, "clk"),
            ansi_port(ast::PortDir::Output, None, "q"),
        ],
        vec![],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["c", "q", "x"]),
            inst_positional(
                "dff",
                "u",
                vec![Some(id_expr("c")), Some(id_expr("q")), Some(id_expr("x"))],
            ),
        ],
    );
    let unit = unit_of(vec![dff, tb]);
    let sink = CollectSink::default();
    let out = elaborate(&unit, &sink);
    assert!(out.is_none(), "surplus connection must be a hard error");
    assert!(diag_codes(&sink).contains(&MsgCode::ElabPortMismatch));
}

// v3-14 [Fix 2]: named connection to a nonexistent port → ElabPortMismatch.
#[test]
fn v3_14_named_ghost_port_errors() {
    let dff = module_p(
        "dff",
        vec![],
        vec![ansi_port(ast::PortDir::Input, None, "clk")],
        vec![],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["c"]),
            inst_named(
                "dff",
                "u",
                vec![("clk", id_expr("c")), ("ghost", id_expr("c"))],
            ),
        ],
    );
    let unit = unit_of(vec![dff, tb]);
    let sink = CollectSink::default();
    assert!(elaborate(&unit, &sink).is_none());
    assert!(diag_codes(&sink).contains(&MsgCode::ElabPortMismatch));
}

// v3-15 [Fix 3]: an unconnected INOUT warns with "inout", not "output".
#[test]
fn v3_15_unconnected_inout_warning_text() {
    let leaf = module_p(
        "leaf",
        vec![],
        vec![ansi_port(ast::PortDir::Inout, None, "io")],
        vec![],
    );
    let tb = module_p("tb", vec![], vec![], vec![inst_named("leaf", "u", vec![])]);
    let unit = unit_of(vec![leaf, tb]);
    let sink = CollectSink::default();
    elaborate(&unit, &sink).expect("non-fatal");
    let msg = warn_messages(&sink).join("\n");
    assert!(msg.contains("inout port `io`"), "got: {msg}");
    assert!(!msg.contains("output port `io`"));
}

// v3-16 [Fix 1 + Fix 2 happy path]: regression — the clean multi-fix path still
//        produces the exact v3_1 layout (no false positives from the new checks).
#[test]
fn v3_16_happy_path_unaffected_by_new_checks() {
    let dff = module_p(
        "dff",
        vec![],
        vec![
            ansi_port(ast::PortDir::Input, None, "clk"),
            ansi_port(ast::PortDir::Input, None, "d"),
            ansi_port(ast::PortDir::Output, None, "q"),
        ],
        vec![],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["clk", "d", "q"]),
            inst_named(
                "dff",
                "u",
                vec![
                    ("clk", id_expr("clk")),
                    ("d", id_expr("d")),
                    ("q", id_expr("q")),
                ],
            ),
        ],
    );
    let unit = unit_of(vec![dff, tb]);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("clean path");
    assert_eq!(s.instances.len(), 2);
    assert_eq!(s.cont_assigns.len(), 3);
    assert_eq!(sink.n_errors(), 0);
}

// ════════════════════════════════════════════════════════════════════
//  PR3 — generate / genvar end-to-end unrolling
// ════════════════════════════════════════════════════════════════════

// ── gen builders ──
fn gen_assign(name: &str, value: ast::Expr) -> ast::GenAssign {
    ast::GenAssign {
        lvalue: ident(name),
        value,
        span: SP,
    }
}
/// `for (gv = init; cond; gv = step) [begin:label] body end`.
fn gen_for(
    gv: &str,
    init: ast::Expr,
    cond: ast::Expr,
    step: ast::Expr,
    label: Option<&str>,
    body: Vec<ast::GenItem>,
) -> ast::GenItem {
    ast::GenItem::For {
        init: gen_assign(gv, init),
        cond,
        step: gen_assign(gv, step),
        label: label.map(ident),
        body,
        span: SP,
    }
}
/// `generate <items> endgenerate` as a module item.
fn generate(items: Vec<ast::GenItem>) -> ast::ModuleItem {
    ast::ModuleItem::Generate(ast::GenerateConstruct { items, span: SP })
}
/// Wrap a ModuleItem as a generate item.
fn gitem(mi: ast::ModuleItem) -> ast::GenItem {
    ast::GenItem::Item(Box::new(mi))
}
/// `wire [<msb_expr>:0] names...;` where the msb is an arbitrary expr (so a genvar
/// can appear in the width bound).
fn wire_range_expr(msb: ast::Expr, names: &[&str]) -> ast::ModuleItem {
    ast::ModuleItem::NetVar(ast::NetVarDecl {
        kind: ast::NetVarKind::Wire,
        signed: false,
        range: Some(ast::Range {
            msb,
            lsb: dec("0"),
            span: SP,
        }),
        packed: Vec::new(),
        delay: None,
        names: names
            .iter()
            .map(|n| ast::DeclName {
                name: ident(n),
                unpacked: Vec::new(),
                init: None,
                span: SP,
            })
            .collect(),
        lifetime: None,
        class_type: None,
        class_args: Vec::new(),
        const_param: false,
        span: SP,
    })
}
/// Collect ERROR-severity diag codes.
fn err_codes(sink: &CollectSink) -> Vec<MsgCode> {
    sink.events
        .borrow()
        .iter()
        .filter_map(|e| match e {
            LogEvent::Diagnostic(d) if d.severity == diag::Severity::Error => Some(d.code),
            _ => None,
        })
        .collect()
}

// ge1. generate-for instantiating a `leaf` (input port `a` ← top's `w`) 3× →
//      4 instances (1 top + 3 leaf), top parent None, every leaf parent Some(0).
#[test]
fn ge1_gen_for_instances() {
    let leaf = module_p(
        "leaf",
        vec![],
        vec![ansi_port(ast::PortDir::Input, None, "a")],
        vec![],
    );
    let top = module_p(
        "top",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, None, false, &["w"]),
            generate(vec![gen_for(
                "i",
                dec("0"),
                binop(ast::BinOp::Lt, id_expr("i"), dec("3")),
                binop(ast::BinOp::Add, id_expr("i"), dec("1")),
                Some("g"),
                vec![gitem(inst_named("leaf", "u", vec![("a", id_expr("w"))]))],
            )]),
        ],
    );
    let unit = unit_of(vec![leaf, top]);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("clean generate-for");
    assert_eq!(sink.n_errors(), 0);
    assert_eq!(s.instances.len(), 4); // top + 3 leaf
    assert!(s.instances[0].parent.is_none());
    for inst in &s.instances[1..] {
        assert_eq!(inst.parent, Some(0));
    }
}

// ge2. loop body `wire t; assign t = 1'b0;` ×3 → 3 nets, 3 cont-assigns, the three
//      target nets are DISTINCT (per-iteration g[0].t / g[1].t / g[2].t).
#[test]
fn ge2_gen_for_nets_distinct() {
    let top = module_p(
        "top",
        vec![],
        vec![],
        vec![generate(vec![gen_for(
            "i",
            dec("0"),
            binop(ast::BinOp::Lt, id_expr("i"), dec("3")),
            binop(ast::BinOp::Add, id_expr("i"), dec("1")),
            Some("g"),
            vec![
                gitem(netvar(ast::NetVarKind::Wire, None, false, &["t"])),
                gitem(cont_assign(lv_id("t"), lit("1'b0", ast::IntLitKind::Sized))),
            ],
        )])],
    );
    let unit = unit_of(vec![top]);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("clean generate-for");
    assert_eq!(sink.n_errors(), 0);
    assert_eq!(s.nets.len(), 3);
    assert_eq!(s.cont_assigns.len(), 3);
    let targets: Vec<u32> = s
        .cont_assigns
        .iter()
        .map(|ca| ca.lhs.chunks[0].net)
        .collect();
    let mut uniq = targets.clone();
    uniq.sort_unstable();
    uniq.dedup();
    assert_eq!(
        uniq.len(),
        3,
        "per-iteration nets must not collide: {targets:?}"
    );
}

// ge3/ge4. generate-if branch selection. `if (COND) assign y = a; else assign y =
//      b;` → exactly one cont-assign; COND=1 reads net 0 (`a`), COND=0 reads net 1.
fn build_gen_if(cond: u32) -> ast::SourceUnit {
    let top = module_p(
        "top",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, None, false, &["a", "b", "y"]),
            generate(vec![ast::GenItem::If {
                cond: dec(&cond.to_string()),
                then_b: vec![gitem(cont_assign(lv_id("y"), id_expr("a")))],
                else_b: vec![gitem(cont_assign(lv_id("y"), id_expr("b")))],
                label: None,
                span: SP,
            }]),
        ],
    );
    unit_of(vec![top])
}
#[test]
fn ge3_gen_if_true_branch() {
    let unit = build_gen_if(1);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("clean generate-if");
    assert_eq!(sink.n_errors(), 0);
    assert_eq!(s.cont_assigns.len(), 1);
    let rhs = &s.exprs[s.cont_assigns[0].rhs as usize];
    assert!(
        matches!(rhs, ir::Expr::Signal { net: 0, .. }),
        "then = a (net 0)"
    );
}
#[test]
fn ge4_gen_if_false_branch() {
    let unit = build_gen_if(0);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("clean generate-if");
    assert_eq!(sink.n_errors(), 0);
    assert_eq!(s.cont_assigns.len(), 1);
    let rhs = &s.exprs[s.cont_assigns[0].rhs as usize];
    assert!(
        matches!(rhs, ir::Expr::Signal { net: 1, .. }),
        "else = b (net 1)"
    );
}

// ge5. genvar in a net width bound: `wire [i:0] t;` for i in 0..3 → widths [1,2,3].
#[test]
fn ge5_genvar_in_net_width() {
    let top = module_p(
        "top",
        vec![],
        vec![],
        vec![generate(vec![gen_for(
            "i",
            dec("0"),
            binop(ast::BinOp::Lt, id_expr("i"), dec("3")),
            binop(ast::BinOp::Add, id_expr("i"), dec("1")),
            Some("g"),
            vec![gitem(wire_range_expr(id_expr("i"), &["t"]))],
        )])],
    );
    let unit = unit_of(vec![top]);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("clean generate-for");
    assert_eq!(sink.n_errors(), 0);
    let widths: Vec<u32> = s.nets.iter().map(|n| n.width).collect();
    assert_eq!(widths, vec![1, 2, 3]);
}

// ge6. determinism: elaborate the same unit twice → byte-identical arenas.
#[test]
fn ge6_gen_determinism() {
    let mk = || {
        let top = module_p(
            "top",
            vec![],
            vec![],
            vec![generate(vec![gen_for(
                "i",
                dec("0"),
                binop(ast::BinOp::Lt, id_expr("i"), dec("4")),
                binop(ast::BinOp::Add, id_expr("i"), dec("1")),
                Some("g"),
                vec![
                    gitem(netvar(ast::NetVarKind::Wire, None, false, &["t"])),
                    gitem(cont_assign(lv_id("t"), lit("1'b0", ast::IntLitKind::Sized))),
                ],
            )])],
        );
        unit_of(vec![top])
    };
    let a = elaborate(&mk(), &CollectSink::default()).expect("clean");
    let b = elaborate(&mk(), &CollectSink::default()).expect("clean");
    assert_eq!(a.nets, b.nets);
    assert_eq!(a.instances, b.instances);
    assert_eq!(a.cont_assigns, b.cont_assigns);
}

// ge7. (M1 guard) a stuck genvar step `i = i` → exactly ONE ElabUnsupported, NOT
//      ~4096 duplicate-decl errors.
#[test]
fn ge7_stuck_genvar_one_error() {
    let top = module_p(
        "top",
        vec![],
        vec![],
        vec![generate(vec![gen_for(
            "i",
            dec("0"),
            binop(ast::BinOp::Lt, id_expr("i"), dec("5")),
            id_expr("i"), // step = i (no advance) → stall
            Some("g"),
            vec![gitem(netvar(ast::NetVarKind::Wire, None, false, &["t"]))],
        )])],
    );
    let unit = unit_of(vec![top]);
    let sink = CollectSink::default();
    let _ = elaborate(&unit, &sink); // returns None (had_error); must not flood
    assert_eq!(
        sink.n_errors(),
        1,
        "stuck genvar must emit exactly one error"
    );
    assert!(err_codes(&sink).contains(&MsgCode::ElabUnsupported));
}

// ════════════════════════ user function / task inlining ════════════════════════
// builders
fn tf_port(dir: ast::PortDir, range: Option<(u32, u32)>, name: &str) -> ast::TfPort {
    ast::TfPort {
        dir,
        net_or_var: None,
        signed: false,
        range: range.map(|(m, l)| ast::Range {
            msb: dec(&m.to_string()),
            lsb: dec(&l.to_string()),
            span: SP,
        }),
        name: ident(name),
        default: None,
        span: SP,
    }
}
fn func_def(
    name: &str,
    range: Option<(u32, u32)>,
    ports: Vec<ast::TfPort>,
    body_decls: Vec<ast::NetVarDecl>,
    body: ast::Stmt,
) -> ast::ModuleItem {
    ast::ModuleItem::Func(ast::FunctionDef {
        automatic: false,
        signed: false,
        range: range.map(|(m, l)| ast::Range {
            msb: dec(&m.to_string()),
            lsb: dec(&l.to_string()),
            span: SP,
        }),
        ret_type: ast::ParamType::Implicit,
        ret_two_state: false,
        name: ident(name),
        ports,
        body_decls,
        body: Box::new(body),
        span: SP,
    })
}
fn task_def(
    name: &str,
    ports: Vec<ast::TfPort>,
    body_decls: Vec<ast::NetVarDecl>,
    body: ast::Stmt,
) -> ast::ModuleItem {
    ast::ModuleItem::Task(ast::TaskDef {
        automatic: false,
        name: ident(name),
        ports,
        body_decls,
        body: Box::new(body),
        span: SP,
    })
}
/// `reg [7:0] name;` as a bare NetVarDecl (for function/task body_decls).
fn netvar_decl_reg(name: &str) -> ast::NetVarDecl {
    ast::NetVarDecl {
        kind: ast::NetVarKind::Reg,
        signed: false,
        range: Some(ast::Range {
            msb: dec("7"),
            lsb: dec("0"),
            span: SP,
        }),
        packed: Vec::new(),
        delay: None,
        names: vec![ast::DeclName {
            name: ident(name),
            unpacked: Vec::new(),
            init: None,
            span: SP,
        }],
        lifetime: None,
        class_type: None,
        class_args: Vec::new(),
        const_param: false,
        span: SP,
    }
}
fn call(name: &str, args: Vec<ast::Expr>) -> ast::Expr {
    ex(ast::ExprKind::Call {
        name: hpath(name),
        args,
    })
}
fn task_call(name: &str, args: Vec<ast::Expr>) -> ast::Stmt {
    ast::Stmt::UserTaskCall {
        name: hpath(name),
        args,
        span: SP,
    }
}

/// Peel an inline-function §10.7 return/local-width resize wrapper — a low
/// `Select` (offset 0) and/or a `$signed`/`$unsigned` sign stamp the inline path
/// now adds to size each assigned value to its declared (width, sign) — to reach
/// the underlying folded expression, so the structural inline-folding assertions
/// stay robust to the resize. (These tests' folded bodies contain no legitimate
/// low-Selects, so peeling is unambiguous here.)
fn peel_resize(s: &ir::SimIr, mut eid: u32) -> u32 {
    loop {
        match &s.exprs[eid as usize] {
            ir::Expr::Select {
                base,
                offset,
                kind: ir::SelKind::PartConst,
                ..
            } if matches!(&s.exprs[*offset as usize],
                ir::Expr::Const { val } if s.consts[*val as usize].bits.val[0] == 0) =>
            {
                eid = *base;
            }
            ir::Expr::SysFunc {
                which: ir::SysFuncId::Signed | ir::SysFuncId::Unsigned,
                args,
            } if args.len() == 1 => {
                eid = args[0];
            }
            _ => return eid,
        }
    }
}

// ft-e1. combinational function `function [7:0] add1(input [7:0] x); add1=x+1;
//        endfunction` called as `assign y = add1(a);` → the Call inlines to
//        Binary(Add, Signal a, Const 1) (under a return-width resize) and y's
//        cont-assign points at it.
#[test]
fn ft_e1_function_inlines_to_return_expr() {
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "y"]),
            func_def(
                "add1",
                Some((7, 0)),
                vec![tf_port(ast::PortDir::Input, Some((7, 0)), "x")],
                vec![],
                bassign("add1", binop(ast::BinOp::Add, id_expr("x"), dec("1"))),
            ),
            cont_assign(lv_id("y"), call("add1", vec![id_expr("a")])),
        ],
    );
    let s = elab_ok(&unit);
    // funcs arena stays empty (inline path, no call-frame schema).
    assert!(s.funcs.is_empty());
    assert!(s.blocks.is_empty());
    // one cont-assign onto y (net 1); rhs = Binary(Add, Signal a (net 0), Const 1)
    // under the §10.7 return-width resize (add1 is 8-bit; `x+1` is a 32-bit add
    // via the unsized `1`, truncated to the 8-bit return).
    assert_eq!(s.cont_assigns.len(), 1);
    let ca = &s.cont_assigns[0];
    assert_eq!(ca.lhs.chunks[0].net, 1); // y is 2nd net
    match &s.exprs[peel_resize(&s, ca.rhs) as usize] {
        ir::Expr::Binary {
            op: ir::BinOp::Add,
            lhs,
            rhs,
        } => {
            assert!(matches!(
                s.exprs[*lhs as usize],
                ir::Expr::Signal { net: 0, word: None } // the actual arg `a`
            ));
            match &s.exprs[*rhs as usize] {
                ir::Expr::Const { val } => assert_eq!(s.consts[*val as usize].bits.val[0], 1),
                other => panic!("expected Const 1, got {other:?}"),
            }
        }
        other => panic!("expected Binary(Add, …), got {other:?}"),
    }
}

// ft-e2. straight-line function with a LOCAL var (SSA-by-substitution):
//        function [7:0] f(input [7:0] x); reg [7:0] t; begin t=x+1; f=t+1; end
//        → assign y=f(a)  ==  ((a+1)+1). The local `t` becomes NO net (no extra
//        nets beyond a,y); it is folded into the substitution scope.
#[test]
fn ft_e2_function_local_var_folds() {
    let body = blk(vec![
        bassign("t", binop(ast::BinOp::Add, id_expr("x"), dec("1"))),
        bassign("f", binop(ast::BinOp::Add, id_expr("t"), dec("1"))),
    ]);
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "y"]),
            func_def(
                "f",
                Some((7, 0)),
                vec![tf_port(ast::PortDir::Input, Some((7, 0)), "x")],
                vec![netvar_decl_reg("t")],
                body,
            ),
            cont_assign(lv_id("y"), call("f", vec![id_expr("a")])),
        ],
    );
    let s = elab_ok(&unit);
    // exactly 2 nets (a, y) — the local `t` created NONE.
    assert_eq!(s.nets.len(), 2);
    // rhs root = Add( Add(Signal a, 1), 1 ), each Add under a return/local-width
    // resize (8-bit return + 8-bit local `t`, both fed by 32-bit `x+1` adds).
    let root = &s.exprs[peel_resize(&s, s.cont_assigns[0].rhs) as usize];
    let ir::Expr::Binary {
        op: ir::BinOp::Add,
        lhs,
        rhs,
    } = root
    else {
        panic!("expected outer Add, got {root:?}");
    };
    // outer rhs is Const 1
    assert!(matches!(&s.exprs[*rhs as usize], ir::Expr::Const { .. }));
    // outer lhs is Add(Signal a, Const 1) (under the local-`t` resize)
    match &s.exprs[peel_resize(&s, *lhs) as usize] {
        ir::Expr::Binary {
            op: ir::BinOp::Add,
            lhs: l2,
            ..
        } => assert!(matches!(
            s.exprs[*l2 as usize],
            ir::Expr::Signal { net: 0, .. }
        )),
        other => panic!("expected inner Add, got {other:?}"),
    }
}

// ft-e3. nested non-recursive function call (f calls g) folds fully.
//        function g(input x); g = x + 1; endfunction
//        function f(input x); f = g(x) + 1; endfunction  → y=f(a) == ((a+1)+1)
#[test]
fn ft_e3_nested_function_calls() {
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "y"]),
            func_def(
                "g",
                Some((7, 0)),
                vec![tf_port(ast::PortDir::Input, Some((7, 0)), "x")],
                vec![],
                bassign("g", binop(ast::BinOp::Add, id_expr("x"), dec("1"))),
            ),
            func_def(
                "f",
                Some((7, 0)),
                vec![tf_port(ast::PortDir::Input, Some((7, 0)), "x")],
                vec![],
                bassign(
                    "f",
                    binop(ast::BinOp::Add, call("g", vec![id_expr("x")]), dec("1")),
                ),
            ),
            cont_assign(lv_id("y"), call("f", vec![id_expr("a")])),
        ],
    );
    let s = elab_ok(&unit);
    // outer is f = g(a) + 1, under f's 8-bit return resize.
    let ir::Expr::Binary {
        op: ir::BinOp::Add,
        lhs,
        ..
    } = &s.exprs[peel_resize(&s, s.cont_assigns[0].rhs) as usize]
    else {
        panic!("expected outer Add");
    };
    // inner is g(a) = Add(Signal a, 1), under g's own 8-bit return resize.
    match &s.exprs[peel_resize(&s, *lhs) as usize] {
        ir::Expr::Binary {
            op: ir::BinOp::Add,
            lhs: l2,
            ..
        } => assert!(matches!(
            s.exprs[*l2 as usize],
            ir::Expr::Signal { net: 0, .. }
        )),
        other => panic!("expected inner g() Add, got {other:?}"),
    }
}

// ft-e4. simple task writing an OUTPUT formal, called in an initial block:
//        task setq(input [7:0] d, output [7:0] q); q = d; endtask
//        initial setq(a, y);  → the body's `q = d` lowers to BlockingAssign onto
//        the caller's net y, rhs = the caller's net a.
#[test]
fn ft_e4_task_output_writeback_inline() {
    let unit = module(
        "m",
        vec![
            netvar(ast::NetVarKind::Reg, Some((7, 0)), false, &["a", "y"]),
            task_def(
                "setq",
                vec![
                    tf_port(ast::PortDir::Input, Some((7, 0)), "d"),
                    tf_port(ast::PortDir::Output, Some((7, 0)), "q"),
                ],
                vec![],
                bassign("q", id_expr("d")),
            ),
            proc_item(
                ast::ProcKind::Initial,
                None,
                blk(vec![task_call("setq", vec![id_expr("a"), id_expr("y")])]),
            ),
        ],
    );
    let s = elab_ok(&unit);
    // §13.5.1/§13.5.3 copy-in/copy-out. Nets: a (=0), y (=1), then the formal-width
    // locals — the INPUT d (=2) and the OUTPUT q (=3). The entry block holds THREE
    // statements: copy-in `d_local = a`, the body `q_local = d_local`, and the
    // copy-out `y = q_local`.
    assert_eq!(s.processes.len(), 1);
    let p = &s.processes[0];
    let entry = &p.body[p.entry as usize];
    assert_eq!(entry.stmts.len(), 3);
    let assign = |i: usize| -> (u32, &ir::Expr) {
        match &s.stmts[entry.stmts[i] as usize] {
            ir::Stmt::BlockingAssign { lhs, rhs } => (lhs.chunks[0].net, &s.exprs[*rhs as usize]),
            other => panic!("expected BlockingAssign at stmt {i}, got {other:?}"),
        }
    };
    // stmt[0]: copy-IN d_local (=2) = a (=0).
    assert!(matches!(assign(0), (2, ir::Expr::Signal { net: 0, .. })));
    // stmt[1]: body q_local (=3) = d_local (=2).
    assert!(matches!(assign(1), (3, ir::Expr::Signal { net: 2, .. })));
    // stmt[2]: copy-OUT y (=1) = q_local (=3).
    assert!(matches!(assign(2), (1, ir::Expr::Signal { net: 3, .. })));
}

// ft-e5. unknown function call → E-ELAB-UNRESOLVED-NAME (IR discarded).
#[test]
fn ft_e5_unknown_function_errors() {
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "y"]),
            cont_assign(lv_id("y"), call("nope", vec![id_expr("a")])),
        ],
    );
    let sink = CollectSink::default();
    let out = elaborate(&unit, &sink);
    assert!(out.is_none(), "unknown function must fail elaboration");
    assert!(err_codes(&sink).contains(&MsgCode::ElabUnresolvedName));
}

// ft-e6. recursive function → B1 frame-call: lowered to the func arena (no
//        infinite inline expansion), the call site emits an `Expr::Call`.
#[test]
fn ft_e6_recursive_function_framed() {
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "y"]),
            func_def(
                "rec",
                Some((7, 0)),
                vec![tf_port(ast::PortDir::Input, Some((7, 0)), "x")],
                vec![],
                // rec = rec(x) + 1  → self-call inside its own body
                bassign(
                    "rec",
                    binop(ast::BinOp::Add, call("rec", vec![id_expr("x")]), dec("1")),
                ),
            ),
            cont_assign(lv_id("y"), call("rec", vec![id_expr("a")])),
        ],
    );
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("recursive function now frames (B1)");
    // exactly one FuncDef, well-formed (entry in range, 1 param, not a task).
    assert_eq!(s.funcs.len(), 1, "one frame func");
    let fd = s.funcs[0];
    assert!(!fd.is_task);
    assert_eq!(fd.n_params, 1);
    assert!(
        (fd.entry as usize) < s.blocks.len(),
        "entry indexes the func arena"
    );
    // the cont-assign RHS is an Expr::Call to func 0 with one arg.
    let ca = &s.cont_assigns[0];
    match &s.exprs[ca.rhs as usize] {
        ir::Expr::Call { func, args } => {
            assert_eq!(*func, 0);
            assert_eq!(args.len(), 1);
        }
        other => panic!("cont-assign RHS must be a frame Call, got {other:?}"),
    }
    // the recursive call inside the body lowered to a Call too (no inline blowup).
    assert!(
        s.exprs
            .iter()
            .filter(|e| matches!(e, ir::Expr::Call { func: 0, .. }))
            .count()
            >= 2,
        "both the call site AND the body self-call are Expr::Call"
    );
}

// ft-e7. function whose body has CONTROL FLOW (if/else) → B1 frame-call: lowered
//        to a multi-BB CFG (the `if` is a `Branch` terminator), no longer rejected.
#[test]
fn ft_e7_control_flow_function_framed() {
    let if_body = ast::Stmt::If {
        cond: id_expr("x"),
        then_s: Box::new(bassign("f", dec("1"))),
        else_s: Some(Box::new(bassign("f", dec("0")))),
        span: SP,
    };
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "y"]),
            func_def(
                "f",
                Some((7, 0)),
                vec![tf_port(ast::PortDir::Input, Some((7, 0)), "x")],
                vec![],
                if_body,
            ),
            cont_assign(lv_id("y"), call("f", vec![id_expr("a")])),
        ],
    );
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("control-flow function now frames (B1)");
    assert_eq!(s.funcs.len(), 1);
    let fd = s.funcs[0];
    assert!((fd.entry as usize) < s.blocks.len());
    // the `if/else` lowered to >1 block, and the entry block branches (multi-BB
    // exercises the +base terminator rebase — a single-BB body would not).
    assert!(s.blocks.len() >= 2, "if/else is a multi-BB CFG");
    assert!(
        matches!(
            s.blocks[fd.entry as usize].term,
            ir::Terminator::Branch { .. }
        ),
        "the func entry block ends in a Branch (the `if`)"
    );
    // every Branch/Goto target in the func arena is in range (rebase correct).
    for b in &s.blocks {
        match b.term {
            ir::Terminator::Branch {
                then_bb, else_bb, ..
            } => {
                assert!((then_bb as usize) < s.blocks.len());
                assert!((else_bb as usize) < s.blocks.len());
            }
            ir::Terminator::Goto { target } => assert!((target as usize) < s.blocks.len()),
            _ => {}
        }
    }
    // the call site is an Expr::Call.
    let ca = &s.cont_assigns[0];
    assert!(matches!(
        &s.exprs[ca.rhs as usize],
        ir::Expr::Call { func: 0, .. }
    ));
}

// ── FORK 16. nested fork is a hard ElabUnsupported error (v1 MVP boundary) ────
fn fork_stmt(stmts: Vec<ast::Stmt>, join: ast::JoinKind) -> ast::Stmt {
    ast::Stmt::Fork {
        label: None,
        decls: Vec::new(),
        stmts,
        join,
        span: SP,
    }
}

#[test]
fn nested_fork_is_unsupported_error() {
    // initial fork begin fork a=1; join end join
    // The INNER fork (inside the OUTER fork's child) is the nested case → error.
    let inner = fork_stmt(vec![bassign("a", dec("1"))], ast::JoinKind::Join);
    let child = blk(vec![inner]);
    let outer = fork_stmt(vec![child], ast::JoinKind::Join);
    let unit = module(
        "m",
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["a"]),
            proc_item(ast::ProcKind::Initial, None, outer),
        ],
    );
    let sink = CollectSink::default();
    let (ir, modes) = elaborate_with_modes(&unit, &sink);
    // Inner fork (inside a fork child) → ElabUnsupported error. Elaborate still
    // produces no SimIr (had_error set), but the OUTER fork's mode WAS recorded and
    // the inner fork recorded NO mode entry.
    assert!(diag_codes(&sink).contains(&MsgCode::ElabUnsupported));
    // Only the OUTER fork's mode is recorded; the inner one is rejected.
    assert_eq!(modes.len(), 1);
    assert!(ir.is_none(), "design is rejected by the nested-fork error");
}
