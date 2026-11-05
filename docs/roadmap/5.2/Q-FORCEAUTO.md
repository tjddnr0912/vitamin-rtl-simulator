(§5.2 Q-FORCEAUTO · rank — · source ENG-2 r4 design review N-FORCE; row 33 probes f1, f2 (W/row33/probe/d)) (`force` to an automatic variable is accepted):
* symptom: `force` (and procedural `assign`) to an automatic variable is accepted: in a frame task without a wait it panics `frame_eval.rs:445` "arg bind: no active call window" on every backend (N13_force_tw), with an `@(*)` it is refused only through the frame classify's E3009 (N13_force, N13_force_nc)
* illegal, IEEE 1800-2017 §6.21 :5435-5436
* oracles: iv and sv2v refuse by name ("automatically allocated variables may not be assigned values using procedural force statements")
* `force u = {2{32'(3)}}` and `assign u = …` on an automatic local array are accepted too (f1, f2; iverilog and sv2v → iverilog refuse by name; PRE 79814020 and main's e0888973 alike (W/row33/park/REMEASURE-addendum.txt))
* fix: refuse it by name at elaborate
