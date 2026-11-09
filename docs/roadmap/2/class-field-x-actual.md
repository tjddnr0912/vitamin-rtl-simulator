(§2 class-field-x-actual · OPEN · in Class fields) (a class field's range over an x parameter actual is silent):
* symptom: a parameterized class field's range over an x actual folds silently: `monomorph.rs:20` substitutes the actual at the name's span, and a class field is not a `DeclSite::Module` site
* witness: c1_class `class C #(parameter W = 4); logic [W : 0] f; endclass` `C #(4'bx1) h;` PRE vita-main 8c970e9b rc 0 `A bf=1` (the S1v-R' prototype pRp the same)
* oracles: verilator `%Error: t.sv:3:12: left side of bit range isn't a two-state constant`; iverilog and sv2v cannot parse the class (W/s1v/plan2/filings.txt F-CLASS)
* note: an under-report: a range bound shall not hold x or z (IEEE 1800-2017 §6.9.1), the class of §2 xz-range-lsb
* status: OPEN
