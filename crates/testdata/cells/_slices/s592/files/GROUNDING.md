# §4.5.592 grounding — §2 🆕 V (generate-case arm re-decided per GenPhase walk)
status: completed

PRE (frozen, HEAD 75f46453, release): $S/s592/pre/vita md5 86a2d84a21d0329c57541e51ae9eb4d2
  staged ($S/s592/pre/sep, --features separate-bins): vita 4fbedadeac6515d0169d43ba1b6f526e vcmp 9d850c6c12a6a8d310c6c8f34daf8317 velab b5641c049f60f81a9ddc62a8979ed1d9 vrun 6b37a2a2764cc9ca2abf070a5366a67e
Harness: $S/s592/g/run.py (cells $S/s592/g/c/*.sv, raw output c/<id>.<tool>.txt; tools iv, sv (sv2v->iverilog), vl, pre)

## Oracle model (measured)
- iverilog 13.0 is LEXICAL: a parameter referenced before its declaration is refused ("Unable to bind parameter `K' in `top.gb'"), at module scope too (V01m, M1); with an outer same-named parameter it binds the OUTER one.
- sv2v 0.0.13 and verilator 5.052 are SCOPE-WIDE: a forward reference binds the scope's own later declaration (V01m, M1, N1, d1p ...).
- vita module scope: body params bound in decl order (M1 forward chain E3009 like iverilog) but every generate construct and net sees ALL module params (V01m `k 200 bits=8` = sv2v/vl).
- vita generate scope: a localparam is registered at its item position in EVERY phase walk, and the binding persists into the next phase. Nets-phase consumers before the declaration see outer/unbound; VarInit/Logic/Instances see the inner one left by the previous walk.


## Q1 census (code, HEAD 75f46453)
- Arm deciders: only `elaborate_gen_item` (generate.rs) — `GenItem::Case` (:432-480, i64 label compare, first match else default), `GenItem::If` (`elaborate_gen_if` :548-602, `const_truth_in_scope`), `GenItem::For` (:242-420, init/cond/step re-folded). Called by the 4 walks instance.rs:1066 (Nets), :1193 (VarInit), :1243 (Logic), :1381 (Instances); recursion generate.rs:352/530/535/730. Every other GenItem walker (toplevel.rs collect_instantiated, expr_size_hier.rs, cond_names.rs, gen_enum.rs, param_dup.rs, decl_collide.rs, block_local_class.rs, md_return.rs, package.rs, collect_gen_sva_decls) walks ALL arms structurally, decides nothing.
- Each walk re-decides; failure is reported only in Nets (`phase == GenPhase::Nets`), later walks return silently (scope.rs:124 documents the same class: "a nested generate fold in Nets and fail in Logic — silently deleting the whole generate body").
- Per phase from the decided arm: Nets = NetVar nets, procedural block-local nets (`hoist_block_local_nets`), generate functions/tasks (`register_gen_func/task` -> func_table, rtn_decl_scope, rtn_decl_genvars), errors for PortDecl/Defparam/Import; VarInit = variable initializers + per-scope flush; Logic = cont-assigns, net-init drivers, processes/tasks; Instances = child instances. EVERY phase: `Param` (localparam registration, generate.rs:735-865), carried `typedef enum` labels (LabelPass::Final in Nets, GenRepeat later), nested generates, gen_singleton_labels/gen_loop_labels, param_dup check, rank scope, genvar bind/restore.
- Later phases read from Nets' arm: nets by name (width, `$bits`, hier refs `gb.g.w`), registered functions (V19 calls default's f from k's process), carried enum types (V23). So a mix = nets/functions/enum types from the Nets arm + processes/initializers/instances from the later arm.
- Forward-reference mechanism: a generate-scope localparam is bound at its item position in every walk (key fq = prefix.name) and never unbound, so the Nets walk sees outer/unbound before the declaration while VarInit/Logic/Instances see the inner value left by the previous walk. Module scope differs: body params are bound before every net and generate walk (instance.rs:612-700), so a module-level forward label is consistent (V01m). A module-level generate REGION's localparam is registered positionally like a block's (V24 mixes).
- Same mechanism in GenItem::If (I2/I3 mix; I4 right by accident) and GenItem::For (F1 loud by accident E3010; F3 right by accident) and in non-construct consumers: a Nets-only net width vs a Logic read (N2 `K=8 bits=4`), a localparam initializer whose forward chain resolves one link per walk (M3: Nets scrut 7, Logic scrut 5).
- Chain evolution: values converge one link per walk (M3 probe: Nets/VarInit `scrut=7`, Logic/Instances `scrut=5`).

## Probe P1 (instrument only; records each Nets decision per (prefix, kind, span), compares in later walks)
- binary $S/s592/probe_p1/vita md5 7c60ac701d36b454afdd4d74e5fe60c2; patch $S/s592/probe_p1.patch
- workspace suite (`cargo nextest run --workspace --locked --no-fail-fast`, debug probe): `Summary [  51.933s] 9117 tests run: 9117 passed, 15 skipped`; 1088 DECIDE, 3260 CMPOK, 3 MISMATCH lines = ONE design: vita_gcwp_* t.gb[0] 35..355 (generate_case_and_wildcard_prerequisites.rs d1p pin) `nets=scrut=99 item2 now=scrut=99 item1`.
- corpus (11 workloads, real plusargs, all rc=0): 343 DECIDE (270 if, 73 for, 0 case), 1029 CMPOK, 0 MISMATCH. examples (4): 0 generate decisions.
- grounding cells: mismatching files = every forward-reference cell (V01 V03 V04 V04x V05c V05d V06 V06n V08 V08b V08c V09 V09b V10 V12a V12b V13 V14b V16-V24 V26 V28 V30 V31 I1-I4 F1-F3 M3); none in controls (V01m V02 V05a V05b V07 V12c V14 V15 V15b V25 V27 V28b V28c V29 V30b N1 N2 M1 M2).

## Q2 live cells (106 cells, $S/s592/g/c; PRE + two prototypes; raw output c/<id>.<tool>.txt)
Prototypes (built in gwt from 75f46453, release; patches in $S/s592):
- P4 = global pre-bind: on entry to each generate scope level in the Nets walk (and before the module's Nets generate loop for module-level REGION params), bind the level's own params (`gen_param_decls`) quietly to a fixpoint; at each param's own position restore the key's prior entries in all 9 maps (params, param_meta, param_range, hier_param_range, str_param_raw, real_param_val, hier_params, wide_param_bits, param_type_guessed) before the normal registration; plus a Nets decision record with a loud verify in later walks. vita md5 2bdc82586352e76a4a51e8c8ea2293ff, probe_p4.patch.
- P6 = decision view: no pre-bind of the walk, no cache. A decision input (case scrutinee + labels, if condition, for init/cond/step) containing no user call is evaluated, in EVERY walk, with every STRUCTURALLY EXACT parameter of every enclosing generate level (and the instance's module-level regions) bound at its key to a fixpoint, quietly, then every touched key is restored. Exact = literals, operators, selects, size/primitive/signing casts, `$signed`/`$unsigned`/`$clog2`, and bare names that are same-level exact params or bound genvars. bin md5 f6052d7ce91fc7e2a9d5632ee56204ce run with P6_DECIDE=1 P2_NOPREBIND=1 P5_NOCACHE=1 P2_NOVERIFY=1 (wrapper $S/s592/probe_p6/vita), probe_p6.patch.
- also measured: cache-only (Nets decision reused, no pre-bind, no verify; wrapper probe_cache): 4 correct→other (I4 `else` iv-only, M4 `seven P=5` none, V04 `one 1 bits=8` iv-only, V24s `k` none — each PRE = sv2v+verilator), d1p `def 9 bits=4` consistent-wrong, 23 silent→silent. The row's "decide once" hypothesis is falsified: caching moves PRE's later-walk decisions (right by accident where the arms' nets agree) to the Nets one.
- P2/P3 (P4 without the quiet range check / without fixpoint): E13 value→loud (`check_param_decl_range` in the pre-bind: `undefined name C` for a carried enum label in a range) — the binder must be diagnostic-free; M3/V24 loud without fixpoint / region pre-bind.

Oracle model: iverilog 13.0 lexical (refuses a forward parameter, binds an outer one), sv2v 0.0.13 and verilator 5.052 scope-wide; vita's module scope already follows sv2v/verilator on the shadow split (N2m `bits=8 w=255`, N2m2 `def`; iverilog `bits=4 w=15`, `four`).

| cell | iverilog | sv2v | verilator | PRE | P4 (global pre-bind) | P6 (decision view) | P4 class | P6 class |
|---|---|---|---|---|---|---|---|---|
| AE1_scr_fx | ERR:AE1_scr_fx.sv:9: error: Unable to evaluate param | ERR:AE1_scr_fx.sv2v.v:9: error: Unable to evaluate p | def K=xxxx | ERR:VITA-E3010:generate-case scrutinee is not a cons | zero K=0000 | ERR:VITA-E3010:generate-case scrutinee is not a cons | loud→value=none | same |
| AE1b_scr_fx_bwd | ERR:AE1b_scr_fx_bwd.sv:4: error: Unable to evaluate  | ERR:AE1b_scr_fx_bwd.sv2v.v:9: error: Unable to evalu | def K=xxxx | zero K=0000 | zero K=0000 | zero K=0000 | same | same |
| AE2_if_fx | ERR:AE2_if_fx.sv:6: error: Unable to evaluate parame | ERR:AE2_if_fx.sv2v.v:9: error: Unable to evaluate pa | then K=xxxx | ERR:VITA-E3010:generate-if condition is not a consta | then K=0000 | ERR:VITA-E3010:generate-if condition is not a consta | loud→value=none | same |
| AE3_lbl_fx | ERR:AE3_lbl_fx.sv:8: error: Unable to evaluate param | ERR:AE3_lbl_fx.sv2v.v:9: error: Unable to evaluate p | def 9 bits=4 | k 8 bits=4 | k 200 bits=8 | k 8 bits=4 | silent→silent | same |
| AE4_width_fx | ERR:AE4_width_fx.sv:4: error: Unable to bind paramet | bits=x K=x | ERR:%Error: AE4_width_fx.sv:4:12: left side of bit r | ERR:VITA-E3009:undefined name `K` is not allowed in  | bits=4 K=4 | ERR:VITA-E3009:undefined name `K` is not allowed in  | loud→value=none | same |
| AE5_alias_scr | ERR:AE5_alias_scr.sv:3: error: Unable to evaluate pa | ERR:AE5_alias_scr.sv2v.v:7: error: Unable to evaluat | def K=xxxx | ERR:VITA-E3010:generate-case scrutinee is not a cons | zero K=0000 | ERR:VITA-E3010:generate-case scrutinee is not a cons | loud→value=none | same |
| AE5b_alias_scr_bwd | ERR:AE5b_alias_scr_bwd.sv:3: error: Unable to evalua | ERR:AE5b_alias_scr_bwd.sv2v.v:7: error: Unable to ev | def K=xxxx | zero K=0000 | zero K=0000 | zero K=0000 | same | same |
| AE6_ovr_scr | ERR:AE6_ovr_scr.sv:12: error: Unable to evaluate par | ERR:AE6_ovr_scr.sv2v.v:25: error: Unable to evaluate | def K=xxxx | ERR:VITA-E3010:generate-case scrutinee is not a cons | zero K=0000 | ERR:VITA-E3010:generate-case scrutinee is not a cons | loud→value=none | same |
| AE7_lbl_difflbl | ERR:AE7_lbl_difflbl.sv:3: error: Unable to evaluate  | ERR:AE7_lbl_difflbl.sv2v.v:7: error: Unable to evalu | def 9 bits=4 | ERR:VITA-E3010:undeclared net/variable `top.gb[0].gk | k 200 bits=8 | ERR:VITA-E3010:undeclared net/variable `top.gb[0].gk | loud→value=none | same |
| AE8_param_fx_fwdarg | ERR:AE8_param_fx_fwdarg.sv:4: error: Unable to bind  | ERR:AE8_param_fx_fwdarg.sv2v.v:10: error: Unable to  | def P=xxxx | ERR:VITA-E3009:generate-scope parameter `P` value is | zero P=0000 | ERR:VITA-E3009:generate-scope parameter `P` value is | loud→value=none | same |
| AE9_scr_fx_fwdarg | ERR:AE9_scr_fx_fwdarg.sv:4: error: Unable to bind pa | ERR:AE9_scr_fx_fwdarg.sv2v.v:10: error: Cannot evalu | def | ERR:VITA-E3010:generate-case scrutinee is not a cons | zero | ERR:VITA-E3010:generate-case scrutinee is not a cons | loud→value=none | same |
| AE9b_scr_fx_bwdarg | ERR:AE9b_scr_fx_bwdarg.sv:5: error: Cannot evaluate  | ERR:AE9b_scr_fx_bwdarg.sv2v.v:10: error: Cannot eval | def | zero | zero | zero | same | same |
| B1_blklocal_fwd | bits=4 t=15 K=4 | bits=8 t=255 K=8 | bits=8 t=255 K=8 | bits=4 t=15 K=8 | bits=8 t=255 K=8 | bits=4 t=15 K=8 | silent→correct=sv+vl | same |
| B1n_blklocal_fwd | ERR:B1n_blklocal_fwd.sv:4: error: Unable to bind par | bits=8 t=255 K=8 | bits=8 t=255 K=8 | ERR:VITA-E3009:undefined name `K` is not allowed in  | bits=8 t=255 K=8 | ERR:VITA-E3009:undefined name `K` is not allowed in  | loud→value=sv+vl | same |
| C01_ctl_plain | L0 c0 V=10 / L0 else bits=4 w=0 / L1 cd V=20 / L1 th | L0 c0 V=10 / L0 else bits=4 w=0 / L1 cd V=20 / L1 th | L0 c0 V=10 / L0 else bits=4 w=0 / L1 cd V=20 / L1 th | L0 c0 V=10 / L0 else bits=4 w=0 / L1 cd V=20 / L1 th | L0 c0 V=10 / L0 else bits=4 w=0 / L1 cd V=20 / L1 th | L0 c0 V=10 / L0 else bits=4 w=0 / L1 cd V=20 / L1 th | same | same |
| C02_ctl_shadowbwd | eight bits=8 K=8 | eight bits=8 K=8 | eight bits=8 K=8 | eight bits=8 K=8 | eight bits=8 K=8 | eight bits=8 K=8 | same | same |
| E01_enumlbl_prebind | def 9 bits=4 P=18446744073709551617 | one 200 bits=8 P=1 | one 200 bits=8 P=1 | one 200 bits=8 P=1 | one 200 bits=8 P=1 | one 200 bits=8 P=1 | same | same |
| E02_bitsnet_prebind | eight 200 bits=8 P=8 | eight 200 bits=8 P=8 | eight 200 bits=8 P=8 | def 9 bits=4 P=4 | def 9 bits=4 P=4 | def 9 bits=4 P=4 | same | same |
| E03_cfn_prebind | six 200 bits=8 P=6 | six 200 bits=8 P=6 | six 200 bits=8 P=6 | six 200 bits=8 P=6 | six 200 bits=8 P=6 | six 200 bits=8 P=6 | same | same |
| E03b_cfn_err | P=6 | P=6 | P=6 | ERR:VITA-E3009:generate-scope parameter `P` value is | ERR:VITA-E3009:generate-scope parameter `P` value is | ERR:VITA-E3009:generate-scope parameter `P` value is | same | same |
| E04_badrange | ERR:E04_badrange.sv:3: error: Unable to bind paramet | ERR:E04_badrange.sv2v.v:4: error: Unable to bind par | ERR:%Error: E04_badrange.sv:3:23: Can't find definit | ERR:VITA-E3009:undefined name `Nope` is not allowed  | ERR:VITA-E3009:undefined name `Nope` is not allowed  | ERR:VITA-E3009:undefined name `Nope` is not allowed  | same | same |
| E06_genvar_prebind | L0 def 9 bits=4 / L1 two 200 bits=8 / L2 def 9 bits= | L0 def 9 bits=4 / L1 two 200 bits=8 / L2 def 9 bits= | L0 def 9 bits=4 / L1 two 200 bits=8 / L2 def 9 bits= | L0 def 9 bits=4 / L1 two 200 bits=8 / L2 def 9 bits= | L0 def 9 bits=4 / L1 two 200 bits=8 / L2 def 9 bits= | L0 def 9 bits=4 / L1 two 200 bits=8 / L2 def 9 bits= | same | same |
| E08_fwdbits_param | ERR:E08_fwdbits_param.sv:3: error: Unable to bind pa | eight 200 bits=8 | eight 200 bits=8 | ERR:VITA-E3010:generate-case scrutinee is not a cons | ERR:VITA-E3010:generate-case scrutinee is not a cons | ERR:VITA-E3010:generate-case scrutinee is not a cons | same | same |
| E09_import_sh | def 9 bits=4 | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | k 200 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| E12_realsh | P=3 | P=7 | P=7 | ERR:VITA-E3009:generate-scope parameter `P` value is | P=7 | ERR:VITA-E3009:generate-scope parameter `P` value is | loud→value=sv+vl | same |
| E12b_realsh_case | ERR:E12b_realsh_case.sv:5: error: Cannot evaluate ge | seven 200 bits=8 | seven 200 bits=8 | ERR:VITA-E3009:generate-scope parameter `P` value is | seven 200 bits=8 | ERR:VITA-E3009:generate-scope parameter `P` value is | loud→value=sv+vl | same |
| E13_enum_in_range | ERR:E13_enum_in_range.sv:4: error: Unable to bind pa | bits=3 P=5 | bits=3 P=5 | bits=3 P=5 | bits=3 P=5 | bits=3 P=5 | same | same |
| E14_enum_in_init | ERR:E14_enum_in_init.sv:4: error: Unable to bind par | three P=3 | three P=3 | three P=3 | three P=3 | three P=3 | same | same |
| F1_forfwd_sh | L0 w=10 | L0 w=10 / L1 w=11 / L2 w=12 | L0 w=10 / L1 w=11 / L2 w=12 | ERR:VITA-E3010:undeclared net/variable `top.gb[0].L[ | L0 w=10 / L1 w=11 / L2 w=12 | L0 w=10 / L1 w=11 / L2 w=12 | loud→value=sv+vl | loud→value=sv+vl |
| F2_forfwd | ERR:F2_forfwd.sv:3: error: Unable to bind parameter  | L0 w=10 / L1 w=11 / L2 w=12 | L0 w=10 / L1 w=11 / L2 w=12 | ERR:VITA-E3010:generate-for condition is not a const | L0 w=10 / L1 w=11 / L2 w=12 | L0 w=10 / L1 w=11 / L2 w=12 | loud→value=sv+vl | loud→value=sv+vl |
| F3_forfwd_sh_nonet | L0 | L0 / L1 / L2 | L0 / L1 / L2 | L0 / L1 / L2 | L0 / L1 / L2 | L0 / L1 / L2 | same | same |
| G1_enum_fwdparam | ERR:G1_enum_fwdparam.sv:3: error: Unable to bind par | bits=6 v=1 | bits=6 v=1 | ERR:VITA-E3009:undefined name `K` is not allowed in  | ERR:VITA-E3010:undeclared net/variable `top.gb[0].EB | ERR:VITA-E3009:undefined name `K` is not allowed in  | loud→loud | same |
| H1_genfn_fwd | f=4 | f=8 | f=8 | f=8 | f=8 | f=8 | same | same |
| I1_iffwd | ERR:I1_iffwd.sv:3: error: Unable to bind parameter ` | then 200 bits=8 | then 200 bits=8 | ERR:VITA-E3010:generate-if condition is not a consta | then 200 bits=8 | then 200 bits=8 | loud→value=sv+vl | loud→value=sv+vl |
| I2_iffwd_sh | else 9 bits=4 | then 200 bits=8 | then 200 bits=8 | then 8 bits=4 | then 200 bits=8 | then 200 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| I3_iffwd_sh_rev | then 200 bits=8 | else 9 bits=4 | else 9 bits=4 | else 9 bits=8 | else 9 bits=4 | else 9 bits=4 | silent→correct=sv+vl | silent→correct=sv+vl |
| I4_iffwd_sh_nonet | else | then | then | then | then | then | same | same |
| M1_modchain | ERR:M1_modchain.sv:2: error: Unable to bind paramete | A=3 B=3 | A=3 B=3 | ERR:VITA-E3009:parameter `A` value is not a constant | ERR:VITA-E3009:parameter `A` value is not a constant | ERR:VITA-E3009:parameter `A` value is not a constant | same | same |
| M2_genchain | ERR:M2_genchain.sv:3: error: Unable to bind paramete | A=3 B=3 | A=3 B=3 | ERR:VITA-E3009:generate-scope parameter `A` value is | A=3 B=3 | ERR:VITA-E3009:generate-scope parameter `A` value is | loud→value=sv+vl | same |
| M3_genchain_sh | ERR:M3_genchain_sh.sv:10: error: Unable to bind para | five 5 bits=6 P=5 | five 5 bits=6 P=5 | ERR:VITA-E3009:generate-scope parameter `Q` value is | five 5 bits=6 P=5 | ERR:VITA-E3009:generate-scope parameter `Q` value is | loud→value=sv+vl | same |
| M4_chain2sh | seven P=7 | five P=5 | five P=5 | five P=5 | five P=5 | five P=5 | same | same |
| M5_cycle | ERR:M5_cycle.sv:3: error: Recursive parameter refere | ERR:M5_cycle.sv2v.v:4: error: Recursive parameter re | ERR:%Error: M5_cycle.sv:4:16: Variable's initial val | ERR:VITA-E3009:generate-scope parameter `A` value is | ERR:VITA-E3009:generate-scope parameter `A` value is | ERR:VITA-E3009:generate-scope parameter `A` value is | same | same |
| N1_netfwd | ERR:N1_netfwd.sv:3: error: Unable to bind parameter  | K=8 bits=8 w=255 | K=8 bits=8 w=255 | ERR:VITA-E3009:undefined name `K` is not allowed in  | K=8 bits=8 w=255 | ERR:VITA-E3009:undefined name `K` is not allowed in  | loud→value=sv+vl | same |
| N2_netfwd_sh | K=4 bits=4 w=15 | K=8 bits=8 w=255 | K=8 bits=8 w=255 | K=8 bits=4 w=15 | K=8 bits=8 w=255 | K=8 bits=4 w=15 | silent→correct=sv+vl | same |
| N2m2_modscope_case | four | def | def | def | def | def | same | same |
| N2m_modscope_sh | bits=4 w=15 | bits=8 w=255 | bits=8 w=255 | bits=8 w=255 | bits=8 w=255 | bits=8 w=255 | same | same |
| N2p_netonly_sh | bits=4 w=15 | bits=8 w=255 | bits=8 w=255 | bits=4 w=15 | bits=8 w=255 | bits=4 w=15 | correct(iv)→sv+vl | same |
| P1c_procread_fwd | K=4 v=4 / top.gb.u P=4 | K=8 v=8 / top.gb.u P=8 | K=8 v=8 / top.gb.u P=8 | K=8 v=8 / top.gb.u P=8 | K=8 v=8 / top.gb.u P=8 | K=8 v=8 / top.gb.u P=8 | same | same |
| Q65_catalog | P=2 bits=3 | P=11 bits=12 | P=11 bits=12 | P=11 bits=3 | P=11 bits=12 | P=11 bits=3 | silent→correct=sv+vl | same |
| Q65b_nested_begin | N=1 | N=5 | N=5 | N=1 | N=1 | N=1 | same | same |
| S1_strsh | def 9 bits=4 | s 200 bits=8 | s 200 bits=8 | ERR:VITA-E3010:generate-case scrutinee is not a cons | ERR:VITA-E3010:generate-case scrutinee is not a cons | ERR:VITA-E3010:generate-case scrutinee is not a cons | same | same |
| V01_d1p | ERR:V01_d1p.sv:5: error: Unable to bind parameter `K | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | k 200 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V01m_modfwd | ERR:V01m_modfwd.sv:4: error: Unable to bind paramete | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | same | same |
| V02_back | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | same | same |
| V03_scrfwd | ERR:V03_scrfwd.sv:3: error: Unable to bind parameter | k 200 bits=8 | k 200 bits=8 | ERR:VITA-E3010:generate-case scrutinee is not a cons | k 200 bits=8 | k 200 bits=8 | loud→value=sv+vl | loud→value=sv+vl |
| V04_scrfwd_sh | one 1 bits=8 | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | same | same |
| V04x_scrfwd_sh | one 1 bits=6 | k 200 bits=8 | k 200 bits=8 | k 8 bits=6 | k 200 bits=8 | k 200 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V05a_r1 | def 9 bits=4 | k 200 bits=8 | k 200 bits=8 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | same | same |
| V05b_r1b | k 200 bits=8 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | same | same |
| V05c_r1p | k 200 bits=8 | def 9 bits=4 | def 9 bits=4 | def 9 bits=8 | def 9 bits=4 | def 9 bits=4 | silent→correct=sv+vl | silent→correct=sv+vl |
| V05d_r1q | def 9 bits=4 | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | k 200 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V06_C1 | def 9 bits=4 P=10000000000000005 | p 200 bits=8 P=00000000000000003 | p 200 bits=8 P=00000000000000003 | p 8 bits=4 P=10000000000000005 | p 200 bits=8 P=00000000000000003 | p 200 bits=8 P=10000000000000005 | silent→correct=sv+vl | silent→silent |
| V06n_C1narrow | def 9 bits=4 P=5 | p 200 bits=8 P=3 | p 200 bits=8 P=3 | p 8 bits=4 P=3 | p 200 bits=8 P=3 | p 200 bits=8 P=3 | silent→correct=sv+vl | silent→correct=sv+vl |
| V07_U1 | top.uA.a a / top.uB.a a | top.uA.a a / top.uB.a a | top.uA.a a / top.uB.a a | top.uA.d def / top.uB.d def | top.uA.d def / top.uB.d def | top.uA.d def / top.uB.d def | same | same |
| V08_nested | ERR:V08_nested.sv:5: error: Unable to bind parameter | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | k 200 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V08b_nestedcase | ERR:V08b_nestedcase.sv:6: error: Unable to bind para | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | k 200 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V08c_nestedouterfwd | ERR:V08c_nestedouterfwd.sv:3: error: Unable to bind  | k 200 bits=8 | k 200 bits=8 | ERR:VITA-E3010:generate-case scrutinee is not a cons | k 200 bits=8 | k 200 bits=8 | loud→value=sv+vl | loud→value=sv+vl |
| V09_forcase | ERR:V09_forcase.sv:4: error: Unable to bind paramete | L0 def 9 bits=4 / L1 k 201 bits=8 / L2 def 9 bits=4 | L0 def 9 bits=4 / L1 k 201 bits=8 / L2 def 9 bits=4 | L0 def 9 bits=4 / L1 k 9 bits=4 / L2 def 9 bits=4 | L0 def 9 bits=4 / L1 k 201 bits=8 / L2 def 9 bits=4 | L0 def 9 bits=4 / L1 k 201 bits=8 / L2 def 9 bits=4 | silent→correct=sv+vl | silent→correct=sv+vl |
| V09b_forcase_iter | ERR:V09b_forcase_iter.sv:4: error: Unable to bind pa | L0 def 9 bits=4 / L1 def 9 bits=4 / L2 k 202 bits=8 | L0 def 9 bits=4 / L1 def 9 bits=4 / L2 k 202 bits=8 | L0 def 9 bits=4 / L1 def 9 bits=4 / L2 k 10 bits=4 | L0 def 9 bits=4 / L1 def 9 bits=4 / L2 k 202 bits=8 | L0 def 9 bits=4 / L1 def 9 bits=4 / L2 k 202 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V10_iarr | ERR:V10_iarr.sv:4: error: Unable to bind parameter ` | top.u[0].gb.g k 200 bits=8 / top.u[1].gb.g k 200 bit | top.u[0].gb.g k 200 bits=8 / top.u[1].gb.g k 200 bit | top.u[0].gb.g k 8 bits=4 / top.u[1].gb.g k 8 bits=4 | top.u[0].gb.g k 200 bits=8 / top.u[1].gb.g k 200 bit | top.u[0].gb.g k 8 bits=4 / top.u[1].gb.g k 8 bits=4 | silent→correct=sv+vl | same |
| V10e_iarr_exact | ERR:V10e_iarr_exact.sv:4: error: Unable to bind para | top.u[0].gb.g k 200 bits=8 / top.u[1].gb.g k 200 bit | top.u[0].gb.g k 200 bits=8 / top.u[1].gb.g k 200 bit | top.u[0].gb.g k 8 bits=4 / top.u[1].gb.g k 8 bits=4  | top.u[0].gb.g k 200 bits=8 / top.u[1].gb.g k 200 bit | top.u[0].gb.g k 200 bits=8 / top.u[1].gb.g k 200 bit | silent→correct=sv+vl | silent→correct=sv+vl |
| V12a_ovr | ERR:V12a_ovr.sv:4: error: Unable to bind parameter ` | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 8 bits=4 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 8 bits=4 / top.uB.gb.g def 9 bits=4 | silent→correct=sv+vl | same |
| V12b_defparam | ERR:V12b_defparam.sv:4: error: Unable to bind parame | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 8 bits=4 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 8 bits=4 / top.uB.gb.g def 9 bits=4 | silent→correct=sv+vl | same |
| V12c_ovr_ctl | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | same | same |
| V12e_ovr_exact | ERR:V12e_ovr_exact.sv:4: error: Unable to bind param | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 8 bits=4 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | silent→correct=sv+vl | silent→correct=sv+vl |
| V12f_genvar_exact | ERR:V12f_genvar_exact.sv:3: error: Unable to bind pa | L0 then 200 bits=8 / L1 else 9 bits=4 / L2 else 9 bi | L0 then 200 bits=8 / L1 else 9 bits=4 / L2 else 9 bi | ERR:VITA-E3010:generate-if condition is not a consta | L0 then 200 bits=8 / L1 else 9 bits=4 / L2 else 9 bi | L0 then 200 bits=8 / L1 else 9 bits=4 / L2 else 9 bi | loud→value=sv+vl | loud→value=sv+vl |
| V13_multilab | ERR:V13_multilab.sv:4: error: Unable to bind paramet | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | k 200 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V14_fwd_nomatch | ERR:V14_fwd_nomatch.sv:4: error: Unable to bind para | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | same | same |
| V14b_fwd_nodef | ERR:V14b_fwd_nodef.sv:5: error: Unable to bind param | k 9 bits=4 | k 9 bits=4 | ERR:VITA-E3010:undeclared net/variable `top.gb[0].g2 | k 9 bits=4 | k 9 bits=4 | loud→value=sv+vl | loud→value=sv+vl |
| V15_laterblk | ERR:V15_laterblk.sv:4: error: A hierarchical referen | ERR:V15_laterblk.sv2v.v:5: error: A hierarchical ref | ERR:%Error: Internal Error: V15_laterblk.sv:4:10: .. | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | same | same |
| V15b_sibling | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | same | same |
| V16_hier | ERR:V16_hier.sv:4: error: Unable to bind parameter ` | h 200 bits=4 | h 200 bits=8 | h 8 bits=4 | h 200 bits=8 | h 200 bits=8 | silent→correct=vl | silent→correct=vl |
| V17_difflbl | ERR:V17_difflbl.sv:4: error: Unable to bind paramete | k 200 bits=8 | k 200 bits=8 | ERR:VITA-E3010:undeclared net/variable `top.gb[0].gk | k 200 bits=8 | k 200 bits=8 | loud→value=sv+vl | loud→value=sv+vl |
| V18_inst | ERR:V18_inst.sv:6: error: Unable to bind parameter ` | k 200 bits=8 / top.gb.g.u child-a | k 200 bits=8 / top.gb.g.u child-a | k 8 bits=4 / top.gb.g.u child-a | k 200 bits=8 / top.gb.g.u child-a | k 200 bits=8 / top.gb.g.u child-a | silent→correct=sv+vl | silent→correct=sv+vl |
| V19_func | ERR:V19_func.sv:4: error: Unable to bind parameter ` | k f=101 | k f=101 | k f=8 | k f=101 | k f=101 | silent→correct=sv+vl | silent→correct=sv+vl |
| V20_armlp | ERR:V20_armlp.sv:4: error: Unable to bind parameter  | k 200 bits=8 W=8 | k 200 bits=8 W=8 | k 8 bits=4 W=8 | k 200 bits=8 W=8 | k 200 bits=8 W=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V21_armfor | ERR:V21_armfor.sv:4: error: Unable to bind parameter | k j0 200 bits=8 / k j1 201 bits=8 | k j0 200 bits=8 / k j1 201 bits=8 | k j0 8 bits=4 / k j1 9 bits=4 | k j0 200 bits=8 / k j1 201 bits=8 | k j0 200 bits=8 / k j1 201 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V22_armvar | ERR:V22_armvar.sv:4: error: Unable to bind parameter | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | k 200 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V23_armenum | ERR:V23_armenum.sv:4: error: Unable to bind paramete | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | k 200 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V24_region | ERR:V24_region.sv:3: error: Unable to bind parameter | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | k 200 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V24b_region_net | ERR:V24b_region_net.sv:2: error: Unable to bind para | bits=8 K=8 | bits=8 K=8 | ERR:VITA-E3009:undefined name `K` is not allowed in  | ERR:VITA-E3009:undefined name `K` is not allowed in  | ERR:VITA-E3009:undefined name `K` is not allowed in  | same | same |
| V24c_region_nested | ERR:V24c_region_nested.sv:4: error: Unable to bind p | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | k 200 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V24s_region_import | ERR:V24s_region_import.sv:9: error: 'K' has already  | def | def | def | def | def | same | same |
| V25_iface | ERR:V25_iface.sv:4: error: Unable to bind parameter  | i 200 bits=4 | i 200 bits=8 | ERR:VITA-E3009:generate blocks inside an interface a | ERR:VITA-E3009:generate blocks inside an interface a | ERR:VITA-E3009:generate blocks inside an interface a | same | same |
| V26_xlabel | ERR:V26_xlabel.sv:5: error: Unable to bind parameter | k 200 bits=8 | ERR:%Error: V26_xlabel.sv:4:7: Use of x/? constant i | k 8 bits=4 | k 200 bits=8 | k 200 bits=8 | silent→correct=sv | silent→correct=sv |
| V27_enumfwd | ERR:V27_enumfwd.sv:4: error: Unable to bind paramete | eb 200 bits=8 | eb 200 bits=8 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | same | same |
| V28_bitsfwd | def 9 bits=4 | k8 200 bits=8 | k8 200 bits=8 | ERR:VITA-E3010:generate-case scrutinee is not a cons | ERR:VITA-E3010:generate-case scrutinee is not a cons | ERR:VITA-E3010:generate-case scrutinee is not a cons | same | same |
| V28b_bitsfwd_sh | def 9 bits=4 | k8 200 bits=8 | k8 200 bits=8 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | same | same |
| V28c_ifbits_sh | else 9 bits=4 | then 200 bits=8 | then 200 bits=8 | else 9 bits=4 | else 9 bits=4 | else 9 bits=4 | same | same |
| V29_genfn | f 200 bits=8 | f 200 bits=8 | ERR:%Error: V29_genfn.sv:8:32: Constant function may | f 200 bits=8 | f 200 bits=8 | f 200 bits=8 | same | same |
| V30_strfwd | ERR:V30_strfwd.sv:4: error: Unable to bind parameter | s 200 bits=8 | s 200 bits=8 | s 8 bits=4 | s 200 bits=8 | s 200 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V30b_realfwd | ERR:V30b_realfwd.sv:4: error: Unable to bind paramet | ERR:V30b_realfwd.sv2v.v:5: error: Cannot evaluate ge | r 200 bits=8 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | same | same |
| V31_widefwd | ERR:V31_widefwd.sv:4: error: Unable to bind paramete | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | k 200 bits=8 | silent→correct=sv+vl | silent→correct=sv+vl |
| V32_typefwd | def 9 bits=4 | t 200 bits=8 | ERR:%Error: V32_typefwd.sv:4:13: Reference to 'T' be | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | same | same |
| W1_wide_sh | def 9 bits=4 K=10000000000000005 | k 200 bits=8 K=05 | k 200 bits=8 K=05 | k 8 bits=4 K=05 | k 200 bits=8 K=05 | k 200 bits=8 K=05 | silent→correct=sv+vl | silent→correct=sv+vl |
| W2_narrow_over_wide_fwd | K=05 | K=10000000000000007 | K=10000000000000007 | K=10000000000000007 | K=10000000000000007 | K=10000000000000007 | same | same |

P4 tally: {'loud→value': 22, 'same': 46, 'silent→silent': 1, 'silent→correct': 35, 'loud→loud': 1, 'correct': 1}
P6 tally: {'same': 69, 'silent→correct': 28, 'loud→value': 8, 'silent→silent': 1}


Moved-cell counts (106 cells):
- P6: 28 silent→correct (26 = sv2v+verilator; V16 = verilator, sv2v's `$bits(gb.g.w)` says 4 beside its own value 200; V26 = sv2v, verilator refuses an x label), 8 loud→value (F1 F2 I1 V03 V08c V12f V14b V17, each = sv2v+verilator; iverilog refuses the forward name, F1 iverilog lexical `L0 w=10`), 1 silent→silent (V06_C1: arm now = oracles `p 200 bits=8`, the `P=10000000000000005` token byte-identical to PRE = 🆕 U-b stale wide entry), 0 value→loud, 0 correct→anything, 69 same.
- P4: 35 silent→correct, 22 loud→value (14 = sv2v+verilator; 8 = 🆕 AE descents AE1 AE2 AE4 AE5 AE6 AE7 AE8 AE9: PRE loud, P4 `zero K=0000` / `then K=0000` / `bits=4 K=4` / `k 200 bits=8`, verilator `def K=xxxx` / `def 9 bits=4`, iverilog and sv2v "Unable to evaluate parameter"), 1 silent→silent (AE3), 1 split move (N2p iverilog→sv2v+verilator, = vita module policy N2m), 1 loud→loud (G1 text).
- Workspace suite (`cargo nextest run --workspace --locked --no-fail-fast`, debug, env-gated modes): P4 and P6 each `9117 tests run: 9116 passed, 1 failed, 15 skipped`; the failure is the d1p KNOWN-WRONG pin `generate_case_and_wildcard_prerequisites p2_a_forward_referenced_matching_label_mixes_arms`, new stdout `D1P k 200 bits=8` (= oracles). P6 census over the suite: the view fired with keys 84 times, changed a decision once (vita_gcwp = d1p).
- Corpus (11) + examples (4), `vita vcmp` + `vita velab`: .vu and .velab 30/30 byte-identical PRE vs P6 and PRE vs P4.
- Staged: P6 `vcmp → velab → vrun` == one-shot on 103/103 cells.

## Q3 lane table (proposed cut = P6)
| shared function / lane | consumer lanes | status |
|---|---|---|
| decision view around `GenItem::Case` scrutinee+labels | label, scrutinee, per prefix/iteration/instance | measured: V01 V03-V06n V08-V10e V12a-f V13-V31 W1 AE1 AE3 AE5-AE9 |
| decision view around `elaborate_gen_if` | if condition, else-if chain | measured: I1-I4 V28c AE2 C01 V12f |
| decision view around `GenItem::For` init/cond/step | iteration count | measured: F1-F3 E06 V09 V21 C01 |
| level stack: generate scope levels + module-level regions | nested levels, regions | measured: V08 V08b V08c V21 V24 V24b V24c V24s |
| quiet Param binder (factored `(_, Param)` arm, no `check_param_decl_range`, no unfoldable report) | every param shape the arm takes: int, real, string, wide, typed/untyped, genvar-derived | measured: E01 E02 E03 E04 E06 E12 E13 E14 V30 V30b V31 W1 W2 |
| take/restore of 9 param maps at a key | wide/narrow/string/real/guessed entries | measured: C1 W1 W2 E12 V30 V31 (state after the view = PRE: suite + corpus identical) |
| exactness + call predicates (structural) | AE channels: direct call, alias, override, call over a forward arg, label in a loud-by-accident design | measured: AE1-AE9 all = PRE |
| Nets-only consumers (net/block-local widths, enum ranges), param chains, non-exact forward params | opted out (P6 never binds them for these) | measured = PRE: N1 N2 N2p B1 B1n Q65 G1 M1-M5 E12 E12b V10 V12a V12b |
| staged vcmp/velab/vrun | | measured 103/103 |
| corpus, examples, suite | | measured (above) |
| interface generate | refused before the walk (V25 E3009) | measured = PRE |
| elaboration time | for-loop conds re-run the view per iteration | NOT measured as A/B (unoptimized P6 suite 68.2 s vs PRE-tree probe 51.9 s); implementation must compute exact sets once per level entry and skip the view when no exact key is in scope, then A/B elab time on the corpus (release, both orders) |
Zero unmeasured correctness lanes for P6; one performance obligation.

Byte-identity (designs with no forward reference): the view binds only keys of exact params of enclosing levels; when every exact param a decision input reads is declared before the construct (and its own exact dependencies too), its fixpoint value equals its positional value (same literals, genvars, same-level exact params), so the decision is PRE's; the view then restores every touched key in all 9 maps and binds with no diagnostic, no dedup-set write (`reported_bad_bounds` untouched), no IR constant interned (the Param arm's folds call no `intern_const`). Measured non-vacuity: 84 firings with keys in the suite, 1 decision changed (d1p); P1: 1088 + 343 decisions all phase-consistent except d1p.

## Q4 fix shape and movers
- Which walk has the full information? None positionally: VarInit/Logic/Instances see the previous walk's bindings, and a forward chain converges one link per walk (M3 probe: Nets/VarInit `scrut=7`, Logic/Instances `scrut=5`). The full information is the declarations themselves, available before any walk; so every walk decides from the declarations (fixpoint of exact params), nothing defers, nothing is cached.
- Key question: P6 has no key. For the record/verify variant the key (cur_prefix, span) is never compared across units: a prefix belongs to one instance of one module definition (one CU after the staged merge, cli/src/staged.rs:166-200), entries are written and read inside one elaboration; V12a/V12e (uA/uB different arms, same span) and V10e (instance array) are the U1-class cells and pass on P6.
- Movers: see Q2 counts (P6: silent→correct 28, loud→value 8, correct→anything 0).
- Memories applied: "decide a landing after the fixpoint converges" (decide from the fixpoint, not mid-walk), "record at creation, verify at resolution" (record+verify measured: verify-loud would turn right-by-accident cells loud; record only exact facts), "generate name walk is not a scope model" (decide from a per-level declaration census, ER §2.5 row), "layout keys must outlive the unit" (no cross-unit key).

## Q5 interaction
- 🆕 T (d4dc9c26): T's bit-domain compare distinguished outer vs inner forward labels that PRE's i64 compare happened to reject in both walks (r1 V05a, r1b V05b), so arms mixed (round-2 F-B). After P6, every exact forward label resolves alike in every walk, so T's chooser gets one input per construct instance and `gen_case_region` (9bd5cd65) is unnecessary for exact labels. A label reading a NON-exact forward param (V10, V12a, V12b) stays positional; T must not decide such a label differently per walk (keep its first-elaboration guard for it) until the residue row lands.
- 🆕 U held half (U-b): C1's remaining wrong token is U-b's class (a wide→narrow rebinding at one key across walks leaves the wide entry `bare_ident_route` reads first). P4 avoided it only by binding P narrow in Nets; P6 leaves PRE's token. P6 routes nothing new through U-b's writers (genvar setup, enum labels, imports).
- 🆕 AE: every pre-bind that reaches a PRE-loud consumer leaks the interpreter's 0 (AE1, AE2, AE4-AE9 on P4). P6's exactness + call predicates contain it (AE1-AE9 = PRE).

## Q6 §2 / PROBE_CATALOG by code site
- generate.rs `GenItem::Case`: ROADMAP §2 🆕 T (label i64 sink), 🆕 V, 🆕 AC (`generate.rs:459` label that does not fold = no match); PROBE_CATALOG §4.5.568 label skip (= 🆕 T).
- positional generate binding (generate.rs Param arm, scope.rs walk): PROBE_CATALOG §4.5.565 CLASS (Q65 = its `P=11 bits=3` cell: PRE mix, P6 unchanged (Nets-only width), P4 = sv2v+verilator), §4.5.568 CLASS (outer past a block declaration; genvar wide = U-b), §4.5.570 (parser `const_locals` struct bound, split).
- instance.rs walks: no row. scope.rs:124 doc (shadow-aware walk precondition): untouched.
- New pre-existing, outside the fix path (PROBE_CATALOG candidates): E02 `$bits(v)` in a block localparam reads the OUTER `wire [3:0] v` though the block declares `wire [7:0] v` before it (vita `def 9 bits=4 P=4`, all three `eight 200 bits=8 P=8`); V15 a hierarchical label into a later block `gb.K` is accepted and takes `default` (iverilog "A hierarchical reference (`gb.K') is not allowed", verilator internal error); V27 a forward generate enum label takes `default` (sv2v, verilator `eb`; the 🆕 T/AC sink); M1 a module-scope forward parameter chain is E3009 (sv2v, verilator `A=3 B=3`).

## Verdict
- Start condition for the P6 cut: MET (zero unmeasured correctness lanes; performance A/B owed at implementation).
- Full shape (global pre-bind P4: closes Nets-only widths, chains, non-exact forward params): NOT MET — BLOCKED on 🆕 AE (alias/override/call channels carry the interpreter's 0 into PRE-loud consumers: AE5 alias, AE6 override, AE7 label, AE8/AE9 call over a forward arg).
- Prerequisite / residue rows: (1) "a forward generate parameter read by a Nets-only consumer (net or block-local width, Q65), a forward parameter chain (M2, M3, E12), or a non-exact forward parameter in a decision input (V10, V12a, V12b, AE3) binds by position; the global pre-bind (P4) closes them; BLOCKED on 🆕 AE"; (2) C1's `P` token with 🆕 U-b.
