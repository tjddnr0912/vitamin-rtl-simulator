# §4.5.592 lens DIFFERENTIAL round 1 — REPORT (final)
round: 1 · binaries PRE $S/s592/pre/vita md5 86a2d84a21d0329c57541e51ae9eb4d2, POST $S/s592/post_b/vita md5 020de73fcf94cad012aff0f7a3a7db62 (release; staged pre/sep, post_b/sep)
cells: 40 designed ($L/c/D*.sv; raw $L/c/<id>.<iv|sv|vl|pre|post>.txt); harness $L/run.py, $L/staged2.py, $L/corpus_velab.py
lens verdict: FAIL (product shakes: F1 correct->loud on IEEE-legal designs)

## Findings (most severe first)
F1 BLOCKING (new instance; PLAN risk line called the class "contrived; 0 in suite/corpus", never measured into the "DOWN 0" tally)
  correct->loud on an IEEE-legal shadowed forward reference whose disagreeing later-walk arm/iterations hold only Nets-walk content.
  DL08 (outer N=1; block g: for i<N {wire [3:0] w}; later localparam N=3; $display($bits(g.L[0].w))):
    iv @bits=4 · sv2v @bits=4 · verilator @bits=4 · PRE @bits=4 rc0 · POST rc1 "DL08.sv:4:28: error[VITA-E3010] ... `N` is used before it is declared in this generate block (declared at 7:24) ..."
  DL01 ($bits(g.b.w), if-arms nets only): iv @bits=4 · sv2v @bits=4 · vl "Can't find definition of 'b'" · PRE @bits=4 · POST E3010
  DL03 (arms localparam only, g.b.Q): iv @Q=10 · sv2v @Q=10 · vl ERR · PRE @Q=10 · POST E3010
  DL06 (case arms variables only): iv/sv2v/PRE @bits=4 · POST E3010.  DL11 (hier assign g.b.w=9): iv/PRE @w=9 bits=4 · POST E3010
  IEEE §26.3: reference binds the outer object (legal; Example 1). ER §2.1 "Making a working construct loud is a regression". 4-way: real gap introduced by POST. Attribution: PRE correct on all five.
F2 MAJOR (false code comment + killable survivor) ForStep::Unknown doc says "No measured design reaches it (... an inner real, wide or unfoldable localparam never displaces the outer integer the Nets walk read, so every walk folds alike)"; IMPL says M12 "lane unreached by any probe".
  DM07b (outer localparam [63:0] S=2**40; for (i=S[41:40]; i<1000; i+=500); later localparam integer S=3):
    iv @L1 @L501 · sv2v ERR "initialization expression cannot have undefined bits" · vl @L0 @L500 · PRE rc0 no output (later-walk init unfoldable -> silent return) · POST E3010
  => the init-Unknown arm is reached; M12 (arm -> old silent exit) = PRE's silent-wrong; DM07b kills it; no pin exists. DM07 (i<3) same.
F3 MINOR new message drops the file of "(declared at L:C)": DM10 (forward K in an `include) prints "DM10.sv:4:9: ... (declared at 1:24)" — 1:24 is in DM10_inc.svh.
F4 MINOR advice on legal shadowed designs ("declare it above the generate for") changes IEEE meaning (DL08: 1 iteration -> 3).
F5 MINOR residue (pre-existing, PRE=POST) DO02: #(.P(K)) with forward generate K steers the CHILD's arm silently: iv "@one top.g.u.b bits=4"; sv2v/vl/PRE/POST "@two top.g.u.a bits=8". Not a mix, D1 cannot see it; list as a sink in the new Scoping reads row.
Pre-existing, not this slice: DF04 ($unit f vs region f after use) iv/sv2v/vl @region, PRE=POST @unit. DR01 recursive module E3009 (3 oracles y=1). DF01-09 illegal by IEEE §13.4.3 ("Constant functions shall not be declared inside a generate block"): vl refuses, iv/sv2v/PRE/POST accept alike.

## Clean
forward function calls never fire E3010 (DF01-09 PRE=POST); hierarchical/interface-port names in decisions refused before D1 in PRE and POST (DH01-04, DP01-03); interface generate E3009 both (DI01/03/05); later net/typedef/enum label (DN01-03) PRE=POST=iv; sibling leak DS01, child func-table DT01 PRE=POST=3 oracles; DM08/DM09 PRE silent -> POST loud (UP); DI05m/DX03/DM10 value = known V04 class.
staged==one-shot PRE 40/40, POST 40/40; POST rerun 40/40 byte-identical; corpus+examples .vu/.velab 30/30 byte-identical PRE vs POST (all vcmp/velab rc0); no panic.

# ROUND 3 (delta, post_d md5 5119bb026cab5cde3875b0c7b2208722) — in progress
census: later-walk arms = VarInit{NetVar var-kind+init} Logic{ContAssign, NetVar net+init, Proc} Instances{Instance}; guards of collect_var_init_drivers / elaborate_net_init_drivers match gen_builds_in exactly.
hole candidate: `(_, Param)` arm calls check_param_decl_range in EVERY walk (not Nets-gated) -> reports from a builds-nothing item; testing R3q (forward label, no outer).
round-3 results (post_d): r1 40 cells: same 33 (DL01/03/06/08/11 = PRE = iv), UP 7 (DI05m DM07 DM07b DM08 DM09 DM10 DX03), DOWN 0. staged==one-shot post_d 50/50, PRE 10/10 (new).
R3 new cells (10): 
 R3q BLOCKING NEW ROOT: forward label K (no outer) + label arm holds only `localparam [Nope:0] Q` + default arm net only: iv/sv2v/vl/PRE refuse, post_d `@bits=4` rc0 (loud->silent). check_param_decl_range runs in every walk (Param arm) = a builds-nothing item that REPORTS; silent path skips the own arm; Nets decision came from the silent label skip (T/AC sink). Round-2 skipped-label flag would have kept it loud.
 R3q2 (outer K=5, legal): iv @bits=4, PRE E3009 (spurious), post_d @bits=4 -> UP under IEEE.
 R3a (for body: nested if(i==0) initial) iv/sv/vl/PRE @L0, post_d E3010; R3b identical `initial $display("@same")` both arms: iv/sv/vl/PRE @same, post_d E3010; R3m dead nested if(0) in own arm: iv/sv/PRE @bits=4, post_d E3010 -> correct->loud, SAME ROOT as r1 F1 (syntactic conservative positive set).
 R3d genvar+typedef+net: = PRE = iv (silent ok). R3e assert property: iv unsupported, vl/PRE @a_fail (non-IEEE arm), post_d E3010 (Proc counted) -> UP. R3h interface instance: PRE E3009, post_d E3010 (Instance counted) sideways. R3n g.a.Q: iv refuses, sv/vl/PRE Qa=20, post_d undeclared-hier loud -> UP. R3c hierarchical fn call: E3009 both (unreachable).
round-3 lens verdict: FAIL (R3q).
