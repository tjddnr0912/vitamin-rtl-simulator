# Engineering rules

This is the rulebook for changing vita. It states how work is decided, measured, reviewed and gated:
the accuracy ladder every change is scored against, the two adversarial review lenses, what a census
is and what makes one valid, how to write a gate that does not under-detect, what counts as
evidence, what gives a test teeth, how performance is measured, and the artifact and determinism
discipline. Each entry is one imperative plus the failure mode it prevents.

## 1. Scope and standing

This document is canonical for method. Where another document in this tree describes how work is
done and disagrees with a rule here, this file wins. A rule learned while implementing is merged
into the matching section as one line; the measurement that produced it goes to
[history/lessons.md](history/lessons.md).

Read this before implementing. The queues of open work are in [ROADMAP.md](ROADMAP.md) (§2
silent-wrong residue, §3 loud-to-supported, §6 observability) with a snapshot in
[REMAINING_WORK.md](REMAINING_WORK.md); the specifications the review checklist is drawn from are in
[preview/](preview/); the user-facing surface a change must keep true is in [manual/](manual/).

Vocabulary used throughout:

| Term | Meaning |
|---|---|
| **silent-wrong** | A wrong answer with no diagnostic and a success exit. The outcome this repository exists to prevent |
| **honest-loud** | A refusal or diagnostic that names what the tool cannot do. Always safe, never as good as support |
| **correct support** | The construct runs and its value matches the oracle |
| **PRE** | A binary built from the tree before the change, extracted with `git archive <branch>` into a scratch directory and built separately |
| **POST** | The binary built from the tree with the change applied |
| **lens** | One adversarial reviewer with a fixed attack method. Two are mandatory: differential and soundness |
| **census** | An enumeration, from the source, of every site that can reach a question, each cell measured rather than argued |
| **cell** | One design plus one measured output, in a census or sweep |
| **control twin** | A second cell identical except for the axis under test, used to attribute a result to that axis |
| **anchor** | An expected value fixed independently of any implementation, so a change to shared code cannot move it |
| **hand-IEEE** | An expected value derived by reading the IEEE 1364/1800 text, used where no external tool can arbitrate |
| **oracle** | An independent tool or standard text that decides a cell. Live oracle: `iverilog` plus `vvp`, invoked by the differential harnesses. Second opinion: `verilator`, obtained by hand on 2-state arithmetic, width and sign |

## 2. The accuracy ladder

### 2.1 The ladder itself

| Rule | Prevents |
|---|---|
| Rank every outcome on one ladder: silent-wrong worst, honest-loud always safe, correct support best. Climb it; never descend | A change that reads as an improvement being a descent |
| Do not implement what cannot be verified: with no oracle and no precondition, stay loud | An unverifiable implementation shipping as a silent-wrong |
| Treat making a working construct loud as a regression; loud is earned only by what the tool genuinely cannot do | A refusal added for tidiness stopping designs that worked |
| Treat a panic as loud, not as silent-wrong, and never trade correct support for loud in order to remove one | A net loss on the ladder dressed up as robustness |
| When soundness and differential conflict, the differential wins | An internally consistent argument overriding a measurement |

### 2.2 Never trade one wrong for another

| Rule | Prevents |
|---|---|
| Never trade one silent-wrong for another: convert types at the context boundary, not at the leaf, and stay loud where the context domain cannot be built | A leaf conversion destroying the value before the enclosing operator sees it |
| Remove a feature whose partial support only trades one silent-wrong for another, and build the consumer-by-value matrix before removing, because removal is an edit | Shipping "the better half" of a broken feature, keeping a silent-wrong and adding a regression |
| Treat a clamp as a silent value change: read the sign in the domain before the clamp and let a shape the clamp cannot carry fall through to the path that is already right | A clamped negative becoming a legal value that then fires |
| Equalise head-on an asymmetry where one path is narrower than its twin, prerequisite slices first, rather than routing around it | The workaround becoming the next change's constraint |
| Support only the cleanly verifiable subset where interaction is unpredictable, and keep the rest loud | Forcing support and getting a silent-wrong |
| Do not widen a loud into a silent: a pre-existing root cause does not license expanding the surface. Defer the root and keep the surface loud | The widening being your own regression |
| Treat a fix or a routing that interacts with a latent gap as a regression even when the value is right | The value being correct and the run being worse |
| Prefer a loud guard to partial normalisation for a deep residue | Partial normalisation being a silent-wrong |
| Enumerate the old rejects exhaustively and run a live-oracle differential before relaxing a blanket reject; the teeth are zero regressions against a full sweep | Shapes the blanket reject hid becoming silent-wrong |
| Do not close a queue item by refusing | Some consumer eating the decline as a silent default and producing a wrong value |
| Build both representations and measure them; when both are trades, remove the axis and file honest-loud plus a deferral | Shipping the half that looks better |
| Recognise that a leaf-patching resolution arrives after the enclosing context is decided; support that requires re-lowering the enclosing node cannot be built that way | The cast, concat and width context being baked around a placeholder and the leaf's bits read later |
| Ask whether the consumer can produce the right answer from a representation, not whether the name resolves, when choosing a storage table | Registering a value in a table of the wrong type, which resolves the name and makes the value wrong |
| Read how the neighbouring branch handles a cap before writing a new branch that meets it | A new branch adding a silent clamp where the neighbour raises an error |
| Ask what a halted body leaves before asking whether the statement may run; where nothing is defined, the feature does not exist yet | Every choice trading loud for silent because the outcome was never specified |
| Close every leaking sink before changing the value that leaks | Suppression at one sink plus a mid-body bail turning the other sinks' values into a different silent-wrong |
| Find the code that still runs while a non-clearing latch is true before using that latch as a predicate | Confusing "finishing now" with "already finished" and deleting the output of everything that runs after |

### 2.3 Accidental correctness and cancellation

| Rule | Prevents |
|---|---|
| Check that a static claim you begin consuming is true of the runtime value, and look for the place where two errors were cancelling | Fixing one side breaking the cancellation and turning a correct cell silent-wrong |
| Ask what a conversion was hiding before removing it; the signal is values that are right for the wrong reason | An accidental immunity that is load-bearing being deleted along with the conversion |
| Gate on whether the picture is trustworthy, not on a list of shapes, when a fix materialises a boundary: decide trust by comparing two sources, and check whether the old cells were right only by cancellation | A fabricated answer being as dangerous as a missing one |
| Measure what an accidental loud was blocking before removing it, and promote the accident to a rule when it turns out to have been right | Deleting a coincidence that was doing real work |
| Treat a pre-existing defect in another syntax as a queue line, never as a licence to widen the same defect into a new syntax | The new path inheriting the defect deliberately |
| Fix the producer when a reported regression only reads an input that was already wrong, then run the corpus, because the producer feeds more than the consumer in front of you | A path that is right by accident becoming wrong the moment the classifier reads the truth |
| Feed the input to the existing callers before adding another call to a shared helper; when they are wrong too, the fix belongs inside the helper | Guarding the call sites and fixing only your own regression |
| Measure the premise of a theorem used to narrow a change, especially a data-structure invariant such as "the stored representation is canonical" | A narrowing resting on a premise nobody measured |
| Draw a boundary and keep pre-slice behaviour when two readings are wrong in opposite directions; a guess means out of domain | Either reading breaking a different shape |
| Treat an IR construction that places one expression twice as a semantic change, and gate on observability (side effects, draws, diagnostic counts) before duplicating | One operand being evaluated twice and the two draws mixing |

### 2.4 Width, context and provenance

| Rule | Prevents |
|---|---|
| Treat a cast as the operand's context, not a later truncation: `N'(e)` evaluates `e` at `max(self(e), N)`, so a walk that cannot carry a width must decline | Folding at the operand's own width and resizing afterwards, which is a different operation |
| Fix context misuse by fixing the propagated value (`max(self, N)`), never by discarding context | Discarding context also losing the signedness, which always propagates |
| Hand the context width to the operator, not to the leaf: a context-determined operand evaluates at `max(context, every self-determined operand's width)` | Freezing the context at the leaf and losing bits the moment a sibling is wider |
| Give a routing gate and a soundness guard different predicates: over-reporting is free for a router and a loud regression for a guard | One predicate for both putting a self-determined position into the hazard set and producing new louds |
| Walk each caller to the end of its chain and say what is there before writing that a decline is free | A decline being free only where the pre-change fold would also have declined |
| Count the loud-to-value column on its own and give every axis those new cells touch a control twin without the new construct | Opening a fold's accept set inheriting whatever is already wrong underneath |
| Fix a width-correctness defect at the consumer's fold, not with an admission threshold on the producer: record each constant's declared width and sign beside its value and fold per the standard | A threshold answering the census cell and not the class, so the defect reappears one width up |
| Ask which rule sizes an initializer before asking which consumer to keep loud; do not decline what a type rule answers | Declining the consumer keeping the loud and freezing a scalar spelling's pre-existing silent behind it |
| Answer a query about a shape the parser flattens where the shape still exists, with a parse error for an index that cannot be folded rather than a fall-through | Whoever flattens a shape leaving every query on it to a layer that cannot see it |
| Ask whether the source of a width is still alive after asking where the width came from: a width inferred from an initializer's value does not survive an override, a declared type does. Check every override channel | An overridden parameter binding at the initializer's width |
| Gate on a width a declaration states, never on a width that may have been inferred from an initializer's value | Reading a width off a default that an override replaces |
| Enumerate who reads a stored value before making it more precise; when the readers cannot use the precision, the asymmetry being removed was the safer state | A canonical producer against consumers that still guess |
| Carry the source's signedness on any channel that moves a value across a width boundary, and decline rather than pick a default where the channel cannot know it | Several expressions arriving as one integer and the container's sign being right for only some |
| Count what a domain cannot carry before widening it, and decline what it cannot represent | A fold silently dropping a width or a byte, turning loud into silent-wrong |
| Refuse at the binder a shape the machinery cannot represent, rather than leaving it to a rewrite's arithmetic | A key reused across different shapes reading one index as another dimension |
| Read every guard's own words when widening its caller set: if the sentence names a construct, the predicate must test for that construct, and a shared lowering must recover the lane from something unambiguous | A guard written for one lane becoming a false-loud whose message contradicts the source |
| Measure the three lifetimes of a name before hoisting a block-local declaration into an enclosing scope: shadow, sibling and leak. Build all three probes before writing the hoist | A hoist without a scope trading loud for silent-wrong |
| Deliver a feature the AST cannot carry by a desugar that keeps every axis the consumers need, turning the axis the desugar cannot honour into a loud refusal | A desugar silently dropping the axis it cannot express |
| Ask what the next consumer of a text keys on before a preprocessor stage rewrites it, and keep the form the oracles keep | A normalising joiner changing which bytes a later directive consumes |
| Route any provenance emitted inside an expansion through the collapse site | An offset being meaningful only in the buffer it indexes |

### 2.5 Declines, defaults and folds

| Rule | Prevents |
|---|---|
| Make the sub-shapes a new match arm cannot improve call the arm the expression would otherwise have taken; a decline is a change, not a no-op | A new arm owning a shape it cannot answer and the caller's default firing |
| Keep a fold that predicts runtime behaviour admission-only where possible, so a disagreement with the engine can admit an out-of-range copy (loud) but never a different word | A fold that disagrees with the engine writing the wrong word |
| List a shared fold's consumers before widening it and ask for each whether its context rule is exact for the new leaf; decline at that consumer for the delta only, documented as a delta-limiter | A widening that fixes many cells on some consumers and silently breaks a few on another |
| Count how every consumer of a fold consumes a decline before adding a refusal; a refusal is only as loud as its caller, and making the unknown knowable usually beats widening the refusal | A refusal producing a silent default at a success exit |
| After deriving an equivalence, read the code back and name every input the derivation assumed, then build the design where that input is a default | A provably value-neutral rewrite being a silent-wrong because an input is a fabricated default |
| Audit the path a widened classifier will newly reach before widening; a missing-answer fallback masks defects rather than fixing them, and every unmasked defect counts as your regression | Defects appearing one after another behind a widening |
| Ask before starting how many properties a site's consumer expects and whether enabling one contradicts the rest; partial application of interacting properties can be worse than none | Applying one property alone regressing cells that the pair would fix |
| Close a silent default upward by making the unfoldable shape foldable, rather than turning the default loud, and include cells whose true value equals the default | The subset where the default is accidentally correct going correct-to-loud |
| Make a structurally invisible omission loud: assert on the success path that the pending set is empty | An unclaimed item being simply absent from the IR, with no diagnostic |
| Treat folding a constant into an initial value as removing that value from the initialisation order, not as a pure optimisation | The same read answering differently for a literal and a call |
| Check what the generic path does not evaluate before admitting a node: laziness is a diagnostic question, not a value question | Eager evaluation adding a diagnostic that did not exist and splitting the exit class across backends |
| Recover a statement boundary with an exhaustive predicate rather than approximating it per operation | Returning between two operations losing the write and the diagnostic it owed |
| Find out who answers now when a refusal is made unreachable | A stale neighbouring arm beginning to answer at the wrong width |
| Look for a second, independent sufficient condition when a sound gate kills the feature; weakening trades soundness for coverage and a disjunct does not | Every weakening either re-admitting the counter-example or still refusing the target |
| Enumerate the writers when a change's soundness rests on "this can only happen once, here, or never" | A premise about the engine's own behaviour that the engine refutes |
| Freeze one half at the old decision and write the reason when no axis separates two groups; a freeze is an admission that they are different questions | Forcing one rule over both halves and breaking the half it does not fit |

### 2.6 Guards, gates and what removing one promises

| Rule | Prevents |
|---|---|
| Check that a general floor does not remove a diagnostic that was masking a different, still-broken span | Adding a floor turning loud cells into silent-wrong |
| Read "fixing one arm makes the inconsistency observable" as a reason to check the siblings, not as a reason to leave them | A sibling defect being protected because fixing the first arm exposed it |
| Read landing on one oracle where the tool answered neither as a rung up, not as touching a split axis | A genuine improvement being refused by the rule against split axes |
| Run a three-way census of every masked shape before removing a masking loud guard, and keep a loud with corrected wording for what cannot yet be fixed | The pre-existing silent-wrong beneath becoming yours, with the original wording handing the next reader a wrong root cause |
| Walk the write path to its end and name what it lands on when a read fix routes a name away from an object; give the write its own refusal at the lvalue funnels, not inside the shared resolver | Fixing the read moving the write from a wrong object to a wrong bit |
| Fix a stale-read defect at the read, not at the store: change what the reader resolves to, statically, in the one sidecar every backend consults | A store-side forward reordering every settle consumer, digest and record order |
| Close what a removed loud gate exposes in the same slice | The pre-existing defect becoming yours because you widened its reach |
| Cut a reject gate from a hazard set measured on a PRE build, never from a proxy; a proposed gate predicate is itself subject to measurement | A proxy predicate false-rejecting byte-correct designs in bulk |
| Run the real design a loud gate was blocking, not a reduction of it, before removing the gate; removing a loud gate is a promise about everything underneath | Independent pre-existing defects being invisible to a minimal probe |
| Say so when a handler cannot use an argument, and read the neighbouring branches: if one warns and yours does not, the asymmetry is the defect | A bare return being indistinguishable from "there was nothing to do", the one outcome no user can debug |
| Record a partially fixed count in the queue with the number, in the same slice; for a side-effecting operand the count is the semantics | A partially fixed silent defect looking exactly like a fixed one |
| Make a second pass verify rather than overwrite when a computation is done twice: record what the first pass produced and make a mismatch loud | A consumer running between the two passes keeping the first answer while everything after keeps the second |
| Make executor selection ask a latched fatal first; latching is not skipping execution | Execution continuing past the latch and failing at an unwrap |
| Read the ladder per build: a change is a promotion only where the fallback is not compiled | The same change being a promotion in one build and a descent in another |
| Narrow the coverage, not the semantics, when something cannot be proved | Changing runtime semantics in order to admit everything |
| Judge each declarator of a declaration independently and split only where the verdicts differ | One name's verdict discarding the whole declaration and making every later use undeclared |
| Gate interacting properties on one precondition, all or nothing, and leave pre-slice behaviour verbatim when the precondition is absent | Partial application being worse than none |
| Refuse where a snapshot mechanism has one slot and the expression needs more | Two calls in one expression, or a recursive call, reading the same overwritten snapshot |
| Catch what an executor cannot do in the executor, not in elaborate, when a body is lowered into two copies and the caller may use either | An elaborate gate on one copy false-louding designs that run through the other |
| State the fallback that leaves existing answers literally unchanged when adding a new claim, and move a pre-existing strictness asymmetry in its own slice | A new catch-all changing an unrelated verdict |
| Check whether the workaround for a refused construct is itself blocked, and update a message that lists supported positions whenever the capability list changes | The user searching for a workaround that does not exist |
| Refuse in elaborate what elaborate cannot do; a debug assertion disappears in release, so a panic is not loud | Release silently doing the wrong thing |
| Ask the direction table for an argument's direction: an output actual is a write destination and an `inout` is both, which is a stand-down | A snapshot redirecting the destination so the write disappears |
| Give a verdict name a documented contract and check the promise is literally true at every new site | A "writes on every evaluation" verdict given to a conditional write, and a short-circuit path reading a stale value |
| Poll a latched fatal inside the statement loop so the process stops at the fatal point | A fatal that does not stop, letting a testbench print its own success line afterwards |
| Do delimiter matching on the token stream, not on raw text | A delimiter inside a comment or a string closing a construct and making the rest executable |
| Make an unmatched opening delimiter an error | A diagnostic-free fallback making the defect non-local, dependent on the whole compilation unit |
| Ask the storage question on the write path wherever the read path asks it | A frame-local value writing unconditionally into module storage and silently not changing |
| Match the value to the oracle and state the risk in a warning: correct-or-loud means "do not let it go unnoticed", not "change the value" | Truncating silently, or refusing legal code |
| Treat non-conformance plus "the user cannot change the source" as a gap even where a document calls the refusal a deliberate policy; a warning can buy the safety the refusal was buying | A vendor-supplied library that cannot be simulated at all |
| Route initializers by block; splitting one block's initializers into a main sweep and a trailing group destroys declaration order | Interleaving disappearing and draws coming out in the wrong order |
| Wire rather than refuse when the key is wrong | A refusal papering over a mis-keyed lookup |
| Inherit the old storage class's capabilities when reclassifying into a new one, and register in both tables only when the two representations match exactly | Capabilities going false-loud on reclassification |
| Kill only the events you created: split the dirty list rather than clearing it, because a value can be restored and an event cannot | A previous stage's events disappearing, since re-writing the same value is not a change |
| Write a truth capture as a negation of a negation, never as one expression named twice | Naming the same expression identifier twice making the engine evaluate it twice |

### 2.7 Diagnostics and observability are product surfaces

| Rule | Prevents |
|---|---|
| Treat a machine-readable rail that misdescribes itself as a silent-wrong of its own kind, because its audience cannot check it | A wrong manifest being graded as a documentation nit |
| Report a diagnostic against the user's own name once, never a synthesized carrier name, and suppress it when the primary carrier is equally unknown | The diagnostic leaking an implementation name and misdiagnosing a construct that is simply not overridable |
| Treat a wrong observability log as a silent-wrong: derive every observed value from the single engine source, allow-list value exports to formatter-supported kinds, keep unparsed probes loud, and gate with a three-way comparison plus a determinism golden | A wrong log misleading its only audience |
| Put the reachable cause first in a user-facing refusal message | The message advertising an unconstructible cause and omitting the real one |
| Publish the same values you decide with: when you add a judging layer, follow its reason string to wherever it is reported, in the same commit | A rail reporting "nothing was refused" while a layer refuses locally |
| Do not let a desugar's diagnostics share a code with the constructs it desugars, and reserve the name space a lowering uses as a channel | Warning suppression moving a simulator-generated fact and a user-called task together |
| Report from the place that holds the reader and the sink together, not from where the value is produced | A range diagnostic landing after the line it belongs to inside one stream |
| Carry the execution context in the same record as a static capability census | A count from one executor reading as though another had run |
| Put the identifier and the discriminating rule into a new loud message, and inject a resolver through a single trait where a layer has spans but no resolver | Many copies of one sentence carrying the information of one, with no anchor for the agent rail |
| Count diagnostics per user-written construct, using the oracle's count as the standard, and save and restore the duplicate-suppression flag per construct | Reporting per leaf eating the error cap faster and deleting unrelated later diagnostics |
| Write the role, not a capability list, in user-facing documentation | A capability list being false the moment a feature opens, with no test to catch it |
| Capture the pre-expansion argument vector at the point it exists | A wrapper's substitution, a filelist expansion or an environment knob leaving no trace, so the run cannot say what it compiled |
| Emit observability output in the same process and stream as the run | A dump subcommand that exits being unable to coexist with a run log |
| Send observability output through the same writer as diagnostics | A separate print not being captured, and its order being wrong |
| Stamp a derived value with its provenance: flag, environment variable, or automatic | The hardest case to find being the one that came from neither flag nor environment |
| Do not break a line between a flag and its value; carry a long value past the margin instead | A wrapped flag reading as a bare flag plus a stray source file |
| Read a defect an observability feature exposes as that feature's first proof, and comment the canonical site of any list-of-flags predicate with the reason it must be updated | A frozen flag list rewriting later flags' values as paths |
| State only what a diagnostic knows and list the conditions; do not infer | A message sending the reader after something that does not exist |
| Carry the defer-time span in the defer record and set it in the resolve loop | The only diagnostic in a log having no file, line and column |
| Grep the wording of a restriction in diagnostic text when a change lifts it, and re-derive each site's reason separately | The tool telling users that working code is illegal, and a blanket replacement making one site false again |
| Point the caret at the operand the message is about, copying a neighbour that already does | A multi-line condition sending the reader to the wrong token |
| Apply an escaping rule at every site that prints the value | A control character in a name splitting a warning across two lines |
| Refuse a per-call profile that cannot see every execution path rather than shipping it, and publish the blind region's map instead | A rail reporting zero where it cannot see, which reads as free |
| State an attribution convention in the artifact and verify it by construction | A consumer having to infer the convention from prose, and a reintroduced double count being invisible |
| Run the design and read the output before writing what the output means; when one claim appears in a diagnostic, a docstring and a comment, fixing one fixes a third of it | A message describing behaviour the executor does not have |
| Record a refusal as a first-class expected state, separate from a run with an exit code, so refused-as-pinned, refused-for-another-reason, promoted, and refused-becomes-silently-wrong are distinct outcomes | The one move the ladder forbids being graded as a promotion |
| Make a give-up state a value carrying a span and a reason, not an empty answer | Diagnostics with accurate locations that cannot be narrowed |
| Check that a node's diagnostics moved with it when code rebuilds an operation in its own spelling | A value-only differential being unable to see that the same expression is loud in one form and silent in another |

## 3. Review method

Every design or implementation change gets an adversarial review of at least two lenses,
differential and soundness. A design that changed during review is re-reviewed.

### 3.1 What a review is

| Rule | Prevents |
|---|---|
| Verify a suspected silent-wrong by reproducing it against a live differential oracle, not by argument | A defect that is argued about rather than reproduced being neither confirmed nor refuted |
| Run the inspection with the roles separated: author, moderator, reviewer, recorder | One agent playing every role and validating its own reasoning |
| Use the specifications under [preview/](preview/) as the review checklist; where a separate checklist exists, add it rather than substituting it | A review with no predefined checklist inspecting whatever the reviewer happens to notice |
| Review on four axes: architecture and system integration, performance and efficiency, maintainability and readability, robustness and testability | A single-axis review missing the defect classes it never asks about |

### 3.2 The briefing

The briefing decides what a round costs, so it carries all of this.

| Rule | Prevents |
|---|---|
| Build the PRE binary before the briefing and hand the reviewer its path | A reviewer building its own PRE overwriting in-progress work or measuring a different tree |
| Tell reviewers explicitly not to touch the working tree | A lens restoring files to build PRE and destroying uncommitted work |
| Take a snapshot commit before briefing and hand PRE out as `git archive <branch>`; that commit, not a scratch directory, is the restore canon | A scratch snapshot disappearing and the tree being unrestorable |
| Give the reviewer the list of already-killed mutations and documented survivors, and require findings outside it | A round re-deriving the previous round's results |
| Name the previous round's numbers in the briefing as re-measurement targets | Prior numbers being inherited as facts and never re-checked |
| Say that reporting clean is a good result and that findings must not be invented | A reviewer under implicit pressure producing noise |
| Allow the soundness lens a separate `CARGO_TARGET_DIR` for mutant builds | Mutant builds colliding with the session's build state |
| Aim later rounds only at what changed since the previous one | A full re-review spending the budget on settled ground |
| Require a build with `--features separate-bins` when the staged binaries are in scope | A stale staged binary replaying pre-fix behaviour, with the finding attributed to current code |
| Hand the reviewer the measurement table (cell by oracle by PRE and POST by classification) as a file, and open with "attack outside this table" | The reviewer rebuilding the table from scratch |
| Write the budget into the briefing in tool calls and designs, and require a report of what was found plus what to do next when it is exceeded | A review without a budget always spending the whole of it |
| Hand out a snapshotted binary and record its hash; when a blocking fix lands mid-round, re-freeze and say which binary the numbers describe | Lenses scoring different builds, so every finding has to be re-measured |
| Keep attribution per slice when several slices share one review: disjoint files, one PRE and one POST binary, a census per slice with its own cell prefix, and questions grouped per slice | A finding that cannot be reverted without touching the other slices |
| Do not rebuild the binary while a reviewer is measuring; make changes in a copy and re-review afterwards | The reviewer having to annotate which binary each measurement used |
| Require an explicit non-vacuity proof: byte-identity means something only when the fast arm actually fires, with the firing count and observed argument values recorded | "Nothing happened, so they were the same" being indistinguishable from a working optimisation |

### 3.3 The differential lens

The differential lens reproduces behaviour against a live oracle and reports, per divergence, the
oracle's raw output text and a classification. Its report names the probe resolution used.

| Rule | Prevents |
|---|---|
| Compare semantic equivalence, never structural | Structurally different but semantically identical output reading as a divergence, and the reverse |
| Classify every divergence four ways: real gap, no-oracle, vita-ahead, harness format | Undifferentiated divergences all being treated as defects, or all dismissed |
| Run the module twin of every interface cell the lens files as pre-existing; when the twin is wrong the same way, the row is a shared-model row, not an interface row | An interface row filed for a defect that lives in the shared model |

### 3.4 The soundness lens

The soundness lens argues from the source and the standard, and its premises are censuses, not
prose. Commission it explicitly.

| Rule | Prevents |
|---|---|
| Commission the soundness lens for: all-sites and variant enumeration, disjointness proof, same-name collision, guard traversal completeness, and an audit of the population path of every map being consumed | A soundness lens without a task list checking whatever it finds interesting |

### 3.5 Rounds and deltas

| Rule | Prevents |
|---|---|
| Make a later round a delta briefing: the changed hunk list and the already-killed mutations, plus a demand for findings outside them | A later round re-measuring the first |
| Re-review after fixing a blocking finding: the fix is a new mechanism no lens has seen, and the reviewers' existing reproduction is the first thing to mutate | The next round's blockers all sitting inside the previous round's fix |
| Treat the fix for one round as the finding of the next: a delta round is not optional after a design change, and its brief must name the delta | Each round correcting the previous correction, none found by the author |
| Read a verify phase that dies wholesale as leaving its findings unverified, not cleared, and rebuild a failing reproduction from the stated mechanism | Sub-verifications dying and the findings being filed as clear |
| Read a stalled reviewer's partial output before killing it; the point where it stopped marks where something looked wrong | The most valuable finding of a slice sitting in a lens that never filed a report |
| Re-measure yourself any cell the two lenses report differently; a lens's "both oracles agree" has been a measured split | A split filed as a two-oracle agreement |
| Measure a lens's proposed stricter or simpler rule against PRE before adopting it; refusing what PRE accepted is a ladder descent | A reviewer's simplification regressing hundreds of cells |
| Write the quantifier of every property a fix claims (one shape, one rule, every rule); a per-shape fix returns next round through another door, and a generalised floor that erases a loud which was masking another defect is loud→silent-wrong | The same root coming back each round under a different spelling |

### 3.6 Stopping, reverting and prerequisites

The round budget is three. A fourth is a scope signal, not a fourth patch.

| Rule | Prevents |
|---|---|
| Stop and count when each fix on one axis produces the next blocking finding: revert, ship the separable halves, and file what every attempt uncovered as the prerequisite | A fourth patch on an axis that is wrong |
| Revert and measure the condition after mis-scoping a guard twice, instead of attempting a third scope | The third attempt being another guess |
| Revert to pre-existing behaviour and register the measured shapes when the condition cannot be named | An unnamed condition being encoded as a guess |
| Read a root that returns through a different door each round as the stop signal: revert whole and write the prerequisite into the queue row | Each narrowing breaking a different case |
| Revert a producer axis that yields a new blocker every round and make the consumer decline on what it cannot vouch for; the producer's patch gets its own row with its measured cells | A fourth attempt on the producer axis |
| Fix the other code path first when a precondition lives there; a workaround predicate that is wrong twice is an ordering problem, not a predicate problem | Consecutive rounds of regressions from workarounds |
| Ship the separable half and revert the rest with the prerequisite written down when blockers on one axis exceed three and most are products of your own fixes | Two slices in a row, each fix locally correct, the axis wrong both times |
| Decide fix-or-revert from the root, never from the effort spent: ask whether the root is pre-existing and independent, whether the fix needs machinery the frozen IR cannot hold, and how wide the blast radius is | A separable half going out with the revert because nobody looked |
| Count the rounds and read where the blockers are: when they sit outside what you built, in what you routed to, you are discovering a prerequisite | A fourth fix on shared code with a different blast radius |
| Do not propagate a closure out of a slice until the slice is committed, and re-measure rather than restoring old text when re-opening one | A row marked resolved coming back with the revert, its old text overstating the residue |
| File the wall as one infrastructure line and point the feature rows at it when three requests stop at the same prerequisite | The next person walking into the same wall through a fourth door |
| Record the mechanism, not the verdict, when reverting, so the next attempt starts from a measured prerequisite line rather than from the queue line, and treat a defect a change merely exposes as belonging to the code it exposes | The next attempt repeating the reverted one, and a slice absorbing an unrelated root cause |

## 4. Census method

A census is an enumeration, taken from the source, of every site that can reach a question, with
each cell measured rather than argued. It is the unit of work here: a slice opens with a census and
closes with one. Four kinds recur, and they answer different questions — a producer census asks who
writes a value, a routing census asks where a value goes, an ordering census asks when it arrives,
and a consumer census asks who reads it and what each reader does with it. One never substitutes for
another.

### 4.1 Start from a census, not from an implementation

| Rule | Prevents |
|---|---|
| Start a queue item with a census from the code — grep every site that builds the construct, decide whether the defect reaches each, confirm with the oracle — never with an implementation | The queue recording a symptom and the class being larger than the row says |
| Ask whether a function already implements the rule and whether every place that should call it does; the detector is axis-independent | A rule implemented exactly and called from only some of its sites |
| Re-run the census before starting a slice: an estimate written at the end of the previous slice is a hypothesis | The previous slice having moved the gates the estimate was measured against |
| Re-measure every open queue row at HEAD against both oracles before ranking, and re-measure class (loud versus silent-wrong) first, because class decides ranking and ages fastest | Ranking rows on shapes that have since changed |
| Ask what a row's mechanism can reach, not what the reporter ran | A row that names a symptom being scoped to that symptom |
| Grep the open queue for the function you are about to change before implementing any review finding; where a row says built or reverted, run its designs first | A one-line routing fix reproducing a regression that was already measured and reverted |
| Grep the queue for the site and read every line that names it before trusting a row's "no prerequisite" field | The older line, which is usually the measured one, going unread |
| Grep the failure messages of green pins when choosing the next item | A green test's message containing the next slice verbatim and nobody reading it |
| Measure the whole-value operation first for an element-select silent-wrong: correct there means access routing, wrong means a storage gap | The slice being sized from the symptom |
| Enumerate the sub-classes a row's fix would serve and ask which of them the existing channel already answers | A row's stated cause pricing machinery most of its sub-classes do not need |
| Write both what was measured and what could not be measured into any sentence that closes a family | The gap between what was measured and what was closed leaving the document |
| Instrument the rejection point with the node kind and aggregate it, rather than ablating one gate | An ablation measuring only that gate's axis, so a kind with no arm looks the same either way |

### 4.2 Containers, spellings and passes

| Rule | Prevents |
|---|---|
| Give a post-patch or re-spell pass as many sites as the type has containers, and read a sibling spelling that is already correct as the signal that one container was missed | An omission looking like a missing capability |
| Enumerate all sites for a shared function or desugar: every scope, caller, parser variant, assign site, reserve path, statement dispatch and declaration-level validation | The most frequently repeated defect class in this repository |
| Count a type's containers and pin each one when a pass respells or patches expressions held by that type | A per-container omission repeating, with the loud spelling visible and the silent one not |
| Enumerate the containers of the type a post-hoc patch pass patches, by grepping the type in the frozen IR, not the call sites that build it | A container carrying an unpatched sentinel into the engine |
| Count the passes of one family and check the hit count, not the symptom | The same omission repeating once per pass, so fixing some of them makes the design run and print the wrong value |
| Record a route census inside the emitters, with the table an emitter needs to file a row from an identifier alone, never at the callers | A new caller bypassing the seam and a row reading zero beside real call sites |
| Census the emitters that share a diagnostic's context string through one resolver, and measure at least one of the others against the oracles | A change made for one emitter silently moving the others |
| Census the consumers that resolve names later than they are collected, and carry the collection scope with the item, when a block gets a scope of its own | Everything that worked only because the block's names were flattened breaking at once |
| Treat a gate that exists for one consumer as the gate for every consumer of the same shape, and count the copies | Binders that call none of the copies, and a further copy the docstring already claimed |
| Census a scope rule at every spelling of the scope it names: module, interface, package, compilation unit | A rule about a declaration written in one scope being applied to a spelling where it does not hold |
| Enumerate the AST forms new keys can appear in that the old keys could not, such as lvalues, iteration and port connections, when a table's key set widens | A read-only rewrite gaining a write side and an element write becoming a bit write |
| Bisect a diagnostic page per header or per file before pricing the items | A page that reads as several items being one root plus its uses |
| Probe a queue row's plain twin at three widths — at most 32, 33 to 64, and above 64 — before building for the row's shape; the answer names the lane | The position named in the row being incidental while the plain twin was already wrong |
| Census a parameter rule over four channels: module body, instance-elaborated, package, and instance override, filing the override channel as its own row | The override channel folding in the parent, before the target's width exists |
| List a parse-time constant table's gates and census each with a control twin: overridability, the declared type, every declaration of the name, and the readers you did not write | Each skipped gate hiding a defect, including correct designs turned loud |
| Run the real design behind the row and take the next page in the same slice when it is the same table | The next page being two lines away and deferred to another slice |
| Put the multi-line cell in the census for a position query | A single-line use being unable to tell two readings of position apart |
| Give every census consumer a scalar control twin beside the element spelling | A column reading as wins where the control twin shows a pre-existing silent the element spelling is about to inherit |
| Widen the other operand on every axis before believing a boundary; a census band is a property of its operands, not of the defect | A band that is an artefact of pairing every cell with the same sibling |
| Keep the eligibility set identical to the process set | Designs dropped between eligibility and processing with no diagnostic |

### 4.3 Producers, populations and writers

| Rule | Prevents |
|---|---|
| Justify removing a defensive check with an exhaustive producer census — constructors, struct literals, field writes, direct plane writes — not with a green suite | A green suite being a coverage statement rather than a proof of the invariant |
| Census a set's writers before relaxing a guard that never fires positively | The guard firing on its own producer, so relaxing it re-opens a real case |
| Enumerate the resource — every argument the engine writes back — not the sites you edited, and treat a guard that cites another guard as its model as a census of two | The cited model never having called the funnel either |
| Search for a data structure that already records a property before building the mechanism a reverted slice named as its prerequisite | Building what an existing map already answers |
| Open the code that populates a list your check reads | A check over a list that is always empty being dead code shaped like a guard |
| Audit the population path of any set a check consumes, and check that the population does not zip formals positionally | The candidate set being empty, so the check never runs, and named arguments being invisible |
| Count everything the site you are moving sets, not only the field that motivated the move; two fields set by one function are usually one fact | Moving half a fact and making the other half's consumers silently wrong |
| Search the text of an approximation you are replacing; the places that depend on it are the places that say so in a comment | One consumer being left on the approximation |
| Find the further collectors of a concept by grepping the constructor, not the name | The same declaration falling into different kinds in different collectors |
| Enumerate a shared map's readers before measuring anything when routing a value out of it, and ask of each whether it reads a value or uses membership as a proxy | A proxy going stale and the regression living in old code |
| Write the same expression in the neighbouring scope before building the mechanism a wall is attributed to; when the tool contradicts itself, the correct half is the implementation | Building what is already built one scope over |
| Name the mechanism, not the missing input, when collapsing several rows into one infrastructure item, and re-measure the others the day it lands | Rows that share a provenance but not a domain, so closing one closes only one |
| Measure the end-to-end outcome of the pair when a re-grounding says closing one item moves the refusal | Fixing either half alone producing a worse report than fixing neither |
| Check the input set before looking for missing machinery when a feature works in one place and not another | The classifier walking a narrower set than the feature reaches |
| Put the PRODUCER census of any new per-instance carrier (key, parameter) in the review brief; a second producer (alias, pass-through) re-seeds the default after every consumer is routed, and a routed predicate must be checked against the stored value it guards | Every consumer routed and the default still landing |

### 4.4 Routing, ordering and consumers

| Rule | Prevents |
|---|---|
| Count every place that asks whether a value belongs to a store before opening a new one: the read funnel, the write funnel, the specialised evaluator and the reader wrapper | Each fix making a different piece of the output correct, so stopping anywhere looks like success |
| Count how many code paths a reject row blocks before narrowing it; one row can cover two executors | Threading one executor and leaving the other silently wrong, green in the whole suite |
| Run an ordering census as well as a routing census for a construct that writes into another instance: routing answers whether the value reaches the right storage and is silent about when | An exhaustive routing census reporting clean over an ordering silent-wrong |
| Grep every read of a shared carrier type and give each site an explicit non-empty decline naming its reason, then measure the declines | Readers with no slot for a new field binding the wrong type in silence |
| Census the readers of both slots before moving a value from one slot of a record to another | One consumer reading the slot the other one wants |
| Count how many times one feature reads the store — value, offset, width and index are each a read — and route every read through the seam | One read left outside the seam making the write land elsewhere |
| Re-walk the call graph by store-access spelling after wiring a consumer, not by a list of names | A task's own arguments bypassing the formatter and reading the old store |
| Re-check a "the funnel discards the wrapper" argument per lane: a lane that stores a whole value answers differently from one that stores bits | One lane keeping the flag the funnel was supposed to drop |
| Ground a loud-to-supported candidate by running the same context set through both the candidate path and its sibling and comparing a capability-parity matrix | A silent-wrong common to both paths being invisible |
| Widen a read and sweep the write twin in the same iteration; the detector is a scalar or fixed twin that is loud while this path is quiet | Fixing one read leaving several same-class write silent-wrongs |
| Measure capability parity before unifying or routing storage classes; where neither representation dominates, extend additively | The weaker axis silently regressing |

### 4.5 The axes a census must vary

| Rule | Prevents |
|---|---|
| Include the `signed` spelling of every cell in a typedef census | An unsigned-only table certifying the sign axis by omission |
| Put one instance per census cell, or compare sorted line sets | A two-instance cell printing in display order, so a second instance's pre-existing value reads as new |
| Add the census axis a narrower type cannot represent when routing a value through it: a sign for an unsigned, a fraction for an integer, "never" for a count | Every literal on the axis being non-negative and the regression shipping |
| Sweep the container dimension as well as the operand's when the symptom is "reads the wrong element"; the immunity band is a function of container size | Probing one width, finding nothing, and reading that as no defect |
| Put a narrow constant beside a wide literal under unary minus, remainder and division in the census; those are the operators where a wrapped intermediate cannot be recovered | A large census being green over regressions in the non-commuting family |
| Vary every field of a record — base, width, direction — because the no-op combination certifies a lane that does not work | A normalisation measured only where it is the identity |
| Give every census cell a keyword-spelled control twin | New silent-wrongs turning out to be pre-existing on the plain spelling |
| Measure "the gate rejects that shape" per spelling | One refused spelling not refusing the family |
| Read one of several grouped operators diverging as the signal that the divergent one has a property the grouping missed | The common rule being blamed instead of the operator's own property |
| Read a characterisation that concentrates entirely on one value as a signal that the other half survives for a different reason | An accidentally correct half blinding the characterisation to its own axis |
| Classify every operator on an axis as sign-sensitive or bit-pattern before changing that axis's interpretation, write the table into a comment, and measure whether an uncovered operator was right only by cancellation | Fixing one operator exposing a latent defect in its neighbour |
| Census by routing and ask whether one of the sites implementing a rule is already correct; the correct site is both the proof and the specification | Assuming everything is wrong and missing the reference implementation already in the tree |
| Build one cell on each side of a domain boundary in any change that moves that boundary | A large sweep containing no cell that reaches the boundary |
| Write the factorial table and check that every output the mechanism can produce is in the readout before publishing a refutation; a refuting census varies the claim's axis and holds everything else fixed | A one-column readout of a multi-column mechanism refuting nothing |
| Vary every field of a reported shape, not only the one the report names | The field held constant being the one that matters |
| Count the loud→value cells separately; multiply position by the five binders (module, instance, package, generate, override), give every cell a keyword/scalar spelling twin as its control, one instance per cell, and open a §2 row with the plain twin of its shape | A census that cannot say which cells descended the ladder |

### 4.6 Queue rows and incoming reports are claims

| Rule | Prevents |
|---|---|
| Split a broadly written item's scope with a three-oracle census first; an axis where the oracles split is off limits | The slice taking on an unarbitrable axis |
| Include the cells where the silent default equals the true value when removing that default | The whole table reading as wrong-to-loud, hiding the correct-to-loud subset |
| Ask whether the oracle orders your axis by kind before keying a shared table on one fact | One ordering key being unable to reproduce several per-kind orders |
| Enumerate the resumption kinds, not the code sites, and give each its own two-oracle cell | Two kinds sharing a site, so a site census answers "all converted" twice |
| Measure the twins a row lists as "kept correct" before using them as the regression baseline | Listed twins turning out to be a split and separate silent-wrongs |
| Run the row's own cited line and ask which context it is in before building the machinery a row prices; a keyword's meaning is context-dependent | A reject gate keyed on a keyword over-rejecting everywhere the standard neutralises it |
| Diff a census cell's diagnostic text against its control's before classifying, when a whole position column is loud | Cells reading as still loud for a reason unrelated to the feature |
| Run the real design after a rule a queue line claims will open it, and write the ladder that follows | The claim being a hypothesis about a second error page nobody has seen |
| Measure a new loud gate on the designs it will refuse, not on the one that motivated it: enumerate the syntactic shapes that reach the arm and run PRE on each | Ordinary style and non-scope regions going loud |
| Measure the end-to-end outcome before promising that closing a gate unblocks a design | A refusal moving instead of closing |
| Add the arm to both evaluators when a text is folded by two | The first arm fixing many cells and leaving a whole family at the wrong width |
| Measure the whole axis against the oracles when a report names one cell | A reporter knowing the cells they hit and not the cells they did not, including their own suggested workarounds |
| Census the consumers of a diagnostic model field, not just whether the model has a slot | A field nobody fills and nobody renders being a dead contract, not an unimplemented feature |
| Re-run every item of an incoming report at HEAD | "Still true", "already fixed" and "true but not a defect" being indistinguishable |
| Survey third-party RTL before ranking priorities | A corpus you wrote yourself finding what you already suspect |
| Run the oracle first and vita second when building a workload, and forbid simplifying or rewriting the RTL so vita accepts it; a refusal is a result | The workload measuring only what vita can already do |
| Count how many rows of a manifest already contain a state combination you believe cannot occur | A believed-impossible combination being present and unhandled |
| Re-measure with the oracle any assumption written as a degenerate special case | A diagnostic pointing at a phenomenon that does not exist |
| Count a resolver's callers when you meet "this capability does not exist"; a resolver with one consumer has grown to fit that one question | Several binding sites using a literal-only twin while a general resolver sits unused |
| Grep every place that enumerates a subset before widening it | One layer accepting, another refusing, and the fallback message asserting something false |
| Check a demand claim with the same suspicion as a correctness claim | A revert's justification resting on usage nobody verified |
| Enumerate the spellings of a feature a report names and measure each | A passing test being evidence about its own spelling and nothing else |
| Treat a comment saying "only" the way you treat one saying "cannot": ask what the other cases are and run one | The sentence that is the entire defect reading as a scope note |
| Run the suite, the corpus and the examples before the review when adding a loud gate on a shape the engine used to accept, and ask what made a refused working design work | The full suite refuting the claim in one test |
| Measure a planned reject row before building it; over-rejection is a ladder descent | A row planned because two documents say it is needed, over code that never reads the input |
| Ask at which phase a reject row's reason is true; "this row is dead" expires | The same row being recorded three different ways |
| Ask what a refusal actually blocks before asking what to build | A refusal that is pure conservatism, where deleting one line buys coverage |
| Split a feature-named row with a census; the part that genuinely needs machinery is usually already refused under another name | A row bundling unrelated shares |
| Split a one-word reject row by what designs do, not by which tables exist; a table nobody reads refuses nothing | Unrelated populations sharing a word and a priority |
| Re-measure every sentence that cites a kind as its reason when you open that kind's row | A comment claiming a scan refuses something a neighbouring change already opened |
| Read a function that takes an alternative store as a parameter as using that store only on the paths that name the parameter | Opening a row leaving the other arms silently wrong |
| Re-measure a queue row's FIX SHAPE with the same suspicion as its symptom and oracle count; when the shape changes one shared key, ask first whether the oracle sets that key per kind — a root returning each round through a different door is one key under several rules | A one-key fix built where the standard has several rules |
| Ask whether a decline is a decline before "adding only where None": when the existing lane returns a WRONG value rather than declining, the new lane must be routed ahead of it, not behind it as a fallback | A fallback that never runs because the wrong answer is already there |
| Reject a cited cell that both candidate rules answer identically as evidence; measure a cell where they differ, and size the slice as the whole subclass minus what an existing channel already answers | A slice justified by a cell that cannot distinguish the rules |

### 4.7 Completeness for a change already under way

| Rule | Prevents |
|---|---|
| Define a name's shadow set as every place a module binds one: ports, import exports, enum labels, instance names, block-local declarations | A census over declarations alone missing most of the binders |
| Find the nearest spelling of the same question that already works, and ask what it calls, before accepting a stated wall | A prerequisite being carried through several slices while the machinery already exists |
| Walk forward after changing a width to every site that re-derives the value from a width, not only the sites that read the width | A read-back that assigns into the variable you already set being invisible to a census of readers |
| List every call the original's caller makes before copying a call; the twin reads from its own state and most of the contract is invisible at the call site | Copying the maps without the containment gate and making a nested case silent-wrong |
| Re-measure a documented split's discriminator; it ages | A row standing for several slices on a discriminator that does not hold today |
| Include the pre-existing branches of the same gate in a new fence's blast radius | The same leaf staying silent-wrong through the older branch |
| Grep a predicate's documentation for the condition it named before changing the component that condition is about | A guard becoming a pure false-loud that looks identical to one still needed |
| Record which census cells are single-instance and which are multi-instance when an oracle contradicts itself on the second instance | A self-contradicting oracle being cited for a multi-instance cell |
| Grep a property's string after fixing one site and count the rest, updating every comment that cites the equivalence argument in the same edit | Identical sites staying unfixed and a comment surviving its own premise |
| Control a path-dependent feature by the declaration that decides the path, verify the callee takes that path, and fix every path in one iteration | A matrix that never enters the mechanism it claims to measure, and divergence between paths, which is worse than uniform wrong |
| Grep for existing partial support before building infrastructure for a construct recorded as rejected, and treat an additive-looking parser gap as a possible storage or evaluation-model gap | New infrastructure built for a one-sub-form gap |
| Grep for every site that needs a fact you have just learned, and fix the second instance of the class in the same slice | The same silent-wrong being reproduced one layer in |
| Grep for downstream comments that assume "there is no such thing before this point" after inserting a stage into a pipeline | A later stage's comment becoming false and a whole change set being lost |
| Measure the claim "the upper layer refuses this first" by running that shape through that layer | Mutation survival being indistinguishable from "no design of that shape in my set" |
| Build the drain twin in the same slice when a new site starts reading an alternative store, answering the drain question separately for each termination path | The same failure being produced in consecutive slices |
| Write in one sentence which shape makes a new structure meaningful and count that shape | Zero meaning the design has no basis, not that the test is missing |
| Count who does not call an existing funnel before building one | Infrastructure built for a one-call-site omission |
| Enumerate and measure the indirect paths — calls, hierarchical names, methods — that bypass a new loud gate before claiming it is the only net | The gate not being the only net, with the claim untested |
| Read the sibling funnel in the same file every time; branch parity finds this by reading, not by probing | The twin funnel having the same shape and the same defect |
| Read every other arm of a match in the same sitting when a fix lands in one, and ask whether the fixed arm's reason applies there | Two arms two lines apart giving different answers |
| Grep an existing sibling across the workspace after adding a sidecar, mirror every copy site, and assert the map is non-empty at the consumer before reading the output | A sidecar reaching the engine through separate field-by-field copies, one of which is silently empty |
| Count the render sites of a per-statement fact and give an executor without a seam the ability to write it, with a census cell per site | One seam being mistaken for the funnel |
| Route every site that folds the same field through one named funnel with the old call as its fallback, then measure the funnel's new lane for the shapes it must not change | The same text folded by the same evaluator at many sites being fixed at one |
| Census three lifetimes for a name-keyed parser rewrite: declaration, shadow, and export | The declaration lifetime being right on the first build while the other two are silent-wrong or loud |
| Census the regions that have not yet been asked the same question after fixing one | The same defect shape existing once per region, the largest one last |
| Count how often a row is the sole blocker; a row with no sole-blocker cases cannot be closed alone | Closing one of a pair gaining nothing |
| Record marginal and standalone gain separately | A plan of cumulative numbers hiding rows with no standalone gain |
| Grep every caller of a primitive before changing shared semantics such as conversion, resize or width rules; that enumeration is the scope decision | Review rounds spent walking from the leaf back to the primitive |
| Count the small consumer-by-value matrix before adding or removing a feature | Fixing one cell being mistaken for knowing the axis |
| Add an arm to every walker when adding an expression kind | One walker's catch-all swallowing the new kind |
| Ask a source-scan pattern by prefix; a pattern naming one family member is a whitelist, not a scan | A sibling function a few lines away going uncounted |
| Make a classifier that walks statement lvalues also audit the side tables that hold write destinations | Walking a node not being the same as seeing its effect |
| Re-audit every comment that cites "another gate rejects it anyway" when you unify predicates | The justification holding only while there is exactly one gate |
| Check the other kinds of callable object in the same sitting when applying a rule to one | One half being closed with a comment and the other left open |
| Drive the whole idiom, file access included, to the end after opening a gate | A minimal reproduction stopping short of the silent-wrong beneath, which the user meets first |
| Fix both halves of a shallow and deep walker pair in the same slice | Twins having the same defect twice |
| Reproduce the stated basis before calling a place marked deliberately unverified a misdiagnosis | A reported false positive being a measured, correct constraint |

## 5. Gates, predicates and classifiers

Under-detection in a shared walker is the repeating source of silent-wrongs here, so a gate is
written to fail closed and is measured on what it refuses as well as on what it admits.

### 5.1 One rule, one home

| Rule | Prevents |
|---|---|
| Put the canonical home of a question beside its twin in `sim-ir`, not at the consumer | A second consumer inventing a second spelling |
| Put a guard in one documented funnel that every site shares, and name the predicate after the prohibition reason, not after a type enumeration | Per-site guards leaving sibling axes open and a split predicate giving each axis different coverage |
| Add semantics to shared machinery as an opt-in parameter, never a default, and document the positive precondition — when it is safe to turn on | One consumer's need imposing risk on every other, and the next reader falling into the same trap |
| Move the value-free rules of a renderer into a crate both the constant-domain twin and the runtime can reach, and make both call them | A twin re-deriving a rule set one finding at a time and converging only asymptotically |
| Put a context or width rule on the consumer, never inside a shared evaluator: grep who calls the function and whether they agree on context, and where two disagree the rule lives at the call site | One consumer's context being imposed on all of them |
| Expose a flat entry point by having the canonical implementation finish its normalisation and delegate to it | A second spelling of the judgement drifting |
| Choose refusal over routing when routing would spell an existing rule a second time, and make correct support a separate slice that gives the funnel an escape hatch | A second spelling of a split rule |
| Delete the old entry point when the canonical implementation moves | The old method being one edit away from pointing at a different entry, so read and write lanes diverge |
| Reduce a new shared input to one function rather than one datum, so the contract is a property of one expression | Two reduction loops having to agree |
| Write a node's children, their order and their evaluation conditions in one place and have every walker consume it | Independent recursions inevitably diverging |
| Keep one shared list of positions that must not be hoisted and have every hoister consume it | The second hoister not reading the first hoister's list |
| Give two walkers that must see the same child set one child-list function, and mark unreachable reads unrepairable so both answers agree | A recorded read the transformation cannot reach being silently wrong |
| Write the naming rule two stages share once and have both use it; a limitation whose reason is another stage's implementation detail is a defect in that stage | The path where the name exists and the path that looks for it diverging |
| Treat a guard as a funnel, not a site: enumerate every place that builds the operand and pass them all through one function | A guard at one leaf missing the other operand-building sites |
| Mint an identifier that indexes parallel vectors through one funnel that fills every table, with an assertion per table and an explicit empty slot for the case that owns no row; do not guard at the reader | One producer pushing some of the tables, shifting every later identifier, so a reader returns another entry's data |
| Treat every writer of the primary map as a writer of the side map when a key space becomes rebindable, and route them all through one funnel so a grep for the raw writer returns only the funnel | "A writer that forgot" being merely absent rather than unrepresentable |
| Fill a new table with the same producer as its twin, so one provenance rule covers both scopes | A second, independently written producer being a second rule wearing the first one's name |
| Split a predicate per resolver when a value has two representations; one predicate cannot serve two lookup orders | Subsystems disagreeing about which map wins |
| Frame two implementations with different strengths as "where do they split", and extract the split predicate once | Two copies of an admission predicate drifting invisibly, whose only symptom is a slow path |

### 5.2 A predicate that cannot under-detect

| Rule | Prevents |
|---|---|
| Answer a property question by walking the expression, never by enumerating spellings; use an exhaustive allow-list and fail closed on unknown variants | A spelling-counting classifier contradicting itself inside one design |
| Extend a shared classifier or gate only with a full consumer census, and make accept-gate walkers conservative or exhaustive | Under-detection in a shared walker, the repeating source of silent-wrongs |
| Spell a gate predicate as an exhaustive match, never as a boolean shorthand; the compiler must catch a new variant | An implicit catch-all letting a new identifier default to the quiet side |
| Close a syntactic walker's blind spot by opting into an already exhaustive walker with one axis parameterised, not by writing a new walker | A new walker repeating the old one's omissions |
| Build a scope or safety guard as an allow-list of provably safe forms plus a reject, not as an enumeration of dangers; a recursive allow-list recurses over every value sub-expression | Enumerating dangers repeatedly omitting a category, and one unvisited sub-expression being an escape |
| Choose a walker's polarity from the gate: an accept gate takes a conservative walker, a reject gate takes a positive one | A conservative walker in a reject gate refusing working designs |
| Read a catch-all answering false in an expression walker as "this node may reference anything", not as "unknown" | A conservative accept gate giving an answer independent of the name asked about |
| Never skip the classifying recursion in any arm: compile first and discard the result if you must, and count the recursive calls per arm | A shortcut arm skipping admission, so a diagnostic disappears and a draw vanishes |
| Enumerate the arms of a hazard walk that answer from a rule rather than from their children; those must still descend for the guard | Wrapping the hazard in braces walking past every syntactic guard |
| Gate every consumer of one walk on one predicate, not each on its own | Consumers of the same walk diverging |
| Print both accept sets and name the difference before mirroring a predicate across a phase boundary, and ask whether the skip is needed at all | Two accept sets that are neither equal nor nested leaving the shapes between them fail-open |
| Prefer the funnel that sees the value to the one that sees the syntax | A gate on the AST inheriting every hole in the AST-level predicate |
| Compare the two phases' resolvers, not their intents, when leaning on an existing gate for a precondition | A check that folds less than you do being unable to cover you |
| Count the enumeration behind a shared classifier's "sees all of them" comment | An expression position not being seen, leaving a real divergence |
| List the children instead of smearing "unknown" over a node, and separate nodes that are evaluated from nodes that are not | A node with no effect in it standing the whole statement down and making a working design loud |
| Widen an analysis lattice until it can distinguish the answers you need; a narrower lattice is itself a misdiagnosis | One boolean being unable to separate two outcomes, so a large share of reported items are that collapse |
| Special-case only where you have a distinguishable reason, and check whether a user can write the same spelling | A blanket special case dropping a live path from the join |
| Use all segments, not the head segment, when the question becomes "can this callee touch that name", and add it as an opt-in parameter rather than copying the walker | A flattened name being reachable by a path the head-segment rule cannot see |
| Check that an early-return predicate's walk has the same arm set as the lowering it gates | An under-detecting gate making the fix miss one spelling |
| Mirror the questions an existing arm's conditions answer, not the conditions themselves | A restored predicate answering for the wrong family |
| Close a gate's blind spots fail-closed with an opaque flag raised when a read has no nameable root, not by enumerating shapes, and confirm the cost is structurally narrow | Hierarchical and package-scoped names escaping an identifier-keyed gate |
| Check that two enumeration arms share a contract before merging them; the same type is not the same meaning | A merge making an unevaluated position contribute a read and false-rejecting a harmless statement |
| Write a width formula in its standard form with its domain, and extend the walker to its own output when the emitted shape feeds back into it | The one parameter value the census did not run |
| Do not assume the expression arena is a tree: read what the existing walkers filter before writing a new one, because that filter documents the arena's properties | A buffer sized by index meeting a back edge and the whole seal disappearing |
| Never fold a depth or count limit into a plain true or false; make limit exhaustion a distinct state, and remove the limit where an iterative rewrite can | The folded value being another question's answer, deleting a diagnostic |
| Confirm that a canonical predicate answers your question before calling it, and state the delta explicitly when it does not | "Is it pure" not being "may it be evaluated twice", and closing everything losing the genuinely pure cases |
| Make every query used for a decision three-state; folding "not yet known" into "no" is a silent-wrong | A placeholder answering with a fabricated fact that the caller reads as a fact |
| Ask whether a caller uses the answer for a decision before adding a fallback, not whether it is visible in the engine | A fallback that looks harmless in one lane demoting correct support to loud-wrong in another |

### 5.3 The predicate must match what it gates

| Rule | Prevents |
|---|---|
| Make a classifier use the same name resolver as the lowering of the expression it classifies | Classifier and lowering diverging silently under shadowing |
| Extract a lowering's decision into a side-effect-free function and make the lowering match on it too | A docstring saying "mirrors X" being a drift waiting to be measured |
| Make the shared decision function say which of its answers are facts | A mirror being exact only for nodes it built itself |
| Make a gate that decides whether a body is safe to process walk the same statement arms as the processor it gates | The gate certifying a set the processor does not act on |
| Put a dispatch hook at the very top, detection first, and enumerate deny hooks over every write path | A hook below another check never seeing the case |
| Make a gate predicate match the destination consumer set exactly | Over- or under-approximation at the gate |
| Build extensions as strictly additive, fail-closed subsets that leave the remainder on the old path | A non-additive change moving cells that were already right |
| Extend by adding a discriminator branch with the existing path kept verbatim, and prove the new eligibility set disjoint | A rewritten shared path moving existing cells |
| Pay for a first placement with an explicit gate keyed on a property the old lane's correct cells do not have, and sweep for movement | Placing a new lane first and silently moving cells the old lane got right |
| Check the accuracy parity of a helper's branches before filling it with new traffic; a guard present in one branch is usually unmoved, not unnecessary | Routing new traffic into the narrow branch turning wrong into a different wrong |
| Re-apply the caller's scope rule at a resolver-first hook, keyed on the node's root, and put the decline before both resolvers | A new arm bypassing the caller's own arm and folding the wrong object |
| Name the consumer a constant folder was written for and the rule that consumer needs — bound versus value, saturate versus wrap, self-determined versus context width — before reusing it | A folder silently declining a whole family, never wrong-valued, so censuses miss it |
| Make a twin of a predicate in another phase the same walk, not the same intent, and ask over which set of names it quantifies and when that set is complete | A parse-time twin being blind to declarations an elaborate-time walk sees |
| Give a leaf with no width of its own a tri-state width — unknown, context-sized, or a known width — and grep every consumer for the predicate it uses to tell the first two apart | One predicate reading a placeholder as known, so a fold declines and a loud becomes a value |
| Let the evaluator, not the table, supply the width of a region made only of context-sized leaves | The table having no answer for such a region |
| Pass the position as a flag and make a resolver in a count or size position refuse rather than read an environment | A general resolver letting a local supply a count, or answering past a shadow |
| Attach the four-state qualifier to any argument that an operator is safe to narrow | Low-bit closure holding only in two-state, so narrowing deletes an unknown |
| Distinguish three states — a wrong answer, unknown, and a right answer — because replacing a wrong answer with unknown routes to a conservative path and can drop machinery | An "unknown" answer making a cast skip context descent |
| Attach a width-invariance qualifier to low-bit closure and to "narrowing computes the same thing": some leaves' value depends on their width | An argument valid for ordinary leaves being applied to a fill |
| Extract the sign half of a declaration rule as a pure function rather than folding the range to get it | A classifier emitting diagnostics and changing the program |
| Follow the lowering's whole decision procedure, including pre-steps such as inline substitution; no fixed name-resolution order is a rule | Two orders each holding the other's counter-example |
| Put a wrapper-piercing predicate inside the recursion | A wrapper below an operator being invisible |
| Keep the original verdict statement verbatim in the body when adding a pre-filter, so a wrong filter falls back safely and instrumentation can show it never fires | The pre-filter becoming a second spelling of the rule |
| Answer in code who establishes the property a new predicate asserts; when the answer is nobody, either establish it or narrow the predicate to a context where it holds | A predicate asserting a representation property being handed a value that lacks it |
| Fix the contract of a shared kernel when an operand does not fit it, rather than rewriting the operand into an equivalent pair | A trick that is perfect on the value axis breaking a width cap into a silent unknown |
| Separate the questions a single boolean was answering — how each operand is read, and what the result's sign is | Special-casing at the operand leaking cost into width, lanes and performance |
| Count the early returns of the function that produces a width before basing a byte-identity argument on a width comparison | A short circuit on a special type running the operation at the wrong width |
| Ask two questions of a cap you intend to delete — whether the constant is the boundary of representable values or of supported syntax, and what type consumes a value past it — and reject on fitness, not on width | Relaxing a domain guard as if it were a capability limit and leaking an out-of-domain value |
| Read two spellings of one hazard diverging — direct refused, indirect accepted — as proof that the walker cannot see one layer down | A partial guard being accepted as complete |
| Treat a parser-side fold as scope-free: widening the set of names it claims means probing every binder and tying the stand-down to that syntactic scope | A parser fold answering for a name a later declaration shadows |
| Fix an ordering defect by moving a binder before its first consumer — a span comparison, or a split pass — never by moving it earlier | Moving a binder ahead of everything re-ordering every other consumer |
| Treat an earlier failed attempt recorded in a comment as evidence about that attempt, not about the question, and name the term it was missing | A recorded failure being read as proof of impossibility |
| Read every property a gate decides from the same environment it seeded | Two resolvers for one name being a divergence waiting for a scope |
| State a widening's property — adding a rule can only add candidates — and enforce it, rather than patching the shapes in front of you | Each fix being written for one shape and the property breaking again through another door |
| Make the baseline of a widening the whole rule set, not one privileged rule | A flag meaning "not the original rule" protecting only the original rule |

### 5.4 Arms, early returns and escapes

| Rule | Prevents |
|---|---|
| Ask what a gate actually prevents, not what its comment says it prevents | A refusal that fires for a different reason than the hazard leaving the hazard open |
| State which properties of the declaration every arm must preserve, and check the siblings, when adding an arm to a selection chain | One declaration getting two answers in one design |
| Treat a partial accept set as a decision about siblings: enumerate what the excluded cases share with the included ones | Excluding a sub-case to dodge a split reproducing the split inside the tool |
| Read a guard's justification as a precondition on another component and check that component's current behaviour before removing the guard | Deleting the guard alone turning loud cells into silent-wrong |
| Scope a surviving call site of a multi-site guard explicitly rather than letting an earlier arm shadow it, and record when a site is right only by accident | A guard with several call sites being retired wholesale while one site was doing a different, live job |
| Fence the operand a clamp will act on, not the destination; the admission predicate must ask about every node the clamp can reach | A wide leaf under a narrow target passing the gate and losing a sign bit |
| Check the arms of the child walk you recurse through: a guard must descend where an answer need not | Wrapping the hazard walking past the guard |
| Treat the positional binding as the hazard when a desugar's parameter count becomes variable: make the count uniform per construct and measure a following value parameter | A following parameter silently eating a carrier slot |
| Re-stamp a copy alias's sign at the one interpreter read and make the compiled paths decline on a mismatch | An alias that substitutes the source handing the source's declared sign to every consumer |
| Fold a package function's body in the package's scope, and refuse the module-scope fallback for a bare name inside it | A same-named module constant answering for a different object than the text says |
| Count which code paths do not run today because a conservative predicate answers false, before replacing it with the canonical rule | Cells that were accidentally right through the old path going correct-to-wrong |
| Make a precondition predicate ask about the argument the caller actually passes, not the declared one | The predicate answering "safe" and the executor failing, blaming the check |
| Do not split the context one consumer reads; "it is a cold field" is not a justification | Values coming from a new store while time and randomness come from the old one |
| Give each kind of unreachable case its own assertion and name the layer that refuses; the filter, the gate and the assertion must ask the same question | One refusal claiming a gate row that does not exist, and the misunderstanding leaking into a test's admission filter |
| Check the early returns of a value-conversion primitive: "already the right width" is not "nothing to do", and a difference between exit paths must be written down as intentional | One path keeping a flag the others clear, so every same-width assignment is silently wrong |
| Keep a compensating clear in one place and let consumers rely on it | Two spellings in consumers hiding each other, so neither dies under mutation |
| Audit an existing helper for latent defects when a new path starts calling it: unchecked arithmetic, shift masks at the width boundary, copied functions inheriting the original's defect | The new traffic inheriting an old defect |
| Judge name resolution and classification from an AST-gathered pure-function set rather than from mutable elaboration state | A diagnostic that exists only in one phase silently deleting a whole body at a success exit |
| Put a stand-down in the arm whose hazard it answers, keyed on the statement; a gate that can answer without looking at the statement cannot be used for a statement-level decision | A module-global early return turning off unrelated arms of the same match |
| Prove a value dead with "definitely written on every path", not "not read before the first write", and read the exact meaning of the reused predicate's success case | A conditional write satisfying the wrong contract and copy-out returning a stale value |
| Do not unwrap a block to iterate its statements; that drops its declarations. Write the same meaning as a declaration initializer to check both spellings, and ask about redeclaration in a subroutine body | The declaration-initializer spelling passing while the statement spelling is refused |
| Write an evaluator in the target domain rather than restricting inputs to tame someone else's fold; bounding the leaves does not bound the result | Permitted leaves building a result outside the domain, so spellings go loud-to-silent |
| Positively identify a subject rather than trusting a name whitelist | A method-name whitelist admitting user class methods, child instance functions and module functions with the same names |
| Transfer-audit every arm of the old predicate when replacing one: the replacement must prove it covers all of the old obligations, not that it is more accurate | One dropped arm being a regression and another a silent-wrong |
| Restore a dropped arm by axis, not verbatim | A verbatim restore reinstating a pre-existing silent-wrong |
| Check a constant fold used to predict runtime behaviour on three axes — domain, resolver and scope — and prefer an identifier-free allow-list where order independence is required | Any one axis differing diverging silently from the engine |
| Prove trip count and syntactic escape separately before admitting a loop body's writes, and handle every loop form at once | An escape being erased by the walk's join, and elaboration depending on which loop form the user wrote |
| Expand the condition of a branch you want to delete into a truth table and answer each row separately | Answering only the axis the branch is named after and leaving half the rows unexamined |
| Separate decision from execution in a fast path — decide everything, then execute — so that "a decline has no side effects" becomes a property of the code | A partially executed fast path emitting a diagnostic and then declining |
| Collect a run observation before its consumer exists, not at the end of the run from a structure whose fields may have been moved out | A move being silent where a partial move would be a compile error, and a grep audit missing it |
| Obtain a classifier's observation export by restructuring the classifier itself to collect reasons, never by writing a new predicate | A second predicate drifting from the classifier |
| Confirm every offset and stride is handled when going from one dimension to several, and keep direction in one place | A double flip of direction |
| Decide a deferred mirror's offset and direction-dependent kind at resolution time | Baking it at lowering time making the opposite case silent |
| Mirror the read's flatten prefix for a nested or packed select write, failing closed on the shapes it cannot express | The write landing on a different bit |
| Defer what is unknown at defer time, pre-resolve caller-scope dependencies into the sidecar, and lower each argument into every representation resolution may need | Information unavailable at one phase being guessed |
| Treat a gap between a predicate and its own comment as the defect, and re-read every guarantee near a guard you change | A kind-only predicate swallowing a shape its comment excludes |
| Mark items added to a classification set so they neither gain candidacy nor remove anyone else's | Merely gathering a span under a new rule making a name look shadowed |
| Pass three questions before moving an evaluation — how many times, when, and what it reads — and use an inertness predicate for everything the move passes over | "It is pure, so moving it is free" answering only purity |
| Write the necessary condition when a constraint's stated justification is only sufficient | A whole idiom staying closed behind a sufficient condition |
| Define a shadow set as what a scope actually declared, not as what appears under its key | A flattened block-local, whose key merely looks inner, being picked up by every other reader of that scope |
| Verify that a derived decision is actually consumed; when a comment says one key wins and the code does not, a fall-through is usually re-running its own walk | The innermost key being derived and then thrown away |
| Replace an assumption with a lookup: a constraint that looks like missing machinery is usually a caller assuming a special case the general path already normalises. Keep the special case an identity so the IR stays byte-identical | New machinery duplicating an existing normalisation |
| Use capture, mutate, install where the source can alias the destination, and document both the aliased and the non-aliased case | In-place ordering silently losing a value in recursion or copy-out |
| Fresh-probe the simplest form before rewriting on the strength of "the executor cannot do this": check storage interior mutability and the classification that routed it there | Building infrastructure for a feature that mostly works already |
| Build a guard on the value, not on syntax | A literal-shape guard being pierced by the first change that reaches the same value another way |
| Prefer a value-based judgement, over the lowered IR and over every sub-expression, to a shape-based one | A walker missing new shapes, where a value cannot hide |
| Decide whether a cap you are deleting is a capability limit or a domain guard, and measure one cell on each side of the boundary | Relaxing a domain guard leaking an out-of-domain value at a success exit |
| Ask about sign at the self-determined width | A width-unlimited fold being unable to separate signed from unsigned with the same bit pattern, and false-rejecting correct designs |
| Define opt-in as "where the paired record is actually reached", not "where it can be enabled", and count the early returns in between | The flag turning on the width while the record never runs |
| Do not fold an overflow modulo without a context width | The fold answering at a width the language does not use |
| Check that a static claim is true of the value when you begin consuming it, and find the place where it was cancelling out | Fixing one side breaking the cancellation |
| List what the sibling arms do besides the arithmetic before trusting an early return on a no-op arm | A no-op for one rule skipping every rule |
| Fix a self-firing guard by changing the route, not the guard | Relaxing the guard re-opening the case the suite pins |
| Ask what the consumer channel can carry after widening what a producer may carry, and enforce the difference where the two meet | The new spelling's door being closed while the open door declares the wrong width |
| Say in the comment which of the two "no value" answers a region or enable test reads when a fold feeds both a value and a test, and prove the arm cannot turn one into the other | The enable test reading the wrong one |
| Ask whether the lie a guard's comment names is the type's fault, and check what the consumers already do | A decline that was right for a clamp being kept for a truthful record and blocking correct cells |
| Decide at the producer whether a recorded string is relative or absolute, spell absoluteness in the string itself, make every renderer honour the marker, and census the renderers | A value recorded relative to a runtime prefix encoding the caller and being unrepairable downstream |
| Record a producer's keys at the one site that mints them and key on membership; never strip by "everything that is not this" | A complement silently including every shape you did not enumerate |
| Bind a declaration's shape sets once the whole declarator is parsed, at every binder, and census the index kinds | A shape being bound at the type token, before the dimensions after the name are parsed |
| Write a width rule as two functions — a shape pass and an evaluation pass — and let the second take the width the first computed | A bottom-up fold computing each node at its own width and looking right on every leaf whose operands already share the final width |
| Gate a named source replacing a literal on constness at the consumer that has the names, not on resolvability | "Does it resolve now" being a property of pass order, so refusing turns correct designs loud and accepting a variable reads before initialisation |
| Write the pairing predicate of a twin without an AST field so a user-written collision cannot satisfy it, and measure the collision on PRE | A user redeclaration satisfying the twin predicate |
| Run the target evaluator on the degenerate inputs your rewrite can produce before choosing the target shape, and pick the one whose failure is loud | A rewrite routing a new shape into an old evaluator inheriting that evaluator's leniencies |
| Cap arithmetic that builds a range from a width at the width the value can actually occupy | A shift by a user-parameterised width wrapping in release and failing in debug |
| Cap a recorded quantity at the source | An amplifying path being unreachable only because a different component happens to refuse first |
| Check the direction of the risk: release correct with debug or CI failing is the worst split to debug, and the fix belongs at the producer | A consumer-side absorption hiding a producer defect |
| Do not let an invariant rest on a side condition, and fix an unreachable arm that leans on one in the same round | The side condition changing and the invariant becoming false |
| Let the standard, not the code, decide which positions are self-determined | A context rule being applied where the standard declares self-determination |
| Grep a value's consumers before writing "this only decides one thing", and split the variable in two when a sentinel cannot be tolerated everywhere | A sentinel doubling as a sizing context |
| Make a tail that asserts an invariant unconditional | A conditional stamp meaning the property is absent exactly when the condition is false |
| Treat a memo without invalidation as a claim that the value cannot change: grep every write site, and narrow the cache to a prefix where invalidation is impossible | In-place patching changing a cached answer, so unrelated later lines change an earlier result |

### 5.5 Scope, ownership and order

| Rule | Prevents |
|---|---|
| Keep "which scope" and "whose it is" as separate answers, and change every reader when you introduce ownership | Scopes without a minted prefix sharing a key, so a flush claims someone else's item |
| Give ownership order and initialisation order separate data structures | One axis being unable to satisfy both |
| Make an ordering requirement data — a rank path — when it cannot be expressed as pass order | No rearrangement of passes being able to satisfy it |
| Separate "runs first" from "creates no event"; when measurement says the order is right and the behaviour is still wrong, you need a phase | An initialisation write handing an edge-sensitive process an edge |
| Disqualify only the offending element, never the whole name; things that cannot exist simultaneously have no standing to disqualify each other | A dead branch of a conditional generate breaking a live pair elsewhere |
| Split a value that answers three questions into its components | One key breaking in several places at once |
| Record which role an added behaviour belongs to when a function serves two, and split by parameter | A syntactic region behaving like a real scope |
| Collect in one pass what must interleave in declaration order | Two loops producing two orders that never interleave |
| Choose an ownership discriminator expressive enough to separate the two nearest candidates, and check that they give different answers | A boolean being unable to separate two nested scopes |
| Read the qualifiers in a refusal comment and count the cases where the condition is false | A true sentence being silent about a third case, which is harder to see than one that has expired |
| Answer "what catches this if it is wrong?" before adding an approximation; when the answer is nobody, it is a judgement and must ask the real question | A misrouted expression reaching no evaluator at all |
| Count side-effect sites, not operations, when designing a re-run fallback, and put the bail before the effect | The canonical path emitting a diagnostic twice |
| Count the values whose only consumer was the branch you are deleting | A verdict with no consumer being dead, and dead verdicts being silent |
| Recognise that a call path's head may be the function name or the receiver, and that the segment count decides which | An argument-only walker missing every method call on a variable |
| Apply a monotone invariant at every recursion point, not only at the top level | A construct working outside a loop and not inside one |
| Do not treat a statement as unknown because it carries a timing prefix; a timing prefix only adds expressions | One prefixed statement ending the whole walk |
| Make a reader total before opening a row, and keep it total with a structural pin | A trait's default implementation being a silent capability opt-out that returns plausible values |
| Check the shape of each routing bitmap: a handle's slot can be half-dead, so the question is membership and a present word | A bare handle read being routed to an empty heap |
| Attach a precondition to the executor, not the feature | The delegated path's precondition applied to the driven path refusing every target design |
| Check whether an existing correction already covers a legitimate difference in two computations' input sizes before adding a conservative signal | Treating a missing entry as a signal and making the two sets diverge |
| Check what a desugar merged before adding a rule that judges after it, and restore the flag when the merged forms have different oracle answers | A shorthand being silently accepted under a rule written for a different form |
| Make an implicit declaration a phase, not a use-site action, and keep exactly one collector | One design giving two verdicts depending on pass order |
| Make a regex argument about the maximum range the match can consume | "This pattern cannot match that" being true only when the counter-example is isolated |
| Let the lexer look at the previous significant token when one spelling has two meanings in the grammar | Different spacings of one construct being treated differently |
| Ask whether a guard is needed in the opposite direction after building it in one | The closing side still consuming the terminator |
| Read the engine code when a gate's premise is "the engine cannot do this", and make the discriminator use the same set as the storage it drives | The fallback path already handling the shape while the gate under-approximates |
| Make a gate that asks whether an executor can run a statement look at the whole statement; the effect can be in the right-hand side | An assignment being classified by its destination and silently doing nothing |
| Use different predicates for "is this an effect" and "can this executor never do it" | Re-using one set for both re-routing a whole family and turning working designs loud |
| Let the code that reads a set decide where it is populated, and compare fill time against read time exhaustively | The set being filled after the call sites that read it are lowered |
| Measure a third time instead of guessing a third time when a comment says the real condition is not yet named, and gate on the destination rather than the whole body | Reverts that lose measured-correct shapes |
| Place a new arm that lowers an lvalue below every check documented as detected first | A string element write becoming a silent packed bit write |
| Make the scope of a hazard analysis equal to the scope of the transformation, and analyse the expressions a statement evaluates as one sequence | Cross-boundary ordering hazards being invisible and an existing guard being disabled |
| Put repairable and unrepairable hazards on different channels | Unrepairable reads being quietly believed fixed |
| Merge two similar gates rather than leaving both | Two similar gates both being unaudited |
| Judge aliasing by target, not by spelling | An unrelated child scope disqualifying its parent |
| Ask a two-discriminator state with one dedicated predicate | Checking one flag leaving the other half ungated, which fails in debug and writes to the wrong destination in release |
| Do not describe where a write happens as a statement shape; a call that returns a value can appear anywhere an expression can | Most reported items in a family being that one defect |
| Make the set of admitted nodes equal to the set where the lowering can emit copy-out: wider than the lowering is loud, narrower is a false-loud | Working designs being refused |
| Use the branch's knowledge of the condition's value, and measure those premises with an oracle rather than asserting them | Collapsing a condition to one bit making a standard idiom loud |
| Add a third lattice value for "read-safe, no write promised" | A conditional write falling into "references, therefore reads" |
| Compare the binding rather than forbidding a name; when two scopes resolve to the same thing, the two lowerings are the same lowering | Ordinary cross-scope references being killed |
| Open the time gate in the same slice as the reference gate | A callee proven inert for a name still yielding the scheduler |
| Justify an early return on "already safe" only where the invariant is monotone | Ownership changing exactly at the point the early return skips |
| Move a symmetric decision into a pure pre-computation | An order-dependent gate never checking the first declaration |
| Use the desugar's own representation to state the rule where it is simpler and more general | A member-based rule being unable to express hand-written part-selects |
| Remember that structure fan-out renames a variable but not the call argument | A walk that tracks member names seeing the argument as touching nothing |
| Re-check any rule written as "direct child" the moment nesting becomes possible | Deeper keys being claimed by nobody |
| Raise a primitive's guards to the top of the function when you add a stage outside the primitive | The new stage not inheriting the primitive's guards |
| Nest a dependent field inside the variant whose validity it depends on | An invalid combination having somewhere to be written, so a grade's message becomes dead code |
| Ask about sign on a self-determined walk with the same admission as the bound walk | A width-unlimited fold being unable to tell two spellings apart that differ only in width and sign |
| Read a value from both representations when one alone leaks half a family, and make both width- and sign-correct | A saturating fold turning a negative into a count and reporting a false message |
| Write an opt-in predicate as reachability to the recording site, not as the condition for enabling | Early exits between the enable and the record |
| Do not use an enumeration's catch-all arm as a gate predicate; a name such as "implicit" usually means classified and not recorded | The parser knowing something the AST throws away, so distinct declarations are indistinguishable downstream |
| Put a recovered parser fact in as the last fallback so an explicit declaration still wins | An explicit range being overridden by the recovered default |
| Ask a guard in the domain of the question it answers: whether a declaration is of a type is a declaration question, whether an expression folds to one is a value question | Widening the resolver refusing a legal override, and reverting letting the folded default swallow it |
| Separate "the width can be computed" from "the width's provenance can be vouched for"; to claim the second, every leaf on the path must vouch for it | Provenance being laundered through a map that also records inferred widths |
| Enumerate the observers of every effect you move, asking whether a name reads that resource, not whether it uses its argument | An overlap gate keyed on root names being empty, and its emptiness becoming the reason to pass |
| Fix a stale proxy by retyping it — take the set it was always about — not by extending the predicate | Each new domain needing another alternation term |
| Check what the other paths into a guard build | A predicate with no arm for a generated shape falling into its catch-all and measuring as a no-op |
| Record a syntactic fact at the site that knows it when a predicate needs a fact the data structure does not carry, and test the degenerate count | A degenerate case leaving the same key as the general one |
| Ask whether a funnel is reached before the name is resolvable | "Every write position calls it" not being "every write position is checked" |
| Use a three-valued provenance record for scope resolution: set-or-clear cannot distinguish "this scope bound the name and it is not declared" from "this scope never bound it" | The walk sailing outward and vouching for an ancestor |
| Name a stored key's lifetime: a layout that names another type by its bare key dies at the end of the unit that declared it | Cells outside the one spelling that was tested failing |
| Apply each import before the first thing it must be visible to, in two passes around the binder, not "earlier" | A body import reaching the header |
| Key a new path positively on the names it is for, never on "did not fold": that predicate is two populations | Deliberate declines being admitted to the new path |
| Read what the runtime reads for the same decision when a new analysis follows an IR field for control flow; a field only ever patched after the fact is a snapshot, not a fact | A walk missing every target whose placeholder is still in place |
| Record a stand-down after the enclosing construct's scope snapshot so the restore drops it, and pin both halves | The fix trading one silent width for a permanently loud site |

### 5.6 Domain reference

Two reference rows carry the width and name axes that recur across gates. Each item is a measured
source of silent-wrongs; keep them in agreement when touching either axis.

| Axis | Rules that hold together |
|---|---|
| **Width and type** | Keep the self-width table and evaluation in agreement; route a width branch on the storage-kind discriminator before width, because handle kinds have width zero; keep string routing single-sourced; use the context-or-plain lowering for a target-width fill; read four-state raw as value masked by known; extend a resize by the right-hand sign and stamp the target sign; guard real-to-integer strictly; apply two-state unknown-to-zero per write path and per storage; carry string and handle formals in a sidecar mask; keep type signedness symmetric across every declaration; make comparison and case collective per the standard; give an untyped parameter its type from the value, failing open; const-fold only a single constant as provably safe |
| **Name and scope** | Thread sticky attributes across a comma list; pair a flat map with nested scopes by lazy snapshot and restore covering both type and variable over the whole declaration region; keep alias and copy side maps name-keyed with set-or-clear; treat a flat registry plus scoped resolution as unmodelled scope precedence and file it as infrastructure; mirror a new variable binding on the declaration binding with enclosing snapshot and restore isolation; track consumption in collect-then-apply and make leftovers loud; funnel symbol aliases through the one resolver; normalise a sub-select offset by subtracting the declaration base and make a clamp loud rather than silent |

## 6. Measurement

Measurement beats argument. A claim about behaviour is not settled until a probe, a sweep or a
census has produced it, and the probe itself is a claim to check.

### 6.1 What counts as evidence

| Rule | Prevents |
|---|---|
| Write why a counter-example is structurally impossible when it cannot be built | A failed reproduction attempt being recorded as "it does not happen" |
| Treat a line that differs from the oracle inside an anchor as a finding: measure it, record it, assign an owner | A divergence being filed as anchor noise |
| Confirm whether a diagnostic's severity decides the exit class before writing that only the error stream differs | An item being graded a grade too low and deferred |
| Confirm which stream a diagnostic goes to before writing "unobservable"; counting streams and checking streams are different jobs | A same-stream case being recorded as needing merged descriptors |
| Distinguish a projection from a measurement in writing, and re-measure when a batch closes | A projected number being cited as a measured one while new rows drift the accumulation |
| Settle a claim about attribution with a corpus sweep | Two lenses reporting opposite things about the same shape |
| Ask which instrument would have made an outside diagnosis right when it is wrong | The missing instrument being the more valuable item and going unbuilt |
| Re-measure the re-measurement: a refutation is a claim too | A narrow census refuting a correct report because it read one output column |
| Re-measure a revert's stated reason before ranking or building on it | A revert propagating its reason faster than a feature propagates its behaviour, so several documents citing each other are one measurement |
| Delete the competitor and ask again when a tool's answer matches what a competing write would leave | Ordering and a dropped write looking identical |
| Measure a candidate discriminator against the design that matters | A map's docstring describing its intent while only a run describes its contents |
| Build the design that tests any bound you write beside a known imprecision | A bound asserted in the same breath as the imprecision going unmeasured |
| Count the input distribution before arguing that a control-flow difference shows up in other bits | An untested assumption standing in for a measurement |
| Name the axis of a byte-identity argument and enumerate the observation channels — value, diagnostic, exit class, order, time — and check which channel someone else's comment was about | "The values are the same" being read as "the output is the same" |

### 6.2 Probes

| Rule | Prevents |
|---|---|
| Re-derive a stated oracle rule from a probe finer than the effect | The probe's own rounding being read as the oracle's answer |
| Pick a probe from the smallest quantity the rule can produce, not from the design's units | The probe being unable to resolve the effect |
| Use the band's edges as the proof of the mechanism: the fix must move every cell inside the band and none outside it | A fix accepted without evidence that it is the right mechanism |
| Build a twin that fixes the axis a report names and varies everything else | Designs differing in several places being unable to say which difference produced the result |
| Overflow the destination in any probe that measures width or truncation, and make the grid finer than the delay being measured | A value that fits giving the same answer whether narrowing happens or not |
| Compare byte output through `hexdump -C` | Control bytes disappearing in a terminal, so output reads as empty |
| Suspect the harness first when a probe's conclusion disagrees with a constant read from the code, and observe through a width-preserving path with no convenience conversion | A convenience conversion truncating and coincidentally matching the oracle |
| Use a width-preserving format in any probe that measures width | A convenience conversion erasing the discriminator |
| Never truncate PRE output | A failure on the second line being missed and a pre-existing defect attributed to your change |
| Re-measure attribution yourself even when two lenses converge, and read a debug-only assertion as a smell because debug fails where release is silently wrong | A pre-existing defect being filed as your regression |
| Measure an ordering rule with an observable witness whose value differs per occurrence, and measure the whole rule before fixing one symptom | A report seeing one face of the rule and the patch breaking another relation |
| Move a coverage instrument so it answers for every executor | An instrument present in one path making an experiment look dead |
| Flip the default and run the whole suite as the cheapest coverage instrument; it is a measurement, not an implementation, so revert it afterwards | A corpus differential being far weaker and missing the alternative path entirely |
| Instrument all layers independently even where production short-circuits | The layers behind the first refusal never being measured |
| Say so when the attribution unit is contaminated, and check that the weight is not one design repeated | A count of tests being meaningless because most of them are one path |
| Read the run manifest's backend and refusal fields before claiming two backends agree | A comparison that did not check which backend ran not being a comparison |

### 6.3 PRE, POST and sweeps

| Rule | Prevents |
|---|---|
| Extract PRE with `git archive main` into a scratch directory (`tar -x -C <scratch>/presrc`) and build it separately, not as a worktree; a change to existing binding or classification requires it | An oracle-only differential being unable to distinguish a correct-turned-loud cell from a pre-existing gap |
| Score a PRE-and-POST sweep in three classes: loud-to-correct, silent-to-loud, wording-only | A two-class sweep hiding the class that is a regression |
| Ask of every queue mechanism both which cells become correct and which cells are correct today | The change fixing the reported cell and breaking a neighbouring correct one |
| Re-measure a queue row's mechanism as carefully as its symptom | Fixing the recorded site breaking a cell that is correct today |
| Run the shapes your new lane handles through the existing lane and record what it returns before placing yours later; a wrong number forbids the safe placement | "It can only add answers where the old one had none" preserving both silent-wrongs |
| Diff the per-file distribution and the first line of each file's page after a ladder rung, never the total | A diagnostic cap hiding the pages behind it, so clearing some leaves the count unchanged |
| Measure what the baseline did for the shapes a stricter invariant would newly refuse; a restriction is only safe where the baseline was already refusing | A simpler, stricter rule regressing in bulk |
| Read a differential sweep as certifying only the fields it varies | The only axis that mattered never being varied |
| Measure narrowing, equal and widening separately when changing a site that passes a context width down | The same code being right for widening and a different computation for narrowing |
| Write down which axes a sweep multiplies before counting cells, and do not claim zero regressions from your own sweep | A large sweep of one shape saying zero while a sweep of another shape says otherwise |
| Count fixed and regressed separately | A total hiding two directions that nearly cancel |
| Write the axis list before building a sweep and hand it to the reviewer with a demand for the missing axis | A missing axis not announcing itself, and adding cells not being a defence |
| Measure the fallback plan too | "The intersection that never regressed" being a hypothesis |

### 6.4 Oracle censuses and their budget

| Rule | Prevents |
|---|---|
| Do not count diagnostics with the `VITA_SCW_CHECK` self-check enabled | The check's own diagnostics inflating the count |
| Budget a hand-run oracle census at the rate it actually sustains — a `verilator` census runs about 1500 cells per 30 minutes: run it over a width subset, keep one `--prefix` per executable, and hand-IEEE the cells whose oracle is untrusted, saying so in the briefing | The census not finishing, or an untrusted oracle cell being recorded as measured |
| Treat a width and its value as one answer: truncation commutes with some operators and not with division, remainder or right shift, so always pin a cell from the non-commuting family | A census built from the commuting family certifying a value that was already truncated |
| Ladder the axis you changed across the width boundaries: 8, 16, 32, 33, 64 | "Fixed" being indistinguishable from "fixed below the old default's width" |
| Treat a value-preserving wrapper as a width claim: name the width each side computes at and check the identity holds at both | The stated invariant not being true at the width the expression uses |
| Treat the order of two queue rows that share a root as a measurement: run the cells that separate the two orders, not the cells the rows quote | Cells that both candidate orders answer the same way being cited as evidence |
| Find the cell where two candidate rules differ before adopting one, and check it is the cell you measured; where each rule owns a disjoint set of leaves, ship both and say what separates them | A cell both rules answer the same way being recorded as a measurement, making the choice a coin flip |
| Measure a warning that a change has a wider blast radius; the decisive probe is the size at which the wrong reading stops being out of range | A slice being priced for a sweep it does not need |

## 7. Testing

The full local gate is `cargo nextest run --workspace --locked`: 7352 tests, 15 skipped. Named gates
that must be green in the same commit as the change that moves them are the `sim-ir` schema-hash,
frozen-shape, no-float and body-reference suites, the artifact header and round-trip gates, the
diagnostic-code bijection, the parser depth and node-budget guards, the live `iverilog` differential,
backend equivalence, and the vendored-libm determinism pins.

A test has teeth when a wrong implementation fails it. Coverage, a green suite and byte-identity are
not teeth on their own; the standard of proof is a mutation that must die, a control the fix must
move, or an anchor no shared code can shift.

### 7.1 What gives a test teeth

| Rule | Prevents |
|---|---|
| Run a new guard's test against the reverted binary: green is coverage, a red revert is evidence of teeth | A test passing because the design never burns the code path |
| Measure entry to a zero-coverage surface with mutation | An honest body plus a green suite plus code in the file reading exactly like coverage |
| Know the axes mutation cannot see — fields nothing compares, summed floors, aggregate assertions satisfied by one stub, and catch-alls accepting any failure — and pin counts exactly, one property per assertion | A battery reporting full coverage over an unprotected field |
| Count what a gate executes by statement and effect kind, and map each kind to the observer that can see it: store, queue, diagnostic, exit code, arm state | Half the executed statements having their whole effect in a queue, so dropping them keeps the suite green |
| Build the design where the two inputs diverge | A parameter's existence being unverified while every caller passes the same value |
| Observe both sides separately when verifying an asymmetric rule | A one-sided observer certifying both sides |
| Keep one condition per question, and re-run the whole mutation set after a fix, looking at what came back to life as well as what newly dies | Overlapping guards deepening defence and destroying observability, so a fix silently kills another test's teeth |
| Assert that a probe entered the branch it was written for, with a counter or an ordering assertion | Mutations passing vacuously because the branch was never entered, the smallest value often being a different branch |
| Mutate each operand of a sum, product or shift separately, and check the harness default is not that operation's identity | An operand being indistinguishable from the whole expression because the other one is always the identity |
| Test provenance by deliberately desynchronising two sources and asserting that a call given one answers with that one's value | A gate asserting the two results are equal passing an implementation that ignores its argument |
| Do not add a parameter no mutation can kill | A dead parameter being indistinguishable from a real gap |
| Include the operations where two paths diverge — comparison of unequal lengths, concatenation, replication — not only the ones where they coincide | A whole operator family staying hidden behind equality and length |
| Put a detector for "no output and a success exit" in every sweep, and grep the corpus for the combination when a sweep is silent | A regression that deletes the result entirely scanning as clean |
| Prove a differential gate has teeth by reverting each of its behaviours one at a time and requiring every revert to fail, drawing the behaviour list from the call sites | Vacuous behaviours and store sites that are never entered |
| Treat observation granularity as an axis and sweep both: per-event observation erases batch effects, batch observation buries per-event effects | Half the matrix never being swept because two axes share a predicate |
| Verify a gate's teeth on new code with a deliberate failure, and read a coverage number that does not move as the signal that the path is not entered | A narrowed refusal leaving the admitted count unchanged and the work looking finished |
| Re-read what a failing test models before calling it a regression; a gate can harden against the fix | A correct fix being reverted because its paired initial state was not updated |
| Test a cache by keeping the owner alive and changing the input state; when ownership makes that hard, build a seam that hands over state without handing over the cache | A fresh owner per state making staleness structurally invisible |
| Count a positive marker in any probe that asserts absence; zero means suspect the harness first | "The diagnostic disappeared" being a harness failure that reads as a finding |
| Read the assertions before calling a test a pin or a measurement, and verify the premise of any skip or early return on the spot | A census test asserting only that something is non-zero and printing its numbers to a captured stream |
| Compare diagnostic counters as well as values in an equivalence gate, and count whether the corpus contains an admitted shape that moves the counter | A duplicate diagnostic eating the per-run cap, and a comparison of zero against zero certifying nothing |
| Confirm that a battery, sweep or fuzz you cite actually passes through the branch in question before writing that it was measured exhaustively | Every case in the battery sitting on one side of the branch |
| Do not leave a gate suite red: flip the behaviour's assertion in the same edit that changes the behaviour, and re-measure after removing a refusal row | A red test masking its own area's mutations |
| Give a dedicated observation channel to each axis where the value does not move: operand order needs a side-effecting evaluation, stage placement needs a boundary sweep | A value-only gate passing every reordering and every relocation |
| Observe only what the mutation moves in an anchor | A broad observation dragging in a known divergence, so the anchor certifies wrong behaviour and later blocks its fix |
| Name the anchor that protects shared code every time you widen sharing | Sharing removing drift and removing the differential's sensitivity with it |
| Pair an artifact comparison with an assertion that the artifact exists | Two absent artifacts comparing equal and counting as a match |
| Count every line that mentions a signal when moving it from one place to another: update the positive assertions and re-aim the negative ones | Negative assertions staying green in a file where the subject cannot appear at all |
| Count the whole family of sidecar tables a feature needs, not one of them | A missing sidecar making a test vacuous rather than failing, so both backends do the same wrong thing |
| Compare stdout, diagnostics and exit class in a backend or path differential | Omitting one channel letting a descent on that axis pass |
| Verify a guard actually fires on its target subset, a direct per-item count being the robust form | A vacuous guard reading as protection |
| Pin any "this wrapper covers every site" comment with a test | Most of the sites being covered and the rest running away |
| Ask what the number would be if the feature did nothing; when that equals the expected answer, the cell is decoration | A probe whose answer equals its failure mode certifying itself |
| Establish a pin's claimed property a second way | A property that holds only one way measuring a path, not a property |
| Run the neighbours of the fixed cell, not the fixed cell | The fix being confirmed on the only cell that was ever checked |
| Build a design with two same-time processes writing the same variable to test an ordering change | A green suite, a green corpus and byte-identical waveforms being no evidence at all about order |
| Convert all deliberate-loud pins before re-running, because a test asserting many cells stops at the first | A fail-fast test hiding the pins behind it |
| Test a twin renderer against the runtime spelling of the same construct in the same design, byte for byte, before consulting the oracle | The twin approximating, with the divergence found only by an oracle sweep |
| Write the census first and copy the oracle's raw line into the test | Hand-computed expected values not being pins |
| Make a loud pin name the gate it measures, and test it by changing the spelling the gate does not key on | The pin being loud because of a different gate than the one it claims |
| Prove the rest untouched with the cheapest byte-identity oracles the repository has — the examples' waveforms, the corpus digests and the full suite — before any review round | An observationally unconfined change being reviewed instead of measured |

### 7.2 Anchors, differentials and oracle-free areas

| Rule | Prevents |
|---|---|
| In oracle-free areas the teeth are hand-IEEE pins plus an internal equivalence differential: a new spelling must be byte-identical to a verified existing one | An unverifiable area having no regression detection at all |
| Ask what a simulator you are importing a rule from merges that vita keeps separate — storage, defaults, identity, event channels — and write the failing design for each merged field | The sentence the rule rests on being true of one object and false of the other |
| Build the twin that differs only in the merged field | Both naive translations shipping and failing in opposite directions |
| Pin the working form as well as the non-goal when pinning a non-goal | The loud widening later and swallowing the working form |
| Choose a base case verified correct on every axis except the one under test | A broken base case being unable to separate two defects |
| Verify a mutation actually changes meaning before reporting survival | An equivalent mutation's survival being filed as a coverage hole |
| Test save and restore with nesting, never with siblings | A design that resets on entry making siblings structurally immune |
| Put the discriminating operand somewhere other than the first argument slot | Two different implementations giving the same answer on the first slot |
| Pin at least one cross product of a new axis with each existing axis | Two test files each having one axis and none having both, which is exactly where the regression lives |
| Add a guard for an invisible failure even when its value is only recovered by the next slice | A value that silently disappears |
| Confirm with mutation, in every change that moves code into sharing, that an absolute anchor protects the rule | Shared-function mutations passing the whole differential battery |
| Treat a differential's teeth as decreasing as delegation grows: full coverage can mean no oracle teeth, so a delegation change owes an absolute anchor | Both backends printing the same wrong answer with the differential green |
| Read the product build as having no oracle: the alternative executors are selectable only behind a default-on feature, and a build without it cannot choose them | "Byte-identical to the other backend" being cited in a build where the other backend cannot run |
| Ask the flip run in the current default's direction so the oracle axis keeps being exercised | The suite quietly becoming single-backend and the whole rule going vacuous |
| Pin an emptied gate table with an emptiness assertion and a note on why it is empty, rather than deleting it | The next row being added with no obligation to build a design or say why it cannot |
| Invert a test whose subject has disappeared — keep the design, flip the expectation — and retire it only after moving the teeth somewhere else | The design that produced the last real case being deleted with the test |
| Wire a losing experiment into the real executor and run the whole suite | Being behind a feature flag being treated as an exemption from verification |
| Measure the safety of buffer reuse with a contamination probe: fill the borrowed buffer with garbage before every call and run the whole suite | "Nobody reads a slot this call did not write" staying an assertion |
| Classify a surviving mutation three ways — equivalent, blind axis, or redundant — and delete a duplicate check the callee already performs | A redundant check making two twins disagree about where the question is answered |
| Grep the name of any test a comment cites as a lock | The cited test never having existed, so the path was never locked |
| Justify a guard whose property the return type cannot carry by the other half of the rule, not by a test | A comparison that is structurally blind to a type stamp |
| Update a documentation pin by strengthening it | A pin that checks one word letting the same class of falsehood return |
| Strengthen the pin that protects a user-visible sentence in the same change that alters it | The help surface being the least-tested and staying false longest |
| Read a verdict path narrower than the failure modes as no information at all | Green meaning nothing |
| Prove an equivalence without restating the rule: ask the same bits at two widths so the general path is the oracle | The test containing a copy of the rule |
| Make a pin a derivation, not a record: write why the number is what it is | The next reader being unable to tell whether the number is right, and re-pinning a wrong one |
| Run a property anchor against every backend, not only the one you changed | The sibling backend keeping the same defect |
| Name the mutant that would survive without a proposed test row before adding it | A row that decides nothing being worse than none, because it reads as coverage |
| Include rows on the reject arm | A battery made only of admitted rows being unable to test admission |
| Read what a harness filters before reusing it | A filtered harness being structurally blind to the filtered axis |
| Check that a test row leaves the defect's symptom somewhere it can be observed; position decides whether a row kills a mutant | A defect being structurally immune in the position the row happens to use |
| Give a fast path placed inside the canonical implementation a test-only entry point with an explicit switch | Every existing test through the canonical path exercising only the fast path |
| Compare every channel a new store point owns: value, dirty set, edge kind, last writer, waveform queue, deferred diagnostic queue | A value-only snapshot being blind to the other channels |
| Review your own test design and assert that the code under test runs | A test design that never enters the paths it was written for |
| Re-run a mutation that died by non-termination against your own tests only | A suite that stops being unable to say which row discriminated |
| Ask where the discriminator would be before calling a surviving mutation equivalent, and write "equivalent" only after failing to find one | A vacuous row being built, or a real defect missed |
| Write the equivalence test for a shape fast path against the function you are skipping, not against a table of expected answers | The fast path and the general path disagreeing about a shape the fast path did not compute |

### 7.3 Oracles: choosing, disqualifying and recording

| Rule | Prevents |
|---|---|
| Pin the fact a self-describing artifact asserts, never the phrasing | A test asserting the sentence and passing while the sentence is false |
| Assert the value when a pin's subject is a value, and strip every other error source from the cell to confirm it still fails | A test asserting a non-zero exit being satisfied by an unrelated error in the same design |
| Ask each tool the same question in two positions — a direct interrogator and an indirect one — before recording an axis as an oracle split; a tool that answers those differently is not an oracle there | An axis being parked as unarbitrable while a third, wrong answer is kept |
| Prefer a suite run to an argument | A decision being defended in prose while a shipped test already refutes it |
| Pair every relation pin with a test that pins the values against an oracle, and say so in both files | Both columns being uniformly wrong, the relation holding, and the suite staying green |
| Put two spellings of the same access in one design with the same bits; when an oracle answers them differently it is not the oracle for that cell | Two designs making one contradiction read as two independent results |
| Write "unmeasured", not "equivalent", when you could not construct a reaching design | A gap being closed on paper and refuted by a small design later |
| Write linear tests knowing they cannot see exponential cost | A depth test proving nothing about the limit it was written for |
| Bring a second oracle before declaring vita ahead, count how many places the claim is encoded, and leave the list of things to flip with it | A single-oracle conclusion being pinned in several places and then refuted |
| Let the ladder decide when two oracles split, not a majority, and measure a new oracle's scope of applicability before using it | A masking oracle producing plausible garbage that is adopted as an answer |
| Give a memo its own test | The cache being unprotected and indistinguishable from dead code |
| Confirm the harness installs a feature's sidecar before testing that feature | A missing sidecar making the path nonexistent rather than failing |
| Build the oracle for a string-returning method as a synthetic function and check exact length | An oracle silently padding its own answer |
| Remove a test temporary directory before creating it; process identifiers are reused and each test is its own process | A test asserting absence being red only in the full suite, which reads as a product defect |
| Build the strongest regression test as an internal equivalence differential: a new spelling produces byte-identical output to an equivalent existing spelling, and unsupported cases are identically loud on both | A new spelling drifting from the verified one |
| Fix prose in place and leave correct assertions alone when a reading was wrong; then add the fine-grained twin so the rule is pinned once | Correct measurements being deleted along with the wrong explanation |
| Check that the cell a row's property rests on compiles on both oracles before shaping a fix around that property | A rejected cell being recorded as an oracle split |
| Run the control spelling on every oracle before writing an oracle count, and re-run it when the row is picked up again | A row underselling its strongest cell, or overselling a one-oracle cell as two-oracle |
| Record the crash text and mark the cell one-oracle when a tool crashes | Silence being read as agreement |
| Keep vita's own explicit spelling as the regression oracle where both external oracles refuse | An oracle-free cell having no regression detection |
| Treat a failing deliberate-loud pin as a claim to re-measure against the oracle, never a regression on sight, and move the file's prose reason with it | A loud-to-correct conversion being reverted as a regression |
| Ask the oracle whether a sibling path should follow, never a consistency argument | Extending by consistency alone voluntarily enlarging the unverifiable area |
| Measure first the axes where a two-state tool is not an oracle: unknown values, out-of-range indices and event or delta order. Its scope is two-state arithmetic, width and sign | Plausible garbage being adopted as an oracle answer |
| Read zero suite coverage of an arm as unverified, not as true | A claim being true everywhere the suite reaches and false outside it |
| Record an oracle's own defects with the width or range condition attached | The next sweep reading the oracle's defect as a vita regression |
| Let the specification decide, not a majority, when two oracles are each wrong on a different axis, and record the verdict with its condition | A self-contradicting oracle being followed |
| Read a value differential as blind to a performance collapse; the gate is what catches it | Two lenses and a large sweep passing while a cost explodes |
| Ask whether an oracle exists rather than assuming either way, and expect it to be half an oracle; pin the halves to different tests | A whole area being treated as oracle-free for many slices |
| Pin the boundary of a new leniency with the oracle by running each position, rather than inferring it from a specification sentence | "It is standard, so be generous" becoming silent acceptance of typos |
| Anchor a new benchmark against an external published reference value as well as against mutual agreement between tools | Several tools being wrong together |
| Convert a test that pins a loud into a value pin when the loud becomes correct, and keep the narrative in the docstring | The reason it was loud being lost |
| Contrast a suspicious construct with a different type in the same position when the parser may have silently demoted a lifetime | A qualifier being dropped in silence for one type only |
| Measure an obvious fix and revert it when wrong | Reading a specification sentence without separating the two situations it covers |
| Read a two-state oracle's zero as its two-state-ness, not as an oracle split; where one oracle rejects and the other accepts, it is a real split and off limits | A two-state artefact parking an axis as unarbitrable |
| Record the second oracle's actual output text, not the conclusion drawn from it | The load-bearing half of "both oracles agree" being the one that was assumed |
| When a fix removes an answer and names ONE fallback, census every producer that does not feed that fallback and measure each on the must-stay set; a cell that is right only because two widths coincide is not a control | An operator-top override and `defparam`, correct by a width coincidence, regressing 8 → 32 the moment the literal arm was gated (§4.5.470) |
| Check a testbench for same-time-step blocking writes to a sampled input before reading a finish-time or cycle-count divergence as a defect; two oracles agreeing on a §4.7 ordering race is a coincidence of their schedulers, and the race-free (non-blocking) form is what settles it | A conformant process order being filed as a silent-wrong |
| Ask whether another lane of the tool already answers the question before reaching for an external oracle; a self-contradiction proves a defect and needs no third party | A defect that one binary demonstrates against itself going unmeasured |
| Name the missing capability in a loud pin's docstring, with the measured expected values from both oracles | A pin that says only "this is loud" telling a future reader nothing about whether loud is still right |
| Open and read the cited text when a comment calls another implementation buggy | The standard and the other tool being on the same side, with vita the outlier |

### 7.4 The mutation battery

| Rule | Prevents |
|---|---|
| Default the battery scope to the whole workspace — `cargo nextest run --workspace --locked --no-fail-fast` — and never select packages and a test target together (`-p A -p B --test X`), because the target filter applies to every selected package | A narrow filter manufacturing false survivals |
| Run a generous narrow set first and re-confirm only the survivors at full scope when a full pass is prohibitively slow; a narrow filter can produce a false survival but never a false kill | Either abandoning the battery as too slow or trading away its soundness |
| Count non-termination, crash signals, leak failures and retried failures as kills, not only plain failures: scan the runner's output for `FAIL`, `TRY 1 FAIL`, `TIMEOUT`, `SIGSEGV`/`SIGABRT`/`ABORT` and `LEAK-FAIL` | A hang or crash being reported as survived, filing a real defect as covered |
| Treat survival as unexplained: build a discriminator and measure it, write the equivalence argument into the code where it is equivalent, and prove unreachability with a deliberate failure | Survival being filed as equivalence and a real gap closed on paper |
| Investigate a fake survival anyway; the question is why it did not die, not whether the mutation was applied | A false survival being discarded along with the real defect it points at |
| Fill an expectation column before running: state the expected outcome of every mutant | A wrong expectation passing unnoticed as another death, so a misunderstanding ships |
| Give every emitted table a gate that is an asymmetric upstream mutation: name the change its numbers must be invariant under and pin that pair | A determinism golden that runs the same input twice being unable to see a rail that reports the wrong number |
| Treat a count protected only by tests that never exercise two producers together as untested | Coverage of each producer alone certifying a combination nobody ran |
| Prove each shortcut mutation's equivalence individually and confirm reachability separately | Survival being indistinguishable from "no test exists" |
| Ask the canonical implementation back with an equality assertion for any value a shortcut invents rather than reads | A non-obvious premise being defended by argument |
| Write down why each mutation survived, and pin a cost-only specialisation with an operation-mix census rather than a value differential | Equivalent and uncovered being indistinguishable |
| Pin the non-vacuous count separately from the total in a differential | A count of agreeing designs hiding how many compared one implementation with itself |
| Do not relax an exact coverage pin into a floor; when a pin breaks often, make the gate cheaper instead | A floor passing after the true value grows and then halves |
| Choose test values that separate the domains: an odd number for division, a value past the word boundary for width, a negative for sign | A value that cannot separate two domains letting a wrong premise stand |
| Include both orders when testing an asymmetric parameter | Only the harmless direction being present |
| Build a test for the correctness argument itself: imagine the mutation that inverts the argument and put the design that kills it in the same commit | A paragraph of reasoning having no pin |
| Verify that a decomposed oracle gives the same answer as the original form | The decomposition itself diverging |
| Run a whole-suite flip when a backend uses an alternative store | A design that runs on the default backend never reaching the alternative store, so nothing sees the defect |
| Do not put a shape an earlier stage already rejects into a reject row's neighbouring pin | The pin being vacuous and claiming the lower stage does the upper stage's job |
| Put a name in every argument position when admitting a new task | A defect whose only discriminator is a non-literal argument being structurally invisible |
| Pin the remaining bypass paths by count per file plus the name of the row that blocks each | The next change that opens a row breaking there first, which is the point |
| Choose the shape of a refusal pin so that both halves of the gate refuse it by their own name | One change admitting every test that used the same refused shape |
| Instrument instead of auditing by eye | A multi-site audit done by reading |
| Record what killed each mutant, and run the battery with `--no-fail-fast` | Several killers turning out to be one test, and change-detector pins reading as coverage |
| Re-check the harness's hand-picked sidecar list every slice | A row having been vacuous from the day it was written |
| Read an unused-assignment warning in a control-flow arm as a possible defect, not a style issue | A missing alternative branch ending a process early |
| Treat building the discriminator for a surviving mutation as a defect-finding procedure, not a mutation-killing one | Survivors hiding a real divergence |
| Check that a refusal design is not refused at an earlier stage | The test passing while measuring nothing |
| Find who re-decides the value before concluding a surviving mutation is equivalent | A downstream re-binding making upstream context invisible |
| Build the one discriminator shape that works, even when it is awkward | Ordinary inputs being unable to discriminate |
| Alias two destinations to see order | Order being invisible when every destination differs |
| Measure whether a survivor is unreachable rather than equivalent, and write the two differently | An unreached arm of moved code being disguised as a kill |
| Pin every neighbour of a removed row with its reason | A wrong reason letting someone later delete a load-bearing row |
| Build an absolute anchor that fixes what a design means as a value | A differential between two backends being blind in principle to a rule they both read |
| Detect an empty harness by writing a test that pins the feature's refusal | The row being green while the design does not do what it says |
| Separate "the gate is weak" from "the mutation is insufficient" before reading a survival | Deleting one of two summed terms leaving the row firing and saying nothing |
| Add and remove an unrelated statement and see whether the answer changes | A classifier that sees only part of the statement passing for an unrelated reason |
| Read clustered survivors as a diagnosis of the test axis and state their common property in one sentence | Survivors being treated as code defects one at a time |
| Keep a defensive arm you cannot reach where the alternative silently drops the statement, and write in the docstring that it was measured dead, how, and what the honest behaviour would be | "Unreachable" and "unreached in every test run so far" being treated as the same claim |
| Ask what the defect would look like and build that, rather than building the shape and observing it is fine | A probe whose operation is idempotent making the hazard invisible |
| Restore both the source and the binary after each mutation case | A source-only restore leaving the canonical build emitting mutant values |
| Derive the restore set from the mutant list, never hard-code it, and compare `git status --short` before and after the battery | A mutation outside the hard-coded set staying applied, making every later verdict a false kill |
| Read "a mutant that should change nothing died" as battery contamination and check the tree before explaining the case | A contaminated run being rationalised case by case |
| Specify each substitution by line number | A pattern matching two sites applying to neither and being recorded as survived |
| Score a mutation that touches a loop-exit condition on a bounded runner only, and run the battery in the foreground | An unbounded simulation exhausting memory, taking the machine down, and leaving the mutation in the tree |
| Check the exit code of the substitution and of the build, record a failure as a build failure rather than a survival, and grep the source for the changed line | A failed edit running a stale binary and the result being recorded as survival |
| Restore a mutation from a byte snapshot copied by explicit path with `cp`, never with `git checkout -- .` | Uncommitted work being destroyed and later kills becoming unattributable |
| Snapshot every file the working tree reports as modified, not only the mutation targets | Files outside the snapshot being reverted and the change's edits vanishing silently |
| Run the restore loop under a shell that word-splits as the script expects: declare `#!/bin/bash`, because `zsh` passes an unquoted `$VAR` list as one argument | The restore silently copying nothing and mutations stacking across cases |
| Install the restore as `trap restore EXIT` inside the script, and keep an isolation guard that diffs against the snapshot before each case | An external timeout killing the command before the restore runs |
| Verify a substitution pattern by counting string occurrences, never by applying it to the tree and reverting | The verification pass itself reverting unrelated files |
| Flush the runner's results line by line, keep a copy of the original outside the tree, and confirm by process identifier — not by `grep -c`, whose quoting returns a false zero — that no previous runner is alive | Two runners writing the same files and leaving a mutation in the tree |

### 7.5 Golden, corpus and determinism gates

| Rule | Prevents |
|---|---|
| Put the minimum condition that breaks a boundary into the boundary's test, and do not let a docstring carry a wrong argument as authority | The test named for the boundary not defending it |
| Build an order-sensitive probe on the side where mapping order and execution order differ | A same-side probe being unable to separate the two orders |
| Suspect the harness first when a differential reports zero divergences: assert the row counts of both files and the key ordering, and plant a known divergence | A comparison of zero rows reading as agreement |
| Make a diagnostic-counting needle a discriminating fragment of the rendered form | A one-character needle being coincidentally right and later wrong |
| Measure a corrected diagnostic message against the cases that still reject, not against the ones the change opened | The replacement claiming support for a position that is correctly refused |
| Accumulate a digest over the whole run, folded per cycle after reset, not over the final state | A final-state print hiding every divergence the design later overwrites |
| Mutate one line of the upstream design and re-run every new workload, golden or differential gate | A digest that does not move being empty and indistinguishable from a working one |
| Make the mutation asymmetric, touching one end of the data path only | A symmetric mutation cancelling on both ends, so "the mutation did not take" and "the workload cannot measure" are confused |
| Give a corpus row a third state when its axis has been ruled unarbitrable: pin both answers and name the ruling | The row being permanently red, or pinning vita's own answer and self-certifying |
| Put two spellings of the same construct side by side in one file with one output line where a language offers several spellings of one meaning | Spellings differing and nobody noticing |
| Pin the opposite half of any rule you fix | Being unable to tell whether the rule was fixed or moved |
| Pin the two boundary cells, unsigned wrap and signed wrap | A later simplification undoing the narrowing silently |
| Put the widened spelling and the original spelling side by side in the census | A width-twin defect being invisible without the pair, where the tool contradicting itself proves a defect |
| Put the observed value in the assertion of a residue-pinning test, quoting PRE only after running PRE | A regression being enshrined as expected behaviour |
| Test a state-moving transformation at the boundary | A value-dependent defect agreeing with the oracle in the middle and diverging only at the end |
| Use two cheap detectors when routing: the domain twin and the scope twin | The suite going green with both defects present because no test paired the new domain with those contexts |
| Pin the wording of a refusal only while the construct has no value, say in the docstring that it is a wording pin, and convert it to a value pin when a value exists | Wording pins breaking in bulk when the refusals they quote are removed |
| Choose probe inputs where every wrong implementation gives a different answer, and say in the comment why that input | A fixed-point input passing whatever the implementation does |
| Assert the value in a new battery cell rather than an exit code | Predicting an oracle's answer and pinning the prediction instead of measuring it |
| Flip both spellings of the default backend for the flip run — the `Backend` derive's `#[default]` and the `backend:` literal in `SimOpts` initialisation; changing one moves half the CLI | A flip run that exercises only one entry point |

## 8. Performance measurement

A performance number is a measurement with a method, or it is nothing. Every A/B is release-built,
interleaved, run in both orders, and attributed to a mechanism before it is used.

### 8.1 The A/B protocol

| Rule | Prevents |
|---|---|
| Interleave a performance A/B by run and run both orders, discarding the first run | Sequential blocks and a single order each producing a result whose sign is an artefact of position |
| Measure with release binaries only, check the binary size when snapshotting, and record the profile in the briefing | A debug binary reporting a large fake regression |
| Measure retired instructions, not wall time, when the target delta is below one percent | Repeated rounds disagreeing in sign between minimum and median |
| Build a harmless control binary — the pre-change source plus the layout change only — for any A/B; per-shape movement of a percent or two is layout, not execution | A field addition alone moving benchmarks in both directions with no executed code changed |
| Check that two profiles share a denominator, and record both | A run that ends inside the sampling window shrinking the denominator, so every unchanged function looks larger |
| Convert share to share times wall time when asking whether a function changed, and keep the post-change share as-is when asking what is expensive now | An overall improvement raising every unchanged function's share |
| Run an A/B back to back and re-measure the baseline each time | Machine state drifting within a session |
| Bisect a performance regression the way you bisect a wrong value, using a synthetic probe and a control twin that changes one attribute | A real design mixing two costs that a probe separates |
| Build measurement discipline into the default shape of the tool — take everything to be measured at once, so round-robin is the default and the first round is discarded — and warn when handed a debug binary | Each caller re-deriving the protocol and getting it wrong |
| Choose a committed control for a performance record | Numbers from untracked third-party material not reproducing |
| Build with `CARGO_PROFILE_RELEASE_STRIP=none CARGO_PROFILE_RELEASE_DEBUG=1` and a separate `CARGO_TARGET_DIR` to profile, and parse the "Sort by top of stack" section | Stripped symbols making every frame anonymous, and the call-tree section giving the root everything |
| Verify inlining after extracting a hot tail, and use the built-in control group of shapes that never call the new callee to attribute movement to layout | A small extraction costing several percent until it is inlined |
| Write the design and workload a performance sentence was measured on into the sentence | A statement about headroom being refuted by the next shape |
| State the design and the workload behind any estimate of what an optimisation is worth | A ceiling from one design being a property of that design |
| Record the shape a revert's measurement covered, not only its verdict, and re-run when a new workload shows that shape | "It buys nothing" being true of the designs measured and false of real RTL |
| Say which design a cost was measured on and what fraction of its run the cost could occupy when grading a cost invisible | A measurement that could not have shown anything being cited as evidence of nothing |

### 8.2 Attribution before optimisation

| Rule | Prevents |
|---|---|
| Do not use a measured gain as a result until you have a mechanism for it | The number being right and the attribution wrong |
| Record both absolute times with any ratio between two layers | A ratio alone lying when the other layer improves |
| Re-profile before trusting a recorded bottleneck, and read "my specialised code is barely in the profile" as the path not being entered | The queue naming one component and the profile naming another |
| Do the division even when the candidate looks small — its value is that it makes you open the function | A larger win inside the same function going unfound |
| Do the division before building: calls to move times cost difference per path, over total runtime | A profile percentage being mistaken for a target size, and the first failing gate for the only one |
| Multiply a scan's unit by its call frequency and count the calls by instrumentation | A function's name not giving its unit, so a per-delta cost reads as per-timestep |
| Ask first whether every fast path that already exists is being called | A helper that documents its own reason for existing having a caller that never calls it |
| Look for places that build a proof and then discard it | A structure that crosses regions dropping the classification the next region has to recompute |
| Open the call graph under a profile line | A top-of-stack row being an inlined blob that does not name the target |
| Re-read the "this is an allocation choice, not semantics" notes on any path a routing change makes newly hot | A per-call scratch allocation that was free becoming the cost |
| Ask the real admission predicate by extracting and calling it, rather than approximating a boundary with a necessary condition | Approximating the boundary sending shapes to no evaluator at all |
| Profile after the census: a remaining rejection list does not mean the axis is worth anything | Opening a whole axis for a ceiling that is a rounding error |
| Measure what percentage a shared function occupies in each layer before writing that fixing it benefits all of them | One layer having its own inlined copy and another barely using the function |
| Measure both candidates' ceilings with discriminating designs before ordering them | Choosing by count being luck rather than an argument |
| Count how many branches inside a hot function actually run before optimising it; the profile says where it is hot and only a census says which shape runs there | Optimising the branch that almost never runs |
| Write the stop verdict before implementing: sum the profile share of what a stage targets and compare it with the stop threshold | The stage being built and then scored |
| Count which lines inside a hot function disappear, not the function's share | A hot function's name not being what it does |
| Ask whether a clone in the hottest loop is required | A clone added to satisfy the borrow checker outliving the condition that needed it |
| Date any fixed-cost number a plan rests on and re-measure it before deciding | A plan built on a number that has since changed by a large factor |
| Treat headroom and a plan that captures it as different propositions; when the remaining stages sum to a fraction of the ceiling, change the axis or close and record it | "Try harder" replacing a decision |
| Count the executed operations before writing a code-generation plan | The plan's premise being a claim about the hot path's composition that nobody measured |
| Check whether the reason a previous attempt lost still holds | A preceding stage having consumed a later stage's justification |
| Treat a function you decided to share as a wall for code generation; one spelling and inlined cannot both apply | The decision being made implicitly as an implementation detail |
| Look at the program-length distribution: when half the executed programs are one operation, the cost is in the calling convention | The optimisation targeting what is computed instead of what it takes to compute one thing |
| Measure the ratio of choosing which call to make against what the call does before changing the representation | Replacing one form with another being a complete wash |
| Treat a register file as an interface: splitting a statement into two operations sends a large value to memory and back | The compiled form being slower than the interpreted walk |
| Demand a differential for a performance report's root cause as well as for a correctness report's, and delete cause candidates by experiment | A scaling diagnosis being refuted by the oracle showing steeper scaling |
| Do not add an optimisation with no measured gain | A second code path being a drift risk in itself |
| Build a discriminating question and measure it yourself when a report gives you a location | A bottleneck's location not being its cause |
| Acquire the comparison tier and measure it rather than recording that you cannot | A document carrying "the gap size is unknown" while the tool is one install away |
| Run an A/B on whether an optimisation accepts a design before claiming its effect | Coverage being a different axis from speed, with zero coverage visible only by changing the benchmark |
| Count how many times a correctness primitive names its operand: performance is also a ladder | An impure operand named more often meaning more side effects, and a pure one becoming linear in width |
| Build the benchmark that contains the shape in the same change that alters its cost | The cost change being invisible |
| Build the cell where the tool does automatically what a report did by hand | Pasted source text having no formals, so the comparison is not the one the report asked for |
| Count rather than time when an operator looks slow: put a print inside the operand and read the multiplier as an integer | Timing saying "expensive" where counting says the multiplier is exactly the declared width |
| Grep the other callers of the thing you are about to gate | A sibling that already solved it holding the sound predicate, the measurement and often the report |
| Get enough points to fit a curve before accepting or rejecting a scaling claim, and report the residuals | Two points being unable to distinguish linear from super-linear |
| Divide a per-evaluation cost out from the evaluation count early | A conservative purity predicate's blast radius being invisible until someone counts |
| License a skip by a complete dependency set, not by purity: reproduce the rule the oracles use, collect the callee's reads, and decline when one read cannot be attributed | Refusing every call and re-evaluating on every pass |
| Grep any predicate that delegates to a whole-tree helper and then recurses; a promise about the answer is not a promise about the cost | A construct-free input becoming superlinear |
| Ask whether the condition of a loop over a whole collection can be asked once | A loop that reads as processing a subset touching everything |

### 8.3 Baselines and targets

| Rule | Prevents |
|---|---|
| Use `iverilog` and vita's own alternative backend as the performance baseline; both share vita's contract of four-state, event-driven simulation | A different contract measuring something else |
| Never make a two-state compiled simulator a performance target; cite its numbers only as the ceiling compilation can buy, always with the sentence that the contract differs | Unknown-value planes, delta cycles and event-queue cost all being booked as slowness |
| Fix the cost model rather than moving a limit; a recursion depth cap replaced by a node budget is still a cap unless the walk deduplicates | The seal disappearing on a small, deeply nested source |
| Alternate the direction of a fixpoint that iterates a map in declaration order, and leave the honest bound in the comment | A chain in the unfavourable direction settling one link per round |
| Check three things when extracting a block into a helper: whether the block read a local, whether that local remains at the call site, and whether the node kind is a link in a recursive chain — and return early inside the helper when all three hold | Each node folding its operand twice, which is exponential in depth |

## 9. Artifacts and determinism

Artifacts are byte-identical across supported platforms, and the `sim-ir` shapes that back them are
frozen. Byte identity comes before performance.

| Rule | Prevents |
|---|---|
| Prefer a change that leaves the golden IR untouched: check the common funnel shared by both executors first, and keep non-target designs byte-identical | A change flipping the golden root hash for designs it does not affect |
| Treat `crates/vita-artifact/src/header.rs::CURRENT_FORMAT_VERSION` as the only canonical statement of the format version; never copy the number into prose | A restated constant freezing while the real one moves |
| Bump the format version for exactly three reasons: a frozen `sim-ir` shape change, adding a staged trailer sidecar, and an existing sidecar's enumeration gaining a variant | An artifact silently mis-decoding, or a skipped bump giving the user an unhelpful loud |
| Bump for an appended enumeration variant even though it is backward compatible: the bump buys the accurate format-mismatch diagnostic. Inserting a variant in the middle is a frozen-shape change instead | The user getting "undecodable trailer", which does not say how to fix it |
| Put cross-platform byte identity ahead of performance | A faster non-deterministic representation breaking reproducibility |
| Re-pin only the AST schema hash for an AST field addition; a value-only change re-pins neither | A needless full artifact regeneration, or a missed one |
| Use an ordered map when a parser generates AST items | A hash map violating the cross-platform byte-identical golden |
| Sort scope keys numerically, not as strings | Lexicographic ordering interleaving generated scope numbers wrongly |
| Key an order by a pass-independent value such as a source offset when the order's definition is declaration position | Two things counted in different passes never interleaving |
| Split a counter per slot when two traversals share it and visit different sets | The same item getting a different number in each phase |
| Write out the whole vector being compared before citing a sort key as justification | Lexicographic sorting grouping across slots instead of by element, where the tie-break is not really a tie |
| Key a new rule on the new shape so everything else takes the same path as before | Existing designs moving |
| Choose a carrier value that is impossible for every design predating the change | The guard perturbing designs it has nothing to do with |
| Make a relative fallback reproduce the old output byte for byte for producers you did not convert | Unconverted producers changing output |
| Over-approximate a divergence between a pre-resolve and a post-resolve computation with a sidecar flag so both sides derive from one source | The two phases computing different answers |
| Read the trailer chain — what the pipeline writes and what the staged reader reads — before choosing between a sidecar and a derivation; a format bump is the fix's cost, not a reason to build an alternative | A criterion for choosing an item being mistaken for a constraint on its fix |
| Answer a determinism golden that goes red on a new non-deterministic field by an explicit declaration — isolate it or make it deterministic — and keep an existence assertion beside the exclusion | The field's property never being written down in code, and the rule going vacuous |
| Check whether an existing node already carries the meaning before adding a field to a frozen or hashed type | An avoidable schema-hash flip |
| Follow the infrastructure precedents: climb the system-task ladder from no side effect, to engine state with a side table, to an engine effect with a frozen identifier and a format bump; desugar a side-effecting system function in statement form for single evaluation; keep engine-facing sidecars append-only with defaults; isolate a reused shared buffer by taking and restoring it; emit several items from one parse function through a pending queue drained at the top of the collection loop; save, restore and clear persistent side maps, because scope restore does not reach them | Each item being a measured source of artifact or pollution defects |

## 10. Working rules

### 10.1 Files, modules and frozen-type placement

| Rule | Prevents |
|---|---|
| Keep a source file under about a thousand lines and split on approach: submodules with a prelude re-export, crate-visible items, and re-export from the crate root. Types stay at the crate root so child modules keep access to private fields | A file growing past the point where a reviewer can hold it, and a split that has to fight visibility |
| Keep a single large function and a single trait implementation whole; those are the documented exceptions to the size rule | A split that breaks a trait implementation into pieces no reader can follow |
| Never move a schema-hashed type between modules: the canonical key embeds the module path, so a move flips the hash and invalidates every artifact on disk. Frozen `sim-ir` types and every AST type live at their crate root | An invisible artifact-wide staleness caused by a refactor |
| Keep frozen types verbatim: adding, removing or reordering a field flips the root hash. Do it only deliberately, with the format bump and the golden re-pin in the same commit | An accidental shape change invalidating every artifact |
| Spell `sim-ir` cross-type fields fully qualified as `sim_ir::Foo`; `crates/sim-ir/tests/body_refs.rs` rejects bare references | A bare reference producing a registry key that does not match the canonical one |
| Check parser recursion with `RUST_MIN_STACK=2097152` — the 2 MiB CI test-thread stack the depth guard is tuned to, where a local shell defaults to 8 MiB — after touching the block-body path, and extract an `#[inline(never)]` cold helper or box large locals when the frame grows | Deep nesting overflowing the stack |
| Treat a per-level frame as a budget: box a value in the callee, never in the recursive frame, and use the parser's depth-guard test as the canary | One added value costing bytes per nesting level until the depth guard overflows |
| Separate concurrent sessions with a worktree | A shared checkout moving its head under another session and stranding commits |

### 10.2 Planning and slicing

| Rule | Prevents |
|---|---|
| Ship the subset that provably does not need a prerequisite, and prove the subset rather than asserting it from a naming convention | A change being blocked entirely, or shipping on an unproven convention |
| Make the retirement of a multi-call-site predicate a change of its own, recorded with the measurement and the prescribed deletion | Folding it into the change that invalidated it widening that change's blast radius |
| Pre-verify in simulation the expression a desugar will generate, pin every variant of a context-determined feature before implementing, give a large semantic space its own slice, and record the plan durably | The desugar emitting an expression the simulator handles differently |
| Order work by risk: a pure parser desugar reusing existing AST, then routing to an existing mechanism, then composing single-property primitives, then new infrastructure | The riskiest option being chosen first |
| Draw slice boundaries where the oracle is, and measure the corpus before planning the order | A conceptually clean decomposition producing a gate that cannot run a single corpus design |
| Reproduce an incoming report and then re-find the cause | The reported diagnosis naming a feature that already works |
| Price a loud-to-correct item in two-oracle cells per edit site before picking it out of a row that lists several | Several edit sites buying one cell while a neighbour buys many for one |
| Read a dependency running the wrong way as the signal that the rules belong lower, not that a twin may approximate | A twin being allowed to diverge for a build-graph reason |
| Do not build a third executor as a substitute; the only permitted separations are role and build | A third implementation being a third spelling of the semantics |
| Keep the reference interpreter out of performance optimisation; when the profile points at it, the answer is that the design should not be running there | Every specialisation becoming a second spelling of the rule |
| Read "not a product surface" as a statement about the selection flag, not about the function | Live code being treated as dead |
| Write an option as an enumeration, not a boolean, when the question is which input to pass rather than on or off | A third policy being added without the callers reconsidering |
| Distinguish "the default is the right shape" from "the wrong shape is unrepresentable", and claim the second only when the type makes the wrong state impossible | A paragraph arguing that rules must be types while the wrong form still compiles |
| Price a name-keyed rewrite by asking where the key is constructed; the absence of a funnel is the estimate | "Small and additive" turning out to be a prerequisite |

### 10.3 Comments, documents and queues

| Rule | Prevents |
|---|---|
| Treat the document that enumerates a parallel-table set as part of the code and update it in the same edit | The enumeration falling a table behind on the day it is written |
| Re-prove in this file any property a comment asserts | A neighbour's property copied into a comment being false the moment it is copied |
| Diff the two bodies as part of writing a comment that says "twin of" | The comment being false |
| Name what actually holds an invariant and say so in the documentation | The next change leaning on the same false argument |
| Re-check before committing whether another change in the same slice invalidated the premise you wrote into a comment | The stated reason being false and carrying your signature |
| Say what a temporary workaround is for when you use one, and remove it when you fix the root | The workaround becoming the next change's defect |
| Delete a caller-less macro or helper but leave the reason for its deletion in place | The next reader believing the gate still has teeth |
| Read what another copy of the same rule already says before writing a comment about it | One file documenting a backstop as required while another records that removing it is byte-identical |
| Write what you measured to reach a "cannot" verdict, not why it cannot be done | "Cannot" in a comment being a claim that the next change refutes |
| Check against the code any comment asserting that duplication was avoided | Hand-matched arms drifting while the comment says they cannot |
| Re-read the documents written before a review when the review changes the design; the high-risk sentences are the ones naming a file, a function or a count | A paragraph shipping false in both its place and its count, with every gate green, because no test reads prose |
| Name an implementation with the reason it is there when it must be named | A later move reading as a detail instead of a contradiction |
| Write a queue edit, including a deferral, into the canonical queue first and mirror it afterwards, checking the mirrors at every close | Mirrors carrying rows their own declared source does not have |
| Say what a constant is for and point at the canonical site instead of restating it in prose, and grep the number itself when a bump ships | Restated constants decaying silently with every gate green |

### 10.4 Tooling and machine safety

| Rule | Prevents |
|---|---|
| Read twenty lines either side of an insertion point after a scripted splice | A new item inserted before a documentation block stealing that block |
| Assert the anchor exists before a scripted splice and verify the result | A replacement that matched nothing reporting failure as success |
| Require a full diff and an oracle re-verification for any agent given write access | A write tool replacing a whole file, with a small count change as the only symptom |
| Make each edit an independent write, grep to confirm it landed, then write the comment or the report; a multi-edit script must report failures and continue rather than aborting | An abort in the middle losing every earlier write while the comment claims a state that does not exist |
| Re-establish the state after any tool result that did not visibly complete: `git status --short` and `git diff --stat` for an edit, a re-read for a write, a re-run for a command | A truncated result being equally consistent with an execution failure, so later steps reason about a tree that does not exist |
| Rebuild after changing build configuration, because product and oracle configurations share a target path | A fast "finished" line meaning the previous configuration's binary is still there |
| Never run two full suites concurrently | Hard-coded temporary paths making the two processes write the same files, which reads as a product flake |
| Verify a feature flag with `cargo tree -p <crate> --no-default-features -e features`, reference dependent crates with `default-features = false`, and rebuild after the change | Feature unification silently re-enabling the default and a green build proving nothing |
| Redirect a gate's output to a file and capture its exit code separately (`cargo … > /tmp/x.log 2>&1; T=$?`); after an unavoidable pipe, read zsh's `${pipestatus[1]}` or bash's `${PIPESTATUS[0]}`, because `$?` reports the last command's status | A compile failure reading as green |
| Take a hang out of the battery and measure it once by hand | A terminate-after not being a cheap kill, because the child keeps writing to the pipe |
| Do not add a pre-emptive build before the test runner; the runner builds anyway | The build pass running twice |
| Wait on a process identifier (`while kill -0 $PID; do sleep …; done`), never on a `pgrep -f` pattern that matches the waiting shell's own command line | A permanent deadlock that looks exactly like a slow compile |
| Run a background wait's predicate once by hand before arming it, prefer a condition read from an artifact you have inspected, and kill the wait when the answer arrives another way | A wait on a string the producer never writes running until someone notices |
| Never send an uncatchable kill to a test runner mid-build | The lock surviving and the next run blocking at no CPU |
| Grep every site that decides a default value before flipping it | Only part of the surface moving while the result is called a whole-suite run |
| Write a log line from a parallel process with a single write, and make the aggregation declare contamination when it meets a value outside the known set | Interleaved fragments tearing rows and inflating counts |
| Treat a revert as an edit: specify the deleted range by line, grep the deleted symbols for surviving references, and check that adjacent tests, documents and helpers were not deleted with it | A regression test being deleted invisibly, because the suite is green either way |
| Before a new ERROR from a lint-class rule, run one shape per file through the second tool and put the resulting table in the module doc; a shape the second tool accepts is a warning at most, and a shape nobody ran is recorded "unmeasured", never "accepts" | Six in-tree fixtures failing under a rule written from the LRM sentence (§4.5.472): whole-vs-partial writes, `always_latch`, `input` vs `inout` actuals were all separate cells |

---

The measurements behind these rules — the designs that were run, the numbers that came back, and
which rule each incident bought — are in [history/lessons.md](history/lessons.md).
