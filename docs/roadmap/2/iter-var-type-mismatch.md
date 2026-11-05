(§2 iter-var-type-mismatch · OPEN · in Ranges / bounds / selects) (an iteration variable smaller than the associative key is fit by value, not -1):
* An iteration variable of another type than an `[int]` / `[integer]` / `[byte]` / `[shortint]` / `[longint]` key: the engine's fit test is by value (`frame_eval.rs` `assoc_iter_compute`), where §7.9.8 returns −1 whenever the variable is smaller than the index type (c_iterk_int_byte, `[int]` with `byte k`: `f st=1 k=-3`, §7.9.8 `st=-1`), and `next`/`prev` read the variable's own value, not its cast to the key
* verilator refuses the mismatch ("Non-variable on LHS of built-in method 'first'")
* a typed key refuses it since §4.5.616
* hand-IEEE
* OPEN
