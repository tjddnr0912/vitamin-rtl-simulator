(§5.b STORE-STACK-POS · OPEN) (store-path speed depends on where frames sit on the stack):
* the store path's cost depends on where its frames sit on the stack: BASE's code (9483559e, 0.00 % more instructions) with every frame below `simulate` 48–96 B lower runs `real_store` / `int_store` +4–6 % cycles, and 112–160 B lower within ±1 % (a 128-B period: 4 of 8 placements slow), while picorv32 runs 6.5–7.3 % fewer cycles at 48–112 B lower (`k_schedule_nba_scalar`)
* evidence: ENG-1c r3's two new `Scheduler` fields (+48 B in `simulate`'s frame) measured `real_store` +5.7–5.9 %, `int_store` +5.1–5.4 %, picorv32 -6.7–7.6 % wall vs BASE (both orders), and the landing (§4.5.628) keeps BASE's `Scheduler` size instead (W/eng/eng1c/r4 REPLAN.txt, P1_MATRIX.txt)
* suspected cause, unmeasured: `NetArena::write_lvalue_inner` reloads the caller's 72-B `Value` temporary with 32-B loads whose store straddles a cache line at the slow placements
* acceptance: a flat `base_s` sweep — BASE with a 0–160 B frame between `simulate` and `native::run::run`, every shift within ±3 % cycles on `real_store`, `int_store` and picorv32 (W/eng/eng1c/r4/bin/mkcand.py, run_p1.sh)
* status: OPEN
