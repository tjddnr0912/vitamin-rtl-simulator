(§5.2 Q-PKGUNIQ · rank ② · source row 41 plan §16 F5) (a package call reaching a `unique` violation is refused, on four OpenTitan pages):
* slot: page row
* slot: TW1 at row 41's landing, §4.5.634: as C rows the four page rows would put OT-12 at 19 against 16 at the last check, +3 where TW1 stops at +2, so the C slot is a fresh opus-plan's (W/program/q6-2026-10-31/REPLAN.txt D6)
* slot: `any` until then
* scope: §3.b unique-pkg-closure, with a page line: `hmac_reg_top.sv:118` (`tlul_pkg::get_bad_cmd_intg`, `get_bad_data_intg` → `get_cmd_intg`, `get_data_intg`, `tlul_pkg.sv:196–207`, `tlul_socket_1n.sv:148`)
* scope: hmac, usbdev, spi_host, spi_device
