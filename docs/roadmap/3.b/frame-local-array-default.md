(§3.b frame-local-array-default · OPEN · in Subroutine / frame) (`'{default: v}` onto a function-local unpacked array is refused):
* symptom: `'{default: v}` onto a function-local unpacked array is E3009 (`a keyed assignment pattern … is supported … as '{default: v} on an unpacked array`)
* every importing instance elaborates `prim_mubi_pkg`'s uncalled assert function (186 of uart's 200 capped diagnostics)
* the frame lowering fills the local's elements (nearest: `frame-array`)
* oracles: verilator and sv2v → iverilog `A o=0`
* witness: row-9 R08, every OpenTitan page
* status: OPEN (§5.2 row 33, PARKED 2026-11-04 behind Q-TYPEOVERVAL)
