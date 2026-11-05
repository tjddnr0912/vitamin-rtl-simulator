(§5.2 Q-MIX · rank ① · source ENG-1 r2 (P21, M01, M03, N36, N36i)) (in a mixed level and edge event list, the level term wakes on bit 0 only):
* symptom: mixed level + edge header list (`always @(v or posedge clk)`): the level term wakes on bit 0 only — `elaborate/src/events.rs` `classify_event_list` makes the whole list `SensKind::Edge`, so `v` is an `AnyEdge` term read through the bit-0 edge mask
* witness: `logic [1:0]` 01 → 11 wakes nothing where all three print `W t=3 v=11`, a real 1.0 → 2.0 likewise (int too, PRE = POST)
* IEEE 1800 §9.4.2 "any change in the value"
* fix: per-term kind in a mixed list (level terms on the change stamp)
* note: size S–M
