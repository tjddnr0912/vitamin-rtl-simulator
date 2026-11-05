(§2 🆕 B · BLOCKED · in Start-order rows) (a `case` on a signed right shift with an unsigned label keeps the sign):
* ⓑ `case (b>>>2)` with an unsigned label: vita `eq236`, oracles `eq44`
* `stmt_flow.rs` outer `$unsigned`
* re-lower via `lower_size_ctx_entry(.., ext=false)`
* status: BLOCKED (sign provenance vs a default: `expr_self_signed`)
