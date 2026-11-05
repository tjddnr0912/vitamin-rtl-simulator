(§5.2 Q-SVALOCALKIND · rank ① · source Q-DECLTYPE grounding G1 (W/qdecltype/ground/g1/sva_local_bit)) (an SVA local variable's declared 2-state kind is dropped):
* slot: past the last numbered slot
* slot: after Q-DECLTYPE's S12 lands (it leaves `SvaLocalDecl` as it is: a resolved width and sign, no kind)
* an SVA local variable keeps its width and sign but not its kind (`SvaLocalDecl`, `hdl-ast/src/lib.rs`, no kind field), so a `bit` local holds x
* witness: sva_local_bit `property pb; bit [3:0] lv; @(posedge clk) (1, lv = 4'bxx01) |-> (lv == 4'b0001); endproperty` `assert property (pb) else fails++;` `#42 $display("R fails=%0d", fails);` PRE `R fails=4` at exit 0, verilator `R fails=0`, hand-IEEE 0 (a 2-state variable holds 0 where x is assigned), iverilog `t.sv:4: syntax error` `t.sv:4: error: Invalid module item.`, sv2v `t.sv:4:20: Parse error: unexpected token '[' (Sym_brack_l)`
* evidence: control sva_local_logic (`logic [3:0] lv`) PRE `R fails=4` = hand-IEEE (x compares false), verilator `R fails=0` (2-state: not an oracle there)
* evidence: control sva_local_ctl PRE `R fails=0`, verilator `R fails=0`
* note: §4 n2c-seq-local-vars is the unsupported full feature, not this
* fix: record the kind in the declaration's type record and convert at the local's assignment
* status: silent-wrong, pre-existing (PRE 92805863)
* source: W/qdecltype/GROUND.txt G1, W/qdecltype/plan/PLAN.txt P6
