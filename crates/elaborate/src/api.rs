//! public entry points — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

pub fn elaborate(unit: &ast::SourceUnit, sink: &dyn LogSink) -> Option<ir::SimIr> {
    let (ir, _modes) = elaborate_with_modes(unit, sink);
    ir
}

/// Like [`elaborate`], but also returns the [`ForkModeTable`] the simulate path
/// threads into `SimOpts.fork_modes`. `elaborate` is a thin forwarder onto this
/// so the ~25 existing `elaborate(...)` callers keep compiling verbatim.
pub fn elaborate_with_modes(
    unit: &ast::SourceUnit,
    sink: &dyn LogSink,
) -> (Option<ir::SimIr>, ForkModeTable) {
    let (ir, modes, _names) = elaborate_with_sidecars(unit, sink);
    (ir, modes)
}

/// Like [`elaborate_with_modes`], but ALSO returns the [`NetNameTable`] for VCD
/// hierarchical naming. Both side tables ride in `SimOpts` and never perturb the
/// golden `SimIr`. Uses the `1ns/1ns` timescale base (no delay scaling).
pub fn elaborate_with_sidecars(
    unit: &ast::SourceUnit,
    sink: &dyn LogSink,
) -> (Option<ir::SimIr>, ForkModeTable, NetNameTable) {
    let (ir, sc) = elaborate_with_timescale(unit, sink, &std::collections::BTreeMap::new(), -9);
    (ir, sc.fork_modes, sc.net_names)
}

/// Full elaborate entry with the resolved timescale env from
/// `hdl_preprocess::resolve_module_timescales`. `mod_unit_exp` maps each module name
/// to its delay-unit exponent and `global_prec_exp` is the design-wide tick base;
/// `#delay` literals scale to `round(d × 10^(unit−prec))` ticks. Also returns the
/// per-process multiplier table for `SimOpts.proc_multipliers` (`$time`/`$realtime`
/// scaling) and the [`SeverityTable`] for `$fatal`/`$error`/`$warning`/`$info`.
/// All side tables ride out-of-band; the golden `SimIr` is unchanged.
pub fn elaborate_with_timescale(
    unit: &ast::SourceUnit,
    sink: &dyn LogSink,
    mod_unit_exp: &std::collections::BTreeMap<String, i8>,
    global_prec_exp: i8,
) -> (Option<ir::SimIr>, Sidecars) {
    // Legacy 4-arg surface: no per-module precision map ⇒ every module's prec
    // is taken as the global precision (S = 1) ⇒ the two-stage `#delay`
    // conversion degenerates to the old single rounding. The CLI passes the
    // real map through [`elaborate_with_timescale_prec_roots`].
    elaborate_with_timescale_prec_roots(
        unit,
        sink,
        mod_unit_exp,
        &std::collections::BTreeMap::new(),
        global_prec_exp,
        None,
    )
}

/// [`elaborate_with_timescale`] with an explicit ROOT override (`--top`): when
/// `roots` is `Some`, exactly those units are elaborated as top instances (in
/// the given order) instead of the every-uninstantiated-module default — the
/// worklib/`--top` surface. An unknown name is a loud elaborate error.
pub fn elaborate_with_timescale_roots(
    unit: &ast::SourceUnit,
    sink: &dyn LogSink,
    mod_unit_exp: &std::collections::BTreeMap<String, i8>,
    global_prec_exp: i8,
    roots: Option<&[String]>,
) -> (Option<ir::SimIr>, Sidecars) {
    elaborate_with_timescale_prec_roots(
        unit,
        sink,
        mod_unit_exp,
        &std::collections::BTreeMap::new(),
        global_prec_exp,
        roots,
    )
}

/// [`elaborate_with_timescale_roots`] + the per-module PRECISION map from
/// `hdl_preprocess::resolve_module_timescales` — the full two-stage `#delay`
/// surface (doc-08): a real delay first rounds to the declaring module's own
/// precision, then scales to global ticks. An empty `mod_prec_exp` degenerates
/// to the old single global-grain rounding (S = 1).
pub fn elaborate_with_timescale_prec_roots(
    unit: &ast::SourceUnit,
    sink: &dyn LogSink,
    mod_unit_exp: &std::collections::BTreeMap<String, i8>,
    mod_prec_exp: &std::collections::BTreeMap<String, i8>,
    global_prec_exp: i8,
    roots: Option<&[String]>,
) -> (Option<ir::SimIr>, Sidecars) {
    elaborate_located(
        unit,
        sink,
        mod_unit_exp,
        mod_prec_exp,
        global_prec_exp,
        roots,
        None,
    )
}

/// [`elaborate_with_timescale_prec_roots`] plus a `SpanResolver`, so every
/// elaborate-time diagnostic carries `file:line:col`.
///
/// The front end owns the preprocessor's `SourceMap` and passes this view of it;
/// elaborate stays free of any preprocessor dependency. `resolver: None` reproduces
/// the previous behavior exactly (unlocated diagnostics), which is what the crate's
/// own unit tests and any AST-only caller use.
#[allow(clippy::too_many_arguments)]
pub fn elaborate_located(
    unit: &ast::SourceUnit,
    sink: &dyn LogSink,
    mod_unit_exp: &std::collections::BTreeMap<String, i8>,
    mod_prec_exp: &std::collections::BTreeMap<String, i8>,
    global_prec_exp: i8,
    roots: Option<&[String]>,
    resolver: Option<&dyn diag::SpanResolver>,
) -> (Option<ir::SimIr>, Sidecars) {
    elaborate_located_params(
        unit,
        sink,
        mod_unit_exp,
        mod_prec_exp,
        global_prec_exp,
        roots,
        resolver,
        &[],
    )
}

/// [`elaborate_located`] plus CLI parameter overrides for the TOP instance(s)
/// (`-G NAME=VALUE`). `top_params` is `(name, raw value text)`; an unknown name, a
/// `localparam` target, and a value that does not parse are all loud.
#[allow(clippy::too_many_arguments)]
pub fn elaborate_located_params(
    unit: &ast::SourceUnit,
    sink: &dyn LogSink,
    mod_unit_exp: &std::collections::BTreeMap<String, i8>,
    mod_prec_exp: &std::collections::BTreeMap<String, i8>,
    global_prec_exp: i8,
    roots: Option<&[String]>,
    resolver: Option<&dyn diag::SpanResolver>,
    top_params: &[(String, String)],
) -> (Option<ir::SimIr>, Sidecars) {
    let mut el = Elaborator::new(sink);
    el.span_resolver = resolver;
    el.mod_unit_exp = mod_unit_exp.clone();
    el.mod_prec_exp = mod_prec_exp.clone();
    el.global_prec_exp = global_prec_exp;
    el.root_override = roots.map(<[String]>::to_vec);
    el.top_param_overrides = top_params.to_vec();
    el.run(unit);
    el.assert_block_local_inits_drained();
    let class_rand = el.class_rand_table();
    let class_constraints = el.class_constraints_table();
    let class_dist = el.class_dist_table();
    let class_randc = el.class_randc_table();
    let sc = Sidecars {
        class_rand,
        class_constraints,
        class_dist,
        class_randc,
        fork_modes: std::mem::take(&mut el.fork_modes),
        coverage_manifest: std::mem::take(&mut el.coverage_manifest),
        proc_multipliers: std::mem::take(&mut el.proc_multipliers),
        proc_prec_mults: std::mem::take(&mut el.proc_prec_mults),
        severities: std::mem::take(&mut el.severities),
        timeformat_stmts: std::mem::take(&mut el.timeformat_stmts),
        stage_stmts: std::mem::take(&mut el.stage_stmts),
        handle_copy_stmts: std::mem::take(&mut el.handle_copy_stmts),
        queue_slice_stmts: std::mem::take(&mut el.queue_slice_stmts),
        radixes: std::mem::take(&mut el.radixes),
        proc_scopes: std::mem::take(&mut el.proc_scopes),
        assign_ranks: std::mem::take(&mut el.assign_ranks),
        queue_bounds: std::mem::take(&mut el.queue_bounds),
        net_dims: el.array_dims.clone(), // the sparse decl map IS the table
        net_decl_ranges: std::mem::take(&mut el.net_decl_range),
        file_directed_stmts: std::mem::take(&mut el.file_directed_stmts),
        init_procs: el.init_procs(),
        final_procs: el.final_procs.clone(),
        defer_marks: std::mem::take(&mut el.defer_marks),
        defer_acts: std::mem::take(&mut el.defer_acts),
        func_table: std::mem::take(&mut el.func_metas), // B1 (empty until frame funcs lower)
        func_names: std::mem::take(&mut el.frame_func_names), // N1 %m
        task_calls_proc: std::mem::take(&mut el.task_calls_proc), // B2
        task_calls_func: std::mem::take(&mut el.task_calls_func), // B2
        // SVPART: 2-state net ids, derived from the decl-time kind map (reuses the
        // $typename `intro_kind` table — no extra elaborate state).
        wired_and_nets: std::mem::take(&mut el.wired_and_nets),
        wired_or_nets: std::mem::take(&mut el.wired_or_nets),
        two_state_nets: el
            .intro_kind
            .iter()
            .filter(|(_, k)| net_kind_is_two_state(**k))
            .map(|(&id, _)| id)
            // Heap handles (dyn/queue/assoc) with 2-state elements skip
            // `intro_kind` but still need the 2-state default (§7.5.2).
            .chain(el.two_state_heap_handles.iter().copied())
            .collect(),
        // N3 Phase 2 heterogeneous-heap element-kind markers.
        real_elem_dyn_nets: std::mem::take(&mut el.real_elem_dyn_nets),
        string_elem_dyn_nets: std::mem::take(&mut el.string_elem_dyn_nets),
        // N7 class/OOP sidecars.
        class_handle_nets: std::mem::take(&mut el.class_handle_nets),
        class_new_sites: std::mem::take(&mut el.class_new_sites),
        class_layouts: el.class_layout_table(),
        class_field_inits: el.class_field_inits(),
        class_vtable: std::mem::take(&mut el.class_vtable),
        class_calls: std::mem::take(&mut el.class_calls),
        class_field_widths: std::mem::take(&mut el.class_field_widths),
        randomize_with: std::mem::take(&mut el.randomize_with),
        // SVA-REST assertion control.
        assert_fire: std::mem::take(&mut el.assert_fire),
        assert_ctl: std::mem::take(&mut el.assert_ctl),
        clocking_inputs: std::mem::take(&mut el.clocking_inputs),
        clocking_commit: std::mem::take(&mut el.clocking_commit),
        clocking_outputs: std::mem::take(&mut el.clocking_outputs),
        ca_delays: std::mem::take(&mut el.ca_delays),
        net_names: el.net_name_table(), // BEFORE finish() consumes `el`
        instances_info: std::mem::take(&mut el.instances_info),
    };
    if el.had_error {
        (None, sc)
    } else {
        (Some(el.finish()), sc)
    }
}
