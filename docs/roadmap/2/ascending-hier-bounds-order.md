(§2 ascending-hier-bounds-order · OPEN · in Diagnostics / artifacts) (a late-folding part-select of an ascending hierarchical net says out of order):
* symptom: A part-select of a hierarchical net declared ascending whose bounds fold only after elaboration is E3009 "bounds out of order" although they are in order (rb03 `u.g2[gb.A:gb.B]`, rb16; verilator and sv2v `3e000000`): the deferred lane builds its width before the net's direction is known (`width_from_msb_lsb_checked`), and the literal twin's refusal says the net is descending (rb03b)
* fix: name the ascending hierarchical net in both refusals, and build the width in its direction when the lane resolves it
* status: OPEN
