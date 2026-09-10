# 08 · Timescale and Precision

How a module's time unit and precision are resolved, how a `#delay` becomes an integer
number of simulation ticks, how time is reported back to the design, and how all of that
survives the staged artifact path. The scheduler that consumes the resulting ticks is
[06-simulation-engine.md](06-simulation-engine.md).

Time is a correctness property, not a convenience. A transition placed one precision unit
off can hide a setup or hold violation, invert the order of two events, and write the
wrong timestamp into a waveform — all at exit 0, with nothing to notice. Accumulating
time in floating point makes exactly that failure inevitable: a value like 0.1 has no
exact IEEE-754 binary representation, and after enough additions the low bits of the
current time are gone. vitamin therefore keeps simulation time in a single 64-bit integer
and does every conversion at a defined rounding point.

---

## 1. `` `timescale unit/precision ``

```verilog
`timescale <time_unit>/<time_precision>
```

Each side is a mantissa and a unit. Both are mandatory.

| Field | Mantissa | Unit |
|---|---|---|
| `time_unit` | `1`, `10`, `100` | `s`, `ms`, `us`, `ns`, `ps`, `fs` |
| `time_precision` | `1`, `10`, `100` | `s`, `ms`, `us`, `ns`, `ps`, `fs` |

Internally each side is a base-10 exponent of seconds: `s` is 0, `ms` −3, `us` −6, `ns`
−9, `ps` −12, `fs` −15, plus 0, 1 or 2 for the mantissa. So `100ps` is −10 and `10ns` is
−8.

| Rule | Behaviour |
|---|---|
| Mantissa other than 1, 10, 100 | `E-PP-BAD-DIRECTIVE` |
| Precision coarser than unit | `E-PP-BAD-DIRECTIVE`: *time_precision (…) coarser than time_unit (…)* |
| Placement | a compiler directive, outside any module declaration |

```verilog
`timescale 1ns/1ps     // the common RTL verification pairing
`timescale 10ns/100ps
`timescale 1us/1ns     // a slow interface model
```

---

## 2. Which timescale governs a module

A `` `timescale `` applies to **every module that follows it in the expanded text**,
until the next one. The preprocessor records each directive's offset in expanded-text
coordinates; a module is governed by the **last region whose offset is at or before the
module's own span start**. A module that precedes every region uses the base.

| Situation | Result |
|---|---|
| The design contains no `` `timescale `` anywhere | every module takes the base **1ns/1ns**, and the run emits `W-PP-TIMESCALE-DEFAULT` (`VITA-W1017`): *no \`timescale in the design; assuming the 1ns/1ns base* |
| Some modules are governed and some are not | the ungoverned modules take the 1ns/1ns base, and the run emits `W-PP-TIMESCALE-MIXED` (`VITA-W1018`), naming up to eight of them and then `(and N more)` |
| Every module is governed | no diagnostic |

IEEE 1800 §3.14.2.2 makes the mixed form an error, and other tools refuse to elaborate
it. vitamin runs it — the design does have a well-defined answer here, and Icarus Verilog
accepts it too — but says so, and names the modules, so the warning is actionable. A CI
that wants the standard's severity gets it with `-Werror=W-PP-TIMESCALE-MIXED`.

The base is a constant, independent of platform and of compilation order, so it does not
weaken the byte-identical-output contract or the artifact staleness hash.

### 2.1 File order and its guards

The rule is order-sensitive by construction, and that is a real hazard: the same file
compiled after a different neighbour inherits a different unit.

```
file A: `timescale 1ns/1ps
        module fast_logic … endmodule

file B: module no_timescale_here … endmodule
        └ compiled after A → governed by 1ns/1ps
        └ compiled alone   → the 1ns/1ns base + W-PP-TIMESCALE-DEFAULT
```

Two guards bound the damage:

- The mixed-specification warning above fires whenever the design is order-sensitive in
  the way that matters.
- In the filelist path, when the same source appears twice and the duplicate is dropped,
  the sticky `` `timescale `` state each occurrence would have inherited is compared. A
  difference is a hard `E-FLIST-DUP-CTX-CONFLICT` (`VITA-E8003`) rather than a silent
  pick: *`{tok}` included twice under differing sticky context: first (`{first}`)
  inherits `X`, duplicate inherits `Y`*, where an absent context renders as
  `(base 1ns/1ns)`.

`` `resetall `` strips its own token and nothing else. It does **not** reset the
timescale, macros or `` `default_nettype ``.

### 2.2 In-module declarations

> **Status at HEAD.** `timeunit` and `timeprecision` are not implemented. Neither word
> appears in the lexer, the parser or elaborate, so it lexes as an ordinary identifier
> and the declaration dies at `VITA-E2002` (`E-PARSE-UNEXPECTED-TOKEN`). The
> preprocessor directive is the supported channel for setting a module's time unit and
> precision.

There is no `--timescale` command-line override either. The design's own directives are
the only input.

---

## 3. The global tick

A mixed-timescale design cannot store time in per-module units. vitamin picks one grain
for the whole design:

```
global_prec_exp = min(prec_exp) over every module in the design
```

An empty set — no module carries a timescale — yields the base exponent −9, i.e. a 1 ns
tick. Every time value in the engine is a count of these ticks, held in a `u64`. No
floating-point value ever accumulates.

At a 1 ps tick, `u64` spans roughly 2.1×10^7 seconds, about 213 days of simulated time;
at the finest 1 fs grain, about five hours. Either bound is far past any RTL run, and the
count is exact at every point in between.

---

## 4. Per-module multipliers

Two integers per module carry the timescale into the engine, both derived at elaborate
time and both at least 1:

| Symbol | Definition | Meaning |
|---|---|---|
| `M` | `10^(unit_exp − global_prec_exp)` | one of the module's time units, in global ticks |
| `S` | `10^(prec_exp − global_prec_exp)` | one step of the module's **own** precision, in global ticks |
| `P` | `M / S` | steps of the module's own precision per module time unit |

A module absent from either map — the no-timescale base — defaults to the global
exponent, so `M = S = 1`. The exponent is capped at 18 so the power of ten cannot
overflow a `u64` on an absurd ratio.

These become two parallel per-process vectors that ride the sidecar tables into
`SimOpts`, next to the scalar `global_prec_exp`. The engine sets its current multipliers
**per activation**, from the entry for the process's template; empty tables mean
`M = S = 1`, which is exactly the single-timescale case.

Because the multipliers are per process, a system function that reports time answers in
the units of the module that is *running*, not of the module that happens to be last in
the design.

---

## 5. Delay conversion

### 5.1 Where a delay is converted

`Terminator::Delay.amount` is an ExprId whose value is in the declaring module's **time
units**, and it is evaluated **at suspension time**. A constant `#5` and a runtime `#d`,
`#(d*2)` or `#r` therefore share one path; a constant is simply folded to a constant
node. The conversion is one shared function, `delay_ticks_of(value, M, S)`.

One class of delay is converted earlier: a delay expression that contains a **time
literal** (`#5ns`, `#(2500ps)`, `#(2.5ns + 1ns)`) is folded in the delay domain at
elaborate time, before generic expression lowering, in both the procedural and the
structural (`assign #d`) lanes. The fold is asked first rather than as a fallback,
because the generic integer path would answer some of these shapes wrongly rather than
declining. A delay with no time literal keeps the generic path untouched.

### 5.2 The rule

| Input | Result |
|---|---|
| real | two-stage: `r = round(v × P)`; if `r < 0` the delay is `u64::MAX` and never fires; otherwise `r × S`, saturating |
| any X or Z bit | **0 ticks** |
| integral | `v × M`, saturating; a negative integer yields `u64::MAX` and never fires |

Rounding is half away from zero, which for a non-negative delay is half up.

Region selection at the terminator is `inactive = (region == Inactive) || ticks == 0`, so
**any** delay that resolves to zero ticks lands in the Inactive queue, not only a
syntactic `#0`.

### 5.3 Why the rounding has two stages

Stage 1 rounds at the **declaring module's own precision**. Stage 2 scales the result to
global ticks. Rounding once at the global grain instead would keep digits the module
declared away, which is a silent wrong answer for any mixed-precision design.

```
// top:  `timescale 1us/10ns
// fine: `timescale 1ns/100ps      ← drags the global precision to 100 ps
//
// global_prec_exp = -10           (100 ps tick)
// top: M = 10^(-6 - -10) = 10,000     one microsecond = 10,000 ticks
//      S = 10^(-8 - -10) =    100     one 10 ns step  =    100 ticks
//      P = M / S         =    100     100 precision steps per microsecond
//
// #3.453 in top:
//   stage 1: round(3.453 × 100) = round(345.3) = 345   → 3.45 us, top's own grain
//   stage 2: 345 × 100 = 34,500 ticks
//   $realtime in top reads 3.45
//
// One-stage rounding would give round(3.453 × 10,000) = 34,530 ticks — a delay
// resolved 100× finer than the module declared.
```

```
// top:  `timescale 1ns/1ns        ← precision equals unit
// fine: `timescale 1ns/1ps        ← global precision is 1 ps
//
// top: M = 1000, S = 1000, P = 1
// #2.5 in top:
//   stage 1: round(2.5 × 1) = 3    (half away from zero) → 3 ns
//   stage 2: 3 × 1000 = 3,000 ticks; $realtime reads 3
//
// One-stage rounding would give round(2.5 × 1000) = 2,500 ticks, i.e. 2.5 ns —
// a half-nanosecond delay in a module that declared nanosecond precision.
```

When every module's precision equals the global precision — the overwhelmingly common
single-timescale case — `S` is 1, `P` is `M`, and the two stages collapse into one
rounding. A design with one `` `timescale 1ns/1ps `` and `#2.5` gives 2,500 ticks either
way.

### 5.4 Pinned boundaries

| Delay | Ticks | Note |
|---|---|---|
| `#0` | 0 | Inactive queue |
| a real that rounds to zero, `-0.0` included | 0 | `-0.0` compares `>= 0` and fires as a zero delay |
| `#(-1e-9)` under a coarse precision | 0 | rounds to zero, then fires |
| `#(-1.0)` | never fires | the **rounded** value decides the sign, not the raw product |
| a negative integer | never fires | `u64::MAX` |
| `#(1ns - 5ns)` | never fires | the sign is read in the units domain, before any clamp |
| a delay with an X or Z bit | 0 | matches Icarus Verilog |
| an integer that overflows the tick domain | saturates | a wrapped delay would fire *early*, which is worse than one that fires late or not at all |
| `min:typ:max` | the `typ` arm | |

The elaborate-side fold saturates at `u32::MAX` ticks; the runtime path saturates at
`u64::MAX`. Neither wraps.

### 5.5 Time literals finer than the design precision

A time literal whose own unit is finer than the design's global precision is the one
place a delay value can carry a fraction the tick grid cannot hold. It is rounded **at
the leaf** — `round(x·10^e / S) · S`, converted back to module units — and nowhere else.
The gate is the literal's unit, not the presence of a fraction, because the two rules
answer different questions:

| Expression | Timescale | Ticks | Why |
|---|---|---|---|
| `#(2.5ns + 2.5ns)` | `1ns/1ns` | 5 ns | a real literal at the design grain keeps its fraction to the end of the expression |
| `#(1250fs + 1250fs)` | `1ns/1ps` | 2 ps | each sub-precision leaf is rounded before the addition; rounding the sum once would give 3 ps |
| `#(2500ps)` | `1ns/1ns` | 3 ns | 2.5 module units, rounded half away from zero at the module grain |
| `#(2.5ps)`, `#(0.4ns)` | `1ns/1ns` | 0 | genuinely sub-precision; they resolve to no delay |

A continuous-assign rise delay that resolves to zero stays "no delay" rather than
becoming a `#0`.

> **Status at HEAD.** `#(2*1250ps)` and `#(2500ps/2)` under `1ns/1ns` land on Icarus
> Verilog's answer (2 ns for both) where Verilator says 3 ns and 1.25 ns. The two
> reference tools disagree here, so this is an oracle split rather than a settled value;
> [../ROADMAP.md](../ROADMAP.md) carries the row.

### 5.6 Worked examples

```verilog
`timescale 1ns/100ps
module round_probe;
  reg a;
  initial begin
    a = 0;
    #1.44 a = 1;  // round(14.4) = 14 → 14 ticks = 1400 ps
    #0.05 a = 0;  // round( 0.5) =  1 →  1 tick  → 1500 ps
    #0.04 a = 1;  // round( 0.4) =  0 →  0 ticks, same time step (Inactive)
    $finish;
  end
endmodule
// waveform: a=1 at 1400 ps, then a=0 and a=1 both at 1500 ps
```

```verilog
`timescale 1ns/100ps
module fast_mod(output reg q);
  initial #2.5 q = 1;      // M = 10, S = 1 → 25 ticks = 2500 ps
endmodule

`timescale 1us/10ns
module slow_mod(output reg r);
  initial #1 r = 1;        // integral: M = 10,000 → 10,000 ticks = 1 us
endmodule

`timescale 1ns/100ps
module tb;
  wire q, r;
  fast_mod u_fast(.q(q));
  slow_mod u_slow(.r(r));
  initial begin
    $dumpfile("mixed.vcd");
    $dumpvars;
    #1001000 $finish;
  end
endmodule
// global precision = 100 ps
// q rises at tick 25      (2500 ps)
// r rises at tick 10,000  (1 us)
```

```verilog
`timescale 10ns/1ns
module boundary;
  reg clk;
  initial clk = 0;
  always #5 clk = ~clk;    // integral: 5 × M(=10) = 50 ticks = 50 ns per half period
  // A posedge every 100 ns; ten of them at 1000 ns, i.e. $time == 100 in this
  // module's 10 ns units.
endmodule
```

---

## 6. `$time`, `$realtime`, `$stime`

All three report the current time in the **calling module's time unit**, and differ in
how they treat the part below that unit.

| | `$time` | `$realtime` | `$stime` |
|---|---|---|---|
| Return type | 64-bit integer | `real` (IEEE-754 double) | 32-bit unsigned |
| Unit | the calling module's `time_unit` | same | same |
| Sub-unit part | rounded to the nearest integer | kept as a fraction | rounded, then truncated to 32 bits |
| Typical use | comparing event times, control flow | waveform annotation, human-readable logs | legacy 32-bit code |

```
$time     = (now + M/2) / M          // round half up; time is non-negative, so this
                                      // is also round half away from zero
$realtime = now as f64 / M as f64
$stime    = ((now + M/2) / M) & 0xffff_ffff
```

`$time` **rounds, it does not truncate** — IEEE 1800 §20.3.1 says "rounded to an integer
value", so 1.5 gives 2 and 2.5 gives 3, matching Icarus Verilog.

```systemverilog
// `timescale 1ns/100ps, so M = 10 and one tick is 100 ps.
// After #2.3 the current time is 23 ticks.
$display($time);      // 2    — (23 + 5) / 10
$display($realtime);  // 2.3  — 23 / 10
```

A `$strobe` or `$monitor` capture snapshots **its registering module's** `M` (and its
scope for `%m`). The postponed flush drives the current multiplier from that snapshot per
render and restores the entering value afterwards, so a mixed-timescale design does not
report one module's time in another module's units just because that module ran last in
the time step.

---

## 7. `%t` and `$timeformat`

`%t` renders a time value through the live `$timeformat` state:

```rust
struct TfState { units_exp: i32, prec: u32, suffix: String, minw: i32 }
```

| State | Default when `$timeformat` has never been called |
|---|---|
| `units_exp` | the global precision exponent |
| `prec` | 0 fraction digits |
| `suffix` | empty |
| `minw` | 20 |

The value handed to `%t` is in the current module's units, so the net decimal shift is
`log10(M) − (units_exp − global_prec_exp)`. A non-negative shift appends zeros; a
negative shift cuts into the digits. A real value is scaled in f64 and rounded to `prec`
digits; an integral value is shifted as exact decimal string arithmetic. A value that is
entirely unknown collapses to a single `x` or `z` character with the zeros and the
zero-filled fraction appended; a scale-down clears the unknown bits and goes numeric. The
suffix is appended verbatim, and the result is right-justified in the explicit `%Nt` or
`%0t` width if one is given, else in `minw` — a negative `minw` left-justifies in its
absolute value.

`$timeformat` itself:

| Rule | Behaviour |
|---|---|
| Zero arguments | resets to the defaults above |
| Exactly four arguments | `(units, precision, suffix, min_field_width)`, evaluated as runtime expressions at execution time |
| Any other arity | `E3009`: *$timeformat requires zero or four arguments (units, precision, suffix, min_field_width)* |
| As a deferred-assertion action | `E3009` — it would be captured for maturation instead of updating the format state; call it as a plain statement |
| `units` clamp | `[global_prec_exp − 64, global_prec_exp + 64]` |
| `precision` clamp | `[0, 64]` |
| `min_field_width` clamp | `[-4096, 4096]` |
| `suffix` | a string literal keeps its exact text; every other expression, a numeric literal included, goes through the same value path, so a literal `8'h6E` and a register holding `8'h6E` both render `n` |

`$printtimescale` is not recognised: it falls through the system-task map and produces a
warn-and-skip.

---

## 8. Waveform timestamps

Waveform timestamps are the **raw global tick count**, and the header names the grain.

- The VCD `$timescale` line carries `SimOpts.timescale_unit`, rendered by
  `timescale_unit_string(global_prec_exp)`: the exponent is clamped to `[-15, 2]`,
  floored to a multiple of three to pick the unit word, and the remainder becomes a
  mantissa of 1, 10 or 100. So −10 renders `100ps` and −8 renders `10ns`.
- A design with no directive gets `$timescale 1ns $end`.
- FST output derives its own exponent from the same string.

A `#1` delay under `` `timescale 10ps/1ps `` therefore appears as time 10 in a file whose
header reads `1ps`: the header states the tick grain, and the timestamp counts ticks.
Details of both writers are in [07-vcd-format.md](07-vcd-format.md).

---

## 9. The staged flow

The one-shot and staged (`vcmp` → `velab` → `vrun`) paths must resolve identical time.
Three carriers do that, and each is documented in
[14-staged-artifacts.md](14-staged-artifacts.md).

| Stage | Carrier | Shape | Missing-trailer behaviour |
|---|---|---|---|
| `.vu` (compile) | timescale tail after the hashed source-unit frame | `(unit_exp: BTreeMap<String, i8>, global_prec_exp: i8, prec_exp: BTreeMap<String, i8>)` | tolerant: empty maps and `global_prec_exp = -9`, i.e. the 1ns/1ns base |
| `.velab` (elaborate) | timescale trailer segment | `(proc_multipliers: Vec<u64>, global_prec_exp: i8, proc_prec_mults: Vec<u64>)` | tolerant: `(vec![], -9, vec![])` |
| observability | `run.json` field `global_time_precision` | `i64` — the resolved exponent, `-9` for the base | — |

The timescale trailers are deliberately **tolerant** rather than loud, because their
absent form has a well-defined meaning (the base) rather than an unknown one.

The resolution is per compilation unit at compile time, and re-resolved at link time.
When `velab` merges several `.vu` files, the global precision is the **minimum across
every unit**, and the per-module unit and precision maps merge with first-occurrence-wins
under the same search-order rule that decides which copy of a duplicated module survives.
So a design compiled unit-by-unit resolves the same global grain as the same design
compiled in one shot.

The trailer is what makes that hold: `vrun` sees only the `.velab`, and re-deriving the
multipliers would require the source it does not have.

---

## 10. Verification

Every rounding value in this document is pinned against live Icarus Verilog, and the
mixed-precision cases are the reason the conversion has two stages rather than one.

| Gate | What it holds |
|---|---|
| `crates/cli/tests/timescale_two_stage.rs` | the three shapes of §5.3: a module whose precision is coarser than the global grain rounds at its own precision first; a `1ns/1ns` module rounds `#2.5` half away from zero to 3 ns; a single-timescale design is byte-identical to a one-stage conversion |
| `crates/cli/tests/timescale_postponed.rs` | a `$strobe` or `$monitor` in the postponed region renders time with the multiplier of the module that **registered** it, not of whichever process ran last in the time step |
| `crates/cli/tests/timeformat.rs` | the full `%t` and `$timeformat` surface, byte-pinned to Icarus Verilog except the one scale-down case where that tool's own output is malformed |
| `crates/cli/tests/procedural_delay_time_literal.rs`, `delay_time_literal_in_expression.rs`, `delay_real_timelit_and_sized_negative.rs` | the time-literal fold across a census of six timescales, eighteen literals and five procedural lanes, differentially against Icarus Verilog and Verilator |
| `crates/cli/tests/time_param_declared_width.rs` | a delay named by a parameter converts at the parameter's declared width |
| the staged-flow suite | a timescaled design threads through `vcmp` → `velab` → `vrun` to byte-identical output |

The method for a new timescale case is the same one those tests use: run the design under
Icarus Verilog and under vitamin, parse the waveform timestamps, and require agreement to
the last precision unit. When they differ, dump the per-stage rounding — the module's own
`P`, the stage-1 integer, `S`, the final tick — because the divergence is almost always a
missing stage rather than a wrong arithmetic.

---

## 11. Related documents

- [06-simulation-engine.md](06-simulation-engine.md) — the scheduler, the time wheel, and where a converted delay lands.
- [07-vcd-format.md](07-vcd-format.md) — the `$timescale` header and timestamp encoding.
- [14-staged-artifacts.md](14-staged-artifacts.md) — the `.vu` and `.velab` trailers that carry the resolved timescale.
- [15-error-code-reference.md](15-error-code-reference.md) — `VITA-W1017`, `VITA-W1018`, `VITA-E8003`, `VITA-E3009`.
- [../manual/005_system-tasks.md](../manual/005_system-tasks.md) — the user-facing description of `$time`, `$realtime`, `$timeformat` and `%t`.
- [../manual/006_limitations.md](../manual/006_limitations.md) — the shipped limits, including the unimplemented `timeunit` / `timeprecision`.
