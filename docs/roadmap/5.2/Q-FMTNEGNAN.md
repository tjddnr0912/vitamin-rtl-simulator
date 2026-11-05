(§5.2 Q-FMTNEGNAN · rank ① · source §4.5.619 census (`real_signed_zero_render.rs` z08)) (a negative NaN prints without its sign under `%f`):
* slot: after Q-NANCANON
* symptom: a NaN whose sign bit is set prints `nan` under `%f` (vita `nan=nan`; iverilog = sv2v = verilator `nan=-nan`, as glibc's printf)
* cause: `render.rs` `nonfinite_c` and `vcd_writer::fmt_g` spell every NaN `nan`
* fix: census first: the other specs, the `+` flag (pinned: a NaN takes no `+`), the VCD spelling and the NaN producers' sign on each oracle
* scope: render only
* note: size S
