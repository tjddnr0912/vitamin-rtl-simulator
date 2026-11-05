(§3.b eh1-assert-on · OPEN · in Parser accept) (VeeR EH1 with `ASSERT_ON` stops at its concurrent properties):
* symptom: VeeR EH1 under its default `ASSERT_ON` parses past its deferred module-item assertions since §4.5.606 and stops at elaborate: 25 `E3009` in `dma_ctrl.sv`'s concurrent properties (`$past` argument must be a simple signal) and `veer.sv:998` (a sequence consequent must be a single bounded boolean sequence)
* measured on the row-9 workaround RTL with `snapshots/default`
* the EH1 corpus row runs `ASSERT_ON` off or needs these
* status: OPEN
