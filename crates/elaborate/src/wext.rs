//! §5.2 Q-WREXT — the write-extent view: which writer writes which statically decided
//! bits of which net, read once over the finished IR. One record (ER §5.1); its home
//! test is `crates/elaborate/tests/wext_home.rs`, its pins `crates/cli/tests/wext_view.rs`.
//!
//! WHAT IT READS. Every chunk of every lvalue the finished IR stores through, in this
//! order: the continuous rows (`cont_assigns`, one append point `push_cont_assign`, the
//! row's kind from `ca_idents`), the process bodies (`processes[p].body`, every block in
//! body order, its statements in block order, the process kind from `proc_idents`, one
//! process append point `push_process`, one statement append point `push_stmt`), then the
//! frame bodies (`funcs[f]`: the blocks of the global `blocks` arena reached from its
//! `entry` through the terminators, in ascending block id). A statement writes through
//! its lvalue when it is one of `stmt_lvalue`'s four: `BlockingAssign`,
//! `NonblockingAssign`, `Force`, `Release` (`Form` keeps which: a reader decides whether
//! a force or a release is a writer, the view does not).
//!
//! THE EXTENT of one chunk, decided here and nowhere else:
//! - `Poison`: the chunk's net id is not a net (`POISON_NET`, a hierarchical-write
//!   sentinel never patched). Only an IR that latched `had_error` keeps one; the finished
//!   IR of a clean run has none (premise judge 1 K7: 0 on 14 real designs).
//! - `Word`: an unpacked-array element (`word` set). Not compared bit-wise here (§5.2
//!   Q-ARRWORDMD owns element extents).
//! - `Bits { lo, hi }`: `[lo, hi)` as `sim_ir::const_chunk_written_bits` answers — the one
//!   home of "which bits a constant chunk writes" (called, not copied: no arithmetic of
//!   the view's own). The net width is the kernel's `width.max(1)`, as
//!   `check_whole_net_multidriver` passes it.
//!   `by: Kernel` names that home ; `by: Geometry` the geometry constant below (C2), for
//!   a packed-array element or an array-of-struct member with constant indices.
//! - `Undecided`: neither decides. That is an offset the kernel does not read — a
//!   run-time index, a constant tree with a multiply, a cast, a conditional or a call
//!   (§5.2 Q-MDCONSTOFF's list: a packed element's `flatten_word` stride product and its
//!   bounds guard are such trees) that the geometry constant did not record either (a
//!   genvar EXPRESSION index such as `2*g+1`: reach, not soundness, premise judge 1 N5)
//!   — or a chunk that writes no bit at all (wholly out of range: the kernel answers
//!   `None` for both, and so does the view) — or any chunk with an offset or a width
//!   expression in an elaboration that held a provisional constant (THE STAND-DOWN,
//!   below). `Undecided` is not a fact: it never stands for "the whole net".
//!
//! THE WRITER is recorded as the IR has it (premise judge 1 K5): a static task inlined
//! into its caller writes through statements of the CALLER's process, so its writes are
//! that process's (`Writer::Proc`) ; a function, or a task lowered to a frame, writes in
//! its frame body (`Writer::Body`), and the view does not attribute a frame body to the
//! processes that call it. A declaration initializer arrives as the synthesized
//! `var_init` process, a `wire` initializer as a `net_init` row, an instance port hookup
//! as a `port` row, a gate primitive as an `assign` row: the kind string is the one the
//! producer pushed, unedited.
//!
//! OUT — channels that store into a net without an lvalue chunk the view reads (premise
//! judge 1 K1, measured): a frame call's copy-out `out_binds` (q8_task_auto_2comb: no
//! write on the automatic task's actuals), a system task's output arguments (`$fread`,
//! `$sscanf`, `$value$plusargs`: plain expression arguments), the clocking outputs
//! (`clocking_outputs`), an `inout` actual (`inout_actual_exprs`) and the task enables
//! kept for a process (`task_calls_proc`). A reader that needs them reads them beside
//! the view (`cont_array.rs`'s `other_writers` does today).
//!
//! NOT HERE: no target set (which nets a rule looks at: declaration facts, §5.2
//! Q-MDRVMIX's re-plan; no `intro_kind` read), no pair test (which two writes overlap is
//! the reader's), no reader in production. The three checkers that answer parts of this
//! today (`multidriver.rs` rules A-D over the AST, `netdecl.rs`
//! `check_whole_net_multidriver` over continuous rows, `cont_array.rs` `other_writers`)
//! are not re-pointed: their re-point is filed with the readers (§5.2 Q-WREXT). The
//! record is not serialized (no IR field, no `.vu` / `.velab` byte, no `SchemaHash`).
//!
//! WHEN IT IS BUILT. Once per elaboration, in the end phase (`driver.rs`,
//! `build_write_view` right before `check_whole_net_multidriver`): every deferred
//! hierarchical write is patched by then and no array the view reads is appended after it
//! (the sole-writer checks, `check_readmem_assoc_waits`, then api.rs's
//! `assert_block_local_inits_drained`, `finish_assert_item_gate` and `edge_backstop` only
//! read them; `finish` moves them), so the writes, their forms and their kernel extents
//! are read from the finished IR. Three facts are not: the geometry constant, the
//! declaration fact and the `via` intervals (C2, below) are taken during lowering, each
//! when the lowering decides it (the geometry constant also in the end phase: a deferred
//! hierarchical element write's chunk is built through `flatten_word_eids` after the
//! `$bits` patch, and a hierarchical read's before it, which is not a view write), and
//! the first of them can be stale (THE GEOMETRY CONSTANT's REVIEW-ONLY line); a kernel
//! extent reads the finished IR's offset constant, which can itself be a lowering-time
//! fold of a provisional value (the same line). Both cases stand down (THE STAND-DOWN).
//! It goes to `Sidecars::write_view` (the test and dump entry) and nothing reads it in a
//! `vita` run.
//! Built eagerly because it is cheap (Q-WREXT build r1, the view's instructions over an
//! elaboration alone, same release binary, median of 3: +0.12 % y_wext_procs, +0.27 %
//! y_wext_mix, +0.11 % y_wext_port, +0.04 % el2w; W/wrext/build/cost/probe/VIEW_COST.txt):
//! the row that reads it adds only its reader's cost.
//!
//! C2 (§5.2 Q-WREXT C2) adds three records the lowering writes while it decides them,
//! each through one writer fn in this file (`wext_rec`, `WextRecords`), none serialized:
//!
//! THE GEOMETRY CONSTANT (plan L2). A packed-array element, an array-of-struct member and
//! a part of either lower their offset through `flatten_word` / `flatten_word_eids`
//! (`array_geom.rs`): a row-major `coord * stride` sum under a per-dimension bounds guard
//! (`cond ? flat : GEOM_OOB`), a tree the kernel does not read. When every SEALED index is
//! one numeric constant leaf without x / z (`const_index_value`) whose value is in
//! `[0, 2^31)`, `GeomAcc` follows the same arithmetic the lowering emits — the
//! coordinate (`idx - lo`, or `hi - idx` on an ascending dimension), the stride, the guard
//! (`lo <= idx <= hi` on every dimension) — and `record_geom` keeps, keyed by the offset
//! ExprId the twin returns, the value the engine computes for that tree: the flat bit
//! offset, or `GEOM_OOB` when a guard fails. `carry_geom` keeps it through the one
//! composition a write takes on top (`packed_lval.rs`'s `base + l`, the member and the
//! nested part-select of an element). Only a twin call with two or more dimensions
//! records (one dimension emits no guard and no multiply: the kernel reads it) and an
//! array-WORD index never does (a `word` is not a bit extent). A chunk the kernel declines
//! and whose offset ExprId holds a record gets `Bits { by: Geometry }` from
//! `sim_ir::chunk_written_bits_at` (the kernel's width and `sel_written_bits`, so a
//! `GEOM_OOB` offset writes no bit: `Undecided`). Not recorded, so still `Undecided`: a
//! run-time index, a SEALED index the seal wrapped (a narrow signed or an unsigned narrow
//! index becomes a concatenation or an extension), a negative or `>= 2^31` index, an
//! ascending dimension with a negative low bound (refused by the lowering), and a genvar
//! EXPRESSION index (`2*g+1` lowers to a constant tree, not a leaf: premise judge 1 N5 —
//! this home folds leaves only, because a leaf is the one value the engine reads as it is;
//! folding the tree would be a second fold of `Mul` beside the kernel's). Equality with the
//! engine: `sim-engine`'s `geometry_constant_is_what_the_stores_write` writes every
//! geometry-decided chunk of a grid through both stores and compares. REVIEW-ONLY: a later
//! in-place edit of a recorded ExprId's tree. One such edit makes the view stale today
//! (batch 5's soundness lens F1, W/batch5b/lens-sound/REPORT.txt): `lower_bits_fold`
//! (`expr_special.rs`) lowers `$bits` of a hierarchical name to a provisional `Const` 32
//! (kept in `undecided_placeholders`), and `resolve_deferred_hier_bits`
//! (`hier_defer/bits.rs`) patches that node in place to the real width in the end phase,
//! AFTER `record_geom` read the 32 at lowering; the lowering-time folds `norm_sub_k` /
//! `norm_k_sub` (`packed.rs`) and `const_bound` (`arrays/pattern.rs`) read the same 32 and
//! fold it into a fresh constant the patch never reaches. So on a design with a
//! hierarchical `$bits` in an index the geometry constant can name bits the finished IR
//! does not write, and a kernel extent names the bits the finished IR writes, which are the
//! folded 32's, not the design's (W/wrext/fix1/cells w01 w03 w06 w07 w08,
//! W/batch5b/lens-sound/cells a1-a5; the store itself is §2 bits-hier-placeholder). Since
//! §5.2 Q-WEXTPOS that elaboration stands down (THE STAND-DOWN, below): its select
//! extents are `Undecided`, so neither stale value reaches a `W` line. The other in-place
//! edits are not stale-able here: `param_query.rs`'s `patch_expr_param_const*` and
//! `hier_defer/read.rs` write deferred READ placeholders — a `Signal` node (`POISON_NET`)
//! for a hierarchical net, parameter or select, a `Call` node (`POISON_FID`,
//! `inline_fn.rs`) for a hierarchical function call; neither is a constant leaf; and
//! `edge_gate.rs` repoints constant EDGES to a fresh id the record does not hold, so the
//! kernel reads the fresh constant.
//!
//! THE STAND-DOWN (§5.2 Q-WEXTPOS, unit C1). One flag, `WextRecords::provisional`: "this
//! elaboration held a provisional constant". It is set by the one registered provisional
//! constant and by any in-place overwrite of a `Const` slot, and nothing clears it:
//! `register_provisional_const` registers the value-guess `Const` its one caller
//! `lower_bits_fold` (`expr_special.rs`) makes for `$bits` of a hierarchical name (in the
//! set `undecided_placeholders` the dead-arm and late-edge decisions read, and in the
//! flag), and the patch funnel `patch_expr` / `patch_expr_with`, through which every
//! in-place write of an `exprs` slot goes (the end phase's resolvers in
//! `hier_defer/bits.rs`, `hier_defer/read.rs` and `param_query.rs`), raises it when the
//! slot it overwrites is a `Const`. A provisional `Const` that is fixed by repointing its
//! parent (`pending_fill_width`'s bare fill, an rhs `resolve_pending_fill_widths` in
//! `hier_defer/write.rs` replaces in the statement or row) sets nothing and is no view
//! fact, because the view reads lvalue chunks only. With the flag set, `chunk_extent`
//! decides no select: after `Poison` and `Word`, every chunk with an offset or a width
//! expression is `Undecided` (a lowering-time fold — `norm_sub_k`, `norm_k_sub`,
//! `const_bound`, the geometry constant — may have read the provisional value, and the
//! finished IR does not say which did); a whole-net chunk (no offset, no width) reads no
//! expression and keeps the kernel's `[0, net_w)` (a declared width from a hierarchical
//! `$bits` is refused by the lowering). The flag is the elaboration's, not the chunk's: a
//! select that never read the provisional value stands down too (reach lost, never a
//! wrong extent). `edge_gate.rs`'s `materialize` hands `&mut self.exprs` to
//! `sim_ir::selfwidth::for_each_constant_edge_mut`, which repoints edge fields and writes
//! no root slot: the one listed in-place hand-off outside the funnel. The funnel
//! `debug_assert!`s that the view is not built yet (in a debug run only, so the narrow
//! gate checks it and a release run does not). The stand-down is deleted when §5.2
//! Q-HIERBITS lands (the registration then has no caller); the funnel stays, the patch
//! census `wext_home.rs` H11 counts. NOT A FACT HERE: the view of an elaboration that
//! latched `had_error`. After an error `placeholder_expr` stands a `Const` 0 in for a
//! refused expression and a refused defparam leaves the parameter's default; neither is
//! registered, and a reader is to return on `had_error` before it reads (§5.2 Q-MDRVMIX's
//! re-entry (a); the row is parked and nothing reads the view).
//!
//! THE DECLARATION FACT (PLAN2 addendum A1). `decl(net)` is `Some(DeclFact { site, kind,
//! dir, type_param })` only for a net created by one of three declaration lowerings, each
//! passing what it knows: `elaborate_netvar_decl_inner`'s integral `add_net` with the
//! caller's `DeclScope` (instance.rs module body -> `Module`, generate.rs -> `Generate`;
//! an interface body, a package and a hoisted block local write no fact), `elaborate_ports`
//! for a module's ANSI header (`AnsiPort`; an interface's ANSI header writes none) and
//! `elaborate_instance`'s non-ANSI `PortDecl` loop (`NonAnsiPort`). `kind` is the
//! declared keyword after `shape_kind` (not `ir::NetKind`), `dir` the net's direction,
//! `type_param` whether a type parameter types it. `record_decl_fact` writes only when
//! the `add_net` just before it pushed a net (a refused duplicate keeps the first
//! declaration's net and writes nothing). Every other net (task / function formals and
//! locals, temporaries, loop variables, implicit nets, string / handle declarations) has
//! `None`. No `intro_kind` and no net name is read. Q-NETSHAPE: the declared shape of a
//! net is that open record row's; this fact is the part a target set reads today, kept
//! here until that row owns it.
//!
//! HOW A STATEMENT CAME TO SIT WHERE IT IS (PLAN2 addendum A2). `record_via` keeps a
//! statement-id interval `[start, stmts.len())` (ids are appended by the one funnel
//! `push_stmt`, in order) for two lowerings: `inline_task` from its INLINE decision to its
//! return (`TaskInline`: copy-in, the body, copy-out and every temporary in between; the
//! method statements, the deferred hierarchical enable and the frame call it dispatches
//! first are not inlines) and the `DeferredAssert` arm around its action arms
//! (`DeferAction`). A procedural write's `via` is `DeferAction` when its statement lies
//! in a `DeferAction` interval, else `TaskInline` when it lies in a `TaskInline` one,
//! else `Direct`; a continuous row has none. The interval says when the statement was
//! pushed, not which body it went to: a statement pushed while a task inlines but placed
//! in another body (a hoisted initializer) carries `TaskInline` there too (PLAN2 R8).
//! ENG-2's `rs.stmts` (`read_set.rs`) keeps a per-statement mark for the read set
//! (another purpose: its task mark covers the body, not the copy-in / copy-out, and its
//! action mark both assertion kinds); not read here.
//!
//! EACH WRITE'S OWN POSITION (§5.2 Q-WEXTPOS, unit C2: continuous rows ; fix F1: the rule
//! and V). `Write::at` is a raw source span (`ast::Span`, a byte range of the preprocessed
//! text; no file, line or column is kept here), or `None`. THE RULE (R-AT): `Some(s)` only
//! when `s` is one of
//! - OWN: the text of the parsed node that IS this write — an `assign`'s target, a gate's
//!   output terminal, a port connection (a named one from its `.`, a positional one its
//!   expression, on a plain instance and on each element of an instance array alike), a
//!   procedural assignment, force or release statement ;
//! - FROM: one written construct vita splits into several rows, every row that construct's
//!   own text — the element rows of a whole-array `assign` or of an array port, the element
//!   instances of an instance array (named or positional: its element rows share their one
//!   connection's text), the copies of a generate body, the instances of one module, the
//!   copies of a `do .. while` body, the elements of an array pattern ; so a generate body
//!   elaborated N times, a module instantiated N times and a `do .. while` body give N
//!   writes with one `at` ;
//! - HOLDER (procedural rows only): the innermost PARSED statement whose lowering pushed a
//!   store vita built — a hoisted temporary, a loop's control variable, an
//!   intra-assignment capture, a case scrutinee, an inlined task's copy-in, copy-out and
//!   local initializer (the CALL statement), a block-local initializer (its `begin` block,
//!   in a frame body through `emit_frame_local_inits` and in a module process body through
//!   `emit_per_entry_block_inits` alike), a guarded wait's snapshot (the user's `@(.. iff
//!   ..)` statement: fix S) — always an `ast::Stmt` a parsed body holds, never one vita
//!   built (THE RULE OF ENTRY, C3 below).
//!
//! Otherwise `None`, and NEVER an item that holds several writes as peers (an instance item
//! for its connections, a gate list, an `assign` list, a declaration list, a `primitive`, a
//! module name) or a node from elsewhere (another call site, a child module's name). The
//! view does not record which of the three a row is: a reader that needs the difference
//! reads `via` and the target's name ; the source map is the reader's (two rows from one
//! macro expansion have two offsets and one `file:line:col`, the invocation's).
//! A continuous row takes the span its producer passes to `push_cont_assign` as `at`,
//! recorded by `record_row_at` into `row_at` (lockstep with `cont_assigns`, checked at the
//! build in a debug run). The positive set is the two kinds the planned reader prints
//! (§5.2 Q-MDRVMIX, parked): an `assign` row is its own assignment's target `lv.span()`
//! (`netdecl.rs` `elaborate_cont_assign`, one row per assignment of a list or per gate
//! terminal; `cont_array.rs` `cont_assign_whole_array`, every element row of one whole-array
//! `assign` its one target), a `port` row the port connection in the parent's instantiation
//! (`conn.span`, the `ConnSite` `wire_ports` picks: a positional connection's expression, a
//! named one's `PortConn` span — the parser's from the `.`, or the one `instance_array.rs`
//! builds per element, which carries the parsed connection's span, the expression's for a
//! positional list) and `None` for a `.*` connection (its expression is synthesized and
//! stamped with the child module's name, which is not the write's text). The kinds no
//! reader prints carry `None`: `net_init` (`var_init.rs`), the level-select net
//! (`events.rs`) and the time-0 pulse (`const_level_header.rs`). A procedural write's `at`
//! is a statement's (C3, below). `at` is not `ca_idents`' span for an `assign` row
//! (`push_cont_assign`'s `span`, the profile identity: the statement), which is kept as it
//! was ; a `port` row's identity is the same connection span. The dump prints one last
//! field per `W` line, ` at=<lo>` (the decimal byte offset) or ` at=-`. REVIEW-ONLY: that
//! the spelled argument is the right node's on a form with no pin (`wext_view.rs` pins
//! `lc(src, lo)` per form; `wext_home.rs` H13 counts each caller's spelling, H15 each
//! construction of a connection or a statement node in this crate and its span field) ;
//! that the top node of a parsed connection expression, gate terminal or `assign` target
//! carries its own text's span (sampled by the pins, censused by the premise judge: not a
//! rule a test holds).
//!
//! A STATEMENT'S OWN POSITION (§5.2 Q-WEXTPOS, unit C3 ; fix S: THE RULE OF ENTRY). A
//! statement's interval is recorded only by `lower_parsed`, a call a fn makes only on a child
//! of the parsed statement or body it was handed ; every other way into the statement
//! lowering (`stmt_main.rs`, one implementation `lower_stmt_entry` under three names)
//! records nothing. `lower_parsed` reads `start = stmts.len()` at its entry and at its one
//! exit (in `lower_stmt_entry`, its `record` set) calls `record_stmt_at` with the fn's OWN parameter's span `s.span()` (never
//! `cur_span`, a context's: 41 writers), which keeps the interval `[start, stmts.len())` with
//! that span in `stmt_at` — nothing when the statement pushed no IR statement, when the span
//! is empty (`debug_assert!`ed after those two returns), or when THE MASK is up. The plain
//! entry `lower_stmt` lowers a statement vita BUILT under THE MASK, `wext_unlocated`, a depth
//! counter inside which no interval is recorded (a hoisted temporary, a dyn-array or frame
//! local's built initializer, an intra-assignment event's wait, a covergroup's bitmap store,
//! a frame call's unpack, the in-body `iff` guard's wait loop `lower_event_iff_wait` ; 20
//! calls, `wext_home.rs` H14) ; the mask is also around `lower_synth_proc`'s body.
//! `lower_shell` lowers ONE built node with no interval of its own and no mask, so its
//! statement children, all parsed, record themselves: the `always @(.. iff ..)` desugar's
//! `if (guard) <body>` (`desugar_event_iff`), lowered by `lower_proc_block` — its one call.
//! What the owner decided (2026-11-20, as recorded): "a statement elaborate built carries NO
//! position, with no exception argued from 'it pushes no write'". So an elaborate-built
//! statement records no interval of its own, and a store its lowering pushes reads the
//! parsed statement that holds it (HOLDER) or `None` ; no hand class of what a built
//! statement can push is kept (fix F1's MASKED / NOSTORE classes are deleted), and a call
//! nobody classed is the plain entry: a forgotten class LOSES a position, it never invents
//! one. The rule is held by four token rules over this crate in `wext_home.rs` (R1: no fn
//! that builds a statement records or names a recording fn ; R2: a recording fn takes a
//! built statement only from a rewriter or as the one shell ; R3: one `lower_shell` call, a
//! rewriter lowers nothing ; R4: no routine, item or body definition is built here but the
//! listed `ProceduralBlock`s) ; what those cannot see is that test's head note. The built
//! nodes keep their spans: a diagnostic raised inside one is anchored there (`cur_span`), the
//! position record only never reads them. A procedural write takes the span of the INNERMOST
//! recorded interval holding its statement id, among equal intervals the one recorded first
//! (a statement that pushes nothing of its own, a label, has exactly its child's interval,
//! and the child exits first) ; no interval, or a FLOOR, gives `None` ; every chunk of one
//! lvalue the same `at`. Every body ELABORATE synthesizes (`lower_synth_proc`'s callers)
//! gives `None`: the declaration-initializer flush `var_init` (its Block's span is the FIRST
//! initializer's, a context for the others), `sva` including a concurrent assertion's action
//! blocks, `cover`, the clocking commit handler and the module-level assertion wrapper
//! (`lower_user_proc`). THE FLOORS, `wext_body_enter` / `wext_body_exit`: each body root
//! (a function's and a task's frame body, a class method's body) saves the mask depth, sets
//! it to 0 and at its exit restores it and records `(start, end, None)` over the body's ids,
//! so a prologue store (a local's initializer) is `None` and a store the caller pushes after
//! the body is the caller's statement, even when the body is lowered inside an open
//! statement (a package function's frame on its first call). The process root
//! (`lower_proc_block`, `wext_body_enter_keep`) records its floor but KEEPS the depth: it is
//! reached under `lower_synth_proc`'s mask (premise judge round 1, line 1). The callee
//! entries `inline_task_body` and `lower_frame_body_stmt` (`wext_located`) save, set 0 and
//! restore the depth with no floor, so a task inlined into a synthesized body (an
//! assertion's action calling a static task) records its own statements, and a copy-in or
//! copy-out its call pushes is the call statement ; the bodies they and the body roots
//! lower come from the routine tables, which hold parsed definitions only (R4).
//! `build_write_view` `debug_assert!`s the mask depth and the open bodies are 0.
//! V (fix F1): each body root also takes the span of the ITEM its body belongs to — the
//! process block (`lower_proc_block`), the function or task (the frame bodies), the class
//! method — never the body's own (an implicit body block starts at its first statement),
//! keeps it as the open item while it is open and restores the enclosing root's at its exit
//! (a frame body lowered inside an open process statement nests) ; the callee entries keep
//! the caller's. `note_stmt_at` records nothing for a statement whose span starts at the open
//! item's first byte: no user-written statement of a process or a routine starts at its
//! item's keyword, while a body the PARSER builds stamps one span on its item and on every
//! statement — a UDP's `initial` and `always` (hdl-parser `udp.rs`, `udp_table.rs`: the
//! whole `primitive`), the enum `.name()` function `$enum_name$<T>` (`enums.rs`: the FIRST
//! `.name()` call's path) and the two `type_param_shape` guards (no store) — so their stores
//! take the floor's `None` where they took a position that was not theirs. V is a value
//! rule, not provenance: which code built a body is not a fact here (the structural row
//! of parser-made names and bodies owns it ; V is what it replaces). Its one residue on user
//! text: a deferred assertion ITEM's root statement (an unlabeled `DeferredAssert` or a
//! labeled block, hdl-parser `item_dispatch.rs`) starts at the item's first byte, so a store
//! whose innermost recorded statement is that root reads `None` (HOLDER lost, the safe
//! direction ; the action statements inside it are their own ; no producing form found).
//! REVIEW-ONLY (agy read of batch 6, NB3): V compares raw offsets with no unit id, which is
//! sound in one expanded buffer (every file of one `vita` / `vcmp` run) ; the staged `velab
//! -L` path splices units whose spans each start at byte 0 (`cli/src/staged.rs`, which drops
//! the source maps for that reason), so an inlined package routine's statement at offset N
//! of one unit, called from a process item starting at N in another, is declined by V and
//! reads the call statement's position (no cell measured ; no pin).
//! V also declines the `always @(.. iff ..)` shell (its span is the process item's, `p.span`),
//! which `lower_shell` already records nothing for: the two agree there, and V stays the
//! stand-in for the parser's provenance until the structural row gives each body its
//! builder.
//! THE RESOLUTION, one reverse pass (`stmt_at_of`; the plan's sort by `(start, Reverse(end),
//! Reverse(record index))` is its unit tests' reference): every recorder records at its OWN
//! exit over `[stmts.len() at entry, stmts.len() at exit)`. It needs two facts. (1)
//! Statement ids are only appended: `wext_home.rs` H1 (`push_stmt` the one arena appender)
//! and H1b (the arena's `&mut` writers patch in place ; `retain` and `splice` act on a
//! block's id list). (2) Every recorder is an entry / exit pair on one call stack: H7's
//! equal body enter / exit count per (file, fn), and `push_interval`'s `debug_assert!` in a
//! debug run. So of two recorded intervals either one holds no id of the other or the one
//! recorded first lies inside the other (its recorder ran inside the other's) — an equal
//! pair included. Painting the intervals last-recorded first therefore lets every inner
//! interval overwrite its container, and among equal ones the first-recorded paints last.
//! `push_interval` `debug_assert!`s that order against the interval recorded before.
//! REVIEW-ONLY for C3: that a `lower_parsed` call's argument really is a child of what its
//! fn was handed (read at all 28 by the plan and the premise judge of fix S ; not a token
//! rule) ; that a masked built statement's store takes the parsed statement around it
//! (HOLDER above) — precision lost, the text holding the write, never another construct's ; that a `TaskInline` write's `at` is the callee's statement or the
//! call (the view cannot tell which) ; the call site of an inlined callee's statement (not
//! recorded) ; a statement from a macro body or an include (the record holds the raw offset
//! of the preprocessed text) ; the spans of masked statements (not recorded, never
//! measured) ; that no other body the parser builds carries a span that is not its own and
//! is not its item's (V cannot see it ; `wext_home.rs` H15p counts every statement
//! construction of the parser with its span field).

use super::*;

/// Who writes: the IR's own identity of the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Writer {
    /// Continuous row `row` of `cont_assigns`; `kind` is `ca_idents[row].kind`.
    Cont { row: u32, kind: &'static str },
    /// A statement in process `pid`'s body; `kind` is `proc_idents[pid].kind`.
    Proc { pid: u32, kind: &'static str },
    /// A statement in frame body `funcs[func]`; `task` is its `is_task`.
    Body { func: u32, task: bool },
}

/// How the store writes: a continuous row, or the statement that carries the lvalue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Form {
    Continuous,
    Blocking,
    NonBlocking,
    Force,
    Release,
}

/// Which home decided a `Bits` extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum By {
    /// `sim_ir::const_chunk_written_bits` read the offset tree.
    Kernel,
    /// The geometry constant recorded for the offset ExprId (module head, C2).
    Geometry,
}

/// Which bits of the net one chunk writes, as far as the finished IR decides it statically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Extent {
    /// Storage bits `[lo, hi)`, `lo < hi`.
    Bits { lo: u32, hi: u32, by: By },
    /// An unpacked-array element: not compared bit-wise.
    Word,
    /// The kernel does not decide it (or it writes no bit).
    Undecided,
    /// The net id is not a net (an IR that latched `had_error`).
    Poison,
}

/// How a procedural statement came to sit in its body (module head, A2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Via {
    /// Lowered from the body's own text.
    Direct,
    /// Pushed while a static task call was inlined (copy-in, body, copy-out).
    TaskInline,
    /// Pushed while a deferred assertion's action arms were lowered.
    DeferAction,
}

/// Where a declaration fact was recorded (module head, A1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeclSite {
    /// A module-body data declaration.
    Module,
    /// A generate-scope data declaration.
    Generate,
    /// A module's ANSI header port.
    AnsiPort,
    /// A module's non-ANSI `PortDecl` with no body declaration of its name.
    NonAnsiPort,
}

/// The scope a data declaration is lowered in, as `elaborate_netvar_decl`'s caller knows
/// it (and `elaborate_ports`': `Module` or `Iface`). Only `Module` and `Generate` record a
/// fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeclScope {
    Module,
    Generate,
    Iface,
    Package,
    BlockLocal,
}

impl DeclScope {
    /// The fact's site for a body declaration in this scope.
    pub(crate) fn body_site(self) -> Option<DeclSite> {
        match self {
            DeclScope::Module => Some(DeclSite::Module),
            DeclScope::Generate => Some(DeclSite::Generate),
            DeclScope::Iface | DeclScope::Package | DeclScope::BlockLocal => None,
        }
    }
}

/// What a declaration lowering knew about the net it created (module head, A1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeclFact {
    pub site: DeclSite,
    /// The declared keyword after `shape_kind` (an omitted port kind is `Wire`).
    pub kind: ast::NetVarKind,
    pub dir: ir::PortDir,
    /// A type parameter types the declaration (`shape_param` is set).
    pub type_param: bool,
}

/// The value the lowering emits for a failed bounds guard (`flatten_word`'s twins): far
/// above any net, so a store at it writes nothing.
pub(crate) const GEOM_OOB: u32 = 0x8000_0000;

/// The records the lowering writes for the view (module head, C2), each only through its
/// writer fn below. Taken by `build_write_view`.
#[derive(Debug, Default)]
pub(crate) struct WextRecords {
    /// Offset ExprId -> the value the engine computes for it (`record_geom`, `carry_geom`).
    geom: std::collections::HashMap<u32, u32>,
    /// Per net id (`record_decl_fact`).
    decls: Vec<Option<DeclFact>>,
    /// `(via, start, end)` statement-id intervals (`record_via`).
    vias: Vec<(Via, u32, u32)>,
    /// Per continuous row, its write's own position (`record_row_at`; lockstep with
    /// `cont_assigns`).
    row_at: Vec<Option<ast::Span>>,
    /// `(start, end, at)` statement-id intervals in record order (C3): a statement's own
    /// span (`record_stmt_at`) or a body root's floor `None` (`wext_body_exit`).
    stmt_at: Vec<(u32, u32, Option<ast::Span>)>,
    /// The mask's depth (`wext_unlocated`): above 0 no statement interval is recorded.
    mask: u32,
    /// Body roots entered and not yet left (`wext_body_enter*` / `wext_body_exit`).
    open_body: u32,
    /// The span of the ITEM whose body root is open innermost (a process block, a
    /// function, a task, a class method), `None` while no body root is open (V,
    /// `note_stmt_at`); each root saves the one before and restores it at its exit.
    body_item: Option<ast::Span>,
    /// The elaboration held a provisional constant (THE STAND-DOWN; `note_provisional`,
    /// `note_patch`).
    provisional: bool,
    /// `build_write_view` has run (the funnel's debug order check).
    view_built: bool,
}

/// A body root's token (C3): the mask depth to restore, the first statement id and the
/// enclosing root's item span (V) to restore.
pub(crate) struct WextBody {
    depth: u32,
    start: usize,
    item: Option<ast::Span>,
}

impl WextRecords {
    /// One statement-id interval (C3). Recorders run at their own exit, so an interval
    /// recorded earlier either holds no id of this one's or lies inside it (the order the
    /// resolution in `build_view` rests on; checked in a debug run).
    fn push_interval(&mut self, start: usize, end: usize, at: Option<ast::Span>) {
        debug_assert!(
            self.stmt_at.last().is_none_or(
                |&(a, b, _)| b as usize <= start || (start <= a as usize && b as usize <= end)
            ),
            "§5.2 Q-WEXTPOS: a statement interval crosses the one recorded before it"
        );
        self.stmt_at.push((start as u32, end as u32, at));
    }

    /// A statement's own interval `[start, end)` with its span: nothing when the span is
    /// empty (the zero-span guard; `record_stmt_at` asserts it in a debug run), and nothing
    /// when the span starts at the first byte of the open body's item (V, module head: a
    /// body the parser builds stamps its item's span on every statement).
    pub(crate) fn note_stmt_at(&mut self, start: usize, end: usize, sp: ast::Span) {
        if sp.lo >= sp.hi {
            return;
        }
        if self.body_item.is_some_and(|item| item.lo == sp.lo) {
            return;
        }
        self.push_interval(start, end, Some(sp));
    }

    /// A provisional `Const` was registered (`register_provisional_const`).
    pub(crate) fn note_provisional(&mut self) {
        self.provisional = true;
    }

    /// An `exprs` slot holding `old` is about to be overwritten in place (the patch
    /// funnel): a `Const` there was provisional.
    pub(crate) fn note_patch(&mut self, old: &ir::Expr) {
        if matches!(old, ir::Expr::Const { .. }) {
            self.provisional = true;
        }
    }
}

/// `flatten_word`'s arithmetic over constant leaves, one index at a time (module head).
pub(crate) struct GeomAcc {
    live: bool,
    seen: bool,
    inb: bool,
    flat: u64,
}

impl GeomAcc {
    /// A twin call over `dims` dimensions; `words` = the array-WORD domain.
    pub(crate) fn new(dims: usize, words: bool) -> Self {
        GeomAcc {
            live: dims >= 2 && !words,
            seen: false,
            inb: true,
            flat: 0,
        }
    }

    /// Index `k`: its sealed leaf's value (`None` = not a constant leaf), the dimension's
    /// `(lo, size)`, its direction and stride.
    pub(crate) fn index(&mut self, v: Option<i64>, lo: i64, size: u32, asc: bool, stride: u64) {
        if !self.live {
            return;
        }
        let v = match v {
            Some(v) if (0..=i64::from(i32::MAX)).contains(&v) => v,
            _ => {
                self.live = false;
                return;
            }
        };
        if (asc && lo < 0) || size == 0 || stride > u64::from(u32::MAX) {
            self.live = false;
            return;
        }
        self.seen = true;
        let hi = lo + i64::from(size) - 1;
        if v < lo || v > hi {
            self.inb = false;
            return;
        }
        let coord = if asc { hi - v } else { v - lo } as u64;
        self.flat = self.flat.saturating_add(coord.saturating_mul(stride));
    }

    /// The value the engine computes for the twin's result, when decided.
    pub(crate) fn value(&self) -> Option<u32> {
        if !self.live || !self.seen {
            return None;
        }
        if !self.inb {
            return Some(GEOM_OOB);
        }
        u32::try_from(self.flat)
            .ok()
            .filter(|&f| f <= i32::MAX as u32)
    }
}

/// One write site: one chunk of one lvalue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Write {
    pub writer: Writer,
    pub form: Form,
    /// The statement id (`None` for a continuous row).
    pub stmt: Option<u32>,
    /// How the statement came to sit there (`None` for a continuous row).
    pub via: Option<Via>,
    /// The chunk's index in its lvalue.
    pub chunk: u32,
    /// The chunk's net id as the IR has it (`Poison` when it is not a net).
    pub net: u32,
    pub extent: Extent,
    /// The write's own source position (module head, C2), raw; `None` when unrecorded.
    pub at: Option<ast::Span>,
}

/// The view: every write site, in the order the module head names, and the declaration
/// facts per net.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriteView {
    pub writes: Vec<Write>,
    decls: Vec<Option<DeclFact>>,
}

impl WriteView {
    /// The declaration fact of net `net` (module head, A1).
    pub fn decl(&self, net: u32) -> Option<DeclFact> {
        self.decls.get(net as usize).copied().flatten()
    }
}

/// The arrays the view reads, borrowed (the elaborator's own, or a hand-built set in
/// this file's unit test).
pub(crate) struct WextSource<'a> {
    pub(crate) nets: &'a [ir::NetVar],
    pub(crate) exprs: &'a [ir::Expr],
    pub(crate) consts: &'a [ir::ConstVal],
    pub(crate) cont_assigns: &'a [ir::ContAssign],
    pub(crate) ca_idents: &'a [ProcIdentRow],
    pub(crate) processes: &'a [ir::Process],
    pub(crate) proc_idents: &'a [ProcIdentRow],
    pub(crate) stmts: &'a [ir::Stmt],
    pub(crate) blocks: &'a [ir::BasicBlock],
    pub(crate) funcs: &'a [ir::FuncDef],
    pub(crate) geom: &'a std::collections::HashMap<u32, u32>,
    pub(crate) vias: &'a [(Via, u32, u32)],
    /// Per continuous row, `Write::at` (`WextRecords::row_at`).
    pub(crate) row_at: &'a [Option<ast::Span>],
    /// Statement-id intervals with their span or a floor's `None`
    /// (`WextRecords::stmt_at`, record order).
    pub(crate) stmt_at: &'a [(u32, u32, Option<ast::Span>)],
    /// THE STAND-DOWN's flag (`WextRecords::provisional`).
    pub(crate) provisional: bool,
}

impl Elaborator<'_> {
    /// The end phase's one build (see the module head): the view over the elaborator's
    /// own arrays, kept for `Sidecars::write_view`.
    pub(crate) fn build_write_view(&mut self) {
        let rec = std::mem::take(&mut self.wext_rec);
        debug_assert_eq!(
            rec.row_at.len(),
            self.cont_assigns.len(),
            "§5.2 Q-WEXTPOS: row_at must stay lockstep with cont_assigns"
        );
        debug_assert_eq!(rec.mask, 0, "§5.2 Q-WEXTPOS: the mask is up at the view");
        debug_assert_eq!(
            rec.open_body, 0,
            "§5.2 Q-WEXTPOS: a body root is still open at the view"
        );
        debug_assert!(
            rec.body_item.is_none(),
            "§5.2 Q-WEXTPOS: a body item is still set at the view"
        );
        let mut view = build_view(&WextSource {
            nets: &self.nets,
            exprs: &self.exprs,
            consts: &self.consts,
            cont_assigns: &self.cont_assigns,
            ca_idents: &self.ca_idents,
            processes: &self.processes,
            proc_idents: &self.proc_idents,
            stmts: &self.stmts,
            blocks: &self.func_blocks,
            funcs: &self.funcs,
            geom: &rec.geom,
            vias: &rec.vias,
            row_at: &rec.row_at,
            stmt_at: &rec.stmt_at,
            provisional: rec.provisional,
        });
        view.decls = rec.decls;
        self.wext = view;
        self.wext_rec.view_built = true;
    }

    /// The one registration of a provisional `Const` (THE STAND-DOWN): `eid` is a value
    /// guess an end-phase patch replaces in place. It joins `undecided_placeholders` (a
    /// width or count over it is undecided until the patch) and raises the flag.
    pub(crate) fn register_provisional_const(&mut self, eid: u32) {
        self.undecided_placeholders.insert(eid);
        self.wext_rec.note_provisional();
    }

    /// The patch funnel, value form: overwrite `exprs[eid]` in place (an index write: a
    /// slot past the arena panics, as the index writes it replaces did), raising the flag
    /// when the slot held a `Const` (THE STAND-DOWN).
    pub(crate) fn patch_expr(&mut self, eid: u32, e: ir::Expr) {
        debug_assert!(
            !self.wext_rec.view_built,
            "§5.2 Q-WEXTPOS: an exprs slot patched after the write view was built"
        );
        let slot = &mut self.exprs[eid as usize];
        self.wext_rec.note_patch(slot);
        *slot = e;
    }

    /// The patch funnel, edit form: run the site's own edit on `exprs[eid]` when the slot
    /// exists (as the `get_mut` writes it replaces did), raising the flag first when the
    /// slot holds a `Const` (THE STAND-DOWN).
    pub(crate) fn patch_expr_with(&mut self, eid: u32, edit: impl FnOnce(&mut ir::Expr)) {
        debug_assert!(
            !self.wext_rec.view_built,
            "§5.2 Q-WEXTPOS: an exprs slot patched after the write view was built"
        );
        if let Some(slot) = self.exprs.get_mut(eid as usize) {
            self.wext_rec.note_patch(slot);
            edit(slot);
        }
    }

    /// The geometry constant's writer (module head): `eid` is the offset ExprId a
    /// `flatten_word` twin returns, `acc` its arithmetic over the same indices.
    pub(crate) fn record_geom(&mut self, eid: u32, acc: &GeomAcc) {
        if let Some(v) = acc.value() {
            self.wext_rec.geom.insert(eid, v);
        }
    }

    /// The geometry constant through `eid = base + add` (`add` a constant the lowering
    /// pushed beside it): the engine's 32-bit sum, kept when `base` holds a record.
    pub(crate) fn carry_geom(&mut self, eid: u32, base: u32, add: u32) {
        if let Some(v) = self
            .wext_rec
            .geom
            .get(&base)
            .and_then(|v| v.checked_add(add))
        {
            self.wext_rec.geom.insert(eid, v);
        }
    }

    /// The declaration fact's writer (module head, A1): `net` is the id `add_net` gives
    /// the declaration it was just handed (`nets.len()` before the call); nothing is
    /// written when that call created no net or `site` is `None`.
    pub(crate) fn record_decl_fact(
        &mut self,
        net: u32,
        site: Option<DeclSite>,
        kind: ast::NetVarKind,
        dir: ir::PortDir,
        type_param: bool,
    ) {
        let Some(site) = site else {
            return;
        };
        if net as usize >= self.nets.len() {
            return;
        }
        let decls = &mut self.wext_rec.decls;
        if decls.len() <= net as usize {
            decls.resize(net as usize + 1, None);
        }
        decls[net as usize] = Some(DeclFact {
            site,
            kind,
            dir,
            type_param,
        });
    }

    /// The continuous row's position writer (module head, C2): one entry per row, pushed
    /// by `push_cont_assign` beside the row.
    pub(crate) fn record_row_at(&mut self, at: Option<ast::Span>) {
        self.wext_rec.row_at.push(at);
    }

    /// The statement's position writer (module head, C3), called once, at the one exit of
    /// `lower_stmt_entry` for `lower_parsed` (fix S: the plain entry and the shell never call
    /// it), with the fn's own `s.span()` and `start = stmts.len()` read at its entry:
    /// `[start, stmts.len())` with that span. Nothing when the mask is up (inside a
    /// statement vita built, `wext_unlocated`), when the statement pushed no IR statement,
    /// when the span is empty, or when it starts at the open body item's first byte (V,
    /// `note_stmt_at`).
    pub(crate) fn record_stmt_at(&mut self, sp: ast::Span, start: usize) {
        let end = self.stmts.len();
        if self.wext_rec.mask > 0 || start == end {
            return;
        }
        debug_assert!(
            sp.lo < sp.hi,
            "§5.2 Q-WEXTPOS: an empty span on a statement that pushed IR"
        );
        self.wext_rec.note_stmt_at(start, end, sp);
    }

    /// THE MASK (module head, C3): `f` lowers a statement vita built (the plain entry
    /// `lower_stmt`, fix S) or a synthesized body (`lower_synth_proc`), so no statement
    /// interval inside it is recorded; its stores take the interval around it, or none.
    pub(crate) fn wext_unlocated<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        self.wext_rec.mask += 1;
        let r = f(self);
        self.wext_rec.mask -= 1;
        r
    }

    /// A callee body's entry (module head, C3: `inline_task_body`, `lower_frame_body_stmt`):
    /// the callee's own text is located again under a mask; the depth is restored after.
    pub(crate) fn wext_located<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let depth = std::mem::replace(&mut self.wext_rec.mask, 0);
        let r = f(self);
        self.wext_rec.mask = depth;
        r
    }

    /// A body root (module head, C3: a frame or class-method body's `ProcessBuilder`):
    /// save the mask depth and set it to 0, remember the first statement id ; `item` is the
    /// span of the routine ITEM the body belongs to (V), never the body's own.
    pub(crate) fn wext_body_enter(&mut self, item: ast::Span) -> WextBody {
        self.wext_rec.open_body += 1;
        WextBody {
            depth: std::mem::replace(&mut self.wext_rec.mask, 0),
            start: self.stmts.len(),
            item: self.wext_rec.body_item.replace(item),
        }
    }

    /// The process root (`lower_proc_block`, premise judge line 1): save the mask depth
    /// and KEEP it (a synthesized body is lowered under `lower_synth_proc`'s mask),
    /// remember the first statement id ; `item` is the process block's span (V).
    pub(crate) fn wext_body_enter_keep(&mut self, item: ast::Span) -> WextBody {
        self.wext_rec.open_body += 1;
        WextBody {
            depth: self.wext_rec.mask,
            start: self.stmts.len(),
            item: self.wext_rec.body_item.replace(item),
        }
    }

    /// A body root's exit: restore the mask depth and the enclosing root's item, and record
    /// the FLOOR, `(start, stmts.len(), None)`, when the body pushed any statement.
    pub(crate) fn wext_body_exit(&mut self, tok: WextBody) {
        self.wext_rec.open_body -= 1;
        self.wext_rec.mask = tok.depth;
        self.wext_rec.body_item = tok.item;
        let end = self.stmts.len();
        if tok.start < end {
            self.wext_rec.push_interval(tok.start, end, None);
        }
    }

    /// The statement-id interval's writer (module head, A2): `[start, stmts.len())`.
    pub(crate) fn record_via(&mut self, via: Via, start: usize) {
        let end = self.stmts.len();
        if start < end {
            self.wext_rec.vias.push((via, start as u32, end as u32));
        }
    }
}

/// The one builder.
/// H15v (§5.2 Q-WEXTPOS fix F1, P5 (i) and (ii), premise judge L9 ; fix S, U2): what is
/// wrong with ONE instance's located connection spans, one entry per port (a concatenation
/// or an array port wires several rows of one connection) as (port index, connection node,
/// span) — the node is the connection's expression as an address, so two ports reached
/// through ONE connection (a repeated port name in a named list, IEEE 1800-2017 23.2.2.1
/// Example 6, which `wire_ports` resolves to the first connection of that name for both)
/// are one node, not two. (i) two DIFFERENT connection nodes whose spans start at one
/// offset (each connection is its own text ; F1 gave every positional port of an array
/// element the item's span) ; (ii) a connection that starts at the instance item's first
/// byte (the item holds the connections as peers, R-AT's NEVER list). Empty = none.
pub(crate) fn conn_span_violations(
    conns: &[(usize, usize, ast::Span)],
    item: Option<ast::Span>,
) -> Vec<String> {
    let mut v = Vec::new();
    for (n, &(i, ka, a)) in conns.iter().enumerate() {
        for &(j, kb, b) in &conns[n + 1..] {
            if ka != kb && a.lo == b.lo {
                v.push(format!(
                    "H15v (i): ports {i} and {j} share one connection span (lo {}) on two connections",
                    a.lo
                ));
            }
        }
        if item.is_some_and(|it| it.lo == a.lo) {
            v.push(format!(
                "H15v (ii): port {i}'s connection span is its instance item's (lo {})",
                a.lo
            ));
        }
    }
    v
}

/// H15v's assertion site (a debug run only): `wire_ports` calls it once per instance, after
/// the instance's port rows are pushed.
pub(crate) fn check_conn_spans(conns: &[(usize, usize, ast::Span)], item: Option<ast::Span>) {
    if cfg!(debug_assertions) {
        let v = conn_span_violations(conns, item);
        debug_assert!(v.is_empty(), "§5.2 Q-WEXTPOS fix F1: {}", v.join(" ; "));
    }
}

pub(crate) fn build_view(s: &WextSource<'_>) -> WriteView {
    let ctx = ir::selfwidth::ExprCtx {
        exprs: s.exprs,
        consts: s.consts,
        nets: s.nets,
    };
    // `via` per statement id, when any interval was recorded: TaskInline first, then
    // DeferAction over it (the module head's order of precedence).
    let mut via_of: Vec<Via> = Vec::new();
    if !s.vias.is_empty() {
        via_of = vec![Via::Direct; s.stmts.len()];
        for want in [Via::TaskInline, Via::DeferAction] {
            for &(v, a, b) in s.vias.iter().filter(|r| r.0 == want) {
                let b = (b as usize).min(via_of.len());
                for slot in via_of.iter_mut().take(b).skip(a as usize) {
                    *slot = v;
                }
            }
        }
    }
    let via = |sid: u32| via_of.get(sid as usize).copied().unwrap_or(Via::Direct);
    let at_of = stmt_at_of(s.stmt_at, s.stmts.len());
    let at = |sid: u32| at_of.get(sid as usize).copied().flatten();
    let mut writes = Vec::new();
    let mut add =
        |writer: Writer, form: Form, stmt: Option<u32>, at: Option<ast::Span>, lv: &ir::Lvalue| {
            for (k, c) in lv.chunks.iter().enumerate() {
                writes.push(Write {
                    writer,
                    form,
                    stmt,
                    via: stmt.map(via),
                    chunk: k as u32,
                    net: c.net,
                    extent: chunk_extent(ctx, s.geom, s.provisional, c),
                    at,
                });
            }
        };
    for (row, ca) in s.cont_assigns.iter().enumerate() {
        let kind = s.ca_idents.get(row).map_or("", |id| id.kind);
        let w = Writer::Cont {
            row: row as u32,
            kind,
        };
        let at = s.row_at.get(row).copied().flatten();
        add(w, Form::Continuous, None, at, &ca.lhs);
    }
    for (pid, p) in s.processes.iter().enumerate() {
        let kind = s.proc_idents.get(pid).map_or("", |id| id.kind);
        let w = Writer::Proc {
            pid: pid as u32,
            kind,
        };
        for bb in &p.body {
            for &sid in &bb.stmts {
                if let Some((form, lv)) = store_of(s.stmts, sid) {
                    add(w, form, Some(sid), at(sid), lv);
                }
            }
        }
    }
    for (f, fd) in s.funcs.iter().enumerate() {
        let w = Writer::Body {
            func: f as u32,
            task: fd.is_task,
        };
        for b in frame_blocks(s.blocks, fd.entry) {
            for &sid in &s.blocks[b as usize].stmts {
                if let Some((form, lv)) = store_of(s.stmts, sid) {
                    add(w, form, Some(sid), at(sid), lv);
                }
            }
        }
    }
    WriteView {
        writes,
        decls: Vec::new(),
    }
}

/// `at` per statement id (module head, C3): the span of the INNERMOST recorded interval
/// holding it, among equal intervals the one recorded FIRST ; no interval, or a floor
/// (`None`), gives `None`. The record order makes this one reverse pass: every recorder
/// records at its own exit, so an interval inside another (or equal to it and recorded
/// inside its call) is recorded before it, and painting the intervals last-recorded first
/// lets each inner one overwrite its container (the order `push_interval` checks in a
/// debug run).
fn stmt_at_of(stmt_at: &[(u32, u32, Option<ast::Span>)], n: usize) -> Vec<Option<ast::Span>> {
    if stmt_at.is_empty() {
        return Vec::new();
    }
    let mut at_of = vec![None; n];
    for &(a, b, sp) in stmt_at.iter().rev() {
        let b = (b as usize).min(n);
        for slot in at_of.iter_mut().take(b).skip(a as usize) {
            *slot = sp;
        }
    }
    at_of
}

/// The form and lvalue of a storing statement (`stmt_lvalue`'s four), else `None`.
fn store_of(stmts: &[ir::Stmt], sid: u32) -> Option<(Form, &ir::Lvalue)> {
    match stmts.get(sid as usize)? {
        ir::Stmt::BlockingAssign { lhs, .. } => Some((Form::Blocking, lhs)),
        ir::Stmt::NonblockingAssign { lhs, .. } => Some((Form::NonBlocking, lhs)),
        ir::Stmt::Force { lhs, .. } => Some((Form::Force, lhs)),
        ir::Stmt::Release { lhs } => Some((Form::Release, lhs)),
        ir::Stmt::SysTask { .. } | ir::Stmt::Disable { .. } => None,
    }
}

/// The blocks of one frame body: every block of the global arena reached from `entry`
/// through the terminators, ascending.
fn frame_blocks(blocks: &[ir::BasicBlock], entry: u32) -> BTreeSet<u32> {
    let mut seen = BTreeSet::new();
    let mut todo = vec![entry];
    while let Some(b) = todo.pop() {
        let Some(bb) = blocks.get(b as usize) else {
            continue;
        };
        if !seen.insert(b) {
            continue;
        }
        match &bb.term {
            ir::Terminator::Goto { target } => todo.push(*target),
            ir::Terminator::Branch {
                then_bb, else_bb, ..
            } => {
                todo.push(*then_bb);
                todo.push(*else_bb);
            }
            ir::Terminator::Delay { resume, .. } | ir::Terminator::Wait { resume, .. } => {
                todo.push(*resume)
            }
            ir::Terminator::Fork {
                children,
                resume_bb,
                ..
            } => {
                todo.extend(children.iter().copied());
                todo.push(*resume_bb);
            }
            ir::Terminator::Call { ret_bb, .. } => todo.push(*ret_bb),
            ir::Terminator::Return => {}
        }
    }
    seen
}

/// One chunk's extent (the module head's four cases, in that order; the kernel before the
/// geometry constant; THE STAND-DOWN after `Word` when `provisional`).
fn chunk_extent(
    ctx: ir::selfwidth::ExprCtx<'_>,
    geom: &std::collections::HashMap<u32, u32>,
    provisional: bool,
    c: &ir::LvalChunk,
) -> Extent {
    let Some(nv) = ctx.nets.get(c.net as usize) else {
        return Extent::Poison;
    };
    if c.word.is_some() {
        return Extent::Word;
    }
    // THE STAND-DOWN (module head): a select's offset or width may hold a fold of a
    // provisional value; a whole-net chunk reads no expression. Deleted when §5.2
    // Q-HIERBITS lands.
    if provisional && (c.offset.is_some() || c.width.is_some()) {
        return Extent::Undecided;
    }
    let net_w = nv.width.max(1);
    if let Some((lo, hi)) = ir::const_chunk_written_bits(ctx, c, net_w) {
        return Extent::Bits {
            lo,
            hi,
            by: By::Kernel,
        };
    }
    let geo = c
        .offset
        .and_then(|e| geom.get(&e))
        .and_then(|&v| ir::chunk_written_bits_at(ctx, c, i128::from(v), net_w));
    match geo {
        Some((lo, hi)) => Extent::Bits {
            lo,
            hi,
            by: By::Geometry,
        },
        None => Extent::Undecided,
    }
}

impl WriteView {
    /// The dump: one line per write site in the view's order, then one `N` line per net
    /// the view names (ascending id). Text, deterministic: a site's location and a net's
    /// name come from the sidecars of the same elaboration.
    pub fn render(&self, ir: &ir::SimIr, sc: &Sidecars) -> String {
        use std::fmt::Write as _;
        let name = |n: u32| {
            sc.net_names
                .get(n as usize)
                .cloned()
                .unwrap_or_else(|| format!("#{n}"))
        };
        let loc = |id: Option<&ProcIdent>| match id {
            Some(i) if i.line > 0 => format!("@{}:{}:{}", i.file, i.line, i.col),
            _ => String::new(),
        };
        let mut out = String::new();
        let (mut bits, mut geo, mut word, mut und, mut poi) = (0, 0, 0, 0, 0);
        for w in &self.writes {
            match w.extent {
                Extent::Bits { by: By::Kernel, .. } => bits += 1,
                Extent::Bits {
                    by: By::Geometry, ..
                } => geo += 1,
                Extent::Word => word += 1,
                Extent::Undecided => und += 1,
                Extent::Poison => poi += 1,
            }
        }
        let _ = writeln!(
            out,
            "WEXT writes={} bits={bits} geo={geo} word={word} undecided={und} poison={poi}",
            self.writes.len()
        );
        let mut named = BTreeSet::new();
        for w in &self.writes {
            let who = match w.writer {
                Writer::Cont { row, kind } => {
                    format!("C{row}:{kind}{}", loc(sc.ca_idents.get(row as usize)))
                }
                Writer::Proc { pid, kind } => {
                    format!("P{pid}:{kind}{}", loc(sc.proc_idents.get(pid as usize)))
                }
                Writer::Body { func, task } => {
                    let f = sc
                        .func_names
                        .get(func as usize)
                        .cloned()
                        .unwrap_or_default();
                    format!("F{func}:{}:{f}", if task { "task" } else { "func" })
                }
            };
            let form = match w.form {
                Form::Continuous => "cont",
                Form::Blocking => "blk",
                Form::NonBlocking => "nba",
                Form::Force => "force",
                Form::Release => "release",
            };
            let sid = w.stmt.map_or("-".to_string(), |s| format!("s{s}"));
            let ext = match w.extent {
                Extent::Bits {
                    lo,
                    hi,
                    by: By::Kernel,
                } => format!("bits[{}:{lo}]", hi - 1),
                Extent::Bits {
                    lo,
                    hi,
                    by: By::Geometry,
                } => format!("geo[{}:{lo}]", hi - 1),
                Extent::Word => "word".to_string(),
                Extent::Undecided => "undecided".to_string(),
                Extent::Poison => "poison".to_string(),
            };
            let net = if matches!(w.extent, Extent::Poison) {
                format!("#{}", w.net)
            } else {
                named.insert(w.net);
                format!("n{}:{}", w.net, name(w.net))
            };
            let via = match w.via {
                None => "-",
                Some(Via::Direct) => "direct",
                Some(Via::TaskInline) => "task",
                Some(Via::DeferAction) => "defer",
            };
            let at = w.at.map_or("-".to_string(), |sp| sp.lo.to_string());
            let _ = writeln!(
                out,
                "W {who} {form} {sid} c{} {net} {ext} via={via} at={at}",
                w.chunk
            );
        }
        for n in named {
            let nv = &ir.nets[n as usize];
            let decl = match self.decl(n) {
                None => "-".to_string(),
                Some(d) => format!(
                    "{:?}/{:?}/{:?}{}",
                    d.site,
                    d.kind,
                    d.dir,
                    if d.type_param { "/tparam" } else { "" }
                ),
            };
            let _ = writeln!(
                out,
                "N n{n}:{} {:?} w={} alen={} dims={} decl={decl}",
                name(n),
                nv.kind,
                nv.width,
                nv.array_len,
                if sc.net_dims.contains_key(&n) {
                    "y"
                } else {
                    "n"
                }
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(width: u32) -> ir::NetVar {
        ir::NetVar {
            kind: ir::NetKind::Logic,
            width,
            msb: width.saturating_sub(1),
            lsb: 0,
            signed: false,
            array_len: 1,
            dir: ir::PortDir::Internal,
            init: ir::BitPacked {
                val: vec![0],
                unk: vec![0],
            },
        }
    }

    fn whole(net: u32) -> ir::Lvalue {
        ir::Lvalue {
            chunks: vec![ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: ir::SelKind::Bit,
            }],
        }
    }

    /// A chunk whose net id is not a net is `Poison` and does not index the net arena
    /// (a refused lvalue's `POISON_NET`; only an IR that latched `had_error` has one).
    #[test]
    fn a_chunk_on_no_net_is_poison() {
        let nets = vec![net(8)];
        let cas = vec![
            ir::ContAssign {
                lhs: whole(POISON_NET),
                rhs: 0,
                delay: None,
            },
            ir::ContAssign {
                lhs: whole(0),
                rhs: 0,
                delay: None,
            },
        ];
        let ids = vec![ProcIdentRow::default(), ProcIdentRow::default()];
        let v = build_view(&WextSource {
            nets: &nets,
            exprs: &[],
            consts: &[],
            cont_assigns: &cas,
            ca_idents: &ids,
            processes: &[],
            proc_idents: &[],
            stmts: &[],
            blocks: &[],
            funcs: &[],
            geom: &std::collections::HashMap::new(),
            vias: &[],
            row_at: &[],
            stmt_at: &[],
            provisional: false,
        });
        let ext: Vec<Extent> = v.writes.iter().map(|w| w.extent).collect();
        assert_eq!(
            ext,
            vec![
                Extent::Poison,
                Extent::Bits {
                    lo: 0,
                    hi: 8,
                    by: By::Kernel
                }
            ]
        );
        assert_eq!(v.writes[0].net, POISON_NET);
    }

    /// The registration's setter raises the flag (THE STAND-DOWN).
    #[test]
    fn reg_sets_flag() {
        let mut r = WextRecords::default();
        assert!(!r.provisional);
        r.note_provisional();
        assert!(r.provisional);
    }

    /// The funnel's setter: overwriting a `Const` slot raises the flag.
    #[test]
    fn patch_of_const_sets_flag() {
        let mut r = WextRecords::default();
        r.note_patch(&ir::Expr::Const { val: 0 });
        assert!(r.provisional);
    }

    /// The funnel's setter: overwriting a deferred read placeholder (a `Signal` on
    /// `POISON_NET`, a `Call` on `POISON_FID`) leaves the flag clear.
    #[test]
    fn patch_of_signal_keeps_clear() {
        let mut r = WextRecords::default();
        r.note_patch(&ir::Expr::Signal {
            net: POISON_NET,
            word: None,
        });
        r.note_patch(&ir::Expr::Call {
            func: POISON_FID,
            args: Vec::new(),
        });
        assert!(!r.provisional);
    }

    /// With the flag set a select (offset and width) is `Undecided`, a whole-net chunk
    /// keeps the kernel's `[0, 8)` and an array word stays `Word`; with it clear the select
    /// is the kernel's `[2, 3)`.
    #[test]
    fn stand_down_takes_selects_only() {
        let nets = vec![net(8)];
        let consts = vec![make_const_u32(2, 32), make_const_u32(1, 32)];
        let exprs = vec![ir::Expr::Const { val: 0 }, ir::Expr::Const { val: 1 }];
        let sel = ir::LvalChunk {
            net: 0,
            word: None,
            offset: Some(0),
            width: Some(1),
            kind: ir::SelKind::Bit,
        };
        let word = ir::LvalChunk {
            net: 0,
            word: Some(0),
            offset: None,
            width: None,
            kind: ir::SelKind::Bit,
        };
        let cas: Vec<ir::ContAssign> = [sel, word]
            .into_iter()
            .map(|c| ir::ContAssign {
                lhs: ir::Lvalue { chunks: vec![c] },
                rhs: 0,
                delay: None,
            })
            .chain([ir::ContAssign {
                lhs: whole(0),
                rhs: 0,
                delay: None,
            }])
            .collect();
        let ids = vec![ProcIdentRow::default(); 3];
        let ext = |provisional: bool| -> Vec<Extent> {
            build_view(&WextSource {
                nets: &nets,
                exprs: &exprs,
                consts: &consts,
                cont_assigns: &cas,
                ca_idents: &ids,
                processes: &[],
                proc_idents: &[],
                stmts: &[],
                blocks: &[],
                funcs: &[],
                geom: &std::collections::HashMap::new(),
                vias: &[],
                row_at: &[],
                stmt_at: &[],
                provisional,
            })
            .writes
            .iter()
            .map(|w| w.extent)
            .collect()
        };
        let whole8 = Extent::Bits {
            lo: 0,
            hi: 8,
            by: By::Kernel,
        };
        let bit2 = Extent::Bits {
            lo: 2,
            hi: 3,
            by: By::Kernel,
        };
        assert_eq!(ext(false), vec![bit2, Extent::Word, whole8]);
        assert_eq!(ext(true), vec![Extent::Undecided, Extent::Word, whole8]);
    }

    /// EACH WRITE'S OWN POSITION (C2): row `r` takes `row_at[r]`, every chunk of a row
    /// the same one; a row past the record is `None`.
    #[test]
    fn row_at_joins_each_row() {
        let nets = vec![net(8), net(8)];
        let two = ir::Lvalue {
            chunks: vec![whole(0).chunks[0].clone(), whole(1).chunks[0].clone()],
        };
        let cas: Vec<ir::ContAssign> = [two, whole(0), whole(1)]
            .into_iter()
            .map(|lhs| ir::ContAssign {
                lhs,
                rhs: 0,
                delay: None,
            })
            .collect();
        let ids = vec![ProcIdentRow::default(); 3];
        let row_at = vec![Some(ast::Span::new(7, 9)), None];
        let v = build_view(&WextSource {
            nets: &nets,
            exprs: &[],
            consts: &[],
            cont_assigns: &cas,
            ca_idents: &ids,
            processes: &[],
            proc_idents: &[],
            stmts: &[],
            blocks: &[],
            funcs: &[],
            geom: &std::collections::HashMap::new(),
            vias: &[],
            row_at: &row_at,
            stmt_at: &[],
            provisional: false,
        });
        let at: Vec<Option<u32>> = v.writes.iter().map(|w| w.at.map(|s| s.lo)).collect();
        assert_eq!(at, vec![Some(7), Some(7), None, None]);
        assert_eq!(v.writes[0].at, Some(ast::Span::new(7, 9)));
    }

    /// The plan's resolution, spelled with the sort (PLAN V4: sort key `(start,
    /// Reverse(end), Reverse(record index))`, later paints win) — the reference the
    /// one-pass `stmt_at_of` must equal on every family its recorders can make.
    fn stmt_at_sorted(
        stmt_at: &[(u32, u32, Option<ast::Span>)],
        n: usize,
    ) -> Vec<Option<ast::Span>> {
        if stmt_at.is_empty() {
            return Vec::new();
        }
        let mut at_of = vec![None; n];
        let mut order: Vec<usize> = (0..stmt_at.len()).collect();
        order.sort_by_key(|&i| {
            let (a, b, _) = stmt_at[i];
            (a, std::cmp::Reverse(b), std::cmp::Reverse(i))
        });
        for i in order {
            let (a, b, sp) = stmt_at[i];
            for slot in at_of.iter_mut().take(b as usize).skip(a as usize) {
                *slot = sp;
            }
        }
        at_of
    }

    /// C3, V4's meaning: innermost interval, equal intervals to the one recorded first,
    /// a floor or no interval `None` — on a hand-built family whose ids 0..12 are:
    /// 0 outside every interval ; 1 a prologue store under a floor ; 2 a statement inside
    /// the floor ; 3 a statement equal to its child's interval (the child, recorded first,
    /// wins) ; 4 the caller's own store after the body ; 5..7 a call statement around a
    /// floor of the same extent recorded first (the floor wins) ; 8..10 nested three deep ;
    /// 11 after every interval.
    #[test]
    fn stmt_at_innermost_first_recorded_floor() {
        let sp = |lo: u32| Some(ast::Span::new(lo, lo + 1));
        // record order = exit order of the recorders
        let rec = vec![
            (2, 3, sp(20)),  // a statement inside the floor
            (3, 4, sp(30)),  // the child ...
            (3, 4, sp(31)),  // ... and its parent, same ids
            (1, 4, None),    // the floor of a body lowered inside the caller
            (1, 5, sp(10)),  // the caller's statement (its own store 4)
            (5, 8, None),    // a callee's floor ...
            (5, 8, sp(50)),  // ... and the call statement, same ids, recorded after
            (9, 10, sp(90)), // nested three deep
            (8, 11, sp(80)),
            (8, 11, sp(81)),
        ];
        let got = stmt_at_of(&rec, 12);
        let lo: Vec<Option<u32>> = got.iter().map(|s| s.map(|s| s.lo)).collect();
        assert_eq!(
            lo,
            vec![
                None,
                None,
                Some(20),
                Some(30),
                Some(10),
                None,
                None,
                None,
                Some(80),
                Some(90),
                Some(80),
                None
            ]
        );
        assert_eq!(got, stmt_at_sorted(&rec, 12));
        assert!(stmt_at_of(&[], 5).is_empty());
    }

    /// C3, the one-pass resolution equals the plan's sorted one on every family a
    /// call-stack of recorders makes (each records `[len at entry, len at exit)` at its
    /// exit, a floor `None`, empty ones skipped): 300 seeded trees, up to depth 5.
    #[test]
    fn stmt_at_one_pass_equals_sorted() {
        fn gen(
            rng: &mut u64,
            depth: u32,
            len: &mut u32,
            rec: &mut Vec<(u32, u32, Option<ast::Span>)>,
        ) {
            let mut next = || {
                *rng = rng
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (*rng >> 33) as u32
            };
            let start = *len;
            *len += next() % 3;
            let kids = if depth < 5 { next() % 3 } else { 0 };
            let floor = next() % 4 == 0;
            let after = next() % 2;
            for _ in 0..kids {
                gen(rng, depth + 1, len, rec);
            }
            *len += after;
            if start < *len {
                let at = (!floor).then(|| ast::Span::new(start * 7 + depth, start * 7 + depth + 1));
                rec.push((start, *len, at));
            }
        }
        for seed in 0..300u64 {
            let mut rng = seed;
            let (mut len, mut rec) = (0u32, Vec::new());
            for _ in 0..3 {
                gen(&mut rng, 0, &mut len, &mut rec);
            }
            let n = len as usize + 1;
            assert_eq!(
                stmt_at_of(&rec, n),
                stmt_at_sorted(&rec, n),
                "seed {seed}: {rec:?}"
            );
        }
    }

    /// C3, the zero-span guard: an empty span records no interval (`record_stmt_at`
    /// asserts it in a debug run, before calling here) ; a span of one byte does.
    #[test]
    fn zero_span_records_nothing() {
        let mut r = WextRecords::default();
        r.note_stmt_at(2, 3, ast::Span::new(5, 5));
        assert!(r.stmt_at.is_empty());
        r.note_stmt_at(2, 3, ast::Span::new(5, 6));
        assert_eq!(r.stmt_at, vec![(2, 3, Some(ast::Span::new(5, 6)))]);
    }

    /// H15v (fix F1, P5 (i) ; fix S): two ports whose two connections start at one offset.
    #[test]
    #[should_panic(expected = "H15v (i)")]
    fn h15v_i_two_ports_one_span() {
        check_conn_spans(
            &[
                (0, 1, ast::Span::new(10, 11)),
                (1, 2, ast::Span::new(10, 14)),
            ],
            Some(ast::Span::new(5, 40)),
        );
    }

    /// H15v (fix S, U2): two ports reached through ONE connection node (a repeated port name
    /// in a named list: `wire_ports` finds the first connection of that name for both) are
    /// no violation ; a third port on another node at the same start is.
    #[test]
    fn h15v_same_node_two_ports() {
        let one = [
            (0, 7, ast::Span::new(10, 14)),
            (1, 7, ast::Span::new(10, 14)),
        ];
        assert_eq!(
            conn_span_violations(&one, Some(ast::Span::new(5, 40))),
            Vec::<String>::new()
        );
        check_conn_spans(&one, Some(ast::Span::new(5, 40)));
        let three = [one[0], one[1], (2, 8, ast::Span::new(10, 12))];
        assert_eq!(conn_span_violations(&three, None).len(), 2);
    }

    /// H15v (fix F1, P5 (ii)): a connection at its instance item's first byte.
    #[test]
    #[should_panic(expected = "H15v (ii)")]
    fn h15v_ii_the_item_span() {
        check_conn_spans(&[(0, 1, ast::Span::new(5, 6))], Some(ast::Span::new(5, 40)));
    }

    /// H15v: keyed by connection — one port's entry is no pair ; distinct starts, a root
    /// instance (no item) and no connection pass.
    #[test]
    fn h15v_clean() {
        let a = (0, 1, ast::Span::new(10, 11));
        check_conn_spans(
            &[
                a,
                (1, 2, ast::Span::new(13, 14)),
                (2, 3, ast::Span::new(16, 20)),
            ],
            Some(ast::Span::new(5, 40)),
        );
        check_conn_spans(&[a, (1, 2, ast::Span::new(5, 6))], None);
        check_conn_spans(&[], Some(ast::Span::new(5, 40)));
        assert_eq!(
            conn_span_violations(&[a, a], Some(ast::Span::new(5, 40))),
            Vec::<String>::new()
        );
    }

    /// Fix F1, V: with a body item open, a statement whose span starts at the item's first
    /// byte records nothing (whatever its end) ; one starting a byte later, or before the
    /// item (an inlined callee's text), records ; with no body open (`None`, never 0) a
    /// statement at offset 0 records.
    #[test]
    fn v_item_head_records_nothing() {
        let mut r = WextRecords::default();
        r.note_stmt_at(0, 1, ast::Span::new(0, 4));
        assert_eq!(r.stmt_at.len(), 1, "no body open: offset 0 records");
        r.body_item = Some(ast::Span::new(10, 50));
        r.note_stmt_at(1, 2, ast::Span::new(10, 50));
        r.note_stmt_at(2, 3, ast::Span::new(10, 20));
        assert_eq!(r.stmt_at.len(), 1, "a statement at the item's first byte");
        r.note_stmt_at(3, 4, ast::Span::new(11, 20));
        r.note_stmt_at(4, 5, ast::Span::new(4, 8));
        assert_eq!(
            r.stmt_at,
            vec![
                (0, 1, Some(ast::Span::new(0, 4))),
                (3, 4, Some(ast::Span::new(11, 20))),
                (4, 5, Some(ast::Span::new(4, 8)))
            ]
        );
    }

    /// C3: a statement's `at` is joined per statement id, every chunk of its lvalue the
    /// same one (here in a frame body; a process body reads the same `at`).
    #[test]
    fn stmt_at_joins_each_chunk() {
        let nets = vec![net(8), net(8)];
        let two = ir::Lvalue {
            chunks: vec![whole(0).chunks[0].clone(), whole(1).chunks[0].clone()],
        };
        let stmts = vec![
            ir::Stmt::BlockingAssign { lhs: two, rhs: 0 },
            ir::Stmt::BlockingAssign {
                lhs: whole(1),
                rhs: 0,
            },
        ];
        let blocks = vec![ir::BasicBlock {
            stmts: vec![0, 1],
            term: ir::Terminator::Return,
        }];
        let funcs = vec![ir::FuncDef {
            entry: 0,
            n_params: 0,
            locals_len: 0,
            is_task: true,
        }];
        let rec = vec![(0, 1, Some(ast::Span::new(40, 45))), (1, 2, None)];
        let v = build_view(&WextSource {
            nets: &nets,
            exprs: &[],
            consts: &[],
            cont_assigns: &[],
            ca_idents: &[],
            processes: &[],
            proc_idents: &[],
            stmts: &stmts,
            blocks: &blocks,
            funcs: &funcs,
            geom: &std::collections::HashMap::new(),
            vias: &[],
            row_at: &[],
            stmt_at: &rec,
            provisional: false,
        });
        let at: Vec<Option<u32>> = v.writes.iter().map(|w| w.at.map(|s| s.lo)).collect();
        assert_eq!(at, vec![Some(40), Some(40), None]);
    }
}
