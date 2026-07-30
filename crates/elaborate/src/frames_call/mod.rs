//! frame call sites — split out of the original `elaborate` lib.rs (mechanical move).
//!
//! R19 policy split (the file had grown past the 1000-line cap): `args` holds the
//! ACTUAL→FORMAL binding (named-argument reorder, default fill), `emit` holds the
//! call emitters, and the small shared helpers stay here.

use super::*;

mod args;
mod emit;

impl Elaborator<'_> {
    /// R5-B: a fresh named temp holding an inout-function call's RETURN value. The
    /// name lets a synthetic `Ident` reference it (module-scoped, like
    /// `fresh_string_temp`); the net id builds the return-capture out-bind lvalue.
    pub(crate) fn emit_discarded_call(&mut self, b: &mut ProcessBuilder, call: u32) {
        let tmp = match self.cur_discard {
            Some(d) => d,
            None => {
                let w = self.ir_bits_of(call).unwrap_or(32).max(1);
                self.fresh_ia_tmp(w)
            }
        };
        let sid = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: whole_net_lvalue(tmp),
            rhs: call,
        });
        b.push_stmt_id(sid);
    }

    /// v7 P2-C: is `name` a formal DECLARED `string` in the body being lowered?
    /// Innermost-wins (a shadowing inner non-string formal returns `false`, so an
    /// outer string formal of the same name never leaks in). `false` if not a formal.
    pub(crate) fn formal_is_string(&self, name: &str) -> bool {
        self.formal_str
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, s)| *s)
            .unwrap_or(false)
    }

    /// Inline a user-function call at an expression site → the ExprId of its return
    /// value (SD2 inline path; a 0-time function = zero schema cost). The common
    /// case is a combinational function whose body reduces to the return expression
    /// once the formals are substituted by the actual-arg ExprIds. Returns a
    /// placeholder ExprId on any unsupported shape (after emitting the diagnostic)
    /// so arena edges stay valid.
    /// IEEE §13.5.3: build the effective actual-argument list for a tf call, filling
    /// omitted TRAILING formals with their default values. Returns None (after a loud
    /// diagnostic) on too many actuals, or a missing actual for a formal that has no
    /// default. The default expressions are lowered in the CALLER scope at the call
    /// site, like any other actual (so a constant / module-scope default just works;
    /// a default that references an earlier FORMAL resolves in the caller's scope,
    /// not the formal — out of scope here).
    /// G10 (IEEE §13.5.4): reorder named arguments (`.formal(v)` / `.formal()`) to
    /// positional using the callee's formal list. Leading positional args fill slots
    /// 0..k; each named arg scatters to its formal's index; every unbound slot uses the
    /// formal's default. Loud (correct-or-loud) on: an unknown / duplicated formal, a
    /// positional arg after a named one, a `.formal()` with no default, a default that
    /// references another formal, a missing actual, or too many positionals. Returns the
    /// fully-positional args (owned) so both the frame and inline call paths see a plain
    /// list. Only invoked when at least one arg is a `NamedArg`.
    pub(crate) fn fresh_ret_temp(
        &mut self,
        func: &ast::FunctionDef,
        rw: u32,
        rsig: bool,
    ) -> (u32, String) {
        if func.ret_string {
            let name = self.fresh_string_temp();
            ((self.nets.len() - 1) as u32, name)
        } else {
            // A `real`/`realtime` return needs an IEEE-754 f64 net. Building a `Reg` for it
            // (which this did for every non-string return) rounded the value through the
            // integer domain — `return 1.5` came back as `2.000000`, silently, on every path
            // that goes through this temp. Pre-existing, and reachable from the direct rhs
            // since §4.5.274; the general hoister would have carried it into every other
            // expression position, so it is closed here rather than multiplied.
            let is_real = matches!(
                func.ret_type,
                ast::ParamType::Real | ast::ParamType::Realtime
            );
            let w = if is_real { 64 } else { rw.max(1) };
            let name = format!("$ia_ret${}", self.nets.len());
            let net = self.nets.len() as u32;
            self.add_net(
                &name,
                ir::NetVar {
                    kind: if is_real {
                        ir::NetKind::Real
                    } else if w == 32 && rsig {
                        ir::NetKind::Integer
                    } else {
                        ir::NetKind::Reg
                    },
                    width: w,
                    msb: w.saturating_sub(1),
                    lsb: 0,
                    signed: is_real || rsig,
                    array_len: 1,
                    dir: ir::PortDir::Internal,
                    init: default_init(
                        if is_real {
                            ast::NetVarKind::Real
                        } else {
                            ast::NetVarKind::Reg
                        },
                        w,
                    ),
                },
            );
            (net, name)
        }
    }

    /// Declared return self-width + signedness of a function (`function [15:0]`,
    /// `function integer`, `function signed [7:0]`, bare `function`).
    pub(crate) fn func_return_dims(&mut self, func: &ast::FunctionDef) -> (u32, bool) {
        let kind = match func.ret_type {
            ast::ParamType::Integer => ast::NetVarKind::Integer,
            ast::ParamType::Real => ast::NetVarKind::Real,
            ast::ParamType::Realtime => ast::NetVarKind::Realtime,
            ast::ParamType::Time => ast::NetVarKind::Time,
            ast::ParamType::Implicit => ast::NetVarKind::Reg,
        };
        let (w, _msb, _lsb, signed) = self.range_to_dims(kind, func.range.as_ref(), func.signed);
        (w, signed)
    }

    /// True if `a` is a bare net Ident or an integer/string literal — i.e. a thing
    /// `lower_expr` can lower without a fatal unresolved-name. A hierarchical /
    /// scope name (`top.dut`) or anything else returns false (dump-family skips it).
    pub(crate) fn is_net_or_const_arg(&self, a: &ast::Expr) -> bool {
        match &a.kind {
            ast::ExprKind::Ident(path) => {
                path.segments.len() == 1
                    && self.symbols.contains_key(&self.fq(&path.segments[0].name))
            }
            ast::ExprKind::IntLit { .. } | ast::ExprKind::StrLit { .. } => true,
            _ => false,
        }
    }
}
