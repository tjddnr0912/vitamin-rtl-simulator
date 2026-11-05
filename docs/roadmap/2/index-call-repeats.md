(§2 index-call-repeats · OPEN · in Index sealing) (an index holding a call or `$urandom` is evaluated more than once):
* `gp[0][$urandom] = 1` draws twice (`array_word_index_domain.rs`), and the element write `m[f(2)] = 4'hA` over `logic [15:0][3:0]` calls `f` three times (§4.5.587 probe p5, iverilog once; e_sel2: a `unique` miss in `f` reports W4031 three times, both oracles once)
* iverilog
* OPEN
