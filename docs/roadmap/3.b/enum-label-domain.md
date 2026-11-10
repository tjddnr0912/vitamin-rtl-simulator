(§3.b enum-label-domain · OPEN · in Constants / parameters) (enum labels that the 64-bit label fold cannot hold):
* enum labels the i64 label fold cannot hold are E3009 (`enum label … value is not a foldable constant`): an unbased unsized `'z` label (`csrng_pkg` `CMD_STS_UNDRIVEN`: verilator and sv2v → iverilog `A s=001 undr=zzz`; iverilog `undr=00z`, an oracle defect), labels wider than 64 bits (55 `lc_ctrl_state_pkg` labels: all three `A s=b79f1234b79f1234b79f`), and a `'1` label sized at 32 bits instead of its 3-bit base (row-9 X01, overlay text only: verilator and sv2v → iverilog `A s=111`)
* cause: `package.rs:928–938`, `const_fn.rs:1255`, `instance.rs:131`
* fix: expected fix per §5.2 row 48: labels take the parameters' 4-state wide lane at the base's width and sign
* witness: row-9 R25, row-9 R28, row-9 X01
* witness (2026-11-10, PARSE-1 lane L12): l12_enum64 `typedef enum logic [63:0] {A = 64'hFFFF_FFFF_FFFF_FFFF, B = 64'd0} e_t;` `$display("L e=%h", e);` with `e = A`: PRE 56c997be `t.sv:2:34: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected an enum label value that fits the enum base type, found identifier 'e_t'`; iverilog = verilator `L e=ffffffffffffffff`
* status: OPEN (§5.2 row 49)
