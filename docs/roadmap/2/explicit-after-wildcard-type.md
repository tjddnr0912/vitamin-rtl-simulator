(§2 explicit-after-wildcard-type · in Oracle splits (recorded, not chased)) (an explicit import after a wildcard import of the same type name):
* an explicit import after a wildcard import of the same type name (`import p::*; import q::s_t;`, n02, §4.5.609 review) — verilator binds `s_t` to p's type (`A o=09`), sv2v → iverilog to q's (`A o=95`, as Icarus Verilog does on the untyped twin)
* vita = sv2v and iverilog (PRE untyped and POST typed `A o=95`)
* split
