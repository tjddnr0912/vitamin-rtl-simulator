(§5.2 Q-TDREDECL · rank ②) (a type name declared twice in one package is accepted):
* symptom: a type name declared twice in one package is accepted where both oracles refuse it: `package p; typedef struct {logic [3:0] a; logic [3:0] b;} T; typedef struct packed {logic [3:0] a; logic [3:0] b;} T; endpackage` with `import p::T; T v; v = T'{4'h1, 4'h2};` prints `A m23 v=12` (row66.m23) where iverilog 13.0 refuses (`Type identifier T has already been declared`) and verilator 5.052 refuses (`Duplicate declaration of TYPEDEF`)
* witness: an enum `t` then a packed struct `t` in p, `p::t v; v = v.first();` `A dr06b 0 A0` (grid2.dr06b), both refuse alike
* cause: the newest registration decides, and the enum slot survives the struct item
* note: an explicit import then a local typedef of the name is §2's struct-binding-by-name (w2stale.pke.ps.mem)
* §5.2 row 41's holder reads what its flat twin reads on such source, admitted: a page vector after a `logic signed`, an `enum bit` or an unpacked record of its name, a struct element after a page vector of its name, a page vector declared twice (vita the last, sv2v the first) and a wildcard-imported name referenced, then declared (IEEE 26.3), each holder line = its flat twin's on PRE (W/row41/lens3-sound/sets/s2 d01 d04 d06 d07 d11, W/row41/lens3-diff/d1h/h3_redecl_page; iverilog and verilator refuse each)
* fix: refuse a second type declaration of a name in one scope
* status: silent-wrong on invalid code, pre-existing (PRE 01cac4b3 = row 66 POST)
* source: row 66 battery M23 / M28 discriminators (W/row66/r1/bat/disc m23, W/row66/grid2/dr06b; pins `type_record.rs` m23 m28, KNOWN-WRONG)
