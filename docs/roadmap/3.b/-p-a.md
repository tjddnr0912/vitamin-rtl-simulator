(§3.b %p-ⓐ · OPEN · in System tasks & file I/O) (`%p` of a packable unpacked struct prints a flat vector):
* a packable unpacked struct declares as its flat vector (`hdl-parser/src/structs/decl.rs:466–505`): `%p` of `'{8'h41, 16'h4243}` prints `4276803`, `%h` `414243` (verilator `'{a:'h41, b:'h4243}`)
* `%u` / `%z` → §5.2 Q-PCTUREC
* `string sa[2]` is E3010 at a whole-array use
* oracles: verilator
* status: OPEN
