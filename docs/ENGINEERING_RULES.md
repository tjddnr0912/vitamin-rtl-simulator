# Engineering rules

The rulebook for changing vita: how work is decided, measured, reviewed and gated; each entry is one imperative plus the failure it prevents.

## 1. Scope and standing

This file is canonical for method and wins over any document disagreeing on method. Merge a rule learned while implementing into the matching section as one line. The file does not grow: a slice adding a rule makes room by merging or compressing rows, so `wc -c` after the slice is at most `wc -c` before it.

Read §1 and §2.1–§2.2 before implementing, then your role's sections (§3.2); no agent reads or is handed the whole file. Open work is in [ROADMAP.md](ROADMAP.md), one line per item; the user-facing surface a change must keep true is in [manual/](manual/).

Vocabulary:

| Term | Meaning |
|---|---|
| silent-wrong | A wrong answer with no diagnostic and a success exit; the outcome this repository exists to prevent |
| honest-loud | A refusal or diagnostic naming what the tool cannot do; always safe, never as good as support |
| correct support | The construct runs and its value matches the oracle |
| PRE | The binary built from the pre-change tree, extracted with `git archive <branch>` into a scratch directory, built separately |
| POST | The binary built with the change applied |
| lens | An adversarial reviewer with a fixed attack method, differential or soundness; §3.7 sets how many |
| census | An enumeration, from source, of every site able to reach a question, each cell measured, not argued |
| cell | One design plus one measured output |
| control twin | A cell identical but for the axis under test, attributing a result to it |
| anchor | An expected value fixed independently of any implementation, so shared-code changes cannot move it |
| hand-IEEE | An expected value read off the IEEE 1364/1800 text where no tool can arbitrate |
| oracle | An independent tool or standard text deciding a cell. Live: `iverilog` + `vvp`, run by the differential harnesses; second opinion: `verilator`, by hand on 2-state arithmetic, width and sign |

## 2. The accuracy ladder

### 2.1 The ladder itself

| Rule | Prevents |
|---|---|
| Rank every outcome on one ladder (silent-wrong worst, honest-loud always safe, correct support best); climb, never descend | A descent sold as a gain |
| Do not implement what cannot be verified: with no oracle and no precondition, stay loud | Unverifiable silent-wrongs |
| Making a working construct loud is a regression; loud is earned only by what the tool truly cannot do | Refusing working designs |
| Treat a panic as loud, not silent-wrong, and never trade correct support for loud to remove one | A ladder loss as robustness |
| When soundness and differential conflict, the differential wins; prefer a suite run to an argument, measuring an obvious fix and reverting it when wrong | Argument over measurement |

### 2.2 Never trade one wrong for another

| Rule | Prevents |
|---|---|
| Never trade one silent-wrong for another: convert types at the context boundary, not the leaf; stay loud where the context domain cannot be built | Leaf conversions losing the value |
| Remove a feature whose partial support only trades silent-wrongs, building the consumer-by-value matrix first (removal is an edit) | Half a fix plus a regression |
| A clamp is a silent value change: read the sign before it; let a shape it cannot carry fall through to the path already right | A clamped negative firing |
| Route a value onto another type only with its whole type, domain and width, stated | Silent guessed widths |
| Equalise head-on a path narrower than its twin, prerequisite slices first, never routing around it | Workarounds as constraints |
| Where interaction is unpredictable, support only the cleanly verifiable subset, the rest loud; for a deep residue prefer a loud guard to partial normalisation | Partial support going silent |
| Never widen a loud into a silent: a pre-existing root, or the same defect in another syntax, is a queue line, not a licence to expand the surface; defer the root, keep it loud | Your own widening regression |
| A fix or routing that interacts with a latent gap is a regression even when the value is right | A right value, a worse run |
| Before relaxing a blanket reject, enumerate its old rejects and run a live-oracle differential: zero regressions over a full sweep | Hidden shapes going silent |
| Never close a queue item by refusing | A decline eaten as a default |
| Build and measure both representations; when both are trades, remove the axis and file honest-loud plus a deferral | Shipping the nicer half |
| Do not build by leaf patching support that needs the enclosing node re-lowered: the leaf resolves after the context is decided | Context baked on a placeholder |
| Choose a storage table by whether its consumer can produce the right answer from the representation, not by name resolution | Names resolving to wrong values |
| Read how the neighbouring branch handles a cap before writing a new one meeting it | A silent clamp beside an error |
| Ask what a halted body leaves before whether the statement may run; if nothing is defined, the feature does not exist yet | Loud traded for silent |
| Close every leaking sink before changing the leaking value | Other sinks going silent-wrong |
| Find what still runs while a non-clearing latch is true before using it as a predicate | Output lost after the latch |

### 2.3 Accidental correctness and cancellation

| Rule | Prevents |
|---|---|
| Check a static claim you start consuming holds for the runtime value, and look for two cancelling errors | Breaking a cancelling pair |
| Before removing a conversion or an accidental loud, ask what it hid (values right for the wrong reason are the signal) and measure what it blocked; promote a right accident to a rule | Deleting a load-bearing accident |
| When a fix materialises a boundary, gate on whether the picture is trustworthy (two sources compared), not a shape list, and check if old cells were right only by cancellation | Fabricated answers |
| When a reported regression only reads an already-wrong input, fix the producer and run the corpus (it feeds more than this consumer); feed a shared helper's input to its existing callers before adding a call, fixing inside it if they are wrong too | Fixing only your call site |
| Measure the premise of a theorem that narrows a change, especially a data-structure invariant ("the stored representation is canonical"); enumerate writers when soundness rests on "only once, here, or never" | Narrowing on a false premise |
| When two readings are wrong in opposite directions, draw a boundary keeping pre-slice behaviour; a guess means out of domain | Either reading breaking a shape |
| Census aliases copying a value's type before correcting its source's type: an alias with no type of its own was right only where the wrong type agreed | Aliases turning wrong |
| An IR construction placing one expression twice is a semantic change, gated on observability (side effects, draws, diagnostic counts) | Double evaluation |

### 2.4 Width, context and provenance

| Rule | Prevents |
|---|---|
| A cast is the operand's context, not a later truncation: `N'(e)` evaluates `e` at `max(self(e), N)`, so a walk that cannot carry a width declines; fix context misuse in the propagated value (`max(self, N)`), never by discarding context; hand the context width to the operator, not the leaf: a context-determined operand evaluates at `max(context, every self-determined operand's width)` | Narrow folds; lost sign; bits lost to a wider sibling |
| Opening a width context, exclude a non-bit-vector domain on both sides (the target's declared domain, any operand in the region) by a conservative `_`-free walk whose unresolved arms decline | A real operand widening |
| Give a routing gate and a soundness guard different predicates: over-reporting is free for a router, a loud regression for a guard | New louds from a shared set |
| Fix a width defect at the consumer's fold, not a producer admission threshold: record each constant's declared width and sign with its value, fold per the standard | The defect one width up |
| Ask which rule sizes an initializer before choosing a consumer to keep loud; never decline what a type rule answers | A silent frozen behind a loud |
| Answer a query on a parser-flattened shape where the shape still exists, with a parse error, not a fall-through, for an unfoldable index | Queries in a blind layer |
| Gate on a declared width, never one inferred from an initializer's value, lost on an override; check every override channel | Binding at the default's width |
| Enumerate a stored value's readers before making it more precise; if they cannot use it, the asymmetry was safer | Guessing consumers |
| Carry the source's signedness on any channel moving a value across a width boundary, declining, not defaulting, where it cannot know it | A partly right container sign |
| Count what a domain cannot carry before widening it, declining what it cannot represent; refuse at the binder a shape the machinery cannot represent, not in a rewrite's arithmetic | Dropped widths; misread indices |
| Widening a guard's caller set, read its words: a guard naming a construct must test for it; a shared lowering must recover the lane unambiguously | False-louds contradicting source |
| Before hoisting a block-local declaration into an enclosing scope, probe its name's three lifetimes: shadow, sibling, leak | Scope-blind hoists |
| Deliver a feature the AST cannot carry by a desugar keeping every axis consumers need, refusing loudly an axis it cannot honour | Dropped axes |
| Before a preprocessor stage rewrites text, ask what its next consumer keys on and keep the form the oracles keep; route provenance emitted inside an expansion through the collapse site | Shifted directive bytes; stray offsets |
| Evaluate a shared constant walk as §11.8.2 evaluates a region, in one walk entry: pass 1 decides width and sign over the tree, pass 2 pushes both into every context-determined operand; each self-determined position (cast operand, concat part, shift count, exponent, select index, system-function argument) re-enters it as its own region, a comparison's operands one region at the larger side. Until then a routing arm excludes by structure (leaf-only positions, sign-homogeneous trees), never by a value-aware mirror; afterwards delete the structural rule | Narrow folds; leaked signs; partial mirrors |
| Key a guard on its stated premise, per binding ("type is a guess" fires only where no channel supplied the type); re-derive it when its row closes for most shapes, keeping it where the AST cannot tell readings apart | Guards outliving premises |
| Prove a pass-skip first: name the invariant the skipped pass would re-establish (extensions at the push-down's sign, nodes folded at the region's width), show pass 1 guarantees it, and measure x/z cells so the skip declines only where the taken pass would | Skipped pass-2 changes |
| Widen a fold over x/z by the operator's rule (a known bit deciding it), carry an x answer as an x bit, not a decline, and measure loud→value at every binder reading the fold | An x read as 0 |
| Decide a value's domain from the declaration in one predicate every binder asks (only an untyped declaration takes its value's type); when the representation moves, census the old one's consumers and scope fixes to it | Typing by the default's form |
| Size a case comparison once over the case expression and every item, comparing each item at that width and sign; size an assignment-pattern item through the pattern path (a stream item zero-extends into its element; only a whole-rhs stream is left-justified) | Self-width wraparound; pattern spellings disagreeing |

### 2.5 Declines, defaults and folds

| Rule | Prevents |
|---|---|
| Make sub-shapes a new match arm cannot improve call the arm they would otherwise take (a decline is a change, not a no-op); find who answers when a refusal becomes unreachable | Defaults or stale arms answering |
| Keep a fold predicting runtime behaviour admission-only where possible: disagreeing with the engine admits an out-of-range copy (loud), never a different word | Writing the wrong word |
| Before widening a shared fold or adding a refusal, ask if each consumer's context rule is exact for the new leaf and how it consumes a decline (a refusal is only as loud as its caller: walk each caller to its chain's end and say what is there before calling a decline free); decline per consumer for the delta only, as a documented delta-limiter, preferring to make the unknown knowable over widening the refusal | Silent defaults at success; falsely free declines |
| After deriving an equivalence, read the code back, name every assumed input, and build the design where that input is a default | Fabricated-default wrongs |
| Ask first how many properties a site's consumer expects and if enabling one contradicts the rest; gate interacting properties on one precondition, all or nothing, pre-slice behaviour verbatim without it | Partial application |
| Close a silent default upward by making the shape foldable, not the default loud, with cells whose true value equals the default | Right cells going loud |
| Make a structurally invisible omission loud: assert on success the pending set is empty | Silently absent items |
| Folding a constant into an initial value removes it from the initialisation order; it is not an optimisation | Literal vs call divergence |
| Check what the generic path leaves unevaluated before admitting a node: laziness is a diagnostic question, not a value one | New eager diagnostics |
| Recover a statement boundary with an exhaustive predicate, not a per-operation approximation | Lost writes and diagnostics |
| When a sound gate kills the feature, seek a second independent sufficient condition (a disjunct), not a weakening; narrow coverage, not semantics, when something cannot be proved | Re-admitted counter-examples |
| When no axis separates two groups, freeze one half at the old decision and write why (they are different questions) | One rule misfitting a half |
| Give a store rule a single-mention primitive when its operand may be evaluated only once (a user call, `$random`, anything containing one), gating multi-mention lowering on repeatability, never applied to a call; spell a narrowing or extending review fix in single-mention operations (select, sign stamp, primitive), a sign extension's fill being a second mention | Re-evaluated operands |
| Measure a widening arm's exclusions on the `localparam logic [W-1:0] L = …;` twin on PRE against both oracles; a wrong twin is the walk's defect: exclude the shape, file the walk as prerequisite, never patch it in the routing slice | Importing a walk's defect |
| Admit a desugar's items by shape when its value runs through a shared fold and binder: carry literal leaves, keeping the old refusal for names, operators and casts until the shared code is fixed (a hand-spelled twin proves only its leaves) | Exclusions lagging a class |
| Admit a width the parser already folded by its source text: a bound counts only when the text at its span is its lexeme; the folded expression hides a name or `$bits(T)` folded in the reading scope | Wrong-scope names |
| Record a placeholder's shape from its declaration only when that fold is exact by construction (structural predicate: literal kinds, names bound from exact values, ring operators, no value test), verifying it loudly at resolution; a value-aware repair of the shared fold is a prerequisite | Inherited fold defects |
| Deliver a time-0 run in the oracles' region: a constant's initial change is a net change by the time-0 settle that the armed header waiter sees, never a `#0` prologue or in-body wait; measure first on PRE with a hand spelling (`wire kw; assign kw = 1'b1;`) | Late prologues; missed glitches |
| Decide an index's constness from its leaves through the lowering's name funnel, three-valued: strict (every leaf provably constant) for callers admitting on `true`, permissive (some leaf provably live) for a stand-down or refusal, unknown keeping old behaviour; never via a fold with another lookup | Changing selects as constants |
| Give a single-mention shape (`TwoState(e)`, the ternary sign extension) only an operand whose width is declared (`ir_bits_of` answers); keep the old shape for fabricated widths, never a width-preserving node | Lost asserted widths |
| Hand-spell the composition on PRE before adding an IR primitive: if existing nodes express it in one mention, measure it, ship without a format bump, and state the engine arm's semantics at the builder | Needless nodes and bumps |
| Let a consumer newly reading a leaf resolve names only where the source leaves one object, decided by position (a top-level generate condition) and a per-definition census of declarations at every depth (declared once, top-level, a written integral type), never by maps a generate walk fills per position and phase | Phase-dependent, blind gates |
| Never fix a shared value domain in a slice that only routes through it: while its other rules (width, NUL bytes, shadowing) are wrong on the same cells, a more correct operation breaks right-by-accident cells | Swapped silent-wrongs |
| Let the consumer order the domains (only a truth test may ask integer before real), and give every fold lane of one value one decline policy | Integer-folded reals; split lanes |

### 2.6 Guards, gates and what removing one promises

| Rule | Prevents |
|---|---|
| Check a general floor removes no diagnostic masking another, still-broken span; before removing a masking loud guard, three-way census every masked shape and keep a correctly worded loud for what cannot yet be fixed | Owning masked silent-wrongs |
| Read the ladder per build (a promotion only where the fallback is not compiled); landing on one oracle where the tool answered neither is a rung up, not a split-axis touch | Descents elsewhere; refused gains |
| Ask the storage question on the write path wherever the read path does; when a read fix routes a name away from an object, walk the write to its end, name what it lands on, and refuse it at the lvalue funnels, not in the shared resolver | Writes to wrong objects or bits |
| Fix a stale read at the read, not the store: change what the reader resolves to, statically, in the sidecar every backend consults | Reordered settles and digests |
| Say so when a handler cannot use an argument; if a neighbouring branch warns and yours does not, that asymmetry is the defect | Undebuggable bare returns |
| Record a partially fixed count in the queue with its number, same slice (for a side-effecting operand the count is the semantics), and a held shape's oracle lines beside every pin (`REFUSED` marker) | Partial fixes looking fixed; re-grounding cells |
| When a computation runs twice, the second pass verifies the recorded first result, loudly on mismatch, never overwriting it | Stale first answers |
| Make executor selection ask a latched fatal first (latching is not skipping execution) and poll it in the statement loop so the process stops at the fatal point | Running past a fatal |
| Judge each declarator independently, splitting a declaration only where verdicts differ | One verdict killing the rest |
| Refuse where a snapshot mechanism has one slot and the expression needs more; ask the direction table for an argument's direction (an output actual is a write destination, an `inout` both, so a stand-down) | Overwritten snapshots; lost writes |
| When a body is lowered into two copies and the caller may use either, catch in the executor what it cannot do, not in elaborate; refuse in elaborate what elaborate cannot do, since a debug assertion vanishes in release | False-louds; silent releases |
| When adding a claim, state the fallback leaving existing answers literally unchanged; move a pre-existing strictness asymmetry in its own slice | Unrelated verdicts moving |
| Check if a refused construct's workaround is itself blocked; update messages listing supported positions when capabilities change | Phantom workarounds |
| Give a verdict name a documented contract and check it literally holds at every new site | False verdict names |
| Match delimiters on the token stream, not raw text, and make an unmatched opening delimiter an error | Comments closing constructs |
| Match the value to the oracle and state the risk in a warning (correct-or-loud means "not unnoticed", not "change the value"); non-conformance plus "the user cannot change the source" is a gap even where a document calls the refusal policy | Silent truncation; refused libraries |
| Route initializers by block, never splitting one block's initializers into a main sweep and a trailing group | Lost declaration order |
| Wire, not refuse, when the key is wrong; reclassifying into a new storage class, inherit the old one's capabilities and register in both tables only if the representations match exactly | Masked lookups; false-loud capabilities |
| Kill only events you created: split the dirty list, never clear it (a value can be restored, an event cannot) | Lost earlier events |
| Write a truth capture as a negation of a negation, never one expression named twice | Double evaluation |
| Place a refusal for a lane defect at that lane's store, keyed on the two widths it needs (stored value's, target's), never in a lowering shared with correct lanes | Un-fixing other lanes |
| Never guard by static scan a property the engine decides per time step (every channel into the step is a hole); ship the half the defect cannot reach (output under the defect equals the old lane's), filing the engine fix as the other half's prerequisite | Leaking scan gates |
| End a run at its time step's stable point: latch the request, drain every region still owed (rest of batch, `#0`, NBA, deferred, ticks due at `now`), park the requester against re-entry, and consume the latch where time would advance | Dropped or re-entered processes |
| Key a strict-path guard per axis, not per name, when a carrier covers only some uses; containers whose value flows through one node share one carrier | Carriers moving no cells |

### 2.7 Diagnostics and observability are product surfaces

| Rule | Prevents |
|---|---|
| A self-misdescribing machine-readable rail or wrong observability log is silent-wrong (its audience cannot check it): one engine source for every observed value, exports allow-listed to formatter-supported kinds, unparsed probes loud, gated by a three-way comparison plus determinism golden | Misleading the only audience |
| Report a diagnostic once against the user's own name, never a synthesized carrier's, suppressing it when the primary carrier is equally unknown | Leaked names |
| State only what a diagnostic knows, listing the conditions without inferring, and put the reachable cause first in a refusal message | Chasing impossible causes |
| Publish the values you decide with: a new judging layer's reason string reaches every place it is reported, same commit | Hidden local refusals |
| Give a desugar's diagnostics a code apart from the constructs it desugars, and reserve the name space a lowering uses as a channel | User tasks suppressed with facts |
| Report from where the reader and the sink meet, not where the value is produced | Late diagnostics |
| Carry the execution context in the same record as a static capability census | Misattributed counts |
| Put the identifier and discriminating rule into a new loud message, injecting a resolver through one trait where a layer has spans but none; point the caret at the operand the message is about | Anchorless messages; misplaced carets |
| Count diagnostics per user-written construct, the oracle's count as standard, saving and restoring the duplicate-suppression flag per construct | Per-leaf cap exhaustion |
| Write the role, not a capability list, in user-facing documentation | Stale capability lists |
| Capture the pre-expansion argument vector where it exists, and stamp a derived value with its provenance: flag, environment variable or automatic | Unknown compile inputs |
| Emit observability output in the run's process and stream, through the diagnostics writer | Uncaptured, misordered output |
| Never break a line between a flag and its value (carry a long value past the margin), and apply an escaping rule at every site printing a value | Misread flags; split lines |
| Read a defect an observability feature exposes as its first proof; comment a list-of-flags predicate's canonical site with why it must be updated | Stale flag lists |
| Carry the defer-time span in the defer record and set it in the resolve loop | Location-free diagnostics |
| Run the design and read the output before writing what it means; fix a claim everywhere it appears (diagnostic, docstring, comment); when lifting a restriction, grep its wording and re-derive each site's reason | Working code called illegal |
| Refuse a per-call profile blind to some execution path, publishing the blind region's map instead; state an attribution convention in the artifact and verify it by construction | Unseen cost as zero; double counts |
| Record a refusal as a first-class expected state apart from an exit-coded run (refused-as-pinned, refused-otherwise, promoted and refused-becomes-silently-wrong distinct), and a give-up state as a value with a span and a reason, not an empty answer | Forbidden moves as promotions |
| When code rebuilds an operation in its own spelling, check its diagnostics moved with it | Form-dependent loudness |

## 3. Review method

Each design or code change gets adversarial review at its tier (§3.7).

### 3.1 What a review is

| Rule | Prevents |
|---|---|
| Verify a suspected silent-wrong on a live differential oracle, not by argument | Argued defects |
| Separate roles: author, moderator, reviewer, recorder | Self-validation |
| Give a review a checklist: the brief's questions, the touched rule sections, a [preview/](preview/) spec only where one covers the change | A review inspecting whatever it happens to notice |
| Review 4 axes: architecture/system integration, performance/efficiency, maintainability/readability, robustness/testability | Unasked classes |

### 3.2 The briefing

| Rule | Prevents |
|---|---|
| Build PRE before the briefing from a snapshot commit, hand it out as `git archive <branch>` with its path, and tell reviewers not to touch the working tree; that commit, not a scratch directory, is the restore canon | A reviewer's own PRE build overwriting work; an unrestorable tree |
| Freeze, hash binary (staged: `--features separate-bins`; soundness mutants: own `CARGO_TARGET_DIR`); mid-measurement edit a copy, re-review after; after a mid-round blocking fix re-freeze, naming the binary measured | Mixed builds |
| Brief measurement table (cell × oracle × PRE/POST × class) as a file, opening "attack outside this table"; budget in tool calls, designs (on overrun: findings, next steps); clean is good, never invent findings | Waste, noise |
| Prove non-vacuity: byte-identity counts only if the fast arm fires, firing count, observed args recorded | Vacuous identity |
| Excerpt this file per role: every role §1 and §2.1–§2.2; the implementer §4 plus the touched §2 and §5 subsections; the differential lens §3.1–§3.3, §3.5, §6 and §7.3; the soundness lens §3.1, §3.2, §3.4, §3.5, §4 and the touched §2 and §5 subsections; an L lens both sets; adoption and regression verdicts §3.5–§3.7 and §6.3; a gate or corpus run none (CONTRIBUTING › The gate). Extract with `sed -n '/^### 3\.3 /,/^###* [0-9]/p'` | Bloated contexts |

### 3.3 The differential lens

| Rule | Prevents |
|---|---|
| Compare semantically, never structurally; per divergence report raw oracle output, class (real gap, no-oracle, vita-ahead, harness format), probe resolution | Misread divergences |
| Before shipping "X never produces an event", ground its firing side too: vary operand kind (literal vs var, constant wire vs reg) on both oracles | Silent-only fits |
| Run module twin of each interface cell filed pre-existing; if wrong alike, row is shared-model, not interface | Misfiled interface rows |

### 3.4 The soundness lens

| Rule | Prevents |
|---|---|
| Commission soundness lens explicitly, premises censused from source and standard: all-sites and variant enumeration, disjointness proof, same-name collision, guard traversal completeness, each consumed map's population path | Unfocused lenses |

### 3.5 Rounds and deltas

| Rule | Prevents |
|---|---|
| Brief later rounds as deltas: changed hunks, prior numbers to re-measure, killed mutations, documented survivors; demand findings outside them; after a blocking fix or design change run one naming the delta, mutating existing repro first | Re-derived, inherited results, unreviewed fixes |
| Read a stalled lens's partial output before killing it (its stop point is suspect), a dead verify phase as unverified (rebuild failing repros from mechanism); re-measure yourself cells lenses disagree on (a lens's "both oracles agree" was a split) and a lens's stricter or simpler rule on PRE before adopting it; refusing what PRE accepted is a ladder descent | Lost, false-cleared, wrongly adopted findings |
| State each fix claim's quantifier (one shape, one rule, every rule); a per-shape fix returns through another door; a generalised floor erasing a loud masking another defect is loud→silent-wrong | Respelled roots |

### 3.6 Stopping, reverting and prerequisites

The round budget is three; a fourth is a scope signal, not a fourth patch. Two consecutive blocking rounds on one axis end that axis's attempt before the budget does. An owner's program in ROADMAP §5.2 may lift both for its rows: a blocking finding is fixed, not reverted for round count; an axis reaching round three is re-planned, not patched.

| Rule | Prevents |
|---|---|
| Also stop if each fix on one axis yields the next blocker, a root returns through another door, or blockers sit in what you routed to: revert (whole for a returning root, else ship separable halves; a producer axis's patch gets own row with measured cells, consumer declining what it cannot vouch for), queue uncovered prereq; round three is for another axis or the revert's delta | Wrong-axis patches |
| After two mis-scoped guards or an unnameable condition, revert to pre-existing behaviour, measure the condition, register measured shapes; fix first the path holding a precondition (a workaround predicate wrong twice is an ordering problem) | Guessed third tries |
| Decide fix-or-revert from root, not effort (pre-existing and independent? needs machinery frozen IR cannot hold? blast radius?); on revert record mechanism, not verdict; a defect a change merely exposes belongs to exposed code; propagate closures only after commit; on re-opening re-measure, never restore text | Blind reverts, absorbed roots, stale closures |
| When three requests stop at one prerequisite, file one infrastructure line and point the feature rows at it | The next request hitting the same wall |
| Once a prerequisite lands, re-derive its reverted slice from the row and current code, never by restoring the old patch and its compensations | Floors for a missing prerequisite surviving |

### 3.7 Tiers, bundles and pipelines

ROADMAP §5.2 tags each row's tier (untagged: S at least); an F or structural-stage row never takes a lighter review.

| Rule | Prevents |
|---|---|
| C (corpus admission, no vita source change): one judge checks the evidence (study/03 rules 1–5, CI time), no lens | Unmeasured admissions |
| L (fresh lane), only if measured: PRE refuses each affected input at parse or by one named elaborate refusal, the sweep moves nothing PRE accepts, no arm joins a shared fold, classifier, resolver or walker; one lens: §3.3 on each new cell, §3.4 on the new accept set | A lens re-measuring the sweep |
| S (shared code edited or routed into, every lane measured): both lenses. F (an untargeted lane of shared code; binding, scope; scheduling, time 0; driver verdicts; cross-instance writes; representation, format; unsafe, FFI; loud→value through shared code): a design review first, both lenses in full, a mutation battery | Shared roots reviewed lightly |
| Escalate mechanically: a sweep moving a PRE-accepted source makes the row S at least, those sources leading the brief; a BLOCKING L finding re-reviews the fix at S; a design-changing fix gets a design review, any tier | Tiers argued down |
| Before lenses, run the full gate and sweep frozen release PRE and POST over every cell, example, corpus design and preserved repro (`.vu`, `.velab`, waveform, stdout, stderr, exit) | Reviewing what a sweep measures |
| Bundle 2–3 C/L/S rows, none across a stage boundary, whose actual diffs share no vita source path and lane tables do not meet, each gated and swept alone, then one PRE, POST, gate, sweep; each slice's cells on own and bundle POST, a difference splitting it; lenses, cell prefix per slice, briefs naming sibling axes, never pinned as residue; drop a BLOCKING slice, re-run the rest, a changed result stopping the landing; land clean slices singly | Diluted lenses; pins siblings move |
| Build the next row during a review in its own worktree off main once its files miss the reviewed diff; rebase after the landing, re-run scoped gate, cells, sweep (conflict: re-ground); a stage row's build waits for the prior one to land | Trees moving under review |

## 4. Census method

A census enumerates from source every site reaching a question, each cell measured; slices open and close with one. Producer, routing, ordering, consumer censuses ask who writes a value, where it goes, when it arrives, who reads it and how; none substitutes for another.

### 4.1 Start from a census, not from an implementation

| Rule | Prevents |
|---|---|
| Open a queue item with a census, not an implementation: grep each site building the construct over mechanism's whole reach, not reporter's cell; decide if defect reaches each; confirm on oracles | Symptom-sized rows |
| Before building, or accepting "this capability does not exist", find any function implementing the rule; count sites that should call it but do not | Partly called rules |
| Re-census before a slice; before ranking re-measure open rows at HEAD on both oracles, class first; before implementing a review finding or trusting "no prerequisite", grep queue for function or site, read each line, run designs of rows marked built or reverted; grep green pins' failure messages when choosing next item | Stale, unread rows |
| Before building for a row's shape, probe its plain twin (whole-value op for an element-select silent-wrong: right means access routing, wrong a storage gap; widths ≤32, 33–64, >64, naming lane); size fix by its sub-classes minus what an existing channel answers | Mis-sized slices |
| Write what was and was not measured into any sentence closing a family or giving a "cannot" verdict, never why it cannot be done; before publishing a refutation, check factorial readout holds each output the mechanism can produce, varying only claim's axis | Overclaims; refuted "cannot"s |
| Instrument rejection point by node kind and aggregate; do not ablate one gate | Armless kinds |

### 4.2 Containers, spellings and passes

| Rule | Prevents |
|---|---|
| Give a respelling or patching pass a site and pin per container of the type (grepped in frozen IR, not call sites; a correct sibling spelling flags a missed one), and a guard or coercion keyed on a slot's KIND the same arm in each slot constructor (`add_net(` twins: frame function, class method, hoist temp) | Missed containers, constructors |
| Before changing a shared function, primitive (conversion, resize, width), desugar or pass family, enumerate each site: scopes, callers, parser variants, assign sites, reserve paths, stmt dispatch, decl validation; check hit counts, not symptoms; that enumeration is the scope decision; a gate for one consumer gates each same-shape consumer (count copies) | Missed sites |
| Record a route census inside the emitters, never at the callers, with the table an emitter needs to file a row from an identifier alone | A new caller bypassing the seam |
| Census emitters sharing a diagnostic's context string through one resolver, and measure at least one other against the oracles | One emitter's change silently moving the others |
| If a block gets its own scope, census late-resolving consumers; carry each item's collection scope | Mass breakage |
| Census a scope rule at every spelling of the scope it names: module, interface, package, compilation unit | A rule applied to a spelling where it does not hold |
| Census a parameter rule over four channels — module body, instance-elaborated, package, instance override — and file the override channel as its own row | The override folding in the parent before the target's width exists |
| Census a subroutine-body rule at every binder that can inject a body into the table it reads (package import, `pkg::` call, interface, class), asking when each injection runs against the one-shot pass that computed the rule, and at the declaration's text outside the body (formal defaults, return-type bounds): where that text is lowered and who collects callees from it | Bodies or defaults arriving after or outside the rule's pass |
| If a table's key set widens, enumerate AST forms only new keys reach: lvalues, iteration, port connections | Unseen write sides |
| Price a diagnostic page after bisecting per header or file and censusing by construct in `[in scope]`, not `file:line` (instance errors report at instance site) | Mispriced pages |
| Census each gate of a parse-time constant table with a control twin: overridability, declared type, each decl of the name, readers you did not write | Skipped gates |
| Run the real design behind a row, not a reduction, also after a rule claimed to open it and before removing a loud gate or promising that closing one, or one of a pair, unblocks it (removal promises everything underneath); write the ladder; close what it exposes and take a same-table next page in the same slice | Unseen next pages; defects a probe missed |
| Give each census cell a keyword or scalar control twin beside element spelling, and a position query a multi-line cell; if a whole position column is loud, diff each cell's diagnostic with its control's before classifying | Misread cells |
| Before believing a boundary, widen the other operand on each axis (container dimension for "reads the wrong element"); a change moving a boundary needs a cell each side | Artefact bands |

### 4.3 Producers, populations and writers

| Rule | Prevents |
|---|---|
| Census each producer (constructors, struct literals, field writes, direct plane writes; aliases, pass-throughs re-seed defaults) before weakening a defensive check or never-positive guard, never a green suite, and when briefing a new per-instance carrier (key, param); check routed predicates vs stored value | Unproven invariants |
| Enumerate the resource (each argument the engine writes back) and everything a moved site sets, not your edits or motivating field; two fields one function sets are usually one fact; a guard citing another as model is a census of two | Half-moved facts, copied omissions |
| Before accepting a wall or building a reverted slice's prereq or a rejected construct's infra, find the nearest working spelling (one scope over, a sibling consumer carrying the guard) and what it calls; grep for a structure recording the property or partial support; in a routing census or if the tool contradicts itself, find the correct site: it is proof and spec; an additive-looking parser gap may be a storage or evaluation-model gap | Rebuilt machinery |
| Open population path of any set a check reads (no positional zip of formals); keep eligibility and process sets identical; check input set first if a feature works in one place only | Empty, mismatched sets |
| Read a per-net predicate via the element loop, not `read_net(net, None)` (element 0 of an unpacked array) | Element-0 verdicts |
| Search a concept's collectors by constructor, not name, and a source scan by prefix, not one family member | Uncounted siblings |
| Before moving a value out of a shared map or between record slots, census both sides' readers: value read or membership proxy? | Stale proxies, wrong-slot reads |
| Lower a whole into parts only where it is their sole writer, censusing finished-IR writers incl. channels no stmt carries (clocking-block output commit, one-way `inout`, unresolved placeholder); drop a decl's per-net record where a second decl merges onto the net (block-local coalesce) | Mixed writers, stale per-net records |
| Name the mechanism, not the missing input, when collapsing several rows into one infrastructure item, and re-measure the others the day it lands | Rows sharing a provenance, not a domain |

### 4.4 Routing, ordering and consumers

| Rule | Prevents |
|---|---|
| Before opening a new store count each membership question (read/write funnels, specialised evaluator, reader wrapper); route each read of a feature (value, offset, width, index) via the seam, re-walking call graph by store-access spelling, not names | Stray access |
| Count code paths a reject row blocks before narrowing it | Unthreaded executors |
| For a construct writing into another instance, census ordering plus routing | Ordering silent-wrongs |
| Give each read of a shared carrier type an explicit non-empty decline naming its reason; measure declines | Wrong-type binds |
| Re-check "the funnel discards the wrapper" per lane (whole-value vs bit) | Lane mismatch |
| Compare a capability-parity matrix before grounding a loud-to-supported candidate (one context set on it and its sibling) or unifying or routing storage classes; where neither dominates, extend additively | Parity regressions, shared silent-wrongs |
| Widen a read and sweep its write twin in one iteration, found by a scalar or fixed twin loud where this path is quiet | Wrong write twins |
| Before routing traffic into a helper or path, audit it (unchecked arithmetic, boundary shift masks, copied defects; its tests' names and docs, each documented gap the new traffic's defect) and its branches' accuracy parity; a one-branch guard is usually unmoved, not unneeded | Inherited defects |

### 4.5 The axes a census must vary

| Rule | Prevents |
|---|---|
| Census what a narrow form hides: each cell's `signed` spelling in a typedef census; the axis a narrower routed type cannot carry (sign, fraction, "never" for a count); a narrow constant beside a wide literal under unary minus, remainder, division | Omitted axes |
| Put one instance per census cell or compare sorted line sets; if an oracle contradicts itself on a second instance, mark which cells are multi-instance | Misread instances |
| Vary each field (base, width, direction) and spelling of a record, reported shape, report-named feature or gate-rejected shape, not only the named one | Unvaried fields, spellings |
| When one of several grouped operators diverges, look for the property the grouping missed; when a characterisation concentrates on one value, ask why the other half survives | Blaming the common rule; an accidental half hiding the axis |
| Before changing an axis's interpretation, classify every operator on it as sign-sensitive or bit-pattern in a comment table, and measure if an uncovered operator was right only by cancellation | Fixing one operator exposing its neighbour's defect |
| Count loud→value cells separately, every axis they touch with a control twin lacking the new construct; multiply position by 5 binders (module, instance, package, generate, override); open a §2 row with its shape's plain twin | Hidden descents; inherited wrongs |
| Refuse from a measured pair matrix: if a rule judges two decls, operands or kinds, run each unordered kind pair on both oracles with a two-name control; derive accept set | Unmeasured pairs |

### 4.6 Queue rows and incoming reports are claims

| Rule | Prevents |
|---|---|
| Split a broad item's scope by a 3-oracle census first; axes where oracles split are off limits | Unarbitrable axes |
| Enumerate resumption kinds, not sites, each with a 2-oracle cell | Kinds merged by site |
| Re-measure claims before use: each incoming report item at HEAD; demand claims like correctness claims; a documented split's discriminator; a row's "kept correct" twins before baselining; an assumption written as a degenerate special case (on the oracle); a combination believed impossible (count manifest rows holding it); a comment saying "only" or "cannot" (run one other case) | Claims taken as facts |
| Before accepting a row's root, read the type alias behind any tuple it cites by position, and build the no-construct control (a plain declared name in place of the construct) before naming the class after the construct | A misread tuple field; a class wider than its construct |
| Treat a fix shape (row's or grounding's) as a hypothesis: before keying a shared table on one fact ask if oracle sets it per kind (a root returning through new doors is one key under several rules); build, re-census, count moved cells first | Wrong-shape fixes |
| Before building a loud gate or reject row, measure what it refuses: enumerate shapes reaching the arm, run PRE on each, cut it from that hazard set, never a proxy; over-rejection is a ladder descent. Gating a shape the engine accepted, run suite, corpus, examples before review; ask what made a refused working design work | Over-rejection |
| Census a diagnostic model field's consumers, not its slot | Dead contracts |
| Survey third-party RTL before ranking; build workloads oracle-first, never simplifying or rewriting RTL for vita; a refusal is a result | Biased workloads |
| Grep each enumeration of a subset before widening it | Layers disagreeing |
| Re-measure a reject reason (at which phase it is true; each sentence citing a kind as reason when opening that kind's row); ask what a refusal blocks before what to build; split a feature-named row by census, a one-word reject row by what designs do, not which tables exist | Stale refusals, bundled rows |
| Read a function taking an alternative store as a parameter as using it only on paths naming the parameter | Other arms left silently wrong |
| Before "adding only where None", check the existing lane really declines; if it answers wrong, route the new lane ahead of it, not behind it as a fallback | A fallback that never runs |
| Measure a cell where candidate rules differ, not one both answer identically; if PRE matches oracle via a constant, vary coinciding value before attributing movement | Blind evidence |
| Run a row's own design before building what it prices: read the route it took (`run.json` `subroutines[].route`) before the function it names, and the context of its cited line (a keyword's meaning depends on it) | Fixing a site the shape never reaches; keyword-keyed over-rejection |

### 4.7 Completeness for a change already under way

| Rule | Prevents |
|---|---|
| Build a shadow set once from each binder (module: ports, import exports, enum labels, instance names, block-locals; routine: formals, body locals, block-locals, body-local enum labels, return name); hand that one function to each consumer (scope-hook stand-down, gate write set, binder skip set) | Missed binders |
| Before copying a call, list each call the original's caller makes; after adding a sidecar, mirror each copy site of an existing sibling workspace-wide; assert map non-empty at consumer before reading output | Incomplete copies |
| After changing a width, walk forward to each site re-deriving the value from a width; include gate's older branches in a new fence's blast radius | Unwalked sites |
| After a fix or learned fact, grep each site, census each region needing it, fix the second instance in the same slice, update comments citing equivalence argument in the same edit | Defect one layer in |
| Before replacing an approximation, changing a component, inserting a pipeline stage or unifying predicates, grep docs, comments premised on it (a named condition, "nothing before this point", "another gate rejects it anyway"); before committing re-check them and documents written before a design-changing review (risky sentences name a file, function or count) | Stale premises; false prose under green gates |
| Control a path-dependent feature by its path-deciding decl, verify callee takes it, fix each path in one iteration | Divergent paths |
| Test "the upper layer refuses this first" by running shape through it, and a new loud gate's only-net claim by measuring its bypasses: calls, hierarchical names, methods | Untested coverage |
| If a new site reads an alternative store, build its drain twin in the same slice, per termination path | Repeat failures |
| Count before building: shape making a new structure meaningful (state it in one sentence), consumer-by-value matrix before adding or removing a feature, how often a row is sole blocker (a never-sole row cannot close alone); record marginal and standalone gain separately | Baseless work |
| Count a per-stmt fact's render sites (a census cell each; let an executor without a seam write it); route each site folding one field via a named funnel with old call as fallback, measuring its new lane on shapes it must not change | Partial fixes |
| Census 3 lifetimes of a name-keyed parser rewrite: decl, shadow, export | Missed lifetimes |
| Cover a fix's or addition's twins in the same sitting: file's sibling funnel (by reading, not probing), every other match arm (does the reason apply? an inconsistency one arm's fix exposes means checking the siblings), each walker for a new expr kind, both evaluators folding a text, both halves of a shallow/deep walker pair, the other callable kinds | Twin defects |
| Make a classifier walking stmt lvalues audit write destinations' side tables | Unseen writes |
| After opening a gate, drive the whole idiom, file access included, to the end | Hidden silents |
| Reproduce stated basis before calling a deliberately unverified place a misdiagnosis | False misdiagnoses |
| Say why a mechanism worked for its narrow shape before generalising it; the reason is usually a property the wider shapes lack | Losing cells once generalised |
| Classify the cells a first fix leaves wrong by what they share before calling them pre-existing; a shared property names the rule's missing half | Half a rule filed as residue |

## 5. Gates, predicates and classifiers

### 5.1 One rule, one home

| Rule | Prevents |
|---|---|
| Write a question or rule twins share once and make both call it: a question beside its twin in `sim-ir`, not at the consumer; a renderer's value-free rules in a crate both the constant twin and the runtime reach | Second spellings converging one finding at a time |
| Write a naming rule two stages share once and have both use it; a limitation whose reason is another stage's implementation detail is that stage's defect | The naming and lookup paths diverging |
| Fill a new table with the same producer as its twin, so one provenance rule covers both scopes | A second producer posing as the first rule |
| Guard in one documented funnel every operand-building site, enumerated, passes through, named for the prohibition, not a type list | Per-site guards missing siblings, axes |
| Add shared-machinery semantics only opt-in, documenting when safe; put context/width rules on consumers (grep callers; on disagreement, at call site) | One consumer's needs forced on all |
| Expose a flat entry as the canonical implementation's post-normalising delegate; drop the old entry when the canonical moves | Drifting spellings; diverging lanes |
| Refuse, not route, when routing respells an existing rule; correct support is its own slice adding a funnel escape hatch | Respelling a split rule |
| Reduce a new shared input to one function, not one datum | Two reduction loops having to agree |
| Write a node's children, order, evaluation conditions, no-hoist positions once for all walkers/hoisters; mark reads a transform cannot reach unrepairable | Diverging walkers |
| Route every writer of parallel tables (minted ids; primary-map writers once keys rebind) through one funnel filling all, asserting per table, with explicit empty slot per rowless case, so raw-writer grep finds only it; no reader guard | Shifted ids; forgotten writers |
| Split a predicate per resolver for a two-representation value; frame two unequal-strength implementations as "where do they split" and extract that predicate once | Drifting copies |

### 5.2 A predicate that cannot under-detect

| Rule | Prevents |
|---|---|
| Walk expressions against an exhaustive allow-list of provably safe forms (a match compiler-checked for new variants, not a boolean, over every value sub-expression), failing closed on unknowns, flagging reads with no nameable root opaque (confirm cost is structurally narrow); never enumerate spellings/dangers; measure refusals and admissions | Quiet omissions; escaping names |
| Extend a classifier, gate or fold arm auditing the path it newly reaches (each defect a missing-answer fallback masked is your regression), a shared one only with full consumer census or per-consumer enabling; take walker polarity from the gate (accept: conservative/exhaustive; reject: positive), reading a catch-all's false as "may reference anything", not "unknown" | Arms answering in refusing lanes; working designs refused; name-blind answers; defects surfacing one by one |
| Close a syntactic walker's blind spot by opting into an exhaustive walker, one axis parameterised (all segments, not head, for callee reach), never a new one; before a new walker read existing filters: the arena is not a tree | Repeated omissions; back edges |
| Census every binder (declarations, imports, aliases, injected bodies, upward routine search, loop variables) before a name-keyed set admits; feed the augmented set only to same-polarity consumers; if the scope model misses a binder (contextual keyword), decline on any token-vector use of the name | Missed binders |
| Never skip an arm's classifying recursion (compile and discard; count calls per arm); list arms answering from a rule, the child walk's too; they still descend for the guard | Lost admission; escaping hazards |
| Mirror or lean on another phase's predicate as the same walk and resolver, not intent: diff both accept sets, ask which names it quantifies and when complete, and if the skip is needed | Fail-open gaps |
| Count the enumeration behind a classifier's "sees all"; treat a region-wide `_ => None` tail as a per-leaf-kind silent-wrong class: list its kinds, ask which the standard makes exact (sign stamp, cast), measure each on every consumer | Unseen kinds |
| Derive written arguments from engine write-back handlers as a write view of the argument table, not the read view's complement; a seed is both | Dropped seed writes |
| List children instead of smearing "unknown"; separate evaluated from unevaluated positions; merge enumeration arms only on shared contract, not type | False stand-downs |
| Widen a lattice, or split a boolean (operand reading vs result sign), until it separates needed answers; a narrower lattice misdiagnoses | Collapsed outcomes |
| Special-case only with a distinguishable reason, checking if users can write the same spelling; before a fallback, ask if a caller decides on the answer, not if the engine shows it | Dropped live paths; demoted lanes |
| Write a width formula in standard form with its domain; extend the walker to its own output when it feeds back | One unmeasured parameter value |
| Make decision queries three-state: never fold "not yet known" or limit exhaustion into true/false; remove limits an iterative rewrite can | Fabricated facts |
| Before reusing a predicate/folder, confirm it answers your question (its consumer's rule: bound/value, saturate/wrap, self-determined/context width); state the delta | Silent mismatched reuse |
| Identify a subject by membership in the set its producer mints, recorded where minted, never by name list, shape or complement; pair an AST-field-free twin by a predicate no user collision satisfies, measuring collisions on PRE | User names passing as ours |
| Where the IR does not keep a type's identity or 2-state-ness, gate on the declaration's own text, not on the kind the lowering recorded | A wrongly recorded kind passing type checks |
| Read a range as written in the declaration only when it starts before its own left bound, since a parser-synthesized range shares one span with its bounds | A synthesized range read as a written dimension |

### 5.3 The predicate must match what it gates

| Rule | Prevents |
|---|---|
| A classifier follows its lowering's resolver and whole procedure (pre-steps like inline substitution; no fixed resolution order) and reads gated properties from the environment it seeded | Divergence under shadowing |
| Narrow a shared gate on the binding a reference resolves to, never a same-named declaration's property; without the binding, leave it | Wrong-twin narrowing |
| Extract a lowering's decision, or a declaration rule's sign half (not by range folding), into a side-effect-free function the lowering also matches on, saying which answers are facts | Drifting mirrors; classifier side effects |
| Make a gate or early-return predicate walk the arms of what it gates and match its consumer set exactly; gate every consumer of one walk on one predicate | Certifying unprocessed sets; diverging consumers |
| Put a dispatch hook at top, detection first; deny-hook every write path | Shadowed hooks |
| Extend additively and fail-closed: a discriminator branch, old path verbatim, proven-disjoint eligibility; a lane placed first gates on a property the old lane's right cells lack, then sweep for movement | Moving right cells |
| Fold a package function in package scope, no module fallback for bare names; a resolver-first hook reapplies the caller's scope rule on the node's root, declining before both resolvers | Wrong same-named objects |
| Give a widthless leaf a tri-state width (unknown, context-sized, known), grep how consumers tell the first two apart; let the evaluator, not the table, size all-context-sized regions | Placeholders read as known |
| Pass position as a flag; a count/size resolver refuses, never reading an environment | Locals supplying counts |
| Qualify "safe to narrow" and low-bit closure as four-state, width-invariant | Deleted unknowns; misapplied fills |
| Distinguish wrong, unknown, right: swapping wrong for unknown reroutes to a conservative path able to drop machinery | Casts skipping context descent |
| Put a wrapper-piercing predicate inside recursion; a hazard refused direct but accepted indirect means the walker is blind one layer down | Partial guards passed as complete |
| Keep the old verdict verbatim behind a new pre-filter so a wrong filter falls back and instrumentation can show it never fires | Pre-filter as second spelling |
| Name in code who establishes a predicate's asserted property (if nobody, establish it or narrow the predicate); ask a precondition about the argument passed, not the declared one | Unbacked assertions |
| Fix a shared kernel's contract for an unfitting operand instead of rewriting it into an equivalent pair | Width caps broken into silent unknowns |
| Before deleting a cap, ask if it bounds values (domain guard) or syntax (capability limit) and what type consumes values past it; measure each side; reject on fitness, not width | Out-of-domain leaks |
| Treat a parser-side fold as scope-free: widening its names means probing every binder and tying stand-down to that scope | Folding shadowed names |
| Bind a declaration's shape sets after the whole declarator parses, at every binder, censusing index kinds; fix an ordering defect by moving a binder before its first consumer (span comparison, split pass), never wholesale earlier | Too-early binding; reordered consumers |
| Verify a derived decision is consumed (a comment claiming a key wins that code ignores usually hides a fall-through re-running its walk); read a commented failed attempt as evidence about it, naming its missing term | Discarded keys; false impossibility |
| State and enforce a widening's property (rules only add candidates) against the whole rule set, not one privileged rule; don't patch shapes | Property breaking elsewhere |
| Never gate a runtime-fact hazard on a static read-set shape (build the wake or file the class); detect runtime-ness by the spellings the lowering arm makes runtime, not argument kinds | Wrong static cuts; constant queries refused as runtime |
| Make a width/sign mirror read the canonical sidecar; on disagreement fix the mirror, then census every context walk treating the leaf as opaque | Container widths for members |
| A hook postponing a statement family captures only members whose whole effect (text, descriptor, control step, side effect) it holds, keeping every item one instance produces; others run in place, loudly | Corrupted member effects |
| Run an existing mechanism's recorded residues on a new spelling reaching it; set synthesized free parameters (holder kind, default) to land on the oracle | Inherited residue classes |
| Select a value's arm by the exact predicate selecting its type's (or return both from one function), checked against every return of the type's producer | Type-value mismatch |

### 5.4 Arms, early returns and escapes

| Rule | Prevents |
|---|---|
| Judge a gate by what it prevents, not its comment: a predicate-comment gap is the defect; re-read guarantees near a changed guard; ask if the lie its comment names is the type's fault, checking what consumers already do | Open hazards; needless declines |
| Adding a selection-chain arm, state declaration properties every arm must preserve and check siblings; a partial accept set decides siblings: list what excluded cases share with included ones | Two answers per declaration |
| Before removing a guard, check what its justification depends on; scope each surviving site of a multi-site guard explicitly, recording accidental rightness; fix a self-firing guard by rerouting, not relaxing | Loud cells going silent-wrong |
| Fence the operand a clamp acts on, not the destination; admission asks about every node the clamp can reach | Lost sign bits |
| If a desugar's parameter count varies, make it uniform per construct and measure a following value parameter | Parameters eating carrier slots |
| Re-stamp a copy alias's sign at the one interpreter read; compiled paths decline on mismatch | Leaked source sign |
| Never split context one consumer reads; "cold field" justifies nothing | Mixed stores |
| Give each unreachable kind its own assertion naming the refusing layer; filter, gate, assertion ask one question | Phantom gate rows |
| Before trusting an early return (no-op arm, conversion's already-right width) or byte identity from a width comparison, count exits (width producer's too) and list what other paths do beyond arithmetic; document differences as intentional | Skipped clears/rules/widths |
| Keep a compensating clear in one place consumers rely on | Copies masking each other |
| Skip what another emitter hoisted by membership in the hoisted set, never re-running admission; resolve it under the lowering's prefix by its function; decide a call-sensitive gate on the statement as written, carrying the verdict on the node's span | Unlowered initializers; hoist-blind gates |
| Judge resolution/classification from an AST-gathered pure-function set, not mutable elaboration state | Phase-only diagnostics |
| Put a stand-down in the arm whose hazard it answers, keyed on the statement; a statement-blind gate cannot decide per statement | Disabled sibling arms |
| Prove death by "written on every path", not "unread before first write", checking the reused predicate's exact success meaning | Stale copy-out |
| Never unwrap a block to iterate statements (drops declarations); test declaration-initializer spelling and subroutine redeclaration too | Spellings disagreeing |
| Write evaluators in target domain, not input limits (bounded leaves don't bound results); run the target evaluator on a rewrite's degenerate inputs before choosing target shape, picking one failing loudly | Out-of-domain results; inherited leniencies |
| Replacing a predicate, count paths it kept from running, prove every old arm's obligation covered (not accuracy), restore a dropped arm by axis (its conditions' questions), not verbatim | Correct-to-wrong cells |
| Check a runtime-predicting fold on domain, resolver, scope; prefer an identifier-free allow-list where order must not matter | Engine divergence |
| Prove trip count and syntactic escape separately before admitting loop-body writes, for all loop forms at once | Erased escapes |
| Expand a branch you'd delete into a truth table and answer each row; where a constraint's justification is only sufficient, write the necessary condition | Unexamined rows; closed idioms |
| In a fast path decide everything, then execute, so a decline has no side effects | Pre-decline diagnostics |
| Collect run observations as they occur, not at run end from movable fields; export a classifier's observations by making it collect reasons, never a new predicate | Silent moves; drifting predicates |
| Going multi-dimensional, handle every offset/stride/direction in one place; set a deferred mirror's offset and direction-dependent kind at resolution; give nested/packed select writes the read's flatten prefix, else fail closed | Double flips; wrong bits |
| Defer unknowns, pre-resolve caller-scope dependencies into the sidecar, lower each argument into every representation resolution may need | Guesswork |
| Define a shadow set by what a scope declared, not what sits under its key; mark added classification items so they neither gain candidacy nor remove others' | False shadowing |
| Before moving an evaluation answer how often, when, what it reads, with an inertness predicate over everything the move passes over | Purity-only answers |
| Before building for apparently missing machinery ("the executor cannot"), fresh-probe the simplest form, check interior mutability and routing, replace a caller's special-case assumption with a lookup (special case an identity, IR byte-identical) | Building what works |
| Capture, mutate, install where source may alias destination; document both cases | Lost values |
| Guard and judge on the value, over lowered IR and every sub-expression, in the funnel seeing it, not on syntax or shape | Pierced shape guards |
| Let the standard decide self-determined positions; ask sign at self-determined width; never fold overflow modulo without context width | Wrong sign/width |
| Define opt-in as where the paired record is reached, not where it can be enabled, counting early returns between | Width on, record never run |
| After widening a producer, enforce what consumer channels can carry where they meet | Wrong widths |
| Grep a value's consumers before saying it decides one thing; split a sentinel not tolerated everywhere; if a fold feeds a value and a test, comment which "no value" the test reads and prove no arm swaps them | Double-duty values |
| Decide relative vs absolute for a recorded string at the producer, spelled in the string, honoured, censused across renderers | Unrepairable relative values |
| Write a width rule as two functions: a shape pass, and an evaluation pass taking the width the shape pass computed | A bottom-up fold at each node's own width, right only where operands already share the final width |
| Gate a named source replacing a literal on constness at the consumer holding the names, not resolvability | Pass-order dependence |
| Cap at source (width-built ranges at the value's real width; recorded quantities where recorded) and fix the producer: release-correct, debug/CI-failing is the worst split | Masked producer defects |
| Make invariants unconditional (asserting tails too; fix unreachable arms leaning on side conditions in the same round) | Conditional properties |
| Treat a non-invalidating memo as a no-change claim: grep every write site; narrow it to a prefix where invalidation is impossible | Stale cached answers |
| Check a helper's documented preconditions at each new call site and build a sibling where they fail; never list its callers in its doc | Misuse past a precondition; stale caller lists |
| Make a mutual recursion's termination structural (every departing call takes a proper sub-expression or one ungated entry) and pin it with a mutant, not an audit of arms | A cycle through a catch-all arm |
| Charge a recursion budget only where a position can re-enter the callee's declaration (a default argument), not on nested arguments, and gate a delegate restarting depth at 0 | Early louds; a depth-reset overflow |

### 5.5 Scope, ownership and order

| Rule | Prevents |
|---|---|
| Keep "which scope" and "whose it is" as separate answers, and change every reader when you introduce ownership | A flush claiming another scope's item |
| Give ownership order and initialisation order separate data structures, and turn an ordering requirement pass order cannot express into data (a rank path) | Orders no pass arrangement can satisfy |
| Separate "runs first" from "creates no event" (order measured right, behaviour wrong: add a phase); open the time gate in the reference gate's slice | Init writes making edges; inert callees yielding the scheduler |
| Disqualify the offending element, not the whole name (things that cannot coexist cannot disqualify each other); decide a LIST-level rule on the sorted list, never per term: drop dead terms, refuse only if none live | Dead branches/terms killing live ones |
| Before widening resolver visibility, census every foreign-prefix folding lane (package/`$unit` routine, generate call site, typedef/formal/return/default range, const-func body), not one per review round; before widening what a nearer scope rebinds, census later resolvers (bare-NAME-typed vars/members follow rebinding; clearing replacements assume completeness), measuring both on enclosing-scope vars and an identical outer type | Lanes only the old blindness protected; retargeted/lost bindings |
| Key a rule on a lowering-context flag (`array_iter`, `subst` stack, `const_call_pkg`) only where the value reads it; census what folds while set (inlined bodies, defaults, interpreted callees) | Rules firing in callees |
| A scratch binding pass at a foreign prefix restores every table it writes, not just its unwind list; record a stand-down after the enclosing scope snapshot so restore drops it, pin both halves | Leaked bindings; permanently loud sites |
| Fold a declared type in its declaring scope (elsewhere decline, never approximate coinciding scopes); store registered bounds as folded literals/unshadowable carrier names, never parsed exprs; a parse-time NAME fold is no literal (misses for-init, foreach, class-param, enum-label shadows), so use sites trust only source literals | Types resolved in a shadowing scope |
| Split a three-question value into components and a two-role func by param, recording each added behaviour's role; nest a dependent field in the variant it depends on | Keys breaking together; regions as scopes; writable invalid states |
| An ownership discriminator must separate the two nearest candidates (check they differ); ask a two-discriminator state via one dedicated predicate | Unsplit nested scopes; half-gated state |
| Count where a refusal comment's qualified condition is false; before approximating, answer "what catches this if wrong?" (if nobody, ask the real question); before adding a conservative signal, check if an existing correction covers a legitimate input-size difference | Unseen third cases; unevaluated misroutes; diverging sets |
| For a re-run fallback count side-effect sites, not ops, bail before the effect; census a moved effect's observers by who reads the resource, not who uses its arg | Doubled diags; empty overlap gates passing |
| Count values whose only consumer was the deleted branch; check what other paths into a guard build | Dead silent verdicts; generated shapes as no-ops |
| A call path's head is the func name/receiver, by segment count; a timing prefix only adds exprs, never makes a stmt unknown; structure fan-out renames a var, not the call arg | Walks missing calls/args or stopping early |
| Apply a monotone invariant at every recursion point; re-check "direct child" rules once nesting can occur; justify an "already safe" early return only where monotone | Loop-only failures; unclaimed deeper keys; skipped ownership changes |
| Make a reader total before opening a row; pin totality structurally | Silent trait-default opt-outs |
| Check each routing bitmap's shape: a handle's slot can be half-dead, so ask membership and a present word | Handle reads routed to an empty heap |
| A refusal keyed on a select BASE's value follows every lowering expansion (parens, both `?:` arms, `let` with/without args, via formals, any depth) by running it, not a bounded copy | Flat bits for elements |
| A route-widening arm excludes what it cannot carry, leaving it on its old route, never refusing; refusing needs an "observable difference" predicate, not "unprovable by the proof walk" | Working designs refused |
| Certify a use's decl where names resolve (parser stamps, elaborate verifies), not by narrowing trust in a NAME-keyed parser map (`var_struct`, `struct_scalar_vars`, `struct_layouts`), which answers for the last writer | Wrong same-named layouts |
| A lowering taking a construct PRE refused inherits every check reading the original stmt, exactly over the lowering's positions and stmt kinds (fewer leaks a flattened block-local, more is false-loud); widen only into walked kinds; trust a decl's type record only as far as the parser's binding there; put a new lvalue-lowering arm below every check documented as detected first; adding a stage outside a primitive, hoist its guards to func top | Silent-wrong/false-loud; new paths skipping guards |
| Carry a SCOPE-resolved construct lowered by POSITION per phase (generate-block decls) only without name resolution: literal inputs written directly in scope; names declared once there, unread from above; "declared above" is no fact (constant above with input below, parser-folded `$bits(t)`, callee free names); between-phase verifying fails on shared keys; refusing illegal pairs needs every binder; `raw.len() == span.len()` marks source literals (synthesized ones carry the replaced span, `dec_range` an empty one) | Each wider rule meeting a new lane |
| Move a symmetric decision into a pure precompute; decide a per-phase re-made choice (generate-case arm) once per instance, keyed by scope prefix and span, and reuse it; caching input availability fails (inputs bind differently per phase) | Order-dependent gates; arms mixed across phases |
| Adopt a SIBLING scope's entry insert-if-absent, once per key across its tables; own defs win; never write enclosing tables | Duplicate statics; loud turned silent |
| A body-lowering lane installs every module-lane gate table (`decl_pos`, block-local maps, routine tables); a shared import guard takes the `$unit`/same-scope discriminator from each call site | VACUOUS gates; wrong import verdicts |
| Keep one current binding per name in all maps, at binders, never by reader precedence: bind a wildcard-imported bare type name once per NAME from who bound it (importer's own typedef, type param, explicit import stays); rebinding one map clears the rest; a scoped one (genvar) suspends and restores | Maps disagreeing on current write |
| A package offers only what it EXPORTS, per its own decls' record; a key registered for another purpose (replay layout) is no export | Leaked imported types |
| Check what a desugar merged before adding a rule judging after it (restore the flag where merged forms' oracle answers differ); state rules in its repr where simpler and more general | Shorthands passing another form's rule; rules missing hand-written part-selects |
| Make implicit decl a phase with one collector, not a use-site action; collect in one pass what must interleave in decl order | Pass-order verdicts; never-interleaving orders |
| Argue a regex from the max range the match can consume; when one spelling has two grammar meanings, the lexer reads the prior significant token; after guarding one direction, ask if the opposite needs it | Isolated-only proofs; spacing-dependent parses; eaten terminators |
| When a gate claims "the engine cannot", read the engine; key it on the storage's set; admit exactly where the lowering can emit copy-out (wider loud, narrower false-loud); merge similar gates | Under-approximating/unaudited gates |
| Attach a precondition to the executor, not the feature; judge executor capability on the whole stmt (rhs can hold effects); never locate a write by stmt shape (value-returning calls appear anywhere); "is an effect" and "can never do it" are separate predicates | Refused designs; no-op assignments; families re-routed loud |
| A set's reader decides where it fills; compare fill/read time exhaustively, a sibling lane's mirror edit included (measure a cell on that lane); fill a sorted pass's read set from the AST before the pass, not per body | Empty sets at read; dead mirrors passing as coverage |
| If a comment says the real condition is unnamed, measure a third time, not guess; gate on the destination, not the body | Reverts losing correct shapes |
| Scope a hazard analysis to the transform, analyse a stmt's exprs as one sequence, channel (un)repairable hazards apart | Hidden/falsely-fixed hazards |
| Judge aliasing by target binding, not spelling; never forbid a name: scopes resolving alike share a lowering | Cross-scope refs killed |
| Use the branch's knowledge of the condition's value, premises measured by an oracle; add a third lattice value, "read-safe, no write promised" | Idioms made loud by one-bit collapse; conditional writes as reads |
| Ask sign on a self-determined walk with the bound walk's admission; when one repr leaks half a family, read both, each width- and sign-correct | Width/sign spellings conflated; negatives saturated |
| Write an opt-in predicate as reachability to the recording site, not the enable condition; ask if a funnel runs before the name resolves | Early exits between enable and record; called mistaken for checked |
| Never gate on an enum catch-all ("implicit" usually means classified, unrecorded); record a needed fact at the syntactic site knowing it, test the degenerate count, put recovered facts last so explicit decls win | Lost parser facts; degenerate keys; overridden ranges |
| Ask a guard in its question's domain: type-of-decl is a decl question, folds-to-type a value question | Legal overrides refused/swallowed |
| Separate "width computable" from "provenance vouched" (every leaf must vouch); resolve scope three-valued: set-or-clear conflates "bound, undeclared" and "never bound" | Laundered provenance; vouching for ancestors |
| Name a stored key's lifetime: a bare-key layout ref dies with its unit; apply each import before the first thing it must be visible to, two passes around the binder, not "earlier" | Untested spellings failing; body imports in headers |
| Key a membership test, new path, stale proxy on the POSITIVE set it means, never absence from a sibling set, "did not fold", an extended predicate; reuse an existing positive set | Neither-set items/declines passing; ever-growing proxies |
| Following an IR field for control flow, read what the runtime reads (an after-the-fact patched field is a snapshot); when a sidecar puts ExprIds where a gate's comment says none, census that position's arena walkers (tier-3 `frames`, wprog, probe) in the slice | Placeholder targets missed; stale-comment panics |
| A first-activation guard is not t0 init: decline inits reading inputs written before first activation (formals, module nets), per declarator unless a declined one READS a hoisted one | First-call args bound; siblings dropped |
| Rebuild a derived summary from ENDPOINTS after a slot-rewriting phase, not OR-ed hops; never let delivery ORDER hinge on a redundant recompute; measure (un)changed shapes on both oracles | Phantom edges; flipped order |
| One engine spelling per value-defined conversion, called by every lane (store, cast, bind; IR-0 compositions inherit limits like `$rtoi` saturation, duplicate operands); destination-TYPE store rules live in the element/field funnel at its width/sign: handle-width pre-coercion breaks wider fields, a still-real lane (push, native store) converts nothing | Lanes disagreeing on one conversion |
| Deliver a zero-delay write at its LRM region's promotion point, never on time advance; land time 0 once the settle CONVERGES after inits, in the one pass evaluating the delayed rhs | Transient baselines; shifted `$random` draws |
| An event is a change after arming: match a level waiter to a per-change sequence stamped at write, not batch dirt or an arm-time snapshot; measure same-time resume order (clock gen first, stimulus from longer delay) before an after-the-arm wait rule | Pre-arm wakes; missed glitches; cycle shifts |
| Order same-time resumes by the scheduling EVENT: delay, `#0`, fork-arm, join resumes number when scheduled; wakes between batch takes share one (decl order inside, after due resumes); a body's arms and joined parent run as it yields; fit every resumption kind | Decl-order resumes |
| Register a declaration whose meaning depends on its scope instance from the walk instantiating the scope, never a structural prescan | Both generate branches registered; unbound genvars |

### 5.6 Domain reference

Each item is a measured silent-wrong source; keep each axis consistent.

| Axis | Rules that hold together |
|---|---|
| Width and type | Self-width table = eval; storage kind before width (handles: width 0); one string router; context-or-plain lowering for target-width fills; four-state raw = value & known; resize extends by rhs sign, stamps target sign; strict real-to-integer guard; two-state unknown-to-zero per write path/storage; string/handle formals in sidecar mask; symmetric signedness across decls; collective comparison/case per LRM; untyped param typed by value, failing open; only a lone constant folds as provably safe |
| Name and scope | Sticky attrs threaded across comma lists; flat map vs nested scopes: lazy snapshot/restore of type and var over the whole decl region; alias/copy side maps name-keyed, set-or-clear; flat registry + scoped resolution = unmodelled precedence, filed as infra; new var binding mirrors decl binding with enclosing snapshot/restore isolation; consumption collect-then-apply, leftovers loud; symbol aliases via the one resolver; sub-select offset minus decl base, clamp loud |

## 6. Measurement

A behaviour claim is settled only by probe, sweep or census; the probe is a claim too.

### 6.1 What counts as evidence

| Rule | Prevents |
|---|---|
| When a counter-example cannot be built, write why it is structurally impossible | A failed repro recorded as "it does not happen" |
| When an outside diagnosis is wrong, ask which instrument would have made it right | The missing instrument going unbuilt |
| Treat oracle-differing anchor lines as findings (measure, record, assign owner); before writing "only stderr differs" or "unobservable", check if severity sets exit class and which stream it reaches | Divergences filed as noise |
| Settle attribution by corpus sweep; label projections, re-measure them at batch close; re-measure a refutation or revert's reason before ranking/building on it | Unmeasured claims cited |
| Run candidate discriminators on the design that matters; build designs testing any bound stated beside known imprecision; count input distribution before claiming control-flow differences reach other bits | Assumptions as measurements |
| If an answer matches what a competing write would leave, delete the competitor, re-ask; name a byte-identity argument's axis and channels (value, diagnostic, exit class, order, time) and which a comment meant | "Same values" read as "same output"; dropped writes as order |

### 6.2 Probes

| Rule | Prevents |
|---|---|
| Probe finer than effect: re-derive an oracle rule at smallest quantity it yields, not design units; overflow destination in width probes; grid finer than delay; timescale precision finer than smallest offset (`#0.5` under `1ns/1ns` rounds) | Probe rounding read as answer |
| Prove a mechanism at band edges (every cell inside moves, none outside); build a twin fixing reported axis, varying the rest | Unproven mechanisms |
| Read bytes via `hexdump -C`, widths via width-preserving paths without convenience conversion; if a probe contradicts a code constant, suspect harness first | Hidden bytes; erased discriminators |
| Read computed widths directly (`$bits`, child's localparam), never from determined output; too-large widths show only where leaf's self width sets region (narrower destination, shift) | Wider wrong widths unseen |
| Never truncate PRE output; re-measure attribution yourself even when lenses converge, reading a debug-only assertion as a hint that release is silently wrong | Pre-existing defects blamed on you |
| Measure ordering rules whole, with per-occurrence witness values, before fixing one symptom; test ordering changes with two same-time processes writing one variable; alias destinations to see order | Invisible order; partial patches |
| Instrument, never audit by eye, answering for every executor/layer even where production short-circuits | Unmeasured paths, layers |
| Flip the default, run the whole suite (always when a backend uses an alternative store) as cheapest coverage instrument, then revert | Alternative path never reached |
| Flag contaminated attribution units, check weight is not one design repeated; read run manifest's backend/refusal fields before claiming backends agree | Skewed or unchecked comparisons |

### 6.3 PRE, POST and sweeps

| Rule | Prevents |
|---|---|
| When changing existing binding/classification, build PRE from `git archive main` extracted to scratch (`tar -x -C <scratch>/presrc`), not a worktree | New louds read as old gaps |
| Score PRE/POST sweeps in three classes (loud-to-correct, silent-to-loud, wording-only), fixed/regressed apart, and ladder rungs by per-file counts and each page's first line, never totals | Totals hiding regressions, pages |
| Measure which cells become and which are correct now: queue row's mechanism, not just symptom; existing lane's answers on new lane's shapes before placing it later (a wrong one forbids that); baseline for shapes a stricter invariant newly refuses (restrict only where it refused) | Breaking correct cells; keeping wrong ones |
| A sweep certifies only axes it varies: list them first, demand missing one from reviewers, claim no zero-regression from own sweep, split narrowing/equal/widening for passed-down context widths, measure fallback plan too | Deciding axis never varied; fallbacks unmeasured |
| Attribute a flip-run failure by running the same test on PARENT under the same flip before filing it against the bundle | A pre-existing backend divergence charged to the slice |

### 6.4 Oracle censuses and their budget

| Rule | Prevents |
|---|---|
| Count no diagnostics with `VITA_SCW_CHECK` on; budget a hand-run census at its sustained rate (`verilator` ~1500 cells/30 min): width subset, one `--prefix` per executable, untrusted-oracle cells hand-IEEE'd, flagged in briefing | Inflated counts; untrusted cells |
| Treat width and value as one answer: pin a non-commuting cell (division, remainder, right shift), ladder changed axis over 8, 16, 32, 33, 64, check value-preserving wrappers at each side's width | Truncated values certified |
| Decide between two rules (or two root-sharing queue rows' order) only on a cell where they differ, check it is the cell measured; if each owns disjoint leaves, ship both, saying what separates them | Coin-flip choices |
| Measure a wider-blast-radius warning at the size where wrong reading enters range | Pricing unneeded sweeps |

## 7. Testing

Full local gate: `cargo nextest run --workspace --locked`. Green in the commit moving them: `sim-ir` schema-hash, frozen-shape, no-float, body-reference suites; artifact header, round-trip; diagnostic-code bijection; parser depth, node-budget guards; live `iverilog` differential; backend equivalence; vendored-libm determinism pins.

A test has teeth if a wrong implementation fails it: a mutation that must die, a control the fix must move, an anchor no shared code can shift; not coverage, green or byte-identity alone.

### 7.1 What gives a test teeth

| Rule | Prevents |
|---|---|
| Prove teeth by failure: new guard's test against reverted binary and deliberate failure; each differential behaviour (from call sites) reverted singly; per-item count that a guard fires on target; unmoving coverage means an unentered path | Vacuous tests, guards |
| Measure entry to a zero-coverage surface by mutation, knowing its blind axes (uncompared fields, summed floors, one-stub aggregates, catch-all failures); pin counts exactly, one property per assertion, never relaxed to floors (cheapen gate instead) | Unprotected fields at full coverage; floors hiding drift |
| Count each statement/effect kind a gate executes, map it to its observer (store, queue, diagnostic, exit code, arm state); diff stdout, diagnostics, their counters (with admitted corpus shapes moving them) and exit class across backends/paths, and every channel of new store points (value, dirty set, edge kind, last writer, waveform, deferred-diagnostic queues); give each axis the value misses its own channel: order via side-effecting operands, placement via boundary sweep | Value-only gates missing effects |
| Make inputs discriminate: diverge two inputs; mutate each sum/product/shift operand with non-identity defaults; desync two sources, assert a call given one returns its value; discriminator off first slot; a name in every arg of new tasks; diverging operations (unequal-length compare, concatenation, replication); values every wrong implementation answers differently (odd for division, past the word for width, negative for sign), commenting why | Inputs passing anything |
| Observe both sides of asymmetric rules apart, test asymmetric parameters in both orders, sweep observation granularity per-event and batch | One-sided or half-swept evidence |
| Keep one condition per question and gate suite green (flip assertion in the edit changing behaviour; re-measure after removing refusal rows); after a fix re-run every mutation, checking what revived and died | Masked or killed teeth |
| Read assertions before calling a test a pin or measurement; assert a probe, battery, sweep or fuzz enters its target branch (counter/ordering assertion; smallest value is often another branch); verify skip/early-return premises on the spot | All cases on one side |
| Test a cache by keeping its owner alive while the input state changes; when ownership makes that hard, build a seam handing over state without the cache | Invisible staleness |
| Pair artifact comparisons with existence assertions; on moving a signal, count every line naming it: update positive assertions, re-aim negative ones; pin a "wrapper covers every site" comment with a test; grep any test a comment cites as a lock | Vacuous comparisons; phantom locks |
| Ask what a cell would show if the feature did nothing; if that equals the expected answer, the cell is decoration | A probe certifying itself |
| Read a verdict path narrower than the failure modes as no information | Green meaning nothing |
| Establish a pin's claimed property a second way, and check a decomposed oracle gives the original form's answer | A path measured as a property; a diverging decomposition |
| Run the fixed cell's neighbours, not the cell; a slice-local pin is a bundle pin (§3.7) | Unconfirmed pins, fixes |
| Write the census first; pin a value as a value, not an exit code: copy the oracle's raw line, pin a residue's observed value (quoting PRE only after running it), write why the number is what it is | Unmeasured or unexplained pins |
| Make a loud pin name its gate, tested by changing a spelling the gate ignores; shape a refusal pin so each gate half refuses it by its own name; pin remaining bypass paths by count per file, naming each one's blocking row | Pins loud via another gate |
| Read the prose beside a passing assertion as an unverified claim: a green test checks its values, never the explanation next to them | A wrong explanation under green asserts |

### 7.2 Anchors, differentials and oracle-free areas

| Rule | Prevents |
|---|---|
| When importing another simulator's rule, list what it merges that vita keeps separate (storage, defaults, identity, event channels), build a twin differing only in each | Premise true of one object only |
| Pin working form beside a non-goal, the opposite half of any rule you fix, both wrap cells (unsigned, signed) | Swallowed forms; fixes moved or undone |
| Use a base case verified correct on all axes but the one tested; nest save/restore tests, never siblings; cross a new axis with each existing one at least once; leave defect's symptom observable in the row's position | Masked or immune defects |
| Guard an invisible failure even if only the next slice recovers its value; justify a guard whose property the return type cannot carry by the rule's other half, not a test | Vanishing values; type-blind comparisons |
| When sharing/delegation grows, name/build the absolute anchor (a design's meaning fixed as a value) protecting the rule, observing only what the mutation moves; check it by mutation; run every anchor on every backend; full coverage can mean no oracle teeth | Backends agreeing on wrong answers |
| Treat product build as oracle-free (alternative executors only behind a default-on feature); flip in current default's direction, both spellings: `Backend`'s `#[default]` and `SimOpts`' `backend:` literal | Vacuous backend comparisons |
| Keep evidence when a subject goes: assert an emptied gate table empty with reason; invert a test (keep design, flip expectation), retiring it once its teeth moved; pin a removed row's neighbours with reasons | Load-bearing designs deleted |
| Wire a losing experiment into the real executor; fill borrowed buffers with garbage before every call, running the whole suite | Untested flags, assertions |
| Classify a survivor as equivalent, blind axis or redundant after checking it changes meaning, write why, delete a check the callee already does | Equivalent/uncovered confused |
| Update a documentation pin by strengthening it, and strengthen the pin guarding a user-visible sentence in the change altering that sentence | Doc falsehoods returning or staying |
| Give a fast path inside the canonical implementation a test-only switch; test equivalence against general path (same bits at two widths; skipped function), not a restated rule or answer table | Tests copying the rule or blind to general path |
| Include reject-arm rows; each, and its neighbour pin, must be refused by the stage under test, not earlier; detect an empty harness with a test pinning feature's refusal | Vacuous refusal pins |

### 7.3 Oracles: choosing, disqualifying and recording

| Rule | Prevents |
|---|---|
| Before recording an oracle split, ask each tool the same bits several ways (direct and indirect interrogator; two spellings in one design; a first read alone and beside a later read, in two same-bit designs); a tool answering differently is no oracle there | Self-contradicting oracles |
| Before an oracle count, split or fix rests on a cell, check it compiles on both oracles, run control spelling (for shadowing, the unshadowed one) on each, again when resumed; a tool unable to bind the construct answers with the outer object: a defect, not a reading | Mis-stated oracle counts |
| Pair every relation pin with an oracle value pin, noted in both files | Uniformly wrong columns passing |
| Write "unmeasured", not "equivalent", without a reaching design; read zero arm coverage as unverified; a linear test cannot see exponential cost, nor a value differential a performance collapse (the gate does) | Gaps closed on paper; unseen cost |
| Before declaring vita ahead, bring a second oracle, count where the claim is encoded, list what flips with it; anchor new benchmarks to an external published reference too | Unanchored agreement |
| Let the ladder settle an oracle split, the spec when each is wrong on a different axis, never a majority; record verdict with its condition; a corpus row on a ruled-unarbitrable axis gets a third state pinning both answers, naming the ruling | Masking oracles; self-certified rows |
| Ask if an oracle exists, measure its scope before use, expecting half an oracle (pin halves apart); a two-state tool answers two-state arithmetic, width and sign: measure first its non-oracle axes (unknowns, out-of-range indices, event/delta order); its zero is two-state-ness, but reject-vs-accept is a real split, off limits | Garbage adopted; axes wrongly parked |
| Build a string-method oracle as a synthetic function, check exact length | Silent oracle padding |
| Remove a test temp dir before creating it; PIDs recur across per-test processes | Full-suite-only red |
| When a reading was wrong, fix prose in place, keep correct assertions, add a fine-grained twin | Deleting correct measurements |
| Record oracle evidence raw: second oracle's output text, crash text (cell one-oracle), iverilog's own run before calling a literal two-oracle (sv2v rewrites literals), oracle defects with width/range condition | Assumed agreement |
| Ask the oracle, running each position, if a sibling path follows or where a new leniency ends (pin it); never argue from consistency or a spec sentence | Accepted typos; unverified growth |
| Pin a refusal's wording only while it has no value, its docstring marking a wording pin, naming missing capability and both oracles' values; once correct, convert it to a value pin, keeping narrative | Bulk breakage; lost reasons |
| When parser may have demoted a lifetime, contrast another type in the same position | One type's qualifier dropped |
| When a fix removes an answer and names one fallback, census every producer not feeding it, measure each on the must-stay set; a cell right by width coincidence is no control | Coincidental producers regressing |
| Before calling a finish-time/cycle-count divergence a defect, check for same-step blocking writes to sampled inputs; oracles agreeing on a §4.7 race is coincidence; non-blocking form settles it | Conformant order as silent-wrong |
| Ask if another lane answers before an external oracle; read cited text when a comment calls another implementation buggy | Unmeasured defects; vita the outlier |
| Read a self-contradiction as proof of a defect, never of which half is wrong; decide the direction by a PRE three-way measurement before fixing | Fixing the half that was right |
| Build the strongest regression test, beside hand-IEEE pins where no oracle exists, as an internal equivalence differential: a new spelling byte-identical to an equivalent verified one, unsupported cases identically loud on both; where both external oracles refuse, vita's own explicit spelling is the regression oracle; test a twin renderer against the runtime spelling in one design first | New spellings drifting; oracle-free cells unpinned; approximating twins |
| Re-read what a failing test models (gates harden against fixes); treat a failing deliberate-loud pin as a claim to re-measure against the oracle, never a regression on sight, moving the file's prose reason with it; convert all of them before re-running (a many-cell test stops at the first) | Correct fixes reverted; pins hidden by fail-fast |
| Assert the value when a pin's subject is a value, and strip every other error source from the cell to confirm it still fails | An unrelated error satisfying a non-zero-exit assertion |
| Pin the fact a self-describing artifact asserts, never its phrasing | A test passing while the sentence is false |
| Give a memo its own test | An unprotected cache indistinguishable from dead code |
| Read what a harness filters before reusing it, and re-check every slice that it installs every sidecar table the feature needs | Vacuous tests; a missing sidecar making the path nonexistent |

### 7.4 The mutation battery

| Rule | Prevents |
|---|---|
| Run battery as `cargo nextest run --workspace --locked --no-fail-fast`, recording each killer; never `-p A -p B --test X` (target filter hits every package); if too slow, re-confirm a generous narrow set's survivors at full scope (narrow can false-survive, never false-kill) | False survivals; one killer as many |
| Count non-termination, crash, leak, retried failures as kills (`FAIL`, `TRY 1 FAIL`, `TIMEOUT`, `SIGSEGV`/`SIGABRT`/`ABORT`, `LEAK-FAIL`); re-run a non-termination kill against own tests only | Hangs scored as survived |
| Write every mutant's expected outcome before running the battery | A wrong expectation passing as another death |
| Add no parameter that no mutation can kill, and before adding a test row name the mutant that would survive without it | Dead parameters; rows that decide nothing |
| Pin a correctness argument: imagine the mutation inverting it and commit the design killing it in the same commit | Reasoning with no pin |
| Treat survival as unexplained: find who re-decides the value, build the one working discriminator however awkward (a defect-finding step), write equivalence argument in code only after failing, prove unreachability by deliberate failure | Real gaps closed on paper |
| Ask why even a fake survival did not die; separate "gate weak" from "mutation insufficient"; read clustered survivors as a test-axis diagnosis, stating their common property in one sentence | Discarded defects; one-by-one chasing |
| Gate every emitted table, workload, golden or differential with an asymmetric one-line upstream mutation (one end of data path) that must move it, pinning the change its numbers must stay invariant under | Unmovable digests; cancelling mutations |
| Pin a differential's non-vacuous count apart from its total; a count no test exercises with two producers together is untested | Hidden self-comparisons; unrun combinations |
| Write unreachable and equivalent survivors differently, proving each shortcut's equivalence/reachability apart; keep an unreachable defensive arm whose absence silently drops the statement, documented as measured dead, how, and honest behaviour | Unreached passing as killed |
| Assert a value a shortcut invents (not reads) equal to the canonical implementation's; pin a cost-only specialisation with an operation-mix census | Premises defended by argument |
| Build what the defect would look like, not a shape turning out fine: add and remove an unrelated statement to see if answer changes; read an unused-assignment warning in a control-flow arm as a possible defect | Idempotent probes; missing branches |
| Restore source and binary after each case from a set derived from mutant list, never hard-coded; compare `git status --short` before/after battery; read a died should-change-nothing mutant as contamination, checking tree first | Mutants left applied |
| Specify substitutions by line number; verify a pattern by counting occurrences, never apply-and-revert; check substitution/build exit codes, grep changed line, record failure as build failure | Stale binaries scored as survival; unrelated files reverted |
| Restore from a byte snapshot of every modified file via `cp` by explicit path (never `git checkout -- .`), under `#!/bin/bash` (`zsh` passes unquoted `$VAR` as one argument), as `trap restore EXIT` with a pre-case diff against snapshot | Lost edits; stacked mutants |
| Score loop-exit mutations only on a bounded runner; run battery in foreground; flush results per line, copy the original outside tree, confirm by PID (not `grep -c`, which false-zeroes) no earlier runner lives | Memory exhaustion; leftover mutants |

### 7.5 Golden, corpus and determinism gates

| Rule | Prevents |
|---|---|
| Put the minimum boundary-breaking condition in the boundary's test (a docstring's wrong argument is no authority); probe order where mapping/execution order differ, a state-moving transformation at the boundary | Undefended boundaries; end-only defects |
| Suspect harness when a probe asserts absence or a differential finds zero divergences: count a positive marker, assert both row counts and key order, plant a known divergence; detect "no output, success exit" in every sweep, grepping corpus when silent | Harness failures read as clean |
| Count diagnostics by a discriminating fragment of rendered form; check a corrected message against cases still rejecting | Coincidental needles; over-claiming messages |
| Fold digest every cycle after reset over the whole run, not final state: every request strobe/byte enable, loads included, plus cycle count; admit a 2-state oracle's digest only after randomising x state over 64+ seeds, with testbench reset asserted by an edge, not a declaration initialiser | Unseen divergences; lucky seeds |
| Put spellings of one meaning (a widened beside the original) side by side in one file, one output line; test routing with domain and scope twins | Unnoticed twin defects |

## 8. Performance measurement

### 8.1 The A/B protocol

| Rule | Prevents |
|---|---|
| Interleave a performance A/B by run, both orders, back to back, discarding the first run and re-measuring the baseline each time; make that the tool's default (measure all at once, round-robin), warning on a debug binary | A sign set by position or drift |
| Measure release binaries only, check binary size when snapshotting, record the profile in the briefing | A debug binary's fake regression |
| Below a one-percent target delta, measure retired instructions, not wall time | Minimum and median disagreeing |
| Give any A/B a harmless control binary (pre-change source plus only the layout change): a percent or two per shape is layout; after extracting a hot tail verify inlining, with shapes never calling it as control | Layout read as execution |
| Check two profiles share a denominator, record both; ask "did it change" in share × wall time, "what is expensive now" in post-change share | Unchanged functions looking larger |
| Bisect a performance regression like a wrong value, with a synthetic probe and a one-attribute control twin; choose a committed control for any performance record | Mixed costs; unreproducible numbers |
| Profile with `CARGO_PROFILE_RELEASE_STRIP=none CARGO_PROFILE_RELEASE_DEBUG=1` and a separate `CARGO_TARGET_DIR`, parsing "Sort by top of stack" | Anonymous frames; root takes all |
| Name the design and workload behind every performance sentence or estimate, and when grading a cost invisible, say what fraction of that design's run it could occupy | One design's result read as general |
| Record the shape a revert's measurement covered, not just its verdict; re-run when a new workload shows that shape | "Buys nothing" true only where measured |

### 8.2 Attribution before optimisation

| Rule | Prevents |
|---|---|
| Use a measured gain only with its mechanism, add no optimisation without a measured gain, and record both absolute times with any layer ratio | Wrong attribution; a drifting path |
| Re-profile before trusting a recorded bottleneck (specialised code barely in the profile means its path is not entered), date and re-measure fixed costs a plan rests on, and check a past attempt's losing reason still holds | Deciding on stale numbers |
| Do the division before building, even for a small-looking candidate: calls moved × cost difference per path, over total runtime; scale a scan's unit by its call frequency, counted by instrumentation | Share read as size or unit; bigger in-function wins missed |
| Ask first if every existing fast path is called, and look for places building a proof then discarding it | Unused paths; recomputation |
| Open the call graph under a profile line; before optimising a hot function census which branches run (the profile says where, not which shape) and count lines that would disappear, not its share | Optimising a rare branch or a name |
| Re-read "allocation choice, not semantics" notes on paths a routing change makes hot, and ask if a hottest-loop clone is required | Free allocations becoming the cost |
| Extract and call the real admission predicate, not a necessary-condition approximation of the boundary | Shapes sent to no evaluator |
| Profile after the census (a remaining rejection list makes no axis worth anything) and write the stop verdict before implementing: targeted profile share against the stop threshold; headroom and a plan capturing it differ, so when remaining stages sum to a fraction of the ceiling, change axis or close and record | Building for a rounding error |
| Measure a shared function's share per layer before claiming a fix helps all, and candidates' ceilings on discriminating designs before ordering them | Inlined copies; ordering by count |
| Before a code-generation plan count executed operations and program lengths (half one-op means the cost is the calling convention); measure choosing a call against what it does before changing the representation | A plan on an unmeasured premise |
| Treat a function you decided to share as a code-generation wall (one spelling and inlined cannot both apply), and a register file as an interface: splitting a statement into two operations sends a large value through memory | Implicit decisions; slower compiled code |
| Give a performance report's root cause a differential, deleting cause candidates by experiment; given a location, build and measure a discriminating question; acquire the comparison tier, not record you cannot; build the cell where the tool does automatically what the report did by hand | A location taken for a cause |
| A/B whether an optimisation accepts a design before claiming its effect; add a shape's benchmark in the change altering its cost | Invisible coverage or cost |
| Count how often a correctness primitive names its operand (performance is also a ladder); count, not time, a slow-looking operator by printing inside the operand and reading the multiplier as an integer; divide a per-evaluation cost out from the count early | Side-effect and width-linear blowups |
| Grep the other callers of what you'll gate, and any predicate delegating to a whole-tree helper then recursing (a promise about the answer is not one about cost) | Missed siblings; superlinearity |
| Fit a curve on enough points, with residuals, before accepting or rejecting a scaling claim | Two points hiding superlinearity |
| License a skip by a complete dependency set, not purity: reproduce the oracles' rule, collect the callee's reads, decline on an unattributable read | Re-evaluating every call each pass |
| Ask if a whole-collection loop's condition can be asked once | A "subset" loop touching all |

### 8.3 Baselines and targets

| Rule | Prevents |
|---|---|
| Baseline on `iverilog` and vita's alternative backend (vita's four-state, event-driven contract); never target a two-state compiled simulator, citing it only as compilation's ceiling, contract difference stated | Contract costs booked as slowness |
| Fix the cost model rather than moving a limit; a node budget replacing a depth cap is still a cap unless the walk deduplicates | The seal vanishing on deep sources |
| Alternate the direction of a fixpoint iterating a map in declaration order, leaving the honest bound in the comment | One link settling per round |
| When extracting a block into a helper, return early inside it if the block read a local remaining at the call site and the node kind links a recursive chain | Exponential double folding |

## 9. Artifacts and determinism

| Rule | Prevents |
|---|---|
| Prefer golden-IR-neutral changes: check the funnel both executors share first, keep non-target designs byte-identical, key a new rule on the new shape, pick a carrier value no older design can hold, make a relative fallback reproduce unconverted producers byte for byte | Unrelated designs or hashes moving |
| `crates/vita-artifact/src/header.rs::CURRENT_FORMAT_VERSION` alone states the format version; never copy the number into prose | A restated constant freezing |
| Bump the format version for exactly a frozen `sim-ir` shape change, a new staged trailer sidecar, or a sidecar enumeration gaining a variant, appended ones included for the accurate mismatch diagnostic; a mid-list insertion is a shape change | Silent mis-decode; vague loud |
| Keep artifacts byte-identical across supported platforms, with frozen `sim-ir` shapes, ahead of performance | Non-reproducible output |
| Before adding a field to a frozen or hashed type, check no node already carries the meaning; an AST field re-pins only the AST schema hash, a value-only change neither | Avoidable or missed re-pins |
| Order deterministically: ordered maps for parser-generated AST items, scope keys sorted numerically, a pass-independent key (source offset) for declaration order, a counter per slot when traversals visit different sets, the whole compared vector written out before citing a sort key | Platform- or pass-dependent order |
| Read the trailer chain (what the pipeline writes, what the staged reader reads) before choosing sidecar or derivation; a format bump is a cost, not a reason for an alternative; over-approximate a pre-/post-resolve divergence with a sidecar flag so both derive from one source | Phases disagreeing |
| Answer a determinism golden reddened by a new non-deterministic field with an explicit declaration (isolate it or make it deterministic) plus an existence assertion beside the exclusion | A vacuous, unwritten property |
| Follow the precedents: system-task ladder (no effect → engine state + side table → engine effect + frozen id + format bump); side-effecting system functions desugared to statements, evaluated once; append-only defaulted engine-facing sidecars; shared buffers taken and restored; multi-item parses via a pending queue drained atop the collection loop; persistent side maps saved, restored and cleared, as scope restore misses them | Artifact and pollution defects |

## 10. Working rules

### 10.1 Files, modules and frozen-type placement

| Rule | Prevents |
|---|---|
| Keep a source file under about a thousand lines, split on approach into submodules with a prelude re-export and crate-visible items, re-exported from the crate root, types kept at the root for private-field access; a single large function or trait implementation stays whole (the documented exceptions) | Unreviewable files; split impls |
| Never move a schema-hashed type between modules (the canonical key embeds the module path): frozen `sim-ir` types and every AST type live at their crate root; keep frozen types verbatim, adding, removing or reordering a field only deliberately, with format bump and golden re-pin in the same commit | Artifact-wide staleness |
| Spell `sim-ir` cross-type fields fully qualified as `sim_ir::Foo`; `crates/sim-ir/tests/body_refs.rs` rejects bare references | A non-canonical registry key |
| After touching the block-body path, check parser recursion with `RUST_MIN_STACK=2097152` (the 2 MiB CI test-thread stack the depth guard is tuned to; local shells have 8 MiB); treat a per-level frame as a budget: extract an `#[inline(never)]` cold helper or box large values in the callee, never the recursive frame, the depth-guard test as canary | Stack overflow on deep nesting |
| Separate concurrent sessions with a worktree | A head moving under another session |

### 10.2 Planning and slicing

| Rule | Prevents |
|---|---|
| Ship the subset that provably needs no prerequisite, proved, not asserted from a naming convention; retire a multi-call-site predicate in its own change, recorded with the measurement and the prescribed deletion | Blocked changes; wider blast radius |
| Pre-verify in simulation the expression a desugar will generate, pin every variant of a context-determined feature before implementing, give a large semantic space its own slice, and record the plan durably | Generated code run differently |
| Order work by risk (pure parser desugar on existing AST, routing to an existing mechanism, composing single-property primitives, new infrastructure), cut slices where the oracle is, and measure the corpus before planning the order | Riskiest first; gates running no corpus design |
| Reproduce an incoming report, then re-find the cause; price a loud-to-correct item in two-oracle cells per edit site before picking it from a multi-site row | Fixing what works; cheap cells first |
| Read a dependency running the wrong way as a sign the rules belong lower, not that a twin may approximate; build no third executor as a substitute (the only permitted separations are role and build); keep the reference interpreter out of performance work: a design the profile finds there should not run there | Extra spellings of the semantics |
| Read "not a product surface" as about the selection flag, not the function | Live code treated as dead |
| Write an option as an enumeration, not a boolean, when the question is which input to pass; claim "the wrong shape is unrepresentable" only when the type makes it impossible, not when the default is merely right | Unreviewed policies; false type claims |
| Price a name-keyed rewrite by where the key is constructed; the absence of a funnel is the estimate | "Small and additive" being a prerequisite |
| Admit a change to implementation with a lane table in the briefing: each shared function it edits or routes into (fold, resolver, walker, classifier or gate with more than one caller), each consumer lane it reaches, each lane marked measured (census cells on PRE and the oracles), opted out (a parameter the lane does not pass, byte-identical) or unmeasured; build only with no lane unmeasured, else narrow to opt-in for measured lanes or file the shared change as its own prerequisite row | Unmeasured-lane blockers burning the round budget |

### 10.3 Comments, documents and queues

| Rule | Prevents |
|---|---|
| Record a finished slice only in its commit message (`§4.5.N`, the bug, the mechanism, byte-identity, the review): no archive entry, snapshot row, lessons entry or REMAINING_WORK rewrite; find a past slice with `git log --grep` | Per-slice doc writes no later step reads |
| Keep ROADMAP to open work, one line per item, and put no gate or test counts in any document | Prose and counts going stale |
| Write a queue edit, deferrals included, into the canonical queue only, deleting mirrors, not syncing them; give a question one canonical section, splitting it where two are needed, and verify a doc migration by sorted line sets (`comm -23`), not headings | Stale copies; items lost in a move |
| Treat a document enumerating a parallel-table set as code, updated in the same edit; say what a constant is for and point at its canonical site, not restating it, grepping the number when a bump ships | Enumerations and constants decaying |
| Re-prove in this file any property a comment asserts: diff both bodies for "twin of", check "duplication avoided" against the code, read other copies of the rule, and re-verify every sentence of a comment block you open to fix one | Copied or neighbouring claims false |
| Name what actually holds an invariant, and the reason an implementation is there when it must be named | Leaning on a false argument |
| Say what a temporary workaround is for and remove it with the root fix; delete a caller-less macro or helper but leave the reason for its deletion | Workaround defects; toothless gates |

### 10.4 Tooling and machine safety

| Rule | Prevents |
|---|---|
| For a scripted splice, assert the anchor exists, then verify the result and read twenty lines either side of the insertion point | A stolen doc block; no-match as success |
| Require a full diff and an oracle re-verification for any agent given write access | An unnoticed whole-file replacement |
| Make each edit an independent write and grep that it landed before the comment or report, a multi-edit script reporting failures and continuing; after a result not visibly complete, re-establish state (`git status --short` and `git diff --stat` for an edit, re-read a write, re-run a command) | Reasoning about writes that did not land |
| Rebuild after changing build configuration (product and oracle share a target path); verify a feature flag with `cargo tree -p <crate> --no-default-features -e features`, reference dependent crates with `default-features = false`, rebuild, and test the feature-off build as its own CI job with `--lib` (unification and a test target's dev-dependency re-enable it) | Testing a binary you did not mean |
| Never run two full suites concurrently, add no pre-emptive build before the test runner, and never send an uncatchable kill to a runner mid-build | Shared-path flakes; double builds; stale locks |
| Redirect a gate's output to a file, capturing its exit code separately (`cargo … > /tmp/x.log 2>&1; T=$?`), and after an unavoidable pipe read zsh `${pipestatus[1]}` or bash `${PIPESTATUS[0]}`, not `$?`; write a parallel process's log line in one write, aggregation flagging contamination on a value outside the known set | Failures read as green; torn rows |
| Take a hang out of the battery and measure it once by hand | A child still writing to the pipe |
| Wait on a process identifier (`while kill -0 $PID; do sleep …; done`), never a `pgrep -f` pattern matching the waiting shell; run a wait's predicate once by hand before arming it, prefer an inspected artifact's condition, and kill the wait when the answer arrives another way | Deadlock; waiting on a string never written |
| Grep every site deciding a default before flipping it; treat a revert as an edit: specify the deleted range by line, grep deleted symbols for surviving references, check adjacent tests, documents and helpers survived | Partial flips; invisibly deleted tests |
| Before a new error from a lint-class rule, run one shape per file through the second tool and put the table in the module doc; a shape it accepts is a warning at most, an unrun shape "unmeasured", never "accepts" | A rule failing in-tree fixtures |
