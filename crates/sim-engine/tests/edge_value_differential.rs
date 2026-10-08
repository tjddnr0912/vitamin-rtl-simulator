//! §4.5.601: the value elaborate decides for a part-select width, an indexed width or a
//! replication count (`sim_ir::selfwidth::edge_value`) IS the run-time value of the same
//! expression. Random trees over exactly `edge_value`'s arms (`Const`, `+`, `-`,
//! `$clog2` of one constant) are displayed with `%0d` by the run-time evaluator, which
//! sizes each at its self-determined width and sign; every displayed value must equal
//! `edge_bound_value` of the same tree, and `edge_value` of it as a count must be that
//! value or `Negative`. No external oracle: the property is that the decision and the
//! evaluator are one rule.

use std::collections::BTreeMap;

use diag::{LogEvent, LogSink};
use sim_engine::{simulate_capture, SimOpts};
use sim_ir::selfwidth::{edge_bound_value, edge_value, EdgeFault, EdgeSpec, ExprCtx};
use sim_ir::{BinOp, BitPacked, ConstRepr, ConstVal, Expr, SimIr, Stmt, SysFuncId, SysTaskId};

struct Quiet;
impl LogSink for Quiet {
    fn emit(&self, _: LogEvent) {}
}

const N: usize = 400;

fn design() -> SimIr {
    let mut src = String::from("module t;\ninitial begin\n");
    for _ in 0..N {
        src.push_str("  $display(\"%0d\", 8'd0);\n");
    }
    src.push_str("end\nendmodule\n");
    let (toks, le) = hdl_lexer::lex(&src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, &src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let (ir, _) = elaborate::elaborate_with_timescale(
        &su.expect("source unit"),
        &Quiet,
        &BTreeMap::new(),
        -9,
    );
    ir.expect("elaborate")
}

/// xorshift64, seeded: no `rand` dependency, the same trees on every run.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn konst(ir: &mut SimIr, w: u32, signed: bool, v: u128) -> u32 {
    let v = if w >= 128 { v } else { v & ((1u128 << w) - 1) };
    let nw = w.div_ceil(64).max(1) as usize;
    let mut val = vec![0u64; nw];
    val[0] = v as u64;
    if nw > 1 {
        val[1] = (v >> 64) as u64;
    }
    ir.consts.push(ConstVal {
        width: w,
        signed,
        repr: ConstRepr::Numeric,
        bits: BitPacked {
            val,
            unk: vec![0; nw],
        },
    });
    let c = (ir.consts.len() - 1) as u32;
    push(ir, Expr::Const { val: c })
}

fn push(ir: &mut SimIr, e: Expr) -> u32 {
    ir.exprs.push(e);
    (ir.exprs.len() - 1) as u32
}

fn tree(ir: &mut SimIr, rng: &mut Rng, depth: u32) -> u32 {
    if depth == 0 || rng.below(3) == 0 {
        let v = u128::from(rng.next()) | (u128::from(rng.next()) << 64);
        if rng.below(10) == 0 {
            let w = 1 + rng.below(64) as u32;
            let a = konst(ir, w, false, v);
            return push(
                ir,
                Expr::SysFunc {
                    which: SysFuncId::Clog2,
                    args: vec![a],
                },
            );
        }
        let w = 1 + rng.below(100) as u32;
        let signed = rng.below(2) == 1;
        return konst(ir, w, signed, v);
    }
    let l = tree(ir, rng, depth - 1);
    let r = tree(ir, rng, depth - 1);
    let op = if rng.below(2) == 0 {
        BinOp::Add
    } else {
        BinOp::Sub
    };
    push(ir, Expr::Binary { op, lhs: l, rhs: r })
}

#[test]
fn the_decided_value_is_the_run_time_value() {
    let mut ir = design();
    let displays: Vec<usize> = ir
        .stmts
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            matches!(
                s,
                Stmt::SysTask {
                    which: SysTaskId::Display,
                    ..
                }
            )
        })
        .map(|(i, _)| i)
        .collect();
    assert_eq!(displays.len(), N);
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    let mut roots = Vec::new();
    for &si in &displays {
        let root = tree(&mut ir, &mut rng, 3);
        if let Stmt::SysTask { args, .. } = &mut ir.stmts[si] {
            args[0] = root;
        }
        roots.push(root);
    }
    let (_, out) = simulate_capture(&ir, SimOpts::default());
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), N, "{out}");
    let ctx = ExprCtx::of(&ir);
    let mut negative = 0;
    for (root, line) in roots.iter().zip(lines) {
        let shown: i128 = line
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("not a number: {line}"));
        let decided = edge_bound_value(ctx, *root).expect("every arm is decidable");
        assert_eq!(decided, shown, "tree {root}");
        let count = edge_value(ctx, EdgeSpec::Count { n: *root });
        if shown < 0 {
            negative += 1;
            assert_eq!(count, Err(EdgeFault::Negative));
        } else {
            assert_eq!(count, Ok(u64::try_from(shown).unwrap_or(u64::MAX)));
        }
    }
    // the seed reaches both signs
    assert!(negative > 0 && negative < N);
}
