(§2 enum-base-overflow-silent · OPEN · in Constant domain (i64)) (an enum base wider than 2^64 bits runs with a wrapped width):
* symptom: an enum base range whose bound overflows 64 bits folds to a wrapped width and the design runs, where both oracles refuse it
* witness: b_base_ovf `typedef enum logic [4611686018427387904*4+3:0] {A = 20, B} e_t;` `v = B;` `$display("L n=%s v=%0d", v.name(), v);`: PRE 56c997be `L n= v=1` rc 0
* oracles: iverilog `warning: verinum::as_long() truncated 66 bits to 63, returns 3` and `t.sv:2: error: Enumeration name A has a value that is too large 32'sd20.`; verilator `%Error-ENUMITEMWIDTH: t.sv:2:55: Enum value exceeds width of enum type (IEEE 1800-2023 6.19)` (W/batch2/docs/probe/SUMMARY_r2.txt)
* note: PARSE-1 premise E2 (W/parse1/build/NOTES.txt D5), kept byte-equal by PARSE-1
* status: OPEN
