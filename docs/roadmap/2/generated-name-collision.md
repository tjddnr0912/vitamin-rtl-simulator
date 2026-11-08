(§2 generated-name-collision · OPEN · in Scoping / imports / block-locals) (a generated name a user identifier can spell collides with it):
* symptom: PRE spells generated names in the user identifier space, so a legal design that declares the same simple identifier is refused at exit 1, on 11 of 20 generated-name families (W/qsymmix/replan3/r3/REPORT.txt, census on cc4d0257)
* witness: j6_tpw_1 (a user `T$w` beside type parameter T's carrier) PRE vita-main 8c970e9b rc 1 `` duplicate declaration of type parameter `T`: this `localparam` repeats the `parameter` ... `` and a note
  * oracles: iverilog = sv2v → iverilog = verilator `A bv=4 u=9`
* witness: j6_taskarg_1 PRE rc 1 `` net/variable `top.__taskarg_t_x_2` redeclared (duplicate declaration) ``
  * oracles: iverilog = sv2v → iverilog = verilator `A y=5 u=77`
* witness: j6_udp_1 PRE rc 1 `` net/variable `top.g.__p_udp_q_0` redeclared `` and an E3018 line (W/qsymmix/premise5/cells, ORACLES6.txt)
  * oracles: iverilog = verilator `A q=1`, sv2v a parse error on `primitive`
* census: the 11 colliding families: tpw (`hdl-parser/src/type_params.rs:168`), tpd (`:378`), tpp (`type_param_packed.rs:60`), udp (`udp.rs:481`)
* and in elaborate: sad (`string_array_route.rs:229`), outp (`frames_call/emit.rs:628`), sva (`sva_check.rs:13`, `sva_prop.rs:956`, `cover.rs:96`), clk (`sva_clocking.rs:599`), taskarg (`inline_task.rs:416`), rand (`crv.rs:775`), genblk (`generate.rs:601`, `:657`)
* census: 4 families do not collide (hout, forvar, foreach, grtn), and 5 are not reachable (enum stems and `$__vita_elab_` start with `$`, `__dynsnap_` has no caller, `cp_` / `cross_` are manifest labels)
* note: families filed alone before: §3.b carrier-namespace (tpw, tpd, tpp), §2 🆕 N's `genblk1` collisions, and §2 23, two clocking blocks whose mangled names collide with each other
* note: a spelling no identifier can take exists in the parser: the anonymous member key holds a space (`hdl-parser/src/structs/mod.rs:104-112`)
* note: two paths or more make a CLASS (D8), so it is filed as one entry, and the structure review (D14) decides whether it becomes a structural row: one funnel for generated names whose spelling no identifier can take (ER §5.1)
* note: the owner's decision O1 presented it, not decided (W/qsymmix/replan3/PLAN.txt OWNER DECISIONS)
* status: OPEN
