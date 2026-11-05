(§5.2 Q-UNION · rank ① · source ENG-2 r3 lens-sound N-UNION) (a whole read after part writes covering every bit still counts as a read):
* slot: after the fold-parity rows
* symptom: a whole read after part writes that together cover every bit stays read: `g[1:0] = 3; g[3:2] = 3; h = g; y = g[3];` in an `always_comb` wakes on `g` (RS2_tw2, RS4_tw2, and the `ref` forms RS2 RS4 RMF RMFr RMFc)
* status: pre-existing (PRE = POST)
* oracles: iv = vl no wake (sv2v wakes)
* IEEE 1800-2017 §9.2.2.2.1 exception b
* fix: cover a read by the union of the block's constant write regions
