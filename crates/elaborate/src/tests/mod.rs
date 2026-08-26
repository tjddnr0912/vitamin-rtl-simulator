//! elaborate v1 tests — build a small hdl-ast by hand, elaborate, assert SimIr.

use std::cell::RefCell;

use diag::{LogEvent, LogSink};
use hdl_ast as ast;

use super::*;
use crate::literal::parse_int_literal;

mod core_t;
mod functask_ft;
mod gen_ge;
mod inst_v3;
mod proc_v2;

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
        from_gate: false,
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
            nettype_none: false,
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
                iff: None,
                span: SP,
            })
            .collect(),
    )
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
        unpacked: Vec::new(),
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
        nettype_none: false,
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
                        implicit_name: false,
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
                        implicit_name: false,
                    })
                    .collect(),
                false,
            ),
            span: SP,
        }],
        span: SP,
    })
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
                        implicit_name: false,
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
// ════════════════════════ user function / task inlining ════════════════════════
// builders
fn tf_port(dir: ast::PortDir, range: Option<(u32, u32)>, name: &str) -> ast::TfPort {
    ast::TfPort {
        dir,
        dir_spelling: ast::TfDirSpelling::Declared,
        net_or_var: None,
        signed: false,
        range: range.map(|(m, l)| ast::Range {
            msb: dec(&m.to_string()),
            lsb: dec(&l.to_string()),
            span: SP,
        }),
        name: ident(name),
        unpacked: Vec::new(),
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
        ret_string: false,
        name: ident(name),
        ports,
        body_decls,
        body_enums: Vec::new(),
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
        body_enums: Vec::new(),
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
