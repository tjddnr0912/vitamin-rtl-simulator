(§5.2 Q-DEADARMREAD · rank ① · source ENG-2 design review (PP_star_count)) (a constant-condition dead arm keeps its reads in an implicit read set):
* slot: after Q-RS30FOLD
* a constant-condition dead arm keeps its reads in an implicit read set: two generate-iteration `always @*` blocks with `if (k == 0) n0 = n0 + 1; else n1 = n1 + 1;` each read the counter only the other iteration's live arm writes, so they wake each other to F4016 at t=1 (PRE = POST) where iverilog folds the condition (`any=11 n0=1 n1=1`; verilator does not converge)
* status: pre-existing, outside ENG-2
* IEEE 1800 §9.4.2.2 / §9.2.2.2.1 count what the statement reads, and an arm elaboration removes reads nothing
* fix: leave a `dead_arm.rs` arm out of the read-set walk
