# 05 · The scope ladder

Language scope is a ladder of four tiers. Each tier names a body of language and the machinery the
implementation must own to support it, and each tier rests on the one below: the constructs at a
higher tier are lowered onto the machinery the lower tiers already provide. This document defines the
tiers and states which one the implementation satisfies. It is not a delivery plan — open work is in
[../ROADMAP.md](../ROADMAP.md), and the order in which the tiers were built is in
[../history/](../history/README.md).

## One front end for two languages

IEEE 1800 (SystemVerilog) is a superset of IEEE 1364 (Verilog). Implementing the SystemVerilog
subset therefore covers Verilog-2005 RTL in full, and a second lexer, parser and elaborator for
Verilog would be duplicated machinery with a second set of defects. There is one front end.

VHDL (IEEE 1076) is a different language and would need its own lexer, parser and elaborator.
Everything from `sim-ir` onward — the frozen IR, the event kernel, the `$` builtins and the waveform
writers — is language-neutral and is shared. That boundary is what makes a second front end an
addition rather than a rewrite, and keeping it clean is a standing constraint on the elaborate output
([04](04-architecture.md), [17](17-sim-ir-ir-backbone-freeze.md)).

## The ladder

| Tier | Language scope | What the implementation must own | State at HEAD |
|---|---|---|---|
| L0 · synthesizable RTL core | `module` and ports, `parameter`/`localparam`, `generate`/`genvar`, nets and variables, packed vectors and arrays, multi-dimensional unpacked arrays, continuous assignment, `initial`/`always`/`always_ff`/`always_comb`/`always_latch`, the `case` family, the loop forms, subroutines, gate primitives and UDPs, `#delay`/`@`/`wait`, the display, time, control and dump task families | an event-driven 4-state kernel with the IEEE 1364 region core (Active, Inactive, NBA, Postponed); an integer time axis with a timescale and precision conversion rule; elaboration that resolves parameters, unrolls `generate`, builds the instance hierarchy and detects multiple drivers; a waveform writer whose bytes are a golden | satisfied |
| L1 · SystemVerilog structure | `interface`/`modport`, `package`/`import`, `typedef`, `enum`, packed `struct` and `union`, unpacked records, `string`, dynamic arrays, queues, associative arrays, `foreach`, `unique`/`priority`, assignment patterns, streaming operators, type parameters, compilation-unit items | a symbol and type layer above nets: interfaces, packages and virtual interfaces resolve by flattening and aliasing rather than by a new net kind; engine-side heap storage for the dynamic kinds, with a degrade-loudly contract on a null or absent handle; out-of-band sidecar tables, so that none of this widens the frozen IR — a construct that adds a frozen field costs a `format_version` bump and invalidates every artifact | satisfied |
| L2 · verification layer | classes with single inheritance and virtual dispatch, parameterized classes, constrained random (`rand`, `constraint`, `dist`, `randomize() with`), concurrent assertions with the sequence and property operators, deferred assertions, functional coverage, `automatic` and recursive subroutines, hierarchical references, `program`, `clocking`, the file-I/O and introspection task families | a call-frame model with its own storage and lifetime rules; scheduling regions beyond the 1364 core — Preponed sampling, Observed and Reactive maturation; a constraint solver with a fixed draw order; and the discipline that every one of these lowers to IR-0, synthesizing ordinary nets and processes plus sidecars and adding no frozen IR node | satisfied, with the per-construct restrictions tabulated in [01](01-goals-and-scope.md) |
| L3 · VHDL | IEEE 1076: `entity`/`architecture`, `process`/`wait`, signal assignment, and `std_logic_1164` / `numeric_std` recognised as builtins | a second lexer, parser and elaborator emitting the same `sim-ir`; the VHDL type system mapped onto the existing net kinds; multi-valued `std_logic` resolution | not present. No VHDL front end exists. The re-entry trigger is recorded in [ROADMAP §7](../ROADMAP.md): a SystemVerilog plateau, a decision that the value domain is worth a second front end, and a GHDL-based oracle |

The tiers are scope, not sequence. A construct is admitted at the tier whose machinery it needs; an
item that would need L3 machinery is not admissible at L2 by writing it differently.

## What holds at every tier

These are not tier-specific and do not graduate:

- Mixed `` `timescale `` modules share one global time axis, and the conversion is exact at the
  declared precision ([08](08-timescale-and-timing.md)).
- Waveform output is driven by the RTL's dump tasks, in VCD or FST, and is regression-gated by a
  golden diff ([07](07-vcd-format.md)).
- Every supported construct is verified differentially against a live oracle, and pinned by hand
  against the LRM clause where no tool implements it ([09](09-testing-and-verification.md)).
- A diagnostic carries a source location, a stable code and a message a user can act on
  ([13](13-diagnostics-and-logging.md), [15](15-error-code-reference.md)).
- The system-task compliance corpus keeps at least one case per family.
- The same design produces byte-identical output across platforms and across all three executors.

## Scope-control rules

1. The boundary is the tables in [01](01-goals-and-scope.md). The synthesizability legend in
   [hdl-reference/](hdl-reference/01-synthesizability-legend.md) answers a different question, and
   several constructs a synthesis tool rejects — `initial`, `#delay`, `$display`, `$finish` — are
   mandatory here.
2. A construct enters the supported set with its verification path attached. With neither a live
   oracle nor an LRM-derived hand pin, it stays loud. The absence of an oracle is not a reason to
   defer the construct itself: a tool that refuses a legal construct is evidence about that tool.
3. When two oracles disagree, the IEEE LRM decides. Where the LRM leaves an answer
   implementation-defined — the `$urandom` stream, `$readmem` address handling, the ordering of
   `initial` blocks across instances — vitamin fixes its own answer and documents it as a pin rather
   than presenting it as an oracle.
4. A promotion from loud to supported is additive. It may not make a working construct loud, and it
   is re-reviewed under both review lenses when its design changes.
5. Feature pressure is answered by the ladder, not by the queue. Scope grows a tier at a time, and
   the tier states the machinery the growth requires.

## Extensions that were built, measured and rejected

Each of these is closed with a measurement rather than an opinion, and each carries a re-entry
condition, so re-proposing one means producing a number that beats the recorded one.

| Extension | Verdict | Where the measurement lives |
|---|---|---|
| In-process machine-code generation (cranelift) | rejected: wired into the arena backend it is slower than the interpreted op stream, because about 38% of a run is the shim across the code-generation boundary while the opcode dispatch it could remove is 9 to 11%. It survives as the `jit` cargo feature, off by default and additionally gated by an environment variable | [18](18-acceleration-analysis.md), [ROADMAP §5](../ROADMAP.md) |
| A separate 2-state simulation mode | rejected: the measured cost of 4-state evaluation is the per-value metadata, not the extra states, and the overwhelming majority of evaluated values are already definite and within one machine word | [ROADMAP §5](../ROADMAP.md) |
| A cycle-based execution mode | rejected, with the feasibility argument and its verdict written out | [20](20-cycle-mode-feasibility.md) |
| Levelized combinational evaluation | rejected on measurement | [ROADMAP §5](../ROADMAP.md) |

Performance work that was kept — the flat arena, width-specialised expression evaluation, and the
per-body compiled op stream — is described in [21](21-tier3-native-backend.md) and measured in
[study/01](../study/01-interpreted-vs-compiled.md).

## Where the open work is

| Question | Document |
|---|---|
| what is wrong or missing, by section | [../ROADMAP.md](../ROADMAP.md) — §2 silent-wrong, §3 loud to correct-support, §4 assertion residues, §5 performance and hardening, §6 the observability rail, §7 conditional items, §8 permanent non-goals |
| one screen of status and the next few items | [../REMAINING_WORK.md](../REMAINING_WORK.md) |
| what a user can and cannot rely on today | [../manual/003_language-reference.md](../manual/003_language-reference.md) and [../manual/006_limitations.md](../manual/006_limitations.md) |
| the order in which the tiers were built | [../history/](../history/README.md) |
