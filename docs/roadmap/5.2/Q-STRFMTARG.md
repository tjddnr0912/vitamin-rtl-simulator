(§5.2 Q-STRFMTARG · rank ② · source row 26 r1 lens-diff b14 t03, b15 u01; Q-STRBYTES cell c24 G1) (a string variable as `$display`'s first argument is used as a format):
* symptom: a string variable as `$display`'s first argument is used as a format: `s = "S=%0d"; $display(s, x)` prints `S=7` (= iverilog 13.0 = sv2v), verilator 5.052 `S=%0d          7`
* witness: `$display("[", s, "]")` `[S=93` (= iverilog = sv2v), verilator `[S=%0d]`
* witness: a format built in a variable (`f = {"@G1 ", string'(8'hC8), " %0d\|"}; $display(f, 5)`) prints `@G1 \xef\xbf\xbd 5\|` (iverilog = sv2v `@G1 \xc8 5\|`), verilator `@G1 \xc8 %0d\|          5`
* note: IEEE 1800-2017 §21.2.1 (ieee1800-2017.txt:28576–28578: a string-typed argument with no format specification is a character string) and §21.2.1.2 (:28619–28621: format specifications live in string LITERAL arguments) side with verilator (ruled 2026-10-27)
* evidence: PRE 9d60771d
