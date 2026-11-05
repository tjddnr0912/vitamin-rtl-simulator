(§5.2 Q-LOCSTALE · rank ① · source Q-WITHREAL grounding O6; Q-WITHREAL r2 lens-diff C2) (an array locator whose `with` reads a variable returns every index (native)):
* slot: engine, native lane
* slot: shares slot 13 with Q-XNONINT's string half, the first lane-1 slot with no row after Q-DLYCAT
* slot: no prerequisite, and its files (`sim-engine/src/builtins/`) are disjoint from row 74's, so it may start while 74 is under way
* slot: before Q-WITHREAL resumes
* a locator whose `with` expression reads a VARIABLE returns every index on the native lane: `int id[] = '{1, 2, 3}; iv = 1; qi = id.find_index() with (item > iv)` native `'{0, 1, 2}`, vm = interp = hand `'{1, 2}`
* also `find_first_index()` (native 0, hand 1) and a real array with a real variable (`rd.find_index() with (item > r)`, r = 2.0)
* a literal compare (`item > 1`) is right on every lane
* PRE 01cac4b3 (cells o6_int, g_m_loc_rd_gtr, lg_cmpint)
* statement task `SysTaskId::ArrLocator` (`sim-ir/src/lib.rs:671–679`)
* root (Q-WITHREAL re-plan Q3): `sim-engine/src/builtins/radix_arr.rs:288` `sched.truthy(pred)` evaluates the predicate over `sched.st`, and `builtins/dispatch.rs:436–438` calls `arr_locator(sched, args)` without the reader `dispatch_with` threads, so on native a variable in the predicate reads 0, declaration-initialized or not (native `kernel.rs` `systask_refusal` does not refuse it)
* the reduction twin reads through the threaded reader (`eval/sysfunc.rs:200–216`) and is right
* measured (`W/qwithreal/replan3/q3_table.txt`, cell q3_loc, PRE = Q-WITHREAL r2): 6 / 6 variable-reading locator lines wrong on native, vm = interp = hand — `item > iv` (iv assigned) `3 0` (hand `2 1`), `int jv = 1` (declaration init) `3 0` (`2 1`), both after `iv = 2` `3 0` (`1 2`), `item == kk - 3` `0 x` (`1 1`), `item > rr` `2 0` (`1 1`)
* right on every lane: a literal compare, `id.sum() with (item > iv ? item : 0)`
* fix: thread `nets: Option<&N>` into `arr_locator` and evaluate the predicate and `swap_array_item` through it, as `eval_task_arg_ctx` (`builtins/queues_io.rs:889`) does
* Q-WITHREAL's g3_lo_fx and h5_t11 then move wrong → right on every lane
* source: re-plan Q3 (W/qwithreal/REPLAN3.txt)
