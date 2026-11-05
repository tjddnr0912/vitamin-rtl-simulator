(§2 generate-param-overridable · RECORD · in Scoping / imports / block-locals) (a generate-block parameter is treated as overridable):
* a generate-block `parameter` (type and value) is treated as overridable (`hdl-parser/src/type_params.rs:170–172` `overridable`, `params.rs:847–848` `non_overridable`) where IEEE 1800-2017 §6.20.4 (ieee1800-2017.txt:5256–5262) makes it a localparam
* no override reaches one on PRE (named `#(.N(5))` E3002 = the oracles' errors; `defparam u.g.N` E3009 where iverilog says "Cannot override parameter"; W/qtpcarrier/plan/q/o5v_*), and for a type it only picks the lowering (the carrier is right where the P5 stand-in is wrong: plan/c/o5a)
* what the value predicate feeds (`const_locals`) is unmeasured
* no output change measured
* RECORD
