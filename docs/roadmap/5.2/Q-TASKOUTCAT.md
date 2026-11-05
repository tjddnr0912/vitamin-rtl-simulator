(§5.2 Q-TASKOUTCAT · rank ① · source Q-RELDELAY r2 lens-diff N3 (D03), r2 D7 (D03a D03b D03c)) (a concatenation as a task's output actual is refused):
* symptom: a concatenation as a task's output actual is E3009 (`output/inout arg must be a simple net` for an automatic task, D03a; `... simple net or select (v1)` for a static one, D03b), against IEEE 1800-2017 §13.5 (an output actual is any expression valid on the left-hand side of a procedural assignment): `t2({y, z})` iverilog = verilator `S t=0 y=c z=3` (D03a), iverilog = sv2v = verilator (D03b)
* pins: a part-select actual runs (D03c, = the three)
* status: over-refusal, loud
* status: pre-existing
