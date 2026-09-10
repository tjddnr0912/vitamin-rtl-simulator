# 005 · System Tasks and Functions

This chapter is the complete catalogue of the `$name` system tasks and system
functions vitamin accepts, grouped by family. Each entry gives the signature, the
behaviour, the argument and return types, and the limits that apply; a name that
the simulator does not implement is listed in the same table as its family, with
its status.

Related: [Installation](001_installation.md) ·
[Quickstart](002_quickstart.md) ·
[Language Reference](003_language-reference.md) ·
[CLI Reference](004_cli-reference.md) ·
[Limitations](006_limitations.md) ·
[Error Codes](007_error-codes.md).
The catalogue under [`../preview/hdl-reference/system-tasks/`](../preview/hdl-reference/system-tasks/)
describes the standards surface; where it and this chapter disagree, this chapter
is authoritative for what the simulator does.

Platforms: Linux and macOS. Outputs are byte-identical across them.

---

## How a name is resolved

The lexer takes a system name as one token matching `\$[A-Za-z_][A-Za-z0-9_$]*`,
so `$test$plusargs` and `$value$plusargs` are single identifiers. Elaboration
then routes the name through one of four paths.

| Path | Outcome | Diagnostic |
|---|---|---|
| Recognised task | lowered to a simulator statement | — |
| Recognised function | lowered to a value expression | — |
| Unrecognised task | statement skipped, elaboration continues | `W-ELAB-FEATURE-LIMIT` / `VITA-W3056`, text ``unsupported system task `$name` skipped`` |
| Unrecognised function in an expression | hard elaboration error | `E-ELAB-UNSUPPORTED` / `VITA-E3009`, text `unsupported system function in expression` |

A recognised name whose arguments or placement fall outside the supported shape
is refused loudly with `E-ELAB-UNSUPPORTED` / `VITA-E3009` and a message that
names the constraint. The simulator does not fabricate a value for a name it
cannot perform; every refusal is either a warning that skips a statement or an
error that stops elaboration. The refusals are collected in
[Names recognised and refused](#names-recognised-and-refused).

Diagnostic codes used across this chapter:

| Code | Printed as | Meaning |
|---|---|---|
| `ElabUnsupported` | `E-ELAB-UNSUPPORTED` / `VITA-E3009` | construct outside the supported shape |
| `ElabFeatureLimit` | `W-ELAB-FEATURE-LIMIT` / `VITA-W3056` | legal construct accepted and simplified, or skipped |
| `ElabStrTernaryNumeric` | `W-ELAB-STR-TERNARY` / `VITA-W3058` | a ternary of string literals is an integral value |
| `ElabUserInfo` / `ElabUserWarning` / `ElabUserError` / `ElabUserFatal` | `VITA-I3006` / `VITA-W3007` / `VITA-E3004` / `VITA-F3005` | elaboration-time severity task |
| `RunUserInfo` / `RunUserWarning` / `RunUserError` / `RunFatal` | `VITA-I4005` / `VITA-W4007` / `VITA-E4003` / `VITA-F4004` | runtime severity task |
| `RunVcdOpenFail` / `RunVcdWriteFail` | `VITA-W4018` / `VITA-W4019` | dump file cannot be opened / written |
| `RunDumpMulti` | `VITA-W4021` | extra `$dumpvars` call ignored |
| `RunBadFd` | `VITA-W4022` | file operation on an invalid or closed descriptor |
| `RunReadmem` | `VITA-W4023` | `$readmem*` / `$writemem*` / `$fread` problem |
| `RunVcdPkgVarSkip` | `VITA-W4026` | package variable has no waveform surface |
| `RunPlusargsInvalid` | `VITA-W4028` | invalid value in a matched plusarg |
| `RunUniqueViolation` | `VITA-W4031` | `unique` / `priority` matched no branch |

---

## Display and output

### Tasks

| Task | Signature | Behaviour |
|---|---|---|
| `$display` | `$display([fmt,] args…)` | render and print with a trailing newline |
| `$write` | `$write([fmt,] args…)` | render and print with no trailing newline |
| `$strobe` | `$strobe([fmt,] args…)` | capture the arguments; render and print at the end of the current time step with settled values. Multiple calls print in call order, one line per call, and each call fires once |
| `$monitor` | `$monitor([fmt,] args…)` | install a monitor for its destination; it prints once at the next end-of-step flush and again whenever a rendered argument changes |
| `$monitoron` | `$monitoron` | clear the global monitor-disable flag and force every monitor, including file monitors, to reprint at the next flush |
| `$monitoroff` | `$monitoroff` | set the global monitor-disable flag. The flag is simulation-wide, survives a later `$monitor`, and takes effect before any monitor exists |
| `$fdisplay` | `$fdisplay(fd, [fmt,] args…)` | as `$display`, routed to the descriptor in the first argument |
| `$fwrite` | `$fwrite(fd, [fmt,] args…)` | as `$write`, routed to the descriptor |
| `$fmonitor` | `$fmonitor(fd, [fmt,] args…)` | as `$monitor`, routed to the descriptor |
| `$fstrobe` | `$fstrobe(fd, [fmt,] args…)` | as `$strobe`, routed to the descriptor |
| `$sformat` | `$sformat(dest, [fmt,] args…)` | render into `dest`, a `string` variable or a packed register |
| `$swrite` | `$swrite(dest, [fmt,] args…)` | identical to `$sformat` |

There is one monitor per destination. `$fmonitor` replaces the monitor for its
own descriptor and leaves a standing stdout `$monitor` in place.

A `$display` whose argument rendering latches a `$fatal` has its print
suppressed, so a fatal message is not followed by a half-evaluated line.

```systemverilog
initial begin
  $display("hello, t=%0t", $time);
  $write("a=%h ", a); $write("b=%h\n", b);
  $strobe("settled: q=%b", q);
  $monitor("clk=%b data=%0d", clk, data);
end
```

### Radix-suffixed variants

Each of `$display`, `$write`, `$monitor`, `$strobe`, `$fdisplay`, `$fwrite`,
`$fmonitor`, `$fstrobe` and `$swrite` has `b`, `o` and `h` suffixed spellings.
The suffix sets the default radix of arguments no format specifier consumes.

| Suffix | Default radix |
|---|---|
| none | 10 |
| `b` | 2 |
| `o` | 8 |
| `h` | 16 |

Matching is exact, so `$monitoron` and `$monitoroff` are their own tasks and
never alias `$monitoro`.

### The format function

| Function | Signature | Return | Behaviour |
|---|---|---|---|
| `$sformatf` | `$sformatf(fmt, args…)` | packed-ASCII string value | render through the same engine and yield the text as a value. The format must be a string literal. Supported only as the direct right-hand side of a blocking assignment, or as an argument to a print-family task |

### Argument model

The first argument, if it is a string literal, is the format string; the rest are
value arguments. Every argument the format string does not consume still prints,
in order:

- a string-literal argument, or a runtime `string` value, becomes a format
  segment of its own and its `%` specifiers consume the arguments that follow it;
- any other argument prints in the task's default radix, as a padded `%d` field
  (`%g` for a real), or the padded `%b` / `%o` / `%h` form under a suffixed
  variant.

The fields are joined with no separator. The spacing in a format-free call comes
from the field padding alone.

`$display(cond ? "A" : "B")` — exactly one argument, a ternary with a string
literal on both arms — raises `W-ELAB-STR-TERNARY` / `VITA-W3058`. The IEEE rules
make both arms packed integers, so the call prints a decimal number rather than
text. Write `$display("%s", cond ? "A" : "B")` or an `if`/`else`.

### Format specifiers

Rendering is 4-state aware: an `x` or `z` bit renders as `x` or `z` in its own
column rather than as a fabricated number.

| Specifier | Consumes an argument | Rendering |
|---|---|---|
| `%%` | no | a literal `%` |
| `%m` `%M` | no | the current scope, justified in the field width: the instance path plus the named-block chain of the executing statement; inside a module subroutine, `module.subroutine`; inside a class method, the executing process's instance path plus `class.method`; a `$unit` or package class prints no `$unit::` or `pkg::` prefix |
| `%t` `%T` | yes | a time value rescaled to the `$timeformat` units, with the configured precision digits, suffix and minimum field width |
| `%d` `%D` | yes | decimal. Any `x` or `z` renders as a right-justified `x` or `z` |
| `%h` `%H` `%x` `%X` | yes | hexadecimal, 4 bits per digit, per-nibble `x` / `z` |
| `%o` `%O` | yes | octal, 3 bits per digit, per-digit `x` / `z` |
| `%b` `%B` | yes | binary, 1 bit per digit, per-bit `x` / `z` |
| `%f` `%F` | yes | real, fixed point, 6 fractional digits by default |
| `%e` `%E` | yes | real, scientific; the exponent is signed and at least two digits (`1.500000e+03`) |
| `%g` `%G` | yes | real, the shorter of `%f` and `%e` with trailing zeros stripped, 6 significant digits by default |
| `%c` `%C` | yes | the low 8 bits as one character, justified in the field width |
| `%s` `%S` | yes | a string literal decodes to its text; a nested `$sformatf` renders through the same formatter; a string-domain value emits its exact bytes; any other packed value renders as ASCII — bare `%s` pads at the register width with NUL rendered as a space, while `%Ns`, `%0s` and `%-s` strip NUL |
| `%v` `%V` | yes | strength form, most significant bit first, underscore-joined (`4'b10xz` renders `St1_St0_StX_HiZ`). There is no strength model: a driven bit takes `St` and `z` takes `HiZ` |
| `%u` `%U` `%z` `%Z` | yes | the argument is consumed and no text is emitted. The standard forms write raw binary bytes, which have no meaning in a text log |
| `%p` `%P` | yes | an assignment pattern. A whole aggregate renders as a pattern; any other argument renders as a scalar pattern |
| any other character | no | echoed verbatim as `%` followed by the character |

`%E` and `%G` uppercase the exponent letter and the non-finite labels (`INF`,
`NAN`); `%F` keeps lowercase `inf` and `nan`.

### Flags, width and precision

A conversion is written `%[flags][width][.precision]<spec>`. The flags are parsed
in this order, before the width digits:

| Flag | Effect | Constraint |
|---|---|---|
| `-` | left-justify in the field | must precede the width digits |
| `+` | force a sign on a non-negative `%d`, `%f`, `%e`, `%g` | |
| `0` | zero-pad the field | recognised only as the first width digit |

| Width form | Effect |
|---|---|
| `%0d` `%0h` `%0b` `%0o` | minimum width; leading zeros stripped, at least one digit kept |
| `%Nd` | a fixed field of `N` columns, right-justified, space-padded |
| `%0Nd` | a fixed field of `N` columns, zero-padded, sign-aware for `%d` (`-00042`) |
| bare `%d` | the operand's default decimal field width — the digit count of an `n`-bit value's maximum, right-justified and space-padded; 0 for a real |
| bare `%h` `%o` `%b` | the full vector width, leading zeros retained |
| `%W.Pf` | field width `W` and precision `P` on the real specifiers (`%8.2f`) |

```systemverilog
$display("%0d", count);    // "42"
$display("%5d", count);    // "   42"
$display("%d",  byte_val); // " 42"     (3-column default for 8 bits)
$display("%08h", word);    // zero-padded hex
$display("%8.2f", volts);  // "    3.14"
```

Pairing a radix conversion (`%b`, `%h`, `%o`, `%x`) with a real-typed argument is
refused at elaboration; convert with `$realtobits` first.

---

## Severity tasks

| Task | Signature | Runtime code | Effect |
|---|---|---|---|
| `$info` | `$info([fmt,] args…)` | `I-RUN-USER-INFO` / `VITA-I4005` | continue |
| `$warning` | `$warning([fmt,] args…)` | `W-RUN-USER-WARNING` / `VITA-W4007` | continue |
| `$error` | `$error([fmt,] args…)` | `E-RUN-USER-ERROR` / `VITA-E4003` | mark the run as having errors, continue |
| `$fatal` | `$fatal([n,] [fmt,] args…)` | `F-RUN-FATAL` / `VITA-F4004` | end the run with an implicit `$finish` and a fatal exit class |

Details:

- `$fatal`'s leading integer-literal finish number is consumed and never printed.
- The format and argument split matches the print family: a leading string
  literal is the format.
- The rendered text goes to the diagnostic stream (stderr), never to the print
  stream, so it never interleaves into a `$display` transcript.
- An empty message is replaced by the diagnostic's own title.
- Each message carries the source location and the instance path of the call.
- A severity task inside an assertion body is suppressed while assertions are
  disabled by `$assertoff` or `$assertkill`.

Used as a module item rather than inside a procedure, the four severity tasks run
once at elaboration and report `VITA-I3006`, `VITA-W3007`, `VITA-E3004` and
`VITA-F3005`. `$error` and `$fatal` in that position fail elaboration and no
simulation runs. Arguments must be constants; a non-constant argument is
`VITA-E3009`:

```
an elaboration `$<kind>` argument is not a constant this pass can render
(a literal, a parameter, `%m` and the `%d %h %b %o %s %c` specs are)
```

The parser also synthesizes a severity call for a `unique` or `priority`
statement that matches no branch; it reports `W-RUN-UNIQUE-VIOLATION` /
`VITA-W4031`.

---

## Simulation control

| Task | Signature | Behaviour |
|---|---|---|
| `$finish` | `$finish[(n)]` | end the run with reason `Finish`. The optional `0`, `1` or `2` argument is parsed and not read |
| `$exit` | `$exit[(…)]` | identical to `$finish`; with no `program` constructs in the language subset the two coincide |
| `$stop` | `$stop[(n)]` | end the run with reason `Stop`. vitamin is a batch simulator: `$stop` opens no interactive console |

The run prints one anchor line at the end:

```
simulation ended (Finish) at time 1000
```

The reason is one of `Finish`, `Stop`, `Quiescent`, `DeltaLimit`, `Error`.

Exit codes come from the exit class, not the reason. The exit class is `Fatal` if
a `$fatal` fired, `HadErrors` if a `$error` fired, otherwise `Ok`. The process
exits successfully when the class is `Ok` and the reason is `Finish`,
`Quiescent` or `Stop` — so `$stop` and `$finish` produce the same exit code and
differ only in the reason printed. See [CLI Reference](004_cli-reference.md) for
the exit-code table.

A testbench with no `$finish` and no `$stop` runs until quiescence or the delta
limit; include one.

`$finish` or `$stop` reached inside a subroutine body that runs on the
synchronous frame executor is a runtime fatal (`VITA-F4004`) rather than a
termination:

```
`$finish` was reached inside a subroutine body. vita ends the run with an error
instead of performing it, because a body that stops half-way still owes its
calling expression a value and the reference simulators disagree about which one.
The rest of this body still executes, as it does after a `$fatal`. Move the
`$finish` to the caller.
```

---

## Time

| Name | Kind | Signature | Return | Behaviour |
|---|---|---|---|---|
| `$time` | function | `$time` | 64-bit unsigned | the current time scaled to the calling module's time unit and rounded to nearest, half up |
| `$realtime` | function | `$realtime` | 64-bit real | the same scaling, keeping the sub-unit fraction |
| `$stime` | function | `$stime` | 32-bit unsigned | the `$time` value truncated to its low 32 bits |
| `$timeformat` | task | `$timeformat` or `$timeformat(units, precision, suffix, min_width)` | — | set the `%t` rendering state. Arguments are runtime expressions evaluated when the statement runs |
| `$printtimescale` | — | — | — | not recognised: warns `VITA-W3056` and skips |

Both `$time` and `$realtime` are unit-correct across modules with different
`` `timescale `` directives: the global tick count is scaled by the caller's unit
multiplier.

```systemverilog
$display("t=%0t now=%0d real=%g", $time, $time, $realtime);
```

`$timeformat` semantics:

| Aspect | Rule |
|---|---|
| zero arguments | reset to the defaults |
| `units` | the base-10 exponent of the display unit, clamped to the global precision exponent ± 64 |
| `precision` | fractional digits, clamped to 0…64 |
| `suffix` | a string literal keeps its exact text; any other expression is coerced as `%s` would coerce it |
| `min_width` | the minimum `%t` field width, clamped to −4096…4096 |
| defaults | units = the global precision exponent, precision = 0, suffix = empty, minimum width = 20 |

With the defaults, a `1ns/1ps` design prints `$time` = 5 through `%t` as `5000`.
`%0t`, `%Nt` and `%0Nt` override the width per call.

`$timeformat` accepts zero or exactly four arguments; one to three is
`VITA-E3009`. It is also refused as a deferred-assertion action, where it would
be captured for maturation instead of updating the format state — call it as a
plain statement.

---

## Conversion

| Name | Kind | Signature | Return | Behaviour |
|---|---|---|---|---|
| `$signed` | function | `$signed(x)` | operand width, signed | re-stamp the sign flag. In an unsigned context an unsigned sibling still makes the result zero-extend, per the standard's expression-signedness rules |
| `$unsigned` | function | `$unsigned(x)` | operand width, unsigned | re-stamp unsigned; zero-extend on resize |
| `$rtoi` | function | `$rtoi(r)` | 32-bit signed | real to integer, truncated toward zero. Folds in the constant domain |
| `$itor` | function | `$itor(i)` | 64-bit real | integer to real, exactly. A real argument is accepted and is rounded half away from zero first, so `$itor(3.9)` is `4.0` |
| `$realtobits` | function | `$realtobits(r)` | 64-bit unsigned | the raw IEEE-754 bit pattern as a plain vector |
| `$bitstoreal` | function | `$bitstoreal(v)` | 64-bit real | the same bits reinterpreted as a real. Any `x` or `z` bit yields NaN, because `x` and `z` have no real value |
| `$cast` | task and function | `$cast(dest, src);` or `s = $cast(dest, src)` | task: none; function: 32-bit signed status | write the context-sized `src` into the whole variable `dest`. In this integral, class-free subset an integral cast always succeeds, so the function form always returns 1 |

```systemverilog
real    r;
integer i;
logic [63:0] bits;

i    = $rtoi(3.9);          // 3
r    = $itor(i);            // 3.0
bits = $realtobits(r);      // IEEE-754 pattern
r    = $bitstoreal(bits);   // round-trips
```

`$cast` limits, all `VITA-E3009`:

| Condition | Message |
|---|---|
| arity other than 2 | `$cast takes (dest, source)` |
| destination is not a plain whole variable | `$cast destination must be a plain integral variable` |
| destination is read-only (a parameter, a clocking input, a desugared array parameter) | the read-only write refusal, naming `$cast into` |
| function form outside the direct right-hand side of a blocking assignment | `$dist_* / $cast (function form) are supported only as the direct rhs of a blocking assignment` |

---

## Bit-vector queries

| Name | Kind | Signature | Return | Behaviour |
|---|---|---|---|---|
| `$countones` | function | `$countones(x)` | 32-bit signed | population count of the known 1 bits; `x` and `z` bits never count, and the result is always known |
| `$onehot` | function | `$onehot(x)` | 1 bit | the known-ones count equals 1 |
| `$onehot0` | function | `$onehot0(x)` | 1 bit | the known-ones count is at most 1 |
| `$isunknown` | function | `$isunknown(x)` | 1 bit | any `x` or `z` bit is present |
| `$countbits` | function | `$countbits(v, ctrl…)` | 32-bit | count the bits of `v` matching any of the control values. Requires at least one control argument. Desugared at elaboration into per-bit case-equality tests summed as 32-bit adds |
| `$clog2` | function | `$clog2(n)` | 32-bit signed | ceiling log2, exact at any width. `$clog2(0)` and `$clog2(1)` are 0; an `x` / `z` operand yields a 32-bit `x`. A real argument rounds to nearest first, and a negative, non-finite or ≥ 2^64 real yields `x` |
| `$bits` | function | `$bits(e)` | 32-bit unsigned constant | the bit width of a type or expression. The argument is a type reference and is never evaluated; the call folds at elaboration. `$bits` of a real is 64 |

Refusals, all `VITA-E3009`:

| Condition | Message |
|---|---|
| a real operand to `$countones`, `$onehot`, `$onehot0`, `$isunknown` or `$countbits` | `real operand is not legal for a bit-vector system function` |
| `$countbits` with fewer than two arguments | `$countbits needs an expression and at least one control bit (e.g. $countbits(v, 1) / $countbits(v, 1'bx))` |
| `$countbits` operand of unresolved width | `$countbits operand has an unresolved width` |
| `$bits` of a shape neither the view resolver nor the self-determined width walk can size | `$bits argument shape unsupported (nets, array views, and self-determined expressions fold)` |

---

## Mathematics

### The real-math set

All 21 functions of the standard's real-math set are implemented. Each is pure,
returns a 64-bit real, and is computed through the vendored `libm`.

| Name | Arguments | Result |
|---|---|---|
| `$ln(x)` | 1 | natural logarithm |
| `$log10(x)` | 1 | base-10 logarithm |
| `$exp(x)` | 1 | `e**x` |
| `$sqrt(x)` | 1 | square root |
| `$pow(x, y)` | 2 | `x**y` |
| `$floor(x)` | 1 | floor, as a real |
| `$ceil(x)` | 1 | ceiling, as a real |
| `$sin(x)` `$cos(x)` `$tan(x)` | 1 | trigonometric |
| `$asin(x)` `$acos(x)` `$atan(x)` | 1 | inverse trigonometric |
| `$atan2(y, x)` | 2 | two-argument arctangent |
| `$hypot(x, y)` | 2 | `sqrt(x*x + y*y)` |
| `$sinh(x)` `$cosh(x)` `$tanh(x)` | 1 | hyperbolic |
| `$asinh(x)` `$acosh(x)` `$atanh(x)` | 1 | inverse hyperbolic |

Argument handling: a real argument is used as is; an integral argument coerces to
real as a signed value; an `x` or `z` operand reads 0.0, and so does an absent
argument.

Domain errors propagate NaN or ±infinity exactly as C does — `$sqrt(-1)`,
`$acos(2)` and `$ln(0)` are not clamped and produce no diagnostic. Non-finite
values print the way C spells them: lowercase `nan` for either sign, and `inf` or
`-inf`.

### Why the mathematics goes through a vendored libm

The workspace vendors `libm` 0.2.16 (MIT) at `third_party/libm` and declares it
once with `default-features = false`. Disabling the default features disables the
architecture feature, so no hardware float intrinsic is used and every `f64`
result is bit-identical on any IEEE-754 target. That is what makes the golden
outputs byte-identical across platforms. The dependency is consumed by exactly
one crate — the simulation engine — and powers two things: the real-math set
above and the transcendental steps of the non-uniform `$dist_*` distributions.
`third_party/libm` is a path dependency and not a workspace member, so
`cargo clippy --workspace` does not lint third-party code.

`$random` and `$dist_uniform` deliberately avoid `libm`: they are pure `f64`
multiply, add and floor, which are bit-exact on every IEEE-754 platform on their
own.

### The non-uniform distributions

| Name | Arguments | Return |
|---|---|---|
| `$dist_uniform(seed, start, end)` | 3 | 32-bit signed |
| `$dist_normal(seed, mean, std_dev)` | 3 | 32-bit signed |
| `$dist_exponential(seed, mean)` | 2 | 32-bit signed |
| `$dist_poisson(seed, mean)` | 2 | 32-bit signed |
| `$dist_chi_square(seed, df)` | 2 | 32-bit signed |
| `$dist_t(seed, df)` | 2 | 32-bit signed |
| `$dist_erlang(seed, k_stage, mean)` | 3 | 32-bit signed |

`$dist_uniform` is the uniform member of the family; the other six are the
non-uniform members. Every one reads the seed variable, advances it and writes it
back, so the seed is an in-out argument and must be a plain integral variable at
least 32 bits wide. Degenerate arguments are answered without advancing the seed:

| Call | Degenerate case | Result |
|---|---|---|
| `$dist_uniform` | `start >= end` | returns `start`, seed unchanged |
| `$dist_exponential`, `$dist_poisson` | `mean <= 0` | returns 0, seed unchanged |
| `$dist_chi_square`, `$dist_t` | `df <= 0` | returns 0 |
| `$dist_erlang` | `k <= 0` or `mean <= 0` | returns 0 |

The distribution parameters are evaluated before the seed is read; that order is
observable and fixed.

---

## Random

| Name | Kind | Signature | Return |
|---|---|---|---|
| `$random` | function, pure | `$random` | 32-bit signed |
| `$random` | function, statement-effect | `$random(seed)` | 32-bit signed |
| `$urandom` | function, pure | `$urandom` or `$urandom(seed)` | 32-bit unsigned |
| `$urandom_range` | function, pure | `$urandom_range(maxval[, minval])` | 32-bit unsigned |
| `$dist_*` | function, statement-effect | see above | 32-bit signed |
| `randomize()` | method-form task | `obj.randomize()` or `randomize() with {…}` | 32-bit status written to a sibling variable |

`$urandom_range` swaps its bounds when the first exceeds the second, takes
`minval` as 0 in the one-argument form, and yields a 32-bit `x` if either bound
has an `x` or `z` bit.

A seeded `$random` and any `$dist_*` also work as a bare statement
(`$random(seed);`): the value is discarded and the seed writeback still happens.

`obj.randomize()` draws each `rand` field through the seeded uniform generator. A
null or `x` handle is a no-op, and the success flag is written to the result
variable.

### Seeding and reproducibility

Two generator states live in the simulation state and both start at 0. There is
no seed flag on the command line: a run is deterministic by construction, and
`run.json` reports `"seed": null` for that reason.

| Generator | Algorithm | Initial state | Seeding | Stream |
|---|---|---|---|---|
| `$random` unseeded | the 1364 Annex N linear congruential generator `s' = 69069·s + 1`, with a zero seed substituted by 259341593; the high 23 bits form an `f32` mantissa in [1,2) that is stretched affinely over the signed 32-bit range | 0 | not seedable except through the seeded form | bit-stream compatible with Icarus Verilog; the first three draws from the zero seed are `303379748`, `-1064739199`, `-2071669239` |
| `$random(seed)` | the same kernel, with the seed variable read, advanced and written back | the variable's value; an `x` / `z` read counts as 0 and the zero substitution then applies | the design's own variable | pinned against Icarus: `s=5` yields `-2147138048` then `230383387` |
| `$urandom`, `$urandom_range` | splitmix64, returning the top 32 bits | 0 | `$urandom(seed)` re-seeds the global state and is input-only — the argument is never written back | implementation-defined by the standard. vitamin pins its own splitmix64 sequence from initial state 0 as a contract, not against another simulator |
| `$dist_uniform` | the 1364 Annex uniform algorithm, pure `f64` arithmetic | the seed variable | the seed variable, written back | pinned against Icarus: `$dist_uniform(1, 0, 99)` yields 0 then 11; a zero seed yields 57 |
| the six non-uniform `$dist_*` | ports of the 1364 Annex algorithms; the seed advances through the same integer generator | the seed variable | the seed variable, written back | the seed stream is byte-identical to Icarus. The returned integer is computed through the vendored `libm` and is vitamin's own contract, which can differ from Icarus by the final rounding: `$dist_normal(1,100,10)` is 105 where Icarus gives 106, and `$dist_exponential(1,20)` is 220 where Icarus gives 221; poisson, chi-square, t and erlang match exactly |

The sequence is reproducible because every input to it is constant: both generator
states start at a constant, the kernels are integer or plain `f64` arithmetic
with no hardware intrinsic, the only external input is the design's own seed
variables, and nothing reads the clock, the process id or the environment. The
same design produces the same stream on every run and on every supported
platform.

Refusals, all `VITA-E3009`:

| Condition | Message |
|---|---|
| `$random` with more than one argument | `$random takes at most one seed` |
| `$random(seed)` where the seed is not a plain whole variable | `$random seed must be a plain integral variable` |
| `$random(seed)` where the seed is read-only | the read-only write refusal, naming `advance $random's seed in` |
| seeded `$random` outside a direct blocking-assignment right-hand side | `seeded $random is supported only as the direct rhs of a blocking assignment` |
| `$dist_*` with the wrong arity | `a $dist_* function takes (seed, …) with the distribution's fixed arity` |
| `$dist_*` seed that is not a plain whole variable | `a $dist_* seed must be a plain integral variable` |
| `$dist_*` seed variable narrower than 32 bits | `a $dist_* seed variable must be at least 32 bits (a narrower seed would truncate the RNG state)` |
| `$dist_*` outside a direct blocking-assignment right-hand side | `$dist_* / $cast (function form) are supported only as the direct rhs of a blocking assignment` |

---

## Plusargs

| Name | Kind | Signature | Return | Behaviour |
|---|---|---|---|---|
| `$test$plusargs` | function, pure | `$test$plusargs(query)` | 32-bit signed, 1 or 0 | true when some command-line plusarg starts with `query`. The query must be a string literal; a non-literal query yields `x` |
| `$value$plusargs` | function, statement-effect | `$value$plusargs(fmt, var)` | 32-bit signed, 1 or 0 | split `fmt` at its first `%`, find the first plusarg starting with the prefix, convert the remainder per the specifier and write `var` |

```console
$ vita tb.sv +VERBOSE +N=42
```

`$value$plusargs` conversion rules:

| Aspect | Rule |
|---|---|
| specifier | `%d`/`%D` radix 10, `%h`/`%H`/`%x`/`%X` radix 16, `%o`/`%O` radix 8, `%b`/`%B` radix 2, anything else treated as `%s` (raw bytes packed most significant first, minimum width 8) |
| separators | underscores separate digits and may not lead |
| 4-state digits | `x` and `z` parse positionally for the bit radixes, and the most significant digit's kind extends to the destination width. A lone `x` or `z` is a whole-value `x` or `z` for `%d` |
| width | bit radixes accumulate as many words as the digits need; decimal multiply-accumulates wrapping at the destination width, so a negative `%d` sign-extends into a destination wider than 64 bits |
| miss | the variable is left untouched and the call returns 0 |
| format with no `%` | a pure probe: the prefix hit is returned, nothing is written |
| invalid value | the variable is written all `x`, `W-RUN-PLUSARGS-INVALID` / `VITA-W4028` is emitted, and the status stays 1 |

Refusals, all `VITA-E3009`:

| Condition | Message |
|---|---|
| `$value$plusargs` arity other than 2 | `$value$plusargs takes (format, variable)` |
| non-literal format | `$value$plusargs needs a string-literal format` |
| more than one specifier, or one outside `d D h H x X o O b B s S` | `$value$plusargs format supports one %d/%h/%x/%o/%b/%s spec` |
| target that is not a plain whole variable | `$value$plusargs target must be a plain variable` |
| target that is a hierarchical element select | `a $value$plusargs target cannot be a hierarchical element select — read into a local variable` |
| read-only target | the read-only write refusal, naming `$value$plusargs into` |
| a position that is not evaluated exactly once | see [Placement of statement-effect functions](#placement-of-statement-effect-functions) |
| `$test$plusargs` with a non-literal query | `$test$plusargs needs a string-literal query` |

---

## File input and output

### The descriptor model

| Form | Opened by | Value | Failure |
|---|---|---|---|
| file descriptor | `$fopen(name, mode)` | `32'h8000_0000 \| n`, `n` counting up from 3 | 0 |
| multi-channel descriptor | `$fopen(name)` with no mode | `1 << bit`, the lowest channel bit not currently open; bit 0 is stdout and is reserved, so bits 1…30 are handed out and a closed bit is reused | 0 when the channel space is full |

Three descriptors are pre-opened and are not closable: `32'h8000_0000` (stdin),
`32'h8000_0001` (stdout), `32'h8000_0002` (stderr). Closing one warns and is a
no-op, and the descriptor stays usable.

Open modes: a trailing `b` is ignored. `"r"` and `"r+"` open for reading, writing
only with `+`. `"a"` and `"a+"` create and append, reading only with `+`. Every
other mode string, recognised or not, behaves as `"w"` — create, truncate, write,
reading only with `+`. A mode containing `r` or `+` marks the descriptor
readable; plain `"w"` and `"a"` are write-only.

Files are unbuffered: every write is a direct write to the file.

Write routing:

| Target | Result |
|---|---|
| stdout in descriptor form | routed through the same deterministic sink as `$display`, so it interleaves in statement order |
| stderr in descriptor form | the process's stderr |
| any other open descriptor | the file |
| a bad or closed descriptor | one `W-RUN-BAD-FD` / `VITA-W4022` per descriptor, then dropped |
| a write to stdin | falls through to the descriptor miss: `VITA-W4022` and dropped |
| multi-channel `fd == 0` | warns |
| multi-channel with bit 0 set | stdout |
| multi-channel bits 1…30 | broadcast to their channel files; a missing channel warns |

Read routing:

| Source | Result |
|---|---|
| pushback from `$ungetc` | served first, last in first out |
| stdout or stderr | no byte, no warning, no end-of-file latch — `$fgetc` returns −1 while `$feof` stays 0 |
| stdin | not read: the descriptor miss applies, so `VITA-W4022` and −1. A stdin-driven simulation would not be byte-deterministic |
| a valid write-only descriptor | no byte, no warning, no end-of-file latch |
| an unknown descriptor | one `VITA-W4022` per descriptor |

End-of-file is lazy: the flag is set only by a read that fails on a readable
descriptor.

### Tasks and functions

| Name | Kind | Signature | Return | Behaviour |
|---|---|---|---|---|
| `$fopen` | function, statement-effect | `$fopen(name[, mode])` | 32-bit signed descriptor, 0 on failure | name and mode may each be a string literal, a runtime `string`, or a packed register holding ASCII |
| `$fclose` | task | `$fclose(fd)` | — | descriptor form flushes, closes and clears the read state. Multi-channel form closes every set channel bit except bit 0 |
| `$fdisplay` `$fwrite` `$fmonitor` `$fstrobe` | task | `(fd, [fmt,] args…)` | — | see [Display and output](#display-and-output) |
| `$fgetc` | function, statement-effect | `$fgetc(fd)` | 32-bit signed | one byte, or −1 at end of file, on a bad descriptor, or on a write-only descriptor |
| `$feof` | function, pure | `$feof(fd)` | 32-bit signed | 1 at end of file, 0 while readable, −1 for a bad or closed descriptor; a pre-opened descriptor is always 0. The only pure file function, so `while (!$feof(fd))` works in any expression |
| `$ungetc` | function, statement-effect | `$ungetc(c, fd)` | 32-bit signed | push one byte back and clear end of file; 0 on success, −1 otherwise. Only an exact known −1 is the end-of-file sentinel; any other `c` pushes its low byte with `x` / `z` bits coerced to 0. A pre-opened descriptor returns −1 |
| `$fgets` | function, statement-effect | `$fgets(str, fd)` | 32-bit signed byte count | see below |
| `$fread` | function, statement-effect | `$fread(target, fd[, start[, count]])` | 32-bit signed byte count | see below |
| `$fscanf` | function, statement-effect | `$fscanf(fd, fmt, dsts…)` | 32-bit signed conversion count | see the conversion table |
| `$sscanf` | function, statement-effect | `$sscanf(str, fmt, dsts…)` | 32-bit signed conversion count | the same engine over a byte source instead of a descriptor |
| `$fflush` | task | `$fflush[(fd)]` | — | accepted and dropped with no diagnostic. Writes are unbuffered, so there is nothing to flush |

`$fgets` destination rules:

| Destination | Behaviour |
|---|---|
| a `string` variable | reads the whole line uncapped, through a retained newline or to end of file |
| a packed register | capacity is `width/8` bytes, the full width with no NUL reserved; reads up to that or through a newline and packs right-justified, most significant byte first |
| a register narrower than 8 bits | reads no stream byte, clears the destination to 0, returns 0 |
| end of file, bad descriptor, write-only descriptor | destination unchanged, returns 0 |

The returned count stops at the first NUL, following C semantics, although the
bytes were consumed.

`$fread` destination rules:

| Destination | Behaviour |
|---|---|
| a single register or vector | reads `ceil(width/8)` bytes and merges them into the prior value; end of file leaves the destination unchanged |
| a memory | fills elements ascending from `start`, in declared index terms |
| `start` / `count` | each `x` / `z` bit coerces to 0; a present operand always counts. An out-of-range `start` warns `VITA-W4023` and reads nothing; a too-large `count` warns and clamps |
| a memory with a negative declared base | unsupported: warns `VITA-W4023` and reads nothing |

### scanf conversions

The return value is the conversion count when positive, −1 when no source byte
was available at entry, and 0 when input was present but nothing converted.

| Specifier | Supported | Behaviour |
|---|---|---|
| whitespace in the format | yes | skips a run of whitespace in the input |
| a literal character | yes | must match exactly, otherwise the scan stops |
| `%%` | yes | matches a literal `%` |
| `%*` | yes | assignment suppression: the conversion runs, no destination is consumed, and it is not counted |
| field width digits | yes | an explicit `0` width reads nothing |
| `%c` | yes | exactly one character with no leading-whitespace skip; an explicit width is ignored except that `%0c` reads zero |
| `%[set]` `%[^set]` | yes | full C scanset rules: a leading `^` negates, `]` immediately after `[` or `[^` is literal, `a-z` is a range, a leading or trailing `-` is literal, no leading-whitespace skip. An unterminated scanset stops the scan |
| `%s` `%S` | yes | skip leading whitespace, then a whitespace-delimited run bounded by the width |
| `%d` `%D` | yes | radix 10; a leading `+` or `-` is honoured only here |
| `%h` `%H` `%x` `%X` | yes | radix 16; the 4-state digits `x X z Z ?` are honoured |
| `%o` `%O` | yes | radix 8; 4-state digits honoured |
| `%b` `%B` | yes | radix 2; 4-state digits honoured |
| any other specifier | no | the scan stops at that conversion |

### File refusals

All `VITA-E3009`:

| Condition | Message |
|---|---|
| `$fopen` arity outside 1…2 | `$fopen takes (name[, mode])` |
| `$fopen` outside a direct blocking-assignment right-hand side | `$fopen is supported only as the direct rhs of a blocking assignment` |
| `$fgetc` / `$feof` / `$ungetc` wrong arity | `$fgetc(fd) takes 1 argument(s)`, and the like |
| `$fgets` arity other than 2 | `$fgets takes (str, fd)` |
| `$fgets` target not a plain variable | `$fgets target must be a plain variable` |
| `$fgets` target a hierarchical element select | `a $fgets target cannot be a hierarchical element select — read into a local variable` |
| `$fread` arity outside 2…4 | `$fread takes (target, fd[, start[, count]])` |
| `$fread` target an element select | `$fread target must be a whole memory or a variable, not an element select` |
| `$fread` target not an integral variable or memory | `$fread target must be an integral variable or memory` |
| `$fscanf` / `$sscanf` fewer than two arguments | `$fscanf/$sscanf take (source, format, args...)` |
| `$fscanf` / `$sscanf` non-literal format | `$fscanf/$sscanf need a string-literal format` |
| `$fscanf` / `$sscanf` destination not a plain variable | `$fscanf/$sscanf destination arguments must be plain variables` |
| a read-only destination on any of the above | the read-only write refusal, naming the call |
| an intra-assignment delay on `$fopen`, `$fgetc`, `$feof`, `$ungetc`, `$fgets`, `$fread`, `$fscanf`, `$sscanf` or `$sformatf` | `intra-assignment delay on <call> is unsupported` |
| `$sformatf` non-literal format | `$sformatf needs a string-literal format` |
| `$sformatf` in an expression position that is not format-aware | `$sformatf is supported only as the direct rhs of a blocking assignment` |
| `$sformat` / `$swrite*` first argument not a whole register or string | `$sformat's first argument must be a whole register or SV string` |

### Placement of statement-effect functions

`$fgetc`, `$ungetc`, `$fgets`, `$fread`, `$fscanf`, `$sscanf` and
`$value$plusargs` change state as a side effect, so the call is evaluated into a
temporary once, before the surrounding statement runs. Positions where the call
would run a different number of times than written are refused with
`VITA-E3009`. The message for the descriptor-advancing reads:

```
`$fgets` advances the file position, so vita evaluates it into a temporary once,
before the statement runs. This position is refused because the call would then
happen a DIFFERENT NUMBER OF TIMES than written: the right operand of `&&` / `||`
and an arm of `?:` may be skipped, a loop condition is re-evaluated per iteration,
and a `$monitor` / `$strobe` argument is re-rendered on every later change and
would show the frozen temporary. Read it into a variable in its own statement
first, then use that variable here. Placements evaluated exactly once per
execution are supported — a blocking or nonblocking rhs, an `if` condition, a
`case` scrutinee, a `repeat` count, a `$display`-style argument, an lvalue index
```

The supported placements are therefore: a blocking or nonblocking right-hand
side, an `if` condition, a `case` scrutinee, a `repeat` count, a `$display`-style
task argument, an lvalue index, and a bare statement. Arguments of `$monitor`,
`$strobe`, `$fmonitor` and `$fstrobe`, in every radix spelling, are excluded.

The same family stops the run with `RunFatal` / `VITA-F4004` when it is reached
on the synchronous frame executor:

```
`$fgets` does its work as a statement-level effect, which the synchronous
`&self` frame executor cannot perform — the call would return 0 and leave its
destination untouched, so the run stops here rather than continuing on that
value. This is one of the few positions vita cannot route to the statement
executor: a class-method body, a CONTINUOUSLY re-evaluated expression (`assign`,
`force`, a `wait` condition), or an intra-assignment delay (`x = #1 f(...)`). It
DOES work in a module process, and in a task or function called from a statement
— with or without `automatic`, with or without output formals. Call it there,
assign the result to a variable, and use that variable here.
```

The names this fatal reports are `$fgets`, `$fread`, `$fscanf`, `$sscanf`,
`$fgetc`, `$feof`, `$ungetc`, `$fopen`, `$value$plusargs`, a seeded `$random`, a
seeded `$dist_*`, the function form of `$cast`, and a queue pop. `$sformatf` is
not in the family — the frame executor performs it directly. See
[Limitations](006_limitations.md).

---

## Memory load and dump

| Task | Signature | Behaviour |
|---|---|---|
| `$readmemb` | `$readmemb(file, mem[, start[, finish]])` | load binary tokens into `mem` |
| `$readmemh` | `$readmemh(file, mem[, start[, finish]])` | load hexadecimal tokens into `mem` |
| `$writememb` | `$writememb(file, mem[, start[, finish]])` | dump `mem` in binary |
| `$writememh` | `$writememh(file, mem[, start[, finish]])` | dump `mem` in hexadecimal |

The file name is any string expression, not only a literal — a `string` variable
or a packed register holding ASCII both work:

```systemverilog
reg [1023:0] firmware_file;
initial begin
  firmware_file = "fw.hex";
  $readmemh(firmware_file, dut.ram.mem);
end
```

The memory may be named hierarchically, which is the firmware-loading idiom used
by the workload corpus designs. Address arguments are lowered as integral
expressions, so a real parameter cannot reach the engine as a bit pattern.
`$readmem*` writes its target and is refused on a read-only or clocking-input
memory; `$writemem*` only reads.

`$readmem*` details:

| Aspect | Rule |
|---|---|
| comments | `//` line comments and `/* */` block comments are stripped |
| address tokens | `@<hex>` is supported; a malformed one warns |
| window | `(start)` alone implies `finish = base + length − 1`; neither implies the whole array ascending; `start > finish` walks descending |
| declared base | the minimum index of the outer dimension, 0 when absent. A negative declared base (`reg m[-1:1]`) is unsupported and loads nothing |
| empty name | warns `$readmem: the file-name argument is empty` |
| unopenable file | warns `$readmem: unable to open '<name>' for reading` |
| target not a memory | warns `$readmem target is not a memory` |
| address outside the window | warns `address N outside the load range; stopped` and returns |
| too few words with no `@` token | warns `not enough words for the range [s:f] (W of N); rest unchanged` |

Every one of those is `W-RUN-READMEM` / `VITA-W4023`, anchored at the calling
statement with file, line, column and instance path, so a testbench loading
several memories does not print interchangeable lines:

```
d.sv:4:5: warning[VITA-W4023] $readmem: unable to open 'fw.hex' for reading [in t]
```

`$fread` shares the code and the anchoring.

`$writemem*` details:

| Aspect | Rule |
|---|---|
| header | the first line is always the literal `// 0x00000000`; it does not reflect the base or the start |
| body | one element per line; every line, including the last, ends with a newline |
| window | the optional `(start[, finish])` is an inclusive declared-index window, descending when `finish < start` |
| out-of-range window | a warning; the file is not created and the simulation continues |
| encoding | hexadecimal compresses `x` / `z` per nibble; binary is per bit and uncompressed |

---

## Waveform dump

Waveform output is design-driven: a file is produced only when the design calls
the dump tasks. There is no always-on dump.

| Task | Signature | Behaviour |
|---|---|---|
| `$dumpfile` | `$dumpfile(name)` | set the output path |
| `$dumpvars` | `$dumpvars([level][, scope\|net]…)` | open the file, declare the selected variables, write the header, emit the initial values, and start recording |
| `$dumpoff` | `$dumpoff` | checkpoint at the current time and stop recording |
| `$dumpon` | `$dumpon` | resume recording and re-emit a full snapshot |
| `$dumpall` | `$dumpall` | emit a full snapshot at the current time; it does not resume a stopped dump |
| `$dumpflush` | `$dumpflush` | flush buffered bytes to the operating system immediately. Errors surface at finalize as `VITA-W4019` |
| `$dumplimit` | `$dumplimit(bytes)` | install a byte budget; when it is reached the writer emits one `$comment Dump limit reached $end` and drops further records. An `x` / `z` or zero size installs no budget |

```systemverilog
initial begin
  $dumpfile("wave.vcd");
  $dumpvars(0, top);
end
```

Path resolution order: the CLI `-o` override, then `$dumpfile`, then `dump.vcd`.
A path ending in `.fst`, case-insensitively, is produced by transcoding — the VCD
is written to `<path>.vcdtmp` and transcoded at finalize. See
[CLI Reference](004_cli-reference.md).

`$dumpvars` selection:

| Argument | Effect |
|---|---|
| none, or a level only | no filter: every eligible variable is dumped |
| a net | that net is selected |
| a scope or module name | every variable whose hierarchical name lies within `level` segments below that scope. Level 0 means unlimited, level `N` means `N` levels, so `$dumpvars(1, top)` selects top's own variables |
| an unresolvable scope | degrades to dumping everything rather than producing an empty waveform |
| any other non-net, non-constant argument | dropped |

A second and any later `$dumpvars` call warns `W-RUN-DUMP-MULTI` /
`VITA-W4021` — `extra $dumpvars call ignored` — and does nothing; the first call
wins. A dump file that cannot be opened warns `W-RUN-VCD-OPEN-FAIL` /
`VITA-W4018` and the simulation continues without a waveform.

Variables excluded from the declaration: frame-local variables; the heap kinds
(dynamic array, queue, associative array, string); and internal temporaries.
Package variables live in a reserved scope with no waveform surface — selecting
one explicitly warns `W-RUN-VCD-PKGVAR-SKIP` / `VITA-W4026`.

Arrays are declared and dumped per element: a net with more than one word emits
one `$var` per word, named `leaf[i]` or `leaf[i][j]`, and the initial snapshot
writes one entry per element.

---

## Assertion control and assertion sampling

### Runtime assertion control

| Task | Signature | Effect |
|---|---|---|
| `$assertoff` | `$assertoff` | disable assertion reporting |
| `$asserton` | `$asserton` | enable assertion reporting |
| `$assertkill` | `$assertkill` | disable assertion reporting. In-flight pipeline registers persist but cannot report |
| `$assertcontrol` | — | not recognised: warns `VITA-W3056` and skips |

While assertions are disabled, a gated assertion fire produces no diagnostic and
no exit-class change. Only the global, argument-free form is supported; any
argument at all, including the standard levels and scope list, is `VITA-E3009`:

```
`$assertoff` with a levels/scope argument is unsupported (only the global
no-argument form is supported; a scoped control would silently over-disable)
```

### Sampled-value functions

Six names are recognised inside assertion contexts. Each desugars into reads of a
synthesized previous-value register — one register per distinct signal, shared
across uses, driven by a nonblocking `prev <= signal`.

| Function | Arguments | Result | Desugaring |
|---|---|---|---|
| `$sampled(e)` | 1, any expression | the value of `e` | identity; it recurses so nested sampled calls resolve. No preponed region is modelled |
| `$past(x)` | 1, a simple signal | the previous value | `prev_x` |
| `$stable(x)` | 1 | 1 bit | `prev_x === x` |
| `$changed(x)` | 1 | 1 bit | `prev_x !== x` |
| `$rose(x)` | 1 | 1 bit | `~prev_x[0] & x[0]` |
| `$fell(x)` | 1 | 1 bit | `prev_x[0] & ~x[0]` |

The rewrite descends into unary, binary, ternary and parenthesized expressions.
Refusals, all `VITA-E3009`:

| Condition | Message |
|---|---|
| arity other than 1 | `$past takes one signal argument` |
| an argument that is not a simple identifier | `$past argument must be a simple signal` |
| a hierarchical argument | `$past of a hierarchical signal is unsupported` — one register per name, and a hierarchical name would alias |
| a sampled call in a shape the rewrite does not descend into, such as inside a concatenation | falls through to `unsupported system function in expression` |

---

## Introspection

These fold at elaboration. Their argument is a type or net reference and is never
evaluated.

| Function | Arguments | Result |
|---|---|---|
| `$bits(e)` | 1 | the bit width, 32-bit unsigned; 64 for a real |
| `$size(x[, d])` | 1 or 2 | `abs(left − right) + 1` |
| `$left(x[, d])` | 1 or 2 | the left bound |
| `$right(x[, d])` | 1 or 2 | the right bound |
| `$low(x[, d])` | 1 or 2 | `min(left, right)` |
| `$high(x[, d])` | 1 or 2 | `max(left, right)` |
| `$increment(x[, d])` | 1 or 2 | 1 when `left >= right`, otherwise −1 |
| `$dimensions(x)` | 1 | the total dimension count |
| `$unpacked_dimensions(x)` | 1 | the unpacked dimension count |
| `$typename(net)` | 1 | vitamin's canonical type spelling, as a packed-ASCII string constant. Only a net argument resolves; a type literal, an indexed reference or an expression stays loud |
| `$isunbounded(x)` | 1 | 1 when `x` is the `$` token, otherwise 0 |

The dimension order is the unpacked dimensions in declaration order followed by
the packed dimensions. The dimension index `d` is 1-based and defaults to 1, the
outermost. A `d` below 1 or above the dimension count yields a 32-bit `x`.
Results are 32-bit when the value fits 32 bits and 64-bit otherwise.

`$typename` and `$isunbounded` are pinned against the standard by hand: Icarus
Verilog rejects both, so no differential oracle exists for them.

These names also fold in the constant domain, where a parameter, a localparam or
a range specification needs a value: `$clog2`, `$rtoi`, `$bits`, and the eight
dimension queries. A built-in string method with an integral result over a
constant string folds as well. A system call that cannot fold in a bound or
parameter position reports `a $<name> that is not a constant here` rather than
degrading to a 1-bit net.

The expression hoister treats `$bits`, `$size`, `$high`, `$low`, `$left`,
`$right`, `$increment`, `$dimensions`, `$unpacked_dimensions` and `$typename` as
non-evaluating and stands down rather than hoist a copy out of one. `$clog2` and
`$isunknown` do evaluate their operand and are deliberately absent from that
list.

---

## Observability

| Task | Signature | Behaviour |
|---|---|---|
| `$vita_stage` | `$vita_stage("label"[, values…])` | record one structured stage marker. It never prints |

`$vita_stage` is a vendor extension for agent-facing observability. It lowers to
a no-op statement. Under the `+STAGE_TRACE` plusarg it appends one line per call
to `stage.jsonl` in the `--obs-dir` directory:

```json
{"v":1,"t":1500,"kind":"stage","label":"reset_done","idx":3,"vals":["42"]}
```

`args[0]` is the label and is read as a string. `args[1..]` are values, each
formatted the way `$display %0d` formats it and emitted as JSON strings so `x`
and `z` are representable. `idx` counts calls from 0 and `t` is the current
simulation time. Arguments are runtime expressions evaluated when the statement
runs. Without `+STAGE_TRACE` the call is a pure no-op with no cost beyond the
statement dispatch.

```console
$ vita tb.sv --obs-dir out +STAGE_TRACE
```

The plusarg is matched as the bare flag or as `+STAGE_TRACE=<value>`; a prefixed
neighbour such as `+STAGE_TRACEX` does not arm it.

Refusals:

| Condition | Diagnostic |
|---|---|
| no arguments | `VITA-E3009`: `$vita_stage requires at least a label argument ($vita_stage("label"[, values…]))` |
| used as a deferred-assertion action | `VITA-E3009`: `$vita_stage as a deferred-assertion action is unsupported — call it as a plain statement` |
| a design using it passed to `velab` | a CLI error: `` `$vita_stage` is a one-shot `vita` task — `velab` does not stage it (run one-shot: `vita <design> --obs-dir <D> +STAGE_TRACE`) `` |

`$vita_stage` is the only `$vita_*` name the simulator recognises. Any other
`$vita_*` task warns and skips; any other `$vita_*` function is an error. The
observability rails around it — `run.json`, `results.jsonl`, `coverage.json`,
`trace.jsonl` — are described in [CLI Reference](004_cli-reference.md) and
specified in [`../preview/19-ai-agent-observability.md`](../preview/19-ai-agent-observability.md).

---

## Miscellaneous

### Method-form builtins

Array, queue, associative-array, string and class methods share the same internal
tables as the `$name` builtins but are written as method calls, not as system
names. They are covered by the
[Language Reference](003_language-reference.md); the surface is:

| Family | Members |
|---|---|
| size and existence | `.size()` `.num()` `.exists(key)` `.len()` |
| queue and dynamic array | `.new[n]()` `.delete()` `.delete(index)` `.push_back()` `.push_front()` `.pop_back()` `.pop_front()` `.insert()` |
| associative iteration | `.first(k)` `.next(k)` `.last(k)` `.prev(k)` `.delete(key)` |
| array reductions | `.sum()` `.product()` `.and()` `.or()` `.xor()` |
| array ordering | `.sort()` `.rsort()` `.reverse()` |
| array locators | `.min()` `.max()` `.unique()` `.find()` `.find_index()` `.find_first()` `.find_last()` `.find_first_index()` `.find_last_index()` `.unique_index()` |
| string bytes | `.getc(i)` `.putc()` `.substr(i, j)` `.toupper()` `.tolower()` `.compare()` |
| string conversion | `.atoi()` `.atohex()` `.atooct()` `.atobin()` `.atoreal()` `.itoa()` `.hextoa()` `.octtoa()` `.bintoa()` |
| randomization | `.randomize()` |

`.pop_back()`, `.pop_front()` and the four associative iteration steps are
statement-effect calls: outside a direct blocking assignment they warn and yield
`x` without changing the container. A reduction over an empty array yields the
element type's zero.

### Names recognised and refused

Every refusal in this chapter reports `E-ELAB-UNSUPPORTED` / `VITA-E3009`. The
constructs refused unconditionally, as opposed to an argument-shape error:

| Construct | Reason |
|---|---|
| `$assertoff` / `$asserton` / `$assertkill` with any argument | there is no per-scope assertion grouping; accepting the argument would over-disable silently |
| `$timeformat` with one to three arguments | the standard arity is zero or four |
| `$timeformat` or `$vita_stage` as a deferred-assertion action | the call would be captured for maturation instead of taking effect |
| `$vita_stage` in the staged flow (`velab`) | it is a one-shot `vita` task |
| seeded `$random`, any `$dist_*`, the function form of `$cast`, `$fopen`, `$sformatf` outside a direct blocking-assignment right-hand side | they are statement-level effects |
| `$fgetc`, `$ungetc`, `$fgets`, `$fread`, `$fscanf`, `$sscanf`, `$value$plusargs` in a position not evaluated exactly once | the call would run a different number of times than written |
| a real operand to `$countones`, `$onehot`, `$onehot0`, `$isunknown` or `$countbits` | it would count bits of IEEE-754 storage |
| a `%b` / `%h` / `%o` / `%x` conversion with a real argument | a real has no radix form; use `$realtobits` |
| a sampled-value function with arity other than 1, a non-simple-signal argument, or a hierarchical signal | one previous-value register per name, and a hierarchical name would alias |
| `new[n]` outside `d = new[n]` | the allocation is a statement effect |
| a bare `$` outside a queue element select | `` `$` is only valid inside a queue element select (`q[$]`) `` |
| `randomize() with {…}` as a nested value expression | statement or direct-assign right-hand side only |

### Names not recognised

| Name | Kind | Outcome |
|---|---|---|
| `$printtimescale` | task | `VITA-W3056`, warn and skip |
| `$assertcontrol` | task | `VITA-W3056`, warn and skip |
| `$system`, `$sdf_annotate`, `$dumpports` and its variants, and every other unlisted `$task` | task | `VITA-W3056`, warn and skip |
| any unlisted `$function` in an expression | function | `VITA-E3009`, `unsupported system function in expression` |
| `$fflush` | task | accepted and dropped with no diagnostic at all — writes are unbuffered, so the warning would be misleading and the output bytes are identical either way |
| a read from the stdin descriptor | — | `VITA-W4022` and −1; a stdin-driven simulation would not be byte-deterministic |
