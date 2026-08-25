# Known Limitations

vitamin is a Phase-1 MVP. Its happy path — preprocess → lex → parse → elaborate
→ simulate → VCD — is complete and exercised by a full test suite, but a handful
of behaviors are **intentionally simplified**. This chapter documents every one
of them honestly, so you are never surprised by a divergence from `iverilog` or
the IEEE standard.

Two things to know up front:

- **Most of these are fail-safe.** Where vitamin cannot do the precise thing, it
  prefers to produce `X` (unknown), drop a write, or emit a diagnostic — not a
  confidently-wrong number. The few exceptions are called out explicitly below.
- **"Silent" vs. "loud".** A *loud* limitation announces itself (a warning or
  error with a stable message code; see the
  [Error Codes](007_error-codes.md) chapter). A *silent* one produces a
  documented but quiet divergence — correct enough for most RTL, but worth
  knowing. Each item is tagged.

Platform note: vitamin currently runs on **Linux and macOS only**. Windows is out
of scope for Phase-1. See [Installation](001_installation.md).

The forward-looking ("Phase-2") side of each item lives in the project's
`docs/REMAINING_WORK.md` tracker. This chapter describes what ships **today**.

---

## At a glance

| Limitation | User-visible effect | Loud or silent |
|---|---|---|
| Out-of-range array index (KNOWN index) | Read → `X`; write dropped | Loud (`E-RUN-RANGE` / `VITA-E4002`, rate-limited) |
| UNKNOWN (x/z) array index | Read → `X`; write dropped | Warning (`W-RUN-RANGE-UNKNOWN` / `VITA-W4029`, rate-limited) |
| Arithmetic lane = 128-bit unsigned / 64-bit signed | Wider arithmetic poisons to `X` | Fail-safe (`X`, never a wrong number) |
| `$dumpvars(depth, scope)` args ignored | Always a full dump (a correct superset) | Silent |
| `automatic` block-local that may be read before written | Rejected, not given a leftover value | Loud (`E3009`, with a `note:` at the construct that stopped the analysis) |
| Hierarchical reference to an `automatic` block-local (`tb.a`) | Rejected (IEEE 1800 §23.9 forbids it) | Loud (`E3009`) |
| Dynamic-array-formal call in a `&&`/`||` rhs, another call's argument, a select index, a `case` scrutinee, a `repeat` count, a cast or replication | Rejected, not given a stale snapshot | Loud (`E3009`) |
| `$fgets`/`$fscanf`/`$sscanf`/`$fread` inside a framed subroutine body | Rejected, not a silent 0 with an untouched destination | Loud (`F4004`, at the call) |
| A default argument value whose names bind differently at the call site | Rejected (IEEE 1800 §13.5.4 evaluates it in the subroutine's scope) | Loud (`E3009`) |

The sections below give the detail behind each row. (Earlier editions of this
chapter also listed `casez`/`casex` wildcard leniency, `%t`/`$timeformat`,
word-0-only memory VCD, and a no-op `disable` — all four have since been
implemented with full semantics: the precise IEEE `casez`/`casex` split,
IEEE §21.3.2 `%t` + `$timeformat`, per-element full-width memory VCD, and a
real control-flow-aborting `disable`.)

---

## Out-of-range array/vector index

**What it is.** When a word index into an unpacked array (or memory) falls outside
the declared bounds, vitamin does **not** clamp to the nearest element.

**User-visible effect.**

- **Read** of an out-of-range element returns all-`X`.
- **Write** to an out-of-range element is **dropped** — neighboring valid
  elements are left untouched (no corruption).

```systemverilog
reg [7:0] mem [0:3];
i = 9;
mem[i];        // read  → 8'hxx  (not mem[3])
mem[i] = 8'hAA;// write → ignored; mem[0..3] unchanged
```

An `x`/`z` index behaves the same way: read → `X`, write → no-op.

**Loud or silent.** **Loud.** An out-of-range access with a KNOWN index emits the
`E-RUN-RANGE` diagnostic (code `VITA-E4002`, an error). An UNKNOWN (x/z) index emits
`W-RUN-RANGE-UNKNOWN` (`VITA-W4029`, a warning) instead: reading `mem[idx_q]` while
`idx_q` is still X is ordinary RTL during reset, and reporting it as an error set exit 1
on correct designs. Both are **rate-limited** with independent budgets, so a hot loop
that indexes out of range will not flood your transcript — you will see it, but
not thousands of times.

> Caveat — *sub-dimension* over-indexing of a multi-dimensional unpacked array
> (e.g. `g[0][5]` where the inner dimension is only `[0:3]`) is **not** bounds-
> checked per dimension; it aliases within the flattened address space. The
> outer/whole-element path is checked; the inner sub-dimension is a known 1-D-
> inherited gap. Declared-range normalization (non-zero / descending bases such
> as `mem[4:7]` or `mem[3:0]`) *is* applied correctly.

---

## Wide arithmetic poisons to `X` (fail-safe)

**What it is.** vitamin's integer arithmetic lane is **128 bits unsigned** and
**64 bits signed**. Add / subtract / multiply / divide / modulo run on this lane.

**User-visible effect.** Arithmetic that needs more width than the lane provides
does not silently truncate — it produces all-`X`:

- Unsigned operands wider than **128 bits** → result is `X`.
- Signed operands wider than **64 bits** → result is `X` (signed sign-
  reconstruction is gated at 64 bits).

```systemverilog
reg [127:0] a, b, c;
a = 128'h1 << 64;
b = 128'h1 << 64;
c = a + b;     // OK: 128-bit unsigned lane carries past bit 63 → 0x2_0000…
```

Everything else — bitwise ops, reductions, concatenation, part/bit-select, and
shifts — is **full-width** regardless of vector size. Only the *arithmetic* lane
is bounded.

**Loud or silent.** **Fail-safe, not silent-wrong.** An over-wide arithmetic
result is `X`, which propagates visibly through your design rather than handing
you a plausible-but-wrong number. An honest "unknown" beats a quiet truncation.

---

## `$dumpvars(depth, scope)` arguments are ignored

**What it is.** vitamin accepts the `depth` and `scope` arguments to `$dumpvars`
but does not act on them — every variant performs a full dump of all nets.

**User-visible effect.** `$dumpvars(1, tb.dut)` and bare `$dumpvars` produce the
**same** VCD: everything is dumped. This is a **correct superset** of what you
asked for — no signal you wanted is ever missing; you may simply get more than you
selected.

```systemverilog
initial begin
  $dumpfile("dump.vcd");
  $dumpvars(1, tb.dut);  // dumps the whole design, not just depth-1 under tb.dut
end
```

**Loud or silent.** **Silent.** Because the result is a superset, there is no
diagnostic — nothing is lost, so nothing needs flagging.

---

## Other intentional simplifications

A few smaller, fully-documented choices round out the list:

- **`$stop` = batch terminate.** vitamin is a batch simulator with no interactive
  console, so `$stop` ends the run (under a distinct exit class from `$finish`)
  rather than dropping to a resumable prompt.
- **`assign #d` is inertial**, matching Icarus: a pulse narrower than the
  delay is absorbed. Distinct rise/fall/turnoff delays (`#(2,4)`) are
  honored on gates and continuous assigns.
- **Partial unpacked-array slices are loudly rejected.** Indexing fewer
  dimensions than declared (a row slice) raises `E3009` rather than
  mis-elaborating silently. (Instance arrays `dff u[3:0](...)` are
  supported.)

---

## `automatic` variables declared inside a procedural block

vitamin gives a procedural block's locals one flattened variable rather than a
fresh one per block entry, so an `automatic` block-local is accepted when that
flattening is indistinguishable from real per-entry storage — and rejected
loudly (`E3009`) when it is not. Two things follow from that, and both changed
in the 2026-07-29 release.

**What is accepted.** A local that is written before it is read, on every path,
behaves identically either way. The analysis understands `break`/`continue`
(a jump leaves the block, so it never carries an unwritten value to a later
read), `case` and `if`/`else` arms, and calls to subroutines that provably
cannot touch the name. A declaration initializer re-runs on each block entry,
matching IEEE 1800 §6.21 — including for a fixed-size unpacked array
(`automatic int m[4] = '{1,2,3,4};`). An array filled element-by-element with
literal indices is accepted once every declared index has been written.

Since the 2026-07-30 release, a write inside a **loop body** counts too, when
the trip count proves the body runs at least once and nothing in it can jump
past the write:

```systemverilog
automatic byte cur [];
for (int j = 0; j < 3; j++) fill(cur);   // 3 is constant -> the body ran
ok = (cur.size() == 2);                  // so this is not a read-before-write
```

`repeat (2)` / a constant-true `while` / `forever` are answered by the same
rule. The bound must be written with **plain decimal literals**: a zero trip
count, a `break`/`continue`/`return` reachable before the write, a sized
literal such as `4'd3`, and — for now — a `localparam` bound all keep the local
loud. Two of those deserve a word, because they look over-cautious and are not:

- With a `break`, the statement after the loop really can be reached with the
  local unwritten, so the write cannot be assumed.
- A `localparam` bound would have to be folded by name, and the folder used for
  constants resolves parameters only; a variable of the same name in an inner
  scope shadows the parameter for the *lowering* but not for the folder, so the
  two would disagree. Writing the bound as a literal, or assigning the local
  once before the loop, both work today.

A local that is **never written anywhere in the block** is also accepted, for
any type. There is no first write for a read to be before: the flattened
variable is initialized to the type default once and nothing changes it, which
is exactly what fresh per-entry storage supplies at every entry. This is the
idiomatic "deliberately empty" argument —

```systemverilog
byte exp [];              // no digest is expected in this scenario
run("dma-error", msg, exp);   //  -> e.size() == 0, as IEEE 1800 §7.5 says
```

The **first** write may also be timing-controlled — `#1 x = 7;`,
`@(posedge clk) x = 7;`, `x = #1 7;`, `#1 begin x = 7; end`, and
`wait (c) x = 7;` are all blocking writes, so nothing runs before the write
lands. And once the local is definitely written on a path, no later statement
of any form can make the flattening differ, so the rest of the block is
unconstrained (on a *shared* variable, see the time-advance rule below).

Calling a subroutine that **waits** is fine. A clocked driver task —

```systemverilog
task automatic preload (input int addr, input byte m []);
  @(posedge clk);
  for (int i = 0; i < m.size(); i++) @(posedge clk);
endtask
```

— does not touch the caller's locals, and the analysis now says so: locals
declared after a call to it are analysed normally. (Before 2026-07-29 one
timing control anywhere in a callee made every later local in the caller
loud.)

A call that **returns a value** while writing an `output` argument counts as
writing it, wherever the call sits — any position the statement evaluates once:

```systemverilog
go = rsp_next(fd, r);                            // the rhs of an assignment
if (rsp_next(fd, r) == 1) …                      // an operand of `==`
while (n < limit && rsp_next(fd, r) == 1) …      // an operand of `&&`
case (rsp_next(fd, r)) …                         // a case scrutinee
$display("%0d", rsp_next(fd, r));                // a system-task argument
q = {rsp_next(fd, r)} + other(rsp_next(fd, r2)); // a concat part, another call's argument
arr[rsp_next(fd, r)] = 1;                        // an lvalue index
```

A call in a **conditionally** evaluated operand — the right side of `&&`/`||`, or
a `?:` arm — is evaluated only when that operand is reached, and the write
happens only then, however deeply the operand is nested. An `x` condition
evaluates both `?:` arms, so both writes happen, as the LRM requires.

Since the 2026-07-31 release the same positions work inside a **`task` body**, and
the destination no longer has to be a local of that task — an output actual (or the
assignment's own target) may be a module-scope net:

```systemverilog
int gv;
task automatic outer (input int a, output int done);
  inner(a, gv);                 // a bare call statement, writing a module net
  done = 1;
endtask
```

Before that release this exact shape **aborted the simulator** (`frame lvalue net is
routed`, exit 101, with no diagnostic), and adding an unrelated `#5` or an `else`
branch in the same body made it work — so whether a call wrote memory depended on
what sat next to it. A **`function` body** is the one place still not supported: a
function is entered from the expression that calls it, so it has no call statement of
its own to carry the callee's copy-out. Put the call in a `task` body or a module
process, or assign it to a temporary first. The diagnostic says which case you hit.

The copy-out happens while the expression is being evaluated, so `r` holds a
value written this entry by the time anything downstream reads it. A branch also
knows what its condition evaluated to: `a && f(r)` is true only when *both*
operands ran, so the loop body and the `then` branch know `r` is written — the
loop *exit* does not, because a false condition may have short-circuited the call
away. `a || f(r)` is the mirror image and informs the `else` branch.

An **`inout` argument** normally counts as a *read*, because its copy-in reads the
variable at the call. Since the 2026-07-30 release it counts as a write instead
when the callee overwrites the whole formal before ever looking at it — then no
one can observe the copied-in value, so it does not matter what was there:

```systemverilog
function automatic int nxt (input int fd, inout rec_t r);
  r.count = fd; r.h = "x";        // the whole formal, before any read of it
  return (fd < 2);
endfunction
…
automatic rec_t r;                             // no longer needs pre-filling
while (n < 5 && nxt(n, r) == 1) begin … r … end
```

Unpacked-struct formals are answered member by member. The callee decides this,
so it stays loud when the callee writes the formal only on some paths, reads it
first, `return`s before writing it, or hands the filling off to another call.

Writing a **struct member** counts toward the whole variable. A member is a
constant bit range, so `rm.c = 5;` on a single-member struct writes all of
`rm`, and `rm.a = …; rm.b = …;` covers a two-member one. The same rule accepts
a hand-written `x[31:16] = a; x[15:0] = b;`. Partial coverage stays loud.

**What stays loud, and why.**

- Reading an element the block has not written this entry, or filling an array
  through a computed index (`foreach (a[i]) a[i] = …;`), cannot be proven
  complete, so it is rejected rather than given the previous entry's leftover.
- A subroutine whose body can reach the flattened name — by the bare name, or
  through a hierarchical path such as `t.a` — makes the call a read.
- Two blocks where one **encloses** the other and both declare the same name is
  shadowing, which vitamin cannot resolve through the flattening. Two disjoint
  (sibling) blocks reusing a name are fine, at any nesting depth.
- When two blocks **do** share one flattened variable (same name, different
  blocks), anything that lets simulation time advance in either of them is
  rejected: suspending hands the scheduler to the other block, which writes the
  one variable, so a later read here would see a value its own storage never
  held. This includes **calling a subroutine that waits** — the wait does not
  have to be written inline — and it applies whether or not this block has
  already written the variable, because being written does not make the shared
  variable yours. Declaring the locals `automatic` avoids the sharing entirely:
  each block then gets its own storage.
- A write that reaches the variable only through a `?:` branch is not counted —
  exactly one arm runs, so neither arm's write is guaranteed. The same is true
  *after* a loop whose condition was `a && f(r)`: the loop can exit because `a`
  was false, in which case `f` never ran. (Inside the body it *is* counted; see
  above.)
- A read of the variable elsewhere in the same expression is fine when it sits
  to the **right** of the call (`g = f(r) + r;` reads the written value, matching
  the reference simulator). A read to the **left** (`g = r + f(r);`) has to see
  the value from *before* the call, and vitamin arranges that by taking a copy
  first — so that works too. It is rejected only when the copy cannot serve: when
  the read is spelled as a hierarchical path (`g = t.r + f(r);`), when it happens
  inside a called function's body, when the variable is not a plain bit vector,
  or when two calls in the same expression write it (each would need its own
  copy).
- Referring to an `automatic` block-local **hierarchically** (`tb.a`, or `t.a`
  from a task in the same module) is rejected. IEEE 1800 §23.9 forbids it —
  automatic storage has no static address to name — and accepting it would let
  an outside write reach per-entry storage.

Whenever the rejection comes from the analysis stopping rather than from a read
you can see, the error is followed by a `note:` pointing at the construct that
stopped it and saying why. That location is often several statements later than
the declaration, or in another file:

```
t.sv:4:14: error[VITA-E3009]: an `automatic` block-local `x` whose per-entry lifetime …
t.sv:5:24: note[VITA-E3009]: definite-assignment for `x` stopped here: it is read here
           before any write on this path. Everything after this point is treated as if
           `x` were still unwritten
```

Storage lifetime follows the LRM: automatic storage is created per *activation*,
not per block entry, so a local without an initializer keeps its value across
loop iterations of the same activation and is fresh on each new call.

---

## Dynamic-array arguments to functions

A `function f(input byte b []);` receives a snapshot of the caller's array,
taken immediately before the calling expression runs. Because the snapshot is a
marker placed before the enclosing statement, the call has to sit where that
marker can go: a blocking- or nonblocking-assign right-hand side, a `return`
value, or any unconditionally evaluated operand of one (a concatenation,
arithmetic, a comparison, a system-task argument). A `?:` arm works too, but
only when the function is side-effect free — a conditionally evaluated call
cannot be hoisted without performing its effect on the arm that was not taken,
so a function whose body contains, say, a `$display` stays loud there.

These positions are rejected loudly rather than given the wrong array: the
right side of `&&`/`||`, an argument of **another** call, a select or lvalue
**index**, a `case` scrutinee, a `repeat` count, and a cast or replication
operand. There is also one snapshot slot per formal, so calling the **same**
function twice in a single expression inside another function/task body, and a
**recursive** call inside the function's own body, are rejected too. Assigning
the call to a variable first (`x = f(arr);`) resolves all of them.

> Before the 2026-07-30 release, merely *declaring* a value-returning function
> with an `output`/`inout` argument anywhere in the module — even one that is
> never called — rejected these calls in every position but a bare assignment.
> That was a bug, now fixed; no rewrite is needed to work around it.

---

## Real (`real` / `realtime`) parameters

`parameter real` and `localparam real` are supported: they bind, participate in
real arithmetic (`R/2` divides in the real domain), and can be overridden with any
value that folds to an integer.

Where the language requires an **integral constant** — a range or width bound, a
replication count, a select range, the value of an integer-typed parameter — say
which conversion you mean and it works:

```systemverilog
localparam real R = 3.5;
logic [int'(R)-1:0]   v;   // 4 bits — int'() rounds half AWAY from zero (§6.24.1)
logic [$clog2(R)-1:0] w;   // 2 bits — converts first, then takes the log
wire  [7:0] x = {int'(R){1'b1}};
localparam int  N = $rtoi(R);   // 3 — $rtoi TRUNCATES where a cast rounds
localparam int  M = R * 2.0;    // 7 — a declared integral type is a boundary too
```

The whole expression is evaluated in the **real** domain and only the converted
result becomes an integer, so `R/2` is still `1.75` and a `generate if (R/2 > 1)`
still tests `1.75 > 1`. Converting the real at the leaf instead would decide that
branch on the wrong value, silently — which is why an implicit conversion is the
one spelling that stays rejected:

```systemverilog
logic [R-1:0] y;          // VITA-E3009
wire  [7:0] z = {R{1'b1}};  // VITA-E3009
```

That refusal is not a missing feature. The reference tools disagree about these,
in opposite directions: iverilog sizes `[R-1:0]` as 3 bits while Verilator rejects
the design outright, and for `{R{1'b1}}` iverilog rejects while Verilator
replicates. With no agreed answer to match, vita asks you to write the conversion.

Also still rejected loudly: a `parameter real` declared in an interface body, a
hierarchical reference to another instance's real parameter, **overriding** a
parameter with a real value (`#(.R(2.5))`), a real value in an untyped parameter
that is then used as an integer (`localparam M = R/2.0;` — an untyped parameter
takes its *type* from its value, so that one is a real parameter), and `1.0/0.0`.

> **Known limit — an enum whose label names a `parameter` has no working methods.**
> `typedef enum { A = K, B = K+1 }` with a module `parameter K` parses and the VALUES
> are correct, but `.name()`, `.first`, `.next` and `.num()` on that enum are rejected,
> and the message unhelpfully mentions a "hierarchical function call". The cause is that
> enum methods are resolved while parsing, before any instance override is known — and an
> override really does move the labels (`#(.K(9))` shifts them), so folding the label
> early would be silently wrong. Use a `localparam` instead, which cannot be overridden
> and does fold (`localparam L = 5; typedef enum { A = L, B = L+1 }` works, methods
> included), or write the values literally. One narrow exception remains even for
> `localparam`: a *sized* literal value (`localparam L = 8'h5`) does not fold. Tracked in
> `docs/ROADMAP.md` §0.

> **Fixed — a package-scope `parameter real` used to be silently wrong.** Until
> 2026-08-24, `pk::PR / 2` with `parameter real PR = 3;` in a package yielded `1.0`
> rather than `1.5`: the value crossed the package boundary but its DOMAIN did not, so
> the division happened in the integer domain with no diagnostic. String and real
> package parameters now fold and keep their domain, reached through the scope
> operator (`pk::R`, `pk::S`).
>
> ⚠️ **One form is still refused**: a real or string package parameter reached through a
> WILDCARD import (`import pk::*;` then a bare `R`). It is loud, never silent — the
> package fold succeeds and only the import binding is missing. Use `pk::R` explicitly,
> or import the name directly. Tracked in `docs/ROADMAP.md` §3.

---

## File reads inside a subroutine body

`$fgets`, `$fscanf`, `$sscanf`, `$fread`, `$fgetc` and `$ungetc` write their
destination as a *statement-level* effect. vitamin performs that effect in the
process executor; a subroutine that runs on the frame-call path — one declared
`automatic`, or one with an `output`/`inout` formal, or one returning a string —
executes its body through a different evaluator, which cannot perform it.

Rather than return 0 with the destination untouched (which is what happened
before the 2026-07-29 release, silently), such a read is now a **fatal** the
moment that body actually runs:

```
fatal[VITA-F4004]: `$fgets` writes its destination as a statement-level effect that
this synchronous `&self` frame executor cannot perform, so the read would silently
return 0 and leave the destination untouched. Do the read in a module process, or in
a task vita can inline (no output/inout formals, no `automatic` lifetime), and pass
the result in.
```

The idiomatic workaround for a vector-file walker is to read the line in the
process and pass it to the parser:

```systemverilog
initial begin
  fd = $fopen("vectors.rsp", "r");
  rc = $fgets(line, fd);          // read in the process …
  while (rc != 0) begin
    parse_line(line, r);          // … parse in the subroutine
    rc = $fgets(line, fd);
  end
end
```

A task with no `output`/`inout` formals and no `automatic` lifetime is inlined
into the calling process, so a read inside *it* works normally.

---

## Default argument values

A subroutine's default argument value is evaluated in the scope where the
subroutine is **declared** (IEEE 1800 §13.5.4), not at the call site. vitamin
lowers it at the call site, which gives the same answer whenever both scopes see
the same object — a default naming a module net resolves outward to that net from
a module process, from a generate block, and from another subroutine's body
alike.

When they would differ — the caller declares its own variable of that name — the
call is rejected rather than quietly binding to the caller's:

```systemverilog
int g = 5;
task automatic tw (output int x, input int y = g); x = y + 1; endtask

task automatic outer();
  int g;              // shadows the module `g` at this call site only
  g = 90;
  tw(a);              // error: the default's names bind differently here
endtask
```

Pass the argument explicitly (`tw(a, g)`), or make the default a literal or a
`pkg::` constant.

---

## Where to go next

- [Error Codes](007_error-codes.md) — the full message-code reference, including
  `W3011` and `E-RUN-RANGE` (`VITA-E4002`).
- [Installation](001_installation.md) — supported platforms (Linux / macOS /
  Windows, 3-OS CI).
- `docs/REMAINING_WORK.md` (in the repository) — the live tracker of Phase-2
  refinements for each item above.
