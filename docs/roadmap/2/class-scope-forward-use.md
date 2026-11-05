(§2 class-scope forward use · OPEN · in Class fields) (a name in a class body binds where it is parsed, not in the class scope):
* a class body is a scope (§4.5.610), but a name binds where it is parsed: a method reading a property declared after it in the class body reads the enclosing scope's variable of that name (c.prop_after_method: vita `ga=0d`, verilator `ga=16`), and a derived class declared before its base without `typedef class` (IEEE-illegal, accepted) gets no base record to replay (rv.ci1: vita `ga=0d`, verilator `ga=16`)
* the module's own `s.a` is right in both (`a=5`)
* pinned as recorded known-wrong, not asserted (`class_scope.rs` `class_scope_forward_use_keeps_the_module_variable`)
* oracles: verilator
* status: OPEN
