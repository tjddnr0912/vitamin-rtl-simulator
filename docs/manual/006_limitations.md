# Limitations

This chapter states what a vitamin user actually hits: the places where the simulator
does something simpler than IEEE 1800 describes, the constructs it refuses outright, the
handful of values it answers differently from Icarus Verilog and Verilator, and the
resource caps that bound a design. Every entry describes the shipped tool. Open work is
tracked in [`../ROADMAP.md`](../ROADMAP.md) and summarised in
[`../REMAINING_WORK.md`](../REMAINING_WORK.md).

Three classes run through the chapter:

- **Deliberate simplification** — vitamin implements a narrower behaviour than the LRM
  describes, and the difference is either invisible to a design or a documented superset
  of what was asked for.
- **Loud refusal** — the construct is diagnosed with a stable message code and the run
  stops. No value is produced, so no wrong value can be believed.
- **Known divergence** — vitamin produces a value that differs from the reference
  simulators. These are the residues of the correct-or-loud rule that are still open;
  §3 lists every one.

The rule the project holds itself to is that a divergence is either loud or listed. A
diagnostic code carries three interchangeable spellings — the mnemonic
(`E-ELAB-UNSUPPORTED`), the printed number (`VITA-E3009`), and that number bare
(`E3009`) — and `vita explain <CODE>` describes one. See
[Error Codes](007_error-codes.md).

---

## At a glance

| Limitation | Class | Section |
|---|---|---|
| Out-of-range and unknown array indices read `X` and drop the write | deliberate simplification | §1.1 |
| Seven of IEEE 1800's seventeen scheduling regions are modelled | deliberate simplification | §1.2 |
| `$stop` terminates the batch run | deliberate simplification | §1.3 |
| `unique` / `priority` report the no-match case only | deliberate simplification | §1.4 |
| `%u` / `%z` consume an argument and emit no text | deliberate simplification | §1.5 |
| `casez` reads an explicit `x` in a label as a don't-care | deliberate simplification | §1.6 |
| A null-handle dereference degrades instead of erroring | deliberate simplification | §1.7 |
| Implicit nets are inferred only in IEEE 1364 §3.5 positions | deliberate simplification | §1.8 |
| A 2-state cast evaluates its operand once per coerced bit | deliberate simplification | §1.9 |
| `$dumpvars` selects, and delays are inertial | deliberate simplification | §1.10 |
| `automatic` block-locals flatten to one static net per name | loud refusal | §2.1 |
| A framed subroutine body cannot perform a statement-level side effect | loud refusal | §2.2 |
| `$finish` / `$stop` reached inside a subroutine body | loud refusal | §2.3 |
| A default argument whose names bind differently at the call site | loud refusal | §2.4 |
| The bare label of a `for`-generate block | loud refusal | §2.5 |
| A subroutine declared inside a generate block, called from outside it | loud refusal | §2.5 |
| A real value where the language requires an integral constant | loud refusal | §2.6 |
| Side-effecting system functions in re-evaluated expression positions | loud refusal | §2.7 |
| Enum methods on a label, and on labels valued from a `parameter` | loud refusal | §2.8 |
| Net kinds `trireg` / `supply0` / `supply1` / `tri0` / `tri1` / `triand` / `trior` | loud refusal | §2.9 |
| Constructs with no grammar arm (DPI-C, `specify`, strengths, switch primitives, …) | loud refusal | §2.10 |
| Refusals by language area | loud refusal | §2.11 |
| Ten value-level divergences from Icarus Verilog and Verilator | known divergence | §3.1 |
| Splits where the reference tools disagree with each other | known divergence | §3.2 |
| Platforms, exit codes, threads, and every resource cap | — | §4 |

---

## 1. Deliberate simplifications

### 1.1 Out-of-range and unknown array indices

A word index outside an unpacked array's declared bounds is not clamped to the nearest
element. The read returns all-`X` and the write is dropped, so neighbouring elements are
never corrupted.

```systemverilog
reg [7:0] mem [0:3];
i = 9;
mem[i];         // read  -> 8'hxx  (not mem[3])
mem[i] = 8'hAA; // write -> ignored; mem[0..3] unchanged
```

An `x`/`z` index behaves the same way. The two cases carry different codes because they
are different situations: reading `mem[idx_q]` while `idx_q` is still `X` is ordinary
RTL during reset.

| Situation | Code | Severity | Effect |
|---|---|---|---|
| Known index past the end | `E-RUN-RANGE` / `VITA-E4002` | Error, exit 1 | read `X`, write dropped |
| Unknown (`x`/`z`) index | `W-RUN-RANGE-UNKNOWN` / `VITA-W4029` | Warning | read `X`, write dropped |

Both diagnostics name the array and the source line that touched it, so one table read
from several places produces several distinct reports rather than N identical lines:

```
d.sv:11:5: warning[VITA-W4029] W-RUN-RANGE-UNKNOWN: array word index of `t.tab` is
unknown (x/z); read X / write ignored [in t] [at time 0]
```

Each kind has its own budget of 8 reports, after which one line reads `further
out-of-range diagnostics suppressed` (or `further unknown-index diagnostics
suppressed`). The budgets are independent, so a reset window full of unknown-index
warnings cannot hide the genuine error.

Sub-dimension over-indexing of a multi-dimensional unpacked array is bounds-checked:
`g[0][5]` on `reg [7:0] g [0:1][0:3]` emits `E-RUN-RANGE`, reads `xx`, and exits 1.
Declared-range normalisation for non-zero and descending bases (`mem[4:7]`, `mem[3:0]`)
is applied.

One anchoring choice is coarser on purpose: an out-of-range access inside a `function`
or `task` body is reported at the calling statement, not at the subscript inside the
body. The compiled backends record such an access and report it at the caller's
statement boundary, so the same design prints the same line under every `--backend`
setting.

### 1.2 Scheduling regions

IEEE 1800 defines seventeen scheduling regions. The engine models seven. What the omitted
ones would carry is either refused outright or approximated in a region that is modelled;
the last row says which.

| Region | Where it lives in the engine |
|---|---|
| Preponed | `SimState::preponed_buf`, snapshotted at time 0 and at every time advance, committed at the clocking edge — this is what makes `cb.sig` sample the slot-entry value (§14.13) |
| Active | the slot's active queue, drained after the continuous-assign settle |
| Inactive | the slot's inactive queue (`#0` and an inactive-region delay), promoted wholesale to Active |
| NBA | the scheduler's non-blocking update list plus a delayed-update map |
| Observed | the deferred-observed list, where `assert #0` matures (§16.4) |
| Reactive | the deferred-reactive list, where `assert final` matures |
| Postponed | the postponed flush — `$strobe`/`$fstrobe` first, then `$monitor`/`$fmonitor` |
| the Pre-, Post- and Re- variants | not modelled. A clocking skew other than `#1step` would need one, and is refused loudly. A `program` block runs, but its processes are scheduled in Active rather than in the Reactive stratum IEEE 1800 §24 gives them |

These regions are engine-side structures. The frozen IR's `RegionTag` enum rides
`SuspendState.wake_key`, which no engine code reads; the only scheduling-region field in
the IR that the engine consumes is a delay terminator's own two-valued region
(active or inactive). A terminating `$finish`, `$stop` or `$fatal` drains the deferred
lists and then flushes the postponed region before returning, so a `$strobe` or a
matured `assert #0` in the same slot is not lost.

Concurrent SVA is implemented, not deferred: `assert property`, `cover property`,
sequences with `##N`, `|->` and `|=>` all run and report, and a property wrapped whole in
parentheses is the same property as the unwrapped one. `clocking` blocks work for input
sampling, `@(cb)`, `#1step` skew and anonymous blocks. An `output` direction is accepted,
but a signal merely declared as a clocking output is not driven correctly — see §3.1.
What stays loud is spelled out at the point of use:

- a clocking skew other than `#1step` (`#0`, `#N`, `##N`) — these need a sampling region
  the engine does not model
- an `inout` clocking variable, and a clocking item bound to something other than a net
- a multi-event or level clocking event (`@(posedge a or b)` is accepted for a clocking
  block; `@*` and level events are not)
- a multi-clock or OR-of-clocks property clock, a ranged / `goto` / unbounded /
  multi-clock consequent, `disable iff` combined with a property-level `and`/`or`, an
  action block on `cover property`, and a second `default disable iff` in one scope

### 1.3 `$stop` terminates the run

vitamin is a batch simulator with no interactive console, so `$stop` ends the run rather
than dropping to a resumable prompt. The termination reason is distinct and appears in
the closing line:

```
simulation ended (Stop) at time 0
errors=0 warnings=0 notes=0
```

The exit code is the same as `$finish`: `Finish`, `Stop` and `Quiescent` are all clean
terminations, and the process exits 0 unless an error or fatal was latched during the
run.

### 1.4 `unique` and `priority` report only the no-match case

`unique`, `priority`, `unique0` and `priority0` all parse, on both `if` and `case`.
`unique` and `priority` inject a synthetic no-match arm that reports
`W-RUN-UNIQUE-VIOLATION` / `VITA-W4031` with the message `value is unhandled for
priority or unique case statement`, pinned to Icarus Verilog's wording. `unique0` and
`priority0` parse as the plain statement with that report suppressed, per §12.4.2.

The multi-match uniqueness check is a documented cut. The lowered decision cascade is
first-match-wins, so an overlap between arms is unobservable in the result, and reporting
it would require a second evaluation of every arm.

### 1.5 Format specifiers that consume without printing

`%u`, `%U`, `%z` and `%Z` consume their argument and emit no text. The IEEE forms write
raw binary bytes, which are useless in a text log; Icarus Verilog emits them. `%l` is
cosmetic. Every other specifier the engine implements — including `%v` (strength form),
`%p` (assignment pattern) and the `-` and `+` flags — renders normally; see
[System Tasks](005_system-tasks.md).

### 1.6 `casez` and an explicit `x` in a label

A `casez` label bit written as an explicit `x` is treated as a don't-care, with
`W-ELAB-CASEZ-APPROX` / `VITA-W3011`. `casez` over `z` and `?`, and the whole of `casex`,
follow the precise IEEE split.

### 1.7 Null-handle dereference degrades

Dereferencing a null class handle or an unallocated dynamic-storage handle degrades
instead of aborting: a read yields `X`, a write is a no-op, and
`W-RUN-DYN-DEGRADE` / `VITA-W4020` is emitted once per net. IEEE makes this an error. The
warning is what keeps it honest — the run continues, but the transcript says the handle
was null.

The same code covers every dynamic-storage clamp or drop, so an oversized `new[n]` or a
queue push past a declared bound warns rather than silently capping.

### 1.8 Implicit nets

An undeclared name becomes an implicit 1-bit `wire` only in the positions IEEE 1364-2005
§3.5 defines, and says so with `W-PARSE-IMPLICIT-NET` / `VITA-W2003`.

| Position | Behaviour |
|---|---|
| Left-hand side of a continuous assignment | implicit 1-bit `wire` + `VITA-W2003` |
| Terminal list of a gate or module instance, either direction | implicit 1-bit `wire` + `VITA-W2003` |
| Any other position (ordinary right-hand side, procedural lvalue) | `E-ELAB-UNRESOLVED-NAME` / `VITA-E3010` |
| Under `` `default_nettype none `` | `VITA-E3010` everywhere |
| A wider assignment onto an implicit net | an additional `W-ELAB-FEATURE-LIMIT` / `VITA-W3056` naming the discarded top bits |

Inference can be turned into a project-wide hard error with
`-Werror=W-PARSE-IMPLICIT-NET`.

### 1.9 A 2-state cast evaluates its operand once per coerced bit

A cast to a 2-state type — `int'(e)`, `byte'(e)`, `shortint'(e)`, `longint'(e)`,
`bit'(e)` — has to force any `x`/`z` in `e` to `0`, and that check is built one bit at a
time. When the operand cannot be shown free of `x`/`z` at elaboration, the operand is
named once per bit of the coercion: the operand's own width for a same-width or widening
cast, the target width for a narrowing one.

| Cast over a 32-bit operand | Operand evaluations | Icarus Verilog |
|---|---|---|
| `byte'(e)` (narrowing to 8) | 8 | 1 |
| `int'(e)` (same width) | 32 | 1 |
| `longint'(e)` (widening to 64) | 32 | 1 |
| `int'(int'(e))` | 32 | 1 |
| `int'(e)` over a 4-bit operand | 4 | 1 |

For a pure expression this costs time and nothing else — the value is identical. It
matters when the operand has a side effect:

```systemverilog
int r;
initial r = int'($random);   // vitamin draws 32 times and keeps the last;
                             // Icarus Verilog draws once
```

Affected operands are the ones vitamin cannot prove known: a call to a user function, a
seeded `$random` / `$dist_*`, a file read that advances a descriptor. A 4-state cast of
the same width is unaffected (`integer'(e)` evaluates `e` once), as are size casts
(`24'(e)`), signing casts (`signed'(e)`), and any operand provably already 2-state.
Nesting does not multiply.

The workaround is to assign to a temporary first, which also makes the intent explicit:

```systemverilog
int unsigned t;
initial begin
  t = $random;      // drawn exactly once
  r = int'(t);
end
```

### 1.10 Waveform selection and delay modelling

`$dumpvars` honours both its level and its scope arguments, pinned to Icarus Verilog:
`$dumpvars(1, top)` emits `top`'s own variables and an empty scope for each child,
`$dumpvars(0, top)` emits the whole subtree, and a net argument selects one net. A level
argument with no scope argument dumps everything, and an unresolvable scope degrades to a
full dump rather than an empty waveform. A second `$dumpvars` call warns once with
`W-RUN-DUMP-MULTI` / `VITA-W4021` and is ignored — the header cannot be rewritten.

Package variables have no VCD surface. A bare dump skips them silently; an explicit
`$dumpvars` argument that selects one warns with `W-RUN-VCD-PKGVAR-SKIP` / `VITA-W4026`.

`assign #d` is inertial, matching Icarus Verilog: a pulse narrower than the delay is
absorbed. Distinct rise, fall and turn-off delays (`#(2,4)`) are honoured on gates and on
continuous assigns.

---

## 2. Loud refusals worth knowing in advance

Everything in this section stops the run with a diagnostic. Nothing here produces a
value.

### 2.1 `automatic` variables declared inside a procedural block

vitamin gives a procedural block's locals one flattened net per name rather than fresh
storage per block entry. An `automatic` block-local is accepted when that flattening is
indistinguishable from real per-entry storage, and refused when it is not:

```
t.sv:4:15: error[VITA-E3009] E-ELAB-UNSUPPORTED: an `automatic` block-local `x` whose
per-entry lifetime differs from static (an initializer, or a read before its first
write) is unsupported in a procedural block (v1 flattens block-locals to one static
net); assign it before use, or drop `automatic` [in tb]
t.sv:5:9: note[VITA-E3009] E-ELAB-UNSUPPORTED: definite-assignment for `x` stopped here:
it is read here before any write on this path. Everything after this point is treated as
if `x` were still unwritten [in tb]
```

When the refusal comes from the definite-assignment walk stopping rather than from a read
you can see, a `note` points at the construct that stopped it. That location is often
several statements after the declaration, or in another file.

What is accepted:

- A local written before it is read on every path. The analysis understands
  `break` / `continue` (a jump leaves the block, so it never carries an unwritten value to
  a later read), `case` and `if`/`else` arms, and calls to subroutines that provably
  cannot touch the name.
- A declaration initializer, which re-runs on each block entry per §6.21 — including for
  a fixed-size unpacked array (`automatic int m[4] = '{1,2,3,4};`).
- A fixed-size array filled element by element at literal indices, once every declared
  index has been written.
- A local never written anywhere in the block, for any type. There is no first write for
  a read to be before: the flattened net takes the type default once and nothing changes
  it, which is what fresh per-entry storage supplies at every entry. This is the
  idiomatic deliberately-empty argument (`byte exp [];` passed to a routine that expects
  no digest, arriving as `size() == 0` per §7.5).
- A first write that is timing-controlled. `#1 x = 7;`, `@(posedge clk) x = 7;`,
  `x = #1 7;`, `#1 begin x = 7; end` and `wait (c) x = 7;` are all blocking writes, so
  nothing runs before the write lands.
- A write inside a loop body, when the trip count proves the body runs at least once and
  nothing in it can jump past the write. The bound must be written with plain decimal
  literals. `repeat (2)`, a constant-true `while` and `forever` are answered by the same
  rule.
- A call that writes an `output` actual, in any position the statement evaluates once —
  an assignment right-hand side, an `==` operand, a `case` scrutinee, a system-task
  argument, a concatenation part, another call's argument, an lvalue index. A call in a
  conditionally evaluated operand (the right side of `&&` or `||`, a `?:` arm) counts on
  the path that actually runs, however deeply nested; an `x` condition evaluates both
  `?:` arms, so both writes happen.
- An `inout` actual, when the callee overwrites the whole formal before ever reading it —
  then nobody can observe the copied-in value. Unpacked-struct formals are answered
  member by member.
- A write to a struct member, or a hand-written pair of part-selects, when the members
  together cover the whole variable. Partial coverage stays loud.

What stays loud:

- Reading an element the block has not written this entry, and filling an array through a
  computed index (`foreach (a[i]) a[i] = …;`) — neither can be proven complete, so the
  local is rejected rather than given the previous entry's leftover.
- A subroutine whose body can reach the flattened name, by the bare name or through a
  hierarchical path. Such a call counts as a read.
- Two blocks where one encloses the other and both declare the same name. That is
  shadowing, which the flattening cannot resolve. Two disjoint sibling blocks reusing a
  name are fine at any nesting depth. A collision with an existing net of the same name
  gets its own message telling you to rename.
- Two blocks sharing one flattened variable, where either can let simulation time
  advance. Suspending hands the scheduler to the other block, which writes the one
  variable, so a later read here would see a value its own storage never held. This
  includes calling a subroutine that waits, and it applies whether or not this block has
  already written the variable. Declaring the locals `automatic` avoids the sharing.
- A write reached only through a `?:` branch — exactly one arm runs, so neither arm's
  write is guaranteed. The same holds after a loop whose condition was `a && f(r)`: the
  loop can exit because `a` was false, in which case `f` never ran.
- A read elsewhere in the same expression that a pre-call copy cannot serve: a read
  spelled as a hierarchical path, a read inside a called function's body, a variable that
  is not a plain bit vector, or two calls in one expression both writing it. A read to
  the right of the call (`g = f(r) + r;`) sees the written value, and a read to the left
  (`g = r + f(r);`) is served by a copy taken first.
- A hierarchical reference to an `automatic` block-local (`tb.a`, or `t.a` from a task in
  the same module). §23.9 forbids it — automatic storage has no static address to name —
  and accepting it would let an outside write reach per-entry storage. The message names
  the path and the two fixes: drop `automatic`, or move the declaration to module scope.

A `function` body is the one place a call that writes an `output` actual cannot be
carried. A function is entered from the expression that calls it, so it has no call
statement of its own to hold the callee's copy-out. The diagnostic names that cause
directly and says the same call works in a `task` body or in a module process. Assigning
the call to a temporary first also resolves it.

Storage lifetime follows the LRM: automatic storage is created per activation, not per
block entry, so a local without an initializer keeps its value across loop iterations of
the same activation and is fresh on each new call.

### 2.2 Statement-level side effects inside a framed subroutine body

`$fgets`, `$fscanf`, `$sscanf`, `$fread`, `$fgetc`, `$ungetc`, `$fopen`,
`$value$plusargs`, `$cast`, a queue pop, and a seeded `$random` / `$dist_*` all write
their destination as a statement-level effect. The synchronous frame executor cannot
perform that effect, so reaching one there is a fatal rather than a silent zero:

```
fatal[VITA-F4004] F-RUN-FATAL: `$fgets` does its work as a statement-level effect, which
the synchronous `&self` frame executor cannot perform — the call would return 0 and
leave its destination untouched, so the run stops here rather than continuing on that
value. This is one of the few positions vita cannot route to the statement executor: a
class-method body, a CONTINUOUSLY re-evaluated expression (`assign`, `force`, a `wait`
condition), or an intra-assignment delay (`x = #1 f(...)`). It DOES work in a module
process, and in a task or function called from a statement — with or without
`automatic`, with or without output formals. Call it there, assign the result to a
variable, and use that variable here.
```

The three positions the message names are the whole list. A task or function called from
an ordinary statement can do the read, with or without `automatic` and with or without
output formals. The idiomatic vector-file walker reads in the process and parses in the
subroutine:

```systemverilog
initial begin
  fd = $fopen("vectors.rsp", "r");
  rc = $fgets(line, fd);          // read in the process ...
  while (rc != 0) begin
    parse_line(line, r);          // ... parse in the subroutine
    rc = $fgets(line, fd);
  end
end
```

A related fatal covers a write that leaves the frame entirely: a subroutine on the
synchronous frame executor that tries to write a module or instance net is stopped with a
message naming the net and pointing at the two routes that work — move the write into a
`task` (a task body's out-of-frame write routes automatically), or into the calling
process. Writing an element of a dynamic-array `input` formal, and iterating an
associative array whose key variable is not a body-local, are refused there for the same
reason.

An `input` dynamic-array formal receives a snapshot of the caller's array taken
immediately before the calling expression runs, so the callee sees a stable array even
when the caller mutates it. A dynamic-array formal requires a bare matching
dynamic-array actual; a select, a literal, or a package-scoped array is refused by name.

### 2.3 `$finish` and `$stop` reached inside a subroutine body

A `$finish` or `$stop` written inside a `function` or `task` body is accepted at
elaboration — a parameter check whose `else` arm prints an error and stops is a real
pattern in library RTL, and a branch a design's own parameters never take must not refuse
the design. It is refused only when it is actually reached:

```
fatal[VITA-F4004] F-RUN-FATAL: `$finish` was reached inside a subroutine body. vita ends
the run with an error instead of performing it, because a body that stops half-way still
owes its calling expression a value and the reference simulators disagree about which
one. The rest of this body still executes, as it does after a `$fatal`. Move the
`$finish` to the caller.
```

The disagreement is real: Icarus Verilog does not perform the assignment at all,
Verilator runs the body to completion, and vitamin would commit whatever the return slot
holds. Picking any of them would replace a loud refusal with a silently wrong number. As
the message says, the boundary is the enclosing statement, so a `$display` after the
`$finish` still prints.

Two shapes escape this. A subroutine whose enclosing routine has an `output` formal is
lowered as a call terminator and reaches the statement executor, which performs the
`$finish`: the run ends cleanly with `simulation ended (Finish)` and exit 0, on the same
source construct. And a `$finish` in a plain task that the caller does not wait on a
value for ends the run cleanly, as always. Moving the `$finish` to the caller makes the
behaviour uniform.

### 2.4 Default argument values

A subroutine's default argument value is evaluated in the scope where the subroutine is
declared (§13.5.4), not at the call site. vitamin lowers it at the call site, which gives
the same answer whenever both scopes see the same object — a default naming a module net
resolves outward to that net from a module process, from a generate block, and from
another subroutine's body alike.

When the two scopes would differ, because the caller declares its own variable of that
name, the call is rejected rather than quietly binding to the caller's:

```systemverilog
int g = 5;
task automatic tw (output int x, input int y = g); x = y + 1; endtask

task automatic outer();
  int g;              // shadows the module `g` at this call site only
  g = 90;
  tw(a);              // VITA-E3009: the default's names bind differently here
endtask
```

Pass the argument explicitly (`tw(a, g)`), or make the default a literal or a `pkg::`
constant. A class method or constructor with a non-literal default argument is refused at
declaration for the same reason, with a message that says the value would resolve in the
caller's scope rather than the class scope.

### 2.5 Naming a generate block from outside

A named generate block is referenced by its bare label. A conditional block
(`if` / `if…else` / `case`) and a bare `begin : name` are singletons, so no index appears:

```systemverilog
module dut;
  generate if (WIDE) begin : g
    logic [7:0] x;
  end endgenerate
endmodule

module tb;
  dut u();
  initial $display("%h", u.g.x);   // reads, and `u.g.x = 8'hA5;` writes
endmodule
```

A `for`-generate is different: §27.4 makes its blocks an array, so the index is required
and a bare label is refused with `E-ELAB-UNRESOLVED-NAME` / `VITA-E3010` — at every trip
count, including a loop that runs exactly once.

```systemverilog
generate for (genvar i = 0; i < N; i++) begin : gl
  logic [7:0] x;
end endgenerate
...
u.gl[0].x       // correct
u.gl.x          // VITA-E3010, as in Icarus Verilog
```

vitamin also accepts the redundant `u.g[0].x` on a singleton block. That spelling is a
vitamin extension — Icarus Verilog and Verilator both reject it — so write the bare label
if the design has to build elsewhere. A missing leaf inside a resolved scope, and a scope
that does not exist, both stay loud.

A `function` or `task` declared inside a generate block (IEEE 1800 §27.3) is supported,
and it belongs to that block's scope. Only the taken branch of a generate-`if` declares
one; a generate-`for` body declares one per iteration, each seeing its own genvar value;
a bare call resolves innermost-first, so a generate-scoped `f` shadows a same-named module
`f`; and `%m` inside the body names the declaring block (`t.u.g.show`).

The scope is the limit. The name does not leak outward, so a call from the enclosing
module is `E-ELAB-UNRESOLVED-NAME` / `VITA-E3010` — Icarus Verilog agrees ("Enable of
unknown task"). A hierarchical call THROUGH the block is a gap rather than an agreement:

```systemverilog
generate if (1) begin : g
  function automatic logic [7:0] f (input logic [7:0] v); f = ~v; endfunction
end endgenerate
...
u.g.f(8'h1)     // VITA-E3009, unsupported hierarchical function call — Icarus Verilog runs it
```

And a generate-scoped routine cannot be folded at elaboration time: a `localparam W =
f(N)` written inside the same block is `VITA-E3009` (`… value is not a constant`), because
the constant-function interpreter reads module-body declarations only. Move the function
to module scope for either case. `defparam` inside a generate block remains deferred.

### 2.6 Reals where the language requires an integral constant

`parameter real` and `localparam real` are supported: they bind, they participate in real
arithmetic (`R/2` divides in the real domain), and they can be overridden with any value
that folds to an integer.

Where the language requires an integral constant — a range or width bound, a replication
count, a select range, the value of an integer-typed parameter — name the conversion you
mean and it works:

```systemverilog
localparam real R = 3.5;
logic [int'(R)-1:0]   v;   // 4 bits — int'() rounds half AWAY from zero (§6.24.1)
logic [$clog2(R)-1:0] w;   // 2 bits — converts first, then takes the log
wire  [7:0] x = {int'(R){1'b1}};
localparam int  N = $rtoi(R);   // 3 — $rtoi TRUNCATES where a cast rounds
localparam int  M = R * 2.0;    // 7 — a declared integral type is a boundary too
```

The whole expression evaluates in the real domain and only the converted result becomes
an integer, so `R/2` is still `1.75` and `generate if (R/2 > 1)` tests `1.75 > 1`.
Converting the real at the leaf instead would decide that branch on the wrong value,
silently, which is why the implicit conversion is the one spelling that stays refused:

```systemverilog
logic [R-1:0] y;            // VITA-E3009
wire  [7:0] z = {R{1'b1}};  // VITA-E3009
```

That refusal is not a missing feature. The reference tools disagree about these in
opposite directions: Icarus Verilog sizes `[R-1:0]` as 3 bits while Verilator rejects the
design outright, and for `{R{1'b1}}` Icarus Verilog rejects while Verilator replicates.
With no agreed answer to match, vitamin asks for the conversion.

Also refused, each with its own message: a `parameter real` in an interface body; a
hierarchical reference to another instance's real parameter; overriding a parameter with
a real value (`#(.R(2.5))`); a real value in an untyped parameter later used as an
integer (`localparam M = R/2.0;` — an untyped parameter takes its type from its value, so
that one is a real parameter); and `1.0/0.0`.

A `real` or `string` package parameter folds and keeps its domain through the scope
operator (`pk::R`, `pk::S`) and through a direct import of the name. Reaching one through
a wildcard import (`import pk::*;` then a bare `R`) is refused: the package fold succeeds
and only the import binding is missing, so it is loud, never silent.

Every arithmetic and comparison operator folds a named parameter at any width, including
`/`, `%`, `**`, `<<<`, the reductions, `$clog2`, `$bits` and `$isunknown`. Two boundaries
are deliberate. A constant `x / 0` or `x % 0` is `x` under §11.4.3 and a parameter has no
unknown plane, so the fold declines and the caller stays loud with the divisor named. And
the super-linear division kernels are budgeted at elaboration: a constant `/` or `%` up to
about 65536 bits folds, and a wider one is refused in milliseconds instead of running for
minutes. The runtime lane answers that same shape with `X` above its own cap (§4.5).

### 2.7 Side-effecting system functions in re-evaluated positions

Any system function that advances a file position, or writes through an argument, is
lowered as a statement so it evaluates exactly once. Positions where the count of
evaluations would differ from what the source says are refused with the longest message
in the tree:

> `{}` advances the file position, so vita evaluates it into a temporary once, before the
> statement runs. This position is refused because the call would then happen a DIFFERENT
> NUMBER OF TIMES than written: the right operand of `&&` / `||` and an arm of `?:` may be
> skipped, a loop condition is re-evaluated per iteration, and a `$monitor` / `$strobe`
> argument is re-rendered on every later change and would show the frozen temporary. Read
> it into a variable in its own statement first, then use that variable here. Placements
> evaluated exactly once per execution are supported — a blocking or nonblocking rhs, an
> `if` condition, a `case` scrutinee, a `repeat` count, a `$display`-style argument, an
> lvalue index

`$value$plusargs` carries its own twin of that message, phrased around writing rather
than advancing a position. On top of the position rule, each function checks its own
shape: `$fopen` and `$sformatf` must be the direct right-hand side of a blocking
assignment, `$fscanf` / `$sscanf` need a string-literal format and plain-variable
destinations, `$fread` needs a whole memory or a variable rather than an element select,
and `$timeformat` takes zero or four arguments. An intra-assignment delay on any of them
is refused by name.

### 2.8 Enum methods

The enum methods `name`, `next`, `prev`, `first`, `last` and `num` are defined on a
variable of the enum type, not on a label. Calling one on a label is refused, and both
reference tools refuse it too — Icarus Verilog 13 aborts inside `elab_expr.cc` and
Verilator 5.050 reports `Can't find definition of task/function: 'name'`. Declare a
variable and assign the label:

```systemverilog
pk::e_t v = pk::LA;
$display("%s", v.name());   // LA
```

A separate limit applies when a label's value is not a parse-time constant, because enum
methods are resolved during parsing, before any instance override is known — and an
override really does move the labels:

```
error[VITA-E3009] E-ELAB-UNSUPPORTED: enum method `v.name` is unavailable: the enum type
of `v` was not registered, which happens when a label's value is not a parse-time
constant (an overridable `parameter`, or a sized literal). Give the labels plain decimal
`localparam` / literal values — the VALUES themselves are correct either way, it is only
the methods that are lost
```

`localparam L = 5; typedef enum { A = L, B = L+1 }` works, methods included. A sized
literal value (`localparam L = 8'h5`) does not fold, so it hits the same message.

### 2.9 Net kinds that parse but do not elaborate

`triand`, `trior`, `tri0`, `tri1`, `supply0`, `supply1` and `trireg` parse and are then
refused with `E-ELAB-UNSUPPORTED` / `VITA-E3009` `unsupported net/var kind (v1)`. The net
kinds that elaborate are `wire`, `tri`, `uwire`, `wand` and `wor`; `tri` and `uwire`
collapse to a plain wire with no strength or pull semantics, and only `wand` and `wor`
carry real wired resolution (`z` is the identity; a `0` forces `0` on `wand`, a `1` forces
`1` on `wor`, otherwise `x`), pinned to Icarus Verilog 13.

Multiple whole-net, non-delayed continuous drivers on one net are legal and resolved by
4-state wire resolution. Overlapping partial drivers — where any driver is delayed,
multi-chunk, an array element, or a bit or part select, and the bit intervals overlap —
are refused with `E-ELAB-MULTIDRIVER` / `VITA-E3001`.

### 2.10 Constructs with no grammar arm

These reach no parser rule, so the word lexes as an ordinary identifier or keyword and
the construct dies at `E-PARSE-UNEXPECTED-TOKEN` / `VITA-E2002`.

| Area | Constructs |
|---|---|
| Foreign interface | `import "DPI-C"`, `export "DPI-C" function` — a permanent non-goal |
| Timing and paths | `specify` / `endspecify`, `edge` event control, `ifnone`, `showcancelled` / `noshowcancelled`, `pulsestyle_*` |
| Strengths and switches | drive strengths (`assign (strong1, strong0) y = a;`), `pullup` / `pulldown`, the MOS and bidirectional primitives (`cmos rcmos nmos pmos rnmos rpmos tran tranif0 tranif1 rtran rtranif0 rtranif1`), `vectored` / `scalared` |
| Configuration | `config` / `endconfig`, `library`, `liblist`, `cell`, `design`, `use`, `incdir`, `instance` |
| Types | `chandle`, `shortreal`, `tagged`, `nettype`, `untyped` |
| Compilation units | `timeunit`, `timeprecision`, nested modules, `extern module`, `export` |
| Verification | `checker`, `randcase`, `randsequence`, `expect`, `restrict`, `wait_order`, `first_match`, `intersect`, `solve` / `before`, `constraint_mode`, `rand_mode`, `pre_randomize` / `post_randomize` |
| OOP | `virtual class`, `interface class`, `implements`, `forkjoin` |
| Standard package | `std::`, `semaphore`, `mailbox` |
| Statements | a comma-separated `for`-init (`for (i=0, j=0; …)`) |

A scalar unpacked struct is supported, and so is a one-dimensional fixed array or an
unbounded queue of one (`t a [0:3];`, `t q [$];`), with `a[i].field` selecting a member:
a packable record lowers to a packed element vector, and a record carrying a `string` or
`real` member lowers to one array per member. What stays loud, with a message naming the
shape, is a second unpacked dimension, a bounded queue (`[$:N]`), a declaration
initializer on such an array, and a packed-struct member inside an unpacked struct.
Parameterized classes are monomorphized in the parser and work.

`shortreal`, `trireg`, UPF, SDF, synthesis, a waveform GUI, the UVM ecosystem, and a VHDL
front end are permanent non-goals rather than gaps. `defparam` is IEEE-deprecated and
implemented only for a direct-child `instance.param` target with a constant value; a
multi-level path and a non-constant value are refused by name.

### 2.11 Refusals by language area

Beyond the entries above, each area carries its own refusals; every one names the
construct and, where a workaround exists, states it in the message. This table says where
to expect them.

| Area | What stays loud |
|---|---|
| Interfaces | nested instances, `generate`, `function` / `task` items, `typedef`, `defparam` and `clocking` inside an interface body; interface instance arrays; non-ANSI or interface-typed header ports; a virtual interface that is unbound, bound to a non-interface, or re-bound |
| Classes and OOP | array members and arrays of handles; `real` and `string` members; non-constant field initializers; a method with an output/inout port or a `string` return; a class handle as a port, with dimensions, or with a declaration initializer; down-casts and unrelated casts; mixing a handle with an integral value; access-control violations; unknown members and constructor arity |
| Randomisation | a `rand` handle member; a `randc` field wider than 16 bits and any constraint referencing one; a constraint whose operands do not fit a signed 64-bit predicate; a constraint over a non-`rand` name; a non-constant `dist` bound or weight; a `soft` qualifier inside an inline `randomize() with` block, which takes hard constraints only; `randomize()` with positional arguments or as a nested value expression |
| SVA | the clocking and cross-clock limits of §1.2; `within` outside a top-level antecedent; `throughout` over an unbounded or `goto` sequence; a leading empty-match repetition; a nested `always`; a property-level `and`/`or` over non-boolean or mixed-skew operands; named-property consequents in several shapes; recursion outside a `\|=>` consequent; liveness shapes other than `s_eventually p`, `req \|-> s_eventually p` and `lhs s_until rhs`; sequence local variables outside a single capture on a fixed-delay antecedent |
| Functional coverage | more than 64 explicit bins, an array bin or a cross that exceeds the 64-bin bitmap; a fixed-size bin array; an `iff` guard on ignore/illegal bins; a cross of an `iff`-guarded coverpoint; `option.at_least > 1` on auto bins; non-constant bin values and options; a discarded `get_coverage()` result |
| Arrays and dynamic storage | partial unpacked slices; a whole array used as a value or a formal; shape, direction or element-type mismatch in an array assignment or port bind; assignment patterns that do not match the dimension exactly; nested or chained selects; a whole-handle read or assign; a queue pop or `new[n]` outside a direct blocking right-hand side; nested dynamic storage; `real` / `event` elements; a dynamic handle as a port or in an event control; delayed dynamic assignments; wildcard associative keys `[*]` |
| Strings | a string as a port or with dimensions; a block-scope declaration initializer; a runtime index into a string array; a non-blocking write to a string element; a string inside a concatenation lvalue; a non-constant replication count; an unknown method or a chained call on a non-string result |
| Hierarchical references | element and part-select writes; reads and writes of a named event, a dynamic handle, or a whole unpacked array; multi-dimensional packed part-selects; ascending indexed part-selects; a parameter select needing a declared width; a hierarchical name in an event control; a hierarchical `force` / `release` of a select; a hierarchical call with named arguments |
| Packages | anything but parameters, typedefs, functions, tasks and plain variables in a package body; net declarations; dynamic storage; an `import` inside a `generate`; an import that collides with a local declaration; a part-select of a package array element |
| Parameters and overrides | a real, non-constant, or x/z-bearing override; an array-element-select override of an untyped parameter; array-parameter element-count and width mismatches; `parameter type` defaults that are not integral; a class-handle parameter; an overridable array `parameter` in a module body; an override expression whose top operator takes its width from the context |
| Statements and events | single-bit level event control; a non-LSB edge bit-select; a complex event term; an `iff` guard on a multi-term event control; a multi-term in-body edge wait; reading or assigning a named event; a `disable` that is not a lexically enclosing named block; timing controls inside a `final` block; `assign` / `deassign` / `force` / `release` on a select; a runtime `repeat(n)` in an intra-assignment control; a constant shadowing a net as an lvalue; a duplicate declaration |
| Instances | `.*` on an instance array; an instance array without exactly one `[msb:lsb]` range, or with a non-constant range; a non-ANSI child; a non-identifier port connection; a width mismatch; recursive module instantiation; no top module, or an unknown `--top` |

---

## 3. Known divergences from the reference tools

The rows below produce a value that differs from Icarus Verilog 13.0 or Verilator 5.050.
They are the residues of the correct-or-loud rule that are still open, and they are
tracked in [`../ROADMAP.md`](../ROADMAP.md) §2. Everything else in this chapter either
matches the reference tools or refuses loudly.

### 3.1 Value-level divergences

| Construct | vitamin answers | Reference answers | Workaround |
|---|---|---|---|
| A module-scope `localparam` mixing a signed narrow name into a wider unsigned expression: `localparam logic signed [7:0] NM = -8'sd2; localparam logic [63:0] XM = NM ^ 64'h0;` | `fffffffffffffffe` | Icarus Verilog and Verilator: `00000000000000fe` (§11.8.2 converts at the operand's own width) | Compute it in a `function automatic` local; the same expression over a function local folds `00000000000000fe` |
| A parameter override carrying `x` or `z` onto a 4-state parameter: `leaf #(.K(8'b1010_010x))` | `10100100`, unknown plane dropped, no diagnostic | both tools keep the `x` | Do not carry `x`/`z` through an override; drive the value from RTL |
| A parent `initial` reading a child net at time 0: child has `initial s = 8'hEE;`, parent does `r = u1.s;` in its own `initial` | `xx` | both tools: `ee` | Read the child value after a `#0` or `#1`, or through a port bind or continuous assign — all three read `ee` correctly |
| A range bound taken from a select of a parameter wider than 64 bits: `parameter [135:8] K = …; wire [K[31:24]-1:0] n;` with `K[31:24]` equal to 222 | `$bits(n)` is 1, exit 0, while `$display` of the same select prints 222 | both tools declare 222 bits | Hoist the select into a `localparam` narrower than 64 bits, then use that in the bound |
| `$random` or `$time` inside a function body reached from a continuous assign: `wire [7:0] m = f(8'd5);` where `f` adds `$random` | re-draws on every settle pass, so `m` changes across passes | both tools freeze the value | Assign it once in an `initial` or `always` block rather than a continuous assign |
| `%p` of an associative array with a negative integer key | signed-order iteration at 64-bit key width: `K='{'hffffffffffffffff:'h7, 'h2:'h8}` | Verilator sorts the rendered hex and prints the declared `int` width | Use non-negative keys; every workload-corpus design does, and they agree exactly |
| A signal declared as a `clocking` output (`clocking cb; output q; endclocking`) | driven to `x`, or frozen | Verilator: the expected sequence | Avoid `clocking` output declarations. Icarus Verilog 13 cannot parse `clocking`, so only one reference tool can be consulted here |
| VCD `$scope` naming of generate blocks | `gi[0]` / `genblk1[0]` | Icarus Verilog: `begin gi` / `begin genblk1` | Cosmetic in the waveform tree; name the generate block explicitly |
| `%m` inside a concurrent `assert property` action block | omits the assertion label (`top.nb`) | Verilator: `top.nb.ap` | Print the label yourself |
| An enum label that shadows an outer array name | reads the array; a `foreach` over the shadowed name yields `i=0` | Verilator reads the label and yields `i=31`; Icarus Verilog agrees with vitamin | Rename to disambiguate |

One construct answers differently depending on a detail that should not matter: a
`$finish` reached inside a subroutine body is a fatal at exit 1 when the enclosing
routine has no `output` formal, and is performed at exit 0 when it has one. See §2.3.

### 3.2 Splits where the reference tools disagree with each other

These are picks, not gaps. Where the two reference tools give different answers, vitamin
follows one and says which.

| Construct | vitamin | Reference |
|---|---|---|
| `%p` of an unpacked array of `real` | `R='{1.5, -0.25}` | Verilator prints only element 0, and renders a queue of the same shape correctly — it contradicts itself, so it is not an oracle here |
| `%p` with `x`/`z` digits | `X='{'hx5, 'hzz}` | Verilator is 2-state and cannot compile the assignment |
| `$bits` of a `string` parameter | 16 (§6.16), matching Icarus Verilog | Verilator: 64 |
| `v['1]` (a fill used as an index) | 0, matching Icarus Verilog | Verilator: 1 |
| Widths of `'1 * 2'd2`, `'1 + 1'b1`, `4'd8 - '1` | 2 / 1 / 4, matching Verilator | Icarus Verilog: 3 / 2 / 5. The values agree in all three tools |
| A non-zero-LSB enum base (`enum logic [39:8] {…}`) in a constant select | refused | Icarus Verilog reads the label at the base's width; Verilator honours the declared LSB. Declare the base with a zero LSB and every tool agrees |
| `$readmem*` ordering against a child `initial` | matches Verilator (§4.7 leaves `initial` order nondeterministic) | Icarus Verilog differs |
| `$readmemh` into a `wire` array | accepted | Icarus Verilog refuses; Verilator accepts |
| A header default naming a constant from a body import | accepted | Icarus Verilog rejects; Verilator folds |
| A non-standard string escape such as `"\r"` | `0x0D`, with `W-ELAB-STR-ESCAPE` / `VITA-W3059` naming both readings and suggesting `\015` or `\x0D` | `0x0D` in Verilator; the letter `r` in Icarus Verilog and Xcelium |
| `%m` inside a subroutine declared in a generate block | `t.u.g.show` — the block scope once, matching Icarus Verilog | Verilator repeats the label: `t.u.g.g.show` |
| A bit or part select on a base §11.5.1 disallows | accepted, with `W-PARSE-SELECT-BASE` / `VITA-W2004` | Icarus Verilog rejects all four forms; Verilator rejects two and accepts two |

---

## 4. Platform and resource limits

### 4.1 Platforms

| Platform | Status |
|---|---|
| Linux (`ubuntu-latest`) | built, and the full test suite runs, in CI |
| macOS (`macos-latest`) | built, and the full test suite runs, in CI |
| RHEL 9 / UBI 9 (container) | built, and the full test suite runs, in CI; needs `dnf install -y gcc` for a C linker |
| Windows | absent from CI. The code carries Windows-aware paths, but nothing builds or tests it |

See [Installation](001_installation.md). The toolchain floor is rustc and cargo 1.85.0,
pinned by `rust-toolchain.toml`; `blake3` is pinned at `=1.8.2` and `cranelift-*` (behind
the off-by-default `jit` feature) at 0.120. `--locked` is required for reproducibility
across the three CI platforms. The one `unsafe` block on the product path is a
`signal(2)` call restoring `SIG_DFL` for SIGPIPE on Unix.

### 4.2 Exit codes, signals and panics

| Code | Meaning |
|---|---|
| 0 | clean run |
| 1 | user or design error (lex, parse, elaborate, or a runtime fatal) |
| 2 | stale artifact — the magic, schema hash, format version or producer version does not match. Rebuild with `vcmp` / `velab` rather than debugging RTL |
| 3 | CLI or usage error (no sources, file not found, unknown applet) |
| 101 | a panic anywhere in the pipeline, deliberately not a vitamin exit class |
| 141 | broken pipe on Unix. SIGPIPE is restored to `SIG_DFL` at startup, so `vita design.sv \| head` terminates conventionally rather than panicking |

The `VITA-E9xxx` family is the only one that produces exit 2.

### 4.3 Threads

`--threads N` (or `-j N`) is accepted by every applet, and `VITA_THREADS` overrides the
automatic value. Resolution order is the explicit flag, then the environment variable,
then automatic. The CLI's automatic value is `min(available_parallelism(), 8)` with a
floor of 1; the library default is 1.

`N >= 2` moves VCD file writes onto one dedicated writer thread behind an
order-preserving bounded FIFO, 64 KiB buffered. Nothing else is parallel — the simulation
kernel is single-threaded. The output contract is that the VCD, stdout and exit code are
byte-identical for every `N`; only wall-clock changes. FST targets keep the sidecar VCD
single-threaded so the file is flushed and closed before transcoding, so `--threads` has
no effect on them.

`run()` executes on a spawned worker named `vita-main` with a 256 MiB stack, so depth caps
report cleanly instead of overflowing.

### 4.4 Design-size caps

Every bound here is fail-closed: exceeding one is a loud refusal, never a truncated
design.

| Resource | Constant | Value | On exceeding |
|---|---|---|---|
| Declared width of a single net | `MAX_NET_WIDTH` | 1,048,576 bits | `declared net width {w} exceeds the v1 cap (1048576)` |
| Total nets and variables | `MAX_TOTAL_NETS` | 131,072 | `total net/variable count exceeds the v1 cap …; the design is too large or a generate loop is pathological` |
| Unpacked-array elements | `MAX_ARRAY_LEN` | 16,777,216 | `` unpacked array `m` has {n} elements (cap 16777216) ``; port and string-array twins exist |
| Runtime dynamic-storage elements | `MAX_DYN_ELEMS` | 16,777,216 | every clamp or drop warns `W-RUN-DYN-DEGRADE` / `VITA-W4020`, once per net |
| Live class objects | `SimOpts::max_class_objs` | 1,000,000 (about 160 MiB) | `F-RUN-CLASS-LIMIT` / `VITA-F4024`. The class heap is not garbage-collected, so an unbounded `new()` reaches this |
| Integer literal width | `LITERAL_WIDTH_CAP` | 1,048,576 bits | `integer literal width {w} exceeds the v1 cap …` |
| Decimal literal digits | `MAX_DECIMAL_DIGITS` | 315,656 | `malformed integer literal {shown}` |

### 4.5 Depth, unroll and count caps

Parser:

| Cap | Constant | Value |
|---|---|---|
| Statement nesting depth | `MAX_STMT_DEPTH` | 256 |
| Expression nesting depth | `MAX_EXPR_DEPTH` | 128 |
| Total AST nodes | `MAX_AST_NODES` | 2,097,152 |
| Parse errors before the parser stops | `error_limit` | 50 |
| SVA property parse budget | `BUDGET` | 65,536 |
| SVA sequence parse budget | `BUDGET` | 8,192 |

Elaborate:

| Cap | Constant | Value | Message on exceeding |
|---|---|---|---|
| Intra-assignment `repeat(n)` unroll | `REPEAT_UNROLL_CAP` | 1,024 | `an intra-assignment repeat(n) count exceeds the unroll cap …` |
| `generate for` iterations | `GENERATE_UNROLL_CAP` | 4,096 | `generate-for exceeds the unroll cap (possible infinite loop)` |
| `generate` nesting depth | `GENERATE_DEPTH_CAP` | 32 | `generate nesting too deep (deferred)` |
| Emitted elaborate errors | `MAX_ELAB_ERRORS` | 200 | one extra line `too many elaborate errors; further diagnostics suppressed (cap 200)`; the run is still exit 1 |
| Instance-array elements | `INST_ARRAY_CAP` | 4,096 | `` instance array `{name}` has {n} elements (cap 4096) `` |
| Array-copy unroll | `ARRAY_COPY_UNROLL_CAP` | 4,096 | `unpacked-array assignment copies {n} elements (v1 cap 4096)` |
| Assignment-pattern unroll | `ARRAY_PATTERN_UNROLL_CAP` | 4,096 | `an assignment pattern expands to {n} elements (v1 cap 4096)` |
| SVA sequence alternatives and nesting | `SVA_SEQ_ALT_CAP` | 256 | `an SVA sequence expanded to {n} alternatives (cap 256); narrow the bounded ranges` |
| Const-function interpreter depth / steps | `MAX_DEPTH` / `MAX_STEPS` | 64 / 100,000 | the fold declines and the caller stays loud |
| Const range-bound fold depth | `MAX_DEPTH` | 8 | as above |
| Const string fold depth | `STR_DEPTH_MAX` | 64 | as above |
| Wide-constant work budget | `WIDE_CONST_WORK_CAP` | 2^26 word operations — a 65,536-bit `/` or `%` folds, a 131,072-bit one does not | as above |
| Routed string-array elements | `MAX_ROUTED_STR_ELEMS` | 1,048,576 | as above |
| Fixed string-array initializer | `FIXED_STR_ARRAY_INIT_CAP` | 4,096 | as above |
| Block-local coverage-proof bits | `MAX_COVERED_BITS` | 4,096 | the local stays loud (§2.1) |
| Block-local call-inert walk depth | `CALL_INERT_DEPTH` | 8 | as above |

Engine and runtime:

| Cap | Constant or knob | Value | On exceeding |
|---|---|---|---|
| Delta cycles per time step | `SimOpts::max_deltas` | 1,000,000 | `F-RUN-NO-CONVERGE` / `VITA-F4016`: `did not converge: delta limit ({n}) exceeded at time {t} (zero-delay loop / combinational oscillation)` |
| Block steps per activation without suspending | `SimOpts::max_body_steps` | 100,000,000 | `F-RUN-BODY-STEP-LIMIT` / `VITA-F4027`, naming the process and suggesting a larger budget |
| Subroutine call and frame recursion depth | `MAX_CALL_DEPTH` | 8,192 | `F-RUN-FATAL` / `VITA-F4004`: `frame-call recursion exceeded the depth limit (8192)`. Sized so the worst case fits inside the 256 MiB worker stack |
| Arithmetic operand width | `WIDE_ARITH_CAP` | 1,048,576 bits | `W-RUN-WIDE-ARITH` / `VITA-W4025`: the result is poisoned to `X` and the run continues |
| Expression self-width table clamp | `WIDTH_MAX` | 16,777,216 | — |
| Native `**` expansion | `POW_MAX` | 16 | above it the native lane declines and falls back; not a user error |
| `randomize()` rejection-sampling tries | `MAX_TRIES` | 10,000 | the draw fails |
| Out-of-range and unknown-index diagnostics | `CAP` | 8 per kind, independent budgets | one suppression line per kind (§1.1) |
| Filelist (`-f` / `-F`) nesting | `MAX_DEPTH` | 256 | `E-FLIST-DEPTH` / `VITA-E8002` |
| Simulation time | `SimOpts::time_limit` (`--timeout N`) | unbounded by default | reaching it ends the run cleanly as `Quiescent` — a CI killswitch |

Arithmetic is exact for every operand inside the declarable width regime. `WIDE_ARITH_CAP`
equals `MAX_NET_WIDTH`, so a value that can be declared can always be computed: 256-bit
`+`, `*`, `/`, `%` and `>>>` are exact in both signednesses, as constants and at runtime.
Above the cap, `*`, `/`, `%` and `**` poison to `X` (the kernels would otherwise stall);
`+` and `-` stay exact at any width. Only a replication that inflates an operand past the
declared-width regime can reach that boundary.

### 4.6 Build shapes

| Shape | Effect |
|---|---|
| Default build (`cargo build`) | the `oracle` feature is on: `--backend native`, `vm` and `interp` are all accepted |
| `--no-default-features` (the product shape) | only `native` exists; `--backend vm` is rejected rather than silently ignored |
| `jit` feature | off by default; cranelift is pinned at 0.120 |
| `separate-bins` feature | development only. The default build produces the single `vita` multicall binary; `vcmp`, `velab` and `vrun` are required-features binaries |

A bad `--backend` value is `E-CLI-BAD-FLAG` / `VITA-E0001` at exit 3, with the full
accepted-value sentence. When a requested backend cannot run a design, the run falls back
and says so with `W-RUN-BACKEND-FALLBACK` / `VITA-W4030`; the answer is unaffected and
only the speed changes. `--threads` is validated (`'--threads' needs a positive
integer`), a repeated flag is a duplicate error, and the value is clamped to at least 1.

---

## Where to go next

- [Error Codes](007_error-codes.md) — the full message-code reference, and the
  `-Wno-<CODE>` / `-Werror[=<CODE>]` controls that move a code's severity.
- [Language Reference](003_language-reference.md) — the supported subset, construct by
  construct.
- [System Tasks](005_system-tasks.md) — every implemented system task and format
  specifier.
- [CLI Reference](004_cli-reference.md) — flags, backends, filelists and artifact gates.
- [Installation](001_installation.md) — supported platforms and the toolchain floor.
- [`../ROADMAP.md`](../ROADMAP.md) §2 and §3 — the open silent-wrong residue and the
  loud-to-supported queue; [`../REMAINING_WORK.md`](../REMAINING_WORK.md) is the snapshot.
