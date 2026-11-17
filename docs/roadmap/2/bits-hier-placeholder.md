(§2 bits-hier-placeholder · OPEN · in Ranges / bounds / selects) (`$bits` of a hierarchical name reads a 32 placeholder before patching):
* `$bits(u.X)` lowers to a `Const 32` placeholder until `resolve_deferred_hier_bits` patches it (`expr_special.rs:532`), and a reader of its VALUE before then takes 32: a `case` label (x05 `r=2`), `$bits({($bits(u.X)){1'b1}})` (x24 `r=20`) (verilator and iverilog `1`, `7`)
* index sealing also reads it as 32 (k04_bx `r = mem[$signed(f[$bits(u.X)-2:0]) + 7'sd5]` `r=000000xx`; verilator and iverilog `r=00000004`; the generate-block twin is right)
* §4.5.601 treats it as undecided only for widths and counts
* fix: a placeholder with a recorded `int` shape and no value (`Signal{POISON_NET}` + `hier_placeholder_shape`), after censusing its value readers
* 2 oracles
* face 2 (2026-11-17, Q-MDRVMIX's park; W/wrext/fix1/PLAN.txt part 1, premise judge 1 K5): a select index that `norm_sub_k` / `norm_k_sub` (`packed.rs`) fold for a declared range whose low bound is not 0 or which ascends reads the provisional 32 and folds it into a fresh constant `resolve_deferred_hier_bits` never patches, so the store writes, or the read reads, the wrong bit: PRE == POST, rc 0, silent; the forms: a continuous, a nonblocking and an output-port-connection target, an rvalue, a hierarchical net's and a hierarchical parameter's select, and an index the fold puts out of range (the store dropped, the read x); each confirmed by iverilog 13.0 and verilator 5.054 (W/wrext/fix1/premise/out/orc_b2.txt, orc_b6.txt; sv2v → iverilog is harness-format: a syntax error on its own output)
  * f02_idxpart_lsb_value, `logic [64:1] y; assign y[$bits(u.w) +: 2] = 2'b11;` (`u.w` is `logic [3:0]`): vita `y=xxxxxxxXXxxxxxxx`; iverilog `y=zzzzzzzzzzzzzzZZ`; verilator `y=0000000000000018`
  * f03_rsel_lsb, a `logic [40:1]` read at `[$bits(u.w)]`: vita `r=0`; iverilog `r=1`; verilator `r=1`
  * f04_rsel_asc, the same read of a `logic [0:39]`: vita `r=0`; iverilog `r=1`; verilator `r=1`
  * f06_hiersel_net_lsb, `u.v[$bits(u.w)]` of the child's `logic [40:1] v` with `v[4] = 1'b1`: vita `r=0`; iverilog `r=1`; verilator `r=1`
  * f07_hiersel_param_lsb, `u.K[$bits(u.w)]` of the child's `parameter logic [40:1] K = 40'h8`: vita `r=0`; iverilog `r=1`; verilator `r=1`
  * f11_rsel_lsb_part, an indexed part-select read of a `logic [40:1]` at `$bits(u.w)`: vita `r=00`; iverilog `r=11`; verilator `r=11`
  * f16_port_slice_lsb, `leaf l(.o(y[$bits(u.w)]));` on a `logic [40:1] y`: vita `y4=x y32=1`; iverilog `y4=1 y32=z`; verilator `y4=1 y32=0`
  * f17_nba_lsb, `always_ff @(posedge clk) y[$bits(u.w)] <= 1'b1;` on a `logic [40:1] y`: vita `y4=x y32=1`; iverilog `y4=1 y32=x`; verilator `y4=1 y32=0`
  * k5d_oor_lval_lsb1, `logic [8:1] y; assign y[$bits(u.w)] = 1'b1;` (the fold puts the index out of range): vita `y=xxxxxxxx`; iverilog `y=zzzz1zzz`; verilator `y=00001000`
  * k5e_oor_rval_lsb1, `logic [8:1] y` set to `8'b0000_1000`, `$display("r=%b", y[$bits(u.w)])`: vita `r=x`; iverilog `r=1`; verilator `r=1`
  * w02_norm_lsb1_value (W/wrext/fix1/cells), `logic [40:1] y; assign y[$bits(u.w)] = 1'b1;`: vita `y=xxxxxxxx1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` (bit 32 written); iverilog `y=zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz1zzz`; verilator `y=0000000000000000000000000000000000001000` (bit 4)
  * w12_slice_bound, `logic [3:0] m [0:63]; m[$bits(u.w) +: 2] = '{4'ha, 4'hb};`: vita `m4=0 m5=0 m32=a m33=b`; verilator `m4=a m5=b m32=0 m33=0`; iverilog refuses (`t.sv:7: error: Array cannot be indexed by a range.`)
  * right today (the fold is not taken): a descending range with low bound 0 (f09_desc0_control; k5f_oor_lval_lsb0 `logic [7:0] y`: vita `y=xxx1xxxx`, iverilog `y=zzz1zzzz`, verilator `y=00010000`) and a 2-dimensional rvalue (f10_rsel_md: vita, iverilog and verilator `r=11`)
* face 1 (the same root, a reader of the write view): Q-WREXT's geometry constant and a kernel extent can be stale on such a design (Q-WREXT.md, the boundary findings; W/wrext/fix1/cells w01 w03 w06 w07 w08, W/batch5b/lens-sound/cells a1-a5); no output reads the view on main, and §5.2 Q-WEXTPOS stands the view down on a provisional constant
* fix (2026-11-17): the root row is §5.2 Q-HIERBITS: `$bits` of a hierarchical name lowers as the Signal placeholder with a leaf shape a hierarchical parameter already takes (`u.P`), so no lowering-time fold reads a provisional value; it is the fix line above, made a row (premise judge 1 line 9: no new §2 entry, this one amended)
* OPEN
