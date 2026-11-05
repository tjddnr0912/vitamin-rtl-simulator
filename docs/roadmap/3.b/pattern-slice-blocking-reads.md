(§3.b pattern-slice-blocking-reads · OPEN · in System tasks & file I/O) (a blocking pattern into an array slice that reads variables is refused):
* symptom: a BLOCKING assignment pattern into an unpacked-array slice whose elements or leading index read any variable (`x[2:1] = '{8'h05, w}`) is E3009 (`arrays/pattern.rs`: every element and the leading index must lower to a compile-time constant, `index_is_compile_time_constant`)
* the element-wise write let a later element read an earlier one's new value under every spelling the §4.5.620 reviews tried (a cast, `top.x`, a `let`, `p::x`, a net a continuous assign drives from the target)
* lower it as one multi-chunk write that evaluates the right-hand side and the indices once, as the concatenation lane does (`{x[2], x[1]} = {8'h05, w}` prints `05 03 06` in vita and all three oracles)
* oracles: verilator contradicts itself between the two spellings (pattern `05 05 06`), so sv2v and hand-IEEE §10.4.1 `05 03 06`
* status: OPEN
