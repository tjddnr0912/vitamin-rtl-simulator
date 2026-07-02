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

## Where to go next

- [Error Codes](007_error-codes.md) — the full message-code reference, including
  `W3011` and `E-RUN-RANGE` (`VITA-E4002`).
- [Installation](001_installation.md) — supported platforms (Linux / macOS /
  Windows, 3-OS CI).
- `docs/REMAINING_WORK.md` (in the repository) — the live tracker of Phase-2
  refinements for each item above.
