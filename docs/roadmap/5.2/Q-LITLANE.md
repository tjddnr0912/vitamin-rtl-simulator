(§5.2 Q-LITLANE · rank — F · source 29.md:55, VAL-2.md:14; W/program/reorder2-2026-11-09/PLAN.txt G4) (`const_eval_u32` reads a sized signed literal as its unsigned pattern):
* slot: R7.5 · L1, the literal lane of the record-first order, filed as a row 2026-11-17 (roadmap review G1, main by guide); on goal G-EH1 after Q-OVRXPLANE and before row 29's page half (ROADMAP.md §5.2's goal block)
* what: `const_eval_u32` (`elaborate/src/const_fn.rs:83`), its `IntLit` arm, reads a sized signed literal under `( )` or unary `+` as its unsigned pattern (29.md:55); the lane retires into VAL's homes
* why: it is row 29 r1's producer, and the page half waits on it (29.md:6, :7); R7.5 was a slot name with no row
* readers: row 29's page half (r1), the Q-EXECSEM census, which reads VAL-2's head-note lane table with it (VAL-2.md:14)
* waits on: VAL-1–3 (landed, §4.5.639); its sites and cells are this row's grounding
* status: OPEN (no plan yet)
