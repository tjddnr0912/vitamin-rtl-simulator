(§5.2 Q-CLKSHADOW · rank ①) (clocking-name resolution misses generate and instance shadowing):
* slot: after 75
* clocking-name resolution under generate / instance shadowing: the lowering-time `cb.x` walk (`held.rs` `clk_refused_at`, the write / NBA / event / `$readmem` pre-checks) does not see a generate-local instance, interface or generate block named `cb` (its nets are bound later by the deferred hierarchical resolver) nor a generate-local clocking block (no alias), so a use meant for the inner object is refused naming the outer item (over-refusal: vl `S v=9`) or, for a non-refused outer item, reads or writes it (silent: SH_inst_write_gen_out_int vita `S v=9 t=9`, vl `S v=9 t=3`; CG_gen_cb_shadow `S v=WRONG`)
* symptom: an absolute upward `top.cb.s` from a child instance is a generic E3010 (vl `S v=ab`)
* IEEE 1800 §14.3, §23.6
* fix: bind clocking names in the scope model, not by a lowering-time walk
* source: ENG-1b r2 lens-sound N1 = lens-diff N3 (SH_inst_write_gen, SH_inst_nba_gen, SH_intf_write_gen, SH_genblk_write_gen, SH_inst_readmem_gen, SH_inst_write_gen_out, SH_inst_event_gen, F3_gen_local_cb_shadow, F3_gen_local_cb_shadow_rev), r2 lens-diff N5 (F3_upward_abs_from_child), r1 lens-sound N8 (CG_gen_cb_shadow), r2 lens-sound N7 (SH_inst_write_gen_out_int)
