(§2 negated-min-bound-one-bit · OPEN · in Ranges / bounds / selects) (a range bound over a negated 64-bit minimum takes 1 bit):
* symptom: SILENT: a declared msb `-(64'sh8000_0000_0000_0000) >>> 60` declines and the net takes the 1-bit default at rc 0
* witness: g05a_negmin_bound `logic [-(64'sh8000_0000_0000_0000) >>> 60 : 0] a;` `logic [(64'sh8000_0000_0000_0000) >>> 60 : 0] b;` `$display("A %0d %0d", $bits(a), $bits(b));`: PRE = POST `A 1 9` rc 0
* oracles: iverilog `A 9 9`, sv2v → iverilog `A 9 9`, verilator `%Error: t.sv:2:38: Value too wide for 32-bits expected in this context 64'hfffffffffffffff8` (W/batch3/lens-diff/D3_TABLE.txt; main's re-measure on the frozen binaries, R4)
* note: through a localparam (g05b) the same bound answers 9; the sink is the class of §5.2 Q-DECLSELBOUND (a bound that does not fold takes a silent default)
* note: batch 3 lens-diff G5 (W/batch3/RULINGS-lens-diff-r1.txt R2), PRE = POST, not batch 3's
* status: OPEN
