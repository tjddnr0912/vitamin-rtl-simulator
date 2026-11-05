(§5.2 Q-CONSTPSELOOR · rank ② · source row 24 r2 lens-diff N2 (d/const_partial), r3 probe ch2 (const_partial_plain)) (a constant part-select partly or wholly outside its vector cannot be folded):
* slot: after Q-TPCARRIER, 70 and Q-PARAM2STATE
* a constant-context part-select partly outside its vector has no constant-fold arm: `localparam logic [15:8] V = 8'hA5; localparam int K = 7; localparam logic [3:0] X = V[K+3:K];` is E3009 "parameter `X` value is not a constant: the part-select `V[…]` has no constant-fold arm", also the literal `V[10:7]` and the literal member `P.a[10:7]` (`localparam h_t P`, member `logic [15:8] a`), where IEEE 1800-2017 §11.5.1 (ieee1800-2017.txt:12386–12387) reads `101x`: iverilog 13.0 = sv2v → iverilog `A cpp 101x`, verilator `xxxx`
* oracles: on the member, sv2v → iverilog `1010` (reads the neighbouring member), iverilog refuses (`A hierarchical reference … is not allowed in a constant expression`)
* pins: an in-range select folds (`V[K+3:K]` with K = 9: `0010`, iverilog = sv2v)
* evidence: PRE 9d60771d, every backend
* note: one fold arm serves the named member spelling `P.a[K+3:K]` once row 24 parses it
* symptom: a select wholly outside its vector is refused the same way: `localparam logic [15:8] VP = 8'hA5; localparam bit [2:0] C = 3'd7; localparam logic [1:0] X = VP[C + 3'd1 +: 2];` (bits 1:0) is E3009 "the part-select `VP[…]` has no constant-fold arm" (iverilog = sv2v → iverilog `xx`), and `localparam int X = VP[C + 3'd1];` E3009 "the select `VP[…]` has no constant-fold arm" (sv2v → iverilog `…0x`, iverilog `…00`)
* witness: their member twins `P.a[C + 3'd1 +: 2]` / `P.a[C + 3'd1]` fold the 32-bit-widened index on PRE (`01` / `…0001`, = sv2v; iverilog refuses a hierarchical reference in a constant expression) and are E3009 like the plain twin after Q-MEMBSRC
* status: loud
* source: Q-MEMBSRC design review x/ki, x/kl
