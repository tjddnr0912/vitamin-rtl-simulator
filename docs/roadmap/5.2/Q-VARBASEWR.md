(§5.2 Q-VARBASEWR · rank ② F · source Q6 re-plan page census (2026-11-01), row 29 r3 draft residue) (a variable-base `+:` write into a multi-dimensional packed element):
* slot: after 29: the same lowering, `packed_lval.rs`
* slot: 29's page half PARKED 2026-11-04 behind Q-PARAM2STATE, so this row waits on Q-PARAM2STATE through it
* slot: T3 is not refilled, owner 2026-11-04
* a variable-base indexed part-select of the innermost packed dimension of a multi-dimensional packed element as a write target is E3009: `logic [1:0][31:0] w; int o = 1; … for (int i = 0; i < 4; i++) w[o][8*i +: 8] = 8'(i+1);` prints `nested lvalue select (v1: single-level)` on PRE 719e37d2 and `variable-base indexed part-select of the innermost packed dimension is not supported as a write target` on row 29's r3 (W/row29/r3/bin/vita-post), verilator = sv2v → iverilog `R 0403020100000000`, iverilog refuses (W/program/q6-2026-10-31/replan/cells/c2.sv)
* `tlul_adapter_sram.sv:447–448` (`wmask_int[woffset][8*i +: 8]` on `logic [WidthMult-1:0][TL_DW-1:0]`), hidden today behind that page's earlier errors
* r3's draft residue `leaf-varbase-indexed` (W/row29/r3/draft-roadmap.diff:21) held it outside this table
* fix: lower the base with the element's offset and clip it to the element's own bits: a base that runs past the dimension at run time changes only its in-range bits (IEEE 1800-2017 §11.5.1), the clipping of md-packed-partial-oob-write (cell blk_ipo_c)
* first: the base in range, past each end, x, and a signed base, on PRE, r3 and the oracles
* hmac, usbdev, spi_host, spi_device
