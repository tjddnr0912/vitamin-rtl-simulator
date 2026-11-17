(§5.2 Q-MDRVWHO · rank — · source W/wrext/fix1/PLAN3.txt section 6, Q-MDRVMIX's park (2026-11-17)) (the write record carries its writer's scope):
* purpose: the write view records, for each write, the scope that wrote it, so an E3001 text can name what Q-MDRVMIX's reduced text loses: the writer's instance, the routine's name and where it is declared, and the call position of an inlined statement; no IEEE clause of its own (the text of §6.5 and §9.2.2.2's error)
* why: no goal; off the chain unless a design of the set needs it (an E3001 on a set design whose reduced text cannot say which instance or routine wrote the bits)
* needs: Q-MDRVMIX
* readers: Q-MDRVMIX (its text)
* state: WAITS Q-MDRVMIX (2026-11-17)
* record: as filed below ; superseded order lines are kept for the record
* slot: filed 2026-11-17 at Q-MDRVMIX's park, held until Q-MDRVMIX lands
* the words lost (PLAN3.txt section 6): the instance that wrote (a port of an instance array or a generate loop driving one outer variable reads apart only by scope); the routine's name and its declaration position (a frame body becomes "a subroutine body (in the statement at ..)", whose position lies inside the routine, losing its name and which instance's copy); the process block's position (replaced by the statement's); the call site of an inlined task's statement; a declaration initializer keeps no position
* the shape (PLAN3.txt): a scope fact written at Q-WEXTPOS's two recorders (the lowering's instance, by producer: `ports.rs` sets `cur_prefix = child_prefix` before its push, the declaration initializer sets the flush key) and a declaring-scope fact from `push_func` (absent for a `$unit` or package routine: §2 pctm-unit-routine); it is the by-producer census PLAN3's option (F) needed, which the reduced text avoids
* source: W/wrext/fix1/PLAN3.txt sections 1 and 6, premise3/REPORT.txt; Q-MDRVMIX.md, the park
