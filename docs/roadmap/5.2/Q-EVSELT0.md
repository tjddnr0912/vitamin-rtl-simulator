(§5.2 Q-EVSELT0 · rank ① · source Q-MEMBSRC plan probe t0ev) (an event control on a constant-expression bit-select counts an extra event):
* slot: after Q-IDXVALUE
* symptom: a level event control on a bit-select of a non-zero-LSB vector whose index is a constant EXPRESSION counts one extra event: `logic [15:8] w = 8'hA5; localparam bit [3:0] N = 4'd7; always @(w[~N]) c2++; always @(w[N + 4'd1]) c4++;` with two toggles of `w[8]` prints `3` (native, vm, interp) where iverilog 13.0 = sv2v → iverilog print `2`
* pins: the literal `w[8]` / `w[4'd8]` count `2`
* evidence: PRE 9d60771d
* status: silent-wrong
