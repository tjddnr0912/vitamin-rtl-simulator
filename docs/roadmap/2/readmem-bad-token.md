(§2 readmem-bad-token · OPEN · in Ranges / bounds / selects) (the fixed-memory `$readmem*` loader skips bad characters silently):
* The fixed-memory `$readmem*` loader skips a character that is not a digit inside a data word and carries on after a bad `@` token at the previous address (`crv_draw.rs` `parse_mem_token`, `readmem`): fx_badword `@4 11 g2 33` loads `m[5]=02 m[6]=33`, fx_badaddr `@1g 11 @4 22` loads `m[0]=11 m[4]=22`, each with no warning for the word (iverilog 13: `Invalid input character: g`, loading only `m[4]=11`, and nothing for the bad address; verilator aborts the run)
* the associative-array loader stops at the bad token with W4023 since §4.5.617
* oracles: iverilog
* status: OPEN
