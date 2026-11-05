(§5.2 Q-ESCSCOPE · rank ① · source Q-STRBYTES design review z14 M3, z11 M1 M2 (N1)) (`%m` keeps an escaped identifier's backslash):
* symptom: `%m` keeps an escaped identifier's backslash: `initial begin : \blk+1  $display("M3 [%m]");` prints `M3 [top.\blk+1]` (PRE 9d60771d and Q-STRBYTES §4.5.629 alike, native and vm) where iverilog 13.0, sv2v → iverilog and verilator 5.052 print `M3 [top.blk+1]`
* IEEE 1800-2017 §5.6.1 (ieee1800-2017.txt:2852–2853: the leading backslash and the terminating white space are not part of the identifier)
* witness: a non-ASCII escaped name the same (`begin : \bé` `%12m` `top.\b\xc3\xa9`, iverilog = sv2v `top.b\xc3\xa9`)
* fix: census the scope-name producers `%m` reads (the escaped spelling is kept for the source)
