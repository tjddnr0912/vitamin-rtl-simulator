# lens:differential round 1 — §4.5.587 unique-const-fn (shape b)
status: IN PROGRESS
binaries: PRE pre/sep/vita d0e1cf9d (pre/vita e1e7e571) · POST post_b/vita cad7fe9a · vita_jit f5f5cb52

## Q1 census row 1-2 (444 cells, my rerun into r1/diff/c444, PRE=pre/sep/vita)
cells=444 identical=396 (text + run.json minus timing) text_diff=48 json_only=0; moved set == p3_item1_444.txt (extra 0, missing 0)
42/48 moved cells with oracle files on disk: POST value == verilator value in all 42; 0 moved cell gained an E/W code.
6 g2 cells (c_cast, c_elabt, c_rep, c_rep2, c_rep3, c_strm) have no oracle file in g2/ -> re-run with oracles in Q3.
h_plsb: POST pl=1e (= vl) but keeps a vita-only W4031 (PRE also prints it) -> pre-existing.
## Q2 census row 3 (8 run-time replace cells)
c_rpt, e_trpt, e_ktrpt, e_ktrp2, e_frpt, h_tdly, h_cad, h_wd (if/case x M/N): all byte-identical PRE==POST; W4031 count/time/location unchanged.
## Q3 attack designs batch 1 (r1/diff/atk/{u,p}; u = unique/priority form, p = plain-if twin; PRE/POST/iverilog/verilator)
F1 (BLOCKING, NEW for the u form; root cause pre-existing) d01_ia_hdr_shadow, d02_ia_port_shadow: instance array whose child
 header default / port range calls the child's `cf` (unique-if miss) while the parent has a same-named `cf` without a miss.
 u.PRE E3009 -> u.POST silent `top.u[1] ... p=5` ; iverilog(p) + verilator(u,p): `top.u[1] ... p=a`. Plain twin PRE==POST p=5 (pre-existing
 ia_shadow silent-wrong when the bus width equals the prepass's wrong element width -> replicate instead of slice).
F2 (BLOCKING?, NEW for the u form; root cause pre-existing) d10_xret: function assigning its 4-state return only on the matching path.
 u.PRE E3009 -> u.POST `PX=0000 PI=0`; verilator(u,p) `PX=xxxx PI=x`; iverilog(p) refuses (Unable to evaluate parameter); plain twin PRE==POST 0000/0.
 PS (initialized) -3 bs=6 = all tools.
d05/d05b: u.POST E3009 "package-scoped call `pk::f(...)` needs a body that references only its own formals/locals" at the run-time
 repeat call (PRE d05b same message) -> honest-loud, pre-existing (verilator n=7 + 1 assertion).
## Q3b batch 2 (r1/diff/atk2)
e01_ovr_shapes (real/typed/untyped/wide/string override + defparam, override calls parent's miss fn): PRE==POST byte-identical (E3009 x N) -> override channel untouched.
e02_rt_misc: `s = {f(2){"ab"}}` (string replication, run-time stmt): PRE `s=` (silent-wrong) -> POST `s=abab` = verilator = iverilog(p); W4031 set
 unchanged (2@0 for the two `+:` offsets, 1@1 for $info arg) = verilator's 3 assertions. NEW unplanned silent-wrong->correct move outside P3 (plan says
 string replication callers are not opted in) -> NON-BLOCKING note.
e03_ia_override (instance array, overridden header W, child gen-if on miss fn): u.PRE E3010 -> u.POST = iverilog = verilator (u[1] p=a, v[*]).
e04_ifc: masked by "interface instance arrays are outside the MVP" (PRE p twin same) -> no data.
## Q4 census row 5 (.velab, my rerun of pre/sep + post_b vcmp/velab on 444)
nonmoved: 256 same, 0 differ, 140 no pair (both loud); moved: 10 differ (silent-wrong->correct), 38 POST-only (loud->value).
header bytes 56454c4142000000 23 on PRE and POST (format_version 35). CONFIRMED.
batch 1 d19-d42 + z_ (6 g2 census cells with oracles):
 moved and == both oracles (or the one that runs): d04 d08 d09 d13 d14(default) d19 d21 d24 d26 d27(twins: $bits/$left/$right/$size/$increment/$dimensions/hier) d30 d31 d34 d40 d03 d06(W4031x1 = vl ASSERTx1)
  z_c_cast cw=7, z_c_rep r=7f, z_c_rep2 r=7f, z_c_rep3 r=1555 (= ivl(p) = vl); z_c_elabt I3006 el
 loud->loud or loud both, u.POST == p.POST class (no silent): d22 (Q=fl(30000) step limit, u==p; vl also refuses), d33 (e.name unregistered, p same), d38 (E3018 int w, p same), d41 (override of W stays E3009 after P folds: no mode leak), d42 (pkg param cross-pkg)
 pre-existing gaps PRE==POST in u and p: d07 ($bits(logic[..]) E2002 parse), d15 (class static fn E2002), d16 (let E3009), d17 (type param default E2002), d18 ($bits(T) E3010), d23 (4'bx arg E3009)
 d29 (3-D lvalue m3[1][0][f3(2):0]): u PRE==POST E3009 "nested lvalue select (v1: single-level)"; p twin folds m3=fff05fff x=0f (= ivl, vl) -> honest-loud residue, pre-existing
 d20/d25: `unique if .. else if` chain inside a function: PRE==POST already fold (P=5; P=2) and the run-time `repeat (fi2(5))` prints 0 W4031 where verilator prints 3 -> pre-existing silent report drop (unique-if-chain residue), not this slice
## Q5 census row 4 (lanes) — my rerun (r1/diff/lanes.py)
48 moved census cells (m48/lanes.txt) + 46 attack designs (atk_lanes.txt): native == interp == vm == JIT(VITA_JIT=1) == staged vcmp/velab/vrun
 on normalized output in 94/94; JIT activations>0 in 8/48 census cells. The 27 held/opt-out cells of p4 were not re-run.
## Q6 G6 re-run (r1/diff/g6r, 71 lanes): POST(u) == PRE(p) in 68; differ g12f (u stays E3009, p renders `s=   A`), g13a/g13b (+1 W4031 @1 = verilator).
 0 plain-twin cell changed PRE->POST.
## Q7 systematic (680 PRE/POST pairs: 444 census + 92 attack u/p + 2 -G + 142 G6)
moved 143; W4031 dropped 0; gained 4 (d06, d30, g13a, g13b: each +1 at the time verilator asserts); same count different site/time 0;
rc!=0 -> rc=0: 103; rc=0 -> rc!=0: 1 (g10a negative count, = both oracles error).
POST rc=0 movers vs oracle (sorted value lines): 119 match, 4 mismatch = F1 d01, F1 d02, F2 d10, e02 (harness-format: $info rendering only).
Plain twins already silent-wrong in PRE (oracle text available): exactly 3 (d01, d02, d10) and in all 3 the u form went rc=1 -> rc=0 with
 u.POST == p.PRE byte-value -> the slice makes the unique form inherit a pre-existing silent-wrong; no other inheritance in the measured set.
## Findings (most severe first)
F1 BLOCKING (NEW loud->silent-wrong for the u form; root cause pre-existing ia_shadow) atk/u/d01_ia_hdr_shadow, atk/u/d02_ia_port_shadow
F2 BLOCKING (NEW loud->silent-wrong for the u form; root cause pre-existing const_fn.rs:1511 `env.entry(name).or_insert(0)`) atk/u/d10_xret;
 single value oracle (verilator); iverilog refuses the parameter; IEEE 1800-2017 4-state default is X (recalled)
N1 NON-BLOCKING pre-existing: d25 `unique if .. else if` chain in a function, run-time `repeat (fi2(5))`: vita 0 W4031, verilator 3 (PRE==POST)
N2 NON-BLOCKING pre-existing honest-loud: d05/d05b package fn with unique if called at run time -> E3009 package-scoped call ...
N3 NON-BLOCKING pre-existing honest-loud: d29 3-D lvalue m3[1][0][f3(2):0] u form E3009 nested lvalue select; p folds
N4 NON-BLOCKING NEW benign: e02 string replication `{f(2){"ab"}}` PRE `s=` -> POST `s=abab` (= oracles), not in P3 / plan opt-out says string callers untouched
N5 NIT pre-existing: W4031 text for a `unique if` says "unhandled for priority or unique case statement"
status: COMPLETE
