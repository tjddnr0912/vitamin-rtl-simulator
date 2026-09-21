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
//! ⚠️ A REDECLARATION no longer reaches this function's own diagnostic, and there
//! is none here any more. `insert` used to warn `W3056 function \`f\` redeclared;
//! first declaration used` while `BTreeMap::insert` kept the LAST definition — a
//! false sentence on top of a silent-wrong, since both oracles REJECT the design
//! (`m17.sv` / census p20_a, p20_b, p20_d, p20_e printed `RD=49`, the second
//! body). `decl_collide.rs` now refuses the pair per DEFINITION, and a WRITER
//! CENSUS says that refusal covers every duplicate this function can see:
//!
//! * both callers empty the tables first — `instance.rs` does
//!   `std::mem::take(&mut self.func_table)` immediately before its step-(3.5) loop,
//!   and the interface window takes the whole `RoutineScope` at window entry — so
//!   `insert` can only return `Some` for a name declared twice in the BODY this
//!   loop is walking;
//! * that loop walks `module.body` / `decl.body` top-level items, and
//!   `check_decl_name_collisions` collects exactly those `Func`/`Task` items (plus
//!   the transparent-generate ones, which `generate.rs` registers), per definition,
//!   for modules and interfaces alike.
//!
//! So a second diagnostic here would be a second report of one defect. A routine
//! declared twice inside a LABELLED generate block is the one shape outside that
//! census, and it keeps its warning — in `generate.rs`, with the text corrected to
//! what the table actually does.

use super::*;

impl Elaborator<'_> {
    /// Register `item` if it is a scope-declared routine. Returns whether it was one.
    pub(crate) fn register_declared_routine(&mut self, item: &ast::ModuleItem) -> bool {
        match item {
            ast::ModuleItem::Func(f) => {
                self.func_table.insert(f.name.name.clone(), f.clone());
                true
            }
            ast::ModuleItem::Task(t) => {
                self.task_table.insert(t.name.name.clone(), t.clone());
                true
            }
            _ => false,
        }
    }
}
