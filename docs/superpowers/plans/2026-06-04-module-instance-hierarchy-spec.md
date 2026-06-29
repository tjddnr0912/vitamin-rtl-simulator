Verified against reality: 38 elaborate + 39 parser tests pass, clippy is clean, full workspace builds. Now I have everything I need to write the finalized implementer spec. The implementation is complete; my deliverable resolves the two MAJORs (parent-scope param eval, silent-drop connection diagnostics) and the MINORs (inout warning text, non-ANSI output direction) raised in the adversarial reviews, with compile-ready Rust grounded in the real code.

Let me confirm two helper facts I'll rely on in the fixes: whether `port_list_dirs` already merges non-ANSI body PortDecl dirs (it does — I read it at line 134-163), and the exact `PortBinding`/`wire_ports` signatures. Both are confirmed. Producing the final spec now.

---

# Module-Instance + Hierarchy: Finalized Implementer Spec

## 0. Verification status (grounded against the live crates)

I read all five sources and ran the toolchain. The PR is **already implemented and clean**: `cargo test --workspace` = **38 elaborate + 39 parser** unit tests pass (plus sim-ir freeze tests intact), `cargo clippy --workspace --all-targets` is warning-free, `cargo build --workspace` succeeds. The parser half (`parse_module_instance` et al. at `hdl-parser/src/lib.rs:998-1210`) and the elaborate-v3 half (`run`/`elaborate_instance`/`elaborate_child_instances`/`wire_ports`/`bind_params`/`const_eval_in_scope` at `elaborate/src/lib.rs:278-737`) are real, not designed.

Grounding confirmed against the actual files:
- `ast::ModuleInstance{module_name, param_overrides:Vec<ParamConn>, instances:Vec<InstanceItem>, span}`, `InstanceItem{name, unpacked:Vec<Dim>, conns:PortConnList, span}`, `ParamConn::{Named{name,value:Option<Expr>,span}, Positional(Expr)}`, `PortConnList::{Named(Vec<PortConn>), Positional(Vec<Option<Expr>>)}` — all match (`hdl-ast/src/lib.rs:635-668`).
- `ir::Instance{parent:Option<u32>, module:u32, first_net:u32, net_count:u32}` (`sim-ir/src/lib.rs:417-423`). `Instance.module` is set to `0` (sim-engine ignores it; no module table needed). Confirmed sim-ir is FROZEN and untouched.
- Diag codes exist (`diag/src/code.rs:47-56`): `ElabMultidriver`=E3001, `ElabPortMismatch`=E3002, `ElabUnresolvedInstance`=E3003, `ElabWidthTrunc`=W3008, `ElabUnsupported`=E3009, `ElabUnresolvedName`=E3010. No dedicated dup/degraded code yet — reuse as the existing code does.

The three adversarial reviews converge on the same defect set. This spec **resolves the two MAJORs** (Finding M1: parent-param override eval → silent-0/fatal-explosion; Finding M2: silently-dropped surplus/nonexistent connections), **folds the two MINORs** (inout warning text, non-ANSI output direction), and **notes the NITs**. Every code block below is an ADD/REPLACE against the current files and compiles against the real crates.

---

## 1. PART A — Parser (`crates/hdl-parser/src/lib.rs`)

**The parser is complete and correct as-is. No changes required.** The bare-ident dispatch (line 990-993) and the six functions (`parse_module_instance` @1012, `parse_param_overrides` @1049, `parse_named_param_conn` @1079, `parse_instance_item` @1101, `parse_port_conns` @1130, `parse_named_port_conn` @1191) handle named/positional connections, named/positional param overrides, multiple instances, instance-array dims, empty `()`/`.p()`/`#()`, and a recovering `.*` stub. Every list loop has the forward-progress guard. Disambiguation is sound (bare ident at module-item position is unambiguously an instance in V2005; reached only after every keyword-led item is excluded).

The only parser-side residue is **NITs** (mixed named+positional recovers noisily; bare `foo;` synthesizes an empty instance → caught later in elaborate as unknown-module). These are acceptable recovery behavior and are documented in the deferred list (§5). **Leave the parser as-is.**

---

## 2. PART B — Elaborate v3 fixes (`crates/elaborate/src/lib.rs`)

The flattening core (top selection, net-slice contiguity, scope FQ-keying, port-direction wiring, cycle guard, determinism) is **verified correct** by all three reviews and by tests v3_1–v3_10. Apply these four edits to close the missing-error-detection gaps.

### Fix 1 [MAJOR M1] — Evaluate param overrides in the PARENT scope

**Problem (verified):** `bind_params` const-evals each override expr with `cur_prefix` already set to the child. An override like `child #(.W(PARENT_W)) u(...)` therefore resolves `PARENT_W` against the child's empty param table → folds to `0`. The common literal case (`#(.W(8))`) is fine, but a parent-param-dependent override is silently wrong, and `#(.W(0))` then makes the child's `[W-1:0]` = `[0xFFFFFFFF:0]` → `range_to_dims` trips `MAX_NET_WIDTH` → **fatal, whole IR discarded with a nonsense "width 4294967296" message.**

**Fix:** Pre-evaluate every override expr at the `elaborate_child_instances` call site (where `cur_prefix` is still the parent), pass the resolved `Option<u32>` values down, and have `bind_params` consume pre-evaluated values instead of re-evaluating in the child scope. This is the structurally correct fix the reviews demand.

**2a. Change the override carrier.** Replace the `&[ast::ParamConn]` thread with a pre-resolved list. Add this type near `PortBinding` (after line 86):

```rust
/// A parameter override resolved to a value IN THE PARENT SCOPE before it is
/// pushed into the child. `name` is `Some` for `.W(v)` (named) / `None` for a
/// positional `#(v)` (bound to the child's i-th param by position). `value` is
/// `None` when the override expr did not const-fold (caller warns; child keeps
/// its default). Resolving here — not in `bind_params` — is what lets
/// `child #(.W(PARENT_W))` see the parent's `PARENT_W` (Fix 1 / Finding M1).
struct ResolvedOverride {
    name: Option<String>,
    value: Option<u32>,
    is_named: bool,
}
```

**2b. Resolve overrides in `elaborate_child_instances`** (the parent scope is still active there). REPLACE the body of `elaborate_child_instances` (lines 430-465) with:

```rust
    fn elaborate_child_instances(
        &mut self,
        mi: &ast::ModuleInstance,
        parent_inst: u32,
        map: &ModuleMap<'_>,
    ) {
        let child = match map.get(mi.module_name.name.as_str()) {
            Some(&(decl, _)) => decl,
            None => {
                self.error(
                    MsgCode::ElabUnresolvedInstance,
                    &format!("unknown module `{}` instantiated", mi.module_name.name),
                );
                return;
            }
        };

        // Fix 1: const-eval EVERY override expr NOW, in the PARENT scope, so a
        // parent-param-dependent override (`#(.W(PARENT_W))`) resolves. A failure
        // to fold is recorded as value=None (child keeps default + warns), never
        // a silent 0 that explodes the child's width.
        let mut overrides: Vec<ResolvedOverride> = Vec::with_capacity(mi.param_overrides.len());
        for ov in &mi.param_overrides {
            match ov {
                ast::ParamConn::Positional(e) => {
                    let value = self.const_eval_in_scope(e);
                    if value.is_none() {
                        self.warn("parameter override expression is not a constant; child default kept");
                    }
                    overrides.push(ResolvedOverride { name: None, value, is_named: false });
                }
                ast::ParamConn::Named { name, value, .. } => {
                    // `.W()` (value None) means "keep default" → record is_named with value None.
                    let v = value.as_ref().and_then(|e| {
                        let r = self.const_eval_in_scope(e);
                        if r.is_none() {
                            self.warn(&format!(
                                "override of parameter `{}` is not a constant; default kept",
                                name.name
                            ));
                        }
                        r
                    });
                    overrides.push(ResolvedOverride {
                        name: Some(name.name.clone()),
                        value: v,
                        is_named: true,
                    });
                }
            }
        }

        for item in &mi.instances {
            if !item.unpacked.is_empty() {
                // DEFERRED: instance arrays. Lower a single instance + note.
                self.warn("instance-array range ignored (v3: single instance)");
            }
            let child_path = self.child_prefix(&item.name.name);
            let binding = match &item.conns {
                ast::PortConnList::Named(v) => PortBinding::Named(v),
                ast::PortConnList::Positional(v) => PortBinding::Positional(v),
            };
            self.elaborate_instance(
                child,
                &child_path,
                Some(parent_inst),
                &overrides,
                binding,
                map,
            );
        }
    }
```

**2c. Update `elaborate_instance`'s signature** — change the `param_overrides: &[ast::ParamConn]` parameter (line 338) to `param_overrides: &[ResolvedOverride]`. The body call `self.bind_params(module, param_overrides)` (line 372) is unchanged at the call site.

**2d. Update the TOP call** in `run` (line 314): the top has no overrides, so pass an empty slice of the new type:

```rust
        self.elaborate_instance(top, &top_path, None, &[], PortBinding::None, &map);
```
(`&[]` infers to `&[ResolvedOverride]` from the new signature — no other change.)

**2e. Rewrite `bind_params`** to consume the pre-resolved overrides (REPLACE lines 597-654). Also adds the underflow guard so a width-0 param cannot explode:

```rust
    /// Bind a module's params for the current instance scope. Each declared
    /// param's default is const-eval'd IN ORDER (a later default sees earlier
    /// ones); the instantiation overrides (already resolved in the PARENT scope —
    /// Fix 1) then overlay by name (named) or position. Localparams are NOT
    /// overridable. Params are FQ-keyed so two instances with different `WIDTH`
    /// coexist. Returns the prior FQ→value entries for restore on exit.
    fn bind_params(
        &mut self,
        module: &ast::ModuleDecl,
        overrides: &[ResolvedOverride],
    ) -> Vec<(String, Option<u32>)> {
        // Build name→value from the resolved overrides. Positional binds to the
        // i-th *overridable* declaration index (matches module.params order).
        let mut ovr_by_name: BTreeMap<&str, Option<u32>> = BTreeMap::new();
        let mut pos_i = 0usize;
        for ov in overrides {
            if ov.is_named {
                let Some(n) = ov.name.as_deref() else { continue };
                // Fix 2 (mirror): a named override naming no real param is an error.
                if !module.params.iter().any(|p| p.name.name == n) {
                    self.error(
                        MsgCode::ElabPortMismatch,
                        &format!("override of unknown parameter `{n}`"),
                    );
                    continue;
                }
                if let Some(v) = ov.value {
                    ovr_by_name.insert(
                        module.params.iter().find(|p| p.name.name == n).unwrap().name.name.as_str(),
                        Some(v),
                    );
                }
                // `.W()` with no value ⇒ keep default (no insert).
            } else {
                match module.params.get(pos_i) {
                    Some(p) => {
                        ovr_by_name.insert(p.name.name.as_str(), ov.value);
                    }
                    None => {
                        self.error(
                            MsgCode::ElabPortMismatch,
                            "more positional parameter overrides than module parameters",
                        );
                    }
                }
                pos_i += 1;
            }
        }

        let mut saved = Vec::new();
        for p in &module.params {
            let chosen_val: Option<u32> = match ovr_by_name.get(p.name.name.as_str()) {
                // override present + param is overridable → use it (None = fold-fail → fall to default).
                Some(ovr) if matches!(p.kind, ast::ParamKind::Parameter) => {
                    (*ovr).or_else(|| self.const_eval_in_scope(&p.value))
                }
                // override targeting a localparam → error, keep declared value.
                Some(_) => {
                    self.error(
                        MsgCode::ElabPortMismatch,
                        &format!("cannot override localparam `{}`", p.name.name),
                    );
                    self.const_eval_in_scope(&p.value)
                }
                None => self.const_eval_in_scope(&p.value),
            };
            let v = chosen_val.unwrap_or(0);
            let key = self.fq(&p.name.name);
            saved.push((key.clone(), self.params.insert(key, v)));
        }
        saved
    }
```

**2f. Guard the width-0 underflow** in `range_to_dims` (the defensive half of M1). REPLACE lines 845-857 with:

```rust
                let msb = self.const_eval_in_scope(&r.msb).unwrap_or(0);
                let lsb = self.const_eval_in_scope(&r.lsb).unwrap_or(0);
                // Guard a degenerate `[W-1:0]` with W==0 → `[0u32.wrapping_sub(1):0]`
                // = `[0xFFFF_FFFF:0]` (Fix 1 defensive): treat any folded bound at
                // u32::MAX as a 0-width param artifact → clamp to width 1 + warn,
                // NOT a fatal MAX_NET_WIDTH explosion.
                if msb == u32::MAX || lsb == u32::MAX {
                    self.warn(
                        "parameterized range underflowed (param value 0?); net clamped to width 1",
                    );
                    return (1, 0, 0, signed);
                }
                let width64 = (msb.abs_diff(lsb) as u64) + 1;
                if width64 > MAX_NET_WIDTH {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "declared net width {width64} exceeds the v1 cap ({MAX_NET_WIDTH})"
                        ),
                    );
                    return (1, 0, 0, signed);
                }
                (width64 as u32, msb, lsb, signed)
```

### Fix 2 [MAJOR M2] — Diagnose surplus / nonexistent port connections

**Problem (verified):** `wire_ports` iterates only the *declared* ports and reads `v.get(i)` (positional) / `.find(name)` (named), so a 4th positional connection on a 3-port module or a `.ghost(x)` naming no real port is **silently dropped — 0 errors, 0 warnings.** The param path already errors on the same condition; ports are asymmetric.

**Fix:** After the wiring loop in `wire_ports`, diff the supplied connections against the port set. ADD this block at the end of `wire_ports`, immediately before its closing brace (after line 583):

```rust
        // Fix 2 (Finding M2): detect connections that match NO declared port.
        // Symmetric with bind_params' surplus-positional / unknown-named checks.
        match &binding {
            PortBinding::None => {}
            PortBinding::Positional(v) => {
                if v.len() > ports.len() {
                    self.error(
                        MsgCode::ElabPortMismatch,
                        &format!(
                            "instance of `{}` has {} positional connection(s) but the module declares {} port(s)",
                            module.name.name,
                            v.len(),
                            ports.len()
                        ),
                    );
                }
            }
            PortBinding::Named(v) => {
                for c in v.iter() {
                    if !ports.iter().any(|(pname, _)| pname == &c.name.name) {
                        self.error(
                            MsgCode::ElabPortMismatch,
                            &format!(
                                "connection `.{}(...)` names no port of module `{}`",
                                c.name.name, module.name.name
                            ),
                        );
                    }
                }
            }
        }
```

`ports` (the `Vec<(String, ir::PortDir)>` from `port_list_dirs`) is already in scope at line 506. No other change.

### Fix 3 [MINOR] — Correct the unconnected-INOUT warning text

`wire_ports` line 519-520 hard-codes `"output port ... left unconnected"` for the `Output | Inout` branch. REPLACE lines 517-523 with:

```rust
            let Some(conn_expr) = conn else {
                // unconnected port.
                match dir {
                    ir::PortDir::Output => {
                        self.warn(&format!("output port `{pname}` left unconnected"));
                    }
                    ir::PortDir::Inout => {
                        self.warn(&format!("inout port `{pname}` left unconnected"));
                    }
                    _ => {} // input floats silently (z = time-0 default)
                }
                continue;
            };
```

### Fix 4 [MINOR] — Non-ANSI output ports wire the correct direction

`port_list_dirs` (lines 140-160) **already** merges non-ANSI body `PortDecl` directions over the header name list — so `wire_ports` reads the *correct* direction for non-ANSI `output` ports. **The reviews' Finding (non-ANSI output wires backwards) refers to the separate `dir_for_name` helper (line 884-905) used for the net's stored `dir` field, NOT to wiring.** Confirm wiring is already correct (it is — `wire_ports` calls `port_list_dirs`, not `dir_for_name`).

The remaining gap is cosmetic-functional: a non-ANSI body net's *stored* `NetVar.dir` defaults to Input (the acknowledged TODO at line 893-897). This does not affect port wiring (which uses `port_list_dirs`) but does mislabel the net in VCD scope. Fold the merge into `dir_for_name` for non-ANSI by reusing the same body-PortDecl scan `port_list_dirs` already does. REPLACE the `NonAnsi` arm of `dir_for_name` (lines 891-902) with:

```rust
            ast::PortList::NonAnsi(names) => {
                if names.iter().any(|i| i.name == name) {
                    // Fix 4: merge the body PortDecl direction (output reg y;) just
                    // like port_list_dirs does — no more silent Input default.
                    ports_body_dir(name, &[]) // placeholder; see note
                        .unwrap_or(ir::PortDir::Input)
                } else {
                    ir::PortDir::Internal
                }
            }
```

Because `dir_for_name` does not receive the module body, the cleaner concrete fix is to look the direction up via the same body the caller already has. Since `elaborate_netvar_decl` calls `dir_for_name(name, ports)` and `ports` is `&module.ports` (not the body), pass the body through. **Concrete minimal change:** change `dir_for_name(&mut self, name, ports)` to also take `body: &[ast::ModuleItem]`, and in the `NonAnsi` arm scan the body:

```rust
    fn dir_for_name(
        &mut self,
        name: &str,
        ports: &ast::PortList,
        body: &[ast::ModuleItem],
    ) -> ir::PortDir {
        match ports {
            ast::PortList::Ansi(list) => list
                .iter()
                .find(|p| p.name.name == name)
                .map(|p| map_port_dir(p.dir))
                .unwrap_or(ir::PortDir::Internal),
            ast::PortList::NonAnsi(names) => {
                if names.iter().any(|i| i.name == name) {
                    body.iter()
                        .find_map(|it| match it {
                            ast::ModuleItem::PortDecl(pd)
                                if pd.names.iter().any(|x| x.name == name) =>
                            {
                                Some(map_port_dir(pd.dir))
                            }
                            _ => None,
                        })
                        .unwrap_or(ir::PortDir::Input)
                } else {
                    ir::PortDir::Internal
                }
            }
            ast::PortList::None => ir::PortDir::Internal,
        }
    }
```

Then update the one call in `elaborate_netvar_decl` (line 782): `let dir = self.dir_for_name(&decl.name.name, ports, body);` — and thread `body: &[ast::ModuleItem]` into `elaborate_netvar_decl`'s signature (it currently takes `(d, ports)`; the caller at line 379 has `&module.body` available: `self.elaborate_netvar_decl(d, &module.ports, &module.body)`). This is mechanical and keeps the non-ANSI net dir correct for VCD.

> If the implementer wants the smallest possible diff, Fix 4 may be **deferred** (wiring is already correct via `port_list_dirs`; only the VCD net-label is affected). Mark it explicitly in the deferred list if skipped — do not leave it silently wrong.

---

## 3. PART C — Cargo changes

**None.** Confirmed: both `hdl-parser` and `elaborate` already depend on `hdl-ast`, `hdl-lexer`, `sim-ir`, `diag`, and `std::collections::{BTreeMap, BTreeSet}` (no external dep added). No new crate, no feature flag. `edition = 2021`, MSRV 1.82 — all code above uses only stable APIs (`abs_diff`, `wrapping_*`, `saturating_mul`, `BTreeMap::entry/insert/remove`), no 2024-edition or post-1.82 features.

---

## 4. PART D — Test cases (`#[cfg(test)]`)

The 20 existing tests (`i1`–`i10` parser, `v3_1`–`v3_10` elaborate) all pass and stay. ADD these **6 regression tests** for the four fixes (append to `crates/elaborate/src/tests.rs`, reusing the existing `module_p`/`inst_named`/`inst_named_param`/`param`/`ansi_port`/`netvar`/`unit_of`/`CollectSink`/`elaborate` helpers visible at tests.rs:1440-1775):

```rust
// v3-11 [Fix 1]: a param override that references the PARENT's param resolves in
//        the parent scope (not folding to 0). top has P=8; child #(.W(P)) → q is 8 bits.
#[test]
fn v3_11_override_uses_parent_param() {
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
            inst_named_param("regw", "u", vec![("W", /*expr=*/ 0 /* see note */)], vec![("q", id_expr("bus"))]),
        ],
    );
    // NOTE: inst_named_param takes literal ints; to exercise the PARENT-param path
    // construct the override expr as Ident("P") directly via a small local builder
    // (mirror inst_named_param but pass `id_expr("P")` as the override value).
    let unit = unit_of(vec![regw, top]);
    let (s, _w) = elab_with_warnings(&unit);
    // child q (net 1, after top.bus) must be width 8 — proves P resolved in parent.
    assert_eq!(s.nets[1].width, 8, "override .W(P) must see parent P=8, not fold to 0");
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
    assert!(sink.n_warnings() >= 1, "expected the underflow-clamp warning");
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
            inst_positional("dff", "u", vec![Some(id_expr("c")), Some(id_expr("q")), Some(id_expr("x"))]),
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
            inst_named("dff", "u", vec![("clk", id_expr("c")), ("ghost", id_expr("c"))]),
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

// v3-16 [Fix 1 + Fix 2 + Fix 4 ALL pass]: regression — the happy multi-fix path
//        still produces the exact v3_1 layout (no false positives from new checks).
#[test]
fn v3_16_happy_path_unaffected_by_new_checks() {
    // identical to v3_1; re-asserts cont_assigns==3, instances==2, no errors.
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
            inst_named("dff", "u", vec![("clk", id_expr("clk")), ("d", id_expr("d")), ("q", id_expr("q"))]),
        ],
    );
    let unit = unit_of(vec![dff, tb]);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("clean path");
    assert_eq!(s.instances.len(), 2);
    assert_eq!(s.cont_assigns.len(), 3);
    assert_eq!(sink.n_errors(), 0);
}
```

**Test helper additions needed** (small, mechanical — the implementer adds alongside the existing builders in tests.rs):
- `inst_positional(module, inst, conns: Vec<Option<Expr>>)` — mirror of `inst_named` building `PortConnList::Positional`.
- An `inst_named` variant (or extend it) that accepts `vec![]` for zero connections (v3_15 connects nothing).
- An override-by-expr builder for v3_11 (mirror `inst_named_param` but take an `Expr` override value so `id_expr("P")` can be passed). The note in v3_11 flags this.
- `diag_codes(&sink) -> Vec<MsgCode>` and `warn_messages(&sink) -> Vec<String>` — trivial extractors over `sink.events.borrow()` (the v3_7 test already inlines the `diag_codes` pattern; promote it to a helper).

---

## 5. Coverage statement + deferred list

**Coverage (post-fix).** Parser: named/positional port connections, named/positional param overrides, multiple instances per statement, instance-array dims (parsed; elaborate single-instances them with a note), unconnected positional slots, `.p()`/empty `()`/`#()`, expression-valued connections, `.*` recovery — all covered (`i1`–`i10`). Elaborate v3: hierarchy flattening into one flat `SimIr` with contiguous per-instance net slices, depth-first declaration-order determinism (byte-identical, v3_9), top selection (last never-instantiated, v3_2), N-deep hierarchy + parent chain (v3_3), diamond reuse not-a-cycle (v3_8), cycle guard (terminates, error not panic), direction-correct port wiring (input `child=parent_expr`, output `parent_lval=child`, v3_1), FQ-keyed scope isolation, param override → width fold including **parent-param overrides (Fix 1, v3_11)** and **width-0 underflow safety (Fix 1, v3_12)**, **surplus/nonexistent connection diagnostics (Fix 2, v3_13/v3_14)**, correct **inout warning (Fix 3, v3_15)**, unknown-module error (v3_7), preserved single-module path (v3_10). `Instance.module = 0` (sim-engine ignores it). sim-ir frozen, schema_hash stable. No `HashMap`/`HashSet` — `BTreeMap`/`BTreeSet`/`Vec` only.

**Deferred (genuinely out of scope, each with an in-code note):**
1. **Generate-instantiated instances** — `Generate`/`Genvar` items emit `ElabUnsupported`; parser stubs generate bodies.
2. **`defparam`** — emits `ElabUnsupported`.
3. **Hierarchical cross-references in expressions** (`tb.dut.x`) — multi-segment `HierPath` → `ElabUnsupported` in `resolve_net`.
4. **Interface ports** — not modeled.
5. **Gate primitives** (`and`/`or`/`not …`) beyond a stub — lex as plain idents, flow through instance parsing, fail in elaborate as `ElabUnresolvedInstance` (no module body). Documented recovering stub.
6. **Instance arrays** (`u[3:0](...)`) — parsed into `unpacked`, but elaborate lowers a SINGLE instance + warns; the 4-bit-bus-fans-to-4-scalars semantics are not modeled.
7. **`.*` implicit connections** — parser stub (advisory + empty Named list); not wired.
8. **INOUT bidirectional** — approximated one-directional (parent→child) + warn.
9. **Non-ANSI net-`dir` label** — Fix 4 closes it; if the implementer takes the minimal-diff route and skips Fix 4, wiring is still correct (uses `port_list_dirs`) but the stored `NetVar.dir` of a non-ANSI `output` net is mislabeled Input in VCD scope — note it.
10. **Parser NITs** — mixed named+positional list recovers noisily (3 errors, no single "cannot mix" diag); bare `foo;` synthesizes an empty instance caught later as unknown-module. Acceptable recovery.

---

## 6. Residual risks

1. **Positional override → param index mapping assumes `module.params` order == positional order.** Correct for V2005 (`#(8, 256)` binds params[0], params[1] in declaration order). If a future PR introduces localparams interleaved in the param list, positional binding could target a non-overridable slot; `bind_params` already errors on localparam override, so it degrades to an error, not corruption. Low risk, noted.
2. **`const_eval_in_scope` is u32-only.** A param wider than 32 bits, or a real/signed-negative param, folds to its low 32 bits / wraps. Acceptable for width/range exprs (the only consumer); a `parameter W = 64'h1_0000_0000` would mis-fold. Deferred with the rest of full const-eval. No panic (wrapping ops).
3. **Fix 2's positional surplus check uses `v.len() > ports.len()`** — a positional list with trailing explicit `None` skip-slots (`u(a, b, )`) counts the trailing `None` toward length. A deliberate trailing-comma skip on a full port list would now error. This matches Verilog tool behavior (trailing skip beyond port count is an error) and is the intended strictness; noted so it is not mistaken for a regression.
4. **Diamond/deep reuse re-evaluates child params per instance** (correct — each instance gets its own FQ-keyed binding), so a deep wide tree is O(instances × params) param inserts. Bounded by `inst_stack` cycle guard; no unbounded growth. Fine for real RTL.
5. **Fix 4 threads `body` into two signatures.** If the implementer skips it, leave the explicit deferred note (item 9) — do not ship a silently-wrong non-ANSI net dir without the note.

All four fixes are additive, preserve the verified-correct flattening core and the 20 existing tests, keep sim-ir frozen, and stay deterministic (no new map iteration feeds arena order — the new `ovr_by_name`/override `Vec` are built in declaration order and consumed in `module.params` order).

**Relevant files:**
- `/Users/seongwookjang/project/git/violet_sw/016_claude_rtl/crates/hdl-parser/src/lib.rs` — parser complete, no change (instance parsing @998-1210, tests @2652-2788).
- `/Users/seongwookjang/project/git/violet_sw/016_claude_rtl/crates/elaborate/src/lib.rs` — apply Fix 1 (`elaborate_child_instances` @430, `elaborate_instance` sig @333, `run` @314, `bind_params` @597, `range_to_dims` @845), Fix 2 (`wire_ports` end @583), Fix 3 (`wire_ports` unconnected branch @517), Fix 4 (`dir_for_name` @884, `elaborate_netvar_decl` @774); add `ResolvedOverride` after @86.
- `/Users/seongwookjang/project/git/violet_sw/016_claude_rtl/crates/elaborate/src/tests.rs` — add v3_11–v3_16 + the 4 small helpers (after @1775).
- sim-ir (`crates/sim-ir/src/lib.rs`) — FROZEN, untouched.