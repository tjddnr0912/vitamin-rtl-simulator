(§2 Fabricated width · BLOCKED · in Size cast / signedness) (a cast over a method call or array reduction zero-extends):
* `40'(u1.f(0))`, `longint'(q.sum())` zero-extend
* also over a byte queue `bq` and `int ib[] = '{2**30, 2**30}`: `16'(bq.sum() + 8'd10)` xx07 (verilator 5.052 = hand 0107), `16'(bq[0] + 8'd10)` 0009 (0109), `longint'(ib.sum() * 2)` 0 (verilator -4294967296), `u12_t'(bq.sum())` x02 (hand 002), `u12_t'(a8 + bq.sum())` / `12'(a8 + bq.sum())` xxx (hand 101): no size-context predicate has an array-method arm (`ctx_signed_impl` / `size_ctx_self_width` / `has_opaque_leaf` / `select_chain`; PRE 01cac4b3, Q-WITHREAL r2 lens-sound s1_bare, lens-diff g7 TI_qs, TI_qs2)
* the with-form joins when Q-WITHREAL resumes
* status: BLOCKED (declared width for reduction / string / placeholder operands: §5.2 row 74)
