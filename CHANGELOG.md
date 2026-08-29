# Changelog

All notable changes to **vitamin** are recorded here. The format loosely follows
[Keep a Changelog](https://keepachangelog.com/); dates are ISO-8601.

Per-slice engineering detail lives in [docs/ROADMAP_ARCHIVE.md](docs/ROADMAP_ARCHIVE.md)
(§4.5.x) and [docs/DEVLOG.md](docs/DEVLOG.md); the sections below summarise what
changed for a user of the simulator.

## [Unreleased]

### Fixed

- **Nothing shipped for the `case` collective-signedness gap, and the reason is the
  finding.** §12.5 makes a `case` comparison unsigned as soon as one participant is, and
  that unsignedness must reach the scrutinee's own operators. vitamin applied the rule by
  wrapping the scrutinee in `$unsigned`, which stops at the wrapper — `$unsigned`'s
  argument is self-determined, and all four tools agree on that — so a `>>>`, `/` or `%`
  inside the scrutinee kept its signed reading and `case (b >>> 2)` took the wrong arm.

  ⭐ The fix was found and it needed no new machinery: re-lower the scrutinee through the
  sign-carrying context lowering §11.8.1 size casts already use, passing the scrutinee's
  own width so it borrows only the sign half. Six `case`/`casez`/`casex` cells matched both
  oracles, and `/` and `%` came along.

  ⚠️⚠️ **Reverted after four BLOCKING defects on one axis, every one created by the fix
  itself.** Replacing the wrapper instead of keeping it as the fallback dropped the rule
  entirely for scrutinees the new lowering cannot take. A signed function-call LABEL voted
  "unsigned". The predicate written to stop that voting drifted from the whitelist it was
  supposed to mirror — ⭐ the exact drift its own docstring warned about, and both review
  lenses found it independently. Fixing that surfaced a `$bits` label going from right to
  wrong, because `$bits` returns a signed `int` in SystemVerilog and vitamin folds it to an
  unsigned constant.

  ⭐⭐ The root is one sentence: the collective vote rests on `expr_self_signed`, whose
  "unsigned" is a DEFAULT rather than a fact for calls, non-whitelisted system functions,
  and constants folded from them. Making that vote load-bearing is the prerequisite, and it
  is not met. The prerequisite is now written into the queue with the measured cells, the
  fix shape, and the four defects, so the next attempt starts where this one stopped rather
  than rediscovering it.

- **`>>>` filled with the sign bit whenever its LEFT OPERAND was signed; IEEE 1800
  §11.4.10 says the fill follows the RESULT TYPE.** §11.8.1 makes that type unsigned as
  soon as ANY operand of the surrounding context-determined region is, so an unsigned
  comparand or addend next to the shift demotes it to a logical shift. A 303-cell census
  against both oracles put **70 cells** on the wrong side of it, with **zero oracle
  split** — the two tools never disagreed, so there was nothing to arbitrate:

  ```text
    reg signed [7:0] b = 8'shB3;   (b >>> 2) + 8'd0
      was 236          iverilog 44          verilator 44          exit 0
  ```

  ⭐ The unsignedness arrives from an operand the shift does not contain, which is why
  every part of the shift looks signed when you inspect it alone, and why the neighbouring
  spellings (the shift by itself, against a signed comparand, under `$signed`) are all
  correct. The arm read the left operand's own signedness under a comment asserting that an
  unsigned context *"MUST NOT demote a genuinely-signed `s >>> n`"*. Demoting it is exactly
  what the LRM requires.

  ⚠️⚠️ **And fixing it uncovered a second, larger rule.** Both frame call funnels evaluated
  an argument with the FORMAL's declared signedness as the context sign. §13.5.1 makes an
  argument bind an assignment, §11.8.3 lends it the formal's WIDTH — but §11.8.1 then says
  an expression's signedness does not depend on the left-hand side. Measured under
  iverilog, the formal's declared sign has **zero** effect: `fu16(8'shf7)` and
  `fs16(8'shf7)` are both `fff7`. That had been wrong from the start, but the old `>>>` arm
  ignored the context sign entirely, so shifts were accidentally immune; making the arm
  correct removed the cancellation and turned it into a wrong VALUE
  (`au(b >>> 2)`: 4294967276 → 44). The adversarial review graded that BLOCKING and it is
  fixed, which also closed a documented gap (`ff(8'shf7)` now reads `0000fff7` like
  iverilog rather than `000000f7`) and three pre-existing cells beside it.

  ⚠️⚠️ **THREE funnels, found one per review round.** After two were fixed, the soundness
  lens ran a twelve-site census and concluded "two needed the change and both got it" —
  and the DIFFERENTIAL lens found a third by measurement: a task called from inside another
  frame body, and only when the callee is non-suspendable, so one `$display` in the callee
  routes it to an already-fixed path. A census that enumerates sites can miss one selected
  by a property of the CALLEE; a sweep that varies the design finds it. That is the whole
  argument for requiring both lenses.

  Round 3 re-measured everything: **~9,200 cells, zero regressions, zero backend splits,
  the corpus byte-identical**, with the routing varied over automatic/static, task/function,
  `#delay`/`wait`/`fork`/`$display` bodies, nesting depth 1–4 mixing suspendable and
  non-suspendable callees, class and virtual methods, package and hierarchical calls, and
  every caller context.

  ⭐ A side ruling came out of it: for a signed DEFAULT argument, iverilog answers the same
  default two different ways depending only on whether the subroutine is a function or a
  task, while its own plain-assignment twin agrees with the task half. It is disqualified
  there by its own neighbouring answers; vitamin now matches verilator and is internally
  uniform where it was not. Pinned as a ruling in `oracle_split_rulings.rs`.

- **The inline function fold re-ran a body-local's defining expression once per
  REFERENCE.** vita has two lowerings for a user function; the inline one substitutes a
  local's defining RHS at every place the local is named, and the arena keeps ONE node
  that the evaluator then walks once per reference. Same defect as a shared `case`
  scrutinee and an eager `&&` right operand: a DAG walked as a tree.

  ```text
    reg u; u = $random;  g = u ^ u;    vita 3533466533   iverilog 0   exit 0
           u = $urandom; g = u ^ u;    vita 2354591315   iverilog 0   exit 0
  ```

  ⭐ A function reaches the inline path when it is not `automatic`, its return type is
  4-state (`function [31:0]` — `function int` is framed by `ret_two_state`, whether or not
  `automatic` is written), its body is straight-line, and it has no unpacked formal. That
  is not obvious and cost three failed reproduction attempts.

  A body that assigns a local from a NON-REPEATABLE RHS and then names it twice or more
  is now routed to the FRAME path, which binds the RHS to a slot once. Frame ⊇ inline in
  capability since §4.5.198/199, so this is routing rather than new machinery, and the
  review measured **6 shapes moving loud → correct** as well — `assign y = f(xin)` with
  `$random` in the body was an `F4016` delta-limit **fatal** before. The strongest single
  measurement is that six consecutive `$random` draws interleaved with a testbench's own
  now reproduce iverilog's entire stream, which shows one draw per call in the right
  order rather than an XOR that happens to cancel.

  ⚠️ Adversarial review found **three BLOCKING defects in the first version**, all mine.
  ⓐ `if let Blocking { lhs, rhs, .. }` swallowed `delay`: `body_needs_frame` ignores an
  intra-assignment delay and `fold_straight_line` deliberately ACCEPTS one with a warning,
  but the frame path refuses it — so `u = #3 $signed(a); fn = u+u;` went from `r=42` (the
  oracle's answer) to E3009 exit 1. **correct → loud.** The guard has to be body-wide, not
  per-statement, because the delayed assignment need not be the non-repeatable one.
  ⓑ Repeatability was judged on surface syntax where it has to be judged AFTER
  substitution — `u = $random; v = u; g = v ^ v;` gives `v` the same impure node, and
  seven of a 26-cell census survived through `+`, `{}`, `&`, `[m:l]` and a bare copy.
  ⓒ `walk_expr_refs` had no `Cast` arm, so `int'(u) ^ int'(u)` counted zero references;
  both siblings in the same file already had it.

  ⚠️ Two shapes of the same defect are NOT closed: a side-effecting callee firing once
  per reference, and an out-of-range element read reporting once per reference. Both
  bodies read something non-local, and the routing is gated on `body_reads_only_locals`
  because a framed call contributes only its ARGUMENTS to an implicit `always_comb`
  sensitivity list — routing them would drop a read from that list, which is a different
  silent-wrong. The gate is shared with two other routing clauses, so widening it is not
  a local change. Recorded, with a test pinning today's behaviour so whoever lifts the
  gate sees it go red.

  ⚠️ **Performance is not the reason.** Inline is Θ(refs^depth) and frame is Θ(refs·depth)
  — measured, fitted base 1.99 / 2.98 / 3.95 for 2 / 3 / 4 references, 27–29 ns per node
  visit, with the arena growing strictly LINEARLY (+20 bytes per level) while the visit
  count explodes: 774 bytes of arena and 305,831 visits per call at 4 references and depth
  8. But a census of all 203 `function` blocks in the corpus found 27 on the inline path
  and **25 of them have no body local at all**, so the corpus win is zero.


- **A block-local with the same name as a module-scope net was the module net.** vita
  flattens procedural block-locals into the module namespace, and the guard that makes
  that safe — a width/signedness check, a definite-assignment check, and
  `elaborate_netvar_decl` itself — sat behind one early `return` taken whenever the
  name also existed at module scope. Its comment said the case was "handled by the
  struct/enum/typedef shadow-scoping". It was not. So a shadow created no declaration
  at all and every reference resolved to the shadowed net, at exit 0.

  ⭐ One token separates the two behaviours: rename the MODULE net and the identical
  design is loud. A 46-shape census found **22 silent-wrong cells**, unanimous against
  both oracles:

  ```text
    logic[15:0] over logic[7:0]    val=ef  bits=8    both oracles: beef / 16
    real over int                  3.000000          both: 2.500000
    int signed over int unsigned   4294967293        both: -3
    enum over logic[1:0]           x=1 name=RED      both: x=5 name=GRN
    int x[0:3] over logic[7:0]     arr 1 1           both: arr 5 9
    read before write              55 (leftover)     both: 0
    write inside the block         module x=99       both: module x untouched
  ```

  The last row escapes the module: the shadowed net is readable by a hierarchical
  reference from another module and visible in `$dumpvars`, so a block that only ever
  named its own local was rewriting its parent's state, and the waveform showed the
  parent's net changing with no var for the local at all.

  The fix reuses machinery already in the tree. The function/task path (`$func$`) and
  the generate path both give a local a distinct key and were already correct; a shadow
  now earns a `$blk$<span>` scope the same way `automatic` and dynamic-storage locals
  have since §4.5.249. The condition that disqualified it — `module_names.contains(name)`
  — is now what QUALIFIES it, on ONE declaring block, because a shadow has nothing to
  coalesce with. `walk_scopes_key` already treats `$blk$` as transparent, so every other
  name in the block still falls through to the enclosing module net.

  ⭐ The same move closes the reference from OUTSIDE the block. That gate exists because
  "vita keeps a body's block-locals in a FLAT per-body table, so a reference outside the
  declaring block would silently resolve to the block-local" — false for a name that is
  not in the flat table. It is keyed on `scoped_block_locals`, which covers module
  process bodies only, so a FRAME body keeps both its flat table and its diagnostic.

  Census after: **40 of 46 match an oracle, 0 silent-wrong, 5 loud** (all
  non-shadowing sibling coalesces plus a parser gap). Eight shapes moved loud → support
  as well: a shadowing `string`, a shadowed `wire` (which reported E3018 and blamed the
  user for a net assignment they had not written), a larger local array over a smaller
  module array (E4002 twice at runtime), an `automatic` colliding with a module net, and
  a dynamic array shadowing a dynamic net.

  ⚠️ **Ten existing tests asserted the refusal this removes**, six of them named
  `*_is_loud` / `*_stays_loud`. Every one was re-measured against both oracles before
  being rewritten: all ten designs now produce the oracles' answer, so each is a value
  assertion now, and the two that are still loud say which guard actually fires — one
  of them passes for an entirely different reason than it was written for. ⚠️ One of the
  ten was a cell written in this same change, one edit before the gate it pinned was
  removed; its docstring claimed the scoping "does not teach the OUTER reference to
  resolve", which was a guess about machinery that had needed no teaching.

  ⚠️ The dynamic-pair diagnostic also listed "AND the name does not also name a net in
  the enclosing scope" among the conditions for distinct storage. That clause is now
  false and has been dropped. Recorded, not fixed: vita spells the waveform scope
  `$blk$<span.lo>` where iverilog uses the block's label — a pre-existing convention,
  but this change makes it visible on far more designs.

- **`&&` and `||` evaluated their RIGHT operand even when the LEFT one had already
  decided the result.** IEEE 1800 §11.4.7 evaluates the right operand only when the left
  does not determine the answer. vita's generic evaluator was an ordinary eager tree
  walk — `let l = self.eval(lhs); let r = self.eval(rhs);` — so a guard idiom did not
  guard. The operator's own RESULT was right in all 47 census cells; the damage was
  entirely in state the skipped operand should never have touched:

  ```text
    0 && (c.bump()==1);   then c.cnt      vita 1              ivl 0  verilator 0
    1 || (c.bump()==1);   then c.cnt      vita 1              ivl 0  verilator 0
    0 && ($random != 0);  then $random    vita -1064739199    ivl 303379748
    0 && (mem4[9] != 0);                  vita E4002, exit 1  both exit 0
    0 && f(1)   where f $displays         vita prints         neither oracle does
  ```

  ⭐ The `$random` row is the sharpest: vita's CONTROL draw is byte-identical to
  iverilog's, since the two share an LCG stream — so vita's test draw was provably the
  SECOND draw of that same stream while iverilog's was still the first. A skipped
  operand had consumed a random number. And the fourth row means the canonical
  `if (i < 4 && mem[i])` bounds guard reported an out-of-range read and exited 1.

  The reference implementation was four lines above the defect: the `Ternary` arm
  reaches a branch through a closure, so a branch runs only where the closure is
  called. `&&`/`||` now do the same. ⚠️ The deciding question is the operand's TRUTH
  VALUE, not a bit — `4'b01x0 || f()` skips (a set bit determines it) while
  `4'b00x0 || f()` must not (`Tri::Unknown`), and both oracles agree on both — so the
  predicate is written over `Tri`. Skipping is sound because the table row is constant
  across the right argument there, and rather than restate that answer the deciding
  truth value is fed back in as the right argument, so the one `log_bin_tri` table
  still produces it.

  The two compiled lanes (`native/wprog.rs`, `native_eval/`) have no control flow and
  keep evaluating both operands. That is the same VALUE — the table is total — but not
  the same DIAGNOSTICS, so both now DECLINE an `&&`/`||` whose right operand compiled to
  an indexed element load, which is character-for-character the guard their `Ternary`
  arms already carried, asked of the compiled OPS rather than of the expression shape.
  Only the right operand is guarded; the left runs on every path.

  ⚠️ The 6,240-test suite passed unchanged both before and after. Nothing in it
  evaluated a side-effecting `&&` operand, which is why 16 new cells exist. Adversarial
  review found 0 BLOCKING across ~93,000 differential cells and a nine-item code-path
  census; three NITs were documentation the change had made false, including
  `log_bin_tri`'s own doc comment still asserting *"neither operator short-circuits
  here and neither does the generic evaluator"* — on the function this change had just
  pointed `wprog.rs`'s header at.

  ⚠️ Two measured limits, recorded rather than closed. A statement whose `&&` right
  operand is an unpacked-array read at a RUNTIME index now falls out of both compiled
  lanes and costs up to 4.8× — its neighbours are unaffected, values are correct, and
  corpus demand is zero, but `if (valid && ram[addr])` is not an exotic shape. The
  decline is also invisible in `run.json`: `codegen.able` counts PROCESS BODIES and this
  is EXPRESSION admission, so observability reports full coverage on a design that lost
  it. Separately, the elaborate-time constant folders still evaluate both operands, so
  `localparam OK = (DIV != 0) && (100/DIV > 3)` is still E3009 where both oracles fold
  it to 0 — pre-existing and always loud, but it means the file that cites §11.4.7 is
  now the one that does not implement it.

- **A `case` evaluated its expression once per ARM TESTED — and, with no arm to test, not
  at all.** IEEE §12.5 evaluates the case expression exactly once. `lower_case` built its
  cascade from one shared scrutinee node, which is right in the arena and wrong in the
  evaluator: a shared node is walked as a TREE, so the expression ran again for every
  comparison. Measured against BOTH oracles, vita was wrong in two directions at once:

  ```text
    case (f(n)) 0: 1: 2: 3: default:   n=3    vita E×4   iverilog E×1   verilator E×1
    casez / casex, one arm before the hit     vita E×2   iverilog E×1   verilator E×1
    0,1,2: / 3,4,5:  (multi-label arms)       vita E×6   iverilog E×1   verilator E×1
    case (f(n)) default:  (no Match arm)      vita E×0   iverilog E×1   verilator E×1
  ```

  ⭐ The last row is why this was not a repetition count: with no Match arm there is no
  comparison, so the scrutinee was never evaluated at all — a `$display` inside it did not
  print, and the `E4002` an out-of-range read owes did not fire. The scrutinee is now
  captured into a temp before the cascade, which makes the count exactly one in every row.

  A runtime range report from the scrutinee now anchors on the READ rather than on the
  `case` keyword, which is also more precise than before the change. The capture carries
  the scrutinee's signedness (an unsigned capture would silently take a different arm,
  since each `CaseEq` pair is sized from it), is 4-state (`casez`/`casex` match on its own
  x/z bits), and is taken after §12.5's common-maximum width pass. It reuses the
  `$ia_tmp$` sigil so the existing VCD/FST filter covers it — no synthetic net reaches a
  waveform.

  Frame bodies are covered too, by a different capture net. A frame body cannot write a
  module net — `frame_write_lvalue` is `&self` and reaches only the activation window — so
  its capture is one of the frame's own slots, reserved by a new `reserve_frame_case_tmps`
  pass before the window closes. That is the same span-keyed handoff `repeat` already uses
  for its per-activation counter, and for the same two reasons: a frame's locals are a
  contiguous net-id range that closes before lowering begins, and "the Nth case in each
  walk" is exactly the agreement that drifts silently. The reservation runs over the AST,
  where nothing has an IR width yet, so it reserves at a placeholder width and the lowering
  patches width and signedness — which cannot move the window, since that window is a range
  of net IDs. A lookup miss degrades to the pre-existing per-arm re-evaluation rather than
  to a shared module net.

  ⭐ **The frame half is where the cost was.** `bench/keccak`'s `keccak_f.sv` spends 63.8%
  of its run inside `run_frame_call`, and 79.7% of that was branch conditions — two `case`
  statements of 24 and 25 arms re-evaluating their scrutinee up to 25 times per call.
  **7.06 s → 5.24 s (−25.7%), digest unchanged.** Together with the compiled-body change
  above, `keccak_f.sv` is **8.11 s → 5.24 s (−35.4%)**.

  ⚠️ Real and string scrutinees are still skipped (the capture net would need
  `NetKind::Real`/`String`) and neither shows a measured difference today.

  Corpus 8/10 with every pinned digest matching, `examples/` 4/4 byte-identical, 6,240
  tests green. Adversarial cells over the frame path all match Icarus: recursion, two
  concurrent task activations that each suspend inside the arm they chose, a nested frame
  case, `casez` over an x/z scrutinee, and collective signedness.

- **⚠️ BREAKING: two drivers on one variable is now an ERROR (`VITA-E3001`).** IEEE
  §9.2.2.2 says a variable written by `always_comb` may have no other driver, and a
  declaration initializer is one — so `logic rdy = 1'b1; always_comb rdy = …;` is a
  multiple-driver design. vita reported it as a warning and ran anyway, which meant the
  design was green in the development loop and stopped xrun's elaboration (`*E,MULAXX`)
  at sign-off. It now stops the run. Measured on the workload corpus: **8/10 unchanged,
  every pinned digest matching, zero designs newly rejected**; `examples/` 4/4 unaffected.

  ⚠️ **`always_comb` only — `always_ff` and `always_latch` are NOT included, and an
  external report asking for them was measured and declined.** The report's ground was
  that the clause "does not vary by block kind". It does. One kind per file, verilator
  5.050 `--lint-only -Wall`: `always_comb` gives **MULTIDRIVEN** (citing IEEE 1800-2023
  §9.2.2.2), while `always_ff` and `always_latch` give **PROCASSINIT only** — a style
  note, not a driver ruling. iverilog says nothing about any of the three. The split is
  what the rule is for: `always_comb` models combinational logic, whose output must be a
  function of its inputs at all times, so any other write destroys the property the
  procedure asserts; `always_ff` models a REGISTER, and a declaration initializer is that
  register's power-on value. `logic [7:0] c = 0; always_ff @(posedge clk) c <= c + 1;` is
  the ordinary FPGA initialization idiom.

  ⭐ The widened version was built and reverted, and the thing that refuted it was this
  repository's own `obs_procs` fixture — written in exactly that idiom, and it stopped
  elaborating. **A working test design breaking is evidence AGAINST a new rejection, not
  for it.**

  ⚠️ Plain `always` is out too, for a different reason: `logic clk = 1'b0; always #5 clk =
  ~clk;` is the clock generator every testbench has, and the clause reaches only the
  inference procedures. So are `initial` and `final`.

  ⚠️ **Promoting a diagnostic is the one ladder move that can DESCEND**, so the false
  positive it had was closed first. The detector is the definite-assignment write walk,
  which over-approximates on purpose (name-based; an unresolved call writes everything)
  because it was built for accept gates — and it flagged a module variable that nothing
  writes when an `always_comb` declared a block-local SHADOW of the same name. A procedure
  declaring its own `name` is now skipped for that name; a write reaching the variable
  through a task's `inout` actual is still a driver.

  ⚠️ **There is no opt-out.** `-Wno-<CODE>` suppresses Warning/Info diagnostics only
  (measured: `-Wno-E3001` leaves the error standing), so a design with two drivers on a
  variable stops elaborating. That is the same answer xrun gives.

- **A hierarchical reference could not name a generate block, unless you spelled the index.**
  A singleton generate scope (`if` / `if…else` / `case` / bare `begin : g`) is stored as
  `g[0]`, and the bare-label mapping onto it ran for the LEADING path segment only. That is
  fine for a same-module reference, where the block IS the leading segment (`gblk.x`), and
  exactly wrong one dot further out: `u.gblk.x` commits `u` as the scope and then looked up
  the literal `u.gblk.x` while the net lives at `u.gblk[0].x`. 19 census cells — all four
  spellings × net / localparam / instance-inside × read / write, plus depth 2 — E3010 in
  vita, correct in Icarus.

  ⭐ The report said "generate blocks"; the measurement said one axis. `for`-generate
  references (`u.g[0].x`) already worked, so did the indexed spelling of a conditional
  block, and so did the bare spelling from inside the same module — the last of these has
  been pinned and green since the initial commit, which is why the cross-instance family
  stayed invisible.

- **A named generate-`case` block minted no scope at all.** The parser's case-item arms
  called `parse_gen_branch().1`, keeping the items and discarding the label, where the `if`
  and `for` arms bind it. Its members therefore landed in the ENCLOSING scope: `u.g.x` and
  `u.g[0].x` were both E3010 (the one generate kind no spelling could reach), and when the
  name collided with a parent declaration the design was rejected outright with E3009
  `redeclared` though Icarus and Verilator both run it. The labelled body is now re-wrapped
  as the `GenItem::Block` the elaborator already knows how to scope, so `label[0]` has one
  spelling and no AST field was added.

  ⚠️ A bare label on a `for`-generate stays loud at EVERY trip count (§27.4 makes those
  blocks an array). The one-trip case is the trap: it leaves exactly the `g[0]` a
  conditional block leaves, so a fallback keyed on storage alone accepted it and would have
  begun failing the day a parameter moved from 1 to 2.

  `format_version` unchanged at 29 — the loop-label record is elaborate-side and never
  serialized.

- **A 2-state cast paid for bits its operand did not have.** Round 35 stopped nested casts
  from multiplying; a single cast still expanded to one `CaseEq` per bit of the **target**
  type, so `int'(nb)` on a 4-bit `nb` built 32 terms of which 28 selected bits of a known
  zero-extension. A widening cast now coerces at the **operand's** width and extends
  afterwards — provably the same bits (an unsigned extension is literal zeros, a signed one
  replicates a sign bit that coercion has already forced to 0 or 1).

  Measured on the reporting design's own repro, release, interleaved: **69.6 s → 6.7 s
  (10.4×)**, stdout byte-identical. Fan-out for a 4-bit operand: `int'` 32→4, `longint'`
  64→4, `shortint'` 16→4. The two no-cast controls did not move, which is what shows the
  win is the cast and not a global effect.

  ⚠️ **The reorder introduced a silent-wrong and it was caught by measuring its own
  premise.** The equivalence argument says "the operand's width", but the code asked
  `ir_bits_of(e).unwrap_or(32)` — and a deferred hierarchical reference has no width yet, so
  `longint'(u1.w40)` on a `logic [39:0]` silently dropped its top 8 bits at exit 0. **A green
  6,185-test suite and a green 90-cell three-way sweep both passed over it**, because every
  operand in both had a declared width. Now gated on the width being *known* and pinned.

  ⚠️ Still open, with the numbers, in ROADMAP §2: a same-width `int'(f())` names `f` 32 times
  against Icarus's 1. This removes paying for bits the operand does not have; it does not
  remove paying per bit.

- **A 2-state cast named its operand once per declared bit.** `int'(e)` is lowered into
  a `Concat` of one `CaseEq(Select(e, i), 1'b1)` per target bit, and the engine walks
  that DAG as a tree — so an unguarded `keyword'(e)` cast multiplied the cost of
  evaluating `e` by the cast's declared width, and nesting multiplied again. Counted
  exactly, by putting a `$display` inside the operand:

  | cast | operand evaluations, before | after | Icarus Verilog 13 |
  |---|---|---|---|
  | none | 1 | 1 | 1 |
  | `byte'` | 8 | 8 | 1 |
  | `int'` | 32 | 32 | 1 |
  | `longint'` | 64 | 64 | 1 |
  | `int'(int'(x))` | **1024** | **32** | 1 |

  ⭐ The discriminator is 2-state-ness, not width: `integer'` and `int'` are both 32-bit
  and signed, and differed by **27×** in wall clock on one triple-nested `always_comb` —
  `integer` is 4-state, so no coercion is built for it at all. At around five levels of
  nesting, `velab` **alone** ran past 60 s on a twenty-character expression.

  The fix is the guard the sibling coercion site already had: the inline path's
  formal binding calls the same routine and gates it on "can this operand actually carry
  an x or z", with a comment recording the same measurement. The cast path simply never
  got it. Measured end to end on the reporting design, release, interleaved:
  **8.6×–542× faster** with byte-identical output, and 64 cast cells over x/z-carrying
  operands print identically before, after, and under live Icarus Verilog.

  ⚠️ **This does not make the evaluation count correct**, and the gap is recorded rather
  than glossed. A single `int'(f())` still calls `f` 32 times where Icarus calls it once,
  because a call is conservatively treated as possibly-unknown; a widening cast over a
  call still fans out to the wider width. What was removed is the *multiplication* under
  nesting. If your design casts an expression with a side effect — `int'($random)` is the
  canonical one — the count is still wrong; see ROADMAP §2.

- **`--obs-procs` profile rows of `kind: "port"` had no `file:line:col`.** On a reporting
  design that was 1,267 rows and 51% of all evaluations — the largest category in the
  profile, and the one a reader could not act on, since `scope` only narrows to an
  instance and one instance can carry dozens of ports. Each row now reports the location
  of its port connection in the parent's instantiation, **including the column**, so
  several connections written on one line are told apart. A `.*` wildcard connection
  still reports `("", 0, 0)`: it synthesizes one connection per port from no source text
  of its own, and pointing it at the nearest real token would be a location that does not
  survive being followed.

- **`--obs-procs` and `--obs-procs-time` were missing from `vita --help`.** They worked
  and were documented in the manual, but a reporter reading `run.json`'s `"processes":
  null` concluded the feature was unimplemented. A third flag, `--probe-file`, was
  missing too, and four rows were absent from the manual's flag table. A new test now
  extracts the flag literals from the argument parser's own match arms and asserts every
  one appears in `--help`, so the next undocumented flag is a red test rather than
  another external report.

### Changed

- **The compiled expression lane refused a signal read whenever the context's signedness
  differed from the net's, at equal width — where the bits are the same either way.**

  ⭐ A corpus-wide representation census came first, because the premise needed testing:
  VCS is a 4-state simulator and it is fast, so "4-state is why we are slower" cannot be
  right — 4-state doubles the DATA. Of vitamin's 72-byte value, **16 bytes are the 4-state
  data**; the other 56 are width, signedness and flags, which a compiled simulator bakes
  into generated code as literals. Counting every value the evaluator returns across all
  eight running workloads, **83.9% to 100% are simultaneously definite and at most 64
  bits** — geometric mean 95.7%. That is exactly the shape the compiled lane already
  carries (16 bytes; 8 in its two-state lane). The representation is in the tree; the gap
  is coverage.

  So: a fresh execution-weighted decline census, which put **6,600,872 requests from
  exactly two expressions** on one gate in `darkriscv` — 92% of that design's declines.

  ⚠️ Dropping that gate's sign half for EVERY node was built, measured sound, measured
  1.00× and reverted once before — over picorv32 and keccak, which the record says.
  `darkriscv` was not in that pair, and this corpus exists to break exactly that trap.
  Only the LEAF exemption is taken here, on the ground that the arm never ASKS: no exit of
  it reads the signedness, directly or through a mask.

  ```text
    picorv32 -3.4%   darkriscv -1.6%   biriscv -1.6%   serv -1.2%   sha256 -0.9%
  ```

  every pinned corpus digest unchanged. ⭐ The corpus deltas understate the lane: on hot
  single-shape designs the review measured **4.8×** for a mixed-sign expression tree,
  **2.0×** for `signed ^ unsigned` and **2.3×** for a signed memory read through a runtime
  index. Those shapes are a minority in the corpus, not in the lane.

  Adversarial 2-lens review: no BLOCKING. The differential lens was CLEAN over **~46,000
  cells** — every operator at thirteen widths in all four sign combinations, the widening
  composition, every net kind, 9,360 x/z cells against iverilog, 24,000 fuzzed mixed-sign
  trees — with all ten corpus workloads byte-identical. Its one NIT was mine: the comment
  named only the constant-index half of the arm, while the runtime-index half is admitted
  too and is the only op here that can move a DIAGNOSTIC. Corrected, and pinned.

  ⚠️ Re-censusing after the change shows the gate did not disappear, it MOVED UP: the same
  test now fails at four `Ternary` nodes for 6,425,888 requests, because the gate is
  per-node and a parent inherits nothing from an admitted child. That is the reverted
  set-wide relaxation's territory and is filed rather than taken.

- **The compiled expression backend refused any value whose width differed from its
  context, and that was 84% of its declines.** An execution-weighted census of `wprog`
  declines on the three corpus designs that have no frame bodies — the ones a frame arena
  would do nothing for — put 273k of darkriscv's 325k declined requests on comparison
  operands of unequal width, and most of the rest on a narrower `Signal`, `!` or select
  meeting a wider context. serv and picorv32 have the same two families at the top.

  ⭐ The work-list was far smaller than the request counts suggest: the compile cache runs
  the compiler once per `(expression, width, signedness)`, so 241k requests is **7 distinct
  expressions**.

  A narrower node is now admitted, and *how* is decided by the LRM's sizing rule rather
  than by the width: a **self-determined** node (a leaf, a select, a concat/replication,
  and every one-bit result — comparisons, `&&`/`||`, `!`, the reductions) is compiled at
  its own width and converted on the way out; a **context-determined** operator (`~`,
  unary `-`, the bitwise binaries, `+`/`-`, the shifts, `?:`) is computed at the context
  width. A comparison's operands are sized to `max(self-width)` with their pair signedness,
  which is §11.8.1 and is what the generic evaluator already did.

  Sign extension calls the same `resize_word` the generic path reaches through
  `Value::resize`, so the one conversion this backend emits is not a second spelling of the
  semantics. Zero-extension emits no instruction at all. Truncation still declines.

  ⚠️⚠️ **`self-width < context` does NOT mean "fold narrow, then extend."** That holds only
  for a self-determined node. The first version applied it everywhere and
  `logic [7:0] s = v[8:11] + 4'd1` became **0** instead of **16** — 15 + 1 folded at four
  bits. A pinned test caught it, and it is why the classification is an exhaustive match
  over the operator enums rather than a catch-all: a new operator must not inherit a
  sizing rule.

  ```text
    darkriscv  -6.2%      serv  -2.7%      picorv32  -1.8%
    aes / keccak / keccak-arr: flat (their cost is inside frame bodies)
  ```

  every pinned corpus digest unchanged, and darkriscv moved from parity to **1.08× ahead**
  of Icarus Verilog. The backend's own differential battery grew from 7,960 to **8,225**
  admitted trees and gained a widening sweep of **45,180** programs, every one
  value-identical to the generic evaluator.

  Adversarial 2-lens review: no BLOCKING. The differential lens was CLEAN over ~100,000
  generated cases — including exhaustive 64×64 value sweeps for comparisons, every 4-state
  value pair at small widths, and 1,130 fuzzed deep-tree designs — with the corpus
  byte-identical. Its three NITs were all stale documentation this slice left behind, now
  corrected. ⭐ Its fuzz also turned up a **pre-existing** wrong answer unrelated to the
  change: a comparison does not push its unsignedness down into its operands, so
  `(b >>> 4) > 8'd100` with `b = 8'shB3` is 1 in vitamin and 0 in both oracles. Reduced to
  a two-operator repro, pinned as it stands, and filed.

- **Every frame call rebuilt its local window from the IR, and one whole class of call
  built one it then threw away.** All three frame-entry sites — `run_frame_call_with`,
  `run_task`, `enter_task_frame` — opened with the same six lines: a `Vec` allocation,
  `locals_len` `Value` constructions from `ir.nets[..].init`, and a `free` at the pop,
  **per call**, for a list that is a pure function of the IMMUTABLE IR. `keccak`'s
  `rotl64` is called millions of times; `aes` has 18 frame bodies.

  It is now a per-function template built once at elaboration-time routing setup, plus a
  capacity-capped free-list of retired windows: `clear()` + `extend_from_slice(template)`
  reuses the allocation and restores every slot to its declared default.

  ⭐ And the arm for a function whose locals are all STATIC — which is what a plain
  non-`automatic` Verilog function is — built the window on every call and handed it to
  `static_store.entry(func).or_insert(fresh)`, which **drops it on every call but the
  first**. The entire per-call window cost, paid for a value nobody reads.

  Separately, the static slab was a `BTreeMap<u32, Vec<Value>>` whose stated reason was
  determinism — but the key is a dense `FuncId`, so a `Vec` indexed by it is deterministic
  BY CONSTRUCTION and costs an index instead of a tree descent. Nothing iterated it and it
  is never serialized, so the map bought nothing the index does not.

  ```text
    keccak     4.437 s -> 4.097 s   -7.7%      biriscv     4.080 -> 4.021   -1.4%
    aes        2.865 s -> 2.670 s   -6.8%      serv        7.883 -> 7.847   -0.5%
    keccak-arr 13.420  -> 12.952    -3.5%      darkriscv   7.094 -> 7.135   +0.6%
    sha256     1.335 s -> 1.291 s   -3.3%      picorv32    4.721 -> 4.617   -2.2%
  ```

  every pinned corpus digest unchanged.

  ⚠️ The load-bearing rule is that a window is recycled ONLY from a pop that provably
  DROPS it. `stash_windows_in` also pops `frame_stack` — but it MOVES the window into a
  `FrameRec` and pushes it back when the activity resumes, so recycling one of those would
  hand a LIVE window to the next call, with no loud symptom. Only the two synchronous
  `&self` executors' terminal pops retire; the Case-B fork-in-frame window, which is moved
  into the `frame_windows` arena and outlives the call, is never pooled either.

  ⚠️ `clear()` before the refill is what makes an unwritten local read as its DEFAULT
  rather than as the previous activation's value — an `integer` reads X, an `int` reads 0,
  a `string` reads `""`. That is the property `crates/cli/tests/frame_window_reuse.rs`
  pins, since a reuse bug is invisible to a wall clock and to a final-value assertion that
  never calls the function twice.

  Adversarial 2-lens review found **no BLOCKING**: the differential lens ran ~624 designs
  across all three backends (~1,870 PRE/POST pairs) with zero output and zero exit-code
  differences, all ten corpus workloads byte-identical, VCD bytes identical, and staged
  `vcmp→velab→vrun` identical; the soundness lens censused all five sites that remove an
  entry from `frame_stack` and confirmed the two that retire are terminal — checking, rather
  than taking, `run_task`'s docstring claim that a suspendable task never routes there.
  Its one NIT proposing a third retire site was refuted by measurement: the stated harm
  mechanism did not reproduce and the ceiling on a design engineered to maximise the path
  was 1.2%, below noise.

- **The default backend allocated two `Vec`s per DELTA, and threw a third buffer's
  capacity away on every continuous-assign fixpoint pass.** `native::run::propagate` runs
  once per delta — 5.5 M times on picorv32, 7.0 M on serv, ~15 M on sha256 — and opened
  with `let mut changed = Vec::new(); … let mut woken = Vec::new(); let mut clocked =
  Vec::new();`. The vectors are TINY, median 2 to 8 entries, which is not a reason to
  leave them alone but the reason they cost so much: each is a `malloc(48)`, one to two
  `realloc`s through capacity 4/8/16, and a `free`. Caller attribution of
  `alloc::raw_vec::finish_grow` put `simulate←propagate` at **74.9% of all `Vec` growth
  in the process** on sha256, 65.5% on picorv32, 44.2% on serv.

  Separately, `settle_cont_assigns` built its visit list with `mem::take(&mut ca_dirty)`
  and then dropped the taken buffer, leaving `ca_dirty` at capacity **zero** so every
  later `note_change` push regrew it from scratch — `note_change`'s push line alone
  measured 6.2% of serv.

  ⭐ The interpreter has done the take/restore since its own measurement, with the reason
  written down (`sched/propagate.rs`: *"a fresh Vec pair per call was measurable allocator
  traffic"*) and four named scratch fields behind it. The DEFAULT backend never got it, so
  the reference implementation was in the sibling all along.

  ```text
    serv       9.234 s -> 7.861 s   corpus ratio vs iverilog 0.80x -> 0.92x
    sha256     1.460 s -> 1.345 s                            2.85x -> 3.07x
    picorv32   4.847 s -> 4.564 s                            1.50x -> 1.55x
    darkriscv  7.573 s -> 7.184 s                            0.96x -> 1.00x  (parity)
    biriscv    4.187 s -> 4.175 s      aes / keccak / keccak-arr    flat
  ```

  every pinned corpus digest unchanged. Geometric mean against iverilog is now **1.51×**
  over the eight workloads that run and **1.86×** over the five third-party designs that
  made up the 1.60× quoted in August. ⭐ `darkriscv` reaching parity matters more than the
  percentage: it is the workload the tracker named as the next thing to measure *because
  it loses for a reason the arena hypothesis does not explain*, and it got there without
  anyone looking into it.

  ⚠️ Three traps, each of which silently undoes the fix with no wrong-answer symptom:
  `for p in woken` iterates BY VALUE and consumes the buffer (now `drain(..)`);
  `propagate`'s early return on an empty changed set is the arm it takes MOST often, an
  idle delta, so failing to restore there re-allocates on the next one; and
  `settle_cont_assigns` has two `return`s inside its fixpoint loop.

  ⚠️ The row this came from named the wrong file and the wrong mechanism. It pointed at
  `sched/propagate.rs`, which is the interpreter and is already correct, and it blamed a
  `chunks[0].clone()` in `k_schedule_nba_scalar` — measured: `LvalChunk` is 32 bytes of
  `Copy` fields, its clone allocates nothing, and it is 0.36% of any design. That
  candidate is priced out and struck.

  Adversarial review: 0 BLOCKING across 1,482 differential cells — 10 corpus workloads at
  full N, 130 generated designs, 100 designs whose observable is the WAKE ORDER of up to
  nine processes on one net, plus clocking blocks, `force`/`release`, `wait` predicates
  and an F4016 oscillation — all three backends, with stdout, exit code, VCD, FST, `.vu`,
  `.velab` and the staged path byte-identical.

- **A whole-net read at its own width made three `Value` moves to perform two stores.**
  `Value::resize_keep_sign` — the function every whole-net read goes through — combined the
  signedness, called `resize`, and then re-stamped the signedness. At EQUAL width, which is
  the common case, `resize`'s own arm does nothing but drop `is_str` and return, so the call
  was a 72-byte copy in and a 72-byte copy out to accomplish two field writes. It now
  answers equal width itself.

  ```text
    bench/keccak, N=2000, interleaved, first round discarded
    keccak_f.sv       5.36 s -> 4.44 s   -17.2%
    keccak_f_arr.sv  15.87 s -> 13.70 s  -13.7%
    corpus            aes -16% · biriscv -15% · darkriscv -11% · keccak -18%
  ```

  Digests unchanged everywhere; `picorv32`, `serv` and `sha256` are flat, as expected —
  they spend their time elsewhere.

  ⭐ The collapse is EXACT rather than usually-exact, and that is the whole correctness
  argument. Reaching the arm guarantees `!is_real` and `!is_str` (both return above it), so
  over all six fields: `val`/`unk`/`width` are untouched on both paths; `signed` was
  `(signed && ctx)` and then **unconditionally** `= ctx`, so the first write was dead and
  nothing reads it in between (`resize` consults `signed` only to sign-fill, which needs an
  extension); `is_real` is false and neither path writes it; `is_str` is already false, so
  `resize`'s `is_str = false` was a no-op. The arm keeps that store anyway, to mirror
  `resize` and to make the `is_str` early return above it load-bearing rather than
  incidental — a reorder would start it doing work, and a test pins that.

  ⚠️ It inherits `resize`'s proof obligation along with its shape: a `Value` is canonical by
  construction, so the arm re-establishes nothing, and the `debug_assert!(is_canonical())`
  is what makes a producer that forgets fail loudly in debug rather than reach here
  unnoticed.

  ⚠️ **Three experiments on the same axis are recorded so they are not retried.**
  `#[inline]` on `resize` alone: 0% — LLVM declines a function that size. Shrinking
  `Words::Inline`'s `len` from `usize` to `u8`: **0 bytes** — no niche optimisation survives
  the enum tag against `Vec`, so `Value` stayed 72 bytes. And one that measured −1.5% and
  then reversed sign: splitting `resize`'s >64-bit tail into an `#[inline(never)]` helper was
  worth −1.5% against the OLD baseline and **+3.2% against this one**, because the
  equal-width arm removed the calls it was helping.

  Adversarial review: 0 BLOCKING across ~4,000 PRE/POST cells, 2,248 cross-backend
  equivalence checks, corpus 10/10 with every pinned digest, and byte-identical stdout,
  VCD, FST, `.vu`, `.velab` and staged output. `run.json` matched on every semantic field
  including `codegen` and `native`, so no compile decision moved. Its findings were a
  mangled assert string, an overstated comment, and — the one that mattered — that the arm
  had **no test at all**; there is now a parity cell that writes the old path out longhand
  and compares, killed by a mutation of the sign collapse.


- **A user-function call no longer refuses the process body that holds it.** `is_codegen_able`
  recorded `user_call_in_expr` for any body reaching an `Expr::Call`, and the whole body —
  every statement in it, call-bearing or not — fell to the uncompiled walk. The reason
  recorded for that exclusion described a two-backend world: it said the frame evaluator
  "runs ONLY on the `&self` interpreter read path" and that "the interpreter is then the
  SOLE executor of any Call", neither of which has been true since `NativeKernel::eval_call`
  wired the tier-3 route. Nothing in the op stream evaluates an expression — every op hands
  an ExprId to a `Kernel` method, and each of those declines a call one level down to the
  generic evaluator that has always run it.

  Measured on `bench/keccak`: `keccak_f.sv` **8.11 s → 6.91 s (−14.8%)** and `keccak_f_arr.sv`
  **17.24 s → 15.37 s (−10.9%)**, both digests unchanged. Corpus 8/10 with every pinned
  digest matching, `examples/` 4/4 byte-identical, and a 24-cell call-shape battery
  (blocking / NBA / branch condition / system-task argument / ternary / `&&` / recursion /
  `$fatal` inside / nested / lvalue index / NBA transport delay / `force`) identical
  PRE-vs-POST, 16 of whose cells actually changed `able` — a cell that does not move
  measures nothing.

  ⚠️ This does NOT reach a frame call in a CONTINUOUS ASSIGN, which is a different path
  (`settle_cont_assigns`, never `is_codegen_able`). Measured on the shape an external
  report identified as its single largest cost — `assign w = {128'd0, zpad(src, nb)};` with
  a looping `zpad` — the time is unchanged. That axis is open.

  ⚠️ Also unchanged: the callee body itself. `frame_bodies` still counts every subroutine,
  and each one still runs on `SimState::run_frame_call`, the generic `Value` tree-walk.
  `crates/cli/tests/perf_call_regime.rs` carries the paired benchmark and pins both facts.

### Added

- **`run.json` now carries per-builtin evaluation counts and time** under `--obs-procs` /
  `--obs-procs-time`, as a `builtins` sibling of `processes`. A single expensive process row
  splits into "simulator primitives" vs "your RTL": `$fgets`, `$sscanf`, `$fdisplay`,
  `.push_back()`, `.size()` and the rest each get `calls` and (when timed) `time_s`.

  The convention is stated **in the file**, not just in a doc: `attribution: "self"` means a
  row excludes builtins nested inside it, and `included_in_processes: true` means that time
  is already inside the process row — so the two arrays must never be summed. Verified on a
  nested `$fdisplay(fd, "%s", $sformatf(…))`, where an inclusive convention would have summed
  past its own parent.

  Instrumentation covers all four seams a builtin can reach the engine through, including the
  synchronous `&self` frame executor, which does not go through the task dispatcher — without
  that arm a `$display` inside a subroutine body would have been silently absent. Cost when
  not requested, measured on a constructed worst case (3M pure system-function evaluations in
  a loop): **+0.86%**, inside the measurement noise of a control that touches none of the
  seams.

### Documentation

- **Every published performance number was re-measured (2026-08-28).** The two changes
  above move exactly one shape, and the corpus is the only place that could say so. On the
  same machine, release builds, interleaved samples with the first round discarded:
  `keccak_f.sv` **8.11 s -> 5.41 s (-33%)** at an unchanged digest, carrying that corpus row
  from 1.10x to **1.72x faster than iverilog**; `keccak_f_arr.sv` did not move, because the
  per-call frame-local array it exists to measure is a different cost, and it remains the
  corpus worst case (0.57x). Nothing else moved beyond run-to-run noise. Corpus 8/10, every
  pinned digest matching.

  The README table is rewritten from the new measurement — Verilator 6.8 us per Keccak
  permutation, vitamin 305 us without subroutine calls, 2 700 us with them, iverilog
  4 655 us — and now states the call cost as what it is: **8.9x on the same algorithm**,
  measured against the flat spelling of the identical design. `docs/study/03` gains the
  re-measured cross-corpus table, and `serv` appears in it for the first time (0.78x); it
  began running only in §4.5.382, so the five-design geometric mean is reprinted beside the
  six-design one to keep the two dates comparable. `bench/keccak/RUN.md` carries the raw
  four-tool numbers, all four agreeing on the published Keccak digest.

- **The codegen-discriminator note is corrected a fourth time, and this time the framing
  changes rather than a row.** The 2026-08-25 revision already recorded that `able` is
  anti-correlated with speed. A round-35 report asked, on the strength of a −30% A/B on
  its own design, for the inliner to be *widened* to control-flow bodies. Measured, the
  request is the wrong direction on the design that motivated it:

  | routing of the report's own `idx()` | wall |
  |---|---|
  | frame call (today) | 0.27 s |
  | hand-written source text (what they measured) | 0.83 s |
  | **vita's own inline fold** | **26.9 s** |

  Their hand-inlined file is *source text*, which has no formals — so it never pays the
  formal-binding coercion that vita's inliner would. `idx`'s formals are `int` and
  `int unsigned`; binding a possibly-unknown actual to a 32-bit 2-state formal is the
  same per-bit fan-out described above, and it lands **98×**. The inline path's upside is
  capped at 1.4–1.6× (one saved frame call) and does not grow with body size; its
  downside is unbounded — measured 335× on a six-statement body and 31,000× on a
  twenty-statement one, from the fold's expansion factor alone.

  Also measured and rejected: opening the bytecode VM's `Terminator::Call` refusal. The
  VM and the interpreter are within 1% on these bodies (their cost is expression
  evaluation, which the VM delegates back to the kernel), and the **default** backend
  already runs call-bearing bodies — so `codegen.able` under-reports what actually
  executes, and "able 3/5 → 1/5" does not describe the default path.

- **Throughput does not collapse with instance count.** A report observed a 5-engine top
  running ~5× slower per cycle than a single-core testbench and read it as super-linear.
  Measured over a 128× sweep, cost is a straight line — `t = 0.186 + 0.1597·N` per 200k
  cycles, every point within 3.6% of the fit, with the marginal cost per instance-cycle
  flat from N=2 to N=128. Five engines costing 5× *is* the linear prediction. Per-delta
  work is O(active), not O(design): 1,024 dead instances add 4.6% total, and that as a
  one-time step at elaboration rather than per delta.
## [0.2.0] — 2026-08-26

**The compiled backend is the product.** `native` is now the default and the only
executor a default-features build needs; the tree-walking interpreter and the
bytecode VM are demoted behind the `oracle` feature, where they exist to bisect a
suspected defect against a second and a third implementation of the same
semantics. Output is byte-identical across all three, enforced over the whole
corpus, so `--backend` is a wall-clock knob rather than a semantic one.

Since 0.1.0: **6,169 tests** (was 5,009), `format_version` 26 → 29, a ten-workload
third-party corpus pinned by SHA, FST waveforms, and thirty-odd correctness slices
driven by adversarial two-lens review against live Icarus Verilog and Verilator.

The MAJOR stays 0 while [docs/ROADMAP.md](docs/ROADMAP.md) §2/§3 carry open
correctness items. A MINOR bump does not invalidate artifacts — `verify_header`
gates on the semver MAJOR only — so existing `.velab`/`.vu` files stay valid.

### Added

- **Named and `default:` assignment patterns** (IEEE 1800 §10.9.1/§10.9.2):
  `c = '{mode: 4'h3, en: 1'b1, len: 8'd7}` for a packed struct, and
  `a = '{default: 5}` for a packed struct or a fixed-size unpacked array (any
  bounds, 1-D or multi-dimensional). They mix — `'{mode: 4'h5, default: 1'b0}`
  gives the named member its value and every other member the default. All three
  work in a procedural assignment, in a declaration initializer, and inside a
  task or function body.

  This is worth reaching for even where the positional form already worked. A
  positional pattern is coupled to the declaration order, so **inserting a member
  silently shifts every later value** — and that failure surfaces as a wrong
  result at exit 0, not as an error. The named form makes it impossible.

  `default:` is applied to each member SEPARATELY, in that member's own width, so
  on a `{logic [3:0] mode; logic en; logic [7:0] len;}` struct `'{default: 1'b1}`
  is `13'h0301` and not all-ones.

  Still loud, on purpose: an integer key (`'{0: a, 1: b}`), a type key
  (`'{int: 0}`), a replication (`'{N{e}}`), a keyed pattern whose target is a
  dynamic array / queue / packed array / subroutine argument / continuous assign,
  a member left uncovered, an unknown or duplicated member name, mixing
  positional and keyed elements in one pattern, and a call in the `default:`
  value (which would run once per member instead of once). Note that Icarus
  Verilog 13 rejects every keyed pattern outright, so a design using this form
  will not build there.

- **`--backend <interp|vm|native>`** on `vita` and `vrun` — selects which executor runs
  process bodies. **`native` is the default** and runs every design; `interp` is the
  readable reference semantics and `vm` the bytecode compiler, and both exist to bisect a
  suspected defect against a second implementation. **Output is byte-identical across all
  three**, enforced over the whole corpus by `sim-engine/tests/backend_equiv.rs`, so this
  is a wall-clock knob like `--threads`. `vcmp`/`velab` reject it: nothing in the artifact
  they write depends on the backend, and accepting it would suggest otherwise. A build
  made without the `oracle` feature has only `native`.
- **`sim_engine::codegen_coverage`** (also `run.json`'s `codegen` under `--obs-dir`)
  reports how many of a design's process templates get the compiled op-stream path — the
  rest walk the IR, on `native` as well.

  ⚠️ **Corrected 2026-08-18.** This entry used to say "Real designs measure 50–67%, the
  remainder being the `#delay`-bearing stimulus half. Writing transforms as `function`
  costs nothing: vitamin inlines them during elaborate." An external report measured
  **12–16%** on a function-heavy AES design, with **`user_call_in_expr` 87% of the
  rejections** and `delay` a rounding error — and they were right. Re-measured here, the
  discriminator is not package-vs-module.

  ⚠️ **Corrected again 2026-08-25.** The 08-18 correction replaced one wrong claim with a
  narrower one — "the discriminator is one keyword" — and a second external report
  measured that too. It is not one keyword. **Control flow in the body blocks inlining
  just as `automatic` does**, and a package-QUALIFIED call blocks it at the call site.
  Re-measured here on one call shape (`always_comb b = f(a);` + `always_ff c <= f(a);`),
  varying only the body or the spelling:

  | function form / call spelling | inlined during elaborate? | `able` |
  |---|---|---|
  | `function f(…)`, straight-line body | **yes** — no `Expr::Call` survives | 3/5 |
  | …with a local variable, or calling another function | **yes** | 3/5 |
  | …with a ternary `?:`, a concat, a part-select | **yes** | 3/5 |
  | …reading a module-level signal | **yes** (deliberate — framing would drop it from the `always_comb` sensitivity list) | 3/5 |
  | …containing `for` / `while` / `repeat` | **no** | 1/5 |
  | …containing `if` / `else` | **no** | 1/5 |
  | …containing `case` | **no** | 1/5 |
  | …containing a `$display` or any system task | **no** | 1/5 |
  | …written `return x + 1;` instead of `f = x + 1;` | **no** | 1/5 |
  | **`function int` / `function bit [7:0]` / `function byte`** | **no** — any 2-state return type | 1/5 |
  | …with an unpacked-array formal | **no** | 1/5 |
  | …whose return type is WIDER than its top-level operator | **no** — the context-width route | 1/5 |
  | `function automatic f(…)`, straight-line body | **no** — lowered to a frame body | 1/5 |
  | …with an OUTPUT formal + a control-flow body | **no** — reason `frame_call`, not `user_call_in_expr` | 1/5 |
  | `import p::*;` then `pf(a)` | **yes** | 3/5 |
  | `p::pf(a)` (qualified) | **no** — per CALL SITE | 1/5 (2/5 if only one of the two sites is qualified) |

  ⚠️ **Corrected a third time, 2026-08-26.** Two of the rows above were wrong and one
  sentence pointed the wrong way.

  * The qualified-call row said **2/5**. The refusal is per CALL SITE, not per function,
    so on this table's own two-call shape it is **1/5**; 2/5 appears only when exactly one
    of the two sites is qualified. And the route is not a resolver miss to be opened
    cheaply — `inline_pkg_function` frames it deliberately, with the reason written out
    (the inline fold evaluates the return expression at its self-determined operand width
    and resizes afterwards, so an 8-bit `a+b` bound to a 16-bit return keeps 8 bits and
    `255+255` is 254, not 510). All three spellings measure 510 today.
  * The table named 4 of the **12** shapes that frame. The biggest omission for SV RTL is
    a **2-state return type**: `function int f` frames where `function logic [31:0] f`
    with the identical body inlines.

  * ⚠️⚠️ **And `able` is ANTI-correlated with speed on exactly the body shape the report
    cites.** The closing sentence used to file *"widen the inliner to control-flow bodies"*
    as the improvement. Measured, interleaved, same design, only the body wrapped in
    `if (1'b1) … else …`:

    | body | inlined | framed | winner |
    |---|---|---|---|
    | 6 chained statements, local read **once** each | 0.19 s | 0.24 s | inline 1.27× |
    | …read **twice** each | 1.77 s | 0.31 s | **framed 5.6×** |
    | …read **three times** each | 14.39 s | ~0.35 s | **framed ~40×** |

    Digests are byte-identical in every pair, and `elab_s` stays flat at 0.35 ms across a
    0.16 s → 14.36 s spread of `sim_s` — so the arena SHARES the substituted subtree as a
    DAG and the evaluator re-walks it as a TREE. The inline fold is therefore
    kᶰ in (references per statement)^(chained statements) where the frame path is linear,
    and the shape that blows up — an accumulator re-read inside a chain — is what a
    cryptographic combinational function is made of. Widening the inliner would convert
    the reporting design's 0.31 s into 1.8 s or 14 s at unchanged output.

    ⇒ **`able` is a coverage number, not a speed proxy.** The item filed in ROADMAP §3 is
    now the DAG re-walk (memoise a shared sub-expression per activation), not the inliner.

  So on real RTL `automatic` is rarely the binding constraint: the reporting design has 24
  `function automatic` in its RTL, and removing the keyword from all 24 moved
  `user_call_in_expr` from 262 to 261 — **0.4%** — because 16 of those 24 contain `for` or
  `case`. That observation stands; what changed is what to do about it.

### Documentation

- **Several documents still described the simulator as running on an interpreter.**
  Phase B/D made `native` — a compiled op-stream over a flat arena — the default and
  the only executor a released build contains, but the user manual still said "the
  backend is a deterministic IR-walking interpreter", the CLI reference still listed
  `--backend <interp|vm>` with **"`interp` (default)"**, and three `docs/preview/`
  specs still introduced the backend as `인터프리터 방식 (IR-walking)` in the present
  tense. A reader — human or model — could reasonably conclude vitamin interprets.

  Worst of these, because it is the document an agent reads to interpret vitamin's
  output: **`docs/preview/19-ai-agent-observability.md` claimed `--backend native`
  "still falls back to the VM" and that `"native"` appears only in
  `backend_requested`.** Neither is true — an ordinary run writes `backend: native`,
  `backend_requested: native`, `native.refused: null` — so an agent following that
  spec would read the `backend` field and conclude native had not run.

  All corrected against the running binary. The manual's per-backend speedup table
  was **removed rather than refreshed**: stale numbers are what made the section
  wrong, so it now states the ordering (`native` < `vm` < `interp`) and the commands
  that reproduce it, leaving the numbers in one place. That ordering was re-measured
  for this change on `bench/picorv32` — 18.0 s / 33.9 s / 52.8 s, release, warm — on
  its own default testbench settings, so those figures are not comparable to the
  corpus runner's picorv32 row, which drives a different configuration. The 2026-05 design spec, which predates all of this,
  carries a HISTORICAL banner instead of being rewritten.

### Fixed

- **The constant domain now computes in the width the declaration states, not in i64.**
  Two external reports (round-33 and an AES IP audit) converged on one axis: every
  crypto/AXI constant idiom that mixes a NAME with a wide value was rejected, and which
  operator you wrote decided whether the parameter existed at all. `A ^ 128'h1` folded
  and `A ^ B` — same value, named — did not; `+`, `-`, `*` and the comparisons had no
  wide arm; a select of a >64-bit parameter (`A[127:64]`, `A[127]`) had none either; and
  a reduction (`^A`) or `$countones` had none at ANY width. All of them fold now, and
  every value was measured against **both** iverilog 13.0 and Verilator 5.050.

  The pieces, because they were separate causes:
  - **A NARROW named parameter is now an operand of the wide fold**, read at its
    DECLARED width. The width comes from `param_range` — the map that records only
    declared provenance — and not from `param_meta`, where widths inferred from a value
    also live. That distinction is the whole soundness argument: §4.5.373 built the
    reductions on `param_meta` and measured `localparam W = 4'hF | 4'h0;` reducing 32
    bits where both oracles hold 4, then reverted. With the range declared, the stored
    value IS canonical at that width — measured on the same counterexample.
  - **Wide arithmetic and comparison arms** (`+ - *`, `< <= > >= == != === !==`, `&&`
    `||` `!`), the **reductions** (`& | ^ ~& ~| ~^`), **bit / part / indexed-part
    selects**, the **conditional**, `>>>`, and `$countones` / `$onehot` / `$onehot0` /
    `$signed` / `$unsigned`. Division and modulus stay out.
  - **A package parameter may be wider than 64 bits.** `localparam logic [127:0] K =
    128'he1…;` inside a `package` was `E3009` while the identical declaration in a
    module, in a `#()` header, in a one-element array or inside a packed struct all
    worked — the package's scalar path simply had no fourth domain to fall into. It
    travels through wildcard and explicit imports and through `pkg::K`.
  - **An override carries its WIDTH.** IEEE §6.20.2 gives an untyped parameter the range
    of its FINAL override value, and the i64 channel carried no range, so
    `#(.M_ISSUE(M_ISSUE))` forwarding `{2{32'd4}}` arrived 35 bits wide where every
    other tool has 64 — and the per-port slice `M_ISSUE[32 +: 32]` then read past the
    end. A `-G K=128'h…` override, which could previously only be refused, now applies.
  - **A replication COUNT may be a name.** `{N{32'd2}}` and `parameter [S*32-1:0] MASK =
    {S{…}}` fold. §4.5.371 built this and reverted it over four blocking defects; all
    four are answered by folding the count through the *same* name resolver the
    surrounding fold uses rather than through a second evaluator.
  - **Package `real` reaches module-scope constants.** `localparam real Q = pk::R*2.0;`,
    `int'(pk::R*2.0)`, a `parameter real` port default and `generate if (pk::R > 2.0)`
    were `E3009` while a byte-identical MODULE-LOCAL real folded and the package
    string crossed the same boundary fine.
  - **String methods fold in a constant context.** `localparam int W = S.len();` and the
    `getc` / `compare` / `ato*` family.

- **`generate` / `endgenerate` are optional (IEEE 1800-2017 §27.3), and a `genvar` may be
  declared in the `for` header (§27.4).** `if (…) begin … end` and `for (…) begin … end`
  at module scope are the dominant modern spelling and what synthesis tools are handed;
  vitamin required the wrapper, and the error it produced pointed at the `end`/`else`
  that followed rather than at the missing keyword.

- **A package's STATIC `function` could not call its own sibling under a selective
  import.** `import p::gmul;` made `gmul`'s body's call to `xtime` an
  `E3010 call to undeclared function`, while spelling the same function `automatic`
  worked and so did importing `xtime` as well — which is the caller's business, not the
  callee's. IEEE §26.3 resolves a routine's body in its DECLARING scope; only the frame
  path carried that scope, and the inline path now carries it too (around the BODY only,
  never around the actual arguments, which belong to the caller).

- **A `pkg::`-qualified value may take a method.** `pk::S.len()` was three parse errors
  at one column, none of which contained the words "method", "package" or "::".

### Diagnostics

- **A constant-fold rejection now points at the declaration and names the cause.** The
  caret came from the elaborator's ambient span, which during module elaboration is the
  MODULE HEADER — so every rejection in one module printed the same `file:line:col`, at
  a line holding no parameter, and finding the culprit meant commenting declarations out
  one at a time. And the text stopped at *"value is not a foldable constant
  expression"*, so three declarations rejected for three unrelated reasons (a package
  `real`, a string method, a replication count) were indistinguishable — none of the
  words `real`, `string` or `replication` appeared in any of them, and when the cause
  was an UNDEFINED NAME the name was in hand and thrown away.

- **A missing package file produces one error that names the package, not seven that do
  not.** `function f(input q::mode_e m);` with `q` absent produced seven `E2002`s — the
  first at the `::`, the rest at perfectly correct declarations further down that only
  failed because the port list never closed — and the package's name appeared in none of
  them.

- **An enum method rejection describes enum methods.** `x.name()` on an enum whose label
  names a `parameter` was reported as an *"unsupported hierarchical function call … the
  callee must be a framed function with input-only scalar formals … reached through an
  instance path"*, which describes a different feature entirely, and carried no
  `file:line` at all.

- **A non-constant bound in a declaration is reported once, at the bound.** The range is
  folded by more than one pass, so the same `$clog2(n)` printed twice, both times with
  the caret on the `logic` keyword rather than on the `$clog2`.

- **An unconnected child INPUT warns.** vitamin warned about a dangling `output` and
  `inout` and was silent about a dangling `input` — which is backwards with respect to
  consequence: a dangling output discards a value, a dangling input MANUFACTURES one
  (`z` at time 0) and propagates it. `W-ELAB-FEATURE-LIMIT`, and a ROOT module's own
  ports stay silent as before.

- **A variable with BOTH a declaration initializer and an `always_comb` driver warns.**
  `logic rdy = 1'b1; always_comb rdy = …;` ran at exit 0 with nothing to say, while
  xcelium stops elaboration (`*E,MULAXX`) and Verilator errors (MULTIDRIVEN). Nothing
  about the simulated value changes; this is the warning that was missing.


- **A `real` constant could not reach an integer context, even when you wrote the
  conversion.** `logic [int'(R)-1:0] v;`, `{int'(R){1'b1}}`, `$clog2(R)` in a width, and
  `localparam int M = R*2.0;` were all rejected with `VITA-E3009` — while this manual told
  users to "convert it explicitly with `int'()` or `$rtoi()`", advice that did not work.
  All of them now fold, matching both iverilog and Verilator, at module, `generate` and
  package scope alike. `int'()` rounds half **away from zero** (`int'(-2.5)` is −3) and
  `$rtoi` truncates, per §6.24.1 and §20.10.

  The expression is evaluated **whole in the real domain** and only the converted result
  becomes an integer, so `R/2` is still `1.75` and `generate if (R/2 > 1)` still tests
  `1.75 > 1`. An earlier attempt converted at the leaf instead and silently picked the
  wrong `generate` branch; that is why an IMPLICIT conversion (a bare `R` in a width bound
  or a replication count) stays rejected. It is not a missing feature — the two reference
  tools disagree about those, in opposite directions: iverilog sizes `[R-1:0]` while
  Verilator rejects the design, and for `{R{1'b1}}` iverilog rejects while Verilator
  replicates. With no agreed answer, vitamin asks you to write the conversion.

- **`v[int'(<real param>):0]` selected one bit, silently.** A part-select whose bound was
  an explicitly converted real read a single bit at exit 0 where iverilog reads the whole
  range (`v[int'(3.5):0]` is `v[4:0]`). Same root as the next entry.

- **An unfoldable cast in a declaration bound declared one bit, silently.** `logic
  [int'(NOPE)-1:0] v;` ran at exit 0 with a 1-bit net, while the bare `[NOPE-1:0]` twin was
  already loud about that same undefined name. A bound that cannot be folded is now loud
  whatever node it is written with.

- **`$clog2` of a real literal produced an empty replication.** `{$clog2(8.0){1'b1}}`
  replicated **zero** times at exit 0; both oracles replicate 3.

- **`$bits(<expression>)` used as a packed declaration bound silently declared one bit.**
  `wire [$bits(8'h00)-1:0] c;` produced a 1-bit net, so an 8-bit assignment truncated to
  `1` at exit 0 in every backend — and on a port it truncated across the module boundary.
  The same call already answered 8 at runtime and 8 as an unpacked dimension, so one source
  line had three answers. It now folds for a literal, a concatenation and a replication;
  shapes the constant domain still cannot see are LOUD instead of silently one bit
  (previously even `$bits(<undeclared name>)` declared a 1-bit net). Reported externally.

- **Two rejection diagnostics quoted a restriction that had been removed.** The file-read
  system functions and `$value$plusargs` both said they are "supported only as the direct
  rhs of a blocking assignment", which stopped being true when those calls were opened for
  `if` conditions, `case` scrutinees, nonblocking right-hand sides and more. Following the
  message made a user revert working code. Both now state their actual — and different —
  reasons, and the caret points at the call rather than at the enclosing statement.

- **`a[7:0][3:0]` produced no portability warning** while `(a^b)[3:0]` did, even though the
  same `W2004` text says iverilog rejects every form. iverilog states the rule exactly —
  all but the final index in a chain must be a single value, not a range — so the warning
  now covers it. `mem[i][3:0]`, which is legal everywhere, stays silent.

- **Two diagnostics contradicted each other on one event.** A parameter override wider than
  the 64-bit override channel warned "is not a constant; default kept" while a companion
  check errored — it is a constant, and no default was kept. The wide-literal case is now
  named for what it is; a genuinely non-constant override still reports that its default
  was kept, because there that is true.

### Fixed

- Two doc comments (`sim-engine/src/lib.rs`, `tests/backend_equiv.rs`) still described the
  Stage-B state — "the VM is not yet built, so ALL bodies fall back" — which the dispatch
  in `sched/scan_arm.rs` has contradicted since Stage C landed.

## [0.1.0] — 2026-07-31 · initial release

The first tagged release. Everything below this heading is the development
history that led to it.

**The major version stays at `0` deliberately.** `tool_semver_major` is a hard
staleness gate (`crates/vita-artifact/src/gate.rs`), so a `0 → 1` bump would
invalidate every existing `.velab` / `.vu` artifact — and semver-0 is the honest
signal while [docs/ROADMAP.md](docs/ROADMAP.md) §2/§3 still carry open
correctness items.

Release state: **5009 tests** green on Ubuntu, macOS and RHEL 9 · `format_version`
**26** · MSRV **1.85** · 59 diagnostic codes.

### Fixed

- **A call's copy-out destination was invisible to the classifier.** `Terminator::Call`
  carries only `{target, ret_bb}`; the destinations live in a call-site side table, so a
  statement-level walk over lvalues never saw them. A bare call inside a call frame whose
  `output`/`inout` actual was a module net therefore reached an executor that cannot route
  that write and **panicked with no diagnostic and no source location** (exit 101). Three
  earlier slices had recorded the trigger as "not yet named" and reverted twice, because
  they were looking at statement *position* rather than at the classifier's blind spot.
  Fixing it also turned four neighbouring loud rejections into working support.
- **An unrelated `$display("x")` decided which executor ran a subroutine body.** The
  classifier looked at a blocking assignment's *destination* only, so an effect carried in
  the right-hand side was invisible. Functions shared the same root; a `static task`'s
  `string` local needed a third collector; and `$fatal` could lose to its own `$finish`.
- **A frame-local `string`'s byte writes went to the wrong storage.** `s[i] = c` wrote
  through the dynamic heap while a frame-local string is slab-stored in the frame slot, so
  the write silently vanished. (Pre-existing; surfaced when the loud gate above was lifted.)
- **Eleven further silent-wrongs** found while fixing the round-20 report, including a
  constant fold that was not shadow-aware and a stand-down guard whose scope was a whole
  function and whose key was module-global.

### Changed

- **Runtime array-index and `$readmem*` diagnostics now say WHERE.** `VITA-E4002` /
  `VITA-W4029` (out-of-range / unknown array word index) named the array but not the
  indexing site, and `VITA-W4023` (`$readmem*`, `$writemem*`, `$fread` file trouble) named
  the file but not the call. Both now print `file:line:col … [in instance]` like the
  severity diagnostics, in one-shot and staged runs alike, so one table read from three
  places produces three distinguishable reports instead of N identical lines. Two limits,
  both deliberate: an access inside a `function`/`task` body is anchored at the **calling
  statement** (the compiled backends record it and report it at that statement's boundary,
  and anchoring deeper would make the same design print different lines under different
  `--backend` settings), and an access in a branch condition — evaluated after the last
  statement of its block — stays unanchored rather than borrowing that statement's line.
- `E3009`'s message no longer claims that a bare call statement in that position works —
  that is precisely the form that used to panic. It now names the real boundary, and a
  rejected call terminator is no longer reported as "a timing/suspend/fork control".
- An unroutable frame write is a **runtime fatal naming the net** instead of a panic.
- `--threads` / `-j` help text now says what it is: a waveform-writer budget. Simulation
  is single-threaded, and the flag never affected it.

### Notes on performance

An external report attributed a 4.3× slowdown at combinational depth 6 to vitamin not
levelizing deep combinational cones. Measured against Icarus Verilog with total work held
fixed, **Icarus scales the same or worse** (4.15× vs 3.55× at depth 12) and vitamin's
absolute wall time is 0.80–0.99× of it. The depth cost is a property of interpreted
event-driven delta cycles, not a defect; the gap to a compiled, compile-time-levelizing
simulator is a separate axis, analysed in
[docs/preview/18-acceleration-analysis.md](docs/preview/18-acceleration-analysis.md).

## [2026-07-29 · round-19/20]

### Fixed

- **A call that RETURNS A VALUE now counts as writing its `output` actual.** The
  definite-assignment walk recognized that write only where a call can stand as a
  *statement* — a bare enable, or a whole `if`/`while` condition. The moment the call
  returns a value it can sit anywhere an expression can, and there the walk asked only
  "does this mention the name?", for which a mention is always a read. So
  `go = nxt(5, r);`, `if (nxt(5, r) == 1)` and `while (n < lim && rsp_next(fd, r) == 1)`
  — the standard table-driven `.rsp` / CAVP vector walker — were all rejected. That was
  33 of the 34 diagnostics in the round-19 report, and the whole of its `TB=sha2`
  regression.
- **A branch knows what its condition evaluated to.** `a && f(out r)` is true only when
  BOTH operands ran, so the loop body / `then` branch knows `r` is written even though
  the loop exit does not; `a || f(out r)` is false only when both ran, which is what an
  `else` branch gets. (Verified against iverilog 13 with an observable side effect in
  place of the copy-out.)
- **`void'(f(…, out r));` and the bare `f(…, out r);` statement now lower.** Discarding
  the return value is the natural way to make an out-formal call a statement — and it
  was rejected, which left a testbench no way to express the write at all. Statement
  position is in fact the easiest case: the return slot goes to a throwaway temp and the
  copy-out (including an `inout`'s copy-in) happens. The "supported positions" message
  had also gone stale by omitting statement position; it now lists it.
- **A named argument no longer ends the definite-assignment walk.** The call resolvers
  bailed on the first `.formal(v)` they saw, so a call that uses one precisely to leave
  the other defaults alone read as unanalysable and the walk blamed a local several
  arguments to its left. Positional-then-named mapping (IEEE 1800 §13.5.4) is now
  applied. Building it exposed that the output/inout-formal function path never
  performed the named-argument reorder at all, and produced two diagnostics naming
  neither cause.
- **SILENT-WRONG: a default argument value was evaluated in the CALLER's scope.**
  IEEE 1800 §13.5.4 evaluates it where the subroutine is declared. A caller that
  declared its own `g` therefore hijacked a callee default that names `g`: vita printed
  `91` where iverilog prints `6`, at exit 0 with no diagnostic. The same hazard for a
  class method's default was already closed; the plain function/task twin was not. The
  guard compares BINDINGS rather than banning names, so a default naming a module net
  still resolves — from a module process and from a generate block alike.
- **SILENT-WRONG: file reads inside a framed subroutine body returned 0.** `$fgets`,
  `$fscanf`, `$sscanf`, `$fread`, `$fgetc` and `$ungetc` do their real work as a
  statement-level effect that only the process executor performs; a frame body ran the
  same expression through the pure evaluator, which returns X and touches nothing. So
  `rc = $fgets(line, fd);` inside a `function automatic` yielded `rc=0` and an empty
  string where iverilog reads the line — exactly the walker shape the fix above just
  made reachable. It is now a runtime fatal. (An elaborate-time gate was tried and
  measured wrong: a `task automatic` is lowered both framed and inline, and the inline
  copy — the one its callers run — reads the file correctly.)


- **A callee that advances time no longer ends the caller's definite-assignment
  scan.** The deep (callee-body) reference walker had no arm for `@(posedge clk)`,
  `#1`, `wait`, or `wait fork`, and a `_ => false` in a walker keyed on a name means
  "may reference ANY name". One timing control anywhere in a task therefore made every
  block-local declared after a call to it unusable — which is the standard clocked
  driver (`preload()`, `run_scenario()`), and was 11 of the 12 diagnostics in the
  round-18 report. The same mistake had been fixed in the shallow twin a round earlier.
- **SILENT-WRONG: a shared flattened net plus a call that suspends.** Two same-named
  block-locals share one net; one block writes it, calls a one-line helper that does
  `@(posedge clk)`, and reads back — observing the sibling block's write. vita printed
  `A v=99` where iverilog prints `A v=1`, at exit 0, identically at `c8ad2b4` and
  `46b9816`. The shared-net rule read only *syntactic* timing, so a wrapper hid the
  suspend; and the top-level walk returned early once the local was assigned, so the
  rule was never consulted at all. Both are closed.
- **A struct member write that covers the whole variable is a whole write.**
  `rm.c = 5;` on a single-member struct left the walk believing `rm` was unwritten.
  Members are constant part-selects after the parser's desugar, so the rule is bit
  coverage — which also accepts field-by-field writes and hand-written
  `x[31:16] = a; x[15:0] = b;`. Partial coverage stays loud.
- **`automatic` is no longer silently dropped from an unpacked-struct block-local.**
  `automatic rec_t r;` could not be resolved by the automatic-lifetime parse helper
  (an unpacked struct is not in the typedef table), so the member fan-out parsed it
  with no lifetime at all and the declaration became static. Two same-named struct
  locals in disjoint blocks then shared one net, while the identical `automatic int`,
  enum, and typedef-alias forms each got their own scope.
- **The first block declaring a shared name is now asked the same question as the
  rest.** The coalesce guard keys on the net already existing, so it only ever saw the
  second and later declarations — an ordering artefact, not a semantic distinction.
  The decision is now a pure AST pre-pass.
- **A call actual naming an unpacked-struct record is seen as touching its member
  nets.** The SoA fan-out renames the variable but not the call arguments, so a write
  through a formal was invisible (a false-loud) and an `inout` copy-in read was too.

### Added

- **`-v` now prints what actually ran.** Driven from a Makefile or a wrapper
  script, the arguments you read and the arguments the process received are
  different texts: the shell has already substituted `$(WIDTH)`, the filelist
  expander has spliced `-f` frames away, and `VITA_THREADS` never appears in the
  command line at all. `-v` prints the resolved form as a block at the top of the
  transcript — the invocation (shell-quoted, paste-able), cwd, every filelist
  opened, the sources actually compiled, incdirs, defines, runtime plusargs,
  output/log paths, elaboration roots and libraries, timeout, and the thread count
  *with its provenance* (`--threads`, `VITA_THREADS`, or `auto`). Empty rows are
  omitted, long lists wrap at the value column, and a flag never wraps away from
  its value. Because it goes through the normal progress stream, `-l/--log`
  captures it in the same file and the same order as the diagnostics it explains.
  Every applet echoes its own stage. Pure reporting: nothing about the run changes
  and nothing is hashed into an artifact.

### Fixed

- **A flag's value inside a filelist is no longer treated as a source path.** The
  expander's list of value-taking flags had not been updated since the original
  five, so every flag added later had its value rewritten against the frame's base
  directory inside a `-F` filelist: `--top top` became `--top /abs/ip/top` and the
  run died with "top module not found", while `--hier-tree h.txt` silently wrote
  `ip/h.txt` instead of the path the caller named. Affected `--top`, `-L`,
  `--work`, `--workdir`, `--upstream`, `--obs-dir`, `--hier-tree`, `--inst-paths`,
  `--probe` and `--probe-file`. Source positionals still resolve, which is the
  whole point of `-F`.

- **A chained method call no longer ends the definite-assignment scan.** The
  expression walker had no arm for `s.substr(a, b).atoi()` (IEEE §8.13 chaining),
  so it answered "may reference" for *every* name — one chain anywhere in a block
  rejected the chain's own assignment target and every local declared after it. It
  also blamed the wrong file: a chain inside a callee body was reported against a
  local in the caller.
- **The definite-assignment walk no longer forgets that a local is already
  written.** Its catch-all rejected any unmodelled statement outright, even on a
  path where the local was definitely assigned. Once written this execution the
  per-entry reset it would have observed is already overwritten, and nothing
  un-assigns it. This is why a dummy write *before* a loop cleared the diagnostic
  while the same write *inside* the loop did not.
- **A timing-controlled first write is supported.** `#1 x = 7;`,
  `@(posedge clk) x = 7;`, `x = #1 7;`, `#1 begin x = 7; end` and
  `wait (c) x = 7;` are blocking writes — the process does not continue until the
  write has happened. A timing prefix that reads the local is still a genuine
  read-before-write and stays loud, and on a *shared* flattened net any
  time-advancing statement stays loud (the scheduler can hand the one net to the
  other block in between).
- **A local that is never written is accepted.** `automatic byte exp[];` passed
  to an `input` formal is not a read-before-write — there is no first write for the
  read to be before. With no writer the flattened static net holds the type default
  at every entry, which is exactly what `automatic` supplies. Proving "no writer"
  consults the callee resolver, so a task that pokes the flattened net through a
  hierarchical self-path still counts as one.
- **SILENT-WRONG: a method call was not seen as a read of its receiver.** The
  shared reference walker checked a call's arguments but not its path head, so
  `s.atoi()` did not count as reading `s`. The block-local scope-leak detector is
  built on that walker: a block-local referenced outside its block only through a
  method call went undetected, coalesced onto the outer binding's net, and the
  outside read returned the block's value (vita printed `1234` where iverilog
  prints `9999`).
- **SILENT-WRONG: `atoi`/`atohex`/`atooct`/`atobin` parsed like `strtol`.** They
  skipped leading whitespace, honored a sign, and stopped at an underscore. IEEE
  1800 §6.16.9 says the conversion "scans all leading digits and underscore
  characters (`_`) and stops as soon as it encounters any other character" — so
  `" 3".atoi()` is 0 (not 3), `"-7".atoi()` is 0 (not -7) and `"1_0".atoi()` is 10
  (not 1). Verified byte-identical to iverilog 13 over 17 inputs. The old code
  carried a comment asserting iverilog's stricter reading was "its bug".
- **SILENT-WRONG: a hierarchical reference could reach an `automatic` block-local.**
  IEEE 1800 §23.9 forbids naming an automatic variable hierarchically — it has no
  static address — but v1's flatten publishes one as a module net, so `tb.a = 99`
  from another module silently wrote per-entry storage. Now rejected on read,
  write and select-write, as iverilog rejects it.

### Added

- **`note:` telling you where the definite-assignment walk stopped.** E3009's
  lifetime message names two possible causes; the third — "the analyzer stopped
  here" — was the real one for most sites and was invisible, because only the
  declaration's location was printed. The note carries the construct's own
  file:line:col and one of six reasons.
- The block-local scope-leak diagnostic now has a location at all: the error
  points at the declaration, a note at the reference that makes it illegal. The
  deferred hierarchical read/write/select-write passes likewise carry the
  originating statement's span, so every diagnostic they raise is locatable.

## [2026-07-29 · round-16] 

### Fixed
- **Definite-assignment for `automatic` block-locals now understands control
  flow.** A `break`/`continue` placed before the local's first write no longer
  reads as a live path arriving at a later read unwritten — every path that
  reaches that read has already executed the write. `if`/`else` and `case` arms
  that jump drop out of the merge the standard way. A real `disable` is
  deliberately *not* treated as a jump: it may name a block that is not an
  ancestor, so the statement runs on.
- **A statement-position call no longer ends the definite-assignment scan.**
  Calls are now proven harmless instead of assumed harmful: the callee's body is
  walked (transitively, on a depth budget) for any reference to the name,
  including hierarchical self-paths such as `t.a`. A callee that can reach the
  name still makes the call a read.
- **Fixed-size unpacked arrays declared `automatic` in a block.** A `'{…}`
  declaration initializer is supported (it re-runs on each block entry, matching
  the dynamic-dimension form), and an array filled element-by-element with
  literal indices is accepted once every declared index has been written.
- **The same name reused at two nesting levels of sibling block trees.** The
  net-creation and body-lowering phases nested their per-block scopes
  differently, so with both levels scoped the nets were created under one path
  and resolved under another. They agree at any depth now.
- **A comma declaration is treated as independent declarators.** Previously a
  collision on the first name rejected all of them — with a message naming a net
  that did not exist — and dropped the whole declaration, so every later use
  reported "undeclared net/variable" one line below its own declaration.
- **`q.pop_front();` / `void'(q.pop_front());` on a queue of structs** whose
  member width comes from a named parameter. Such a queue is stored per field,
  and the per-field fan-out had no discarding-pop case.
- **`return f(arr);` where `f` takes a dynamic-array formal**, and the same call
  buried in a concatenation.

### Changed
- Diagnostics: the hierarchical-task-call rejection now carries
  `file:line:col`; the dynamic-array-formal message no longer states a
  "module-process level" restriction that no longer exists, and names the actual
  reason instead; the same-name block-local message no longer infers the other
  declaration's lifetime.

## [2026-07-17]

### Changed
- `format_version` 21 → 22: IEEE **two-stage `#delay` conversion** — in
  mixed-precision designs a delay is now rounded to the declaring module's own
  `` `timescale `` precision first, then converted to the global simulation
  precision (iverilog-verified). The artifact timescale trailers are extended
  accordingly. (The full per-version history lives in the comments of
  `crates/vita-artifact/src/header.rs`.)

### Fixed
- `**` (power): exponent no longer truncated at narrow result widths.
- Conversion of >64-bit **signed** values to `real`.
- Overflow panic on package-enum label ranges.
- u32 overflow panic in indexed part-selects.
- `W-FLIST-OVERRIDE` now flows through the diagnostic gate, so `-Werror=` and
  `-Wno-` apply to it.

### Added
- FST output: case-normalization hardening of the `.fst` extension dispatch.
- The dev-only `separate-bins` Cargo feature now really builds standalone
  `vcmp`/`velab`/`vrun` binaries (thin shims sharing the multicall path).

## [Phase-1 MVP]

vitamin's first milestone: a working, deterministic, 3-OS-reproducible RTL
simulator for the Verilog-2005 synthesizable subset plus a synthesizable
SystemVerilog subset. The full `preprocess → lex → parse → elaborate → sim-ir →
sim-engine → VCD` pipeline drives both the one-shot `vita` flow and the staged
`vcmp → velab → vrun` flow.

### Language & front-end
- Verilog-2005 synthesizable RTL: modules, ANSI/non-ANSI ports, parameters/
  localparam, `generate`/`genvar`, functions/tasks, hierarchy & instances.
- SystemVerilog subset: `logic`, `always_ff`/`always_comb`/`always_latch`, and the
  user-defined types **enum**, **typedef**, and **packed struct**.
- Multi-dimensional **packed** and **unpacked** arrays.
- Full operator precedence; `casez`/`casex`; `fork`/`join`/`join_any`/`join_none`;
  `#delay`, `@event`, `wait`.
- `` `timescale `` (doc-08 model): unit/precision scaling of `#delay`, `$time`,
  `$realtime`.

### Engine & output
- Event-driven IEEE-1364 scheduler (Active / Inactive / NBA regions), 4-state values.
- Word-parallel (u64) 4-state bitwise & reduction evaluation.
- Hierarchical VCD output with real signal names and module scopes.
- `$display`/`$write`/`$monitor`/`$strobe`; `$dumpfile`/`$dumpvars`/`$dumpon`/
  `$dumpoff`/`$dumpall`; `$finish`/`$stop`; `$time`/`$realtime`; real support
  (`$rtoi`/`$itor`/`$realtobits`/`$bitstoreal`).

### Determinism & artifacts
- SchemaHash-frozen `sim-ir` golden root → 3-OS byte-identical output.
- Staged artifacts (`.vu` / `.velab`) with `format_version` staleness gating.
- Stable diagnostic codes (`VITA-Exxxx` / `VITA-Wxxxx`), doc-15 bijection.

### Verification
- 419 workspace tests; differential harness against Icarus Verilog (`iverilog` +
  `vvp`) for representative designs (skips gracefully when not installed).

### Internal artifact-format history
- `format_version` 1 → 2: staged artifact re-rooted at `SimIr` (M3 IR-backbone freeze).
- `format_version` 2 → 3: `real` (f64) type evolution.

### Known limitations
See [docs/manual/006_limitations.md](docs/manual/006_limitations.md). In brief:
arithmetic lane is 128-bit unsigned / 64-bit signed (wider poisons to X, fail-safe);
`casez`/`casex` treat scrutinee-x as don't-care; `$dumpvars(depth,scope)` args are
ignored (full dump); VCD memory dump is word-0 only; `%t` lacks default field width.

### Platforms
Linux and macOS (CI: Ubuntu, macOS, RHEL9). Windows is not currently supported.
