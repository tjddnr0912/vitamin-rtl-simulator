(§5.2 Q-ARRWORDMD · rank ① · source Q-RELDELAY r2 re-plan (X01 X02 X03)) (two continuous assigns to one net-array word are neither refused nor resolved):
* slot: after the fold-parity rows
* symptom: two continuous assignments to one net-array word are neither E3001 nor resolved: `wire [3:0] m [0:1]; assign #3 m[0] = a; assign m[0] = b;` vita `M t=5 m0=0011`, iverilog = sv2v `00x1` (verilator `0011`, 2-state)
* cause: `check_whole_net_multidriver` (`elaborate/src/netdecl.rs`) does not count array-element chunks
* note: a word or whole-array force / release is E3009 (X02 X03), so no release reaches it
* status: pre-existing
