# Lessons — the incidents behind the engineering rules

This file holds the measurements that produced the rules in [../ENGINEERING_RULES.md](../ENGINEERING_RULES.md).
The rulebook states each rule in the present tense with no history attached; this file keeps the case
that produced it — the design that was run, the numbers that came back, what went wrong, and which
rule the incident bought — so a rule can be re-checked against its own evidence before it is trusted,
relaxed or removed. Dates and slice numbers appear here because they are the subject.

Entries are in reverse chronological order. A slice reference of the form `§4.5.N` points at
[ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md); `§5.1-x` and the Phase A–D letters point at
[ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md); `§2` and `§3` row references point at
the open queues in [../ROADMAP.md](../ROADMAP.md).

---

## 2026-09-11

### A named-list twin can be narrower than the fold it guards (§4.5.481)

The row's fix was "one arm, predicate = `sys_fn_is_integer`", and that list is `$clog2 | $bits |
$rtoi`. Its own doc comment justified the exclusion — "the dimension-query family is integer-returning
too but is LOUD in every certified consumer today". It is not: `const_eval_in_scope` folds `$size`,
`$high`, `$low` and their siblings at `const_fn.rs:393`, so `localparam B = $size(x) - 20;` reached the
same unsigned tail and printed `4294967284` against both oracles' `-12`. The predicate's list had been
written from the OTHER twins' accept sets and had never been checked against the fold that actually
runs in this consumer. Rule: when a new predicate guards a FOLD, census the fold's accept set, not the
list some neighbouring predicate happens to use — and check the doc comment that explains the
exclusion, because a stale exclusion reads exactly like a deliberate one. The same measurement found
two of the three twins spelling the rule as a BLANKET `SysCall { .. }` arm, which is latent on four
names (`$unsigned`, `$itor`, `$realtobits`, `$sformatf`) that are only invisible because they are
loud today.

### An OPT-IN admission parameter is how one path closes a silent-wrong while its twin keeps a loud (§4.5.482)

The gatherer that decides which block-local declarators get their own scope is shared by the module
process path and the subroutine path. On the subroutine path an initializer-free sibling pair is
SILENT-WRONG (it reads the other block's leftover); on the module path the exact same pair is LOUD
(the read-before-assign guard). Widening the shared admission predicate globally would have closed the
silent-wrong and, in the same edit, converted a loud into a value on a shape nobody had measured. The
fifth admission reason was therefore added as a PARAMETER (`admit_static_plain`) that only the
subroutine feed passes `true`, and the module-path mirror was not touched at all — which is what let
the module louds come out byte-identical, full diagnostic text and both note wordings included. Rule:
when a shared classifier serves one path that is silent and one that is loud, the new rule is opt-in
per feed; a global widening is two decisions wearing one diff. The same slice kept its own downstream
NESTING filter unwidened for the same reason, and proved it by measuring the nested cell's diagnostic
text PRE and POST rather than arguing it.

### Two uncarried positions can share one carrier NODE, and a per-NAME guard must become per-AXIS (§4.5.483)

The row priced the work as "each container needs its own per-instance slot", one slice per container.
Two of the four containers — a `T'(e)` cast and a packed struct member — turned out to lose the shape
at the same kind of site: each emits a cast NODE with a literal `signed` bool baked in at parse. One
appended `CastTarget` variant carried both, and the frozen `StructMember` type was not touched at all,
because the struct's own carrier is a parser-local layout table. Rule: before pricing per container,
ask what NODE each container's value flows through; containers that share a node share a carrier. The
second half is the trap: the guard that keeps the strict compare was keyed on the type parameter's
NAME, so the moment any use of `T` was uncarried, every axis of `T` stayed strict. Adding the carrier
alone would have moved ZERO cells. A partial carrier needs a per-AXIS guard (here a bitmask of blocked
axes plus a shift on both sides of the compare) before a single cell can move — and the axis that
still has no carrier has to stay blocked, which is what keeps the move loud→value and never
loud→silent-wrong.

### A carrier reached through an ALIAS or a PASS-THROUGH is a second producer (§4.5.479)

The reader census of the per-instance shape carrier `T$s` was 97 sites and complete, and the two
BLOCKING findings were both PRODUCERS: `n #(.T(T))` handed a literal snapshot of the inner module's
DEFAULT shape instead of the outer instance's `T$s`, and `parameter type U = T` / `localparam
type U = T` gave `U$s` a literal too. Both silently bound the default's sign — new silent-wrongs of
exactly the class the slice was closing. Rule: for a per-instance carrier, census the sites that
WRITE the key as carefully as the sites that read it; an alias and a pass-through each write it.

### A routed predicate with an unrouted value is invisible to every test but an x-write (§4.5.479)

`if two_state(shape_kind(k)) { intro_kind.insert(net, k) }` — the condition took the per-instance
kind and the VALUE stored the raw declared one, at two frame-local sites and one inline-task site.
Every downstream consumer of `intro_kind` (`$typename`, the X→0 write coercion, the net's default
init) then answered with the module's default, and only an UNINITIALISED read or an `= 'x` write in
a subroutine local could see it. Rule: when routing a kind through a funnel, grep every site that
STORES the kind, not only the ones that test it — and list the sites individually, because a census
row that groups them under a file name hides the member that is wrong.

### A row's SITE claim is measured by the route the design takes (§4.5.480)

The queue row named `reserve_frame_block_locals`. Every cell of the shape it described routes
`inlined`, i.e. `hoist_inline_task_locals`; the named function was a second, independent instance of
the same coalesce, reachable only with a different probe. `--obs-dir`'s `run.json`
`subroutines[].route` answers this in one run. Rule: before reading the code a row points at, run
the row's own design and read which route it took.

### Rebuild a reverted design from its prerequisite, not from its patch (§4.5.475)

§4.5.467 was reverted after three rounds, each fix adding machinery (a floor, a per-rule floor)
to stop the new admission rule from SUBTRACTING candidacy. Once the prerequisite (§4.5.468, a reason
carrier per span) landed, the same feature needed six edits and no floor: an `all()` over a
homogeneous reason set cannot re-admit anything, so the mechanism the third round found had no site.
Rule: after a prerequisite closes, re-derive the reverted slice from the row and the code as they
are now; do not restore the patch and its compensations.

### A row's root is a claim; run the nearest spelling that works before building (§4.5.477)

Two of three rows in one bundle named the wrong site. The `pkg::` row blamed a single-segment early
return that was dead code for the shape — `#(.P({pk::PW}))` bound correctly one wrapper away, which
proved every layer below the top node was already right, and the drop was a missing arm in a
shared predicate one line above. The `localparam` row blamed the override, and a typed or
un-overridden control was equally wrong. Rule: before pricing a row's machinery, run the wrapped,
typed and un-overridden twins of its cell; the twin that works is the specification.

### "Both oracles" in a row must have been run on both (§4.5.476)

A §2 row said `64'h8000…0000 * 64'd2` "is 0 in both oracles". iverilog prints 2^64 — it sizes a
parameter-bound `*` at the doubled width. The row had been written from verilator alone. Rule: a
row that cites two oracles quotes two outputs.

### A lint error needs two tools per SHAPE, not per rule (§4.5.472)

The rule "no other process may write an `always_ff` variable" is one sentence in the LRM and one
error in xcelium. Verilator implements it as six decisions: whole-variable writes only, `always_latch`
excluded, `input` actuals excluded, `inout` actuals included, task-body writes excluded, initializers
excluded. The first cut, written from the sentence, failed six in-tree fixtures — three of them RTL
that every tool runs (`initial for (...) m[i] = 0; always_ff m[a] <= d;`). Rule: before an error, put
one shape per file through the second tool and write the table into the module doc; a cell the second
tool does not reject is a warning at most, and a cell nobody has run is "unmeasured", not "accepts".

### Register a scoped declaration where the scope is live, not in a structural prescan (§4.5.473)

The census proposed hoisting generate-scoped functions in the module-body prescan under a
label-qualified key. That would have registered both branches of a generate-if and ONE body per
generate-for label, and the body reading its genvar would have had no value to read (the binding
is transient). Registering in the generate `Nets` arm — where `cur_prefix` and the genvar are
bound — gave one routine per elaborated scope for free, and the only extra work was replaying the
genvar around the later frame lowering. Rule: when a declaration's meaning depends on its scope
instance, register it from the walk that instantiates the scope.

### A comment stripper runs in source order (§4.5.471)

Two passes ("block comments over the whole body, then line comments per line") let a `/*` inside a
`//` comment win, and the symptom was three frames away (`no source files given`). One scan with
"first opener wins" is shorter than the two passes it replaced, and an unterminated opener becomes a
located error instead of a silent swallow. Rule: any lexer-shaped strip is one left-to-right scan.

### A census's fix shape is a claim about every channel it did not name (§4.5.470)

The row and the census both said: gate the literal arm, and the existing `.or_else(ovr.bits)` will
supply the override's width. True for every channel that WRITES `ovr.bits`. The operator-top
override (`#(.Q(~8'h5A))`) and `defparam` do not, and both were correct on PRE only because the
default literal happened to be as wide as the override. Gating the arm alone regressed them 8 → 32.
The implementer found it by measuring the must-stay set, not by reading the plan. Rule: when a fix
removes an answer and names one fallback, list every producer that does NOT feed that fallback and
measure each; a cell that is right by width coincidence is a regression waiting for the gate.

### The branch-parity twin sat under a comment that named the hazard (§4.5.469)

The value re-fold's hazard — "the width column reads fixed while the value column is still cut" —
was written at the site in §4.5.463 for the `self_meta` lane, and the `ovr_bits` lane three lines
away had the identical hazard for two more slices. A comment that names a hazard on one branch is
a census obligation on its siblings: grep the twin predicates at the same site before closing the
row. The cut also was not "at bit 32" — it was at the DEFAULT's width; the row's number was the
symptom of one default, which is why the census varied the default (64/8/40) before fixing.

### Two filters, one exemption: fix the first alone and the second eats the survivor (§4.5.468)

Filter A dropped the outer shadow span; exempting it from A put outer+inner into a nesting pair,
and filter B — which drops BOTH members of a pair — then removed the inner scope that was correct
before (`c24` 0 → 42). Every candidacy filter downstream of a widened set has to be re-run on the
widened set, and the control that proves it is the cell that was correct BEFORE the widening. Two
narrower predicates ("sole admission reason", per-span mixed lifetimes) were each refuted by one
measured cell; the per-name uniform predicate survived, and its one residue (a static pair beside a
disjoint `automatic` span) is a §2 row with the measured constraint that a per-span fix must not
mix lifetimes inside a pair.

### Two oracles agreeing on a testbench race (bench/keccak)

Measured: vita finished `bench/keccak` at 520025000 where Icarus Verilog reported 500025000 and
Verilator `500 us`, exactly one clock period per permutation, with lanes and accumulator byte-identical
at every `+N`. The divergence was not in `wait (done)`: a probe showed `done` itself rising one clock
later in vita. The testbench wrote `start = 1'b1` with a blocking assignment in the same time step as
the `posedge clk` that the core samples it on. Icarus and Verilator both ran the testbench's
continuation before the core's `always` block and latched `start` in that edge; vita ran the core
first and latched it one edge later. IEEE 1800 §4.7 permits either order. Rewriting the three
testbench writes (`rst_n`, `start`) as non-blocking made all three tools finish at 520025000, so the
race-free form agrees with vita's order, not the oracles'. Bought the §7 rule on checking a testbench
for same-time-step blocking writes before filing a timing divergence, and the classification: a
two-oracle agreement on a race is a scheduler coincidence, not a majority.

---

## 2026-09-09

### A determinism golden cannot see a rail that reports the wrong number (§4.5.465)

Measured: `run.json`'s subroutine census reported two counts swapped for any design containing a
class. The determinism golden had been green for rounds — it runs the same input twice and compares
bytes, so both runs report the wrong number identically. What separated the two was an asymmetric
upstream mutation: adding a class to a class-free design, instantiating a module twice, adding a
comment line. The suite had also never exercised two producers together, so the count it protected
was untested however many tests read it.

Rule produced: for every emitted table, name the mutation its numbers must be invariant under and pin
that pair; a repeat run is not a gate.

### Four parallel `Vec`s and three producers (§4.5.465)

Measured: four `Vec`s indexed by a `FuncId` equal to `funcs.len()`, pushed by three producers, one of
which pushed three of the four. That producer runs first, so every later id was shifted and read
another entry's data — surfacing as a wrong count, and, when the shift ran off the end, swallowed by
a defensive `else { return }`. A bounds check at the reader would have left the next producer free to
desync the next table, and the very next slice added one. The document that enumerated the table set
fell one table behind on the day it was written.

Rule produced: make the id impossible to mint without every parallel table getting its entry — one
function, a `debug_assert` per table, and a `None` slot for the case that legitimately owns no row.

### A manifest that misdescribed itself, and a test that asserted the sentence (§4.5.465)

Measured: an emitted manifest told its reader to join on `decl_file:decl_line` while the object it
named carried neither column. The test that was supposed to protect that text asserted the sentence,
so it passed.

Rule produced: pin the fact, never the phrasing; and treat a machine-readable rail that misdescribes
itself as a silent-wrong of its own kind, because its whole audience is a reader who cannot check.

### A wall that a sibling channel had already climbed (§4.5.466)

Measured: a queue row carried "prerequisite = the declared-width provenance wall" through four
slices. The provenance had been recorded the whole time, in the same map, and was already being read
at the same site by a sibling channel: `#(.P(W8))` bound 8 while `#(.P(W8 + 1'b0))` bound 32, in one
scope, in one run. The gate's own comment recorded an earlier failed attempt that used the map alone;
the missing term was the agreement test between the two width maps, which is what refuses a stale
entry.

Rule produced: before accepting a wall, find the nearest spelling of the same question that already
works and ask what it calls — and treat a recorded failed attempt as evidence about that attempt, not
about the question.

### One expression, two answers, because width and sign walked different scopes (§4.5.466)

Measured: a width resolver walked the scope chain while the sign resolver beside it resolved through
the current scope only. An outer signed parameter read from inside a generate block folded unsigned
(255) where the identical text at module scope folded −1. Both oracles answered −1.

Rule produced: when a gate seeds an environment for one property, read every property it decides from
that environment.

### A fourth admission rule that subtracted (§4.5.467, reverted)

Measured: an admission classifier gained a fourth rule. Merely gathering a span under it made a name
look shadowed, collapsed a nesting pair, dropped the survivor below a count bar, and withdrew scoping
from a disjoint block with nothing wrong with it. The property had already broken twice before
through different doors and each fix had been written for the shape in front of it. Three findings
came out of the attempt to repair it: (a) the property must be stated ("adding a rule can only add
candidates") and enforced rather than patched per shape; (b) a flag meaning "not the original rule"
protects only the original rule, and four dynamic-storage kinds went value → loud while the other
three rules stayed exposed; (c) a general floor also adds, and the addition removed a diagnostic that
was masking a different, still-broken span — 18 cells went loud → silent-wrong.

The slice was reverted. Its prerequisite is recorded as ROADMAP §5.2 row 1.

Rule produced: when the third fix on one axis makes the next blocker, the axis is wrong — revert,
ship the separable halves, and file the thing every attempt kept uncovering.

## 2026-09-08

### A width fix that read FIXED on `$bits` while the value stayed at the old width (§4.5.463)

Measured: installing an override's own `(width, sign)` moved `#(.P(-64'd1))` from `ffffffff` at 32
bits to `00000000ffffffff` at 64 — the right width over a value the previous width had already
truncated. `$bits` said FIXED on every cell. Adding the value half turned the 8-, 16- and 32-bit
cells green, so it looked finished; a width ladder showed it was still wrong from 33 up, because a
later block re-derives the value through `override_at_declared_width(param_decl_width(p), …)` and
overwrites `chosen_val` with that resize, re-imposing the default's 32 after the meta chain had got
it right.

Rules produced: a width and its value are one answer — truncation commutes with `~ - << & | ^ + *`
and not with `/ % >> >>>`, so pin a `/` or `>>` cell; after changing a width, walk forward to every
site that re-derives the value from a width; and ladder the axis you changed (8/16/32/33/64).

### Two arms of one chain, one declaration, two answers (§4.5.463)

Measured: `bind_one_param` chooses a parameter's meta through several arms. A `signed` keyword with
no range belongs to the declaration (IEEE §12.2.1) and only the range comes from the override, but
the fix was applied to the new arm alone: `parameter signed R = 1` reported −91 through
`#(.R(~8'h5A))` and 165 through `#(.R(8'hA5))`, in one design.

Rule produced: when you add an arm to a selection chain, state which properties of the declaration
every arm must preserve and check the siblings — fixing one arm makes the inconsistency observable,
which is a reason to check the siblings, not a reason to leave them.

### A gate deleted for creating nets was not what guarded the collision (§4.5.464)

Measured: an interface body refused every user block-local, recorded as "ungating it is
loud → silent-wrong". Re-measured, the gate blocked NET CREATION, and a block-local that collides
with an existing member needs no net created — so the write already landed on the member. The shape
was already silent-wrong at exit 0; the design in the pinned test exited non-zero only because of a
co-located `foreach`, and deleting that one line exposed it. In the same slice, installing the
classifier maps was not parity either: the module path also runs a containment gate before it hoists,
and copying only the maps made a nested shadow silent-wrong while the module twin stayed loud.

Rules produced: ask what a gate actually prevents rather than what its comment says; when a pin's
subject is a value, assert the value and strip every other error source from the cell; and list every
call the original's caller makes before the one you are copying.

### An oracle split that was one tool contradicting itself (§4.5.460)

Measured: `+`/`-`/`*` sat in ROADMAP §2 as a documented split — `localparam Q = 8'd200 + 8'd100` is
`12c` at 9 bits on iverilog and `2c` at 8 on verilator — so the axis was never chased and vita kept a
third answer (32 bits) that is neither. Asked directly, iverilog's own `$bits(8'd200 + 8'd100)` is 8,
the same 8 the other two give, and only the parameter binding grows to 9. Three more spellings of the
same contradiction turned up in one design: `32'd100000 * 32'd100000` binds at 64 bits while
`32'd1 << 32'd33` binds at 32; `$bits(1 << 32)` is 32 while `parameter A = 1 << 32` binds at 64;
`32'd1 << 32` folds to 0 at 32 bits while `1 << 32` folds to 4294967296 at 64. One tool, four
self-contradictions, against a second tool that answers Table 11-21 consistently everywhere. The row
had stood for several slices; re-measuring it cost one probe file.

Rules produced: before recording an axis as a split, ask each tool the same question in two positions
(a direct interrogator `$bits(<expr>)` and an indirect one); and re-measure a documented split's
discriminator, because it ages.

### Excluding a sub-case handed the slice the split (§4.5.460)

Measured: the first cut of the width arm answered every operator except `+`/`-`/`*`, to stay off the
recorded split. The full suite caught it — `const_expr_self_consistency` pins that `*` and `<<` must
not disagree about the context width, and with the three excluded, `*` kept the wide value while `<<`
folded at 32: vita reproducing exactly the inconsistency that test exists to forbid.

Rule produced: a partial accept set is a design decision about siblings — enumerate what the excluded
cases share with the included ones, and prefer a suite run to an argument.

### A census axis that was never varied (§4.5.460)

Measured: procedural-delay routing was measured over 6 timescales x 14 literals x 5 lanes — 84 cells,
61 fixed, 0 regressed — and shipped a regression anyway. `#(1ns - 5ns)` never fires in either oracle
and never fired in that lane before, but `delay_ticks_in_scope` returns `u32` and `real_delay_ticks`
clamps a negative to 0, so it began firing immediately. Every literal on the axis was non-negative.
The differential lens found it only because it probed a shape the census did not have.

Rules produced: when you route a value through a narrower type, add the axis that type cannot
represent; and read the sign in the domain before a clamp, because a clamp is a silent value change.

### A guard whose doc named its own expiry (§4.5.460)

Measured: `param_init_kept_loud` existed because "the value-inferred tail records the folded value's
minimal width, never narrower than 32", and its doc ended by naming the condition under which it
should be deleted. The slice was the tail learning exactly that, so the guard became a pure false-loud
over three two-oracle cells. The soundness lens found it; no test could, because a guard that keeps
working looks identical to a guard that is needed. It had five call sites across the generate,
instance, package and declaration binders.

Rules produced: grep a predicate's doc for the condition it named before changing the component that
condition is about; and make the retirement of a multi-call-site predicate its own slice, recorded
with the measurement and the prescribed deletion.

### A self-consistency pin over five uniformly wrong columns (§4.5.461)

Measured: `const_expr_self_consistency` exists to pin "wrapping an expression in a value-preserving
operation must not change it", and it had passed since it was written — because every column was
uniformly wrong. A self-determined `(8'd200 + 8'd100) >> 2` is 11 and the file read 75 in all five
columns. The property as stated is not even true: `+ 0` is a 32-bit sibling and IEEE §11.6.1 makes the
whole expression 32 bits, so 75 IS the answer there; the real invariant needed the qualifier "of its
own width" (`+ 8'd0`), which was the spelling that was missing.

Rules produced: a relation pin has no anchor — pair every one with a test that pins the values against
an oracle and say so in both files; and a "value-preserving wrapper" is a width claim, so name the
width each side computes at.

### Retiring a guard over the wrong lane (§4.5.461)

Measured: two §2 rows were queued independently — "delete `param_init_kept_loud`" and "the parameter
value fold is width-unlimited" — and they are one root. The guard existed because the value lane
folded at unlimited precision and masked once at the end; deleting it alone was measured to turn 8
cells loud → silent-wrong. Truncation commutes with `~ - << & | ^ + *` (which is why the guard's own
three cells would have been fine) and not with `/ % >> >>>`, so the three cells the row cited gave
the same answer under both candidate value rules and were not evidence about the order at all. Four
of the guard's five call sites were the expiring job and the fifth was a live one on another lane.

Rules produced: read a guard's justification as a precondition on another component and check that
component's current behaviour; when two rows share a root their order is a measurement, so run the
cells that separate the orders; and scope a surviving call site explicitly.

### A width fence on the target that did not fence the leaf (§4.5.461)

Measured: `eval_const_assign` computes `ctx = max(self, target).min(64)`, and its call site was
already gated on `w <= 64` — but `w` is the target. A `logic signed [64:0]` leaf under a 64-bit
target passes that gate and the `.min(64)` clamps the context, deleting a sign bit at bit 64:
`N65 >>> 1` is `ffff…ce` in the unlimited lane and in iverilog, and `7fff…ce` through the width-aware
one. `const_fold_children` has no `Concat`/`Replicate` arm, so a guard built on it is walked past by
wrapping the hazard in braces.

Rules produced: fence the operand the clamp will act on, not the destination; a guard must descend
where an answer need not; and a pre-existing branch of the same gate is inside the new fence's blast
radius too.

### Branch parity is the pass, not the loop (§4.5.461)

Measured: `instance.rs` hoists a module body's procedural block-locals in one four-line loop and
`iface_inst.rs` had no such loop, which reads exactly like a missing-parity one-liner. Adding the
loop made a colliding block-local silent-wrong (`OUTER b=7`, `u.b = 7`, both oracles 99), because the
module path first builds five classifier maps from a `&ast::ModuleDecl` and on the interface path
those describe the parent module. The module twin of the identical text is correct in PRE and POST,
which is what attributed the defect to the new call rather than to the flatten model.

Rules produced: before copying a call, list what the original's caller set up first; ship the subset
that provably does not need the prerequisite and prove it; and make a gate that decides "is this body
safe to process" walk the same statement arms as the processor it gates.

### A probe's resolution inverting a verdict (§4.5.458)

Measured: `$time` is the design's time rounded to the module's time unit. A delay suite that probed
with it asserted correct numbers and then explained them backwards — its prose said fractional delays
round at the module's unit and cited `#(25ns)` under `10ns/1ns` as 30 ns. Re-probed with `$realtime`
at 1 fs, that delay is 25 ns in all three tools: the probe was rounding 12.5 units to 13 and the
comment was reading its own rounding as the oracle's answer. The same probe made `#(0.4ns)` and
`#(2.5ps)` look like "both oracles fire at once" when they delay 400 ps and 3 ps.

Rules produced: a passing assertion does not validate the sentence next to it; pick the probe from the
smallest quantity the rule can produce; and fix the prose in place, leaving the observations alone.

### The lane before yours may answer wrong rather than decline (§4.5.458)

Measured: placing a new fold lane last can only add answers where the existing ones return `None`,
which is byte-identical by construction — provided the existing lane declines. It did not. The
integer delay lane folded `#(3ns / 2)` with integer division to 1 ns where both oracles delay 1.5,
and `#(5ns / 2ns)` to 2 where both delay 2.5. Placed last, the new lane would have preserved both
silent-wrongs.

Rule produced: run the shapes your lane handles through the existing one and record what it returns
before choosing "after"; placing first costs a gate you must pay explicitly.

### An accidental immunity sized by the container (§4.5.458)

Measured: a signed index handed over unsigned is only visible where the unsigned reading lands inside
the object. On a `logic [7:0]` that is index widths 2 and 3 (`-4'sd1` reads 15 — already out of
range, already `x`, already "correct"); on a `logic [63:0]` it is widths 2 through 6. The fix moved
42 of 224 cells and left 182 byte-identical.

Rules produced: sweep the container dimension too, and use the band's edges as the proof of the
mechanism.

### The arm that subtracts nothing skips the rule (§4.5.458)

Measured: `norm_offset_for_net` normalizes an index three ways — a non-zero declared LSB subtracts
it, an ascending or negative-bound net mirrors it, and an `lsb == 0` net returns the index verbatim
because "the raw index is already internal". True for the arithmetic, false for the sign. The `[9:2]`
spelling of the select was right all along and the `[7:0]` spelling was not. Its twin has the same
shape: `norm_offset_for_range`'s `lo == 0` arm still returned verbatim, and a 0-LSB parameter select
was measurably wrong for exactly that reason. Branch parity found the twin by reading, not by
probing.

Rule produced: list what the siblings do besides the arithmetic before trusting an early
`return raw`, and check the sibling funnel in the same file every time.

### A guard firing on its own producer (§4.5.458)

Measured: `sym_typedef_bits` opens with "if this name is locally declared, stand down" — the right
rule for a typedef shadowed by a same-named variable. It declined for every dim-carrying type
parameter, because the type-parameter group's own registration inserts the parameter's name into that
very set.

Rule produced: when a stand-down never fires positively, census the set's writers before relaxing the
guard, and fix it by changing the route rather than the guard.

### Widening a default made the override channel's capacity a live invariant (§4.5.457)

Measured: `parameter type T = a_t` on an unpacked-array typedef ships by letting the dims ride the
registered typedef rather than the `T$w`/`T$s` value channel. The parse relaxation is opt-in at the
default's call site, which closes the door the new spelling opens but not the one that was already
open: `m #(.T(logic [15:0]))` always parsed, and after the widening it replaced the width while the
default's `[0:2]` stayed, declaring 48 bits where both oracles answer 16.

Rule produced: after widening what a producer may carry, ask what the consumer channel can carry and
enforce the difference where the two meet; choose a carrier whose new value is impossible for every
design that predates the slice.

### A decline is a change (§4.5.457)

Measured: `const_delay_u64` gained a `Unary Minus` arm so a sized literal negates at its own width.
Returning `None` for the shapes it could not fold (x/z, wider than 64 bits) handed the caller its own
"no delay" default, and `assign #(-128'd1) y = a;` fired immediately where iverilog never fires it
and the pre-slice fold did not either. The differential lens caught it because it ran the neighbours
of the fixed cell, not the fixed cell.

Rule produced: in a new match arm, the non-improvable sub-shapes call the arm the expression would
have taken; and where a fold feeds a region or enable test as well as a value, say which of `None`
and `Some(0)` that test reads.

### A cell no oracle accepts is not a split (§4.5.457)

Measured: a §2 row carried "its `buf`-driven twin is a split, so the separating property is one
constant continuous driver, not is-a-wire". The cited design is illegal — `buf b(k, 1'b1)` on a 2-bit
`k`, which iverilog rejects at compile — so it was no-oracle, and the legal per-bit spelling is a
plain two-oracle defect. The property survived for an unrelated reason already on record (a `buf` is
the IEEE §7.3 `z`→`x` coercion, so it computes).

Rule produced: check that the cell a row's property rests on compiles on both oracles before shaping
a fix around that property.

### A declaration-ordered fixpoint that is quadratic in the other order (§4.5.457)

Measured: a fold that resolves nets by repeated passes over a map in net order settles a source-first
chain in one round and a reader-first chain in one round per link — 0.17 s to 0.56 s on a 3,000-link
reverse chain. Reversing the list after each round makes both orders one round and costs two lines.

Rule produced: alternate the direction, and leave the honest bound (a shuffled chain is still a round
per link) in the comment.

### One ordering key against six per-kind orders (§4.5.456, reverted)

Measured: §2 row 7 read "process rank is not recorded", and the rank was the right fact — but vita
has one ordering key (`Activity.tie`) where iverilog orders each resumption kind differently, and it
says so in one run of one design: `initial` child-first, `always_comb`'s t0 arm parent-first, an
edge-woken `always` parent-first, a `#d` delay resume child-first, a `wait()` resume parent-first, a
fork-arm wake parent-first. No permutation of one key can be all six.

Round 1: keying the whole run broke the edge and `wait` wakes (a wrong value, both oracles against).
Narrowing to the t0 arm broke the delay wheel. Round 2, after re-keying edge and wait: `always_comb`'s
t0 arm and the fork-arm wake were still on the rank, both oracles against, value-visible again.

A green suite, a green corpus and byte-identical VCDs were all green on the version that printed `aa`
where both oracles print `cc`. What found it was a design with two same-time processes writing the
same variable. A site census answered "all converted" twice while a kind was still uncovered, because
two kinds share a site and one kind reaches the queue through a composition. The row also listed four
twins as already correct; measured, one was a three-way split and two others were separate two-oracle
silent-wrongs on a different path.

Rules produced: ask whether the oracle orders your axis by kind before keying it once; read a root
that returns through a different door each round as the stop signal; enumerate the ten resumption
kinds rather than the code sites; and measure a row's "kept correct" twins before using them as a
baseline.

### A control twin refuting an oracle count in both directions (§4.5.456)

Measured: a §3 row said a tf-port formal was one-oracle and `parameter type T = a_t` read "`$bits` 32
on both oracles". Measured, the fixed formal is one-oracle because iverilog refuses the control
(non-typedef) spelling itself, the dynamic `[]` formal is genuinely two-oracle, and the type parameter
reads the correct 24 on both — the row had undersold its own strongest cell. iverilog SIGABRTs on an
unpacked-array function return type (`Assertion failed: (lwid == ivl_signal_width(lsig))`).

Rules produced: run the control spelling on every oracle before writing a count; a tool that crashes
is no oracle, so record the crash text and mark the cell one-oracle; and where both oracles refuse,
vita's own explicit spelling is still the decisive twin.

### A cell both rounding rules answer the same way (§4.5.459)

Measured: two rounding rules were live for a delay expression — round each leaf, or round the finished
sum — and the cell the queue row named (`#(2500ps + 1000ps)`, 4 ns) gives 4 under both. The cells that
separate them were elsewhere and disagreed about which wins: a real leaf keeps its fraction to the end
(`2.5ns + 2.5ns` = 5, not 6) while a sub-precision-unit leaf rounds where it is written
(`1250fs + 1250fs` at `1ns/1ps` = 2 ps, not 3). The row's documented oracle-split discriminator said
the split "only exists where precision == unit"; one cell at `1ns/100ps` refuted it. Eight split cells
went from silent no-delay to iverilog's answer, which is a rung up, not a side-change.

Rules produced: find the cell where the candidate rules differ before adopting one; check a documented
split's stated discriminator the same way; and landing on one oracle where the tool answered neither
is a promotion.

### A stated cause that priced machinery two of four sub-classes did not need (§4.5.459)

Measured: a row said a feature "needs a dim slot on the `T$w`/`T$s` channel". Two of the four override
classes needed no new carrier at all — an override whose dims equal the default's, and one that
changes only the element width. The refusal was one literal `false` argument at one call site.

Rules produced: enumerate the sub-classes a row's fix would serve and ask which the existing channel
already answers; when a desugar's parameter count becomes variable the positional binding is the
hazard; and report a diagnostic against the user's name rather than a synthesized carrier's.

### An oracle answering the same access two ways (§4.5.459)

Measured in one design: iverilog reads `pv[-2'sd1 +: 2]` as `1x` and `pm[1][-2'sd1 +: 2]` — the same
bits, the same index, a packed element instead of a vector — as `10`. verilator has no `x` for an
out-of-range select at all. Neither is the value oracle for that cell, so the target became
self-consistency: vita answers `1x` for all four spellings and matches iverilog on the two where
iverilog matches itself.

Rules produced: put the two spellings in the same design with the same bits; and write the
disqualifying table into the test so the next round does not re-adopt the contradicting cell.

### A "wider blast radius" warning that was a claim (§4.5.459)

Measured: a row warned that a parameter select folds at elaborate time, so the fold's consumers had
to be swept. The fold lane was already honest-loud, including the cell where the unsigned reading is
in range on a 64-bit container — which proves the const lane already sign-extends. The blast radius
was the runtime lowering only, and the slice was smaller than the row priced.

Rule produced: the decisive probe for "does this lane already handle the sign" is the container size
at which the wrong reading stops being out of range.

## 2026-09-07

### Fixing the read moved the write from a wrong object to a wrong bit (§4.5.446)

Measured: five readers shared a `symbols`-only name walk, so a generate-scope `localparam` shadowing
an outer array did not stop it — the select `ROTA[1]` read the outer array while the whole-name read
in the same `$display` was right. Declining the index chains fixed every read and moved the write
from "stores into the outer array's word 1" to "stores into a different bit of the same net", because
the lvalue path fell through to the plain bit-select lane. Both oracles reject the program; both vita
answers are silent at exit 0.

Rule produced: when a read fix routes a name away from an object, walk the write path to its end and
give the write its own refusal at the lvalue funnels, not inside the shared resolver, where
over-reporting is a loud regression. The write guard then closed shapes the read fix never touched.

### A queue row's oracle count, priced (§4.5.447)

Measured: the row said a subroutine formal of an unpacked-array typedef was two-oracle. iverilog 13.0
refuses `input logic [7:0] v [0:3]` — the explicit spelling vita already supports — with
`sorry: Subroutine ports with unpacked dimensions are not yet supported`, so nine of the row's ten
cells are verilator-only. The cost was four edit sites for one two-oracle cell, against one site for
seven on each of its neighbours.

Rule produced: run the control twin through both oracles, and price a loud→correct item in two-oracle
cells per edit site.

### A decline that was right for a clamp (§4.5.448)

Measured: `param_decl_range_opt` refused a negative declared bound and its comment recorded why — a
previous attempt wrote `min(l).max(0)` and an ascending `[-2:3]` recorded `(0, 6, true)`, turning two
correct cells into silent-wrongs. The refusal was load-bearing for the clamp; the value type's `u32`
was the actual constraint. Widening it to `i64` made the record truthful, closed 13 cells and left
every non-negative declaration byte-identical. `norm_offset_for_range` and `const_norm_bit` were
already `i64` and already exercised by a neighbouring path, which is why the runtime lane was right
and only the fold lane was wrong.

Rule produced: when a guard's comment names the lie it prevents rather than a property of the domain,
ask whether the lie is the type's fault, and check what the consumers already do.

### A resolver-first hook inheriting the caller's scope problem (§4.5.443)

Measured: giving a shared folder's select arms the "ask the name resolver first" probe its sibling arm
already had is additive on paper and not on scope. The new arm answers through `lookup_scoped` /
`param_sel_range`, which walk module scope, and the caller that owns an interpreter environment had
been applying its shadow rule in its own `Ident` arm. A function formal shadowing a non-zero-LSB
module parameter folded the module parameter where PRE, both oracles and vita's own runtime call all
answered the formal — correct → silent-wrong, found by the differential lens.

Rule produced: re-apply the caller's scope rule at the hook, keyed on the node's root, and put the
decline before both resolvers.

### Twelve deliberate-loud pins that had expired (§4.5.443)

Measured: twelve `loud(...)` pins recorded "an ascending or non-zero-LSB element in the wide domain
declines on purpose", with the reason in the file header. When the fold learned the declared range all
twelve became values, and each matched verilator exactly — loud → correct, not regressions. A
`#[test]` fn asserting many cells stops at the first, so the second population stayed invisible until
the first was converted.

Rule produced: a failing loud pin is a claim to re-measure against the oracle, never a regression on
sight; the file's prose reason moves with it; and convert them all before re-running.

### A field added to a shared carrier is a census of its readers (§4.5.445)

Measured: `TypeInfo.unpacked` let a typedef's unpacked dims reach the declaration. The two carries
were the easy half; the slice was the other seventeen readers, each of which had no slot for the dims
and would have bound the element type in silence — a numeric cast, `$bits` of the bare type name, an
enum base, a packed-struct member, a subroutine formal, a non-ANSI port, a parameter, a function
return type, a `for`-init counter, three `parameter type` desugar sites. Four of the declines turned
out to be refused by both oracles too.

Rule produced: grep every read of the carrier type and give each site an explicit `!field.is_empty()`
decline with its reason named; the empty default makes each one a literal-false short-circuit, which
is what lets the census be exhaustive instead of selective. Then measure the declines.

### One arm of a match fixed, its sibling unread (§4.5.444)

Measured: `record_stmt_loc`'s `instance:` match got its class-method arm fixed one slice earlier; the
`(None, None)` arm beside it still spelled the raw storage prefix, so a `$error` in a named block said
`[in top]` while the `%m` of the next statement said `top.blk` — vita contradicting itself two lines
apart, in a match a reviewer had just had open.

Rule produced: read every other arm in the same sitting and ask whether the fixed arm's reason applies
there; and when two strings are built from the same state for the same statement, make them call one
function.

### A post-hoc patch pass bounded by containers, not call sites (§2 row P)

Measured: four deferred-hierarchical resolvers patched their sentinel `LvalChunk`s by scanning
`self.stmts`, and `Lvalue` lives in exactly two arenas — the four `Stmt` variants and
`ContAssign.lhs`. Every hierarchical continuous assign carried its sentinel net id into the engine,
where the per-net table is indexed directly: a panic, which is below loud. The census that finds this
is one grep for the type in the frozen IR (five sites, two containers).

Fixing the two chunk-patch scans made the design run, and the third scan of the same family —
`resolve_pending_fill_widths` — still walked `stmts` alone: the design stopped panicking and printed
`001` where both oracles print `fff`. A loud → silent-wrong the fix itself introduced, found only
because the soundness lens re-ran the shape after the first two scans were green.
`grep -n 'for s in &mut self.stmts'` in the module was three hits.

The deferred-write guard refused a `wire` target with "procedural hierarchical write to net `x`
(declare it reg/logic)". That rule is true of procedural writes and false of continuous ones, so the
moment continuous assigns were routed into the resolver the guard became a false-loud with a message
that contradicted the source.

Rules produced: enumerate the containers of the type, not the call sites that build it; count the
passes and check the hit count rather than the symptom; and read every guard's own words when you
widen its caller set.

### A hoist without a scope (§2 row Q, reverted)

Measured: a `localparam` in a procedural block was a parse error, both oracles accept it, and the IR
has no block-scoped constant — so the declaration was hoisted to the enclosing container's item queue
under its bare name. Six cells went correct and five went silently wrong, for the three reasons the
hoist erases: SHADOW (an outer constant of the same name is answered by the block's value after the
block), SIBLING (two blocks declaring the same name collapse to whichever hoisted last), and LEAK (a
read after the block, which both oracles reject as undeclared, answers). Only the outer-net cell was
loud, and it was loud by an unrelated guard.

The sound alternative — mangle the hoisted name and rewrite the reads inside the block's extent — was
priced by asking where a single-segment identifier is constructed: 52 sites in the parser, no funnel.

Rules produced: build all three lifetime probes before writing the hoist; and the absence of a funnel
is the estimate that turns "small and additive" into "prerequisite".

### A parser-side fold has no scope (§3 row ⑤ⓕ)

Measured: teaching `$bits(<type>)` to answer for one more class of typedef turned a block-local and a
subroutine formal of that name from the local's declared width into the type's — a
correct → silent-wrong trade, because the expression path it used to fall through to did see the
local. The stand-down set every declaration site already writes did not contain tf FORMALS, and adding
them closed two pre-existing instances the same fold had shipped one slice earlier.

Rules produced: enumerate the binders that can introduce the same name (module decl, block-local, ANSI
formal, non-ANSI formal, genvar, instance) and probe each; and record a formal after the enclosing
subroutine's scope snapshot so the restore drops it, pinning both the shadowed spelling inside and the
same text outside still folding.

### A per-container omission in the container nobody wrote down (§3 row ⑤ⓕ)

Measured: a package-scoped typedef twin respelled the names in its `range` and its `packed` dims and
left `unpacked` alone — a third container of the same field type. The visible symptom was LOUD, which
reads as a missing capability; the invisible one was a silent-wrong that only appears when the
importer happens to declare the same name.

Rule produced: count a type's containers and pin each one; a sibling spelling that is already correct
is the signature of a container the pass never visited.

### A route census counted where it was not taken (round-39)

Measured: `run.json`'s `subroutines` census was first written at the seams that pick frame-versus-inline
— `inline_function`, `inline_pkg_function`, `inline_task`. That is three of at least nine: a function
with an `output` formal is hoisted into a temp plus a statement call, and four hoists and two
`stmt_main` arms reach the frame emitter without passing through `inline_function`. The symptom was a
row reading `sites: 0` beside two real call sites — a silent-wrong in a log. The fix was structural:
record inside the three emitters, which needed a `frame_keys` table so an emitter can file a row from
a `FuncId` alone.

Rule produced: count a route where the route is taken, so a new caller cannot miss the census.

### A refuting census that varied the wrong axis (round-39)

Measured: a five-cell census run here refuted an outside report's stated cause ("the operand contains
a function call") and named two other boundaries instead. That refutation was wrong. The census had
counted the `$unsigned` column; a function returning `int` seals with `$signed`, so the one cell that
would have confirmed the report was the one cell the census could not see. The 2x2x2 that replaced it
— operand sign x contains-call x destination width, both output columns — showed the call axis fires
on both signednesses at equal width.

Rules produced: a refuting census must vary the claim's axis and hold everything else fixed, and every
output the mechanism can produce must be in the readout; and when an outside diagnosis is wrong, ask
which instrument would have made it right — that missing instrument is usually the more valuable item.

### A profile that cannot see the inline path reports "free" (round-39)

Measured: vita lowers a subroutine two ways and only one leaves a call node, so any seam-based profile
reports 0 calls for every inlined subroutine, and `0` is indistinguishable from cheap.

Rule produced: when a measurement has a blind region, publishing the region's map is worth more than
publishing the measurement.

### The docs written before the review are the ones the review invalidates (round-39 doc sweep)

Measured: the slice above wrote ROADMAP §6's paragraph while the census still lived at the three
route-picking seams; the soundness lens then moved the recording into the three frame emitters, and
the paragraph shipped naming the wrong place and the wrong count. Every gate was green, because no
test reads prose.

Two more from the same sweep. ROADMAP §5.2 and REMAINING_WORK §B both declare §5.2 canonical; the
slice filed its two deferrals into REMAINING_WORK and the loop prose but not into §5.2, so both
mirrors carried rows their own declared source did not have. And `format_version`, pinned in code at
`header.rs::CURRENT_FORMAT_VERSION`, was restated in three spec documents: at a HEAD of 31 they read
29, 29, 22 and 22 — nine, nine and two bumps behind, with every gate green.

Rules produced: re-read the docs written before a review when the review changes the design (the
high-risk sentences name a file, a function or a count); write a queue edit where the queue is
canonical first and mirror afterwards; and say what a constant is for and point at the canonical site
rather than restating it.

### "We do not see it in real designs" is as broad as the design it was measured on (outside report)

Measured: ROADMAP §2 already carried a constant-fold slowdown graded "invisible in real designs" on
the strength of `picorv32 0.030 → 0.030 s`. picorv32's elaboration is 0.4% of its run, so that
measurement could not have shown anything. Re-measured against a preserved PRE binary: `biriscv`
+36%, and a module of 20,000 plain `wire [31:0]` declarations +193%.

Six builds of a throwaway worktree located 70% of the delta in one commit. The synthetic probe made it
cheap — one module of N identical declarations isolates the front end — and a control twin (the same
module with a parameter-expression bound instead of a literal) separated two independent costs the
real design mixed together.

Nine call sites asked a literal for its `width` or its `signed` bit through `parse_int_literal`, which
builds a despaced `String`, a digit `Vec`, a `Vec<Bit>` and two `BitPacked` planes; adding a walk over
those call sites tripled elaboration. The replacement decides only the case it can decide from the
lexeme (a `_`-free 1..=9-digit decimal is 32 bits signed by construction) and falls through to the
same parse for everything else.

Rules produced: name the design and the fraction of its run when grading a cost invisible; bisect a
performance regression the way you bisect a wrong value; and write a fast path's equivalence test
against the function you are skipping.

## 2026-09-06

### A scope recorded relative to a runtime prefix (§4.5.435)

Measured: `%m` inside a task body was rendered as `<executing process's scope>.<task>.<labels>` —
right only while the caller lived in the declaring scope. A generate-block caller, a recursion, and a
task called through another task each printed the call chain. The declaring scope is known at lowering
time and the executing scope only at run time, so a chain stored relative to the latter cannot be
repaired by the renderer.

Rule produced: decide at the producer whether a recorded string is relative or absolute, spell
absoluteness in the string itself, make every renderer honour the marker, and census the renderers.

### Stripping by "not the other shape" (§4.5.429)

Measured: `%m` had to drop the `[0]` vita stores on a singleton generate scope. The first draft
dropped every `label[0]` whose label was not in `gen_loop_labels` — and an instance-array element
`w[0]` is also a `[0]` segment and also not a loop label, so `ch w[1:0]()` printed `top.w` for element
0 and `top.w[1]` for element 1: correct → silent-wrong.

Rule produced: record the producer's keys at the one site that mints them and key on membership; a
complement silently includes every shape you did not enumerate.

### A second renderer of the same format string (§4.5.428)

Measured: the elaboration-task message renderer folded arguments to an `i64` and printed `%h` of
`v as u64`: `$info("A=%h", N)` with `logic signed [7:0] N = -1` printed sixteen `f`s where vita's own
runtime `render_template` and verilator print `ff`. The first fix read the width and sign and masked,
and the differential lens then found the rest of the rule set missing — bare `%d` has a default field
width, `%5d`/`%04h`/`%8s` carry theirs, `%s` of a packed value is its bytes, `%-` justifies.

Rule produced: move the value-free rules to a crate both renderers reach and make both call them; the
twin's test is the runtime spelling of the same `$display` in the same design, byte for byte, before
the oracle is consulted. A dependency running the wrong way is the signal that the rules belong lower.

### A sidecar that reaches the engine through three copies (§4.5.426)

Measured: an out-of-band table (`stmt_scopes`) was added to elaborate's `SimOpts`, the engine's
`SimOpts`, its `SimState` copy and the render arm — and rendered nothing. The CLI builds the engine's
`SimOpts` from elaborate's `Sidecars` field by field (`frontend.rs`), the staged path again from the
`.velab` trailer (`staged.rs`), and a native test fixture a third time (`native/tests.rs`).

Rule produced: after adding a sidecar, grep an existing sibling across `crates/` and mirror every
site, then assert the map is non-empty at the consumer on the grounding design.

### `%m` has four render sites (§4.5.426)

Measured: the dispatch seam is where every backend converges — for statements executed by a process. A
`$sformatf` renders in two kernels and a frame body's display, severity and `$sformatf` render in the
`&self` frame executor with no seam at all.

Rule produced: a per-statement rendering fact needs the `&self` executor to be able to write it
(`RefCell`), plus a census cell per site.

### A struct-typed name's shape decided before its dims (§4.5.425)

Measured: the ANSI port path bound a struct-typed port as a scalar struct at the type token, before
the unpacked dims after the name were parsed. A constant index happened to work through another path,
and a genvar or runtime index fell to the generate-array hierarchical reference parser ("expected a
constant generate-array index").

Rule produced: bind the shape sets once the whole declarator is parsed, at every binder, and census
the index kinds — a constant index is the control that hides the defect.

### A queue's selection criterion is not the row's fix (§4.5.441)

Measured: "no format bump" was a criterion for choosing a §2 row, and the row chosen carried a fix (a
per-process `inst_prefix` sidecar) that needs one, because every per-process sidecar rides the
`.velab` trailer. The engine-side alternative ("strip generate segments") cannot tell a generate label
from an instance name by its string. The bump procedure is ten commits deep in `header.rs` and costs
one hash re-pin.

Three more from the same slice. verilator prints the first instance's path for a class method's `%m`
in every instance (`top.u1.C.show` from `top.u2`), so multi-instance class-scope cells pin iverilog
and single-instance cells stay two-oracle. A copy alias that substitutes the source net hands the
source's declared sign to every consumer that reads sign from storage. A package function's body folds
in the package's scope, so the callee environment must be seeded with the package constants and the
module-scope fallback refused for a bare name inside it.

Rule produced: read the trailer chain before choosing between a sidecar and a derivation; the bump is
the fix's cost, not a reason to build the alternative.

### Census mechanics moved out of the loop file

Measured facts recorded as census rules on this date: a post-patch or re-spell pass has as many sites
as the type has containers, and a sibling spelling that is already right is the signal that one was
missed; a parser-side fold has no scope; diagnostics must not be counted with `VITA_SCW_CHECK` on; a
typedef census must include the `signed` spelling of every cell; one instance per census cell, because
a two-instance cell prints in display order and reads as NEW-SILENT when it is the second instance's
pre-existing value; an ordering defect is fixed by "before its first consumer", never by "earlier"; a
verilator census runs at about 1,500 cells per 30 minutes, so it takes a width subset, one `--prefix`
per executable, and hand-IEEE for the untrusted cells; and the shadow set of a name is every place a
module binds one — ports, import exports, enum labels, instance names, block-local declarations —
where a census over declarations alone misses four of the five.

## 2026-09-05

### A review finding whose fix was already a reverted slice (§4.5.423)

Measured: a lens reported "a typed initializer folds `/ % >>` unbounded and wraps after" with 17
designs; the one-line routing that fixed all 17 (and three KNOWN-WRONG pins) was ROADMAP §2 row 14's
slice, reverted four days earlier after seven BLOCKINGs — and its recorded regression reproduced on
the first row-14 design run (`localparam time NM` in a generate scope over a module-scope
`logic [7:0] NM`: `2c` for `12c`).

Rule produced: before implementing any review finding, grep §2 for the function you are about to
change; if a row says BUILT or REVERTED, run its designs first and file the finding as that row's
upside.

### A width rule is two steps and a bottom-up fold does one (§4.5.423)

Measured: IEEE §11.6.1/§11.8.2 size an expression first and evaluate every node at that width second.
A fold that computes each node at its own width and extends afterwards passes every cell whose
operands already share the final width and fails exactly where a narrow sub-expression sits beside a
wider sibling: `g[-C+16]` with a 4-bit `C` wrapped `-C` at 4 bits before the 32-bit literal widened
the sum — `g[17]` where both oracles read `g[1]`; `[(C+D)%3:0]` was `[0:0]` for `[1:0]`. Two
REGRESSION cells in a 324-cell census that was otherwise green.

Rule produced: write the shape pass and the evaluation pass as two functions; in the census, put a
narrow constant beside a wide literal under a unary minus, a `%` and a `/`.

### One funnel for a range bound (§4.5.423)

Measured: `[C+D:0]` folded at `i64` in the parser table's readers and at 56 elaborate sites in 13
files that each called `const_eval_in_scope(&r.msb)` on their own declaration kind — net, param,
typedef, unpacked dim, formal, function local, inline task, instance array, string array. The first
probe fixed the net range and the param range still said 17.

Rule produced: grep the evaluator call on the field, route every site through one named funnel with
the old call as its fallback, and measure the funnel's new lane for the shapes it must not change.

### A parse-time page hides the pages behind it (§4.5.422)

Measured: the parser stops at 50 diagnostics. Clearing one 30-diagnostic page did not reduce ibex's
count (50 → 50): three pages behind the cap became visible — a scoped-constant dim on a port, a
genvar-indexed member access, DPI export.

Rule produced: after a ladder rung, diff the per-file distribution and the first line of each file's
page, never the total; write the next page's shape into the queue with its `file:line`.

## 2026-09-04

### A named source replacing a literal (§4.5.413)

Measured: the slice let an array parameter's `'{…}` be replaced by a named array. The first cut
resolved the name through the constant-array capture and fell back to the pre-existing runtime
whole-array copy. Two facts made that fallback wrong in opposite directions, and only a design pair
told them apart. A name that resolves LATE is still a constant — a sibling header array declared after
its reader, or a non-0-based source the capture does not cover, is not in the capture map when the
reader is captured but its net is registered as constant by the time the decl-init flush runs;
refusing "unresolved" there would have turned two correct designs loud (PRE `6 7 3 4`, verilator the
same). A name that resolves to a VARIABLE is illegal (verilator: "variable isn't const"), and the
runtime copy the fallback emits reads it before its own decl-init: `x x` at exit 0. The body-localparam
control had been answering `6 7` for that illegal shape all along, and the new header placement moved
the copy ahead of the source's init and turned the leniency into a wrong value.

Rule produced: at the consumer that has the nets, resolve the name to its net and ask
`const_param_nets` — "does it resolve now?" is a property of pass order, "is it a constant?" is a
property of the design.

Two smaller ones from the same review. An AST-field-free twin (`ParamDecl` plus `NetVarDecl`, same
name, same span) is a legitimate way to occupy a slot the frozen AST has no field for, provided the
pairing predicate cannot be satisfied by a user-written collision (two declarations never share a
span) and the collision is measured on PRE. And verilator prints later instances' `initial` output
first, so a census cell with two instances reads as NEW-SILENT until the line sets are compared.

### A name-keyed parser rewrite has three lifetimes (§4.5.412)

Measured: the slice rewrote every select on a multi-dim packed parameter to a flat part-select, keyed
on the name. The declaration lifetime was right on the first build (141/195 cells); the two others
were not. SHADOW: a block-local plain `logic [7:0] P;` unbound the name and `block_body` snapshotted
the scope only for a typedef or struct/enum-typed local, so after `end` the module's `P[1]` was
silently flat bit 1 — the same hole was merely LOUD for a struct variable, which is why nobody had
seen it. EXPORT: the dims captured at `endpackage` still spelled the package's own `W` bare, so
`p::P[i]` in an importer without `W` was E3010 in every scoped and explicit-import cell.

Three of the first twelve tests were wrong by hand (`P[i+1]` with a 2-bit `i` is 32-bit arithmetic, a
bit table, an LFSR permutation) and went green against the census's oracle only after the values were
replaced by verilator's.

A rewrite that routes a new shape into an old evaluator inherits that evaluator's leniencies as
silent-wrongs: the flat `+:` answers a 0-width select in silence, and a flat `[hi:lo]` with a runtime
bound answers 0 in silence — both pre-existing on any parameter and both unreachable from legal source
before.

Two verilator quirks kept as controls: `$signed(elem) < 0` on a `signed` multi-dim PARAMETER answers
0 where the identical variable is 1 on verilator and iverilog; and `import p::*; import q::P;` answers
p's even for a scalar keyword control where iverilog and PRE answer q's. Harness notes: a CU-scope
`typedef` before `module` is a vita parse error, `$bits(u.X)` is loud, and two parallel verilator
builds sharing one `-Mdir` race on `verilated.d`.

Rules produced: write three cells before the census (a same-named block-local plain decl with a read
after the block, a same-named sibling-scope decl, a same-named ANSI and non-ANSI port and tf formal,
and a package export read through `pkg::`); copy the oracle's raw line into the test; and run the
target evaluator on the degenerate inputs your rewrite can produce.

### A keyword named as the blocker, and two blockers behind it (§4.5.411)

Measured: a §3 row was written as "an overridable array `parameter`, ibex_pkg.sv:791, ibex_top
overrides it" and priced as "the override channel for aggregates". The line the row cites is inside a
package, and IEEE §6.20.1 says a `parameter` declared in a package, a generate block, a class body or
at compilation-unit scope shall be treated as a `localparam` — nothing can override it. That half was
one context flag and closed for free; the real remainder is the ANSI-header array parameter with a
whole-array default, which the row never named.

The same slice re-learned two pins: a `shapes_that_stay_loud` case was loud because of the parser's
keyword reject, not the const arm it claimed to pin (with `localparam` the previous binary already
answered 52), and a "kept loud in v1" pin covered the very shape the next slice builds. And the
generate-position cells put a `function` inside the generate block, which vita defers loudly — 14
cells read "still-loud" for a reason unrelated to the parameter, and the keyword control twin (also
loud) could not tell, because it shared the harness.

Rules produced: run the row's own line and ask which LRM context it is in; a loud pin must name the
gate it measures, and the test for that gate is to change the spelling the gate does not key on; and
diff a cell's diagnostic text against its control's before classifying a loud column.

### A parse-time constant table has four gates (§4.5.414)

Measured: widening `const_locals` so a struct member width can fold looked like "record more
parameters" and was four separate decisions, with a review finding behind each one the slice skipped.
(1) Overridability — IEEE §6.20.1 makes a package `parameter` and a body `parameter` behind an ANSI
header localparams, and everything else must stay out, because both oracles lay a header parameter's
width out per instance. (2) The declared type — decline only a provable mismatch (`byte B = 200` is
−56 downstream); declining what you merely cannot prove made three PRE-correct designs loud.
(3) Every declaration of the name drops the entry — ports, variables, params, genvars, tf formals; the
one path that did not (`genvar`) turned a wildcard-imported constant into a silently-folded generate
index, and a header genvar's drop must be restored after the loop (IEEE §27.4). (4) The readers you
did not write — the table already fed the generate-index and enum-label folds, so the "five new
shapes" byte-identity claim was short by one shape.

A layout that names another type by its BARE key dies at `endpackage`: the wildcard-import cell
passed, and the explicit-import, scoped and cross-package cells failed.

Rules produced: list the four gates and census each with a control twin; for a stored key, name its
lifetime.

## 2026-09-03

### "One rule opens the workload" is a measurement (§4.5.410)

Measured: the queue said the ibex blocker was one parse rule (`localparam <typedef> X = …`) and that
it was the assignment pattern. Measured before starting, the pattern already desugared on a variable
and was never reached on a parameter; measured after the rule, four more blockers stand behind it, the
largest (nested packed structs) in no queue line.

Two more from the same slice. Every census cell needs a keyword-spelled control twin: two "new"
silent-wrongs — a package's derived constant folding from the untruncated initializer, and a wildcard
import shadowing a module-local variable — were pre-existing on `localparam logic [3:0] P`, and only
the twin could say so. And a value that moves between two slots of one record (`forced_range` →
`explicit_range`) has readers on both slots: the array-element consumer wanted the vector slot, the
non-vector-dimension reject reads the same slot, and `typedef byte` went loud until the split honoured
both.

The review ran three rounds and returned nine findings, all in the import carry, five of them made by
the previous round's fix. A "twin" of a predicate in another phase must be the same walk, not the same
intent: the parser's "local declaration shadows the wildcard" was applied at the moment a declaration
was parsed, while elaborate's walks the whole module before any import binds, so a declaration
standing before the import was invisible to the parser twin and a design PRE ran went loud. And a new
loud gate is measured on the designs it will refuse: "an import inside a generate block is ignored,
make it loud" was right for the motivating cell and a regression for the redundant import the corpus
style writes, and for a bare `generate` region, which is not a scope at all.

Rules produced: run the real design after the rule and write the ladder; give every census cell a
keyword control twin; census the readers of a slot before moving anything into it; ask over which set
of names a cross-phase twin quantifies and when that set is complete; and enumerate the syntactic
shapes that reach a new loud arm and run PRE on each.

### A leaf with no width of its own (§4.5.409)

Measured: an unsized fill has no self width; the width table answered 32 (the parser's container) and
every self-determined consumer sized it wrong. Answering `Some(0)` is right for the consumers that
`max` it with a sibling or gate on `w > 0` — but one predicate (`ast_selfwidths_all_known`) read
`w >= 1` as "known", so a fill now read as UNKNOWN, the tier-3 count fold declined, and
`{('1)+1{8'hA5}}` fell to the engine's lowering, which replicated twice where both oracles reject a
zero count: a loud became a value.

Rule produced: when a table gains a third answer, grep every consumer for the predicate it uses to
tell the other two apart and decide per site which of "unknown" and "context-sized" it meant; and the
region made only of such leaves has a width of its own that the evaluator, not the table, must supply.

### Fix a stale-read defect at the read, not at the store (§4.5.408)

Measured: two fixes make `v = 8'hA5; cap = c;` read the fresh copy — forward `v`'s words into `c`
inside the write funnel, or let the read of `c` resolve to `v`. Only the second is observationally
confined to the defect. The store-side forward also makes every settle consumer of `c` see the new
value in the same pass, which changed picorv32's oracle-pinned digest, made a DFF chain sample its
fresh input on the same edge, split native from the VM on keccak, and reordered VCD records in three
of four examples.

Rule produced: ask which reader the oracle proves wrong and change what that reader resolves to,
statically, in the one sidecar every backend consults; then prove the rest untouched with the three
cheapest byte-identity oracles the repo has — the examples' VCDs, the corpus digests and the full
suite — each before any review round.

### A newly foldable leaf lands on every consumer (§4.5.407)

Measured: adding a fold arm for the six reductions does not choose its consumers. The same `Some(v)`
reaches a range bound (a 32-bit self-determined position where the `i64` walk is exact), a port, a
replication count, and an untyped parameter whose value-inferred tail sizes at ≥32 and computes a
context-determined top at unlimited width. Three cells that were LOUD (`~(|4'b1010)`, `(|x) << 2`,
`-(|x)`) would have become silent-wrong on that one consumer while 165 became correct on the others.
And a text folded by two evaluators needs the arm in both: the first arm fixed 117 cells and left
every bound holding a parameter SELECT at 1 bit.

Rule produced: list a shared fold's consumers and, for each, whether its context rule is exact for the
new leaf; where it is not, decline at that consumer for the delta only, keyed on the shape the slice
opens and documented as a delta-limiter.

### An arm that answers without descending (§4.5.405)

Measured: a guard that walks the operand for a hazard is only as good as the arms that descend.
`Concat` and `Replicate` answer `Some(false)` from IEEE §5.4.1 without looking at their parts, so
every syntactic guard the slice added was walked past by wrapping the hazard in braces — a
hierarchical call NAME (the args were walked, the callee's path was not), an inline formal bound to a
hierarchical actual, a select whose own `[msb+:w]` gives a width while its base is a placeholder.

Rule produced: enumerate the arms that answer from a rule rather than from their children, and gate
every consumer of the walk (sign AND width) on one predicate.

### A stricter rule regressing by declining what the baseline accepted (§4.5.405)

Measured: "route only what the walk can measure" is simpler and stricter than the guard it would
replace, and it regressed 458 cells — the pre-slice classifier routed operands whose width the new
walk cannot measure, so declining them is a change away from the baseline.

Rule produced: before replacing a guard with a stricter invariant, measure what the baseline did for
the shapes the invariant would newly refuse.

## 2026-09-02

### Call the lowering's decision, not a mirror of its maps (§4.5.405)

Measured: the size-cast classifier re-derived "where does this bare name bind" from the same side maps
the lowering reads, and got the ORDER wrong twice — an inline formal binds before any constant
(`subst_lookup`), and the innermost combined key binds a generate-scope `localparam` before an outer
module net. Both were review BLOCKINGs on designs the suite never sees.

Round 2 added the second half: the shared decision hands back an `ExprId` for an inline formal, and
the IR mirror of that node is only exact when the node was BUILT for the formal. An actual handed over
verbatim — a frame call, a class field (a 32-bit handle), a hierarchical placeholder — carries its
real sign in a sidecar the mirror cannot see, so the decision function must also say which of its
answers are facts and the classifier answers `None` for the rest.

Rule produced: extract the lowering's decision into a side-effect-free function and make the lowering
match on it too; a docstring saying "mirrors X" is a drift waiting to be measured.

### The queue row is not the only line about its defect (§4.5.405)

Measured: row 29 said "no prerequisite". Two bullets below it, the same defect's older entry held the
reason a previous slice built the fix and reverted it.

Rule produced: grep §2 for the SITE and read every line that names it; the older line is usually the
one that was measured.

### A path right by accident is a latent producer defect (§4.5.405)

Measured: `64'(P >> 1)` was right on the fill-only path with a wrongly-signed parameter because the
shift's result was positive; the moment the classifier read the parameter's sign, the cell went wrong.
`P < 0` and `64'(P)` had been wrong the whole time. Fixing the producer
(`param_decl_width_opt` typing an overridden parameter by its DEFAULT) fixed 7,982 cells and then
yielded a new blocker in each of three review rounds, so it was reverted and the consumer was made to
decline on what it cannot vouch for; the producer's patch went into its own row.

Rule produced: when a review's "regression" only reads an input that was already wrong, fix the
producer and then run the corpus; and if the producer axis yields a blocker in each of three rounds,
revert it and decline at the consumer.

## 2026-09-01

### A predicate borrowed from another phase is a mirror (§4.5.398)

Measured: four queue rows, five blocking defects in the fixes, four of the five one shape — a rule
written twice, or a predicate copied from a phase that does not share its inputs. The new elaborate
IEEE §6.19 check tried to skip "what the parser already policed" with a local `is_literal`; the
parser's actual fold takes a decimal literal, unary minus and `+ - *`, and the mirror took any
`IntLit`, parens and any unary, so the two accept sets are neither equal nor nested and `[(3):0]` and
`[8'd7:0]` fell between them and stayed fail-open — the exact hole the row existed to close. The fix
was to delete the mirror, because a parser error halts the pipeline and there was never anything to
skip. The three enum-label binders each say "see `instance.rs`" and one took its auto-increment from
the raw value while the others took it from the masked one: same enum, loud at module scope, silent in
a package. The `$itor` gate asked "is this expression real?" at the AST, where the crate's own comment
four hundred lines away says that predicate is blind to a real-returning frame call; asking the VALUE
in the one evaluator answered it for every spelling at once and removed a sign flip past 2^63 and an
`--obs-procs` census inflation with it.

Rules produced: print both accept sets and name the difference before mirroring a predicate across a
phase boundary; diff the two bodies when writing a comment that says "twin of X" (a review lens read
three such comments and found one false); prefer the funnel that sees the VALUE; and cap arithmetic
that builds a range from a width (`1i128 << w` for a user-parameterised `w` wraps in release and
panics in debug).

### A rule copied from a tool that collapses two objects (§4.5.397)

Measured: a §2 copy-net rule was derived from iverilog, which turns `assign n = m;` into ONE net — `n`
and `m` share a value, a default and an event. vita keeps two nets, each with its own storage default
(a driven `wire` starts `z`, a `logic`/`reg` starts `x`), so the sentence the rule is built on is true
of the VALUE and false of the DEFAULT. Both naive translations shipped and both were caught by
adversarial review, one per round, failing in opposite directions. Mirroring the source's event
INVENTS events — `assign vv = {1'b1,1'bx};` moves while a `logic` copy of bit 0 is `x` before and
after, so the mirror woke a child module on a port that holds `x` for the whole run: 16 of 56
generated cells, correct → silent-wrong. Suppressing on the IMMEDIATE source LOSES events — a copy can
stay put because its own default already equals the copied value: `assign vv = 2'b1z;` moves, the
`wire` copy of bit 0 does not, and the `logic` copy of that one does, and iverilog fires on it. The
property that survives is transitive: suppress only when nothing in the source CHAIN moved.

Rules produced: ask what a simulator you import a rule from MERGES that you keep separate, and build
the twin that differs only in the merged field (here `2'b1z` versus `2'b1x`); the fix for round N is
the finding of round N+1, so a delta round is not optional; and a verify phase that dies wholesale
leaves its findings UNVERIFIED, not cleared — a blocking repro that does not reproduce should be
rebuilt from the stated mechanism, which here was exactly right.

### Doing something twice is only safe if the second time is provably identical (rows 8b and 11)

Measured: binding enum labels once in declaration order and once afterwards looked monotone — the
first pass only adds bindings the second would make anyway, and the unwind is a stack. Two review
lenses each measured that false independently, with a third mechanism only one of them saw. All three
had one shape: a consumer that runs BETWEEN the two passes keeps the first answer while everything
after it keeps the second, so one name has two values in one elaboration at exit 0. Two of the three
were gateable (an unfoldable label value, a base width that is not yet a fact — and the gate has to
decline the whole group, because the labels share an auto-increment counter and a width). The third
was not: the fold succeeded in both passes with a different answer, because a name in it resolved to a
wildcard import in one pass and to a body declaration in the other.

Rule produced: the second pass has to VERIFY, not overwrite — record what the first pass bound and
compare, and make a mismatch a loud diagnostic.

### A funnel that resolves a name answers nothing before the name exists (rows 8b and 11)

Measured: the read-only check is called at every write position, so "every write position calls it"
read as "every write position is checked". A destination inside an `automatic` task, or written
hierarchically, is lowered before its name is resolvable, so the funnel is handed a poison net and
returns false. Adding one keyword to a design flipped the same statement from loud to silent-wrong.
One spelling was already correct there for the wrong reason: `$readmem*` happened to carry a side map
into the deferred-resolution pass for an unrelated purpose and asked the read-only question off the
back of it. The `$sformat` check says in its own comment that it mirrors the `$cast` dest guard above;
`$cast` never called the funnel at all, and neither did the seed argument of `$random` and `$dist_*`,
which the engine advances and which is therefore a write position that looks like a read.

Rules produced: when one member of a family behaves and its siblings do not, find out what the working
one has; and enumerate what the RESOURCE is (every argument the engine writes back), not what the code
does.

### Row 14 — declared width provenance: built, measured, reverted

Measured: three earlier slices stopped at "a parameter's declared width is not available in the
constant domain" and each proposed building it. It was already built, one scope over: a constant
FUNCTION's body folds through a width-aware walk that carries each local's declared width and sign and
converts a leaf into its context (IEEE §11.8.2), and gets the answer right, while the module-scope
initializer folded through a width-unlimited walk with no context to convert into.

The routing was gated on `param_decl_width_declared` and not on `param_meta`, whose width can be
inferred from the initializer's own value — IEEE §6.20.2 gives an untyped parameter the type of its
FINAL override, and review found a live case where the declared default is not a fact about the
parameter once an override arrives.

Rows 14, 16 and 21 were filed as one infrastructure item because all three want a declared width;
closing 14 closed only 14, because the other two need the >64-bit fold to compute at the CONTEXT width
and the ≤64-bit walk does not reach them. They share the provenance and not the domain.

Review round 1 pointed out that a package folded `NS ^ 64'h0` to `ff…fe` where a module folded the
identical text to `00…fe`, and routing the package binder through the same width-aware fold looks like
the one-line answer. It is a net loss: it makes the package's stored VALUE canonical while every
consumer of `pk::X` still folds through the width-unlimited walk. Measured over 8,748 package-consumer
designs: 1,233 correct → silent-wrong against 714 fixed, plus one correct → loud.

A provenance set written insert-or-remove and probed by an outward scope walk cannot distinguish "this
scope bound the name and it is not declared" from "this scope never bound the name", so the walk sails
outward and vouches for an ancestor's declaration. A module-scope `logic [7:0] NM` beside a
generate-block `localparam time NM = 300;` made `NM | 64'h0` answer 44, at exit 0, through five scope
kinds. `param_meta` was the obvious level marker and is wrong, because the very declaration that has
to stop the walk (`time`) is deliberately absent from it; the VALUE map is the one every binding
writes.

Three review rounds, seven BLOCKING. Round 1's three were all one root in the design: the gate demanded
provenance of the TARGET and let the walk size every LEAF out of a map whose width is a default. Round
2's two were one from the fix made for a round-1 nit and one in the gate mechanism. Round 3's two were
both in the SHARED width-aware walk, and both had been there since round 1 — the shift-count defect
reproduces today, through a constant function, with none of the new routing involved. The change was
reverted with its diagnosis, both prerequisites, and a pinned test recording what a future fix must
move and what it must not.

Rules produced: write the same expression in the neighbouring scope before building the mechanism a
wall is attributed to; a width is only a fact if a declaration says so; name the mechanism, not the
missing input, when collapsing N rows; enumerate who reads a stored value before making it more
precise; use three-valued provenance for scope resolution; and read blockers that have moved into the
code you routed TO as the discovery of a prerequisite.

Also from that slice: routing the initializer closed a separately-filed four-operator residue, which
was marked RESOLVED before the review finished and came back with the revert — and on re-opening, the
row said four operators lost the sign where only three do.

### Re-grounding the queue — six rows, six changed shapes

Measured: before picking the next slice, every open candidate was reproduced at HEAD against iverilog
and verilator. All six changed shape, in four different directions.

| row | what the queue said | what it measures |
|---|---|---|
| shift count | a constant-function corner | reachable at module scope with no function and no name — a silently wrong bus width, and a `generate if` taking the wrong branch |
| const out-of-range select | a loud gap, one prerequisite | a 10-cell silent-wrong sits under it with a one-conditional fix; the recorded headline is the part not to start |
| `**` at a wide target | loud | 48/108 silent-wrong; the `**` examples are the rare loud corner |
| duplicate name | 3 spellings, §2 | 126 cells, and it belongs in §3 (nothing correct is got wrong) |
| clocking output | "a write lands as x" | declaring the output destroys the signal with no write at all, across a module boundary |
| verilog-ethernet | one ~10-line gate | the gate passes elaborate in 1.03 s and then simulates in 38 hours against 7.6 s |

The verilog-ethernet gate really is ten lines and really does make the design elaborate. It would
still not run — 18,000x iverilog, with `--obs-procs-time` putting 100.0% of it in eighty continuous
assigns that re-evaluate a constant-argument function call on every scheduler pass.

Rules produced: a queue line ages against the binary that wrote it, and a row's CLASS ages fastest, so
re-measure class first; ask what the mechanism can reach rather than what the reporter ran; and
measure the end-to-end outcome before promising that closing a gate promotes a corpus row.

### Row 27 — the shift count

Measured: reading a shift count as SIGNED was wrong, and it was also the only thing keeping a second
defect invisible. A count of −3 is out of `0..64` at any width, so the shift collapsed to 0 — which is
the correct answer for the 32-bit count the language gives that parameter. Making the count unsigned
made the stored width matter for the first time, and the stored width is an untyped parameter's
DEFAULT, not the type IEEE §6.20.2 gives it once an override arrives: 21 cells went
correct → silent-wrong.

The obvious fix was to admit any name whose width has declared provenance, and `param_range` is
documented as exactly that map. It has an entry for the offending overridden parameter — measured, in
the design that mattered, in under a minute. The census went back to 30 fixed and the blocking design
went straight back to wrong. Two maps looked like they answered "is this width a fact" and neither
did.

The x/z override guard needs to know whether a parameter's declared type is 2-state, since
`bit`/`byte`/`int` convert x and z to 0. The parser computes `var_kind` for exactly those keywords and
drops it, because `ParamDecl` has no such field — and the file's own comment documents the same gap
for the 1-bit `logic` range it had to work around earlier. A five-cell fix turned 76 correct cells
loud, so the prerequisite is an AST field, not a conditional.

Rules produced: before removing a conversion, ask what it was hiding — the signal is values right for
the wrong reason; a map's docstring describes its intent and only a run describes its contents; and
the parser computing a fact and throwing it away is a prerequisite, not a patch.

### Row 21 — a context width, and a narrowing gate that was load-bearing

Measured: the wide fold declined a context-determined top it could not widen, and the caller then fell
back to the width-unlimited integer walk, which happens to be right for `+`, `*`, `<<` and `?:`.
Threading a context width made the decline stop firing, and a stale `Paren` arm two lines away began
answering instead, at the operand's width — one pair of parentheses turned a correct value into a
wrong one, and the slice's own headline cell got worse through a paren.

`widen_to` extended each operand in its own signedness. IEEE §11.8.2 decides the expression's sign
first and coerces operands to it, so a signed operand in an unsigned expression is zero-extended — and
vita's runtime evaluator already did exactly that. The constant domain was contradicting the runtime,
in the same binary, on the same text. Both review lenses found it, and neither needed iverilog.

The docstring said the arms consult the canonical context-determined list; they hand-match, so the
file is a sixth reading of it, and the drift was already real (`BitXnor` was in the list and missing
from the arm, so `~^` was wrong before and after). Review caught the sentence, not the operator.

Four unrelated tests refused a value and each named, in its docstring, precisely which missing
capability made the refusal honest; when that capability landed they failed loudly and re-aiming them
was mechanical, because the expected values were already in the comments, measured on both oracles.

The scheduler's "may this continuous assign be skipped?" predicate refused every call, with a comment
giving the reason: "a user function can read state no net records." True, and the wrong criterion — so
`assign y = f(...)` was re-evaluated on every settle pass of every delta for the life of the run, and
one third-party design spent 99.99% of its runtime there. iverilog and verilator both re-evaluate a
continuous assign exactly when a net in its sensitivity list moves, so the obligation is "the
dependency set names everything that can change the value". A function carrying a STATIC local between
calls still works, because the oracles carry the same state under the same trigger rule; a `$display`
in the body prints once instead of thirty times, because an effect's right occurrence count IS the
oracle's evaluation count. What genuinely breaks it is a read the set cannot name: `$random`'s seed,
`$fgetc`'s file position, `$time`.

The measurement behind that certification carried a local whose value was IDEMPOTENT, so re-evaluating
with the same inputs wrote the same thing. Review built the non-idempotent twin, a counter, and the
three tools did not agree at all: the ungated change answered `2 3 3` where the previous release
answered `3 3 3` (verilator's answer exactly) and iverilog said `1 2 3`.

The fix for that is definite assignment, which is correct and reverted the slice's entire headline —
the motivating library function clears its arrays in a `for` loop, and a loop that might run zero
times is not definite assignment. Every attempt to weaken the analysis either admitted the counter
again or still refused the loop. The answer was a second, independent reason to be safe: an assign
with an EMPTY dependency set is evaluated once, which is what both simulators do with an empty
sensitivity list, and it was measured to give iverilog's answer exactly on the counter function the
gate exists to refuse.

ROADMAP §3 had one queue line — a `$finish` in a function body refuses the design — and closing it
made the design elaborate in 1.03 s and simulate for 38 hours. The performance defect underneath was
in a different file, had no queue row, and was reached only because the refusal was lifted first on a
scratch copy; then the two collided, because the performance certification had to admit a system task
for the motivating function to qualify and the `$finish` promotion had to be admitted into that same
certification.

The `$finish` promotion needed an arm in the task executor for symmetry. Instrumenting it across the
whole suite and every spelling showed it never fires; the arm was kept, because the alternative at
that point in the match is a `_ => {}` that DROPS the statement, and its docstring says it was
measured dead, how, and what the honest behaviour would be.

The new fatal said "vita ends the run here rather than choose what the calling expression receives".
Measured, the body keeps executing — the latch is read at the enclosing statement, so a `$display`
after the `$finish` still prints. The sentence described a bail the executor does not have, and it was
repeated in two docstrings and a comment because they were written from the same intent.

The round-1 fix rested on one sentence: an assign with no dependencies is evaluated once, at the
settle seed, and never again. Round 2 measured it false: `k_release` calls `redirty_drivers_of` on its
target unconditionally, on purpose, so the assign is evaluated `1 + releases` times and round 1's
whole blocking table came back through the other arm of the disjunct. The census that should have
preceded the sentence is one grep: `ca_dirty_flag[..] = true` has exactly three producers.

Round 1 returned one BLOCKING; fixing it added a definite-assignment analysis and a disjunct, none of
which any lens had seen, and round 2 found three more BLOCKINGs, all in that fix, all letting the same
counter design back in through a different door. Two of the three were reachable by a one-token edit to
round 1's own repro.

A comment written in the same slice said an array-word imprecision "can produce a value depending on an
unwritten WORD of an array the body does write — a shape no corpus design or probe has produced".
Review refuted both halves in one design: the laundering needs no array at all (a packed part-select on
a plain `reg [15:0]` does it), and two probes produced it.

Waiting for that review, a background loop was armed on `grep -c finished <journal> -ge 2`. The journal
writes `{"type":"result",…}`; the word `finished` never appears in it, so the loop could not terminate
and ran for four hours until the user noticed it. The review itself had completed long before.

Rules produced: find out who answers when you make a refusal unreachable; ask whether another lane of
your own tool already answers the question; check any comment asserting that duplication was avoided;
a pin that names its prerequisite pays for itself; license a skip by a complete dependency set;
measure the pair when two halves of a gap block each other; keep a measured-dead defensive arm and say
so with the method; build the DEFECT rather than the shape; look for a disjunct when a sound gate
kills the feature; run the design and read the output before writing what it means; enumerate the
writers behind any "this can only happen once" premise; re-review after fixing a BLOCKING; build the
design that tests a bound you write beside a known imprecision; and run a wait-loop predicate once by
hand before arming it.

### Row 30 — a fill's width, and three rounds of fixing my own fix (§4.5.406, reverted)

Measured: an unsized fill (`'0`/`'1`) is context-determined and vita's constant folder gave it a hard
32. The fix is four small pieces and it works — 165 of a 264-cell census, 778 of the review lenses'
1,622, zero regressions on its own axis. It was reverted anyway.

IEEE §11.6.1 evaluates a context-determined operand at `max(the context, every self-determined
operand's width)`. Handing the LEAF the context and letting it freeze there is wrong the moment a
sibling is wider: the operator above computes the real width and extends the frozen value with zeros,
because a fill is unsigned. `localparam logic [7:0] M = ('1 & 32'hff00) >> 8;` is `ff` in both oracles
and became `00`; `~('1 & 32'hff00) >> 8` was an honest `E3009` and became a silent `ff` — 150 cells.
Both lenses found it independently, and neither the 264-cell census nor the byte-identity argument saw
it, because every form in that census paired the fill with a one-bit sibling. The same blindness made
"width 32 and below is already correct" false: `logic [7:0] A = '1 >> 2` is `ff` and both oracles say
`3f`, 82 more cells at widths 1..31.

A routing gate and a soundness guard fail in opposite directions. A routing gate that over-reports
costs nothing; a guard whose decline is a LOUD cannot over-report at all. Reusing one predicate for
both put a fill in a self-determined position (a shift count) into the guard's hazard set and declined
folds the pre-slice build performed correctly: 104 NEW-LOUD, in all four binder copies.

The argument that "every caller reaches this fold as one link of an `or_else` chain whose next link is
the pre-slice route" was measured false: `param_bits_at_declared` is reached through
`param_i64_at_declared`, which is the LAST link of every binder chain — its `None` goes to
`param_value_unfoldable`, that is, `E3009`.

Of the 504 cells the slice converted from loud to a value, 215 were silently wrong on the SIGN axis:
`localparam logic [7:0] B = ($signed(4'hF) + 1) | 8'h00;` is `00` against both oracles' `10`, with no
fill anywhere, before and after alike, because `fold_bits_at` decides an expression's sign node-locally
where IEEE §11.8.1 makes the whole region unsigned if any operand is. The slice did not create it; it
removed the loud standing over it.

Round 1's defect was in the slice; round 2's was in round 1's fix; round 3's was in round 2's fix and
landed on a different axis, in shared code the slice only routes into, that predates it. The separable
part was the shift COUNT (a fill there is one bit, IEEE §5.7.1 plus §11.4.10): it needs no width and no
sign context, every cell it moves already had a wrong number, and it fixed 288 of an 880-cell census
with nothing turning from loud into a value.

"A literal `false` for every design without a fill" was true of the ANSWER and false of the cost: the
predicate opened by delegating to a whole-subtree walk and then recursed, so it re-walked the subtree
at every node. A fill-free 8,000-term initializer went 0.31 s to 1.26 s, superlinear (3.2x per doubling
against a linear 2x).

Two review rounds in a row were handed a stale artifact because the tree moved after the snapshot, and
both times a lens opened its report with that instead of with a finding.

Rules produced: a context width is not the leaf's width, and a census's band is a property of its
operands; a routing gate and a soundness guard need different predicates; walk each caller to the end
of its chain before writing "a decline is free"; count the loud→value column separately with a control
twin per axis; three rounds of blockers all in your own fixes means the axis; grep any predicate that
calls a whole-tree helper and then recurses; and freeze the binary, saying so if you unfreeze it.

## 2026-08-27

### A green suite over a fabricated default (round 36, §4.5.387)

Measured: the round-36 cast reorder is provably value-neutral — coercing a widening 2-state cast at
the OPERAND's width and extending afterwards equals coercing the extended value, because the extension
bits are a literal 0 or copies of the sign bit and `CaseEq` is a per-bit function. Both signednesses
were derived, written into the comment, and run against live iverilog over 90 cells: PRE == POST on
all 90.

It was still a silent-wrong, because the argument says "the operand's width" and the code says
`ir_bits_of(e).unwrap_or(32)`. Where `ir_bits_of` answers `None` — a deferred hierarchical reference, a
`string` net, the string-producing system functions, the element-typed `pop`/array-reduction family —
that 32 is a fabrication. `longint'(u1.w40)` with `logic [39:0] w40` is `0000001234567800` in iverilog
13 and in PRE, and `0000000034567800` under the unguarded reorder, at exit 0. The whole 6,181-test
suite was green over it, and so was the 90-cell three-way sweep, because every operand in both had a
declared width.

Rules produced: after deriving an equivalence, read the code back and name every input the derivation
assumed, then ask what the expression does when that input is a DEFAULT and build the design where it
is; and note that a differential sweep certifies only the fields it varies.

### The profile is not allowed to say zero for something that runs (round 36)

Measured: the reporter asked for a call tree to task granularity. vita lowers a subroutine two ways —
a frame body entered through a runtime call seam, and an inline splice that copies the callee's
statements into the caller at elaborate time — so a profile keyed on the runtime seam would report
`0 calls` for every inlined subroutine, and a task showing 0 reads as free.

Rule produced: the accuracy ladder applies to the observability rail; the call tree was refused and
the per-builtin half shipped, with the prerequisite written into the queue.

### State the attribution convention in the artifact (round 36)

Measured: a nested-cost table is ambiguous until someone says whether a row's time includes its
children. The `builtins` object answers it in the file — `attribution: "self"` and
`included_in_processes: true` — and the convention is verified by construction: on a nested
`$fdisplay(fd, "%s", $sformatf(…))` the inclusive convention sums past its own parent, so a
reintroduced double count is visible in the numbers.

### Measure the settle, not just the expression (round 36)

Measured: three rounds of a report chased the cost of evaluating an expression. The largest single term
was how many times the expression is evaluated — a continuous assign whose RHS contains any call or any
system function is visited 6.00x per input change instead of 1.00x, because the purity predicate that
feeds the dirty worklist rejects both node kinds outright. It also fires on vita's own inliner output,
making an inlined function measurably slower than the same expression written by hand.

Rules produced: a per-evaluation cost and an evaluation count multiply, so divide them out early; and
the blast radius of a conservative predicate is invisible until someone counts.

### A green test on one spelling (round 37, §4.5.388)

Measured: the report was "vita cannot do hierarchical references into generate blocks". Three of the
four things that phrase covers already worked, and the fourth had been broken since the beginning.
`hier_ref.rs::named_generate_block_read` pins `gblk.x` and has been green since the initial commit, so
every casual check came back yes while `u.gblk.x` — the same name, one dot further out — was E3010 in
19 measured cells. `hier_resolve`'s arm (b) said, in as many words, "Map only the leading segment." That
sentence is the entire defect, written down, in the function that has it; it read as a scope note
because for the same-module spelling the leading segment IS the block.

IEEE §27.4 makes a generate-`for`'s blocks an array whose name is illegal unindexed and a conditional
block a singleton whose name is legal bare. Both leave the same key in the symbol table when the loop
runs once. The first draft of the fallback keyed on storage alone ("does `[1]` exist"), which answered
a bare label on a two-trip loop with element 0 — a correct loud refusal traded for a silent pick.
Adding the `[1]` test fixed that cell and left the subtler one: a one-trip loop still resolved, so the
reference worked at one iteration and went loud at two.

Fixing the dropped generate-`case` label looked like it needed a `label` field on `GenCaseItem`, which
would have flipped the `hdl-ast` SchemaHash. Wrapping the labelled body in the `GenItem::Block` the
elaborator already scopes reuses the existing naming instead.

Rules produced: enumerate the spellings of a feature a report names; treat a comment saying "only" the
way you treat one saying "cannot"; record a syntactic fact at the site that knows it and test the
degenerate count; and check whether an existing node already carries the meaning before adding a field
to a hashed type.

## 2026-08-26

### A report's severity is a claim (round 34, §4.5.385)

Measured: an item arrived as a LOUD gap — "`-G/--param` cannot carry a value wider than 64 bits". The
repro was exact and the triage was still wrong, because the report varied the OVERRIDE and held the
DECLARATION fixed. Varying the declaration instead found 19 silent-wrong cells under the same line: on
`parameter logic [127:0] K = <wide default>`, `#(.K(5))` was DISCARDED and the default used, at
`errors=0`.

Rule produced: a census must vary every field of the shape, not only the one the report names; the
field that matters is often the one the report held constant.

### A revert's conclusion is scoped to the workloads that measured it (round 34)

Measured: `wprog.rs`'s module header records that dropping the sign half of the admission gate was
built, measured SOUND, measured "slow lane −19.0%" and 1.00x, and reverted. Every number is right. The
conclusion — "it buys nothing" — was true of picorv32 and keccak, which is what was measured, and false
of a `localparam int` operand, which is what SV RTL writes and what a `generate for` genvar is. There
the same gate costs 1.61x.

Rule produced: record the SHAPE the measurement covered, not only the verdict, and re-run when a new
workload shows the shape.

### A wording assertion outlives the limitation it describes (round 34)

Measured: three pins broke in one round, all of the same kind — they asserted the TEXT of a refusal and
the round removed the refusal. One asserted "WIDER than the 64-bit integer channel"; two streaming pins
asserted that "§11.4.14" appears. Each was correct when written and each had to be rewritten as a VALUE
assertion.

Rule produced: pin the wording only while the construct has no value, say in the docstring that it is a
wording pin, and pin the value once there is one.

### A probe whose input is a fixed point (round 34)

Measured: `{<<{8'hA5}}` reverses bits, and `8'hA5` is `1010_0101` — a palindrome. That cell passes
whether the implementation reverses, copies, or returns its input.

Rule produced: choose inputs where every wrong implementation gives a different answer, and say in the
comment why that input.

### An extension needs the expression's signedness (round 34)

Measured: `64'hFFFF_FFFF_FFFF_FFFF + 64'd0`, `-(64'sd1)` and `32'd0 - 32'd1` all reach parameter binding
as the same `i64`. Both oracles extend the first with zeros and the second with ones. Reading the sign
of the container would be right for two of the three and silently wrong for the other, and nothing
downstream could tell which.

Rule produced: the channel that carries a value across a width boundary must carry the source's
signedness, and where it cannot know it (a `defparam`, whose collector folds before the record exists),
`None` must DECLINE rather than pick a default.

### Predicting an oracle's answer and pinning it (round 34)

Measured: a cell in the new sign battery asserted `-100 >>> -7 == -1`, reasoned from IEEE §11.4.10's
sign fill. It is 0 — a negative SIGNED right operand makes the whole expression unsigned (IEEE §11.8.1),
so the fill is zero. All three tools said 0 and the test failed on the first run, which is the good
outcome, but only because the value was asserted at all.

Rule produced: assert the value, not the exit code; a cell that had asserted "exit code 0" would have
shipped the wrong understanding silently.

### An A/B on hand-written source text (round 35, §4.5.386)

Measured: the report measured its own design two ways — calling a small `function`, and pasting that
function's body into the call site — got −30% for the pasted version, and asked for vita's inliner to be
widened. Both halves of the measurement were real; the inference was not. Pasted source text has no
formals. vita's inliner binds the actual to the declared formal, and for a 2-state formal that binding
builds a per-bit coercion; routing the report's own `idx()` through the real inliner measured 98x slower
than the frame call it was asking to replace, against 0.83 s for the text they pasted.

Rule produced: when a report proposes "make the tool do automatically what I did by hand", build the
cell where the tool does it — that cell is usually missing from the report, because the reporter could
not run it.

### A fan-out multiplier hiding as a slow operator (round 35)

Measured: a ternary appeared to cost 10-50x what plain arithmetic cost in the same loop, on all three
backends. The ternary was innocent. `int'(e)` lowers to a `Concat` of one `CaseEq(Select(e, i), 1'b1)`
per target bit, the evaluator walks that DAG as a tree, and the cast names its operand exactly
`target_width` times. A `$display` inside the operand turned a timing mystery into an integer: 8 for
`byte'`, 32 for `int'`, 64 for `longint'`, 1024 for `int'(int'(x))`, 1 for iverilog. And `integer'` and
`int'` are both 32-bit and signed and differ only in 2-state-ness — 27x apart.

Rules produced: count, do not time; and vary one attribute against a twin that shares the others.

### The guard you need may already exist one call site away (round 35)

Measured: `coerce_two_state` has two callers. The inline path's formal binding gates it on
`expr_may_be_unknown` and carries a comment recording this exact measurement ("42.7x on a `longint`
one, 23x `.velab` growth, and nesting multiplies it"). The cast path called the same routine with no
gate. And when the guard was added it measured as a no-op, because a widening cast goes through
`extend_to`, which produces `Concat[Replicate(sign), e]`, and `expr_may_be_unknown` had no `Replicate`
arm, so every widening cast fell into the catch-all.

Rules produced: grep the other callers of the thing you are gating; and check what the other paths into
a guard build, because a gate can be correct and still be dead.

### Fixing part of a wrong count is a rung, not a resolution (round 35)

Measured: the guard takes `int'(int'(f()))` from 1024 evaluations of `f` to 32. Icarus evaluates it
once. The values are byte-identical either way, so nothing on the accuracy ladder moved for a pure
expression — but for an operand with a side effect the count IS the semantics, and 32 is still wrong.

Rule produced: record the partial fix in the queue in the same slice, with the number.

### "It collapses" is a claim about a curve (round 35)

Measured: a report observed a five-engine top running about five times slower per cycle than a
single-core testbench and read it as throughput collapsing with instance count. Over a 128x sweep the
cost is a straight line to within 3.6%, with the marginal cost per instance-cycle flat from N=2 to
N=128. Two points cannot distinguish linear from super-linear, and the reporter had two.

Rule produced: get enough points to fit, and report the residuals.

## 2026-08-25

### A diagnostic's text is a claim, and it decays (§4.5.380)

Measured: §4.5.374 removed the "direct rhs of a blocking assignment" restriction on the file-read
system functions. The diagnostics that state that restriction were not touched, so for a month vita
told users that working code was illegal, and the report's tester correctly noted that following the
message would make them revert four spellings that pass today.

Two sites carried the removed rule — the file-read family and `$value$plusargs` — and they reject for
different reasons: the first because a hoist would change how many times the call runs, the second
because it would move a ref write ahead of a read in the same statement. A single find-and-replace
would have made one of them false again. The replacement text said "a task argument … is supported",
and `$monitor`'s argument is a task argument that is correctly refused, because `$monitor` re-renders
and would show the frozen temporary; an existing pin caught it. The rule that survived contact was not
a list of positions at all but a principle: the call must run the same number of times as written. The
carets pointed at the statement head, so a multi-line condition sent the reader to the `if`, while the
queue-pop diagnostic one module away already did it correctly.

Rules produced: grep the diagnostic text for a lifted restriction's wording and re-derive each site's
reason; write the replacement against the cases that still reject; point the caret at the operand; and
re-run every item of an outside report at HEAD (this one was filed four slices back, and re-running
separated "still true" from "already fixed" and from "true but not a defect" — its `always_comb`
initializer item is accepted by iverilog too, so it is a lint request).

### A reverted slice's prerequisite already answered by an existing map (§4.5.382)

Measured: §4.5.373 built a reduction, reverted it, and wrote the prerequisite in two stages — "the
width must be declared provenance, and the value must be canonical at that width" — and calculated
that eight sites using `param_meta` would have to agree. One existing map answers both: `param_range`
is filled by `param_decl_range` only from a declared range, type or sized literal, and where a
declared range exists the binding already truncates the value to that width. Re-running the
counter-example shows it in place: `parameter A=4'h1; localparam logic [3:0] W=A<<4;` is 0 in vita and
0 in iverilog.

Rule produced: read a reverted slice's prerequisite and search for a data structure that already
records the property before building it — the slice wrote down the search terms.

### A cast is the operand's context (§4.5.382)

Measured: IEEE §11.6.1 evaluates `N'(e)` at `max(self(e), N)`. Folding at the operand's own width and
resizing afterwards is a different operation: `65'(64'd18446744073709551615 + 64'd1) >> 64` is 1 in
both oracles, because the sum carries into bit 64, and 0 if folded at the operand's 64.

Rule produced: a walk that cannot carry a width must decline when a context-determined top must go to a
wider context; the same sentence does not apply to a replication count, a `$clog2` argument or a
generate condition, because the LRM declares those self-determined. The LRM decides which, not the
code.

### A count-position resolver that answered with a local (§4.5.382)

Measured: reusing a general resolver in a count position lets an interpreter's LOCAL VARIABLE supply
the count — `int n = 2; {n{4'hA}}` folds, where iverilog says "a reference to a net or variable is not
allowed in a constant expression". Where only a shadow exists with no value, walking past it and
picking up a same-named module parameter answers for an object the reference did not name.

Rule produced: pass the position as a flag and make the resolver that receives it refuse rather than
read or walk past an environment.

### A pin passing for a different reason than it claims (§4.5.382)

Measured: `generate_scope_decl_init_is_loud` said "a queue decl-init in a generate body is not
desugared, so it is loud". That desugar had been opened in an earlier slice; the cell kept passing
because a keyword-less `if (1) begin … end` did not parse. When the parser accepted IEEE §27.3 the
design ran and printed iverilog's answer. In the same round,
`unbound_local_never_resolves_a_same_named_param`'s "if the width is unknown it is loud" cell was
passing because the initializer did not fold.

Rule produced: establish a pin's claimed property a second way (another spelling, another
initializer); a property that holds only one way is measuring a path.

### A report's axis and a control twin (§4.5.382)

Measured: a round-33 report named the performance axis as "an unpacked array element on the LHS", from
a comparison of three designs (element LHS losing 1.88x, scalar winning, procedural a draw). Changing
one thing at a time moves the axis: with the same element LHS, changing only the index from `a[gy][gx]`
to `a[gy][(gx+2*gy)%5]` gives 0.92x against 2.05x. The LHS was only what made the index exist. Fixing
the wrongly-named axis produced a byte-identical binary that moved nothing.

Rule produced: build a twin that fixes the named axis and varies the rest.

### Removing a defensive check on the strength of a green suite (§4.5.368)

Measured: §4.5.368 removed a duplicate `mask_top()` from `Value::resize`, justified by "a
`debug_assert` was planted and 5,812 tests ran with zero firings". The adversarial lens refuted the
invariant with the engine: `$realtobits` stamps `width = 64` while leaving the plane at the argument's
width, so `$realtobits(<128-bit>)` produces a non-canonical value. No test had ever called it with a
non-64-bit argument. The risk direction is the worst kind — release is fine and debug and CI die,
because the consumer indexes by `nwords(net width)` and absorbs the extra word — so the fix belongs at
the producer. In the same round, `arena.rs`'s zero-width arm rested on "no width-0 net is created
today" and was fixed as well even though it is unreachable.

Rules produced: write down the invariant a removed check was keeping and enumerate the sites that
establish it (constructors, direct struct literals, `.width =`, direct plane writes); check the
direction of the risk; and do not let an invariant rest on a side condition.

### A side map keyed like another map (§4.5.383)

Measured: `param_range` is keyed exactly like `params` and is written by 5 of about 13 binders. That was
safe for months for a reason nobody had written down: the fully-qualified key made every scope disjoint,
so no binder could rebind a key another binder had ranged. §4.5.383 broke that property without noticing
— a wildcard `import pk::*` binds a package declaration at the module's own key, and then a local enum
label, a genvar, a real parameter's integer twin or a body parameter rebinds that same key one phase
later. Two of those were live correct → silent-wrong, and the author had added the clear at three sites
and believed it complete; the adversarial census found 13 writers.

Rule produced: when you make a key space rebindable, every writer of the primary map becomes a writer of
the side map — route them through one funnel with a `debug_assert` enforcing the order, so that a grep
for the raw writer returns only the funnel.

### A census that varies one field of a record (§4.5.383)

Measured: the queue line said "the runtime lane already prints the right value in all three tools", and
it had been measured — on a zero-LSB declaration, where the offset normalization is the identity. With
`parameter [39:8] B`, the same runtime read printed 171 against both oracles' 52, at exit 0.

Rule produced: when the thing under test is a record, the census must vary every field.

### Build the new table with the same producer as its twin (§4.5.383)

Measured: the slice genuinely needed a new map (`pkg_const_range`), and what made it safe was that it is
filled by the same `param_decl_range_opt` the module twin is read back through. One producer means one
provenance rule, so "a value-inferred width declines" did not have to be re-argued in the second scope
and the review could check the claim by reading one function.

### A gate in an earlier phase cannot supply your phase's precondition (§4.5.384)

Measured: §4.5.384 needed §4.5.373's second prerequisite — the recorded width is usable only if the
stored value is canonical at it — and argued it was already supplied, because IEEE §6.19 makes an
out-of-range enum label a loud `E2002`, with a test file to prove it. Both adversarial lenses refuted
that independently, from opposite ends: the check runs in the PARSER over bare literals, while the fold
runs at ELABORATE over `const_eval_in_scope`, which resolves parameters, package constants and constant
functions. Everything in the gap is accepted silently, and both oracles reject those designs.

Rules produced: compare the two phases' RESOLVERS, not their intents; name what actually holds the
invariant (here, that every consumer narrows to the recorded width) and say so in the doc; and cap the
recorded quantity at the source, because the one operation that would amplify was unreachable only
because a different component happened to refuse first.

### A probe whose answer equals its failure mode (§4.5.384)

Measured: `EA[5]` is 1, and a bound the fold declines clamps to 1 as well, so the bit-select and
equal-endpoint cells of the census read "correct" in PRE. Scaling the result (`EA[5]*52`) separated them
and both turned out broken.

Rule produced: for every cell, ask what the number would be if the feature did nothing; if that equals
the expected answer, the cell is decoration.

## 2026-08-24

### Narrowing by theorem (§4.5.373)

Measured: putting six reductions into the constant domain surfaced the width-provenance problem
(`param_meta` mixes declared and inferred widths and fills a miss with 32). The first narrowing was
argued from a theorem: widening the window only adds bits OUTSIDE the operand, which are 0 for a
non-negative value and 1 for a sign-extended negative that already has a set bit, so "is any bit set"
is invariant under width. That gives `|` and `~|` safe at any width and the other four safe only where
the source states the width. It looked clean and it kept the target design.

The premise was false. vita does not wrap a parameter to its claimed width:
`parameter A = 4'h1; localparam W = A << 4;` gives `W = 16` with `$bits` 32 where both oracles give
`W = 0` at 4 bits. The extra bits are INSIDE the true operand rather than outside it, so `|W` is 1
against the oracles' 0 and a `generate if (|W)` flips a branch.

The test written after the narrowing checked `code == Some(0)` and `bits = 32` and did not check the
VALUE, so it was green with the value opposite to the oracle; its docstring said "PRE also prints 0, so
this is pre-existing" and PRE was `E3009`. Both were caught by the adversarial lens.

Three different requests — a select bound in §4.5.371, a concat width in the same slice, and this
reduction — stopped at the same place: the width can be computed and cannot be vouched for. §4.5.373
dug one layer further: before provenance, the value is not canonical at that width.

Rules produced: a theorem is only as true as its premise, so measure a premise that is a data-structure
invariant; put the observed value in the assertion of a residue-pinning test and run PRE before quoting
it; and when three requests stop at one prerequisite, file the wall as one infrastructure line.

### Moving an effect and naming the resource (§4.5.374)

Measured: a hoist that pulls a side-effecting call to the front of a statement was gated on ROOT NAMES,
with an early exit for the shapes that do not use their argument (`$fgetc`, `$fopen`, `$ungetc`). The
comment justified it: they mutate only fd state, "which no expression in the statement can read", since
`$feof` is a separate call and is pure. `$feof(fd)` is pure with respect to its ARGUMENT and reads the
file position, and "it would be re-evaluated where it stands either way" is the definition of the
defect rather than a safety argument. Measured after the file is exhausted,
`x = $feof(fd)*10 + $fgetc(fd)` is 9 in vita, −1 in iverilog, and loud in PRE. The defect is
value-dependent: the two tools agree in the middle of a file and diverge only near EOF.

The same slice produced the third instance of the same shape: the full suite offered eight pins
encoding old restrictions, one at a time (nextest stops at the first failure), and one of them said
"an arbitrary expression position has no statement to desugar into, so it is loud — deliberately, not
as a gap to close cheaply", which is exactly what the slice built.

Mirroring `hoist_stmt_general`'s statement arms into the new family dropped the `UserTaskCall` arm's
`task_call_inout_root_written`, whose reason was already in a comment beside it (an `inout` actual has a
copy-in and therefore READS its actual). Restoring the predicate did not work either, because it looks
at `inout` FUNCTION calls and cannot see system functions; moving the QUESTION rather than the
condition gave a different and correct answer.

The overlap gate missed aliases in two directions — a self-hierarchical path (`m.a` is the same net as
`a` under the v1 flatten) and a package-scoped name (`p::v` does not reach the `Ident` arm). Instead of
adding shapes, an `opaque` flag was raised on any read whose root name cannot be determined, and the
statement refused when it is set; the cost is structurally narrow, because it only applies to shapes
with a write and the target has none.

Folding a duplicated `shape()` call put `Shape::Unevaluated` in the same arm as `Shape::Uncond`.
`Unevaluated` carries the contract that `$bits` and array queries report the operand's TYPE and read
nothing at runtime (IEEE §20.5/§20.6), which is why `shape_children` gives it `[]`; merging made
`$bits(a)` contribute a read and false-rejected a harmless statement.

Rules produced: enumerate the observers of every effect you move, asking whether a name READS the
resource; test a state-moving transformation at the boundary; write what you measured rather than why
something cannot be done; mirror the questions rather than the conditions; close a gate's blind spots
fail-closed; and check that two enum arms share a contract before merging them.

### Removing a gate, and measuring against a moving binary (§4.5.375, reverted)

Measured: `$readmem*`/`$writemem*` accepted only a string LITERAL for the file name, and a
non-matching shape hit a bare `return` — no file read, no diagnostic, exit 0. The branch immediately
below it, the memory argument, WARNS when it does not recognise its operand; the two sat four lines
apart. IEEE §21.4 asks only for a string EXPRESSION, and the canonical SoC testbench keeps the name in
`reg [1023:0] firmware_file`, so the shape that hit the silent arm was the common one.

The gate being removed was the hierarchical memory argument, and underneath it were two independent
pre-existing defects, neither visible until it came off: the silent file name above, and the t0 process
order — vita runs a parent's `initial` before its child's, so a RAM that loads its own memory
overwrites the testbench's load. Both were invisible to the minimal probe (`$readmemh("f.hex",
dut.mem)` with a literal name and no child `initial`) and both appeared the moment the probe was
widened to what serv actually ships.

The feature was correct — 40-odd shapes matching both oracles, one lens CLEAN, the real SoC running end
to end — and it still had to go, on three facts about the root: the ordering is pre-existing and
independent (a plain hierarchical write already loses to a child's `initial`, with no `$readmem`
anywhere); `sim_ir::Process` is frozen and has no instance field, so the order needs an out-of-band
rank sidecar; and the scheduler's `tie` is shared with combinational seeding and fork children. The
file-name fix needs no hierarchy at all, so it shipped while the construct that uncovered it did not.

The soundness lens ran an exhaustive ROUTING census on the hierarchical memory argument — which nets
the exemption admits (heap kinds, mangled names, automatic locals, clocking holds, `NetKind::String`,
const parameters), and whether each write lands in the right storage, on both backends and through the
staged pipeline — and reported CLEAN twice. It never asked WHEN the write lands relative to the target
instance's own initialization, and every probe it wrote gave the child RAM no initializer of its own.

The file-name fix escaped non-printables at the two sites that could receive a NUL and left three
others interpolating raw, defensibly, because those three fire only after the filesystem accepted the
name. The reviewer verified the reasoning and still flagged it: those three can see a tab or a newline,
and a newline splits one warning across two lines.

The differential lens ran while the session rebuilt `target/release/vita` three times and had to
retract four findings — all four matched both oracles once re-measured. It recovered by copying the
binary to a frozen snapshot, recording its md5 and the md5 of the `git diff` it was built from.

Rules produced: say so when a handler cannot use an argument, and read the neighbouring branches; run
the REAL design a loud gate was blocking; decide fix-or-revert from the root and ship the part that
stands alone; run an ordering census as well as a routing one for a construct that writes into another
instance; apply an escaping rule at every site; and give a review a frozen artifact.

### A truncated tool result is not a completed edit (2026-08-24, caught by the owner)

Measured: a tool call ended with "Tool result did not finish before the conversation ended" and work
continued as though the edit had landed. It had not — the file was unchanged, and the next several
steps reasoned about a tree that did not exist. A truncated result looks like a display problem and is
equally consistent with an execution problem.

Rule produced: re-establish the state before continuing — `git status --short` and `git diff --stat`
for an edit, a re-read for a write, a re-run for a measurement.

### A revert's reason is a measurement (§4.5.376)

Measured: §4.5.375's revert rested on one sentence — a parent's `initial` runs before its child's
"while both oracles run the child's first". Re-measured on the exact design it cites: iverilog prints
`aa bb cc dd` and verilator prints `01 02 03 04`, which is vita's own answer. The two oracles do not
agree; IEEE §4.7 makes `initial` execution order explicitly non-deterministic. There was no two-oracle
silent-wrong, so there was nothing to revert for.

Within one slice the claim had been written into the §3 queue line, into a new §2 row, into the §5.2
briefing and into a test docstring — four copies, each citing the others' neighbourhood, none an
independent measurement. A day later it read as thoroughly corroborated and it ranked the next slice.

The check that settled it: when a tool's answer matches the value a competing write would leave, you
cannot tell ordering from a dropped write. Removing the competitor, verilator honoured the parent's
hierarchical `$readmemh` on its own (`aa bb cc dd`), which made `01 02 03 04` provably an ordering
result. The revert also claimed the construct went silent-wrong on its own motivating idiom, and none
of the four upstream testbenches it named has a competing child load; serv does not elaborate with or
without the construct, so PRE and POST stop at the same three unrelated errors, byte-identical.

Rules produced: a claim whose only evidence is a previous slice's prose is one measurement, not four;
record the second oracle's actual output text; delete the confound and ask again; and check the demand
claim with the same suspicion as the correctness claim.

### Routing a value out of a map breaks its proxies (§4.5.377)

Measured: the package parameter fold answered one question — integer-only — so `pkg_consts` happened to
hold every package parameter. Adding the string and real domains routed those two out of it, and both
regressions the slice produced were code that had been using `pkg_consts` to mean something else. The
duplicate-name check for a package's single name space (IEEE §26.3) asked `consts`, so
`parameter S = "RED"; int S;` ran at exit 0 where both oracles reject it, while the integer twin
`parameter N = 7; int N;` stayed loud — which is the tell. And `nonconst_bound_reason` fell through for
a name it did not recognise, so `logic [P::S-1:0] v;` silently clamped to one bit where both oracles
give 5391684; its own comment had predicted this.

Two cheap detectors both fired: the DOMAIN TWIN (the identical design in the domain not changed) and
the SCOPE TWIN (the same design at the scope not changed — module scope was loud and package scope went
quiet). Both were found by the slice's own soundness lens; the suite went green with both defects
present, because no test had ever paired a string parameter with those two contexts.

Rules produced: enumerate a shared map's readers before measuring anything, asking of each whether it
reads a value or uses membership as a proxy; and fix a stale proxy by retyping it rather than extending
the predicate.

## 2026-08-23

### The workload corpus (§4.5.369)

Measured: performance judgement had long rested on two designs — `picorv32` (third party) and
`bench/keccak`, which this project wrote, and wrote in order to measure. Those two drew a picture of
"level with iverilog, losing only on one kernel". Bringing in eight permissively-licensed third-party
designs and pinning them to an oracle flipped the picture in both directions: on real third-party RTL
vita is generally ahead of iverilog, and three of the eight did not run at all. All three failed on one
axis — parameter folding in the constant domain (a string ternary, a string `Ident`, an integer
replication) — which the internally-derived §2 queue carried as a single line.

All eight scouts were given the same order: clone, licence, self-contained run configuration,
iverilog, and only then vita, with the instruction written verbatim into the brief that a design vita
rejects or gets wrong is the most valuable result available and that the RTL must not be simplified,
rewritten or shrunk to make vita accept it.

`bench/picorv32`'s testbench prints `trap=%b addr=%h` — the last two values of the run — so every
divergence the design later overwrites is invisible. The corpus contract became a `DIGEST=` line
accumulated over the whole run, folding the bus and handshake bits with a rotate-xor every cycle after
reset.

That digest was deterministic, cycle-resolution, byte-identical across re-runs and an exact match with
iverilog — and asserted nothing about the core. Mutating `picorv32.v:1240` from `reg_op1 + reg_op2` to
`reg_op1 + reg_op2 + 32'd1` left `DIGEST=7836648e76208dc9` unchanged, because the program is only
`addi`/`add`/`beq` and no computed register value ever reaches the bus. The only check that does not
pass over an empty gate is a mutation. verilog-ethernet initially reported "does not move" for a
different reason: it is a loopback design, TX and RX share one `lfsr` instance, and changing the CRC
polynomial makes TX attach a wrong FCS that RX verifies as correct. Touching only the RX side moved it
immediately.

The most valuable finding of the slice came from a differential lens that stalled for five hours and
was killed; its last written sentence was that picorv32's digest does not move under a mutation of the
core's adder and that it was running an iverilog control to find out whether the mutation was dead.

Each manifest row carries `Expect::Runs` or `Expect::Refused { diag }`, which separates outcomes that
look identical from outside: a refusal matching its pinned diagnostic (green — the ladder is working),
a refusal for a different reason (`DRIFTED`, red — the pin does not describe the design), a design
that now runs (`PROMOTED`, green plus "move the row"), and a refused design that answers and answers
wrongly (`REGRESSION`, red). That last cell is why the design exists; scoring it as a promotion would
pass the one move the ladder forbids. A unit test pins it by name.

`measure()` takes everything to be measured at once so round-robin is the default shape and the first
round can be discarded, and the runner warns when handed a debug binary. That was first written up as
"a caller has no way to measure sequentially", and the adversarial soundness lens refuted it on the
spot: `measure(&jobs[0..1])` followed by `measure(&jobs[1..2])` compiles, and that is exactly the fake
+12.5%. Interleaving is a property of the call site, not of the type.

Each manifest row also had `expect` and, beside it, a free `expect_exit`. The three refusal rows had
`expect_exit: 1`, so on the day the gap closes vita exits 0, the equality breaks, and the grade is
"was loud, now crashes" — the event the corpus exists for, reported red and with a false message, and
`Grade::Promoted` and its whole message were unreachable dead code. The fix moves the value inside the
variant: `Expect::Runs { exit: i32 }` leaves a refusal row nowhere to write an exit code.

Rules produced: survey third-party RTL before ranking; run the oracle first and forbid shrinking the
workload; accumulate the digest over the whole run; mutate one upstream line asymmetrically and re-run;
read a stalled agent's partial output; record refusal as a first-class state; build measurement
discipline into the default shape without claiming the wrong shape is impossible; and nest a dependent
field inside the variant whose validity it depends on.

### Widening a domain that cannot carry what it claims (§4.5.370)

Measured: string constant values live in `str_param_raw`, which does not carry a width, and parameter
consumers ask the string path before the numeric one, which does. Widening the domain made
`localparam [95:0] X = {"A","B"}` fold 96 bits into 16 (both oracles 96) — PRE was loud, so
loud → silent-wrong. The same slice hit it twice: `{"ab",""}`'s empty string is one NUL byte
(IEEE §5.9, both oracles `0x616200`) and a text join represents it as zero characters, giving
`0x6162`. Both answers are the same: decline.

The width gate was written as `p.range.is_none() && p.ty == Implicit`. Three keywords fall through it —
range-less `logic`, `reg` and `bit` are 1 bit, with `ty` `Implicit` and `range` `None`. The parser
records that in `var_kind` and throws it away, and `ParamDecl` has no such field, so `parameter bit P`
and `parameter P` were literally indistinguishable downstream. That gap was already producing a
silent-wrong: `localparam bit N = 8'hFF` read 255 at 8 bits where both oracles give 1 at 1 bit. The fix
was three lines, because `var_kind` survives as far as `finish_param_assignment` — added as the LAST
fallback, so an explicit range still wins.

An escalation guard asking "is this parameter a string?" was asked in the widened VALUE domain, so an
ordinary numeric parameter whose default happens to be a string expression (`parameter W = {"A","B"}`)
refused a legal numeric override — both oracles and PRE give `#(.W(9))` = 9, so correct → loud. Undoing
only the guard was worse: the folded default then swallowed the override and 16706 came out at exit 0.
The third version was the answer: a numeric override aimed at this parameter makes the fold fallback
stand down.

`const_str_in_scope` already resolved `StrLit`, `Paren`, `Ident` and `PkgScoped` and had exactly one
caller — string equality comparison — so nobody ever asked it for a VALUE, and seven parameter-binding
sites used a literal-only twin, which is what blocked three third-party designs.

Rules produced: count what a domain cannot carry before widening it; do not use an enumeration's
catch-all as a gate predicate; ask a guard in the domain of the question it answers; and count a
resolver's callers.

### An initializer's self width does not survive an override (§4.5.371)

Measured: taking an untyped parameter's width from the initializer expression's self width made
`#(parameter P = {2{8'h1}})` overridden by `#(.P(32'hDEADBEEF))` bind at 16 bits (`beef`) where both
oracles give 32 (`deadbeef`); IEEE §6.20.2 takes the range from the FINAL override value. One cell was
worse: the form with `wire [P-1:0] bus` was LOUD in PRE and POST silently built a 514-bit bus. The
comment calling that arm part of the type-determined family is exactly where it went wrong: a DECLARED
type survives an override (`parameter byte P` stays 8 bits) and a width determined from the value does
not.

Answering the same arm under `declared_only` was justified as "the width comes from the operands'
declared widths". The resolver that computes it refutes that: it reads a name's width from
`param_meta`, which is where value-inferred widths are recorded, and guesses `(32, false)` on a miss.
The concatenation was a wrapper that launders the provenance `declared_only` exists to protect —
`localparam W = ~8'hCB; localparam Q = {W}; logic [(Q[15:8])+8-1:0] v;` declared a 263-bit net where
iverilog gives 1. That is the regression §4.5.363 closed, re-entering through a concat.

Widening the replication count produced four BLOCKINGs across three rounds, the last three all made by
the previous fix: widening the value without the width twin degraded a width-aware walk to the
unlimited domain; folding through a name resolver picked up a constant function's LOCAL VARIABLE and
answered 170 (IEEE §11.4.12.1 makes a count a constant expression); switching to a module-scope
evaluator walked past a same-named local and picked up the parameter, answering 43690; and that
evaluator restarts call depth at 0, so a call inside the count overflowed the stack where PRE was loud.
The thread through all four is that a count is a question about scope, depth and provenance, not about
value, and those are different machinery from the width axis being worked on.

The width-twin defect was caught with no oracle at all: `({N{4'd15}} + 4'd1) > 4'd0` answered 1 and
`({2{4'd15}} + 4'd1) > 4'd0` answered 0 in the same file.

Rules produced: ask whether a width's source is still alive; separate computing a width from vouching
for its provenance; stop and count when blockers on one axis exceed three; and put the widened spelling
beside the original in the census.

### Ending a run inside a frame body (§4.5.372, partly reverted)

Measured: putting `$finish` in a subroutine body requires the body to stop mid-way, and what a halted
body leaves in the caller's lvalue is undefined. On `r = f(7)` the three tools give three answers:
iverilog 55 (it does not perform the assignment at all), verilator 21 (it runs the body to the end),
and vita's implementation `x` (it stops and commits the partial value). Every choice trades a loud for
a silent.

`$fatal` reached through an argument suppressed the output of `$display` and `$write` at the same
site; the co-located `$fdisplay`/`$fwrite` and the postponed `$strobe`/`$monitor` have no such check.
That was a residue until the same slice added a mid-body bail, at which point the unsuppressed sinks'
printed values changed from 8 to `x` — a silent traded for a silent.

`call_fatal` is not cleared once consumed, because the run was ending. Used as the predicate for "do
not print now", it removed every `final` block's output, since `final` runs after the scheduler has
consumed the latch; iverilog and the check-less PRE both print it. The predicate is
`call_fatal && !finished`.

Three files enumerate what a frame body may contain —
`elaborate/frames_classify.rs`, `sim-engine/native/frames.rs`, `state/frame_eval.rs`. Teaching one made
elaborate accept and native refuse, falling back to the VM with a message saying "the executor drops
that statement" when the executor executes it.

§4.5.371 had four BLOCKINGs of which three were produced by the previous fix; §4.5.372 had six of which
four were.

Rules produced: ask what a halted body leaves before asking whether the statement can run; close every
leaking sink before changing the value that leaks; find the code that still runs while a non-clearing
latch is true; grep every place that enumerates a subset; and stop and count when your own fixes are the
majority of the blockers.

### Reading a helper's contract (§4.5.367)

Measured: `copy_bits` OR-merges bits and its doc says "The destination range must be ZERO on entry",
which holds because every existing caller builds into `Value::zeros`. A part-select WRITE targets the
slot's current value, so calling it directly writes `8'h0F` into `8'hF0` and produces `8'hFF`. A
4-state slot is x-initialised so an anchor fires; a 2-state slot's first write is correct and only a
re-assignment is contaminated, so it passes quietly. The sibling that fixes it (`replace_bits` — clear,
then the same `copy_bits`) also inherits a different precondition: `copy_bits` indexes `dst.val[dw]`
directly and does not grow the allocation, where the `set_vu` it replaced does.

In the same slice, `width > 0 && lsb >= 0 && lsb + width <= BOUND` had been hand-copied into two files
with two spellings of the bound; §4.5.359 caught the same mistake after it had become six spellings.
The helper's doc then named its caller — and there were two.

Rules produced: read a helper's doc preconditions and answer in one sentence whether the new call site
satisfies them, building a sibling when it does not; extract a gate into a helper the moment you copy
it to a second site; and do not name a helper's callers in its doc.

### Performance method (§4.5.367, §4.5.368)

Measured: running PRE five times and then POST five times gave a fake +12.5% on picorv32 (0.45 s);
interleaving gave −0.9%. A performance slice handed the lens `target/debug/vita` (25.9 MB against a
release 5.7 MB) and the lens measured a +88% picorv32 regression before noticing and rebuilding
release. And fixing a site with 6.6% self time produced −15.6%, which is not a contradiction:
`Value::set_vu` is not inlined, so it appears as its own frame and its 12.8% leaf share counts only in
the caller's inclusive figure — the real target is about 19%.

Rules produced: interleave by run; measure release only and check the binary size when snapshotting;
and do not use a measured gain as a result until you have a mechanism for it.

### Fixing one site exposes the same axis (§4.5.366)

Measured: making constant comparison read as 64-bit unsigned exposed a latent defect in `>>>` — the
other sign-sensitive operation on that axis — and 14 cells went correct → silent-wrong. `>>>` had been
wrong before, and while the comparison read signed the two errors cancelled.

`const_i64_is_unsigned_at(w, signed)` asserts that an `i64` is the unsigned bit pattern at width `w`,
and its walk skips leaf normalisation in a context where masking is off, so a narrow SIGNED leaf
arrives sign-extended and code that trusts the predicate reads the extension as magnitude. The
part-select width built on it was loud → silent-wrong. The fix did both: extend leaf normalisation into
that context, and narrow the predicate (`>= 64` to `== 64`).

Above 64 bits the `i64` has already truncated the value. Reading the truncation as unsigned makes the
subtraction shape right (`(65'd1-65'd2) > 65'd0`) and the carry shape wrong
(`(64'hFF..FF + 65'd1) > 64'hFF..FF`); reading it as signed does the opposite. Neither dominates, which
is the definition of a guess. A sibling helper (`const_unsigned_selfdet`) had already drawn the boundary
in the same place.

Rules produced: classify every operator on an axis before changing its interpretation, and check what
was right by cancellation; answer in code who establishes a new predicate's property; and draw a
boundary and keep pre-slice behaviour when two readings are wrong in opposite directions.

## 2026-08-22

### Adversarial review briefing and battery (moved out of the loop file)

Measured practices recorded on this date: hand the reviewer a pre-built PRE binary by path and forbid
tree edits; give the already-killed mutation list and the documented survivors and require findings
outside them; name the previous round's numbers; say that a clean report is a good result; allow the
soundness lens a separate `CARGO_TARGET_DIR`; aim round 2 at the delta only; require a
`--features separate-bins` build before touching staged binaries; and take the snapshot commit before
briefing, because a scratch snapshot disappears and the commit is the restore canon.

The judging rules recorded with them: commission the soundness lens for all-sites enumeration,
disjointness, same-name collision, guard traversal completeness and the population path of every
consumed map; compare semantic equivalence; classify a probe divergence four ways (real gap,
no-oracle, vita-ahead, harness format); read two spellings of one hazard diverging as a walker that
cannot see one layer down; and remove a feature whose partial support only trades one silent-wrong for
another, after building the consumer x value matrix.

Review orientation: hand over the measurement table as a file and start with "attack outside this
table"; write the budget into the briefing; make round 2 a delta briefing; and snapshot the binary and
record its hash, because a moving tree makes the lenses score different builds (§4.5.363).

### A context rule inside a shared evaluator (§4.5.363)

Measured: putting "evaluate this node at its honest width" into `const_eval_in_scope`'s `Binary` arm
produced four defects at once, because consumers with opposite width rules share that function.
Declaration range bounds, dimensions and repetition counts are self-determined (IEEE §11.6.1), where it
is right. A parameter's value is an assignment (IEEE §11.6), where the RHS is evaluated at
`max(self, target)`: forcing self-determination made `localparam int Q = W[7:0] + 8'd240` answer 36
against both oracles' 292, and that cell had been honest-loud, so loud → silent-wrong. A guard inside
one arm is not a guard — the `Unary` arm walked straight past and `~W[3:0]` traded one silent-wrong for
another. And walking the subtree at every `Binary` node is quadratic: a select-free 8,000-term constant
expression became 121x slower. Moving the same predicate to `array_geom`'s declaration-range fold
removed all four and made it O(1) per bound.

Rule produced: grep who calls a shared evaluator and whether they agree on context; when two disagree,
the rule lives at the call site, and a self-determined consumer is the safe home.

### The order of two domains is the consumer's call (§4.5.364)

Measured: where both the integer and the real domain can answer, which to ask first is decided by what
the value is for. A TRUTH consumer (`const_truth_in_scope`, a generate condition) can ask integer first,
because truncation does not change zero from non-zero. A MAGNITUDE consumer (a delay, a width, a count)
must ask real first, and `param_real_value` had already written down why: an exactly-integral
`parameter real R = 11` is registered in both `real_param_val` and `params`, so an integer walk finds
the `i64` twin and folds `R/2` with integer division where both oracles give 5.5. §4.5.364 copied
`const_truth_in_scope`'s order and produced that silent-wrong.

Returning the real domain's decline is worse: it has no `%`, no bit operations, no shift, no `$clog2`
and no call arm, so the decline becomes the consumer's silent default and `#(RD % 4)`,
`#($clog2(RD))` and `#(half(RD))` — five cells — went correct → silent-wrong. Real first, integer
fallback. The proof that this is not a trade is structural: the `i64` twin exists only when the real is
exactly integral, so the two lanes cannot differ in VALUE, only in which operators exist, and every
operator missing from the real domain is integer-only.

A predicate may be lenient when it asks "should this expression be widened to the real domain" and must
be shadow-correct when it CHOOSES the domain. `expr_mentions_real`'s name arm walks `real_param_val`
alone, so an inner `localparam R = 9;` shadowing an outer `parameter real R = 5;` still answers real and
the real lane folds the outer 5 (both oracles 9). The repository had already written that trap down, in
`real_param_is_non_integral`'s comment. The fix is an opt-in parameter, with existing callers passing a
literal `false` for mechanical byte-identity, and the new resolver's direction proved: the combined
predicate is a superset, so the walk stops at the same place or further in, and the difference is
`true → false` only.

When one value has several fold lanes, the decline policy must be the same in all of them. The new
TimeLit lane declined with `?` where its two siblings saturate, and the consumer eats a decline as "no
delay" — so a discarded delay fires EARLIER than a clamped one, and `#(20000s)` fired immediately. Two
lenses found it independently.

An adversarial lens filed a BLOCKING saying "PRE is byte-identical to both oracles, so this is
correct → loud". Re-measured on a 1 ns grid instead of 10 ns, PRE was silently DISCARDING the delay
(PRE `t=11 bus=1`, iverilog `bus=z`): every sample had fallen outside `[t, t+D)`.

Rules produced: the order of two domains is decided by the consumer, and a decline must always fall
back; a predicate that chooses a domain must be shadow-correct, added opt-in; the decline policy must
be identical in every lane; and a probe's grid must be finer than the effect it measures, with a
structural reason written down when no counter-example can be built.

### A probe whose value fits the width (§4.5.365)

Measured: "the frame path is already correct" was confirmed by passing `3.0` to an `input int` formal
and written into the documentation. The value fits in 32 bits, so the answer is the same whether
narrowing happens or not; passing `f(300.0)` to an `input byte` showed all three routings wrong.

Of the four binding sites implementing one rule, the inline task was already correct, because it
copies the input actual into a formal-width local net and that store goes through `coerce_assign`.
Finding it changed the design question from "how do I write this rule" to "how do I bring the other
three here", and one shared helper was the answer.

`lower_real_to_int_cast` uses an exact rounding above 32 bits (`trunc + (frac >= 1/2 ? +/-1 : 0)`) with
its f64 tie-to-even reason in a comment, while the <=32-bit branch was `$rtoi(e +/- 0.5)`. For odd `|e|`
in `[2^52, 2^53)` the ulp is 1.0, so that addition is a tie and answers `e+1`. It had been hidden while
only `int'()`/`byte'()` reached it; routing every <=32-bit formal there would have made it a
wrong-for-wrong swap. Moving the exact branch fixed 40 cells.

And `range_to_dims` emits diagnostics; calling it a second time to obtain a sign printed the same W3056
twice on `input logic [3:-2]` and moved the error count from 2 to 3.

Rules produced: overflow the destination in a width probe; census by routing and ask whether one site
is already correct; check branch accuracy parity before filling a helper with new traffic; and bind the
first call's result rather than calling a diagnostic-emitting function twice.

## 2026-08-21

### The second dispatch (§4.5.354)

Measured: `lower_expr_ctx` re-branched the six context-propagation node kinds `lower_expr` already
handles. The original had four domain routes (string concat, string replicate, string compare, handle
gate) and the twin had none properly, so "there is a fill literal somewhere in this node" was a switch
that turned all four off. The deletion test is one line: does the arm pass a CONSTANT context to its
children? `Concat` and `Replicate` operands are self-determined, so the twin arm passed `ctx = 0` to
every operand, which is by definition what the original does — pure duplication, and duplication is
always the stale copy (`repl_zero_ok` unset gives a false loud; an unchecked negative count gives a
silent-wrong). Deleting the arms is shorter than adding four guards, fixes two more defects, and cannot
drift again. The one arm that genuinely does work got a guard copied verbatim, in the original site's
order, so a `diff` checks it rather than an inference.

The census table gained a "twin" column — "how does the sibling with `1'b1` instead of the fill
behave?" — and all six rows had a healthy sibling, which is the statement of the root: one routing
switch, not six bugs. Without that column the fix would have been six guards.

`A → B → A` in tail position does not grow the stack, so the symptom is 100% CPU with flat RSS, which
reads as an infinite loop rather than infinite recursion (only the debug build dies of stack
overflow). The termination argument was made structural with an ungated entry point, reducing the
invariant to one sentence: every departing call is either a proper sub-expression or the ungated
entry. The hand audit that preceded it ("`is_ctx_node` has an explicit arm for every kind it accepts,
so `_` cannot come back") became false inside the same slice, when the `Concat` and `Replicate` arms
were deleted and both kinds fell to `_`. A mutant written to be equivalent killed 8 tests and caught
it.

The item was queued with "no oracle, therefore unverifiable, therefore loud is the answer" nailed on,
and that was wrong: `{s,'1}` has no oracle and neither does `{s, 8'h0F}`, so oracle absence is a
pre-existing property of that path, not something this change creates. The question to ask is whether
the STEP the change adds can be pinned, and it could: half the rule (a concat operand is
self-determined, so a fill is one bit) is confirmed by both oracles on the string-free family, and the
other half is an untouched existing path pinned by two internal spellings.

Four outputs (iverilog, verilator, PRE, POST) all read `r=ab`; `od -c` showed all four were `ab \001`,
and the first test expectation had been written as `"ab"`. And where oracles split (`s <= 1'b1` is 1 on
iverilog and 0 on verilator), the test pins the PAIRWISE AGREEMENT of the fill spelling and the `1'b1`
twin and asserts no value, leaving the split as its own §2 item.

The first design (four guards) fixed all four census cells and passed the gate; only after the
soundness lens found two more (both PRE == POST, the same root) did the structure — "two arms are pure
duplication" — become visible.

`ir_lvalue_width` answers 64 for a real target, which §4.5.353 caught turning `force r = '1;` into
1.84467e+19; §4.5.354 found the identical trap one layer down in `ir_bits_of`, making `r + '1` compute
`2.5 + (2^64-1)` where both oracles give 4. A real has no bit width (IEEE §6.12); the answer such a
function gives is a STORAGE SIZE. Grepping `ir_bits_of` exhaustively found four more sites feeding a
fill context, two of them real (a ternary branch and a `case` selector). And `c ? '1 : r` is sized
exactly as wrongly as `c ? r : '1` while producing correct output, because that branch is not taken.

`lower_expr`'s IEEE §6.2 block is mostly "this operator is forbidden on real" diagnostics, with one
`**` to `$pow` desugar mixed in; the twin lacking the block lost the operator, not just the
diagnostics, and `r ** '1` answered 0 where both oracles give 3.

Across a 240-cell real axis, mismatches fell from 80 to 16 and exactly one cell (`'0 % r`) moved from
"matches the oracle" to loud. The same path answers `'1 % r` WRONGLY and the `1'b0 % r` twin was
already loud, so PRE's agreement was a broken path coincidentally right on one input.

Rules produced: a function that re-branches the same node kind is debt and deletion is usually the
answer; add a twin column to the census; make termination structural and mutate your own reasoning;
distinguish a pre-existing no-oracle condition from one your change creates; compare outputs as bytes;
pin the property, not the value, when oracles split; a design fixed before review is fixed too early;
grep a trap's other layers; probe the branch that is actually taken; extract a diagnostic block that
contains a route rather than copying it; and check a lone "regression" for accidental agreement.

### Late values (§4.5.355, §4.5.357)

Measured: a hierarchical target's fill became one bit not because the width does not exist but because
it does not exist YET at lowering time — and the architecture already solves that problem for the net
id, by lowering a sentinel and patching later. The width is simply a second late value.

Re-asking late must ask the SAME question. The two hierarchical write lanes contain whole-net,
part-select, element and bit-select forms, and bit-select is already correct (a one-bit target, all
three tools agreeing), so enumerating sub-cases risks breaking it. The fix wrote no new rule: both
lanes publish the chunk they decided, and the very same `ir_lvalue_width` that answered 1 at lowering
is asked again against that chunk. All four sub-cases fall out and bit-select is structurally protected
because its width is 1 either way.

ROADMAP recorded the root cause as one sentinel constant. Temporary instrumentation showed two lanes,
the second (`0xFE000000`, element and select) unrecorded, and inside it the bit-select case that must
not be changed.

Nested fills (`'1+1`, `{2{'1}}`, `c?'1:12'h0`, `~'0`) were probed at a 12-bit target, all matched the
oracle, and the conclusion "bare fill is the whole broken set" was written down. The rule is
`max(ctx, sibling)`, so the context is the only source only when the sibling is NARROWER than the
target; at 12 bits the two paths coincide. A 520-cell sweep with a 64-bit target gave
`u.wide = '1+1` = 4294967296 where the local answers 0.

§4.5.355 recorded the residue's constraint as "only a bare fill can be deferred", which was true and
was a statement of the RESULT. The real constraint is "the resolve pass has no lowering scope", and
stated positively that becomes "an expression whose lowering asks the scope nothing" — much wider, and
20 of 24 cells fell inside it.

The first action of §4.5.357 was not to widen but to ask whether the deferral is needed at all — if
`hier_lookup` succeeded at lowering time the constraint disappears. Instrumentation printed `None` in
every case, and only then was the workaround built.

Copying a newly lowered sub-expression's root into an old arena slot lost the output of 48 cells, and
the panic explained why: the expression arena is post-order, so a parent's id exceeds every child's.
That also explains why the previous slice's version worked — a bare fill is a single `Const` with no
children.

§4.5.360 then measured §4.5.357's own recorded prerequisite ("a snapshot of the lowering scope usable at
resolve time") and found the scope to be a single string; everything else is a fully-qualified table
alive for the whole elaboration.

Rules produced: look for an existing late-value mechanism; re-ask the same question rather than
enumerating sub-cases; a recorded root cause is a hypothesis; vary the probe's size across the rule's
axes; state a constraint by what blocks it; rule out the straighter path by instrumentation first; say
why a previous approach worked before generalising it; and re-measure your own recorded prerequisite in
the next slice.

### Half a rule has direction (§4.5.356)

Measured: IEEE §12.5 sizes a case expression and ALL items to a common max. vita had only "push the
selector's width to the labels", which is the whole rule only when the selector has a width, and a bare
fill selector has none. The first fix ("size the selector to the labels' max") passed the census and one
cell caught it: `case ('1) 8'h01: ; '1: ;` selects the `'1` arm in both oracles, and widening only the
selector left the label `'1` at its old one bit, so it fell to `default` — a right answer replaced by a
different wrong answer.

After fixing selector fills, 15 of 420 cells remained, and every one had no fill in the selector: the
common max holds between labels as well. Widening the gate from "a fill in the selector" to "a fill
anywhere" gave 420/420.

Rules produced: ask whether B has something to give A when fixing "A gives B its width"; and classify
the residue after a first fix rather than assuming it is pre-existing.

### Finding every site that asks the same question (§4.5.358, §4.5.359)

Measured: fixing the context width of a `real` assignment in the engine had three sites — the
scheduler's `eval_for_lvalue`, native's `k_eval_for_lvalue`, and the native program compiler. With the
first two fixed the symptom changed shape rather than disappearing: `real r; r = -b;` was correct in a
process containing `#1` and wrong in one without, because a delay-free process is compiled to a native
program. The repository's tests habitually write `#1 $display(…)`, so following that habit alone would
have passed a half-fixed engine.

`range_to_dims` had 28 call sites. Attaching `#[track_caller]` and a temporary caller print and running
the census designs identified exactly 5. Reading and choosing would have missed one of a static/automatic
pair that looks identical from outside — the census turned three into six because of it.

vita has two collectors for subprogram local variables (framed = automatic, inline = static) and three
counting block-locals; a census run only over `function automatic` sees half. One of the six sites in
that slice was a static task local.

Attaching six sites can be done by copying 20 lines six times or by extracting once and calling six
times; the second makes an un-called seventh site visible to a grep. Extracting `func_return_dims` also
had to stay opt-in, because it has three callers and only one has somewhere to record.

Rules produced: make a predicate a named shared function so a grep counts the sites; parameterise the
axis that selects an execution path (delay versus no delay) in tests; instrument rather than read when
call sites exceed ten; count the collectors of a concept, taking language keywords that split
implementation paths as census axes; and extract once rather than copying N times.

### Ask the tool the same question without a destination (§4.5.361, §4.5.362)

Measured: four items were parked as "no two-oracle consensus, cannot start". Changing the question from
"which answer is right" to "which tool is not an oracle here" closed three immediately, two of them
without looking at vita at all. iverilog answers `s<"ab"`, `s<"aa"` and `s<"zz"` all as 1 for
`s="ab"` — three answers that cannot be simultaneously true in a comparison with no width, sign or
conversion, so that 1 carries no information. And it reads `$itor(64'h1_0000_0008)` as 8 in both
`longint unsigned` and signed `longint`, so the divergence is container truncation and not a sign axis.
The disqualifying evidence comes from the NEIGHBOURING cell, not the divergent one. Leniency is not
consensus either: `%` on a real runs in both tools and gives different answers (fmod 1.5 against a
rounded 0), which is the signature of an undefined corner, and a loud reject of illegal code is the top
of the ladder rather than a §3 gap.

Of the four, one (`'1 ** r`) looked like it had something to fix, on the strength of vita contradicting
itself: `a = '1 ** r;` gives 480 and `a = (4'd15+4'd1) ** r;` gives 0, same all-ones base, in one
design. The inconsistency is real, the fix built cleanly, and a 288-cell PRE three-way measurement gave
iverilog agreement 267 → 247 with all 35 moved cells moving AWAY. It was reverted. Measuring the other
side then inverted the diagnosis: `(4'd15+4'd1) ** r` is 1024 in iverilog (16^2.5), so the assignment
context does reach `**`'s base, the defect is that the non-fill path gives too little rather than that
the fill path gives too much, and the 480 that was to be fixed was already right.

§4.5.361 wrote the "which tool is not an oracle here" rule into this file and, in the same session, did
not apply it — it scored candidate fixes by iverilog agreement, and what that score was measuring was
that tool's bug. The disqualifying evidence was one question away: send the same expression somewhere
with no assignment width. `logic [15:0] a = ('1+4'h0) ** r;` is 480 in iverilog and
`real x = ('1+4'h0) ** r;` is 871.42 in all three tools, as is `$pow(('1+4'h0), r)`. A base's value
cannot depend on the width of the variable that will later hold the result. Three places to send an
expression without a destination are almost always available: a `real` destination (no bit width
exists), the system-function spelling the LRM designates as the operator's definition (a function
argument is self-determined, so operator form == function form is a tool-independent gate — this
slice's scoreboard went from 115/192 to 192/192), and the neighbouring operators (the same four bases
give 4, 18, 2, 18 under `+ r` and 480 four times under `** r`).

Changing `**`'s base to self-determined was written as `Pow && fill(lhs) && !fill(rhs)`, copied from the
arithmetic arm below. That arm's `!rf` asks which side supplies the width; the new site asks about the
exponent's DOMAIN, and a fill cannot be real while an expression CONTAINING a fill can (`r + '1`), so
`('1+4'h0) ** (r + '1)` was left unfixed (64976 against `$pow`'s 13071).

`lower_expr_ctx` announces an unresolved name by EMITTING a diagnostic, so "try it and undo it if it
fails" is impossible, and turning a wrong value into a false loud is a regression regardless of the
value being wrong.

Rules produced: ask the tool the same question without a destination; score against another spelling of
the same tool, not against the tool; internal inconsistency proves a defect and not a direction, and the
direction is decided by a PRE three-way measurement; check that a copied guard answers the same
question; and put a side-effect-free predicate in front of any side-effecting conditional operation,
calling the same resolver the operation uses.

### "Not calling it" is a target on the correctness axis too (§4.5.350, §4.5.353)

Measured: §4.5.351 and §4.5.352 found "a fast path was built and is not called" on the performance axis
(−3.9% and −10.5%). §4.5.353 found the same shape on the correctness axis: `resize_fill_rhs` implements
IEEE §5.7.1 exactly and was already called from three places, and three others did not call it. Zero
new machinery; the fix is the same one-line call three times.

The queue records a symptom and a census records the class, twice in a row: §4.5.350's queue line said
2 negative-constant consumers and the census counted 3; §4.5.353's said 1 fill context and the census
counted 3, with `force` and a procedural continuous assign found by a write-twin sweep.

`wire [7:0] a = '1;` is an implicit continuous assign, so it must answer the same as
`assign a = '1;`. Put in one file:

```systemverilog
wire [7:0] a = '1;             // 00000001  wrong
wire [7:0] c; assign c = '1;   // 11111111  right
reg  [7:0] b = '1;             // 11111111  right
```

Fixing context-determination requires pinning self-determination: eight cells measured that a fill in a
shift count, a `**` exponent, an `&&`/`||` operand and a bit index stays one bit. Without them there is
no way to tell whether `8'd1 << '1` became `1 << 255` = 0.

`wire [7:0] a = 'x;` is `xxxxxxxx` on iverilog and `00000000` on verilator, which is verilator being
2-state rather than an oracle split; IEEE §5.7.1 is explicit, so iverilog plus the LRM is canonical. The
reverse case — `'{m: '1}` accepted by verilator and rejected by iverilog — is a real split and off
limits.

The draft added three calls to `resize_fill_rhs` and made a `real` target correct → silent-wrong
(`force r = '1;` from 1.0 to 1.84467e+19). Guarding at the three call sites would have fixed only the
new regression; putting the guard INSIDE the helper fixed four pre-existing instances through the
earlier callers as well.

A short-circuit-removal mutant was recorded as "expected to SURVIVE — it is only an IR or performance
guard" and came back KILLED, because `lower_expr_ctx`'s `Replicate` arm lowers the count through
`lower_index_expr` while `lower_expr`'s arm uses the `$clog2` fold path: the short circuit was a VALUE
guard, and a new guard was nearly placed in front of it.

`resize_fill_rhs` opens with `if !expr_contains_fill(rhs) { return rhs_id; }`, and that walk has no
`MinTypMax` arm, so `(1:'1:2)` stayed unfixed at the fixed site while `lower_expr` lowers it
transparently.

Rules produced: ask whether every function implementing a rule is called from every site that should
call it; start a queue item with a census from the code; put two spellings of one meaning side by side
in one file; pin the opposite half of a rule; read a 2-state oracle's 0 correctly; feed the input to the
existing callers before adding an Nth; write the mutant's expected result first; and check an
early-return predicate's arm set against the lowering's.

### Performance instrumentation (§4.5.351, §4.5.352)

Measured: asking "is the worst case of this change zero?" with wall time gave +0.24% and +0.59% on 25
rounds of one design, with min and median disagreeing in sign. Retired instructions
(`/usr/bin/time -l` on arm64, reproducible to +/-0.02%) settled it at once: a harmless control at
+0.007% against the change at +0.360%, and the linearity (+3 instructions per delayed iteration, −3.3
per skipped one) gave a break-even point.

Adding a field to a struct shifts every later field's offset and changes the text section. A fourth
binary built from PRE source with the field added and the loop left alone — identical executed
instructions — moved picorv32 +0.9% and a continuous-assign-heavy design −5%.

`sample <pid> N` has the sampling WINDOW as its denominator, not the run's wall time. When a process
outlives the window the denominators match; when it ends inside it they do not. A 10 s window with a
~10.3 s POST run gave 7482 against 6879 samples, −8%, so every unchanged function looked larger; a 6 s
window gave 4474 against 4499. And even with matched denominators, share is not absolute time: a 10%
overall improvement raises an unchanged function's share by 1/0.896 = 1.116x, which is exactly what
`exec_vm::run` did (12.96% to 14.60%, ratio 1.127).

Doing the division on a recommended candidate led to profiling its function and finding something six
times larger inside it: the recommendation was real at −2.4% and the division's find was −8.4%.

```rust
for ci in 0..self.st.ir.cont_assigns.len() {
    let Some(d) = self.st.ir.cont_assigns[ci].delay else { continue };
```

reads as "process the delayed ones" and touches everything: picorv32 has zero delayed continuous
assigns and ran that loop 278,601,194 times, 7.2% of the runtime.

`settle_cont_assigns` reads as per-timestep and is per-delta — 1,400,006 calls over 200k cycles, seven
per cycle — so an O(design) scan inside it is O(design x delta). The instrumented count and the profile
agreed independently (7.24% and 7.3%).

`k_write_scalar`, a flat-store path for plain scalar writes, already existed and documented its own
reason for existing; `apply_nba` never called it, and that path carries 24 times the traffic in the same
design. picorv32 −3.9%. The variant of the same shape is a proof built and discarded:
`Op::ScheduleNbaScalar` is gated on `plain_scalar_dest`, so the classification was known at schedule
time, and `NbaUpdate` carried only the value and the offset.

A `wprog` rejection census said 47.6% of rejections were one gate and that was read as the target size;
19% was actually rescued (the first failing gate is not the only failing gate) and 19% converts to 0.8%
of runtime, below noise. The division is three minutes: calls to move x cost difference per path
divided by total runtime.

Three harness rules were paid for in the same slice: the product and oracle configurations share
`target/debug/vita`, so a "Finished in 0.03s" after a configuration change means the previous binary is
still there (half an hour spent on that); `crates/cli/tests/` has 12 hard-coded `/tmp` paths, so two
overlapping full-suite runs write the same files; and machine state drifts within a session, so A and B
must be measured back to back (the same POST moved between 2.29 and 2.34).

The differential lens was asked for an explicit non-vacuity proof and built an instrumented binary
showing the fast arm firing in 22 of 27 designs, with every recorded firing at offsets `(0,0)` and the
`else` seeing `(0,17)`-style values.

Rules produced: measure retired instructions below 1%; build a harmless control binary; check the
denominator and convert share to share x wall; do the division and let it make you open the function;
ask whether a `continue`-only loop's condition can be asked once; multiply a scan's unit by its call
frequency; keep the original verdict verbatim behind a pre-filter; census the regions that have not been
asked the question; ask first whether every existing fast path is called; and require a non-vacuity
proof.

## 2026-08-20

### Negative constants and their consumers (§4.5.350)

Measured: asking a width-unlimited fold whether a constant is NEGATIVE false-rejects correct designs.
`{(4'd0-4'd1){1'b1}}` is 15 copies (255) in both oracles and `{(4'sd0-4'sd1){1'b1}}` is a refusal; the
unlimited domain answers −1 for both, because the two spellings share a bit pattern and differ only in
width and sign. The sign question belongs on a self-determined walk with the same admission as the
bound walk, and the two boundary cells (unsigned wrap, signed wrap) are pinned so that a later
"simplification" back to the unlimited fold breaks the test first.

A replication count folds to a `Const` when it is a parameter and stays an `Add`/`Sub` for
`{(2-3){…}}` or `{(W-4){…}}`; a `u32` fold on the latter saturates to 0, so the zero check says "count
of zero" about −1 — loud, with a false message. Both readings are needed, and both must be
width-correct and sign-correct.

`declared_neg_lsb` carried a comment calling `msb<0, lsb>=0` a degenerate parameter underflow, and the
code clamped the width to 1 and warned "param value 0?". Both oracles give two bits, and the same
branch also receives a parameter-free literal `logic [-1:0]` — the diagnostic was naming a phenomenon
that does not exist. Direction decides which bound is internal bit 0; which bound is negative does not.

The same code silently clamped with `.min(MAX_NET_WIDTH)` where the ordinary path raises "exceeds the
v1 cap" for the same condition.

Two lenses produced the same `debug_assert` panic, one calling it pre-existing and the session having
recorded it as a new descent. The difference was reading PRE's output through `head -1`: the panic was
on the line after the warning.

`allow_neg_lsb`'s comment contracts it to turn on the width and the side-map record together; the
enabling site and the recording site are 550 lines apart in one function with five early `continue`s
between them, and dyn and queue element nets take one of those — receiving the wide width and no
record, where the old clamp at least warned.

Rules produced: ask about sign at the self-determined width and pin both boundary cells; read a value
from both representations where one leaks half a family; re-measure an assumption written as a
degenerate special case; match the neighbouring branch's treatment of a cap; re-measure the attribution
yourself and never truncate PRE output; and write an opt-in predicate as reachability to the recording
site.

### The cap you delete: capability limit or domain guard (§4.5.345)

Measured: replacing a hand-written concat fold with the shared folder relaxed that loop's `total > 63`
cap to `w > 64`, on the reading that an `i64` holds 64 bits so the cap was narrowing a capability. It
was a DOMAIN GUARD: the `i64` constant domain holds 63 unsigned bits, `coerce_int_width(v, 64, false)`
is the identity, and a 64-bit pattern with the top bit set leaks out as a negative `i64`. The result is
`> 0` false, a `generate if` elaborating a different hierarchy, and a `case` item missing — all at exit
0, all loud in PRE, so loud → silent-wrong. The right rejection is by FITNESS ("does this value fit the
domain"), not by width, which keeps the real gain for 64-bit patterns with a clear top bit. An 89-cell
sweep contained no cell that produced a >=64-bit pattern.

Also measured in the same slice: the §2 item recorded the SITE and the REASON
("`eval_const_assign`'s `w.max(tw)` uses `tw=0` as a real width") and its symptom reproduced exactly
(9 against 169). Fixing it as written — degrading width-0 to the unlimited domain — broke a different
cell from 16 to 10000, correct → silent-wrong. `max(self, 0)` (the RHS's own width) is the correct
degrade, right in every cell where the true target is no wider than the RHS, and the real defect was
declining a width that is computable (the product of the packed dimensions). Only a PRE three-way
measurement separates the two readings; an iverilog-only differential reads both as "vita is wrong".

And a silent default such as `unwrap_or(0)` is CORRECT wherever the true value is 0, so turning it loud
is correct → loud for that subset. The adversarial review built the cell (`{2{2'b00}}`); the sweep's
chosen true values were never 0, so the table showed only wrong → LOUD. The answer is to close upward —
route concat, replication and size casts to the existing carry-free folder — rather than to revert.

Rules produced: decide whether a cap is a capability limit or a domain guard and reject on fitness;
build one cell each side of a domain boundary; a pre-existing defect elsewhere is not a licence to widen
it; re-measure the queue's mechanism and ask which cells are correct today; and include cells whose true
value equals the silent default.

### The mutation battery's cost is linking (§4.5.345, and the 2026-08-15 correction)

Measured: a full-workspace pass is about 8 minutes of rustc relinking plus about 30 seconds of actual
testing, because `cli` has 376 integration-test binaries against `sim-engine`'s 32 and one source line
relinks all 442. On macOS the first execution of about 600 new binaries is verified by the OS, so test
enumeration alone costs 15 minutes and 14 mutants is four hours. Where that is prohibitive, the sound
procedure is narrow-first and confirm survivors at `--workspace`, because a narrow filter can only make
a false SURVIVED and never a false KILLED.

The item's original conclusion ("scope to `-p sim-engine`, the tier-3 killers all live there") was
false and cost a whole battery pass: tier-3 slices plant their teeth as absolute anchors and those live
in `crates/cli/tests/*.rs`, so one battery reported all 7 cases SURVIVED. The obvious repair is the
second trap: `-p A -p B --test X` applies `--test` to every selected package and drops A's unit tests
entirely — 58 tests ran instead of 5,457.

### Extracting a block that shared a computation (§4.5.346)

Measured: two whole-node special cases were extracted from `const_eval_in_scope`'s `Binary` arm. In the
original, `let a = const_eval_in_scope(lhs)?` was one computation shared by the special cases and the
general path; the helper copied it, so every binary node folded its left operand twice — 2^depth. A
200-deep left-leaning index went from milliseconds to over four minutes and a suite timeout. Two lenses
and a 147-cell sweep all passed and no value was wrong anywhere; the gate caught it. nextest reports it
as `SLOW` then `TERMINATING` then `TIMEOUT`, and the summary line reads "5,631 passed, 1 timed out", so
grepping only for `FAIL` reads green.

Rule produced: when extracting a block into a helper, ask whether it read a local, whether that local
remains at the call site, and whether the node kind is a link in a recursive chain — if all three,
return early inside the helper.

### Consuming a static type claim (§4.5.349)

Measured: a shared rule answers "if either arm is real, this ternary is real", and the engine's ternary
did not convert the taken integer arm. The mismatch was harmless while nobody consumed the claim. When
this slice began consuming it in a binary operation it appeared — and the way it appeared is the trap:
in PRE both operands received the same wrong context and the errors cancelled, so the answer was right.
Fixing one side broke the cancellation: correct → silent-wrong. The prescription is to make the value
match the claim (make the ternary actually produce a real), which also closes the pre-existing defect
that was leaning on the claim.

Rule produced: when you start reading a static predicate in a new place, check that the nodes it calls
true really produce that type at runtime, and look for places where two errors were cancelling.

### A harness that truncates the value misdiagnoses the mechanism (§2 mixed-real grounding)

Measured: pinning a widening width by printing a `real` result through `$rtoi()` truncated to 32 bits,
so the 32-bit operand cell coincidentally matched the oracle and the conclusion "the widening is exactly
32" was written down. Re-measured with `%f`, it is 64 — which agrees with the real constant's self width
as recorded in the code.

Rule produced: a width probe's observation path must preserve width (`%f` for real, `%h`/`%0d` at the
target width), and when a probe's conclusion disagrees with a constant read from the code, suspect the
harness first — that is the "harness format" branch of the four-way classification.

## 2026-08-19

### Two paths claimed semantically identical (§3.11)

Measured: the plan to send `function automatic` to the inliner instead of a frame rested on "a
non-recursive automatic function is semantically identical when inlined, because SSA folding gives each
call fresh locals, which is what automatic means". The whole suite failed in 15 places, and the
sharpest was a `$random` argument drawn twice — for a reason already written in the failing test's own
doc: inline expansion names the operand a second time (`Select{Bit, base: e}`).

The straighter path existed: the goal was codegen coverage, and opening the codegen-side refusal
(`is_codegen_able`'s `Terminator::Call`) raises the same metric while keeping the frame path and
touching the ladder not at all.

Rule produced: read the names of the tests that protect a path before routing traffic into it —
`…is_a_documented_gap`, `…matches_bare`, `…never_evaluates_an_impure_operand_twice` are a list of that
path's defects, and re-routing inherits them; the suite counts the "same answer" claim, once before and
once after.

## 2026-08-18

### A refusal is only as loud as its caller (§4.5.339)

Measured: "if the width is unknown, refuse rather than fold" sounds like correct-or-loud, and where a
consumer eats the decline as a silent default (a range bound, a replication count, a part-select width)
the refusal makes no sound. Refusing a width-unknown exponent built `logic [f():0]` as a one-bit net at
exit 0 with zero diagnostics where PRE and iverilog both gave 10 bits — correct → silent-wrong. And the
right answer is usually to make the unknown knowable: answering width and sign for the second spelling
of the same syntax (`RPS'(e)` beside `4'(e)`) folded cells that had been loud into correct answers.

### Delegation that resets depth is the source of cycles (§4.5.339)

Measured: a recursion budget should be charged only where a cycle exists. Arguments are a finite AST
descent (`g(g(g(0)))` is three distinct nodes) and charging them halves the legal nesting: 65 levels of
argument nesting went correct → LOUD. The cycle lives where a position returns to the callee's own
declaration node, such as a default argument. Where the delegate restarts depth at 0 it must be gated,
but gating on "does it contain a call" alone makes normal module-scope cells loud, so the safe
condition is a list of disjoint reasons (`depth == 0` OR call-free). And "depth 0 means no call is
live" is usually false: a body-local initializer and a plain `Call` arm both carry the caller's depth,
and a probe fired in three existing suite tests.

### The external aes_top report, third round (§4.5.341)

Measured: an outside report asked, precisely, for a WARNING on `"\r"` rather than a fix, because the
bug was theirs and already fixed. Measuring the whole escape axis against iverilog and verilator instead
showed that all five escapes IEEE Table 5-1 defines (`\ddd`, `\xhh`, `\v`, `\f`, `\a`) were silently
wrong in both value and width — and that two of those five were the workarounds the reporter suggested
(`"\015"`, `"\x0D"`), so following the request exactly would have produced a warning recommending
solutions that do not work.

`Diagnostic::context: Vec<Frame>` — the hierarchy and instance path — had, since the crate was created,
0 of 35 creation sites filling it and 0 arms in the renderer, while an outside report complained that
four copies of one warning from four different instances are indistinguishable. The field was designed
as the answer to that question. In the same run, 33 of 35 sites left `location` as `None` (measured:
0 of picorv32's 71 diagnostics carry a location).

A differential harness reported zero divergences three times in one sweep, all harness faults: BSD
`join` needs `-o` before the files, and with the arguments in the wrong order it prints usage, compares
zero rows, and reads as zero divergences unless the exit code is checked; probing with `64'("\r")`
measures the cast rather than the escape, and iverilog answers 0 for that cast so every row matched; and
a regex delimiter that collides with the data (a backtick-quoted message matched with a backtick-delimited
capture) drops the cell whose escape IS a backtick and manufactures "this cell has no warning".

`count(&err, "R")` was counting three `$error("R")` calls and getting 3 by coincidence, one R per line;
when the renderer began printing the mnemonic `E-RUN-USER-ERROR` it became 18.

Adding `elab_s`/`sim_s` to `run.json` turned `two_runs_byte_identical_bar_wallclock` red immediately.
There are exactly two ways to make it pass and both are explicit declarations: add the field to the
isolated list, or make it deterministic. That gate is the device that forces a new field's property into
the code, and an existence assertion beside the exclusion keeps the exclusion from going vacuous.

`git checkout -- .` destroyed uncommitted work for the second time — this time not in the battery itself
but in the loop that checked whether the substitution patterns matched: each case substituted and then
reverted with `git checkout -q -- .`, which returned 22 of the 32 modified files to HEAD, because the
snapshot held only the 10 mutation targets. The only symptom was `git diff --stat` falling from 32 files
to 10, and the build still worked. Four reinforcements followed: restore with a `cp` script naming
explicit paths (reconstructing a name-to-path mapping by substituting `_` for `/` breaks on names such
as `vita-log`); snapshot everything `git status` lists as modified; run the restore loop under bash,
because zsh does not word-split an unquoted variable and a restore that silently copies nothing let
seven mutations stack, making every kill after the first unattributable and costing the round; and
install the restore as a `trap restore EXIT` inside the script, because an outer timeout SIGTERMed the
suite before a trailing `restore` in the compound command could run (exit 143) and left the tree
mutated. SIGKILL defeats a trap, so the isolation guard that diffs against the snapshot before each case
remains necessary.

Rules produced: measure the whole axis when a report names one cell; census a model field's consumers;
suspect the harness first on zero divergences and assert row counts plus a planted divergence; make a
diagnostic needle a discriminating fragment of the rendered form; answer a determinism golden with an
explicit declaration; and the five battery-restore rules above.

### Document structure (measured during the ROADMAP restructure)

Measured: `REMAINING_WORK §A` said "the next thing to do is ROADMAP §5.2" and `§B` said "canonical
order = ROADMAP §1". §1 had been stale for 15 days — its NEXT queue's entry 0 was a slice completed on
2026-08-03, and the owner instruction attached to it had been overturned by the end of Phase D. A
session resuming from the front of the file restarts finished work.

Of ROADMAP's 3,931 lines, 3,074 (78%) were the completed Phase A-D narrative and only 14% was open
work, so a section number said nothing about queue depth. The migration was verified twice: heading sets
(original is a subset of new plus archive, with differences only where a heading was deliberately
renamed) and LINE sets (whitespace-stripped, sorted, uniqued). The second check found the real loss —
a §1 NEXT list treated as a pointer and deleted held four items that appear nowhere in §2's body,
including the deep item's prerequisite description. Section numbers were preserved, because code
comments and commit messages reference `ROADMAP §5.1-<x>`.

Rules produced: one canonical answer per question, and where two sections are needed, split the QUESTION
and have each name the other; verify a migration by line sets as well as heading sets, using `comm -23`
in front of any edit that looks like a deletion.

## 2026-08-17

### Reading a refusal comment's qualifiers (§5.1-aw)

Measured: the reason tier-3 does not use tier-2's expression VM was recorded as "emitting natives there
would swap the faster path for the slower one on every RHS both accept". The sentence is true and was
confirmed (unconditionally enabling it moved an expression benchmark from 157 ms to 478 ms). The
qualifier narrows it to one case; for an RHS only ONE side accepts — here a >64-bit expression `wprog`
refuses on width — it says nothing, and there the product backend was 1.70x and 2.62x slower than the
retired one.

Rule produced: when a "must not" comment carries a conditional clause, count the cases where the
condition is false. This is harder to see than a reason that has expired, because reading the comment
finds nothing wrong with it.

### Wiring a losing experiment (§5.1-be)

Measured: the `jit` feature had been off by default for a long time and (a) did not compile, since two
fused ops were added, and (b) omitted the per-statement `k_call_fatal` check entirely, so a runaway
design ended `Quiescent` instead of `Error`. Neither had ever been asked, because the suite had never
been run through that module.

Rules produced: being behind a feature flag is not an exemption from verification, and correctness has
to be wired up and asked even when the performance answer is known in advance. A coverage instrument
that answers for only one executor (`VITA_JIT_STATS` in the engine path) makes the experiment look dead
in the default backend.

### Profiles, extraction and redundancy (§5.1-ba, §5.1-bc, §5.1-az)

Measured: `dispatch_body` at 6-12% was an inlined `vm_exec` op loop, and the real target inside it was a
4.5% `memset` from a 1,280-byte per-call scratch buffer. A line documented as "an allocation choice, not
semantics", justified by "tier-3 sends nothing here", was made false by a later routing change.

Buffer-reuse safety was measured with a contamination probe — fill the borrowed buffer with garbage
before every call and run the whole suite — which turns "nobody reads a slot this call did not write"
into a measured fact.

Extracting two shared lines into a function cost +12.5% on an eval-heavy benchmark for two consecutive
runs (+5% memory). The discriminator was a built-in control group: the affected shapes never call the
new callee, so what moved there can only be layout, and `#[inline(always)]` collapsed both to 0.4% and
0.2%.

A surviving mutation on a force check meant neither equivalence nor a blind axis: the callee already has
that gate and the twin path never had the check, so the duplicate was a second spelling of "where is
this answered" and was deleted. Survival has three classes, not two.

The engine's leaf fast path said "Locked by `leaf_fast_path_matches_read_net`", and that test has never
existed; the path had never been locked and the next slice was about to place a second copy beside it.
A word-only comparison lock is also structurally blind to a type stamp — reading a real net through the
integer fast path keeps the bits and loses `is_real`.

Rules produced: open the call graph under a profile line; re-read "harmless choice" notes on newly hot
paths; measure reuse with a contamination probe; verify inlining after extraction and use the control
group; classify survivors three ways; and grep the name of any test a doc cites as a lock.

### Cheap checks and approximations (§5.1-ax, §5.1-ay)

Measured: a Bloom filter is safe because "absent" is certain and "present" re-checks the real structure.
One routing approximation was placed at a boundary with nothing behind it, so a misrouted expression
reached no evaluator at all; the 2-state lane has the same shape but its fallback is the canonical
implementation, so a wrong answer costs a re-run rather than a result. A re-run fallback only holds
without side effects — the fast path emitting an out-of-range diagnostic before bailing makes the
canonical path emit it twice, so the bail goes BEFORE the effect and the thing to count in a fallback
design is the side-effect sites.

The same approximation was documented as "only when `wprog` would refuse" and in fact asked only the
first line of that admission, the width. `compile` also refuses by NODE KIND, and a census counted 75
such cases in <=64-bit contexts (a runtime-offset part-select being the representative), all of which
reached no evaluator and fell to the general path; asking the real judgement made that shape 1.48x
faster.

Rules produced: answer "what catches this if it is wrong?" before adding an approximation; ask the real
admission predicate by extracting and calling it, never by re-spelling it (two copies drift and the only
symptom is a slow path, which no test sees); frame two implementations as "where do they split" rather
than "which do we use"; and write the option as an enum, not a `bool`.

### A user-facing capability list rots every slice (§5.1-au)

Measured: `vita --help`'s `--backend` paragraph was false in every clause — "'vm' (default)" was
overturned when the default moved, and the list of things native cannot do (no fork, no `final`, no
class, no string net, no `$monitor`/`$strobe`, functions only) had all been closed by Phase A. The
speed multipliers were stale too. No slice deliberately left it; it was simply never on anyone's list.

Rules produced: write the ROLE ("this is a debug knob; X is the default and runs everything; the rest
are for bisection"), which stays true as features open; update a pin by strengthening it (the new pin
checks equivalence, which backend is the default, that the rest are debug knobs, and that closed
limitations are not still listed); and strengthen the pin in the same slice that changes a user-visible
sentence, because `--help` is the surface tests reach least.

### The reference implementation is permanently out of performance scope (§5.1-au, C1)

Recorded: making the reference fast is how it becomes unreadable, since every specialisation is a second
spelling of the rule and that is this repository's defect class (an earlier slice found the VM had
diverged from the interpreter in four separate ways). When the profile points at `exec::run_process`,
the answer is that the design should not be running there. Measured for scale only: picorv32 interp
1.319 s against native 0.513 s. "Not a product surface" is a statement about the `--backend` flag, not
about the function — in an oracle build the VM falls back to the interpreter for every body
`is_codegen_able` refuses, and tier-3 delegates frame bodies there.

## 2026-08-16

### A green build does not mean a feature works (§5.1-as, §5.1-at)

Measured: narrowing the product surface with an `oracle` feature (default on) produced the same illusion
three times, all with the symptom "the build is green and nothing is being tested". Referencing a
dependency crate without `default-features = false` nullifies the top-level `--no-default-features`
(the dependency's own defaults come back), which is what `cli → sim-engine` did: the build succeeded and
the feature did nothing. `--lib` is required, because an integration-test target revives the feature
through a dev-dependency. And both configurations share `target/debug/vita`, so a stale binary reported
that a flag which should have been refused was accepted.

Rule produced: verify with `cargo tree -p <crate> --no-default-features -e features`, and make that axis
a separate CI job, because enabling the feature anywhere in the workspace enables it everywhere.

### Deleting a fallback deletes the verdict's consumer (§5.1-at)

Measured: moving the fallback arm behind a `#[cfg]` removed the only place that consumed the gate's
verdict in that build, and `simulate` simply ran the refused design — measured with a forced-refusal
probe: the design runs, exit 0, zero diagnostics. The gate said "out of scope" and nobody listened. And
latching a fatal is not skipping execution: `fatal_run` set `had_fatal`/`finished` and execution
continued to a panicking `expect`.

Rules produced: count the values whose only consumer was the branch you are removing; and read the
ladder per build — making a refusal `exit != 0` is a descent where a fallback exists, because a fallback
is a slow answer and not a wrong one, and it is a promotion only in a build where the fallback is not
compiled.

### The gate's exit code (§5.1-aq)

Measured: `cargo clippy … 2>&1 | tail -3; echo "CLIPPY=$?"` printed `CLIPPY=0` over
`error: could not compile ... (test "backend_equiv")`. The error lines were visible in the output and
the verdict was 0.

Rule produced: redirect a gate to a file and capture the code separately; where a pipe is unavoidable,
read `${pipestatus[1]}` or `${PIPESTATUS[0]}`. A verdict path narrower than the failure modes makes
green meaningless — the same class as a result parser that could not see `TIMEOUT`.

### The battery's killer detection (§5.1-ao)

Measured: a mutation setting `wait fork`'s `resume_bb` to 0 was reported SURVIVED. Run by hand, the
design becomes an infinite loop (block 0 resume re-runs the fork; measured output kept growing to
t=2672). The runner counted only lines starting `FAIL`, and nextest reports it as `TIMEOUT`. An infinite
loop is also not a cheap kill — `terminate-after 4x60 s` catches it and the child keeps writing to the
pipe for those four minutes.

### Emptying the harness with the coverage axis (§5.1-ap)

Measured: at the end of Phase A all three gate layers were empty — `gate_refused!` macro sites 17 to 0,
the system-task refusal set 6 to 4 to 2 to 0, the executor refusal table 4-5-4-3-1-0, and reachable
design rows 0.

Rules produced: keep an empty table with an `is_empty()` assertion and a note on why it is empty, so
the next slice that adds a row has to build a design or say why it cannot; invert a test whose subject
disappeared (six of seven refusal pins were inverted, keeping the design and flipping the expectation,
because that source produced the last non-empty case) and retire only after moving the teeth (the one
retirement moved them to a source scan that fails if the macro reappears); and delete a caller-less
macro but leave the reason, because a reader believing a gate still has teeth is more expensive than
the deletion.

### Plumbing for delegation exposing another feature's defect (§5.1-ap)

Measured: `disable fork` needed `cur_aid` as the root of its kill set. Putting it into tier-3 dispatch
exposed a defect in an unrelated feature: IEEE §16.4 deferred reports are keyed on
`(marker_sid, cur_aid, cur_gen)` and tier-3 had never set that pair, so every report filed under
activity 0 — two processes reaching the same `assert #0` silently lose one `$error`, at exit 0. The
defect had existed since an earlier Phase A slice. The axis is shared code, so a differential cannot see
it; an absolute anchor caught it, and that anchor did not exist on the engine side either.

Rule produced: when you move "what the engine does at this site", count everything set at that site —
two fields set by one function are usually one fact.

## 2026-08-12

### Oracle decay (§5.1-e)

Recorded and measured three times: as tier-3 delegates to shared code, the differential compares vita
with itself, so 100% coverage can mean 0% oracle teeth. Four of the last five slices at that point
contained zero kernel lines and routed native into `SimState`, `builtins::dispatch` and `eval::*`. The
three measurements are §4.5.330/331 (four shared-function mutations passed the entire differential),
§4.5.337 (two shared-dispatch mutations passed every engine gate), and V1 slice 3b (both backends
printed the same `[ ][\u{1}][ ] len=0` — the differential green, the design wrong).

Rules produced: "byte-identical to the VM" is not sufficient evidence for a delegation slice — the gate
needs an absolute anchor, a hand-IEEE expected output the differential cannot in principle protect (an
element default of `""` or `0.0` from `new[]` is the canonical example). Do not build a third executor
as a substitute; the only permitted separations are ROLE and BUILD.

Confirmed as a build property in Phase C (2026-08-17): the `oracle` feature (default on) carries
`interp` and `vm`, and a `--no-default-features` build does not contain them, so comparing two
executors in a product build is impossible and an absolute anchor is the only defence. The
interpreter's role is pinned in `Backend::Interpreter`'s doc as a test tool permanently excluded from
performance work. The flip run's direction reversed with the default: it now asks native → vm, and
without it the suite quietly becomes native-only and the whole rule is vacuous.

### A build-failed mutation reported as SURVIVED (§5.1-l)

Measured: the battery's substitution script died on `assert count==1`, the runner did not check the exit
code, the tree was unchanged, the tests ran, and the result was recorded SURVIVED. Applying it by hand
also failed to compile (a trait not in scope) and a stale binary produced the same wrong answer.

The fake survival was still worth something: it prompted the question "why do these two stores give the
same answer?", and that found a real defect — `k_read_net` was a second spelling of `NetReader::read_net`'s
routing, and frame-local and heap nets do not belong to the arena.

Rules produced: check the exit codes of the substitution and the build and record a failure as
BUILD-FAIL; grep the changed line in the source after substituting; and investigate a fake survival,
because the question is "why did it not die".

## 2026-08-10

### An ablation measures one gate's axis (§4.5.326)

Measured: doc-21 revision 6 disabled `wprog`'s width and sign uniformity gate, measured 0 across eight
designs, and closed the whole admission axis. The worst shape (an array read) is rejected not by
uniformity but by having no arm at all (`Concat`), and a node with no arm is rejected whether the gate
is on or off — the experiment could not see that axis. One `Concat` arm took array reads from 0.94x to
1.26x and memory from 0.92x to 1.85x.

A ten-minute instrument at the rejection point (`compile_node`'s fallthrough), printing the node kind,
separated the axis immediately: `Concat` 32, `Select` 73, `Ternary` 39. An aggregate of "why is this
rejected" is cheaper and more complete than a measurement of "what happens when one cause is disabled".

The answer had been sitting in a green pin's failure message since §4.5.308, in as many words: "a
`Concat` arm in `wprog` would win them back". A green pin's message is never read.

`both_backends_print` keeps only `out|t=` lines, so it is blind to diagnostics, and two new tests
written to measure DIAGNOSTIC ORDER passed by comparing empty streams.

A row testing signed sign-extension was written as `{sa, b}` and was structurally immune, because the
fill runs off the top of the result width and the mask deletes it; `{a, sa, b}` — the middle position —
killed the mutant.

Rules produced: close a family with a census, not an ablation; write what you measured and what you
could not; grep green pins' messages when re-choosing the queue; read what a harness discards before
reusing it; and check that a row leaves the symptom somewhere observable.

### Two candidates, two ceilings (§4.5.327)

Measured: a census left `Select` 73 and `Ternary` 39, and measuring each candidate's ceiling with a
discriminating design showed the loss is larger on `Select` (0.83x against 0.98x) and the machinery
simpler (a constant, in-range window is a shift and a mask). Choosing by count would have given the same
answer, which is luck rather than an argument.

A generic ternary evaluates only the taken branch. Making it eager keeps the value and adds an E4002
from the untaken branch's `LoadIdx` — more loud is also a divergence. §4.5.328 found it as a real
defect rather than a hypothesis: the default backend's ternary is exactly that eager form with no
guard, so `mem[9]` in the untaken branch made `--backend vm` alone exit 1 where interp, native and
iverilog exit 0. It was found because the anchor built to prove the new admission ran on BOTH backends.

Two rows written for unknown-condition merging were both measured redundant, because a 65,536-state
sweep already reaches every matching pattern; they were deleted and the reason written into the source.
A battery made only of admitted rows cannot test admission — the mutant weakening the range check was
killed by one row (`a[4:1]`, a window straddling one bit). And the mutant deleting `-:`'s `- width + 1`
was killed only by a CLI absolute anchor.

Rules produced: measure both ceilings; check what the generic path does not evaluate before admitting a
node; run property anchors against every backend; name the mutant that would survive without a proposed
row; include reject-arm rows; and name the absolute anchor whenever a rule moves into shared code.

### The optimisation target is decided by a census (§4.5.332)

Measured: the profile pointed at the write funnel at 18.9% and the plan was to fix the bit-serial loop
inside it. A ten-minute instrument said 57,130 of 4,058,223 writes are bit-serial (1.4%) and 98.6% are
whole-element stores, so the target was the path leading there.

Putting a fast path inside the canonical implementation makes every existing test exercise only the fast
path; a test-only entry point taking a literal `bool` gives the differential its other side (production
call sites pass a literal, so the branch folds).

A value-only snapshot of a new store point is blind to the dirty set, the edge kind, the last blocking
writer, the VCD queue and the deferred diagnostic queue; the snapshot built here carries all six, which
is what makes removing `note_change`, dropping edge accumulation, and aliasing the unknown plane die
directly.

Reviewing the slice's own test design found two holes: the test design had no edge sensitivity at all,
so `is_edge_target` was false everywhere and glitch capture and `accumulate_edge` never ran across a
20,160-row sweep; and `Value::zeros` never sets `is_real`/`is_str`, which are exactly the two flags the
entry point's admission argument rests on — the claim had zero rows. Before those were fixed the
mutation set died only through another test's infinite loop, which is not attribution.

Two mutation runners left running at once wrote the same files alternately, made both result sets
meaningless, and left a mutation in the tree — twice. Three countermeasures: flush results per line,
copy the original outside the tree before starting (a `finally` cannot catch SIGKILL), and confirm by
PID that the previous runner is dead (`grep -c` gives a false 0 because of the quoting).

Rules produced: count the branches inside a hot function; give a canonical-embedded fast path a
test-only entry point; compare every channel a new store point owns; review your own test design and
assert that the code runs; do not credit a mutation that died by infinite loop; and ask where the
discriminator would be before calling a survivor equivalent.

### A census says what is rejected; the profile says whether it is worth anything (§4.5.329)

Measured: unique-root admission was 93.8% and the residue was mostly width and sign mismatch — and the
generic tree walker was already at 2.2%, so opening the whole axis has a 2.2% ceiling. Making the
canonical `truthiness` O(1) for <=64 bits was written up as benefiting all three layers and measured as
benefiting only native (3.1%): tier-2 has its own inlined copy and the interpreter's conditional share
is small.

Equivalence was proved without restating the rule by asking the same bits at two widths — 4 bits (the
fast path) and 128 bits with a zero top (the loop) — so the loop is the oracle and the test contains no
copy of the rule. The verdict distribution was pinned as `(175, 1, 80)` WITH its derivation (each bit is
0/1/x/z, so True = 256 − 3^4), because a number without a derivation cannot be checked by the next
reader.

### Exposing a flat entry point (§4.5.330)

Measured: wrapping two `u64` planes in two 72-byte `Value`s to call a comparison function was 9.2% of
picorv32. The way to expose the flat entry point without creating a second spelling is to have the
canonical implementation finish its own normalisation and then delegate to it; the admission argument
("the two operands share a width and a sign") is what makes the normalisation a no-op. The price is that
the differential goes blind to that function from then on, so every sharing slice must check by mutation
that an absolute anchor protects the rule — all four were checked here.

## 2026-08-09

### A lowering that rebuilds an operation in its own spelling (§4.5.317)

Measured: `4'(u << r)` inside a size cast was silent while the identical `u << r` outside it was loud,
because the special lowering reconstructs the shift in its own spelling and never passes the general
path's real gate. A value-only differential cannot see this.

A guard placed at one leaf was documented as complete, and six places build the operand. `$signed` and
`$unsigned` are transparent to the value's domain, and implementing that transparency as a wrapper peel
at the call site cannot see a wrapper BELOW an operator (`Binary{Mul, $signed(r), 2}`); the predicate
has to strip at every node inside the recursion.

Reporting per leaf eats `MAX_ELAB_ERRORS` N times faster and deletes unrelated later diagnostics; the
standard is what the oracle reports, and the duplicate-suppression flag must be saved and restored per
construct, or a nested identical construct loses its own report. Siblings cannot test save and restore
at all — a design that resets the flag on entry makes siblings structurally immune, so the sibling-pair
test passes with the restore code deleted; nesting is the direction the mechanism protects.

An `args[0]` predicate and an `args.iter().any()` predicate give the same answer when the target is the
first slot, so a killer design must put the `real` argument outside the position the predicate looks at.

### A remaining backend split is an unfixed site (§4.5.319)

Measured: fixing one defect moved the backend census's split count from 6 to 12. No backend got worse
(WORSE 0), so it was read as "different backends were fixed by different amounts" and deferred as a
class. Fixing the last site brought the split to 0 and the census to 120/120 correct. The cause recorded
in §2 ("the bytecode does not self-determine the exponent") pointed at the wrong place: the exponent was
already a self-determined `Const` in the IR, and the real site was a codegen predicate masking that
constant at its own width.

Rule produced: WORSE 0 means no regression, not "stop here"; a slice is not done until the split is 0,
and stopping earlier requires naming each remaining split by its measured site.

### You own the comment you edit (§4.5.319)

Measured: a block was opened to correct one stale comment and another sentence in the same block was
false — and that sentence was describing exactly the silent-wrong at that site ("a negative signed const
reads huge here, so it is rejected below"): `to_u128` masks a constant at its own width, so a narrow
negative reads as a positive of that width, not as huge.

Also: a codegen lane is not reproduced by an `initial` block, because codegen does not attach and the
specialisation lane is never entered — testing the default backend's specialisation path needs a clocked
process.

### Fix the contract, not the operands (§4.5.319)

Measured: a shared arithmetic kernel reads both operands under one collective sign, and `**` breaks that
contract (IEEE Table 11-21 makes the exponent self-determined, so the two signs are independent). The
kernel papered over the mismatch by stamping the exponent with the base's sign, which is a bit
reinterpretation rather than a value-preserving step. Rewriting the operands into an "equivalent pair"
— widening by one bit when the signs differ so both are signed — was perfect on the value axis and broke
both extremes at once: the extra bit crossed `WIDE_ARITH_CAP` and produced a silent `x` (that loud gate
reads the DECLARED width and is structurally unable to fire on an internally created one), and the
equality the byte-identity argument stands on is false for special types. The correct shape separates
the three questions one boolean was answering — how the left operand is read, how the right is read,
and what the result's sign is — which are identical on most operators and are what let them be folded.

`base.width == w` was used as the basis of a byte-identity argument, and `resize_keep_sign` — which
produces that width — short-circuits on `is_str`/`is_real` and returns its own width, so the equation
breaks on a string operand and the operation runs at 32 bits. A reviewer probe on that arm ran the whole
suite: 114 entries, 0 violations. The claim was true everywhere the suite reaches and false only outside
it.

Rule produced: when an argument rests on an equation, count the types that break it — meaning the
early-return list of the function that produces the width, not a list of value types.

### An oracle's scope of applicability differs per axis (§4.5.319)

Measured: iverilog 13.0 is not an oracle for `**` with a negative exponent at width >= 33, and the
boundary is exactly 32/33. `(-1) ** -2` is 1 at width 32 and 0 at 33 and 64; `0 ** -2` is `x` at 32 and
0 at 33. The wide path loses IEEE Table 11-6's +/-1 rows and the zero-base row entirely. Not recording
this made a reviewer's first width sweep produce 36 false REGRESSION cells. verilator keeps the +/-1 rows
at every width and, being 2-state, has no `x` row, and it contradicts itself on the same expression
(constant folding gives `x` and runtime gives 0). Where both oracles fail on different axes, the spec
text decides — Table 11-6 looks at op1's VALUE, so an unsigned all-ones is not −1 — and the verdict is
recorded together with its width condition.

### Oracle and performance baseline are different tools (decision)

Recorded: the performance baseline is iverilog plus vita's own `--backend vm`, because both share vita's
contract (4-state, event-driven) and therefore measure the same thing. verilator is not a performance
target — it is 2-state, compiled and cycle-oriented, so the x/z plane, delta cycles and event-queue cost
are all booked as "vita is slow"; its numbers are quotable only as the ceiling compilation can buy, and
only with the sentence that the contract differs. Its real place is as the second oracle: two oracles
agreeing while vita differs is a defect, not a debate. The axes where verilator is not an oracle were
measured first: x/z (2-state), out-of-range indices (it masks into a power-of-two array and returns
plausible garbage), and event and delta ordering. Its scope is 2-state arithmetic, width and sign.

### Comparing byte output (moved from the loop file)

Recorded: NUL and control bytes vanish in a terminal, so any feature whose output is a byte stream (file
writes, `$fwrite`, packed strings, VCD and FST) is compared through `hexdump -C`. And for an
element-select silent-wrong, measure the WHOLE-value operation first: correct there means the access
routing is at fault (shallow) and wrong means a storage gap (deep), which decides the slice size.

## 2026-08-08

### Narrowing and widening a context width are not the same code (size-cast slice)

Measured: IEEE §11.8.1 propagates `max(self, context)`, so a context only widens. Code that hands the
context width straight to an operand is right when widening and silently a different computation when
narrowing — `2'(k%4)` turns the divisor `4` into `0` and produces `xx`. Low-bit closure
(`+ - * & | ^ <<`) holds only in 2-state, because one `x` makes the whole result `x` in IEEE and
narrowing deletes it.

A reviewer dismissed a real new defect because their base case (`40'({u1.k})`) was already wrong in
main, hiding it; `u1.k[7:0]` is correct in main and would have separated them. "The gate rejects that
shape" was true of one spelling (a whole-net hierarchical read) and false of the family — element,
bit/part select and string all enter and produce the same defect.

A temporary value for a width verdict was set to `u32::MAX` with the comment "this only decides which
branch runs"; the same value is used a few lines below as a FILL SIZING context, giving a value
regression, a 22x `.velab`, and RSS from 10 MB to 1.4 GB.

A 1,675-cell sweep reported zero regressions and a reviewer's 74,400-cell sweep of a different shape
reported 1,586. The difference is shape, not size: the operand pairs in the first sweep never contained
that combination.

Of four operators grouped under one rule, three were fine and one was not, and the reason is that
operator's own property (`>>>`'s sign changes the operation), not the common rule. One change fixed 886
cells and broke 856, which the total hid. And fixing "the context width narrows wrongly" by treating the
node as self-determined loses the SIGN with the width, because a context's signedness always propagates:
397 narrowing cells fixed and 245 widening and sign cells broken — a trade, which the ladder forbids.
What needs fixing is the propagated VALUE (`max(self, N)`), not the propagation.

### A sweep is axes, not size (§4.5.318)

Measured: one slice claimed "0 regressed" three times and a review overturned it three times with a
different shape — 1,764 cells (missing axis: unsized fill), then 1,620 (missing axis: a nested
width-sensitive sub-expression), then 6,720. Writing the axis list as a comment at the top of the script
was not enough, because only the listed axes were multiplied.

A `None` fallback masks a defect rather than fixing it: a classifier answering "unknown" falls to a
conservative path, and whether that path is right by construction or by accident is a separate question.
Making the classifier smarter exposes everything the fallback was hiding, and those count as your
regressions — leaf interpretation exposed 60 fill cells, fixing those exposed 42 nested-width cells,
then double evaluation of a function leaf, then formal sign.

"Unknown" is also not a substitute for a wrong answer: replacing an unpacked array element's wrong
"unsigned" with `None` made a cast skip context descent entirely and broke 170 of 1,560 cells. The
answer was the element's actual sign. Three states: wrong answer, unknown, right answer.

An inline function's argument binding must apply the formal's width, sign and 2-state-ness (the frame
path gets all three free because the formal is a net); applying sign alone regresses 45 cells and adding
width hits another path's precondition, so partial application is not available — the three are each
other's preconditions.

Fixing a precondition by working around it produced a regression in each of three rounds: a skip keyed on
node kind missed six spellings of the same value (a trap §4.5.310 had already recorded) and blocked
correct fixes in the other direction. A workaround predicate wrong twice is an ORDER problem.

The retreat option was measured too, and was worse: FIXED 303 → 215 and REGRESSED 0 → 80, because nested
bindings feed each other.

Conditioning a stamp ("skip the node when the sign already matches") removed the seal exactly where
nothing else made the expression self-determined: 48 of 1,440 cells, all through that one hole. All 48
were unsigned returns — the signed ones were sealed by ACCIDENT, because an unsigned RHS made the stamps
differ — and a characterisation concentrated 100% on one value is a signal that the other half survives
for a different reason.

Sealing a cast as self-determined produced five regressions across four adversarial rounds, all one
root: elaborate knows the operand's width and sign wrongly — a placeholder or string gives `None` and 32
is FABRICATED, a class field gives a handle net's wrong `Some(32)`, an element has packed rules applied,
and a sign-extending fill names its operand twice. Before the seal, the outer context re-walked the tree
and cancelled the wrong picture by accident. A fabricated `Some` is as dangerous as a `None`, so trust
has to be decided by comparing two sources; enumerating special cases just produces the next case next
round; and cells PRE got right by cancellation all read as regressions afterwards.

An unsized fill's value IS its width, so "low-bit closure" and "narrowing computes the same thing" hold
only where a leaf's value is width-invariant. Folding a range to obtain a declaration's SIGN emits
diagnostics, so the classifier changes the program; extracting the sign half as a pure function removes
the side effect with no restatement. A mutation that looks up an FQ-keyed map with a bare name always
misses and falls back, so it is equivalent and its survival is not a coverage hole. And no fixed
name-resolution order is a rule: "nets first" and "parameters first" each break different designs, so a
classifier that resolves names has to follow the lowering's whole decision procedure, inline substitution
included.

### Performance records cannot become the next slice's premise (preview/21 grounding)

Measured: `native/vm` going from 0.97x to 0.81x reads as a regression and was the VM getting 1.18x
faster with native's absolute time flat. A recorded bottleneck is that moment's bottleneck: the queue
said "the remaining bottleneck is the scheduler" and profiling the same design showed that to be a VM
property (`propagate_changes` 40%) while native's was a generic tree walker at about 50%. In a
specialised backend, "my specialised code is barely in the profile" means the path is not entered.
Third-party benchmark numbers from a gitignored checkout do not reproduce, so the control has to be
committed. And this repository's `[profile.release]` sets `strip = "symbols"`, so `/usr/bin/sample`
returns `???` for everything unless built with `CARGO_PROFILE_RELEASE_STRIP=none
CARGO_PROFILE_RELEASE_DEBUG=1` under a separate `CARGO_TARGET_DIR`; parse the "Sort by top of stack"
section, because counting the call-tree section gives the root everything.

### Registering a value in a table of the wrong type (§4.5.314)

Measured: putting a real into the `i64` `hier_params` table makes `a.P` RESOLVE and makes `a.P/2` an
integer division — a loud becomes a silently wrong value. The same shape occurred four times in one
slice (folding a fill at 32 bits, a real as an `i64`, sending a fill override as `value:None` and
disabling every guard, and re-publishing an interface). The question when choosing a storage table is
whether the consumer can produce the right answer from that representation.

A leaf-patching resolution arrives after the enclosing context is decided: a deferred hierarchical
reference is a placeholder at lowering time, so the cast, concat and width context around it are baked
for the integer path, and replacing the leaf with a real `Const` afterwards makes consumers read the
BITS (`int'(a.P)` = 0, `longint'` = the IEEE-754 word). Where the resolution cannot re-lower the
enclosing node, the support cannot be built.

A test written with `parameter real P = 4` cannot separate the integer and real domains, because
`4/2 == 2` either way; one character produced a false premise that stood for three rounds. Test values
must separate — an odd number for division, a value past 32 for width, a negative for sign. Direction
matters as much as value: a discriminating row written as `a && (a<b)` (the wide operand on the left) is
the harmless direction, because re-reading a small value wide only adds zeros, and the dangerous
direction `(a<b) && b` was absent, so a mutation using one width for both operands passed 5,280 tests —
and real RTL writes that form.

A slice's own correctness ARGUMENT needs a test: the reasoning for evaluating `&&` eagerly ("the generic
path does not short-circuit either") was written as a paragraph with no pin, and a mutation adding a
short circuit passed the whole suite and turned native from loud to SILENT (exit 1 to 0).

And fixing one cell is not knowing the axis: after the `P = 4` incident the conclusion generalised to
"the discriminator is the value", and a feature was removed on that basis. The next round counted the
matrix and refuted it — the discriminator is the OPERATOR (a fractional-quotient division), the
distribution is 72 correct against 6 wrong, and the removal was a regression. A consumer x value matrix
of 13x6 takes minutes.

A revert is an edit: this slice's revert deleted a regression test along with the code, and the loss was
invisible because the suite was green (a mutation reverting that wording passed for as long as it was
missing).

## Undated slices, and the loop-file rules moved on 2026-08-03

### Walker completeness and the give-up state (§4.5.269-271)

Measured: `_ => false` in an expression walker means "this node may reference anything", not "unknown",
and in a conservative accept gate that answer is independent of the name being asked about — so one
chained method call (`s.substr(a,b).atoi()`) refused every local declared later in the block, including
the target itself. The reporter's observation that splitting the chain into two statements makes it pass
is the fingerprint of that structure.

A call path's head may be the function name or the receiver, and the segment count decides which:
`f(x)`'s head is a function and `s.atoi()`'s head is a variable being READ (`pkg::f()` is a separate
node, so a multi-segment path is always a dot path). A walker that looks only at arguments misses
`q.size()` and `a.push_back(x)`, and a scope-leak detector built on it silently reads the wrong net.

A walk that gives up with `None` leaves the caller with no reason and no place to point except the
declaration, which is why the reporter received 21 accurately located diagnostics and could narrow none
of them; `Result<T, GiveUp{span, reason}>` produces a note, and that note is the next round's narrowing
tool.

A monotone invariant ("once assigned, always assigned") has to be usable at every recursion point; the
definite-assignment walk applied it between statements and not inside nested structures, so an
assignment outside a loop worked and the same assignment inside one did not — exactly as the reporter
observed in situ.

`#1 y = 3;` references no more names than `y = 3;`, and a single `delay.is_some()` made it "unverified"
and ended the whole walk. A timing prefix only adds expressions.

Removing an accidental loud has to be measured: modelling the timing form broke a regression pin about a
block-local colliding with a generate-scope net, and that pin was relying on `#1` falling into a
catch-all — and the accident was RIGHT. Promoted to a rule, it becomes: refuse a time-advancing
statement only when the net is SHARED, because advancing time hands the scheduler to another block; and
that check has to be recursive and to run before the ref-free fast path, since `begin #1 y=3; end`
yields just as much without mentioning the local.

Building the oracle was itself a measurement: iverilog could not parse the chain, so a decomposed oracle
(`t = s.substr(a,b); t.atoi()`) was built, and in the DECOMPOSED form vita and iverilog diverged — the
`atoi` family is `strtol`. Checking that the workaround and the original agree is what surfaced it
(chained == decomposed == iverilog).

`parse_radix_prefix` carried a comment saying "iverilog 13 drops it, its bug" and a pin fixing that
behaviour. IEEE 1800 §6.16.9 says the scan takes all leading DIGITS AND UNDERSCORES and stops at any
other character, so a space or a sign ends the scan immediately and `_` is scanned without contributing
— a `strtol` with an LRM citation attached, with both lenses (the LRM and iverilog) on the same side.

Opening a never-written accept exposed IEEE §23.9 (no hierarchical reference to an `automatic`
variable): the v1 flatten produces a static address, so another module's `tb.a = 99` was using
per-entry storage. Measured pre-existing, and the slice widened its reach, so it was closed with it.

### Executor routing (§4.5.277)

Measured: a gate asking "can this executor run this statement" looked only at the destination, so
`rc = $fgets(line, fd)` read as an in-frame write, the task stayed on the synchronous `&self` executor,
fell into the pure `eval` path, returned 0 and wrote nothing — the effect was in the RHS. The symptom
said so: adding an unrelated `$display("x")` to the body made the read succeed, because
`Stmt::SysTask` falls to `_ => true` and moves the whole task to the `&mut` executor.

Over-reporting costs differently per consumer: a routing SUPERSET is free (`&mut` is a superset of
`&self`), while using the same set to change a function's CALL SHAPE re-routed every dyn-formal function
(because `foreach (b[i])` desugars to `b.first(i)`/`b.next(i)`) and the copy-out path cannot bind a dyn
array formal, so working designs went loud. Two questions need two predicates: "is this an effect" and
"can this executor never do it" — an assoc iteration is an effect and the `&self` executor manages it
when the key is body-local.

The insertion point is decided by the code that reads the set: filling a function-routing set during the
task reject stage happens after the call sites inside a frame TASK body are already lowered (a task body
allows nested calls where a function body does not). The only correct point is immediately after
function-body lowering and immediately before task-body lowering.

A fatal latched in a `Cell` (because an `&self` context cannot return `Step::Fatal` mid-expression) was
polled by the scheduler at three places, all BEFORE running a body, so a process that set the fatal in
its own body ran on to its own `$finish` and ended cleanly. Time order decides: a fatal that happened
earlier in the body beats a `$finish` reached later, so the statement loop polls too — and the side
effect, "the diagnostics merge into one", is the correct behaviour.

There are three collectors of body-local net kinds (module scope, frame body-local, inline/static task
body-local) and only the first two have a `string` arm, so the same `string s;` is correct in a
`task automatic` and falls to `Wire` in a static `task`. That one `Wire` produced two different failures:
a plain `s = "hi"` is loud (E3018) and `$fgets(s, fd)` is SILENT, because a system function's destination
write does not pass the lvalue check that raises E3018.

A comment recorded that the real condition for a panic "is not yet named", after two reverts. Measured,
the condition is not "inside a frame body" but "is the copy-out destination outside the frame window" —
a frame-local destination is the write a nested TASK call's `out_binds` already performs, and only a
module-net destination dies at rc 101. Gating on the destination removed the panic without losing ten
measured-correct shapes.

A new arm calling `lower_lvalue` must sit below the checks documented as "detected FIRST": `s[i] = f(…)`
becomes a silent packed BIT write if `lower_lvalue` reaches it first, and the file's own IEEE §6.16.3
comment says so.

### One description shared by several walkers (§4.5.275)

Measured: opening expression positions wholesale needs a detector, a reachability gate, an
evaluation-order gate and a transformer, and four independent recursions inevitably diverge; a single
`shape()` consumed by all four makes classifier/lowering disagreement structurally impossible.

A verdict NAME is a contract. `ExprDa::Writes` means "every evaluation writes", and it was given to a
conditional write inside a nested `&&`, so a short-circuit path read a same-named sibling block's
residue at exit 0. What was needed was `Clean` — read-safe, no write promised.

Hoisting copy-out FORWARD across rhs/lvalue-index, index/index and argument/argument boundaries while
analysing each fragment separately left the cross-boundary ordering hazard invisible and disabled an
existing "two calls writing the same target in one expression" guard. The expressions a statement
evaluates in order are one sequence.

Putting repairable reads (a single segment) and unreachable reads (a hierarchical path, a callee body,
an unresolved method body) into one set means quietly believing the second kind was fixed; the second
kind is a stand-down signal.

`$bits` and the array queries do not evaluate their operand (IEEE §20.5/§20.6) and `$monitor`/`$strobe`
re-render their arguments later; hoisting into the first invents a side effect the source does not have
and hoisting into the second freezes a temporary. That list must exist once and be shared, and here the
second hoister was not reading the list the first had already built.

Capturing a truth value as `x || x` names the same expression id twice, so the engine evaluates it twice
and a `$random` operand is drawn twice; `!!x` is the same 4-state reduction with one evaluation.

Removing a loud exposed what was beneath it, and this time the lid was the slice's own walker: the old
safety gate looked at a single segment and never entered a body, so a hierarchical read and a
callee-body read were already reading post-call values in PRE. Merging the two similar gates is what
made it visible.

Two lenses reported opposite things about the same shape (PRE loud against PRE 56), and a corpus sweep
settled it at PRE 56 — a pre-existing silent-wrong.

A soundness lens ran `git checkout HEAD --` to build PRE and nearly overwrote work in progress.

Round 2 of the same slice added more. A statement's own call OUTPUT actual is a write destination and
not a read, so an evaluation-order snapshot that redirects it makes the callee's copy-out land on the
snapshot net and the user's variable keep its old value — the write disappears rather than the value
being wrong. `inout` is both a read and a destination, so it is a stand-down. "Another gate rejects it
anyway" was valid only while there was one gate, and the same predicate was also the narrow path's gate.
Smearing "unknown" over a node stands the whole statement down even when the node contains no call, so
the children must be listed and the nodes that are not evaluated separated from those that are. Two
walkers that must see the same children need one child-list function, and reads the substitution cannot
reach must be marked unrepairable so the two answers agree. Aliasing is judged by target, not spelling —
a hierarchical read aliases a bare local only on a self path. Asking a two-discriminator state with one
flag left the other half ungated, and that half hit a `debug_assert`: a panic with no diagnostic, or a
write to someone else's net in release. A check consuming a set must audit the set's POPULATION path,
and that population must not zip formals positionally, or named arguments are invisible. And a
`debug_assert` is not loud, because it disappears in release.

### Admission position and execution position (§4.5.274)

Measured: writing "where does the write happen" as a STATEMENT SHAPE breaks the moment a call returns a
value, because the call can then appear anywhere an expression can; 33 of 34 reported items were that
one defect. The admitted node set must equal the set where the lowering can actually emit copy-out —
wider is harmless (the lowering is loud) and narrower is a false-loud.

A branch knows the condition's VALUE: `a && f(r)` being true means both operands were evaluated, so the
BODY knows about the write and the else does not; `a || f(r)` being false means both ran, so the `else`
knows. Collapsing the whole condition to one bit makes the standard `.rsp` walker idiom loud. All four
premises were measured with iverilog by adding output side effects, not asserted.

A three-valued effect lattice with `Clean` (no read, no promise) keeps `c && f(out r)` from falling into
"references, therefore reads". A new walk's catch-all must produce the old walker's answers literally,
or an expression-position condition that does not even mention the name becomes a read.

Where the natural workaround for a refused construct is itself refused, the defect count is two — and
the refusal's own message omitted the statement position from its list of supported positions, so the
user hunted for a workaround that did not exist.

The same rule already existed, with a comment, for class methods (`default_is_scope_safe`), and only the
plain function and task twins lacked it. And a default-argument guard written as "refuse if it names
anything" kills the normal case of a generate block or subroutine body naming a module net; comparing
the BINDING (same prefix and empty substitution passes in O(1), otherwise compare) keeps them.

`task automatic` is lowered into two copies (frame and inline) and the caller may use the inline one, so
an elaborate gate on the frame copy false-louds working designs; what the executor cannot do belongs in
the executor's fatal channel, where it fires only when that copy actually runs.

And opening a gate and stopping at the minimal reproduction leaves the silent-wrong beneath for the
user: driving the reporter's real idiom, file I/O included, is what found the next defect.

### Shallow and deep twins (§4.5.273)

Measured: an earlier round fixed the shallow `stmt_no_ref`'s timing-prefix handling and left the deep
`stmt_no_ref_deep` (used for callee bodies) untouched, so calling a standard clock-driver task
containing one `@(posedge clk)` made every later local in the caller unusable — 11 of a round's 12
diagnostics.

"Does it reference" and "does time pass" are different questions needing different resolvers: a callee
proven inert for a name can still yield the scheduler, so opening the reference gate without the time
gate widens the time hole.

An early return on "already safe" is right only where the invariant is monotone. On a fresh net "once
written, always ours" holds; on a SHARED net it does not, because suspension is where ownership changes
— and that early return meant the shared-net rule was never reached.

An order-dependent gate ("this symbol already exists, so it collides") never checks the FIRST
declaration, which passes because of arrival order rather than meaning; a symmetric answer belongs in a
pure AST pre-computation.

`automatic <unpacked-struct> r;` had its lifetime silently demoted, because the type name is not in
`typedefs`, the automatic-lifetime helper failed, and the following member fan-out parsed with no
lifetime. Contrasting with `int`, an enum and an alias in the same position is what identifies a
different parse path.

Knowing the desugar simplifies the rule: struct members become constant part-selects at parse time, so
at definite-assignment time there are no member cells and BIT coverage is both simpler and more general.
And structure fan-out renames the variable (`$unp$v$m`) while leaving the call ARGUMENT as `v`, so a
walk that tracks member names sees the argument as touching nothing — a false-loud for the write and a
potentially unsound `inout` copy-in read.

### Argument observability (§4.5.272)

Measured: the arguments a person reads and the arguments a process receives are different texts. A
Makefile or wrapper has already substituted `$(WIDTH)`, a filelist expansion has removed the `-f` frame,
and an environment knob leaves no trace in argv at all — and only the expanded form decided the run. At
logging time it cannot be reconstructed.

A `--dump-filelist` subcommand EXITS, so it cannot coexist with a run log and cannot answer what the
failing run compiled. Observability output must be in the same process and the same stream, and sent
through the diagnostics writer so it lands in the `--log` tee with consistent ordering and quiet-flag
policy. A derived value needs its provenance stamped beside it (`--threads`, the environment variable,
or auto — and the third is the hardest to find). A line break between a flag and its value turns `-D` and
`W=32` into a bare flag plus a stray source file, which is the opposite of the feature's purpose.

The feature's first proof was a defect it exposed: the filelist expander's `takes_value` list was still
the original five entries, so every value flag added since had its value rewritten as a source path
inside a `-F` frame — `--top top` was a false loud and `--hier-tree h.txt` wrote to a different
directory at exit 0.

### Static analysis gates (§4.5.266-268)

Measured: a definite-assignment walk carrying a single assigned boolean cannot separate "falls through"
from "jumps out", so a `break` or `continue` before the first write reads as a live path reaching a
later read — 49 of 84 reported items were that one collapse. Widening the lattice fixes it, and
extending non-fallthrough treatment to a general `disable` is the next silent-wrong: a `disable` naming
a non-ancestor block does continue, so that path drops out of the join and a real read-before-write
passes. Only a distinguishable reason (a synthesized `$break$`/`$continue$` label) justifies the special
case, and an escaped identifier can contain `$`, so whether a user can write the same spelling has to be
checked.

A place marked "deliberately unverified" turned out to be a measured, correct constraint: a
statement-position user call ends the walk because v1 publishes block-locals as module nets, so a callee
body can name them — confirmed both with `$display(a)` and with a hierarchical `t.a = 99`. The answer is
to prove the callee cannot touch the name, not to remove the constraint.

A head-segment path rule (`p.segments.first()`) is exact for a caller's own statements and wrong for a
callee body, because a flattened net is also reachable as a hierarchical self path — the question
becomes "can this callee touch that net", which needs every segment, added as an opt-in parameter rather
than a copied walker.

Making an initializer-less `automatic` per-entry and resetting it on block entry looks like a direct
reading of IEEE §6.21 and is wrong: automatic storage is created per ACTIVATION, so iverilog gives
`xx, 10, 11` for three loop entries (residue survives) and `xx, xx, xx` for three calls; only the
initializer re-runs on entry.

Where a property cannot be proved (an element-wise filled fixed array — a `foreach` index is not a
constant), narrow the coverage to the provable subset (literal indices, block top level, an RHS that does
not read the array) and keep the rest loud, rather than changing runtime semantics to admit everything.

A declaration's declarators are independent: judging by `d.names.first()` and applying the verdict to all
of them discarded a whole declaration and made every later use "undeclared" — one module net produced 8
diagnostics, 7 of them innocent. Splitting only where the verdicts differ leaves existing designs
byte-identical.

Two stages building the same name must write the rule once: the nets stage hoisted flat while the logic
stage nested with `with_scope`, so once both levels are scopes the path where the net exists and the
path that looks for it diverge — and the classifier grew a rule discarding every nested candidate, which
surfaced as a feature limit (one level works, two do not).

A rule written as "direct child" (`!rest.contains('.')`) silently broke the moment blocks could nest,
because nobody claimed the deeper keys; the unemitted-initializer guard is what caught it.

A diagnostic must say only what it knows: "this one is `automatic`, so the OTHER is not" is an inference
from "this pair did not get a scope", and scoping is withheld for several reasons — in the reported case
both were `automatic` and the reader went looking for a static twin that does not exist.

A diagnostic raised in a resolve pass has no location unless the defer record carries the defer-time
span, and a hierarchical task-call refusal was the only one of 84 reported items with no `file:line:col`,
in a testbench where it was the only diagnostic.

A snapshot mechanism emitted once before an expression has ONE slot, so an expression calling the same
function twice reads the last snapshot in both places and a recursive call reads its own overwritten
formal (correct only when every level happens to pass the same array, which is exactly what the existing
pin caught). Where the position cannot be split into temporaries (a frame body), refusal is the answer,
and the message must say "there is only one slot".

### Standards conformance versus conservative refusal (§4.5.284)

Measured: doc-15 codified vita's refusal of implicit nets as a deliberate conservative choice that shuts
off the class of accidents where a typo silently becomes a wire, and that reasoning is sound. Two facts
overturned it: IEEE 1364-2005 §3.5 is the standard and the default is `wire`, and the construct appears
inside a foundry-supplied cell library the user cannot edit. A warning buys the same safety and
`-Werror=` restores the original policy. The trigger for revisiting a policy is not that someone
complained but whether the user has the authority to change the code.

The boundary of the new leniency was pinned with the oracle rather than inferred: §3.5 covers two
positions, and running the RHS, a procedural lvalue, `` `default_nettype none `` and the `.name`
shorthand showed all four are errors in iverilog too — so this is conformance, not leniency.

The parser desugars `.a` to `.a(a)`, so the rule "every named port actual is a §3.5 position" silently
accepted `.a` (IEEE 1800 §23.3.2.2 requires a declared object and iverilog refuses it), and
`dotname_missing_signal_is_loud` failed at once. A new rule that judges after a desugar needs to know
what the desugar merged, and the flag must be restored when the merged forms have different oracle
answers.

An implicit declaration is a DECLARATION and therefore a phase: creating it at the use site made one
design give two verdicts, because vita lowers continuous assigns before instances, so
`sub u(.o(IMPL)); assign o = IMPL;` was E3010 at the read and successful at the terminal. And there must
be exactly one collector.

A §3.5 net is scalar, so a wider driver drops the upper bits — legal, and every simulator does it
silently. The differential wins, so the value stays truncated and a warning naming both widths is added.
Correct-or-loud means "do not let it go unnoticed", not "change the value".

### Lexing layers (§4.5.283)

Measured: deleting attribute instances `(* … *)` with a logos skip regex scans RAW TEXT, so in
`always @(*) a = b;  // *)…` the `*)` inside the comment closed the sensitivity list's `(*` and the rest
of the comment became executable code — a wrong value at `errors=0`. The fix is the LAYER, not the
regex: doing the same pairing on the TOKEN STREAM means comments are already gone and a string is one
token, so neither can supply a delimiter.

The code comment argued that `(*)` cannot reach the terminator because only `)` remains, which is true
and irrelevant: the body pattern `([^*]|\*[^)])*` consumes that `)` and runs to the next `*)` anywhere in
the unit. A regex argument has to be made about the maximum range the match can consume.

A diagnostic-free fallback made the defect non-local: an unmatched `(*` quietly became an ordinary token,
so whether it manifested depended on the compilation unit's `(*`/`*)` counts and order — one `@(*)` in a
file passes, two destroys, the diagnostic lands on the second, and it crosses file boundaries. A scanner
that accepts an opening delimiter must treat a missing close as an error.

`(*` is both an attribute opener and part of `@ (*)` (IEEE 1364-2005 A.6.5), so the lexer must look at
the previous significant token — one line on the token stream, and it covers `@(*)`, `@ (*)` and
`@ /* c */ (*)` alike. Guarding the opening side left the closing side using the adjacent `*` and `)`
inside `@(*)` as a terminator, so an unclosed attribute silently swallowed the next sensitivity list.

`attribute_instances_are_skipped_without_eating_implicit_sensitivity` was written for exactly this
boundary and did not defend it: each design had ONE `@(*)` and no following `*)`, so the regex failed
quietly and the fallback rescued it. A boundary test needs the minimum condition that breaks the
boundary — here, two — and that test's doc comment carried the wrong argument with the authority of a
comment.

### Alternative-store backends and routing (V1 slices 2, 2c, 2d, A2-i, A3-i, A3-ii-a, A3-iii)

Measured: `{d[0], x} = 8'hAB` — a heap chunk inside a concat lvalue — had been wrong since an early
slice, and the two tests containing that shape ran on the default backend and never reached the
alternative store. Neither instrumentation nor the corpus said anything; flipping the default and
running the whole suite was the only signal.

Where refusal and routing are both available, the one that spells an existing rule twice loses:
per-chunk routing needs a source-splitting rule that already exists inside the canonical funnel.

A neighbour pin for a reject row must not use a shape an earlier stage already rejects
(`{s, x} = …` with a string never reaches the gate, because elaborate refuses it first), or the row
claims the lower stage does the upper stage's job.

A source-scan pattern naming one family member (`sched.assoc_key_of(`) is a whitelist and missed
`assoc_str_key_of` twenty lines above; ask by prefix. And when the canonical implementation moves, the
old entry point is a second spelling one edit away from pointing at a different entry.

Recording a census projection per slice is cheap and correct, and the accumulation drifts when a slice
creates its own new reject rows (projected about 74.0% against a measured 72.75%).

A function that takes the alternative store as a parameter reads it only on the paths that use the
parameter: `builtins::dispatch` takes `nets` and only the formatter arms use it, while the rest build an
`EvalCtx` over their own `SimState` nets. Those arms were unreachable while every heap kind was refused
and became silently wrong the moment a row opened — measured, `d = new[n]` gives `size=0`, `s.itoa(v)`
gives `s=0`, and `q.push_back(a)` gives `q[0]=x`. A defect whose only discriminator is "the argument is
a net" is structurally invisible to differential rows built from literals: the neighbouring
`q.insert(i, 32'd99)` is a literal and was correct, and two suites shipped green.

The remaining bypass paths are pinned by COUNT per file plus the name of the row that blocks each, so
the next slice that opens a row breaks there first; a runtime assertion cannot do it, because unreachable
code has no call site.

A reject row's REASON is silently invalidated by a neighbouring slice: `k_queue_pop` said "the NetKind
scan refuses queue storage" and a later slice opened that scan, leaving `stmt_effect` as the actual
blocker.

Three test files used `string s; int q[$]` as "a refused design" and one slice admitted all three, so
the shape for a refusal pin has to be refused by BOTH gate halves under their own names and be far away
on the roadmap.

Instrumentation replaces eyeball auditing: `NetArena::heap[net]` plus `assert_owns(net, site)` (which
dies when called with a net it does not own) stood in for auditing sixteen indexing sites and named the
bypass exactly; the same instrument later pointed at a target at BUILD time.

A battery that records only "it died" is fooled by source-scan pins, which are change detectors rather
than behaviour tests — and `cargo nextest` is fail-fast by default, so a killer list gathered without
`--no-fail-fast` is just the first test to run. Three mutations appeared to be caught by pins alone and
only one really was; a mutation caught only by a pin means the behavioural row is vacuous.

A harness's hand-picked sidecar list has to be re-checked every slice: missing `queue_slice_stmts` and
`queue_bounds` means a slice is not a slice and a bound is not a bound, so that row was vacuous from the
day it was written. `NetReader` has 21 methods and tier-3 overrode 7; the remaining 14 defaults all
return plausible values (`None` making the caller X-poison, `false` meaning "not assoc", `xs`), harmless
only while the gate is closed.

A documented "next slice (+81)" was re-measured before starting and the row's standalone gain was +1: 80
of the 81 designs also hit the executor row and 40 of those also hit `fork`, and closing the pair is +37
at the cost of park/resume, window stashing and per-activity call stacks — while the same census showed
a different class at +160 for routing alone, which reversed the order.

A one-word reject row hides its distribution: `class` bundled twelve sidecars, so
`class C; int f; endclass` and constraint solving were refused by the same word, and the measured split
is 121 to 39 (plain OOP against CRV and virtual). The sidecars separate nothing, because all 160 designs
have `class_rand` and `class_vtable` — declaring a `rand` field or a method is enough. A table nobody
reads refuses nothing.

A planned reject row was measured before building and deleted: `resolve_virtual_call` reads only the
evaluated receiver handle VALUE and the shared table and reads no nets at all, and a three-level
inheritance design ran natively with three-way agreement.

Routing bitmaps are not all the same shape: heap and frame nets have wholly dead slots, and a class
handle's slot is half dead — the handle id is in that store and only the fields are in the heap — so the
question is `class[net] AND word.is_some()`.

This repository had treated classes as oracle-free since an early slice, and iverilog 13 supports SV
classes — half. Dereferencing a null handle SEGFAULTs `ivl` during compilation, and it gets virtual
dispatch wrong (IEEE §8.20, with vita's three backends agreeing with the LRM, so vita is ahead there).
The three were pinned to three different tests.

"Routing" lives in several places: when tier-3 began executing frame bodies, three answers were wrong in
turn — the write funnel (`write_routed`; the read side had routed since an earlier slice with no twin),
the specialised evaluator (`wprog::compile` resolving a `Signal` to a slot at compile time), and the
reader wrapper (`HeapRouted` routing only the heap, so the formatter was blind to frames). Each fix made
a different part of the output correct.

The first end-to-end check of that slice fell back to the VM with `buildable:false` and matched iverilog
perfectly. A precondition follows the EXECUTOR rather than the feature: the same subroutine body may not
name a net outside its window when DELEGATED (the engine's `&self` executor over the engine's flat
store) and may when DRIVEN through the kernel — applying the delegated precondition to the driven path
refused every design the slice targeted. A missing `else` in a `Return` arm fell through to
`k_rearm`+`Done`, ending the process on the first task return, and rustc said so as
"value assigned to `bb` is never read". Building the discriminator for two surviving mutations found a
real divergence instead: `frame_or_class_write` and `frame_write_lvalue` still resolved element
read-modify-write indices through the engine store, so `loc[sel] = src[fd]` landed in `loc[0]`.

One reject row can cover two executors: "does not name a net outside its window" blocked both
`run_frame_call` (an expression call) and `run_task` (a subset call statement), and threading only the
first and opening the row left the second silently wrong with the whole suite green — caught by the flip
run, for the third time.

A refusal design has to be checked for earlier-stage rejection: building the A3-iii refusal case as a
module-net assignment (`g = g + 1`) is refused by elaborate as E3009 and the test passes measuring
nothing; what actually reaches that row is a class field write, found with a probe.

A reject row's reason that is a claim about WHEN is invalidated by the next stage: `has_hier_call` said
"`Call.target` is a placeholder and cannot be pierced", which is true INSIDE elaborate (`force_suspend`
exists for precisely that) — and the predicate that reads the row runs in `simulate`, after the patch,
where instrumentation counted zero unresolved targets across every design that reaches the row. This file
recorded that same row three different ways: dead, live over 19 designs, gone. "This row is dead" expires
every slice — it had been called dead twice for two different reasons before a slice admitted
non-parking suspendable bodies and 19 designs landed on it alone.

When a mutation does not die, find who re-decides the value: A3-i's copy-in evaluates the actual at the
formal's width and sign per IEEE §13.4.3, and deleting the width, deleting the sign and zeroing
`k_frame_base` all passed, because `run_task`'s `bind_formal` re-binds each actual to the formal's
declared type on frame entry. Upstream context is observable only where a narrower evaluation would
already have destroyed the value, so the discriminator has exactly one shape: a formal wider than the
actual, with the actual overflowing its own self width (`wide(a + b, r)` where `200 + 57` is 257 in a
16-bit formal and 1 at 8 bits). That single row kills the width, sign and frame-base mutations together.
The mirror image: a copy-out loop reversal dies only in a row that aliases two output formals to one
destination.

Two A3-i survivors were unreachable rather than equivalent, and each was confirmed by building the
design: a call site with no sidecar always exists by engine time (measured: `missing_sidecar=0` through a
hierarchical enable), and every task with a dyn output formal is suspendable (`new[]` is a `SysTask` and
`o = i` is a handle-copy marker — both suspension signals).

### Removing gate rows (§4.5.337)

Measured: SVA is entirely desugared by elaborate, so what reaches the engine is ordinary IR plus two
`StmtId` tables, and both are already read inside the shared dispatch. The reject row was purely
conservative, and deleting one line bought 12% coverage. The parts that genuinely need machinery were
already refused under other names — `cover property` and liveness by the `final` row, deferred
assertions by their own — and a census split the row 760 against 14 with no overlap, so the slice is
one.

The only failure mode of removing a row is "designs that now run", so every neighbour is pinned WITH its
reason; a neighbour refused for the wrong reason lets someone later delete a load-bearing row.

Two mutations of the shared dispatch's suppression rule passed every engine gate and died only on a CLI
absolute pin, and the anchor built for them then also killed a harness mutation.

Without `assert_ctl` in the harness, `$assertoff` is just a printing `Display` — and both backends do
exactly that, so the corpus row is green while the design does not do what it says. The test for an
empty harness is to write a test pinning the feature's REFUSAL: if it returns `Ok(())`, the harness is
empty.

A mutation script restoring with `git checkout -- <file>` reverts to HEAD rather than to the pre-mutation
state and deleted two files of uncommitted slice work, producing two fake kills. And a survival says
nothing about the gate when the mutation is insufficient: deleting `a` from a reject row of
`a.len() + b.len()` leaves the row firing.

### Short-circuiting gates cannot answer "what does closing this buy" (§4.5.336)

Measured: flipping the default backend and running the whole suite is the cheapest coverage instrument,
far stronger than a corpus differential, and it is a measurement rather than an implementation, so it is
reverted afterwards. One run established that tier-3 already executed 54.7% byte-accurately and that all
three failures were tests asserting the default's name — a coverage verdict rather than a correctness
debt, which set the character of the next thirty slices. That verdict held: coverage reached 100.00% by
2026-08-16 (6,470 designs, 0 refused), and most of the correctness defects found on the way were
pre-existing elsewhere.

`SimOpts::default()` hard-coded `backend: Backend::Bytecode`, so moving the enum's `#[default]` alone
would have moved half the CLI while being called "the whole suite".

`writeln!` issues one `write(2)` per format fragment on an unbuffered `File`, so a per-test-process
runner tears rows — the line count inflated 1.85x. What caught it was not a person but a layout-aware
parser: the census script declares contamination when it meets a layer name outside the known set.

A short-circuiting verdict (design, then storage, then executor) cannot be logged for ordering, because
the layers behind the first refusal are never evaluated, so "how many designs pass if this row closes"
has no answer; the instrument evaluates all three independently while production keeps the short circuit.

Where two gates name the same feature twice, the unit of work is the SET: a `string` net is both a design
row and a storage refusal, so the design row fires 369 times with zero SOLE causes and closing one half
gains exactly zero. Marginal (greedy) and standalone gain are different numbers and both are needed —
subroutine frames are +545 alone and +712 after the other two, and `fork` and file-directed rows have
zero standalone gain, so "warm up on the small ones" is not available.

93.7% of the fallbacks were one CLI subprocess path, so the whole integration suite collapses into one
binary path and "N tests" is meaningless: the unit is a `simulate()` call. That the weight was not one
design repeated had to be confirmed separately (SVA was counted across 26 CLI test files and 400+
functions).

### The stop verdict can be written before the implementation (§4.5.335)

Measured: summing the profile share of what a planned stage targets gave about 6% (1.06x), so the
decision not to build a static wake mask could be made before building it — cheaper than building and
scoring, with the same conclusion. `settle_cont_assigns` at 9.1% reads as schedule cost and is mostly
the time spent evaluating the continuous assigns, which a static mask does not remove; the thing to
count is which LINES disappear. The hottest loop cloned an `Lvalue` per evaluation with nothing forcing
it — `ir` is a function parameter and is borrowed independently of `k` — and a clone added to satisfy the
borrow checker outlives the condition that needed it. And a plan resting on "parse plus elaborate is
about 85 ms (14%), so the ceiling is about 7x" measured 19 ms, 3.7% and a ceiling of 26.8x. With a
ceiling of 26.8x and the remaining three stages summing to 1.1x, the problem is the axis the plan aims
at, and the answer is to change the axis or close and record it.

### Count the ops before codegen (§4.5.334)

Measured: the tier-3 codegen plan was "inline leaf loads and arithmetic", and an execution census gave
arithmetic 1.5%, `Load` 39.3% (already a direct memory access) and 4-state rule function calls 30.8%.
The tier-2 JIT lost because "every leaf load is a CALL back into Rust", and tier-3's S1 had already
removed that reason by flattening storage into a `u64` buffer, so a preceding stage can consume a later
stage's justification. Unifying the 4-state rules into free functions was correct and makes those rules
undescendable to machine code: rewriting them in IR is a restatement of the semantics and calling back
is a per-op boundary, so "one spelling" and "inlined" cannot both apply to one function. 47.2% of
executions were one-op programs, surrounded by `Rc` clones, an IR walk, a 72-byte value construction, a
resize and funnel re-derivation — the target is what it takes to do one computation, not what is
computed. Six of seven shortcut mutations survived and all six were equivalent, with reachability
confirmed separately by a `panic!` probe; the equivalence proofs found a duplicate guard and a dead
argument. And the one non-obvious premise (reading a destination width from the slot rather than
walking the IR) was checked by the whole suite on every run with `debug_assert_eq!` rather than argued.

### A compiled representation is not speed (§4.5.333)

Measured: running tier-2's `CompiledBody` on tier-3 was a complete wash. The profile explained it: what
was removed (`k_resolve_lvalue_offsets` 3.1%, malloc 2.2%) was consumed by the new op loop. Where the
cost is inside the call rather than in choosing which call to make (`WProg::run` 20%, the write funnel
10%, the scheduler 8%), a representation change cannot reduce it. A concrete reason a compiled op
sequence can be SLOWER: the register file is an ABI, so splitting one statement into two ops sends a
72-byte value to memory and back, which `compute_effect` → `apply_effect` does not; fusing evaluation and
write where the destination is proven at compile time removed it (2000 ops to 1068, 93.7% of
assignments).

A comment was written into `vm_exec` saying a statement-boundary drain is required "or every diagnostic
moves behind the body", while a sibling file had already recorded of its own copy that deleting it keeps
the whole suite green and 25 designs byte-identical and that it should be treated as an unverified
backstop — and a mutation refuted the new sentence. Consuming `k_call_fatal` per op rather than per
statement returns between `ResolveOff` and `WriteLval` and loses both the write and the E4002 it owed;
the op sequence carries no boundary marker, but "which op ends a statement" is a fact the lowering knows
and can be restored with a `_`-free predicate. `is_codegen_able` says it sees "any expr position that can
REACH a frame Call" and does not look at lvalue index expressions, so `mem[f(i)] = 1` passes — a real
divergence in tier-2. Two of nine survivors were equivalent with a one-line reason
(`NbaLhs::of([c])` is `One(c.clone())`, so the queue entry is the same); a cost-only specialisation is
pinned by an op-mix census rather than a value differential. And 30 of the corpus's 72 designs run a
compiled body at all (the rest have a `Delay`), so "72 designs agree" hides 42 comparisons of a walk with
a walk.

### Writes that live in a side table (§4.5.278)

Measured: `compute_suspendable_tasks` asks whether a statement writes outside the frame window
`[lo,hi)`, and a call's copy-out destination is not in the statement — `Terminator::Call` carries
`{target, ret_bb}` and the destination lives in `task_calls_func`, keyed by the call block's global id.
The walk had never seen it, so `inner(a, gv)`'s caller stayed "subset" and the synchronous `&self`
executor wrote a module net and died at rc 101 with no diagnostic.

"The neighbouring statement decides the answer" recurred: `#5 inner(a, gv);` and
`if (c) inner(a,gv); else gv = 0;` both worked BEFORE the fix, the first because the `Delay` terminator
made the task suspendable and the second because the `else` arm's own out-of-window write did — for
unrelated reasons, exactly as an earlier slice's `$display` case.

Elaborate's and the engine's `TaskCallInfo` are different structs, so two reduction loops make a
pure-function contract depend on two expressions agreeing; one reduction function
(`sim_ir::call_out_nets`) makes it one expression's property. Their input sizes legitimately differ
(elaborate runs before resolve, so a deferred hierarchical enable is missing from its map), and treating
a missing entry as a conservative signal makes the two sets diverge — the correct answer is "a missing
entry is not a signal", because those callers are already force-suspended on both sides by
`FuncMeta.has_hier_call`.

Removing a loud exposed something unrelated to calls: allowing `s[i] = f(a, o)` in a frame body made the
string silently not change, because `SysTaskId::StrPutC` writes `dyn_heap[net]` unconditionally while a
frame-local `string` is slab-stored in the frame slot. It reproduces with no call and no output formal
at all (`string s; s = "zz"; s[0] = 65;`) and is pre-existing. `read_net` branches on
`frame_local`/`dyn_is_handle` and only this write did not ask.

A performance report's stated root ("deep combinational cones are not levelized, so the cost is
depth-squared") was refuted by the oracle: on a depth sweep with total work held fixed, iverilog scales
at least as steeply (4.15x against vita's 3.55x). That cost is a property of interpreted event-driven
simulation, and the comparison point is a compiled simulator that levelizes at compile time. Ordering was
eliminated by experiment: ascending and descending batch sorts were identical.

A one-word fast path for `Value::resize`/`mask_top` was added on the strength of a 19% profile
attribution and measured 0.190 → 0.188 s, so it was discarded — a second code path is a drift risk in
itself, and a profile attributes inlined code to the enclosing symbol, so reading 19% as call overhead is
wrong.

### Performance conclusions have a scope (§4.5.282)

Measured: after landing 11 items and refuting 9 on PicoRV32 alone, "the headroom for constant-factor work
on this representation is exhausted" was written down. Round 25 refuted it on the STRING axis (172x) and
round 26 on the CALL axis (10.7x). Both times the benchmark's shape set the conclusion's scope and the
scope was not in the sentence.

"The bottleneck moved to X" is a location, not a cause. A reporter said the gap was string handling
rather than RTL, and the location was right while the cause was an algorithmic defect; the next round said
"now it is the DUT", and the location was right while half the cause was zero VM coverage. Given a
location, build a discriminating question and measure it: round 26's was "on realistic RTL, are we level
with a baseline in the same tier?", and the answer was "level on that design and 2.6x behind on another
shape".

doc-18 had long recorded that the gap size is unknown because the project does not own VCS or Xcelium.
verilator is free and is in that tier; `brew install verilator` put three tiers side by side on the same
design and machine and fixed the tier-3 plan's budget for the first time.

Coverage is a different axis from speed, and zero coverage is only visible when the benchmark changes:
the bytecode VM is the default backend and 5.5x faster than iverilog on its home ground, and
`is_codegen_able` refuses any process calling a user function wholesale, so its contribution on Keccak is
exactly zero (`--backend interp` and `bytecode` take the same time).

Keccak-f[1600] was added with four oracles — a Python reference, vita, iverilog and verilator — all
producing the same digest, and that digest equals the published reference value `f1258f7940e1dde7`.
Mutual agreement alone allows four tools to be wrong together.

### Loop-file rules moved into the code base (2026-08-03)

Recorded: measure capability parity before unifying or routing storage classes, because "the newer
representation is better" is often false on some axis and unification silently regresses that axis, so
where neither dominates the answer is an additive extension. Read the engine code when a gate assumes
"the engine cannot do X" — there is at least one case where the fallback path already handled the shape
and only the gate under-approximated, and a discriminator that uses the SAME SET as the storage it
drives makes "cannot admit" structural. Verifying a mapping several times over the code path says nothing
about execution ORDER, and an order-sensitive probe must be built where mapping order and execution
order differ (descending, for example) or it is vacuous. And cut a REJECT gate from a hazard set measured
on a PRE build, never from a proxy: a gate built on "the names collide" false-rejected byte-correct
designs in bulk, and a reviewer's proposed gate predicate is subject to the same measurement.
