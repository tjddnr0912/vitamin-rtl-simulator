(§5.2 Q-PKGBLKLEAK · rank ① · source Q-AESINK review r1 (dC2)) (a package routine's block-local writes the enclosing name):
* a package routine's block-local writes the enclosing name: the module, `$unit` and interface lanes refuse the shape (block-local gate, E3009), `q::f` does not run that gate, and the interpreter's env is flat: j_pkg_in2s_seedw `logic [3:0] t; t = 4'd5; begin int t; end f = t;` `00000000`, all three `00000101`
* j_pkg_inf2s_after over a formal, `begin bit [3:0] a; end f = a;`, `00000000`, all three `00000010`
* sh_narrow_seed `logic [7:0] t = 8'hff; begin logic [3:0] t; end` then four bit writes, `00001111`, all three `11111111`
* sh_narrow_formal
* sh_b4w_nz_w_in `00000101`, all three `00001001`
* no seed read in any, so Q-AESINK's mark says nothing
* fix: the gate runs on package routine text, or the env takes block scopes
