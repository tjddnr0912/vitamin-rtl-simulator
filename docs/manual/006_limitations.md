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
| Out-of-range array index | Read → `X`; write dropped | Loud (`E-RUN-RANGE` / `VITA-E4002`, rate-limited) |
| Arithmetic lane = 128-bit unsigned / 64-bit signed | Wider arithmetic poisons to `X` | Fail-safe (`X`, never a wrong number) |
| `$dumpvars(depth, scope)` args ignored | Always a full dump (a correct superset) | Silent |
| `automatic` block-local that may be read before written | Rejected, not given a leftover value | Loud (`E3009`, with a `note:` at the construct that stopped the analysis) |
| Hierarchical reference to an `automatic` block-local (`tb.a`) | Rejected (IEEE 1800 §23.9 forbids it) | Loud (`E3009`) |
| Same dynamic-array-formal function twice in one expression | Rejected, not given the wrong snapshot | Loud (`E3009`) |

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

**Loud or silent.** **Loud.** Out-of-range access emits the `E-RUN-RANGE`
diagnostic (code `VITA-E4002`). The diagnostic is **rate-limited**, so a hot loop
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
- A write that reaches the variable only through the right-hand side of a
  short-circuit `&&`/`||`, or through a `?:` branch (`while (n < 2 && f(r)))`),
  is not counted — the call may not be evaluated at all, and vitamin does not
  propagate values to decide whether it is.
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
taken immediately before the calling expression runs. That works for a
blocking-assign right-hand side, a `return` value, and any unconditionally
evaluated operand of one (a concatenation, arithmetic, a comparison, a
system-task argument).

There is one snapshot slot per formal, so two forms are rejected loudly rather
than given the wrong array: calling the **same** function twice in a single
expression inside another function/task body, and a **recursive** call inside
the function's own body. Assigning the call to a variable first
(`x = f(arr);`) resolves both. A conditionally evaluated operand (a `?:` arm,
the right side of `&&`/`||`) is rejected for the same reason.

---

## Real (`real` / `realtime`) parameters

`parameter real` and `localparam real` are supported: they bind, participate in
real arithmetic (`R/2` divides in the real domain), and can be overridden with any
value that folds to an integer.

Where the language requires an **integral constant** — a select index, a range or
width bound, a replication count, an array size, a memory address — a real is
accepted only when its value is **exactly an integer** (`parameter real R = 4;`).
A fractional value, or an expression over a real parameter, is rejected with
[`VITA-E3009`](007_error-codes.md); convert it explicitly with `int'()` or
`$rtoi()`. This is deliberate: rounding silently, or reading the f64 bit pattern
as an integer, would produce a wrong answer with no diagnostic.

Not yet supported, and rejected loudly: a `parameter real` declared in an
interface body or a generate scope, a hierarchical reference to another instance's
real parameter, and `$clog2()` of a real **when the result feeds a width or a
replication count** (`$clog2(R)` on its own evaluates normally).

> **Known defect — a package-scope `parameter real` is silently wrong.**
> `pk::PR / 2` with `parameter real PR = 3;` in a package yields `1.0`, not `1.5`:
> the package binding is not routed to the real side table, so the division happens
> in the integer domain with no diagnostic. A module-scope `parameter real` is
> correct. Until this is fixed, declare real constants at module scope, or pass them
> as parameters. Tracked in `docs/ROADMAP.md` §2.

---

## Where to go next

- [Error Codes](007_error-codes.md) — the full message-code reference, including
  `W3011` and `E-RUN-RANGE` (`VITA-E4002`).
- [Installation](001_installation.md) — supported platforms (Linux / macOS /
  Windows, 3-OS CI).
- `docs/REMAINING_WORK.md` (in the repository) — the live tracker of Phase-2
  refinements for each item above.
