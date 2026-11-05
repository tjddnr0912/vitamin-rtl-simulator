(§3.b typed-pattern-residue · OPEN · in Parser accept) (typed assignment patterns `T'{…}` beyond an unsigned packed struct):
* a typed assignment pattern `T'{…}` runs for an unsigned packed struct `T` (§4.5.609) and is refused by name (E2002) for: a `struct packed signed` `T`, where the oracles split wherever its sign is read (beside a signed `?:` arm verilator `e=00111`, sv2v → iverilog `e=11111`; `%0d` verilator `v=30`, sv2v → iverilog `v=-2`)
* a union (verilator `u=3`, sv2v refuses)
* an unpacked struct (verilator and sv2v → iverilog `A a=3 b=7`)
* an unpacked array (verilator `a0=1 a1=2`)
* a packed vector type (both `v=12`)
* a struct laid out per instance (both `r=01011`)
* a replicated `T'{N{e}}` (both `r=1010`, the untyped form is loud too)
* a `T` whose newest registration where the pattern stands — every typedef, unpacked record, type parameter and import of the name draws one number — is another kind of type, or an alias of such a name (x09 verilator and sv2v → iverilog `o=0003fffe b=17`; h01 `bits=16`, o46 `a=9 b=7`; the layout maps stay name-keyed, §2 struct-binding-by-name)
* a nested `'{…}` for a packed-union member (verilator and sv2v → iverilog `w=067`; §2 union-member-pattern)
* a select after the pattern (`T'{…}[7:4]`, both refuse)
* oracles: iverilog has no typed-pattern syntax
* status: OPEN
