(§5.2 Q-MIXCAPROC · rank ①) (continuous and procedural writes to overlapping bits of a variable are accepted):
* slot: after the fold-parity rows
* symptom: a continuous assignment and a procedural write to overlapping bits of one variable are accepted: `logic [7:0] y; assign y["\003" +: 2] = a; always_comb y[4] = b;` (also `y[3 +: 2]`, `y[4:3]`) runs, vita `S y=xxx01xxx` on every backend (PRE = Q-RELDELAY)
* IEEE 1800-2017 §6.5 makes it an error ("a mixture of procedural and continuous assignments writing to any term in the expansion of a written longest static prefix of a variable")
* oracles: iverilog refuses (`Cannot perform procedural assignment to bit select 'y['sd4]' because it is also continuously assigned.`), sv2v -> iverilog runs `xxx0xxxx`, verilator `00001000` (2-state)
* pins: the disjoint twin `always_comb y[5] = b;` is legal and runs (vita `xx011xxx`)
* symptom: the whole-variable and literal-member spellings the same (row 24 r3 probe s2t; PRE 9d60771d, every backend): a procedural continuous `assign r = 4'h0;` beside `assign r[1:0] = x;` on `logic [3:0] r` (logic_pca_pa) or beside an output-port actual `sub u(.o(r[1:0]))` (logic_port_pa) runs `r=0000`, and `assign h.a[13:10] = x;` beside a procedural continuous `assign h = '0;` (mem_ca_pa_lit) or a blocking `h = '0;` (mem_ca_blk_lit) runs `h=000000000000`
* oracles: iverilog refuses each (`Cannot perform procedural assignment to variable 'r' because it is also continuously assigned.`), sv2v → iverilog runs the same values
* §6.5 counts a procedural continuous assignment as a procedural write (ieee1800-2017.txt:3483–3484)
* fix: refuse by name
* status: pre-existing, loud on no backend
* source: Q-RELDELAY r5 lens-diff N3 (k_proc_var_ca_str, k_proc_var_ca_num, k_proc_var_ca_sp, k_proc_var_ca_disj_num); row 24 r3 probe s2t (logic_pca_pa, logic_port_pa, mem_ca_pa_lit, mem_ca_blk_lit)
