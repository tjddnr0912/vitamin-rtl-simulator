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

Both name the array AND the source line that touched it:

```
d.sv:11:5: warning[VITA-W4029] W-RUN-RANGE-UNKNOWN: array word index of `t.tab` \
is unknown (x/z); read X / write ignored [in t] [at time 0]
```

so one table read from several places gives you several distinct reports rather than
N identical lines. One case is coarser on purpose: an out-of-range access inside a
`function`/`task` body is anchored at the **calling statement**, not at the subscript
inside the body. (The compiled backends record such an access and report it at the
caller's statement boundary; anchoring it deeper would make the same design print
different lines under different `--backend` settings.)

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

## Scheduling regions, `clocking` blocks and SVA

IEEE 1800 defines **seventeen** scheduling regions. vitamin models **seven** of
them, and the ones it leaves out are the ones no supported construct can observe:

| Region | State |
|---|---|
| Preponed | **implemented** — clocking inputs are snapshotted at time advance and committed at the clocking edge, which is what makes `cb.sig` sample the slot-entry value (§14.13) |
| Active, Inactive, NBA, Postponed | **implemented** — the IEEE 1364 core, carried in the IR as `RegionTag` |
| Observed, Reactive | **implemented** — where `assert #0` and `assert final` mature (§16.4) |
| the Pre-/Post-/Re- variants | **not implemented** — only `program` blocks and non-`#1step` clocking skews can tell the difference, and both are refused loudly |

Concurrent **SVA is implemented**, not deferred: `assert property`, `cover
property`, sequences with `##N`, `|->` and `|=>` all run and report. What stays
loud is spelled out when you hit it — a ranged/`goto`/unbounded/multi-clock
consequent, and an action block on `cover property`.

`clocking` blocks work for input sampling, `@(cb)`, `#1step` skew, anonymous
blocks and output direction. These are refused loudly rather than approximated:

- a skew other than `#1step` (`#0`, `#N`, `##N`) — these need a sampling region
  vitamin does not model
- `inout` clocking variables
- a multi-event clocking event (`@(posedge a or b)`)
- a clocking item bound to something other than a net
- driving a clocking input from a parent through a hierarchical reference

> ⚠️ Historical note, because searching the repository will find the opposite:
> `docs/DEVLOG.md` and `docs/ROADMAP_ARCHIVE_2026-07-16.md` contain a
> **2026-06-18 gate evaluation that ruled clocking blocks "NO-GO" for lack of a
> Preponed region.** That verdict was overturned a week later by the slice that
> built the region. Those two files are a chronological log and a frozen
> snapshot; this manual and `ROADMAP.md` are the current statement.

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
> are correct, but `.name()`, `.first`, `.next` and `.num()` on that enum are rejected.
> *(The message used to describe a "hierarchical function call" — a different feature —
> and carried no `file:line`. It now names the enum method, the location, and the fix.)*
> The cause is that
> enum methods are resolved while parsing, before any instance override is known — and an
> override really does move the labels (`#(.K(9))` shifts them), so folding the label
> early would be silently wrong. Use a `localparam` instead, which cannot be overridden
> and does fold (`localparam L = 5; typedef enum { A = L, B = L+1 }` works, methods
> included), or write the values literally. One narrow exception remains even for
> `localparam`: a *sized* literal value (`localparam L = 8'h5`) does not fold. Tracked in
> `docs/ROADMAP.md` §0.

> **Known limit — an enum method on a LABEL is rejected (so is it by both reference
> tools).** `pk::LA.name()`, and the bare `LA.name()` after an `import pk::*`, are
> `VITA-E3009`. A label is a named *constant*, and the methods are defined on a
> *variable* of the enum type — declare one and assign the label first:
>
> ```systemverilog
> pk::e_t v = pk::LA;
> $display("%s", v.name());   // LA
> ```
>
> This is not vita being narrow: iverilog 13 aborts on `pk::LA.name()` (an internal
> assertion in `elab_expr.cc`) and Verilator 5.050 reports *"Can't find definition of
> task/function: 'name'"*. With no agreed answer to match, vita stays loud.
> *(Until 2026-08-26 the message described a different construct entirely — a chained
> string method `s.substr(a,b).atoi()` for the `pk::LA.name()` spelling, and an
> "unsupported hierarchical function call" for the bare one. It now names the label, the
> enum type it belongs to, and the spelling that works.)*

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
>
> **Fixed 2026-08-25 — a package `real` now reaches module-scope CONSTANTS too.**
> `localparam real Q = pk::R * 2.0;`, `int'(pk::R * 2.0)`, a `parameter real` port
> default and `generate if (pk::R > 2.0)` used to be `VITA-E3009` while a
> byte-identical MODULE-LOCAL real folded — the predicate that chooses the real domain
> had no `pkg::` arm to see it with.

> **Fixed 2026-08-25 — a package parameter may be wider than 64 bits.**
> `localparam logic [127:0] K = 128'he1…;` inside a `package` was `VITA-E3009` while
> the identical declaration in a module, in a `#()` header, in a one-element array and
> inside a packed struct all worked: the package's scalar path had no wide domain to
> fall into. It travels through wildcard and explicit imports and through `pk::K`, and
> `pk::K[127:64]` selects out of it.

> **Fixed 2026-08-25 — a bit/part select of an ENUM LABEL.**
> `typedef enum logic [31:0] { EA = 32'hAB34 } e_t;` then `logic [EA[7:0]-1:0] v;`
> declared a **one-bit** net at exit 0 where every other tool declares 52, at module
> scope and in a package alike — while the RUNTIME read of the same `EA[7:0]` was
> already right. Every constant consumer was affected (a net width, an unpacked
> dimension, a replication count, a `generate` condition, an instance parameter
> override, a port list, a genvar loop bound), and the width a select feeds into
> `$dumpvars` now follows.
>
> ⚠️ **One boundary remains, and it is deliberate**: a non-zero-LSB enum base
> (`enum logic [39:8] {…}`) is refused by this fold because the tools disagree about
> it — iverilog reads such a label as a plain value of the base's width and answers
> 171 for `EA[15:8]`, verilator honours the declared LSB and answers 52. An ascending
> base (`[0:31]`) is rejected outright by iverilog. Declare the base with a zero LSB
> and every tool agrees.
>
> ⚠️ Also unchanged: a module-scope label is not visible to a body `localparam` at all
> (`localparam int Q = EA;`, with no select, is already an error) — the package
> spelling of the same text works.

> **Fixed 2026-08-25 — a bit/part select of a PACKAGE constant.**
> `logic [pk::W[7:0]-1:0] v;` declared a **one-bit** net at exit 0 where every other
> tool declares 52 — and the same text written as a bare name after `import pk::*` or
> `import pk::W` did the same. Every constant consumer was affected (a net width, an
> unpacked dimension, a replication count, a `localparam` value, a `generate`
> condition, a submodule parameter, a port list), and a `parameter real`-style
> declaration was not required to hit it.
>
> ⚠️ The RUNTIME read of such a select was wrong too, which is easy to miss because it
> is right for the common declaration: with `parameter [39:8] B = 32'hAB34;` in a
> package, `pk::B[15:8]` printed **171** instead of 52 — the raw internal bits, with
> the declared LSB never subtracted — and an ascending `parameter [0:31]` was refused
> outright. A package parameter may now also be written in terms of a select of an
> earlier one (`parameter Q = W[7:0];`).
>
> ⚠️ **One boundary remains**: for a parameter WIDER than 64 bits, only the explicit
> `pk::K[m:l]` spelling normalizes a non-zero declared LSB; the bare-imported spelling
> and a net sized from either spelling still read raw bits. Declare such a parameter
> with a zero LSB (`[127:0]`) and every spelling agrees.
>
> ⚠️ Also unchanged: a select of an ENUM LABEL in a constant context
> (`logic [EA[7:0]-1:0] v;`) is still one bit — at module scope as well as in a
> package. Assign the label to a `localparam` first and select from that.

> **Fixed 2026-08-25 — the constant domain computes in the width you declared.**
> A named parameter is an operand of the wide fold now, so `A ^ B` folds where only
> `A ^ 128'h1` used to; `+`, `-`, `*` and the comparisons have wide arms; `A[127:64]`
> and `A[127]` select; and the reductions (`^A`, `&A`, …) plus `$countones` /
> `$onehot` / `$signed` / `$unsigned` fold at any width. A replication count may be a
> name (`{N{32'd2}}`), and a `-G K=128'h…` override applies instead of being refused.
>
> ⚠️ **Two boundaries remain, and both are deliberate.** An UNTYPED parameter's width
> is inferred from its value, and that inference disagrees with the language
> (`localparam W = 4'hF | 4'h0;` is 4 bits in every other tool and 32 in the
> inference) — so a reduction or a select over one stays loud rather than answer from
> a width nothing declared. And a size cast is a CONTEXT for its operand, so
> `65'(64'd18446744073709551615 + 64'd1)` — where the sum must carry into bit 64 —
> also stays loud.

> **Fixed 2026-08-26 — `/`, `%`, `**`, `<<<`, `$clog2`, `$bits` and `$isunknown`
> fold at any width.**
> These were the operators the wide constant domain had no arm for, while the
> RUNTIME lane computed every one of them correctly — so `localparam logic [127:0] Q
> = A / B;` was `VITA-E3009` for a 128-bit `A`, and so was
> `localparam int AW = $clog2(MAX);`, the standard width idiom over a crypto
> constant. `$clog2` and `$bits` also answer in a declaration bound
> (`logic [$clog2(MAX)-1:0] bus;`) and in a `generate` condition. The kernels are the
> simulator's own (`mw_divmod` / `mw_pow`), not a second implementation, so the
> constant and the runtime cannot disagree.
>
> ⚠️ **Two boundaries, both loud.** `x / 0` and `x % 0` are `x` in IEEE §11.4.3, and a
> parameter cannot hold `x`, so they are refused rather than folded to 0 — the
> message names the divisor. And the super-linear kernels are budgeted at elaborate
> time: a `/` or `%` wider than about 65536 bits is refused instead of running for
> minutes (the runtime lane answers the same shape with `X` above its own
> `WIDE_ARITH_CAP`).

> **Fixed 2026-08-26 — a shift or index bigger than 2³² no longer folds to a smaller
> one.** The constant domain's count/index channel took the low 32 bits of a value and
> used them, silently. `64'hDEAD_BEEF_1234_5678 >> 64'h1_0000_0000` folded to the
> operand UNSHIFTED where both iverilog and verilator give 0, `A[2**32]` folded to
> `A[0]`, and `$bits({(2**32+2){8'hA5}})` built a **two**-element replication — all at
> `errors=0 warnings=0`, and with vita's own runtime lane giving the right answer in
> the same run.
>
> A SHIFT is now correct: §11.4.10 vacates with zeros (with the sign bit for `>>>`), so
> any amount at or above the operand's width gives the same answer and the fold
> saturates instead of truncating. A named amount folds the same way its literal twin
> does.
>
> A COUNT or a SELECT INDEX above 2³² is now **loud**, not correct — there is no
> answer to adopt. iverilog truncates a replication count (with a warning) and
> verilator refuses it; for an out-of-range select iverilog gives `x` and verilator, a
> 2-state tool, gives 0. The diagnostic names the index and its range.
>
> ⚠️ **Still loud, for a reason one level down**: an out-of-range CONSTANT select
> (`localparam logic C = B[9];` over an 8-bit `B`) is `x` per §11.5.1, and a parameter
> below 64 bits is stored as an integer with no unknown plane — which is why
> `localparam logic X = 'x;` is loud too, with no select anywhere.

> **Fixed 2026-08-26 — an override of a parameter wider than 64 bits is applied.**
> On `parameter logic [127:0] K = <128-bit default>`, a NARROW override was silently
> DISCARDED and the declared default used instead: `#(.K(5))`, `#(.K(32'hDEADBEEF))`
> and `#(.K(-1))` all ran the child with the default at exit 0. A WIDE override was
> refused with two diagnostics that contradicted each other (*"not a constant; default
> kept"* about a constant that was not kept).
>
> Both are gone. Every channel applies — `#(.K(v))` named and positional, `defparam`,
> `-G K=…`, a generate-scope instance, and a forward through an intermediate module —
> and the value reaches the declared width with the OVERRIDE's signedness, so `#(.K(-1))`
> and `#(.K('1))` are all ones while `#(.K(64'hFFFF_FFFF_FFFF_FFFF))` is zero-extended,
> matching both oracles. An override that fits an integer keeps the parameter usable as
> a width or a bound (`logic [K-1:0]`), which a wide-literal override used to destroy.
>
> ⚠️ **Two remain.** An override EXPRESSION whose top operator takes its width from the
> context (`#(.K(128'h1 << 100))`) is still loud — folding it at the operand's own width
> would drop the bits the context keeps. And a `defparam` records no signedness, so a
> NEGATIVE `defparam` value still stops its sign at bit 63.

---

## A 2-state cast evaluates its operand more than once

A cast to a **2-state** type — `int'(e)`, `byte'(e)`, `shortint'(e)`, `longint'(e)`,
`bit'(e)` — has to force any `x`/`z` in `e` to `0`, and vita builds that check one bit at
a time. If `e` cannot be shown to be free of `x`/`z` at elaboration, it is evaluated
**once per bit of the cast's target type**.

For a pure expression this costs time and nothing else — the value is identical. It
matters when the operand has a **side effect**:

```systemverilog
int r;
initial r = int'($random);   // vita draws 32 times and keeps the last;
                             // Icarus Verilog draws once
```

Affected operands are the ones vita cannot prove known: a call to a user function, a
seeded `$random`/`$dist_*`, a file read that advances a descriptor. A 4-state cast of the
same width is unaffected (`integer'(e)` evaluates `e` once), as are size casts
(`24'(e)`), signing casts (`signed'(e)`), and any operand vita can prove is already
2-state.

**Workaround** — assign to a temporary first, which is also what makes the intent
explicit:

```systemverilog
int unsigned t;
initial begin
  t = $random;      // drawn exactly once
  r = int'(t);
end
```

Nesting no longer multiplies (`int'(int'(e))` costs the same as `int'(e)`), but a
widening cast over a call still fans out to the wider width.

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

## Naming a generate block from outside

A named generate block is referenced by its bare label, and a conditional block
(`if` / `if…else` / `case`) or a bare `begin : name` is a singleton, so no index
appears:

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

A `for`-generate is different: §27.4 makes its blocks an **array**, so the index
is required and a bare label is rejected — at every trip count, including a loop
that happens to run exactly once.

```systemverilog
generate for (genvar i = 0; i < N; i++) begin : gl
  logic [7:0] x;
end endgenerate
…
u.gl[0].x       // correct
u.gl.x          // VITA-E3010, as in Icarus
```

vitamin also accepts the redundant `u.g[0].x` on a *singleton* block. That
spelling is a vitamin extension — Icarus and Verilator both reject it — so write
the bare label if the design has to build elsewhere.

---

## Where to go next

- [Error Codes](007_error-codes.md) — the full message-code reference, including
  `W3011` and `E-RUN-RANGE` (`VITA-E4002`).
- [Installation](001_installation.md) — supported platforms (Linux / macOS /
  Windows, 3-OS CI).
- `docs/REMAINING_WORK.md` (in the repository) — the live tracker of Phase-2
  refinements for each item above.
