(§2 enum-method-width · OPEN · in Size cast / signedness) (an enum method's result is 32 bits instead of the enum's width):
* an enum method's result (`next()`, `prev()`, `first()`, `last()`, `next(N)`) is 32 bits wide instead of the enum's, so a self-determined use reads it wide: `{2'b11, e.next()}` = `300000002` and `$bits(e.next())` = 32 on a module's own `logic [1:0]` enum variable (fp.enum_width_mod_ctl; iverilog and verilator `e` / `2`), `{~e.next()}` = `fffffffd` (verilator `1`), the same in a class method (fp.enum_width_samemod `cn=02 bn=32`, verilator `0e` / `2`)
* `num()` is `int` and right
* comparisons, `case` and a wider assignment are right by value
* the desugar emits untyped integer literals and a ternary chain (`enums.rs` `enum_method_expr`)
* a `$unit` class's enum property replayed into a module (row 67a's cross-container lane, `extends` and the with-block) keeps it refused until this closes (`class_scope.rs` pins, matrix row67a/enumw: on the `$unit` lane 17 of 42 cells wrong, the in-module and module-variable lanes the same shapes)
* near §0 10ⓐ (move the enum-method desugar into elaborate)
* no T-stage row
* 2 oracles
* OPEN
