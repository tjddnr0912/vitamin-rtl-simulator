(§5.2 Q-INLITEM · rank ① · source Q-WITHREAL plan O3 (K2 premise census; cells k2_*, k2i_*, k2l_*)) (an inlined function inside a `with` clause shares the caller's `item`):
* slot: after Q-WITHREAL, its residue
* slot: parked with it
* a static straight-line function inlined inside a `with` clause shares the caller's iterator with its own `with` clauses: the actual's ExprId is bound into the callee body (`elaborate/src/inline_fn.rs:735–737` `self.subst.push((p.name.name.clone(), bound))`) and the engine reads `item` from the innermost ACTIVE fold (`sim-engine/src/eval/eval_core.rs:636–638`), so the caller's `item` is read under the callee's fold.
* witness: Over `real rd[] = '{1.5, 2.5}`, `int id[] = '{10, 20}`, `function real f(real x); f = id.sum() with (item + x); endfunction`: `rd.sum() with (f(item))` 120.0 (hand IEEE 68.0; `f(1.5)` alone 33.0 = hand)
* a body local `t = x * 2.0` 180.0 (76.0)
* a locator `rd.find_index() with (f(item) > 34.0)` `'{0, 1}` (`'{1}`)
* integral twins `function integer h(integer x); h = id.sum() with (item * x);` `rd.sum() with (h(item))` 1000 (150), `id.sum() with (h(item))` 1000 (900; verilator 5.052 = hand on these), `fi(item.index)` 40 (30).
* The iterator NAME also leaks into the inlined body: with a module `integer item = 7`, `function integer g6(integer k); g6 = k + item;` `id.sum() with (g6(1))` 32 (hand 16; `g6(1)` alone 8), the real twin 8.0 (18.0).
* The frame route (automatic, 2-state return, non-straight body) is right on every probe.
* PRE 01cac4b3 = the parked r2 patch on every line
* fix direction: clear `array_iter` / `array_iter_real` while an inlined body is lowered and bind an actual that reads `item` through a temporary (or frame such callees)
* `inline_body_ctx.rs`'s `K::ArrayMethodWith → real domain` (:95–235) is owned here
