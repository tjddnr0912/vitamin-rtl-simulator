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
| `casez`/`casex` treat scrutinee `x`/`z` as don't-care | Over-lenient vs. IEEE; more `case` arms can match | Silent (loud `W3011` only on an explicit-`x` `casez` *label*) |
| Out-of-range array index | Read → `X`; write dropped | Loud (`E-RUN-RANGE` / `VITA-E4002`, rate-limited) |
| Arithmetic lane = 128-bit unsigned / 64-bit signed | Wider arithmetic poisons to `X` | Fail-safe (`X`, never a wrong number) |
| `$dumpvars(depth, scope)` args ignored | Always a full dump (a correct superset) | Silent |
| VCD dump of a multi-word memory/array | Only word 0 appears in the VCD | Silent |
| `%t` field width / `$timeformat` scaling | Value correct; column alignment differs | Silent |
| `disable` statement | No-op; control flow not aborted | Loud (elaborate-time warning) |

The sections below give the detail behind each row.

---

## `casez` / `casex` wildcard matching is over-lenient

**What it is.** vitamin lowers both `casez` and `casex` to a single wildcard
match: `reduction_or(scrutinee ^ label) !== 1'b1`. This masks **every** unknown
bit on **both** sides — including `x`/`z` bits in the *scrutinee* (the value being
switched on), not just wildcard bits in the case labels.

**User-visible effect.** Matching is more permissive than IEEE 1364:

- IEEE `casez` treats only `z`/`?` as don't-care; vitamin also lets an `x` bit
  match anything.
- IEEE `casex` treats `x`/`z` as don't-care, which vitamin does correctly — but
  it applies the same masking to scrutinee `x`/`z`, so a partly-unknown scrutinee
  can match an arm that IEEE would skip.

```systemverilog
reg [3:0] x = 4'b1x10;
casex (x)
  4'b1010: r = 1;   // vitamin AND iverilog: matches (casex masks scrutinee x)
  default: r = 0;
endcase
```

**Loud or silent.** Mostly **silent**. The one loud signal is `W3011`
(`W-ELAB-CASEZ-APPROX`), emitted at elaborate time when a `casez` *label* carries
an explicit `x` bit — the case where vitamin's "mask all unknowns" rule is known
to diverge from strict `casez`. The broader scrutinee-side leniency itself is not
individually warned per evaluation.

Use plain `case` (no wildcards) when you need exact IEEE equality semantics.

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

## VCD dump of a multi-word memory/array emits word 0 only

**What it is.** When a `$dump*` family task snapshots a multi-word net — a memory
or array whose element width spans more than one 64-bit word — only **word 0** is
written to the VCD.

**User-visible effect.** Scalars and ordinary vectors dump fully and correctly.
For a multi-word memory, the VCD waveform shows the low word of each element only;
the upper words are absent.

**Loud or silent.** **Silent.** This is consistent with common VCD-tooling
variance for memory arrays. If you need full memory waveforms, dump the elements
of interest as individual vectors, or inspect them via `$display`.

---

## `%t` field width and `$timeformat` scaling

**What it is.** The numeric **value** of `%t` (and time-printing in general) is
correct, but vitamin does not apply IEEE's default `%t` field width
(right-justified to width 20) or `$timeformat`-driven unit scaling.

**User-visible effect.** The printed time *value* matches; only the column
**alignment** differs from `iverilog`'s transcript.

```systemverilog
#12 $display("[%t]", $time);   // vitamin: [12]   iverilog: [                  12]
```

`%d` already gets IEEE default field-width right-justification (`%0d` for minimal
width, `%Nd` for explicit). The remaining gap is specifically `%t` padding and
`$timeformat` scaling.

**Loud or silent.** **Silent.** This only matters when diffing transcript text
byte-for-byte against another simulator. The simulated values are unaffected.

---

## `disable` is a no-op

**What it is.** The `disable` statement parses, but its target scope is not
resolved. It lowers to a placeholder that continues straight-line execution.

**User-visible effect.** `disable` does **not** abort or unwind a named block or
loop. Execution falls through to the next statement as if the `disable` were
absent.

```systemverilog
begin : blk
  ...
  disable blk;   // does NOT exit blk; the following statement still runs
  ...
end
```

**Loud or silent.** **Loud.** elaborate emits a non-fatal warning
(`disable target scope-id unresolved … emitted as no-op`) so the limitation is
visible in your build output. Do not rely on `disable` for control flow in
Phase-1; restructure with explicit conditionals instead.

---

## Other intentional simplifications

A few smaller, fully-documented choices round out the list:

- **`$stop` = batch terminate.** vitamin is a batch simulator with no interactive
  console, so `$stop` ends the run (under a distinct exit class from `$finish`)
  rather than dropping to a resumable prompt.
- **`assign #d` is transport-delay only.** Continuous-assign delays schedule a
  transport-delay write on each change; there is no inertial pulse filtering, so
  narrow glitches that `iverilog` would absorb propagate through. The
  value-on-delayed-edge is correct.
- **Instance arrays and partial unpacked-array slices are loudly rejected.**
  `dff u[3:0](...)` and indexing fewer dimensions than declared (a row slice)
  raise `E3009` rather than mis-elaborating silently — a loud error, never a
  wrong result.

---

## Where to go next

- [Error Codes](007_error-codes.md) — the full message-code reference, including
  `W3011` and `E-RUN-RANGE` (`VITA-E4002`).
- [Installation](001_installation.md) — supported platforms (Linux / macOS).
- `docs/REMAINING_WORK.md` (in the repository) — the live tracker of Phase-2
  refinements for each item above.
