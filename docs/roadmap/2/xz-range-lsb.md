(§2 xz-range-lsb · OPEN · in Ranges / bounds / selects) (an x / z or zero-divisor lsb of a declared range folds to 0):
* symptom: a net's declared range whose lsb holds x or z bits, or divides by a zero parameter, folds the lsb to 0 at exit 0
* IEEE 1800-2017 §6.9.1 (ieee1800-2017.txt:4259-4260): "It shall be illegal for them to contain any unknown (x) or high-impedance bits"
* witness: hand/xlsb `module top #(parameter N = 3, parameter Z = 0);` `logic [9 : N + 'z] n;` `logic [9 : N + 'x] m;` `logic [9 : N / Z] k;` `logic [9 : N + 4'b00x1] j;` `logic [9 : N - 5'd8] w;` with `$bits` of each
  * PRE vita-main 8c970e9b `A bn=10 bm=10 bk=10 bj=10 bw=15` at exit 0
  * oracles: verilator refuses n, m, k and j, each `right side of bit range isn't a two-state constant (IEEE 1800-2023 6.9.1)` (t.sv:2:16 to t.sv:5:16), `%Error: Exiting due to 4 error(s)`
  * oracles: iverilog `A bn=10 bm=10 bk=10 bj=10 bw=-13`, sv2v → iverilog `A bn=x bm=x bk=x bj=x bw=4294967283` (W/qsymmix/park2/m/xlsb/RES.txt)
* note: the w lane (`N - 5'd8`, folded as i64 -5, `bw=15`) has no two oracles that agree: iverilog `bw=-13`, sv2v → iverilog `bw=4294967283`, verilator stops at the four errors
* note: the same class as Q-PARAM2STATE's both_xz (`logic [4'bx : 4'bz] y;` `A b=1`), whose refusals are S1v's R1x and C1 (§5.2 Q-PARAM2STATE)
* note: found by the Q-SYMMIX re-entry plan (W/qsymmix/replan/PLAN.txt F-3), outside Q-SYMMIX: its gate admits only an lsb that folds to 0 at parse
* status: OPEN
