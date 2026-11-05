(§3.b late-target-capture · OPEN · in Constant edges (§4.5.601)) (fills and delayed captures into a target whose width folds after elaboration):
* an assignment whose target width folds only after elaboration is E3009 for an `'x`/`'z` fill reached by the assignment context (i18 `a[u.W-1:0] = 'x`, j05, j06, b15, rc21) and for an intra-assignment capture whose right-hand side is narrower than the target and whose top region is signed, holds a `'1`, `'x` or `'z` fill, or has `+ - * ** << <<< ~^ ^~` or unary `~`/`-` (e27 `a[gb.L-1:0] = #1 4'hf + 4'h1` `10`, b05 `= #1 '1` `1f`, ia6 a signed 4-bit `ff`, `u.g = #1 4'hf + 4'h1` `10`; a `'0` capture zero-extends exactly and runs) (§4.5.601 sizes such a capture at the right-hand side's own width)
* fix: size the fill and the capture temp at the width decided after the deferred passes (patch the temp net, census its readers first)
* oracles: 2 oracles
* status: OPEN
