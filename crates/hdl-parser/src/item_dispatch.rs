//! the module-body item dispatcher — split out of `module_items.rs` (mechanical move, §5.2 row 66-0).

use super::*;

impl Parser<'_, '_> {
    /// `[IDENT :] (assert | assume) (# | final)` at the cursor: the head of a
    /// deferred immediate assertion (IEEE 1800-2017 A.6.10). `parse_assert` keeps a
    /// `#` other than `#0` loud.
    pub(crate) fn at_deferred_assertion_item(&self) -> bool {
        let k = if self.is_ident() && self.peek_at(1) == Some(TokenKind::Colon) {
            2
        } else {
            0
        };
        matches!(
            self.peek_at(k),
            Some(TokenKind::Word(WordKind::Keyword(Kw::Assert | Kw::Assume)))
        ) && matches!(
            self.peek_at(k + 1),
            Some(TokenKind::Hash) | Some(TokenKind::Word(WordKind::Keyword(Kw::Final)))
        )
    }

    pub(crate) fn parse_module_item(&mut self) -> Option<ModuleItem> {
        // skip a stray lexer error token without re-reporting (already diagnosed)
        if self.at_lex_error() {
            let s = self.cur_span();
            self.bump();
            return Some(ModuleItem::Error(s));
        }
        // IEEE 1800-2017 §16.4: a deferred immediate assertion (`assert #0`,
        // `assert final`, and the `assume` forms) is also a module item
        // (`deferred_immediate_assertion_item`, A.6.10), "treated as if it were
        // contained in an always_comb procedure". Desugar it onto exactly that: an
        // `always_comb` whose body is the statement the procedural parser builds, so
        // a label becomes the named block `parse_labeled_stmt` makes (`%m` reads
        // `top.L`, as in the LRM's own `always_comb begin L: assert #0 … end`). The
        // procedural form is supported, so the item reaches only machinery a
        // hand-written `always_comb` already reaches. A package or a program has no
        // such item (A.1.11, A.1.7) and keeps the loud error below.
        if !self.in_package && !self.in_program && self.at_deferred_assertion_item() {
            let start = self.cur_span();
            let body = Box::new(self.parse_statement());
            // A declaration in the action block is not given a scope of its own — it
            // is bound to a same-named module variable by name (ROADMAP
            // `mdrv-assert-local`; the procedural form is PRE's, unchanged) — so the
            // module item, which this desugar newly accepts, refuses it.
            if action_declares(&body) {
                self.error_at(
                    start,
                    "a deferred assertion item whose action block declares nothing (a \
                     declaration in its action block is unsupported: it would bind to the \
                     module's same-named variable)",
                );
            }
            return Some(ModuleItem::Proc(ProceduralBlock {
                kind: ProcKind::AlwaysComb,
                sensitivity: None,
                body,
                span: start.to(self.prev_span()),
            }));
        }
        // G9: an optional `label :` prefix on a labelable concurrent-assertion module
        // item (IEEE 1800 §16.2: `name : assert|assume property …` / `name : cover
        // property …`). At module scope a leading `IDENT :` is ONLY a label — an
        // instantiation is `TYPE inst (...)`, never `TYPE :` — so gate on the following
        // assert/assume/cover-property keyword; a stray `IDENT :` still falls through
        // and errors loudly. The label only names the assertion (the checker is wrapped
        // in a synthetic `initial` with no label slot), so consume + discard it and let
        // the existing assert/assume/cover arms below materialize the checker unchanged.
        if self.is_ident() && self.peek_at(1) == Some(TokenKind::Colon) {
            let after_colon_assert = matches!(
                self.peek_at(2),
                Some(TokenKind::Word(WordKind::Keyword(Kw::Assert | Kw::Assume)))
            );
            let after_colon_cover = self.text_at(2) == "cover"
                && self.peek_at(3) == Some(TokenKind::Word(WordKind::Keyword(Kw::Property)));
            if after_colon_assert || after_colon_cover {
                self.bump(); // label ident
                self.bump(); // ':'
            }
        }
        // parameter / localparam — a COMMA-LIST shares ONE type prefix across every
        // name (`localparam [T] A = 1, B = 2;` — IEEE §6.20.1). The first name emits
        // inline; the rest queue in `pending_module_items` and drain (in order, same
        // scope) at the next `parse_module_item`/`parse_gen_item`.
        if self.at_kw(Kw::Parameter) || self.at_kw(Kw::Localparam) || self.at_kw(Kw::Specparam) {
            return self.parse_param_list_item();
        }
        // defparam path = expr [, path = expr]* ;  (IEEE §23.10.1)
        if self.at_kw(Kw::Defparam) {
            return self.parse_defparam().map(ModuleItem::Defparam);
        }
        // continuous assign
        if self.at_kw(Kw::Assign) {
            return self.parse_cont_assign().map(ModuleItem::ContAssign);
        }
        // GATE: gate-level primitive instantiation (and/or/nand/nor/xor/xnor/
        // buf/not/bufif0/bufif1/notif0/notif1) — desugared to continuous assigns.
        if let Some(g) = self.gate_kind() {
            return self.parse_gate_primitive(g).map(ModuleItem::ContAssign);
        }
        // non-ANSI body port direction decl
        if matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Input | Kw::Output | Kw::Inout
            )))
        ) {
            return self.parse_port_decl().map(ModuleItem::PortDecl);
        }
        // SV `typedef enum/…/<type> name;` (Phase-2 user-defined types).
        if self.at_kw(Kw::Typedef) {
            return self.parse_typedef();
        }
        // ⓑ-breadth (§25.9): `virtual INTERFACE vif [, vif2];` handle declaration.
        // Distinguished from a `virtual function/task` method by the keyword that
        // follows: an interface/type NAME (an ident) vs `function`/`task`.
        if self.at_kw(Kw::Virtual)
            && matches!(self.peek_at(1), Some(TokenKind::Word(WordKind::Ident)))
        {
            return self.parse_virtual_iface_decl().map(ModuleItem::NetVar);
        }
        // N7: `class NAME …; … endclass` declared inside a module/package body.
        if self.at_kw(Kw::Class) {
            return self.parse_class_decl().map(ModuleItem::Class);
        }
        // v5 ⑥: `modport mp (input a, output b);` — interface body item.
        if self.at_kw(Kw::Modport) {
            return self.parse_modport().map(ModuleItem::Modport);
        }
        // v7 P2-D: module/package-scope `import pkg::…;`.
        if self.at_kw(Kw::Import) {
            // A comma list emits its FIRST term inline and queues the rest in
            // `pending_module_items` — the same mechanism the body-param comma
            // list uses, because this function may return only one item.
            let mut list = self.parse_import_decl_list()?.into_iter();
            let first = list.next()?;
            self.pending_module_items
                .extend(list.map(ModuleItem::Import));
            return Some(ModuleItem::Import(first));
        }
        // net/var declaration
        if self.net_var_kind().is_some() {
            // Module-item scope (also reached for generate items via
            // parse_gen_item → parse_module_item): a net-decl delay IS allowed.
            return self.parse_net_var(true).map(ModuleItem::NetVar);
        }
        // typedef-name declaration: `color_t c, d;` where `color_t` was typedef'd.
        if let Some(info) = self.peek_typedef_name() {
            return self.parse_typed_decl(info).map(ModuleItem::NetVar);
        }
        // procedural blocks → REAL parsing (PR2).
        if matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Initial
                    | Kw::Always
                    | Kw::AlwaysFf
                    | Kw::AlwaysComb
                    | Kw::AlwaysLatch
                    | Kw::Final
            )))
        ) {
            return Some(ModuleItem::Proc(self.parse_procedural_block()));
        }
        // function/endfunction and task/endtask definitions.
        if self.at_kw(Kw::Function) {
            return Some(self.parse_function_item());
        }
        if self.at_kw(Kw::Task) {
            return Some(ModuleItem::Task(self.parse_task_def()));
        }
        // genvar declaration:  genvar i, j;
        if self.at_kw(Kw::Genvar) {
            return Some(self.parse_genvar_decl());
        }
        // generate construct:  generate … endgenerate  (PR3 — real parsing).
        if self.at_kw(Kw::Generate) {
            return Some(ModuleItem::Generate(self.parse_generate_construct()));
        }
        // IEEE 1800-2017 §27.3: **the `generate` and `endgenerate` keywords are
        // optional.** A conditional or loop generate written without them is the
        // dominant spelling in modern SV and is what synthesis tools and other
        // simulators are handed, so refusing it made the wrapper mandatory in vita
        // and only in vita. The error it produced pointed at the `end`/`else` that
        // followed, never at the missing keyword.
        //
        // Only these three keywords reach here: `parse_gen_item` tests them BEFORE
        // falling through to `parse_module_item`, so a bare `if` inside an explicit
        // `generate` block never takes this path and the two spellings cannot
        // recurse into each other.
        if self.at_kw(Kw::If) || self.at_kw(Kw::For) || self.at_kw(Kw::Case) {
            let start = self.cur_span();
            let items = self.parse_gen_item().into_iter().collect::<Vec<_>>();
            return Some(ModuleItem::Generate(GenerateConstruct {
                items,
                span: start.to(self.prev_span()),
            }));
        }
        // module-level concurrent assertion: `assert property(@(clk) …);`
        // (slice S10). Only `assert property` is a module item — an immediate
        // `assert (expr)` is procedural-only and is a loud error here. The
        // concurrent form is wrapped in a synthetic `initial` so it flows
        // through the same procedural ConcurrentAssert collection
        // (`pending_sva`); the checker is materialized at module level
        // regardless, so this is a pure parser-placement change (no AST shape
        // change, no sim-ir change).
        if self.at_kw(Kw::Assert) || self.at_kw(Kw::Assume) {
            let start = self.cur_span();
            self.bump(); // `assert` / `assume`
            if !self.at_kw(Kw::Property) {
                self.error(
                    "`property` after `assert`/`assume` at module level (immediate \
                     assertions are procedural-only)",
                );
                return Some(ModuleItem::Error(start.to(self.prev_span())));
            }
            // SVA-REST: `assume property` is checked exactly like `assert property`
            // in simulation (IEEE §16.12 — the assumption is verified); the same
            // synthesized checker is materialized.
            let stmt = self.parse_concurrent_assert(start);
            let span = start.to(self.prev_span());
            return Some(ModuleItem::Proc(ProceduralBlock {
                kind: ProcKind::Initial,
                sensitivity: None,
                body: Box::new(stmt),
                span,
            }));
        }
        // SVA-REST: module-level `cover property(@(clk) seq);` — wrapped in a synthetic
        // `initial` (like module-level `assert property`) so it flows through the same
        // procedural collection; the counter/report is materialized at module level.
        if self.at_ident_kw("cover")
            && self.peek_at(1) == Some(TokenKind::Word(WordKind::Keyword(Kw::Property)))
        {
            let start = self.cur_span();
            let stmt = self.parse_cover_property();
            let span = start.to(self.prev_span());
            return Some(ModuleItem::Proc(ProceduralBlock {
                kind: ProcKind::Initial,
                sensitivity: None,
                body: Box::new(stmt),
                span,
            }));
        }
        // SVA-REST: `let NAME [(formals)] = expr;` (IEEE 1800 §11.13) — a named
        // expression macro. `let` is contextual (an SV reserved word, never a legal
        // net name), recognized only when followed by an identifier.
        if self.at_ident_kw("let")
            && matches!(
                self.peek_at(1),
                Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
            )
        {
            return self.parse_let_decl();
        }
        // Named SVA declarations (Phase-3 named-SVA slice). `sequence` /
        // `endsequence` / `endproperty` are CONTEXTUAL keywords (`at_ident_kw`,
        // like `throughout`/`within`/`iff`); `property` is `Kw::Property`. Placed
        // before the bare-ident instantiation arm so `sequence s; …` is not
        // mis-parsed as a module instantiation. A net named `sequence`
        // (`wire sequence;`) is unaffected — `net_var_kind` matches first.
        //
        // `sequence` is NOT a Verilog-2005 reserved word, so a V2005 module TYPE
        // literally named `sequence` and its instantiation (`sequence u(.o(o))`) must
        // STILL parse. A no-formals decl `sequence NAME ;` routes here on the cheap
        // 2-token guard. A PARAMETERIZED decl `sequence NAME ( … ) ;` (slice A1)
        // collides with a positional/named module instantiation of the same shape;
        // disambiguate by a content-independent forward scan for the terminating
        // `endsequence` (a decl always has one; an instantiation never does). The
        // scan is what lets `sequence u(.o(o));` (no `endsequence`) stay an
        // instantiation while `sequence s(x,y); … endsequence` is a decl.
        // `property` IS a hard keyword (`Kw::Property`) — it cannot name a module, so
        // there is no masking there.
        if self.at_ident_kw("sequence")
            && matches!(
                self.peek_at(1),
                Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
            )
            && (self.peek_at(2) == Some(TokenKind::Semi)
                || (self.peek_at(2) == Some(TokenKind::LParen) && self.is_sequence_decl_ahead()))
        {
            return self.parse_sequence_decl();
        }
        if self.at_kw(Kw::Property) {
            return self.parse_property_decl();
        }
        // N5: functional-coverage model `covergroup NAME; … endgroup`.
        if self.at_kw(Kw::Covergroup) {
            return self.parse_covergroup();
        }
        // §16.15 `default disable iff (expr);`. Checked BEFORE the `default clocking`
        // arm below so the two `default`-led items do not have to share a lookahead.
        if self.at_kw(Kw::Default)
            && matches!(
                self.peek_at(1),
                Some(TokenKind::Word(WordKind::Keyword(Kw::Disable)))
            )
        {
            self.bump(); // `default`
            self.bump(); // `disable`
            if self.at_ident_kw("iff") {
                self.bump();
            } else {
                self.error("`iff` after `default disable`");
            }
            self.expect(TokenKind::LParen, "'(' after `default disable iff`");
            let e = self.expr(0);
            self.expect(
                TokenKind::RParen,
                "')' after `default disable iff` condition",
            );
            self.expect(TokenKind::Semi, "';' after `default disable iff`");
            return Some(ModuleItem::DefaultDisableIff(e));
        }
        // N4: `clocking …` / `default clocking …` block (IEEE 1800 §14).
        if self.at_kw(Kw::Clocking)
            || (self.at_kw(Kw::Default)
                && matches!(
                    self.peek_at(1),
                    Some(TokenKind::Word(WordKind::Keyword(Kw::Clocking)))
                ))
        {
            return self.parse_clocking();
        }
        // N5: a covergroup INSTANCE `CG_TYPE NAME = new;` — distinguished from a module
        // instantiation (`CG_TYPE NAME ( … )`) by the `=` at lookahead 2. Placed before
        // the bare-ident instantiation arm.
        if self.is_ident()
            && matches!(
                self.peek_at(1),
                Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
            )
            && self.peek_at(2) == Some(TokenKind::Eq)
        {
            return self.parse_cover_instance();
        }
        // bare ident at module-item position ⇒ module instantiation.
        // (No keyword-led item matched above; in V2005 module scope a leading
        //  bare identifier can ONLY begin an instantiation — there is no
        //  bare-ident contassign/decl. The dispatch position itself is the
        //  disambiguation, so no multi-token lookahead is needed to decide.
        //  Gate PRIMITIVES (`and`/`or`/`buf`/…) are keyword-led, never reach
        //  this arm, and are not parsed in v1 — they fall through to the loud
        //  "expected module item" E2002 below.)
        if self.is_ident() {
            let module_name = self.ident().unwrap();
            return Some(ModuleItem::Instance(
                self.parse_module_instance(module_name),
            ));
        }
        // §4.5.428: an ELABORATION system task (IEEE §20.11) — `$fatal("…")` /
        // `$error` / `$warning` / `$info` as a module item (also inside a generate
        // branch: `if (W > 8) $fatal(1, "…");`). Carried as a synthetic `initial` whose
        // call is renamed under `ELAB_TASK_PREFIX`; elaborate runs it at elaboration.
        if self.peek() == Some(TokenKind::SystemTask) {
            let start = self.cur_span();
            let stmt = self.parse_systask_call();
            let Stmt::SysTaskCall { name, args, span } = stmt else {
                return Some(ModuleItem::Error(start));
            };
            let base = name.name.trim_start_matches('$');
            if !matches!(base, "info" | "warning" | "error" | "fatal") {
                self.error_at(
                    name.span,
                    "an elaboration system task as a module item (`$info` / `$warning` / \
                     `$error` / `$fatal`, IEEE 1800 §20.11)",
                );
                return Some(ModuleItem::Error(start));
            }
            let renamed = Ident {
                name: format!("{ELAB_TASK_PREFIX}{base}"),
                span: name.span,
            };
            return Some(ModuleItem::Proc(ProceduralBlock {
                kind: ProcKind::Initial,
                sensitivity: None,
                body: Box::new(Stmt::SysTaskCall {
                    name: renamed,
                    args,
                    span,
                }),
                span: start.to(self.prev_span()),
            }));
        }
        self.error("module item");
        None
    }
}

/// Does a deferred assertion in `s` declare anything in its action block?
fn action_declares(s: &Stmt) -> bool {
    fn declares(s: &Stmt) -> bool {
        match s {
            Stmt::Block { decls, stmts, .. } | Stmt::Fork { decls, stmts, .. } => {
                !decls.is_empty() || stmts.iter().any(declares)
            }
            Stmt::If { then_s, else_s, .. } => {
                declares(then_s) || else_s.as_deref().is_some_and(declares)
            }
            Stmt::Case { items, .. } => items.iter().any(|it| match it {
                CaseItem::Match { body, .. } | CaseItem::Default { body, .. } => declares(body),
            }),
            Stmt::For { body, .. }
            | Stmt::While { body, .. }
            | Stmt::Repeat { body, .. }
            | Stmt::Forever { body, .. } => declares(body),
            Stmt::DelayCtrl { body: Some(b), .. }
            | Stmt::EventCtrl { body: Some(b), .. }
            | Stmt::Wait { body: Some(b), .. } => declares(b),
            _ => false,
        }
    }
    match s {
        Stmt::DeferredAssert { then_s, else_s, .. } => declares(then_s) || declares(else_s),
        Stmt::Block { stmts, .. } => stmts.iter().any(action_declares),
        _ => false,
    }
}
