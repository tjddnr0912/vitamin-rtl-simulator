(§5.2 Q-PKGDUPNAME · rank ① · source owner Q4 (a), 2026-11-17; §2 duplicate-value-name-accepted; 23.md RE-e) (an enum label or typedef redeclaring a package-body name is accepted):
* slot: R9.1 · L1, held; it opens when goal G-PATTGEN starts, and row 23 follows it (owner Q3 (a) and Q4 (a), 2026-11-17, "5개 질의에 대해서 모두 추천안을 채택"; ROADMAP.md §5.2's goal block)
* what: §2 duplicate-value-name-accepted's package-body pairs (18 of its 32 cells): one name declared twice in a package body, one of them an enum label or a `typedef`, runs at rc 0 where iverilog and verilator refuse (IEEE 1800-2017 §3.13)
* why: row 23's RE-e (23.md:43, :69) waits on this fix, so G-PATTGEN's row 23 needs it first
* home: elaborate's pair gate, `elaborate/src/decl_collide.rs:245-247`, which answers false for every package pair with no routine in it (the §2 entry's source line, re-read at this tree)
* readers: row 23 (RE-e)
* the id is row 23's re-entry plan's working name for the same item (W/r23/plan/PLAN.txt §0 m2, §1), kept; the routine-body pairs stay in the §2 entry
* status: held (opens with G-PATTGEN)
