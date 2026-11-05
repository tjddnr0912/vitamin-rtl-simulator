(§5.2 Q-NETCAP · rank ② · source row 41 plan §16 F6) (the v1 net and variable count cap stops two OpenTitan pages):
* slot: page row
* slot: TW1 at row 41's landing, §4.5.634: as C rows the four page rows would put OT-12 at 19 against 16 at the last check, +3 where TW1 stops at +2, so the C slot is a fresh opus-plan's (W/program/q6-2026-10-31/REPLAN.txt D6)
* slot: `any` until then
* symptom: `total net/variable count exceeds the v1 cap (131072)` (`limits.rs` MAX_TOTAL_NETS) at `prim_subreg.sv:48`: rv_plic `u_reg.u_ip_2_p_64`, spi_device
* note: severity not measured, the cap truncates the stop list
