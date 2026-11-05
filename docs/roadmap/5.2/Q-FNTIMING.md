(§5.2 Q-FNTIMING · rank — · source ENG-2 r4 lens-diff N3, r4 lens-sound N-MF5, r5 lens-sound N-MF5L) (a function body accepts a timing control or a task enable):
* a function body accepts a timing control or a task enable: `function automatic void f(); @(*) y = a; endfunction` runs (B14 `A end y=3` on every vita binary; iv "functions cannot have event statements", vl `%Error-FUNCTIMECTL`), and a task enable inside a function runs (K1 `assert (c) else u.tk(m);`, K2 `u.tk(m)`, K3 `st();` in a function called from `always_comb`; iv "Functions cannot enable/call tasks", vl "Functions cannot invoke tasks (IEEE 13.4)")
* status: pre-existing (PRE = POST)
* IEEE 1800-2017 §13.4 :14443-14446 (a function shall not contain a time-controlled statement, shall not enable a task)
* fix: refuse at elaborate
* pins: a task enable inside a function's `fork ... join_none` is legal (§13.4.4 :14686-14692) and stays accepted (MK1 MK3, pinned in `implicit_read_set.rs`, the killers of mutant MF5)
