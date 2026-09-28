# Probe catalog — defects found outside the real-design path

This file records pre-existing defects that a review measured OUTSIDE a slice's fix path: a probe
grid, an operator × width × lane sweep, a lens hitting "outside the table". Each is one row, recorded
verbatim with its repro and oracle values, so nothing a review finds is lost.

It is not a queue and not a progress measure. Rows here do not enter ROADMAP §5.2 and are not counted
in the ROADMAP Summary. A row moves to ROADMAP, and so into the queue, on one of three triggers:

1. a workload-corpus design hits it (`crates/corpus-runner`, [study/03](study/03-workload-corpus.md));
2. it lies inside the fix path of a corpus row;
3. an external report reproduces it.

ROADMAP §2 is frozen under the same rule; its rows predate this file and stay where they are. A
defect found INSIDE a slice's fix path is not catalogued here: the slice closes it, or it becomes a
queue row with the corpus witness that put the slice there.

| found in | defect · repro · oracle values | measured by | code site (if known) |
|---|---|---|---|
| §4.5.563 review (the `ibex` testbench) | vita's single-writer check for `always_ff` (IEEE 1800-2017 §9.2.2.4) covers whole variables only. `initial done = 1'b0;` beside `always_ff @(posedge clk) done <= 1'b1;` is `E3001 E-ELAB-MULTIDRIVER`, but the same two writers on an array element (`initial mem[3'd2] = 32'h11;` / `always_ff … mem[a] <= 32'h22;`), on an element's part-selects (`mem[a][7:0] <= …`) or on a vector's part-select (`w[7:0] <= 8'hab;` beside `initial w = 32'h0;`) run with exit 0. The values agree with iverilog (`m2=00000022`, `m2=0000cdab`, `w=000000ab`), which accepts all four including the scalar; verilator `-Wall` flags MULTIDRIVEN on the scalar only. A missing diagnostic, not a wrong value | soundness lens p1–p4, differential lens tw_arr / tw_scl; re-run by the slice | the E3001 multi-driver check in elaborate |
