(§2 unsized-concat-operand · OPEN · in Ranges / bounds / selects) (an unsized constant concatenation operand is accepted):
* An unsized constant operand of a concatenation is accepted: `localparam logic [47:0] L = { 4'hA, 1 };` is `000a00000001`, = sv2v and verilator 5.052, where iverilog 13.0 refuses ("Concatenation operand \"'sd1\" has indefinite width."), IEEE 1800-2017 §11.4.12 (ieee1800-2017.txt:11976) makes it illegal
* leniency, recorded (row 40 plan b_unsz_ctl; the string twin is §3 string-concat-unsized)
* OPEN
