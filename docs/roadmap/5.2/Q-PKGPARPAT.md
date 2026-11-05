(§5.2 Q-PKGPARPAT · rank ② · source row 41 plan §16 F4) (a package parameter with a keyed struct pattern value is not folded):
* slot: page row
* slot: TW1 at row 41's landing, §4.5.634: as C rows the four page rows would put OT-12 at 19 against 16 at the last check, +3 where TW1 stops at +2, so the C slot is a fresh opus-plan's (W/program/q6-2026-10-31/REPLAN.txt D6)
* slot: `any` until then
* symptom: `keymgr_dpe_pkg.sv:234 / 239` `parameter hw_key_req_t D = '{valid: 1'b0, key: {Shares{KeyWidth'(32'hDEADBEEF)}}}` is E3009 `package parameter .. value is not a foldable constant` (W/row41/r1/filed/w_pkgpar.sv: sv2v and verilator `A d=0beefbeef`, iverilog syntax error)
* scope: hmac's file list only
