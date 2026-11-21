(§3.b ca-late-fill-position · OPEN · in Constant edges (§4.5.601)) (a late fill at a continuous assignment is refused at the module header):
* symptom: a continuous assignment whose target width folds after elaboration and whose right side is an `'x` / `'z` fill (`assign n16[$bits(u.b8)-1:0] = 'x;`, at line 50) is refused at the MODULE HEADER's position (23:8) with PRE's late-width text, not at the assignment; PRE 86fa310d, fix H's reference and its round 3 alike (W/batch6/RULINGS.txt L2-58; main's re-measure W/main_d7/2026-11-20/b6_fixh3; cell `S5_concat_late_x` of `crates/cli/tests/fill_matrix/cells.txt` in the pulled 3c99c4eb)
* cause (main's): a late Fill record at a continuous assignment keeps `cur_span`, which during module elaboration is the module header; the class of §5.2 Q-WEXTPOS (a store or record taking an enclosing construct's position) in a diagnostic
* note: the refusal itself is §3.b late-target-capture's (a legal design); this entry is its position
* status: OPEN
