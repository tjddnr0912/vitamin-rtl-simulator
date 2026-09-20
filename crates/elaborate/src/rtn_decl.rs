//! The ONE registration of a scope-DECLARED `function`/`task` into the routine
//! tables, shared by the module lane and the interface-instance window.
//!
//! §3.b `iface-subr`. `instance.rs`'s step (3.5) owned this as two inline match
//! arms; `iface_inst.rs` needs exactly the same two, because a routine declared in
//! an interface body binds under exactly the module lane's §26.3 rule — the
//! declaration is in the tables BEFORE `apply_import_routines` runs, so a declared
//! `g` beside `import pk::*` wins the wildcard (both oracles `R=1040`). Written
//! twice the two lanes would drift, and the redeclaration behaviour below is
//! precisely the kind of thing that must move in ONE place when it is fixed.
//!
//! ⚠️ The containment gate (`check_block_local_scope_leaks`) is NOT part of this
//! function, deliberately: the two lanes run it at different points. The module
//! lane computes the five block-local classifier maps at step (3b), BEFORE (3.5),
//! so it gates each routine body at registration time; the interface window
//! computes them AFTER its import passes, so it gates the routine bodies in the
//! same loop that gates its `Proc` bodies. Folding the gate in here would have run
//! it against the PARENT module's maps in the interface lane.
//!
//! ⚠️ `insert` keeps the LAST definition while the warning says "first declaration
//! used" — a pre-existing MODULE-lane defect, measured: `m17.sv` (two
//! `function automatic int f`, returning `a+4` then `a+9`) warns and prints
//! `RD=49`, where iverilog and verilator both REJECT the design. It is mirrored
//! rather than fixed here so the interface twin behaves identically; the fix is one
//! edit to this function, which is why it is a function.

use super::*;

impl Elaborator<'_> {
    /// Register `item` if it is a scope-declared routine, warning on a redeclaration.
    /// Returns whether it was one.
    pub(crate) fn register_declared_routine(&mut self, item: &ast::ModuleItem) -> bool {
        match item {
            ast::ModuleItem::Func(f) => {
                if self
                    .func_table
                    .insert(f.name.name.clone(), f.clone())
                    .is_some()
                {
                    self.warn(&format!(
                        "function `{}` redeclared; first declaration used",
                        f.name.name
                    ));
                }
                true
            }
            ast::ModuleItem::Task(t) => {
                if self
                    .task_table
                    .insert(t.name.name.clone(), t.clone())
                    .is_some()
                {
                    self.warn(&format!(
                        "task `{}` redeclared; first declaration used",
                        t.name.name
                    ));
                }
                true
            }
            _ => false,
        }
    }
}
