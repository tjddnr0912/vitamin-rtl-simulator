//! §3.b `iface-pkg-routine`: the ROUTINE scope an interface instance elaborates in.
//!
//! Split out of `iface_inst.rs` (which holds the instance flatten itself) at the
//! 1000-line policy: this file owns the module-scope save/restore (`RoutineScope`)
//! and the sibling-to-sibling carry of STATIC scoped package frames
//! (`StaticScopedCarry`); `elaborate_iface_instances` calls both.

use super::*;

/// The ROUTINE scope of the enclosing module, taken while an interface instance's
/// own body is elaborated.
///
/// §3.b `iface-pkg-routine`. An interface instance is flattened INSIDE the parent
/// module's Nets phase (`instance.rs` pass 4c, pass 8 for a generate-nested one),
/// so before this the parent's routine tables were LIVE while the interface body
/// was lowered. Two measured consequences:
///
/// * the interface's own `import pk::g;` bound CONSTANTS, TYPES and VARIABLES but
///   never ROUTINES, so every bare routine call in an interface body was
///   `E3010 call to undeclared function/task` where both oracles print a value
///   (census c1–c6, c9, c14–c17, c19, c22, c24), and a `localparam` folded through
///   an imported constant function was `E3009` (c21);
/// * worse, a bare `g()` in the interface body resolved to the PARENT's `g`:
///   `interface ifc; import pk::g; … g(40)` inside a module that declares its own
///   `function int g` printed `I=1040` where both oracles print `I=44` (c10), and
///   the same with `import pk::*` (c25) or with the parent importing a DIFFERENT
///   package's `g` (c26, `I=39` vs 44) — a silent-wrong at exit 0.
///
/// The field set is exactly what the module lane takes for its own scope: steps
/// (3.5)/(3.6) of `instance.rs` (the routine tables, their declaring-scope and
/// call-shape sidecars, and the frame-id maps) plus the constant-function tables
/// it takes at (3a.5) and the `scope_imports` list `generate.rs` reads. Emptied at
/// window entry, restored verbatim at window exit.
///
/// ⚠️ `wire_ports` is OUTSIDE this window on purpose: a header-port connection's
/// actual is a PARENT expression, so the parent's tables must be live there. They
/// are — the `wire_phase` block sits outside the body-elaboration guard below.
///
/// ⚠️ ONE EXCEPTION, and it is a parity pin rather than a model: a `::`-spelled key
/// (a scoped `pk::f()` root, or a same-package callee `inject_pkg_callees` put in
/// beside it) whose definition is NOT `automatic` travels from one interface
/// instance of the same parent to the NEXT — see [`static_scoped_keys`] and
/// [`StaticScopedCarry`]. A static routine's locals are ONE variable for the whole
/// design (IEEE §6.21), and vita gives a framed routine one copy per SCOPE that
/// frames it: that is the recorded ROADMAP §2 row "a PACKAGE routine's static local
/// is ONE variable", owned by the module lane too. Before this slice an interface
/// body's scoped call reserved its frame in the PARENT's tables, so two instances
/// of the same interface HIT one frame and matched both oracles by accident
/// (`ifc #(1) u1(); ifc #(2) u2();` calling `pk::stat`, whose body is
/// `int x; x = x + a; return x;` — `L1=10 L2=20` on vita, iverilog and verilator
/// alike). Taking the tables per instance without this exception gave each instance
/// its own `x` and printed `L1=10 L2=10`: a REGRESSION on a cell that was right,
/// traded for no gain. So the static scoped frames keep travelling and the §2 row
/// stays exactly where it was, on the lane that owns it.
///
/// Three boundaries of that exception, all deliberate:
///
/// * an AUTOMATIC `::` key stays per-instance. An automatic routine carries no
///   state between calls, so there is nothing for two instances to agree about, and
///   a per-instance frame is what keeps two concurrent activations from sharing one
///   set of locals.
/// * the IMPORT lane (`import pk::stat;`, a bare-name key) is NOT carried. A
///   bare-name table row is this scope's own binding — the whole point of the window
///   — and its static-local answer is the module lane's pre-existing row verbatim
///   (module twin: two instances of a module importing `pk::stat` print `S1=10
///   S2=10` before and after this slice where both oracles print `S1=10 S2=20`).
///   Row `iface-subr` puts the interface's OWN `function`/`task` declarations in the
///   same bare-name space, and they want exactly that treatment: a static routine
///   DECLARED in an interface has one local per interface INSTANCE in both oracles
///   (census d05 `L1=10 L2=10`, module twin m05 identical), which is what a
///   bare-name key uncarried gives.
/// * the PARENT's own tables never receive the carry. See [`StaticScopedCarry`] for
///   the two designs that measured.
pub(crate) struct RoutineScope {
    func_table: BTreeMap<String, ast::FunctionDef>,
    task_table: BTreeMap<String, ast::TaskDef>,
    rtn_pkg: BTreeMap<String, String>,
    rtn_decl_scope: BTreeMap<String, String>,
    rtn_decl_genvars: BTreeMap<String, Vec<(String, i64)>>,
    tf_decl_scope: String,
    /// The instance whose routines the tables describe (`%m`, the OBS
    /// `subroutine_calls[].name`, the relative-scope walks). Round 1 of the review
    /// measured it left at the PARENT's path: `%m` inside an imported routine
    /// called from `top.u` printed `top.g`, dropping the instance segment the
    /// module lane keeps (`top.w.g`).
    pub(crate) inst_prefix: String,
    /// The `module_facts` key of the body being lowered — the interface's own row
    /// (`build_module_facts` files interfaces too). With the PARENT's name still live
    /// after the `inst_prefix` swap above, `hier_leaf_scope` (`expr_size_hier.rs`)
    /// read the interface body as the parent module's body and resolved
    /// hierarchical leaves against the parent's INSTANCE map (round-2 soundness: an
    /// undeclared widening, unmeasured in breadth). Keyed on the interface, the walk
    /// consults the interface's own instance map, which is empty while nested
    /// instances inside an interface are refused — the pre-slice answer (round 3
    /// measured the mechanism: the `?` fires at the instance lookup, not at the
    /// facts lookup). Lifting that refusal makes the same walk resolve an
    /// interface's OWN children, which is the right scope for it.
    cur_module: String,
    inout_func_names: BTreeSet<String>,
    body_write_func_names: BTreeSet<String>,
    dyn_formal_func_names: BTreeSet<String>,
    frame_idx: BTreeMap<String, u32>,
    task_frame_idx: BTreeMap<String, u32>,
    const_func_table: BTreeMap<String, ast::FunctionDef>,
    const_fn_pkg: BTreeMap<String, String>,
    scope_imports: Vec<ast::ImportDecl>,
}

/// The keys [`RoutineScope`] SHARES between an interface window and its enclosing
/// module scope: every `::`-spelled routine key whose definition is not
/// `automatic`.
///
/// ONE predicate serves both directions (adopt and release) so that a `frame_idx`
/// entry can never be separated from the `func_table` definition it indexes — a
/// frame id whose definition stayed behind is a lookup that lowers another scope's
/// body, and a definition whose frame id stayed behind is re-reserved as a second
/// copy of a variable the standard keeps single.
///
/// Only the SCOPED lane writes `::` keys: `frame_scoped_pkg_routines`
/// (`pkg_scoped_frames.rs`) inserts the root under `pkg::name` and reserves its
/// frame, and `inject_pkg_callees` (`package.rs`) puts the root's transitive
/// same-package callees beside it under the same spelling. A bare name never
/// contains `::`, so the import lane is untouched by this.
fn static_scoped_keys(
    func_table: &BTreeMap<String, ast::FunctionDef>,
    task_table: &BTreeMap<String, ast::TaskDef>,
) -> BTreeSet<String> {
    func_table
        .iter()
        .filter(|(k, f)| k.contains("::") && !f.automatic)
        .map(|(k, _)| k.clone())
        .chain(
            task_table
                .iter()
                .filter(|(k, t)| k.contains("::") && !t.automatic)
                .map(|(k, _)| k.clone()),
        )
        .collect()
}

/// Move every `keys` entry from `from` into `to`, leaving an absent key alone.
fn move_keyed_map<V>(
    from: &mut BTreeMap<String, V>,
    to: &mut BTreeMap<String, V>,
    keys: &BTreeSet<String>,
) {
    for k in keys {
        if let Some(v) = from.remove(k) {
            to.insert(k.clone(), v);
        }
    }
}

/// [`move_keyed_map`] for the call-shape name sets.
fn move_keyed_set(from: &mut BTreeSet<String>, to: &mut BTreeSet<String>, keys: &BTreeSet<String>) {
    for k in keys {
        if from.remove(k) {
            to.insert(k.clone());
        }
    }
}

/// The seven routine maps keyed by a routine NAME, borrowed together.
///
/// They travel as one because [`static_scoped_keys`] decides for all seven at once:
/// a `frame_idx` id whose `func_table` definition stayed in the other scope is a
/// lookup that lowers a body from somewhere else, and a definition whose frame id
/// stayed behind is re-reserved as a second copy of a variable IEEE §6.21 keeps
/// single. Naming them in one place is what makes "all seven or none" checkable.
struct RtnTables<'a> {
    func: &'a mut BTreeMap<String, ast::FunctionDef>,
    task: &'a mut BTreeMap<String, ast::TaskDef>,
    frame: &'a mut BTreeMap<String, u32>,
    task_frame: &'a mut BTreeMap<String, u32>,
    inout: &'a mut BTreeSet<String>,
    body_write: &'a mut BTreeSet<String>,
    dyn_formal: &'a mut BTreeSet<String>,
}

/// Move `from`'s static scoped routine keys ([`static_scoped_keys`], computed from
/// `from`'s own definitions) into `to`, in all seven maps.
///
/// A key `to` ALREADY DEFINES is left in `from`, in all seven maps at once. Round 2
/// of the review measured the overwrite: the window's own import lane had bound a
/// static callee under its `pk::h` key and lowered the imported root's body against
/// that frame, and the adopt then displaced `frame_idx["pk::h"]` with the sibling's
/// fid — one interface instance held TWO copies of one static local (`h=210` where
/// the round-1 build, the module twin and both oracles' per-scope value agree on
/// `110`). The decision is per key and applies to every map, so a frame id is never
/// separated from its definition.
fn move_static_scoped(from: RtnTables<'_>, to: RtnTables<'_>) {
    let mut keys = static_scoped_keys(from.func, from.task);
    keys.retain(|k| !to.func.contains_key(k) && !to.task.contains_key(k));
    move_keyed_map(from.func, to.func, &keys);
    move_keyed_map(from.task, to.task, &keys);
    move_keyed_map(from.frame, to.frame, &keys);
    move_keyed_map(from.task_frame, to.task_frame, &keys);
    move_keyed_set(from.inout, to.inout, &keys);
    move_keyed_set(from.body_write, to.body_write, &keys);
    move_keyed_set(from.dyn_formal, to.dyn_formal, &keys);
}

/// The static scoped package routines already framed by an EARLIER interface
/// instance of the same parent, waiting for the next sibling to hit them.
///
/// The frames go here and NOT back into the parent module's own tables, and the
/// difference was measured twice:
///
/// * `package pk; function automatic int h(…); function int g(…); return h(m);
///   endfunction endpackage` with an interface calling `pk::g(21)` — depositing the
///   static root `pk::g` into the parent left its AUTOMATIC callee `pk::h` behind
///   (automatic keys are per-instance on purpose), and the parent's own step 6.5
///   re-lowered `pk::g`'s body against a table that no longer had `h`:
///   `E3010 call to undeclared function h [in top.$func$pk::g]` on a design PRE and
///   both oracles run (`V=44`).
/// * a parent whose own `pk::g(2)` call follows an interface that imported `pk::g`
///   by its bare name: the import lane injects the transitive callees under `::`
///   keys too, so depositing them handed the parent's scoped call a frame the
///   interface had lowered — `E3010` became `V=2` where both oracles say `V=3`,
///   trading a loud cell for a silent-wrong one.
///
/// So the carry is the sibling-to-sibling channel only. Keyed by the parent MODULE
/// INSTANCE's path (`inst_prefix`, which is `cur_prefix` without the generate-scope
/// segments), because that is the scope the pre-slice build shared: a module
/// instance's routine tables are its own (`instance.rs` step 3.5), so an interface
/// in `top.a` never saw what an interface in `top.b` framed, while a
/// generate-nested `top.gb.i` and a plain `top.i` shared one table.
///
/// One consequence to expect in the OBS rail, and it is the pre-slice cardinality:
/// one frame means ONE `subroutine_calls[]` row, filed under the instance that
/// reserved it, with the sibling's entries counted into it
/// (`ifc #(1) u1(); ifc #(2) u2();` calling `pk::stat` gives one row
/// `top.u1.stat calls=2`, where the pre-slice build gave one row `top.stat
/// calls=2`). The static `subroutines[]` row is unaffected: it counts lowered call
/// SITES, and both sites are still lowered.
/// ⚠️ Which copy a scoped call joins is the pre-existing ROADMAP §2 row (a static
/// package local is one per SCOPE in vita, one design-wide in both oracles) and
/// the carry does not change that: an instance whose IMPORT lane bound the same
/// `::` key keeps its own copy (the adopt never overwrites, see
/// `move_static_scoped`) and exports it only when the carry is still empty, so
/// with an import-lane instance between two scoped-call instances the count of
/// copies follows the declaration order (round-3 soundness S3-1, three orders
/// measured). The fix shape is the design-wide frame, not another rule here.
#[derive(Default)]
pub(crate) struct StaticScopedCarry {
    func_table: BTreeMap<String, ast::FunctionDef>,
    task_table: BTreeMap<String, ast::TaskDef>,
    frame_idx: BTreeMap<String, u32>,
    task_frame_idx: BTreeMap<String, u32>,
    inout_func_names: BTreeSet<String>,
    body_write_func_names: BTreeSet<String>,
    dyn_formal_func_names: BTreeSet<String>,
}

impl StaticScopedCarry {
    fn tables(&mut self) -> RtnTables<'_> {
        RtnTables {
            func: &mut self.func_table,
            task: &mut self.task_table,
            frame: &mut self.frame_idx,
            task_frame: &mut self.task_frame_idx,
            inout: &mut self.inout_func_names,
            body_write: &mut self.body_write_func_names,
            dyn_formal: &mut self.dyn_formal_func_names,
        }
    }
}

impl Elaborator<'_> {
    /// Take the enclosing module's [`RoutineScope`], leaving this interface
    /// instance's own (empty) one in its place. `decl_scope` is the instance path —
    /// the scope IEEE evaluates a routine's default arguments in — and `imports` is
    /// the interface's own import list.
    ///
    /// The static scoped keys are NOT handed over here — see
    /// [`Self::adopt_static_scoped_frames`], which runs later in the window for a
    /// measured reason.
    pub(crate) fn take_routine_scope(
        &mut self,
        decl_scope: String,
        iface_name: String,
        imports: Vec<ast::ImportDecl>,
    ) -> RoutineScope {
        RoutineScope {
            func_table: std::mem::take(&mut self.func_table),
            task_table: std::mem::take(&mut self.task_table),
            rtn_pkg: std::mem::take(&mut self.rtn_pkg),
            rtn_decl_scope: std::mem::take(&mut self.rtn_decl_scope),
            rtn_decl_genvars: std::mem::take(&mut self.rtn_decl_genvars),
            tf_decl_scope: std::mem::replace(&mut self.tf_decl_scope, decl_scope.clone()),
            inst_prefix: std::mem::replace(&mut self.inst_prefix, decl_scope),
            cur_module: std::mem::replace(&mut self.cur_module, iface_name),
            inout_func_names: std::mem::take(&mut self.inout_func_names),
            body_write_func_names: std::mem::take(&mut self.body_write_func_names),
            dyn_formal_func_names: std::mem::take(&mut self.dyn_formal_func_names),
            frame_idx: std::mem::take(&mut self.frame_idx),
            task_frame_idx: std::mem::take(&mut self.task_frame_idx),
            const_func_table: std::mem::take(&mut self.const_func_table),
            const_fn_pkg: std::mem::take(&mut self.const_fn_pkg),
            scope_imports: std::mem::replace(&mut self.scope_imports, imports),
        }
    }

    /// This interface window's own view of the seven name-keyed routine maps.
    fn live_rtn_tables(&mut self) -> RtnTables<'_> {
        RtnTables {
            func: &mut self.func_table,
            task: &mut self.task_table,
            frame: &mut self.frame_idx,
            task_frame: &mut self.task_frame_idx,
            inout: &mut self.inout_func_names,
            body_write: &mut self.body_write_func_names,
            dyn_formal: &mut self.dyn_formal_func_names,
        }
    }

    /// Take the [`StaticScopedCarry`] an earlier sibling instance of `parent` left,
    /// so a `pk::stat()` it already framed is HIT by this body's call
    /// (`inline_pkg_function`'s `frame_idx` hit path) instead of reserving a second
    /// frame with a second copy of the routine's static local. See [`RoutineScope`]
    /// for why the sharing exists and [`StaticScopedCarry`] for why it is a side
    /// channel rather than the parent's own tables.
    ///
    /// ⚠️ POSITION IS THE WHOLE MECHANISM: this must run AFTER `lower_frame_funcs`
    /// (step 6.5) and BEFORE the Logic loop. Step 6.5 builds its frame set from
    /// EVERY `func_table` entry and reserves each one unconditionally, and it
    /// `clear()`s `inout_func_names` / `body_write_func_names` /
    /// `dyn_formal_func_names` before its own early return. Adopting at window entry
    /// therefore fed the carried keys straight back into a re-reservation: measured,
    /// `ifc #(1) u1(); ifc #(2) u2();` calling `pk::stat` handed over `frame_idx
    /// {"pk::stat": 0}` and u2's step 6.5 answered with a fresh id 1 — the same
    /// second copy, and the three call-shape sets wiped besides. In the module lane
    /// the question never arises: a `::` key can only be written by pass 7
    /// (`inline_pkg_function`), after that module's own step 6.5 has run.
    ///
    /// A HIT needs no callee closure: `inline_pkg_function` returns the carried
    /// FuncId and emits the call, so no body is lowered here and no sibling name is
    /// looked up.
    pub(crate) fn adopt_static_scoped_frames(&mut self, parent: &str) {
        let Some(mut carry) = self.iface_static_scoped.remove(parent) else {
            return;
        };
        let to = self.live_rtn_tables();
        move_static_scoped(carry.tables(), to);
        // Whatever this window already defined stays in the carry for the next
        // sibling (see `move_static_scoped`).
        if !carry.func_table.is_empty() || !carry.task_table.is_empty() {
            self.iface_static_scoped.insert(parent.to_string(), carry);
        }
    }

    /// Put this window's static scoped frames into `parent`'s
    /// [`StaticScopedCarry`], for the next sibling instance to hit.
    pub(crate) fn release_static_scoped_frames(&mut self, parent: &str) {
        let mut carry = self.iface_static_scoped.remove(parent).unwrap_or_default();
        let from = self.live_rtn_tables();
        move_static_scoped(from, carry.tables());
        self.iface_static_scoped.insert(parent.to_string(), carry);
    }

    /// Put the enclosing module's [`RoutineScope`] back. All of it or none: every
    /// field is one half of a decision another field carries (a `frame_idx` entry
    /// without its `func_table` definition, or a `rtn_pkg` entry without the table
    /// row it describes, is a lookup that answers with another scope's routine).
    pub(crate) fn restore_routine_scope(&mut self, s: RoutineScope) {
        self.func_table = s.func_table;
        self.task_table = s.task_table;
        self.rtn_pkg = s.rtn_pkg;
        self.rtn_decl_scope = s.rtn_decl_scope;
        self.rtn_decl_genvars = s.rtn_decl_genvars;
        self.tf_decl_scope = s.tf_decl_scope;
        self.inst_prefix = s.inst_prefix;
        self.cur_module = s.cur_module;
        self.inout_func_names = s.inout_func_names;
        self.body_write_func_names = s.body_write_func_names;
        self.dyn_formal_func_names = s.dyn_formal_func_names;
        self.frame_idx = s.frame_idx;
        self.task_frame_idx = s.task_frame_idx;
        self.const_func_table = s.const_func_table;
        self.const_fn_pkg = s.const_fn_pkg;
        self.scope_imports = s.scope_imports;
    }
}
