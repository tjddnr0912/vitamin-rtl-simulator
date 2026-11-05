(§2 assoc-index-widened · OPEN · in Ranges / bounds / selects) (an associative index is widened to 64 bits, not cast to the key):
* The index of an `[int]` / `[integer]` / `[byte]` / `[shortint]` / `[longint]` associative array is the expression widened to 64 bits (`eval_core.rs` `assoc_key_eval_ctx`), never cast to the key type (§7.8.4): p_ctx `p[c + d]` over two `byte`s `p i=254 v=1`, `(e + f) >>> 1` `p i=2147483647`, and a `foreach` over `n[e + f + 3]` (`[byte]`) stops with W4020 (verilator `p i=-2 v=1`, `p i=-1 v=3`, `n i=1 v=2`)
* t_hand_loop_k `a[64'd1099511627776]` on `[integer]` keeps 2^40 (verilator key 0)
* t_wide_fe W4020 (verilator `0` / `done`)
* q5_ctx_leg `u k=240`, `u k=254` (verilator `0`, `-2`)
* q2_forms2 and q2_sysf 9 of 14 / 5 of 6 index forms
* c_carry `[int]` `p[(e + f) >>> 1]` `p i=2147483647` and `[byte]` `n[(c + d) >>> 1]` `n7f=2 nm1=0` (verilator `p i=-1`, `n7f=0 nm1=2`), refused on PRE only because the same file declares a `[bit [31:0]]` key (loud→wrong through this line since §4.5.616)
* k_byte, k_int, k_integer, k_si never end (`while (st)` on a −1 status)
* §4.5.616 cast the index and was reverted for these spellings in its round 2 (soundness F1): the cast stores a key at or above 2^(W−1) written through an unsigned index as negative, and `frame_eval.rs` `assoc_iter_compute` reads a `next` / `prev` argument by the variable's own sign and tests the fit by value, so an unsigned iteration variable stopped early or looped forever (q2_int_prev `last`/`prev` over `[integer]` with `logic [31:0] k`, q2_b32_iter, q2_iu_iter, q2_byte_u8, q2_si_u16: PRE and verilator walk every key)
* fix both together (cast the index, and read the iteration argument and test its fit in the key type)
* typed keys are cast and immune (their variable must have the key type)
* oracles: verilator + hand-IEEE §7.8.4, §7.9.8
* status: OPEN
