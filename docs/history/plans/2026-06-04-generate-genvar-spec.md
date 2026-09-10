Working tree restored: only `crates/elaborate/src/lib.rs` remains modified (the pre-existing uncommitted generate implementation — not mine), and elaborate tests are back to 44 (the pre-me count). hdl-ast and hdl-parser are at HEAD (stub still present).

I have fully verified everything against the real crates. All my spec code compiled and passed (parser 5+1, elaborate 7, plus M2 truncated-header test). Now I'll write the final implementer-ready spec.

---

# GENERATE / GENVAR — Final Implementer Spec (verified against the real crates)

I built and ran every piece below against the actual `016_claude_rtl` workspace (cargo 1.82.0). Result: **191 workspace tests pass, 0 failures**, clippy clean. The parser code, the two verdict fixes (M1, M2), and 13 test cases all compiled and passed; I then reverted my edits so the tree is handed to you clean (only the pre-existing uncommitted `elaborate/src/lib.rs` generate body remains — that part is already done; you ADD the parser + the two fixes + tests).

## Ground-truth corrections to the research design

1. **The elaborate generate implementation is ALREADY in your working tree (uncommitted), NOT at HEAD.** `git show HEAD:crates/elaborate/src/lib.rs | grep -c elaborate_generate` → `0`. The full `elaborate_generate`/`GenPhase`/`walk_scopes`/`cur_inst`/binary-fold-extension is present as a working-tree modification. Do not re-add it. You ADD: (a) parser, (b) M1 guard inside the existing `elaborate_gen_item` For arm, (c) M2 `Span::to` clamp, (d) tests.
2. **The parser stub IS still present** at `crates/hdl-parser/src/lib.rs` lines 974-984 (`"(generate parsing not yet implemented)"` + `skip_balanced_block`). Replace it.
3. **`ModuleItem::Genvar` is a struct-variant** `Genvar { names, span }`, not a tuple — so `parse_genvar_decl` must return `ModuleItem` directly (cannot `.map(ModuleItem::Genvar)`).
4. **`opt_block_label`, `expr(0)`, `prev_span`, `cur_span`, `eat`, `eat_kw`, `at_kw`, `ident`, `parse_module_item` all exist** with the exact signatures used below. `skip_balanced_block` has zero remaining callers after the dispatch edit — delete it.

---

## 1. Rust to ADD/REPLACE

### 1a. `crates/hdl-parser/src/lib.rs` — dispatch edit

In `parse_module_item`, **replace** the stub (currently lines 973-984):

```rust
        // generate / genvar → recovering STUB (unchanged): skip_balanced_block stays.
        if matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Generate | Kw::Genvar
            )))
        ) {
            let s = self.cur_span();
            self.error("(generate parsing not yet implemented)");
            self.skip_balanced_block();
            return Some(ModuleItem::Error(s));
        }
```

**with:**

```rust
        // genvar declaration:  genvar i, j;
        if self.at_kw(Kw::Genvar) {
            return Some(self.parse_genvar_decl());
        }
        // generate construct:  generate … endgenerate  (PR3 — real parsing).
        if self.at_kw(Kw::Generate) {
            return Some(ModuleItem::Generate(self.parse_generate_construct()));
        }
```

### 1b. `crates/hdl-parser/src/lib.rs` — DELETE `skip_balanced_block` and ADD the generate/genvar impl block

Replace the whole `fn skip_balanced_block(&mut self) { … }` (currently lines ~1441-1486, including its doc comment) — it is now dead — with this new impl block. (It sits between the `impl` that contained `skip_balanced_block` and the `// PR2: statements` impl; the closing `}` of the prior impl is preserved.)

```rust
}

// ════════════════════════ PR3: generate / genvar ════════════════════════
//
// Parse-only: build the hdl-ast `GenerateConstruct`/`GenItem` tree; elaborate
// unrolls it. Mirrors the procedural for/if/case shapes (PR2) but produces
// `GenItem`s, not `Stmt`s. Every loop over a sub-item list carries a
// forward-progress guard (`pos == before → bump`) so malformed input can never
// spin, matching the rest of the parser's recovery discipline.
impl<'t, 's> Parser<'t, 's> {
    /// `genvar i, j;` → `ModuleItem::Genvar{names, span}`. The `genvar` keyword is
    /// already at `peek()`. An empty/garbled name list still terminates at `;`.
    fn parse_genvar_decl(&mut self) -> ModuleItem {
        let start = self.cur_span();
        self.bump(); // `genvar`
        let mut names = Vec::new();
        if let Some(id) = self.ident() {
            names.push(id);
            while self.eat(TokenKind::Comma) {
                match self.ident() {
                    Some(id) => names.push(id),
                    None => break, // diagnosed by ident(); stop the list
                }
            }
        }
        self.expect(TokenKind::Semi, "';' after genvar declaration");
        ModuleItem::Genvar {
            names,
            span: start.to(self.prev_span()),
        }
    }

    /// `generate <gen_items> endgenerate`. Dispatch only calls this on the
    /// `generate` keyword; the SV bare-`if`/`for`/`case`-at-module-scope form is a
    /// DEFERRED variant.
    fn parse_generate_construct(&mut self) -> GenerateConstruct {
        let start = self.cur_span();
        self.bump(); // `generate`
        let items = self.parse_gen_items_until(&|p| p.at_kw(Kw::Endgenerate) || p.at_eof());
        self.expect(
            TokenKind::Word(WordKind::Keyword(Kw::Endgenerate)),
            "'endgenerate'",
        );
        GenerateConstruct {
            items,
            span: start.to(self.prev_span()),
        }
    }

    /// Parse `GenItem`s until `stop` is true (or EOF). Shared by the construct
    /// body, gen-blocks (`begin … end`), and case-item bodies. Forward-progress
    /// guarded.
    fn parse_gen_items_until(&mut self, stop: &dyn Fn(&Self) -> bool) -> Vec<GenItem> {
        let mut items = Vec::new();
        while !self.at_eof() && !stop(self) {
            let before = self.pos;
            if let Some(it) = self.parse_gen_item() {
                items.push(it);
            }
            if self.pos == before {
                self.bump(); // never spin on a stuck gen-item
            }
        }
        items
    }

    /// One generate item: `for` / `if` / `case` / `begin…end` block / genvar decl
    /// / a plain module-item (instance, cont-assign, net, procedural block). A
    /// stray `;` (empty item) is consumed and yields nothing.
    fn parse_gen_item(&mut self) -> Option<GenItem> {
        if self.eat(TokenKind::Semi) {
            return None; // empty generate item
        }
        if self.at_kw(Kw::For) {
            return Some(self.parse_gen_for());
        }
        if self.at_kw(Kw::If) {
            return Some(self.parse_gen_if());
        }
        if self.at_kw(Kw::Case) {
            return Some(self.parse_gen_case());
        }
        if self.at_kw(Kw::Begin) {
            return Some(self.parse_gen_block());
        }
        // genvar decls inside generate are legal — keep them wrapped so elaborate's
        // no-op handler ignores them (they never become nets).
        if self.at_kw(Kw::Genvar) {
            return Some(GenItem::Item(Box::new(self.parse_genvar_decl())));
        }
        // anything else → a plain module-item (instance / assign / net / proc / …).
        // `parse_module_item` returns None only after recording an error; wrap a
        // real item, else propagate None (the caller's progress guard syncs).
        self.parse_module_item().map(|mi| GenItem::Item(Box::new(mi)))
    }

    /// `for ( genvar_id = e ; cond ; genvar_id = e ) gen_block`. A `begin : label`
    /// hoists its label onto the For node (see `parse_gen_branch`).
    fn parse_gen_for(&mut self) -> GenItem {
        let start = self.cur_span();
        self.bump(); // `for`
        self.expect(TokenKind::LParen, "'(' after generate 'for'");
        let init = self.parse_gen_assign();
        self.expect(TokenKind::Semi, "';' after generate-for init");
        let cond = self.expr(0);
        self.expect(TokenKind::Semi, "';' after generate-for cond");
        let step = self.parse_gen_assign();
        self.expect(TokenKind::RParen, "')' after generate-for header");
        let (label, body) = self.parse_gen_branch();
        GenItem::For {
            init,
            cond,
            step,
            label,
            body,
            span: start.to(self.prev_span()),
        }
    }

    /// `genvar_id = expr` (no trailing `;`) for a generate-for init/step. LHS is a
    /// single genvar identifier (the LRM restricts it — not a general lvalue).
    fn parse_gen_assign(&mut self) -> GenAssign {
        let start = self.cur_span();
        let lvalue = self.ident().unwrap_or(Ident {
            name: String::new(),
            span: start,
        });
        self.expect(TokenKind::Eq, "'=' in generate-for assignment");
        let value = self.expr(0);
        GenAssign {
            lvalue,
            value,
            span: start.to(self.prev_span()),
        }
    }

    /// `if ( cond ) gen_item [ else gen_item ]`. Dangling-else binds EAGERLY to the
    /// nearest `if` (same rule as the procedural parser).
    fn parse_gen_if(&mut self) -> GenItem {
        let start = self.cur_span();
        self.bump(); // `if`
        self.expect(TokenKind::LParen, "'(' after generate 'if'");
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')' after generate-if condition");
        let (label, then_b) = self.parse_gen_branch();
        let else_b = if self.eat_kw(Kw::Else) {
            self.parse_gen_branch().1
        } else {
            Vec::new()
        };
        GenItem::If {
            cond,
            then_b,
            else_b,
            label,
            span: start.to(self.prev_span()),
        }
    }

    /// `case ( e ) { label{,label}: gen_item | default[:] gen_item } endcase`.
    fn parse_gen_case(&mut self) -> GenItem {
        let start = self.cur_span();
        self.bump(); // `case`
        self.expect(TokenKind::LParen, "'(' after generate 'case'");
        let scrutinee = self.expr(0);
        self.expect(TokenKind::RParen, "')' after generate-case scrutinee");
        let mut items = Vec::new();
        while !self.at_eof() && !self.at_kw(Kw::Endcase) {
            let before = self.pos;
            items.push(self.parse_gen_case_item());
            if self.pos == before {
                self.bump(); // never spin on a stuck case item
            }
        }
        self.expect(
            TokenKind::Word(WordKind::Keyword(Kw::Endcase)),
            "'endcase' for generate-case",
        );
        GenItem::Case {
            scrutinee,
            items,
            span: start.to(self.prev_span()),
        }
    }

    /// One generate-case item: `default [:] gen_item` | `label {, label} : gen_item`.
    fn parse_gen_case_item(&mut self) -> GenCaseItem {
        let start = self.cur_span();
        if self.eat_kw(Kw::Default) {
            self.eat(TokenKind::Colon); // ':' OPTIONAL after default
            let body = self.parse_gen_branch().1;
            return GenCaseItem::Default {
                body,
                span: start.to(self.prev_span()),
            };
        }
        let mut labels = vec![self.expr(0)];
        while self.eat(TokenKind::Comma) {
            labels.push(self.expr(0));
        }
        self.expect(TokenKind::Colon, "':' in generate-case item");
        let body = self.parse_gen_branch().1;
        GenCaseItem::Match {
            labels,
            body,
            span: start.to(self.prev_span()),
        }
    }

    /// `begin [: label] gen_items end [: label]` → a `GenItem::Block`.
    fn parse_gen_block(&mut self) -> GenItem {
        let start = self.cur_span();
        self.bump(); // `begin`
        let label = self.opt_block_label(); // reuse PR2 helper (`: name` or None)
        let items = self.parse_gen_items_until(&|p| p.at_kw(Kw::End) || p.at_eof());
        self.expect(TokenKind::Word(WordKind::Keyword(Kw::End)), "'end'");
        self.opt_block_label(); // optional `: end_label` (no AST slot → discard)
        GenItem::Block {
            label,
            items,
            span: start.to(self.prev_span()),
        }
    }

    /// Parse a control-structure BRANCH body and HOIST a `begin:label` label out of
    /// it. Returns `(label, items)`:
    /// - `begin [: lbl] … end` → `(lbl, inner_items)` (the begin/end is unwrapped so
    ///   the For/If node carries the label directly — elaborate's `label[idx]`
    ///   prefixing expects the loop/if to OWN the label).
    /// - any other single gen-item → `(None, vec![item])`.
    fn parse_gen_branch(&mut self) -> (Option<Ident>, Vec<GenItem>) {
        if self.at_kw(Kw::Begin) {
            match self.parse_gen_block() {
                GenItem::Block { label, items, .. } => (label, items),
                other => (None, vec![other]), // unreachable; defensive
            }
        } else {
            match self.parse_gen_item() {
                Some(it) => (None, vec![it]),
                None => (None, Vec::new()),
            }
        }
    }
}
```

### 1c. `crates/elaborate/src/lib.rs` — M1 stall guard (verdict BLOCKER/MAJOR)

In the existing `elaborate_gen_item` `For` arm, the genvar step rebind currently reads:

```rust
                    self.params.insert(gv_key.clone(), next);
                    idx_count += 1;
```

**Replace those two lines with** (inserts the stall guard right after `next` is folded, before the rebind):

```rust
                    // STALL GUARD (verdict M1): the genvar VALUE namespaces each
                    // iteration's block (`label[iter_val]`). If the step does NOT
                    // advance it (`next == iter_val`, e.g. `i = i`), every iteration
                    // reuses the SAME prefix and collides at `add_net`, emitting one
                    // duplicate-decl error PER iteration up to the unroll cap (~4k
                    // spurious diagnostics). Detect the non-progressing step and stop
                    // with ONE diagnostic. (A value that merely repeats LATER — a
                    // non-monotonic cycle — is still bounded by the unroll cap;
                    // correctness intact, diagnostics less clean. Residual risk R3.)
                    if next == iter_val {
                        if phase == GenPhase::Nets {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "generate-for genvar does not advance (step leaves it unchanged)",
                            );
                        }
                        break;
                    }
                    self.params.insert(gv_key.clone(), next);
                    idx_count += 1;
```

`iter_val` is already bound earlier in the loop (`let iter_val = *self.params.get(&gv_key).unwrap_or(&0);`) and is in scope here — verified compiling.

### 1d. `crates/hdl-ast/src/lib.rs` — M2 `Span::to` clamp (verdict MAJOR, shared with PR2)

**Replace** `Span::to` (lines 46-56):

```rust
    #[inline]
    pub fn to(self, other: Span) -> Span {
        debug_assert!(
            other.hi >= self.lo,
            "Span::to: inverted union {self:?}..{other:?}"
        );
        Span {
            lo: self.lo,
            hi: other.hi,
        }
    }
```

**with:**

```rust
    #[inline]
    pub fn to(self, other: Span) -> Span {
        // CLAMP (verdict M2): a recovery path that composes spans out of order —
        // a parser header whose tokens never advanced past `start` (e.g.
        // `generate for endgenerate`, or PR2's `initial for end`) — would otherwise
        // yield an inverted `[lo, hi)` with `hi < lo`. The old `debug_assert!`
        // PANICKED there (a debug/test-only DoS on truncated input); release
        // silently produced a wrong span. Flooring `hi` at `lo` makes the union
        // total and non-panicking on EVERY input while preserving the normal
        // monotonic-cursor case byte for byte (where `other.hi >= self.lo` already
        // holds), so the determinism / golden-hash contract is unchanged.
        Span {
            lo: self.lo,
            hi: other.hi.max(self.lo),
        }
    }
```

This is the single shared fix that also closes the pre-existing PR2 panic on `module m; initial for end endmodule`. Verified: all hdl-ast schema_hash / frozen-shapes / no_serde_attrs golden tests still pass (the clamp is a no-op on every well-formed union).

---

## 2. Cargo changes

**None.** `elaborate` does not depend on `hdl-parser`; tests hand-build the AST. Confirmed against all three `Cargo.toml`s. MSRV 1.82 / edition 2021 satisfied (verified with cargo 1.82.0).

---

## 3. Test cases (all compiled + passed)

**Parser (`crates/hdl-parser/src/lib.rs` `mod tests`)** — add `gen_of` helper + g1-g6. Note g1 needs `let su = su.unwrap(); let m = first_module(&su);` (two statements — a one-liner `first_module(&su.unwrap())` trips E0716 borrow-of-temporary; I hit and fixed this).

- **g1** `genvar i, j;` → `Genvar{names==["i","j"]}`.
- **g2** `generate for(i=0;i<3;i=i+1) begin:g leaf u(.a(x[i])); end endgenerate` → one `For`; `init.lvalue=="i"`, `step.lvalue=="i"`, **label hoisted** to `Some("g")`, body is one `Item(Instance)`.
- **g3** bare-body `for(...) assign y[i]=a[i];` → `For` with `label.is_none()`, body one `Item(ContAssign)`.
- **g4** `if(W) … else …` → both `then_b` and `else_b` len 1; and `if(W) …` (no else) → `else_b.is_empty()`.
- **g5** `case(W) 0: …; 1,2: …; default: … endcase` → 3 items: `Match{1 label}`, `Match{2 labels}`, `Default`.
- **g6** (M2) loop over `["module m; generate for endgenerate endmodule", "… if (", "… case (", "generate for ("]` → each returns `!errs.is_empty()` and **does not panic**. This is the regression test that fails (panics) without the 1d clamp.

**Elaborate e2e (`crates/elaborate/src/tests.rs`)** — add `gen_for`/`generate`/`gitem` helpers + ge1-ge7.

- **ge1** generate-for instantiating `leaf` (with an **input** port `a`, driven by top's `w`) 3× → `instances.len()==4` (1 top + 3 leaf), `instances[0].parent.is_none()`, all leaf `parent==Some(0)`. (Design note: I originally used an **output** port; 3 outputs onto one `w` is a multidriver → fatal → `elaborate` returns `None`. Use an input port to avoid that — load-bearing fix.)
- **ge2** loop body `wire t; assign t=1'b0;` ×3 → `nets.len()==3`, `cont_assigns.len()==3`, the three `lhs.chunks[0].net` are **distinct** (per-iteration `g[0].t/g[1].t/g[2].t`, no collision). Asserts `n_errors()==0`.
- **ge3/ge4** `if(COND) assign y=a; else assign y=b;` → exactly one cont-assign survives; inspect `s.exprs[ca.rhs]` as `ir::Expr::Signal{net}`: COND=1 reads net 0 (`a`), COND=0 reads net 1 (`b`).
- **ge5** `wire [i:0] t;` in the loop → `nets` widths `[1,2,3]` (genvar in the msb bound; const-eval folds `i`).
- **ge6** determinism: build the same unit twice → `a.nets==b.nets && a.instances==b.instances && a.cont_assigns==b.cont_assigns`.
- **ge7** (M1) `for(i=0;i<5;i=i)` (stuck step) → `n_errors()==1` and `diag_codes` contains `MsgCode::ElabUnsupported` (NOT ~4096 dup errors). This is the regression test that fails (4096 errors) without the 1c guard.

Net AST/IR facts these rely on (verified): `SimIr` has **no net-name field** (`self.symbols` is dropped at `finish`) — so e2e asserts on counts/widths/parents/net-ids, never names. `cont_assign.lhs.chunks[0].net` = target NetId; `s.exprs[ca.rhs as usize]` = RHS expr. `elaborate` returns `None` on ANY error (`had_error`), so "clean" tests use `.expect(...)`.

---

## 4. Coverage statement + deferred list

**Covered (parse + elaborate, with passing tests):** genvar multi-decl; generate-for (labeled begin/end with label-hoist, and bare single-item body) → N-way instance/net/cont-assign unroll with genvar-valued `label[idx]` namespacing; generate-if true/false branch selection (with and without else); generate-case (multi-label match, first-match-wins, default fallback); genvar in const positions (loop cond/step, net width range); genvar-only-in-`self.params` (never a sim-ir net); deterministic ascending unroll; child-instance `parent` wiring via `cur_inst`; 3-phase Nets<Logic<Instances ordering; outward generate-scope name visibility that stops at instance boundaries (the v3 hierarchy tests still pass); unroll cap + depth cap termination; stuck-genvar single-error (M1); truncated-header recovery without panic (M2); forward-progress guards on every item list.

**Deferred (documented, with explicit elaborate diagnostics where reachable):**
- **N1 — genvar in a RUNTIME expression** (`assign out[i]=in[i];`, RHS use): resolves via `const_eval_in_scope`/`self.params` in const positions only; `lower_expr → resolve_net` searches `self.symbols` (nets) only, so a genvar in a non-folded expression yields `ElabUnresolvedName`. **A very common pattern.** v1 limit — must be in release notes and have an explicit "expected error" test. Future fix: fold a const genvar Ident to an `ir::Expr::Const` inside `lower_expr` (or have `lower_expr`'s Ident arm consult `lookup_scoped` before `resolve_net`).
- Nested generate beyond `GENERATE_DEPTH_CAP=32` (one `ElabUnsupported`).
- SV bare `if/for/case`-at-module-scope without the `generate` wrapper.
- Hierarchical genvar references across instance boundaries (the scope walk deliberately stops at instance segments).
- Sibling **unlabeled** generate-fors at one scope (both fall back to fixed `"genblk"` → would collide). Single unlabeled loop is fine; add a code comment. (NIT N2.)
- `func`/`task`/`defparam`/`param`/`port-decl` inside generate → `ElabUnsupported` (intentional).
- `Pow`/`AShl`/`AShr` in const-eval (returns `None`).

---

## 5. Residual risks

- **R1 (parser, low):** `parse_gen_branch` for a bare (non-`begin`) branch calls `parse_gen_item`, which can itself start another `if`/`for` — so `if(a) if(b) x; else y;` parses as nested gen-ifs with eager-else, matching the procedural rule. Acceptable and intended; no test asserts the deep-nest else binding — add one if you want belt-and-suspenders.
- **R2 (M2 clamp, very low):** the clamp removes the `debug_assert!` that the design's "verdict M4" added as a tripwire for out-of-order span composition. If a *future* PR composes spans out of order in a non-recovery path, it'll now silently get a degenerate-but-valid span instead of a panic. Net positive (no debug DoS), but you lose that one assertion. If you want both, keep the assert AND clamp: `debug_assert!(...); Span{ lo: self.lo, hi: other.hi.max(self.lo) }` — though then truncated-input tests must run in release, or the assert must be downgraded to the gen/proc call sites only. I chose the pure clamp (simplest, total, non-panicking everywhere).
- **R3 (M1 guard, low):** the guard catches a *stuck* genvar (`next == iter_val`), not a genvar that revisits a value after progressing (e.g. a pathological `i = (i+1) % 3`). That non-monotonic cycle is still bounded by `GENERATE_UNROLL_CAP=4096` (correctness intact) but would re-emit dup-decl errors per revisited value until the cap. Real generate loops are monotonic; this is acceptable for v1 and noted in the guard comment.
- **R4 (golden hash, low):** I verified `schema_hash`/`frozen_shapes`/`no_serde_attrs` and all 191 workspace tests pass with both fixes applied; the M2 clamp is a byte-for-byte no-op on well-formed input, so the velab golden-hash contract holds. Re-run `cargo test --workspace` after you apply, to reconfirm in your tree state.
- **R5 (scope walk, low):** `walk_scopes` distinguishes a generate segment from an instance segment by `seg.contains('[')`. An instance literally named with a `[` (illegal in Verilog, but the parser doesn't forbid it) would be misclassified as a generate scope. Not reachable from valid input; noted.

Files: `crates/hdl-parser/src/lib.rs` (dispatch + new impl block, delete `skip_balanced_block`), `crates/hdl-ast/src/lib.rs` (`Span::to`), `crates/elaborate/src/lib.rs` (M1 guard in `elaborate_gen_item` For arm), tests in `crates/hdl-parser/src/lib.rs` and `crates/elaborate/src/tests.rs`.