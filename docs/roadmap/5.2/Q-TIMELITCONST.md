(§5.2 Q-TIMELITCONST · rank ① · source Q-DECLSELBOUND plan 3 T1; slot-9 re-order probe) (a time literal in an integral constant folds as an integer):
* slot: after Q-REALSPLIT, row 48's real hand-off, parked behind row 74
* slot: the slot is kept while it waits
* a time literal in an integral constant expression folds as an integer in the module's time unit (`elaborate/src/const_fn.rs:266`, `const_eval_in_scope`'s `TimeLit` arm), where IEEE 1800-2017 §5.8 (ieee1800-2017.txt:3126–3127) reads it as a realtime value scaled to the current time unit, which makes its operator real (§11.8.2, :12689–12691; a relational operator compares two reals, §11.4.4, :11660) and an x / z bit of the other operand 0 on the conversion (§6.12.2, :4398–4399): `logic [((1ns < 4'bx) + 1):0] d;` PRE `$bits(d)` = 1 (the integer fold reads x and the bound takes the one-bit default) where iverilog 13.0, sv2v → iverilog and verilator 5.052 give 2, with no timescale and under a 1ns/1ns timescale alike
* a literal finer than the module unit declines: under a 1s/1s timescale `[(1ns + 1):0]` PRE 1, iverilog = sv2v → iverilog 2 (verilator: "Value too wide for 32-bits")
* not a witness: with no timescale `[(1ns + 1):0]` PRE 3 against iverilog's 2 is the default unit, implementation-specific (§3.14.2.3, ieee1800-2017.txt:2339; vita 1 ns, iverilog 1 s): under a 1ns/1ns timescale iverilog gives 3
* `[(2s * 4'bx):0]` all three n=1 = PRE (the default)
* §2 "A time literal in an inline region folds at 64 bits" reads the same leaf
* PRE 9d60771d (plan 3 tl01–tl06), 01cac4b3 (re-order probe tu03, tu04)
* row 48 r3 (W/row48/DESIGN3.txt R3-1), under a 1ns/1ns timescale: `localparam int T1 = 2ns / 4;` PRE 0 and `(3ns / 2) > 1` PRE 0 where iverilog and verilator give 1 (sv2v → iverilog 0)
* the fix must not hand the literal to today's real evaluator, which folds a region that mentions a real whole in f64 (`((1.0 < 2) + 2) / 2` is 2, all three 1), so `((1ns < 2) + 2) / 2`, 1 today on all four, would turn wrong
* review r3 (R3-F6), other timescales: under 1ps/1ps `3ns / 2000`, `(3ns / 2000) > 1` and `2500ps / 1000` PRE 1, 0, 2 where iverilog and verilator give 2, 1, 3 (sv2v → iverilog 0, 0, 0: not an oracle for a time literal), under 1ns/1ps `2ns / 4` and `(2ns / 4) > 0` PRE 0, 0 (1, 1) and `1500ps * 2` E3009 (3)
* row 48 r4: an untyped `parameter PT = 3ns / 2;` binds the integral 1 (`PT * 2` 2, a `localparam real` copy 1.000; iverilog and verilator 3, 1.500: §5.8 realtime, §6.20.2 real)
* silent-wrong, pre-existing
