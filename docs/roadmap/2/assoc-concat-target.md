(§2 assoc-concat-target · OPEN · in Ranges / bounds / selects) (an associative element as a concatenation-target chunk is not written):
* An associative-array element as a chunk of a concatenation target (`{aa[5], x} = {32'd7, 4'hB};`) is not written: W4020 `assoc element write in an unsupported lvalue shape (ignored)` (`changes.rs` `dyn_write`; the key cannot ride the split's `(offset, word)` pairs)
* c_concat7 `a aa5=0 x=b`, verilator `a aa5=7 x=b`
* the read-back was `x` before §4.5.616 and is the `int` element's default 0 since (pinned `concat_with_an_assoc_chunk_stays_loud`), so both readings are wrong (wrong→wrong): concat_heap__84b26a4744 `{aa[5], x} = 8'hAB` and native_gate__208c8125f0 read `aa5=0` only because 0 is what was written
* verilator
* OPEN
