(§5.2 Q-CAPE3009 · rank ②) (a delayed capture into a hierarchical `real` from an integral source is refused):
* symptom: a capture (`y = #d rhs`, `y = @(ev) rhs`) into a hierarchical `real` / `realtime` destination from an integral, signed, `$random` or `$dist_*` source stays loud: the capture-width record `AtMost` refuses the 64-bit target (`edge_gate.rs`) and the text ("holds the right-hand side at 32 bit(s)") names the integral temp, not the type
* evidence: PRE = ENG-1b
* fix: size the capture from the retyped destination
* source: ENG-1b r1 lens-sound N3 (HX_int, HX_dist, HX_random, HX_str_to_int, HX_cap_real_elem_hier), r1 lens-diff N5, r1 BUILD (HX_cap_real_hier)
