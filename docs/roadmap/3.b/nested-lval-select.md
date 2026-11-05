(§3.b nested-lval-select · OPEN · in Nets / timing) (a refused nested lvalue leaves a chunk that fakes a second writer):
* symptom: a nested lvalue the funnel refuses (`elaborate/src/lvalue.rs:577` `nested lvalue select (v1: single-level)`) leaves an unresolved chunk that `cont_array.rs:369` `other_writers` counts (`c.net >= nets.len()`), a false E3018 on another module's `output reg` (EH1 `ahb_sif.sv:216`, all three `r=1 q=0018005`; row-9 O18)
* one funnel with `md-packed-write`
* status: OPEN (§5.2 row 29)
