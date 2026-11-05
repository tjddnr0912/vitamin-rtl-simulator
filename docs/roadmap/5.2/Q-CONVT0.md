(§5.2 Q-CONVT0 · rank ① · source ENG-1 r2 (C09 C14 C15 C16)) (a continuous copy converting real and integral takes a spurious time-0 event):
* symptom: a continuous copy that converts between `real` and an integral type (`real c; assign c = lv;`, `assign w = r;`) takes a time-0 event where iverilog has none, and a same-delta read sees the old value (vita = verilator, iverilog the new value)
* note: input to ENG-1b's time-0 rule
* fix: a destination / root kind check inside `sim-engine/src/alias.rs` `copy_alias` before such a driver joins the copy nets
* note: size S
