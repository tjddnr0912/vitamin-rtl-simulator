(§3.b zero-repl-concat · OPEN · in Constants / parameters) (a zero-count replication inside a constant concatenation):
* a zero-count replication inside a larger constant concatenation (`{(7-idx){…}}` at idx=7, `lc_ctrl_pkg`'s `TEST_UNLOCKED(7)`) declines the whole concatenation (`package parameter TransTokenIdxMatrix value is not a foldable constant`)
* cause: `elaborate/src/const_wide.rs:599–602` under `fold_concat_parts` (:1169)
* IEEE 1800-2017 §11.4.12.1 allows it beside a positive-width part (alone it stays loud)
* all three `A m=a5c l=bc3`
* witness: row-9 R29
* status: OPEN (§5.2 row 40)
