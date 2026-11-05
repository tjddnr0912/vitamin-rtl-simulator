(§5.2 Q-PCTM · rank ① · source ENG-2 r5 lens-diff D-N6) (`%m` after a suspension in an automatic task names another instance):
* symptom: `%m` after a suspension inside an automatic task names another instance of that task's scope: two instances of `m` with `task automatic t(); #(1+2*P) $display("%m P=%0d", P);` (GM2_inst_delay_stagger) print `T t=2 top.u1.t P=0 \| T t=4 top.u0.t P=1` on every vita binary, where iv = s2v = vl `T t=2 top.u0.t P=0 \| T t=4 top.u1.t P=1`
* witness: the same in a generate-for block (GM1 GM4, GM5 after its delay; before it right: `B t=1 top.g[0].t gi=0`) and in a frame body's `@(*)` (N21_genvar_task: `W top.g[1].t t=5 y=3`, iv `y=4`)
* symptom: a static task declared in a generate block prints `top.g[0]$t` (G1c_gen_task_static, GM3_gen_static_delay), where iv = s2v `top.g[0].t`
* status: pre-existing (PRE = POST, native vm interp)
* IEEE 1800-2017 §21.2.1.6 :28900-28902 (the name of the subroutine that invokes the system task)
