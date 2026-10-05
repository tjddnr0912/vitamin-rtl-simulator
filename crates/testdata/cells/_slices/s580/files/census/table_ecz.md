| id | first probed expr | PRE | POST | sv2v→iv | verilator |
|---|---|---|---|---|---|
| E01 | `v inside {'b1?00}` | x | 1 | 1 | 1 |
| E02 | `v inside {'bx1}` | x | 1 | 1 | 1 |
| E03 | `v inside {'bx1}` | 0 | 0 | 0 | 0 |
| E04 | `v inside {'b?}` | x | 1 | 1 | 1 |
| E05 | `v inside {'bx1}` | 0 | 1 | 0 | 1 |
| E06 | `v inside {'b1x1}` | 0 | 0 | 0 | 0 |
| E07 | `v inside {'h?}` | x | 1 | 1 | 1 |
| E10 | `v inside {'x}` | x | 1 |  | 1 |
| E11 | `v inside {'z}` | x | 1 |  | 1 |
| E12 | `v inside {'1}` | 1 | 1 | 1 | 1 |
| E13 | `v inside {'1}` | 0 | 0 | 0 | 0 |
| E14 | `v inside {'0}` | 1 | 1 | 1 | 1 |
| E15 | `v inside {4'b0001, 'x}` | x | 1 |  | 1 |
| E20 | `v inside {L}` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 1 |
| E21 | `v inside {L}` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 1 |
| E22 | `v inside {P}` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 1 |
| E23 | `v inside {L}` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 1 |
| E24 | `v inside {L}` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 0 | 1 |
| E25 | `v inside {L}` | 1 | 1 | 1 | 1 |
| E26 | `v ==? L` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 1 |
| E30 | `v inside {A}` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 1 |
| E31 | `v inside {A, B}` | 1 | 1 | 1 | 1 |
| E40 | `v inside {{2'b1?, 2'b00}}` | x | LOUD error[VITA-E3009] | 1 | 1 |
| E41 | `v inside {P \| 4'b000x}` | x | LOUD error[VITA-E3009] | 1 | 1 |
| E42 | `v inside {(4'b1?00)}` | x | 1 | 1 | 1 |
| E43 | `v inside {4'(4'b1?00)}` | x | LOUD error[VITA-E3009] | 1 | 1 |
| E44 | `v inside {1'b1 ? 4'b1?00 : 4'b0000}` | x | LOUD error[VITA-E3009] | 1 | 1 |
| E45 | `v inside {{2{2'b?0}}}` | 0 | LOUD error[VITA-E3009] | 0 | 0 |
| E46 | `v inside {~4'b0?11}` | x | LOUD error[VITA-E3009] | 1 | 1 |
| E47 | `v inside {$unsigned(4'b1?00)}` | x | LOUD error[VITA-E3009] | 1 | 1 |
| E50 | `v inside {cf()}` | x | x | 1 | LOUD %Error-UNSUPPORTED; LOUD %Error |
| E60 | `s16 inside {"ab"}` | 1 | 1 | 1 | 1 |
| E61 | `s inside {"ab", "cd"}` | 1 | 1 |  | 1 |
| E62 | `r inside {1.5}` | 1 | 1 |  | 1 |
| E63 | `r inside {[1.0:2.0]}` | 1 | 1 | 1 | 1 |
| E64 | `v inside {2.0}` | 1 | 1 |  | 1 |
| E65 | `r inside {4'b1?00}` | 0 | 0 |  | 0 |
| E70 | `v inside {arr}` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] |  | 1 |
| E71 | `v inside {s.a}` | 1 | 1 | 1 | LOUD %Error-UNSUPPORTED; LOUD %Error |
| E72 | `s.a inside {4'b1?00}` | x | 1 | 1 | 1 |
| E73 | `arr[1] inside {4'b1?00}` | x | 1 | 1 | 1 |
| E74 | `bv inside {4'b1?00}` | x | 1 | 1 | 1 |
| E75 | `iv inside {4'b1?00}` | x | 1 | 1 | 1 |
| E76 | `b` | 0 | 1 | 1 | 1 |
| E77 | `w8` | 0000000x | 00000001 | 00000001 | 00000001 |
| E78 | `v[3:0] inside {4'b1?00}` | x | 1 | 1 | 1 |
| E79 | `{v[3:2], v[1:0]} inside {4'b1?00}` | x | 1 | 1 | 1 |
| E80 | `(v + 4'd0) inside {4'b1?00}` | x | 1 | 1 | 1 |
| E81 | `v inside {e, 4'b0?00}` | 1 | 1 | 1 | LOUD %Error-UNSUPPORTED; LOUD %Error |
| E82 | `v inside {{p, 2'b?0}}` | x | LOUD error[VITA-E3009] | 1 | LOUD %Error-UNSUPPORTED; LOUD %Error |
| E83 | `v inside {4'sb1?00}` | x | 1 | 1 | 1 |
| E84 | `v inside {4'b1?00}` | x | 1 | 1 | 1 |
| E85 | `$unsigned(v) inside {4'b1?00}` | x | 1 | 1 | 1 |
| E86 | `$signed(v[3:0]) inside {8'sb1111?100}` | 0 | 0 | 0 | 0 |
| E87 | `$signed(v[3:0]) inside {4'sb?100}` | x | 1 | 1 | 1 |
| E88 | `v inside {4'd1/4'd0}` |  | x |  |  |
| C01 | `` | else | then | then | then |
| C02 | `(v inside {4'b1?00}) ? 8'd1 : 8'd2` | 0X | 01 | 01 | 01 |
| C03 | `w` | x | 1 | 1 | 1 |
| C04 | `w8` | 0000000x | 00000001 | 00000001 | 00000001 |
| C05 | `f(v)` | x | 1 | 1 | 1 |
| C06 | `f(v)` | x | 1 | 1 | 1 |
| C07 | `r` | x | 1 | 1 | 1 |
| C08 | `w` | x | 1 | 1 | 1 |
| C09 | `L` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 1 |
| C10 | `L1` | LOUD error[VITA-E3009] | 1 | 1 | 1 |
| C11 | `L2` | LOUD error[VITA-E3009] | 1 | 1 | 1 |
| C12 | `L3` | LOUD error[VITA-E3009] | 1 | 1 | 1 |
| C13 | `` | LOUD error[VITA-E3010] | then | then | then |
| C14 | `` | default | item | item | item |
| C15 | `$time` | end | woke 1; end |  | woke 1; end |
| C16 | `` | fail | pass |  | pass |
| C17 | `$time` | fail 1; fail 3; fail 5; end | end | end | end |
| C18 | `ok, o.x` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] |  | 0 0000; 0 0000; 0 0000; 0 0000; 0 0000; 0 0000 |
| C19 | `ok, o.x` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] |  | 0 1000; 0 0011; 0 0011; 0 0110; 0 1001; 0 0011 |
| C20 | `m` | x | 1 | 1 | 1 |
| C21 | `v` | 1000 | 1001 | 1001 | 1001 |
| C22 | `n` | 0 | 2 | 2 | 2 |
| C23 | `y` | X | 1 | 1 | 1 |
| C24 | `$time` | end | end | end | end |
| C25 | `p::pf(v)` | x | 1 | 1 | 1 |
| C26 | `k.m(v)` | x | 1 |  | 1 |
| C27 | `L4` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | x | x |
| C28 | `$bits(w)` | 1 | 2 | 2 | 2 |
| C28b | `$bits(w), $bits(w2), $bits(w3), $bits(w4)` |  | 1 2 2 1 |  |  |
| C29 | `P` | LOUD error[VITA-E3009] | 1 | 1 | 1 |
| C30 | `f(v), LC` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 1 | 1 1 |
| C31 | `i` | LOUD error[VITA-E3010]; LOUD error[VITA-E3010] | i=0 in; i=1 in | i=0 in; i=1 in | i=0 in; i=1 in |
| C32 | `q[0] inside {4'b1?00}` | x | 1 |  | 1 |
| C33 | `` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | edge; end | edge; end |
| C34 | `w8, f(v), g(v)` | xx x xx; xx x 01 | 03 1 02; 02 1 01 |  | 03 1 02; 02 1 01 |
| Z01 | `u8 inside {4'bx100}` | Z01a x; Z01b 0; Z01c x; Z01d x; Z01e 0; Z01f x; Z01g 0; Z01h x; Z01i x; Z01j x; Z01k xxxx; Z01l x | Z01a 1; Z01b 0; Z01c 1; Z01d 1; Z01e 0; Z01f 1; Z01g 0; Z01h 1; Z01i 1; Z01j 0; Z01k 1111; Z01l 3 | Z01a 1; Z01b 0; Z01c 1; Z01d 1; Z01e 0; Z01f 1; Z01g 0; Z01h 1; Z01i 1; Z01j 0; Z01k 1111; Z01l 3 | Z01a 1; Z01b 0; Z01c 1; Z01d 1; Z01e 0; Z01f 1; Z01g 0; Z01h 1; Z01i 1; Z01j 0; Z01k 1111; Z01l 3 |
| Z02 | `u.sig inside {4'sb1?00}` | x | 1 | 1 | 1 |
| Z03 | `h inside {null, h2}, h2 inside {null}` | 1 0 | 1 0 |  | 1 0 |
| Z04 | `L` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 0 |
| Z05 | `L, L2` | 0 1 | 0 1 | 1 0 | 0 0 |
| Z06 | `i` |  | 0; 2 | 0; 2 | 0; 2 |
| Z07 | `r` |  X;  3 |  7;  3 |  7;  3 |  7;  3 |
