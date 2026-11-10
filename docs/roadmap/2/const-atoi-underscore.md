(§2 const-atoi-underscore · OPEN · in Constant domain (i64)) (a constant-folded `atoi` stops at an underscore):
* symptom: the constant-folded string method `atoi` stops at the first `_`, where the run-time method skips it (`const_str.rs`, out O5 of §5.2 PARSE-1)
* witness: atoi_us `localparam string S = "1_0"; localparam int A = S.atoi();` beside `string s = "1_0";`, `$display("L a=%0d r=%0d", A, s.atoi());`: PRE 56c997be `L a=1 r=10` rc 0
* oracles: verilator `L a=10 r=10`; iverilog `L a=1 r=10`; IEEE 1800-2017 §6.16.9: "The conversion scans all leading digits and underscore characters ( _ )" (ieee1800-2017.txt:4642), so 10 (W/batch2/docs/probe/SUMMARY_r1.txt)
* note: PARSE-1 premise E10, kept byte-equal by PARSE-1
* status: OPEN
