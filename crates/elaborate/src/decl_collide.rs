//! The ONE refusal for two DECLARATIONS of one name in a module, interface or
//! package scope, where the two declarations come from DIFFERENT binders.
//!
//! IEEE 1800-2017 §3.13: a module, interface or package body is one declarative
//! region with ONE name space for nets, variables, parameters, ports, genvars,
//! subroutines, instance names and named blocks, so a name is declared there
//! once. vita has a per-binder duplicate guard for each binder SEPARATELY —
//! `add_net` (`net_util.rs`) for net-vs-net, `param_dup.rs` for
//! parameter-vs-parameter — and nothing at all that compares two binders, which
//! is what this walk adds.
//!
//! Measured on the PRE binary (release, md5 ea25320c36c0afaa11dc29220ecce95e)
//! against iverilog 13.0 `-g2012` and Verilator 5.052 `--binary --timing`, both
//! of which REJECT every cell below while vita ran it at exit 0:
//!
//! * two `function int f` in one module / interface / package / transparent
//!   `generate` region — vita warned `W3056 function \`f\` redeclared; first
//!   declaration used` and then used the LAST body (`RD=49` where the first body
//!   answers 44), or said nothing at all in the package lane;
//! * `function int f` beside a `wire`/`logic`/`parameter`/`localparam`/port/
//!   `genvar`/instance/named-block `f`, in EITHER declaration order — vita
//!   printed `O=44`, i.e. the call always took the function and the other
//!   declaration silently kept its own storage (`call=44 net=z`);
//! * `genvar Q` beside a `parameter`/`localparam`/`wire`/second `genvar` Q, and
//!   `localparam Q` beside a `wire Q` — a genvar is recorded only in
//!   `genvar_decls` and a parameter only in `params`, so neither binder could
//!   ever see the other.
//!
//! ⚠️ The refusable pairs are not a hand-cut list any more: they are a MATRIX,
//! measured once over every unordered pair of the fourteen kinds this walk collects
//! (105 pairs, same-kind included). Each pair is one minimal module-scope design
//! declaring both kinds under ONE name, plus a control declaring them under two —
//! generator, designs and verbatim 3-tool logs under `impl/matrix/`. `R` = both
//! oracles reject, `L` = both run it, `S` = the oracles SPLIT. vita runs all 105
//! controls, so no pair is undecidable for want of one.
//!
//! | | fn | tk | net | var | par | lp | port | gv | inst | blk | gblk | tdef | cls | enum |
//! |---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
//! | **fn** | R | R | R | R | R | R | R | R | R | R | R | R | R | R |
//! | **tk** | R | R | R | R | R | R | R | R | R | R | R | R | R | R |
//! | **net** | R | R | R | R | R | R | L | R | R | R | R | R | R | R |
//! | **var** | R | R | R | R | R | R | L | R | R | R | R | R | R | R |
//! | **par** | R | R | R | R | R | R | S | R | R | R | R | R | R | R |
//! | **lp** | R | R | R | R | R | R | S | R | R | R | R | R | R | R |
//! | **port** | R | R | L | L | S | S | R | S | R | R | R | R | R | R |
//! | **gv** | R | R | R | R | R | R | S | R | R | R | R | R | R | R |
//! | **inst** | R | R | R | R | R | R | R | R | R | R | R | R | R | R |
//! | **blk** | R | R | R | R | R | R | R | R | R | R | R | R | R | R |
//! | **gblk** | R | R | R | R | R | R | R | R | R | R | R | R | R | R |
//! | **tdef** | R | R | R | R | R | R | R | R | R | R | R | R | R | R |
//! | **cls** | R | R | R | R | R | R | R | R | R | R | R | R | R | R |
//! | **enum** | R | R | R | R | R | R | R | R | R | R | R | R | R | R |
//!
//! Two `R` cells rest on a narrower footing than the rest, recorded rather than
//! hidden, and NEITHER changes the rule:
//! * **port × port** is `R` for the spelling the matrix measured — two body
//!   declarations, `module top(a); input a; input a;` — which both oracles reject.
//!   The REPEATED-HEADER spelling `module dut(a, a); input a;` is an oracle SPLIT
//!   (verilator rejects, iverilog runs it and prints a different value) and is left
//!   running: a non-ANSI header name is only pushed when the body declares no
//!   `PortDecl` for it, so both repeats drop and the pair never forms;
//! * **typedef × enum-label** and **class × enum-label** are `R` on verilator's
//!   diagnosis alone in substance — iverilog refuses them with a PARSE error on the
//!   line, which is a refusal but not a judgement about the name. Both tools do
//!   reject the file, which is what the `R` means here.
//!
//! 100 `R` · 2 `L` · 3 `S` · 0 undecidable. [`collides`] refuses the `R` cells MINUS
//! the two that another guard already reports — storage-vs-storage is `add_net`'s
//! `net/variable … redeclared` and parameter-vs-parameter is `param_dup.rs`'s
//! §6.20.1 / §27.2 / §26.2 sentence — because one defect gets one report.
//!
//! The `L` cells are the CONTROL this gate is measured against: a non-ANSI port and
//! its own `wire`/`logic` declaration are ONE declaration spelled in two items, and
//! all three tools run them. The `S` cells (a port beside a parameter, a localparam
//! or a genvar) are left on the route they had rather than picking a side.
//!
//! ⚠️ Two neighbours are outside the matrix because they are not pairs of ONE scope:
//! * a `$unit`-scope declaration beside a same-named one here is a §26.4 SHADOW, and
//!   both oracles run it. The parser prepends every `$unit` typedef / parameter /
//!   routine the unit does not itself declare to the front of `body`, and its own
//!   filter cannot see a block label, a generate label or an enum label — so the
//!   walk drops every site whose span lies outside `unit.span`;
//! * a routine's FORMALS and body locals are its own region and are never collected.
//!
//! ⚠️ A LABELLED `generate` block is TWO questions and they have different answers.
//! Its CONTENTS are a declarative region of its own (§27.3) and are never collected
//! here — `generate for (…) begin : g wire f; end` beside a module `function f`
//! RUNS in both oracles. Its LABEL is a declaration of the ENCLOSING region and both
//! oracles reject `generate if (1) begin : fn … end` beside `function int fn(…)`
//! ("Generate block has the same name as function: 'fn'"), so the label is collected
//! and the block is not descended into.
//!
//! ⚠️ One pair in the list has the oracles SPLIT, recorded rather than hidden: a
//! declaration inside an UNLABELLED `generate … endgenerate` is flattened into the
//! enclosing scope here, which is iverilog's model and vita's own — verilator gives
//! the unlabelled block a scope and RUNS `generate begin wire f; end` beside a module
//! `function f`. vita already enforced the flatten on the storage-vs-storage twin
//! before this slice (`add_net` refuses two unlabelled regions each declaring
//! `wire f`), so this follows vita's existing model rather than adding a second one.
//!
//! ⚠️ And two of the parser's OWN desugars must never look like a user duplicate, so
//! both are skipped BY IDENTITY, never by name shape:
//! * a `$`-PREFIXED name ([`internal_name`]) — every internal spelling the parser and
//!   elaborate mint is prefixed, and IEEE §5.6 lets no legal identifier start with
//!   `$`, so the test is exact;
//! * a `parameter type <stem>` CARRIER (`<stem>$w`, `<stem>$s`, `<stem>$d<i>a|b`,
//!   `<stem>$p<i>a|b`) — skipped ONLY when [`carrier_stems`] finds, among THIS unit's
//!   own parameter declarations, both `<stem>$w` and `<stem>$s`, which is what the
//!   producer always and only mints for a type parameter. A suffix-GRAMMAR skip was
//!   tried and was the same defect one narrowing later: `f$w`, `q$d0a` and `T$s` are
//!   legal user identifiers and both oracles reject a design that declares one twice;
//! * and a header ARRAY parameter, which arrives as a `ParamDecl` in `module.params`
//!   AND a `const_param` `NetVar` twin at the front of the body
//!   (`ParamItem::ConstArrayVar`) and would have read as parameter-vs-net.

use super::*;

/// The declarative region being judged — the word the refusal uses for it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum UnitKind {
    Module,
    Interface,
    Package,
}

impl UnitKind {
    pub(crate) fn word(self) -> &'static str {
        match self {
            UnitKind::Module => "module",
            UnitKind::Interface => "interface",
            UnitKind::Package => "package",
        }
    }

    /// `word` with its indefinite article — "an interface body", not "a
    /// interface body".
    fn a_word(self) -> &'static str {
        match self {
            UnitKind::Module => "a module",
            UnitKind::Interface => "an interface",
            UnitKind::Package => "a package",
        }
    }
}

/// What took the name. One variant per BINDER this walk collects; `word` is what
/// the refusal calls it, and `class` is the axis [`collides`] decides on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DeclKind {
    Function,
    Task,
    Net,
    Variable,
    Parameter,
    Localparam,
    Port,
    Genvar,
    Instance,
    NamedBlock,
    GenerateBlock,
    Typedef,
    Class,
    EnumLabel,
}

impl DeclKind {
    /// The word for this binder, with its indefinite article.
    fn word(self) -> &'static str {
        match self {
            DeclKind::Function => "a function",
            DeclKind::Task => "a task",
            DeclKind::Net => "a net",
            DeclKind::Variable => "a variable",
            DeclKind::Parameter => "a parameter",
            DeclKind::Localparam => "a localparam",
            DeclKind::Port => "a port",
            DeclKind::Genvar => "a genvar",
            DeclKind::Instance => "an instance",
            DeclKind::NamedBlock => "a named block",
            DeclKind::GenerateBlock => "a generate block",
            DeclKind::Typedef => "a typedef",
            DeclKind::Class => "a class",
            DeclKind::EnumLabel => "an enum label",
        }
    }

    /// The axis the pair rule is written on. Exhaustive on purpose: a new
    /// [`DeclKind`] must be given a class here, or this file does not compile —
    /// a new binder must never default into the quiet side of [`collides`].
    fn class(self) -> DeclClass {
        match self {
            DeclKind::Function | DeclKind::Task => DeclClass::Routine,
            DeclKind::Net | DeclKind::Variable => DeclClass::Storage,
            DeclKind::Parameter | DeclKind::Localparam => DeclClass::Param,
            DeclKind::Port => DeclClass::Port,
            DeclKind::Genvar => DeclClass::Genvar,
            DeclKind::Instance => DeclClass::Instance,
            DeclKind::NamedBlock | DeclKind::GenerateBlock => DeclClass::Block,
            DeclKind::Typedef | DeclKind::Class => DeclClass::Type,
            DeclKind::EnumLabel => DeclClass::EnumItem,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DeclClass {
    Routine,
    Storage,
    Param,
    Port,
    Genvar,
    Instance,
    Block,
    Type,
    EnumItem,
}

/// One declaration of one name, in source order.
struct DeclSite<'a> {
    name: &'a str,
    kind: DeclKind,
    span: ast::Span,
    /// Reached THROUGH a transparent `generate … endgenerate` region. The refusal
    /// then has to say WHY a line inside a generate counts as a module declaration
    /// — the same clause `param_dup.rs` gives its own §27.2 pair.
    in_region: bool,
}

/// Does a declaration of `a` forbid a second one of `b` in the same region?
///
/// A CLOSED list, and the whole gate: every `true` arm is a pair both oracles
/// reject on a measured cell, and every `false` arm is either a pair both oracles
/// ACCEPT (a port and its own net) or a pair whose duplicate guard already exists
/// elsewhere (`add_net` for storage-vs-storage, `param_dup.rs` for
/// parameter-vs-parameter). Spelled as an exhaustive match over the class pair so
/// a new class cannot fall into a catch-all.
fn collides(unit: UnitKind, a: DeclKind, b: DeclKind) -> bool {
    use DeclClass as C;
    let (ca, cb) = (a.class(), b.class());
    // A PACKAGE body's non-routine pairs are owned elsewhere or unmeasured: its
    // parameter-vs-variable duplicate is already `package.rs`'s E-DUP-UNIT and its
    // parameter-vs-parameter is `param_dup.rs`'s §26.2 lane, so a second report here
    // would be a second report of one defect. The routine pairs are this walk's, and
    // they are measured — see the module doc's matrix.
    if unit == UnitKind::Package && !matches!((ca, cb), (C::Routine, _) | (_, C::Routine)) {
        return false;
    }
    match (ca, cb) {
        // ── MEASURED LEGAL: both oracles RUN these ───────────────────────────────
        // A non-ANSI port and its own net/variable declaration are ONE declaration
        // spelled in two items (`module top(a); input a; wire a;`).
        (C::Storage, C::Port) | (C::Port, C::Storage) => false,
        // ── MEASURED SPLIT: iverilog rejects, verilator runs ─────────────────────
        // A port beside a parameter, a localparam or a genvar of its name. Left on
        // the route it had rather than picking a side.
        (C::Param, C::Port) | (C::Port, C::Param) => false,
        (C::Genvar, C::Port) | (C::Port, C::Genvar) => false,
        // ── OWNED BY ANOTHER GUARD: refused already, one defect one report ───────
        (C::Storage, C::Storage) => false, // `add_net`: `net/variable … redeclared`
        (C::Param, C::Param) => false,     // `param_dup.rs`: the §6.20.1/§27.2/§26.2 sentence
        // ── EVERY REMAINING PAIR: both oracles reject it ─────────────────────────
        // Spelled as an explicit product of the classes, not as `_`, so a NEW class
        // makes this match non-exhaustive and has to be placed deliberately — the
        // catch-all here is the REFUSING side, which is the dangerous one for a
        // reject gate.
        (
            C::Routine
            | C::Storage
            | C::Param
            | C::Port
            | C::Genvar
            | C::Instance
            | C::Block
            | C::Type
            | C::EnumItem,
            C::Routine
            | C::Storage
            | C::Param
            | C::Port
            | C::Genvar
            | C::Instance
            | C::Block
            | C::Type
            | C::EnumItem,
        ) => true,
    }
}

/// The `<stem>` of a `parameter type <stem>` CARRIER spelling, if `name` has a
/// carrier SUFFIX at all. Producer census (`crates/hdl-parser`, every `format!`
/// that builds a declaration name holding a `$`):
/// * `<stem>$w` / `<stem>$s` — width and shape (`type_params.rs:169/:170`, `:933/:941`);
/// * `<stem>$d<i>a` / `<stem>$d<i>b` — unpacked dims (`type_params.rs:369/:370`, `:951`);
/// * `<stem>$p<i>a` / `<stem>$p<i>b` — packed dims (`type_param_packed.rs:60/:61`, `:191`).
///
/// ⚠️ A SHAPE test, never an identity. `f$w`, `q$d0a` and `T$s` are legal simple
/// identifiers (IEEE 1800-2017 §5.6) and a user writes them; the caller must ask
/// [`carrier_stems`] whether the stem is a type parameter of THIS unit before
/// skipping anything.
pub(crate) fn carrier_stem(name: &str) -> Option<&str> {
    let (stem, suf) = name.rsplit_once('$')?;
    if stem.is_empty() {
        return None;
    }
    let ok = suf == "w" || suf == "s" || {
        let b = suf.as_bytes();
        b.len() >= 3
            && matches!(b[0], b'd' | b'p')
            && matches!(b[b.len() - 1], b'a' | b'b')
            && b[1..b.len() - 1].iter().all(u8::is_ascii_digit)
    };
    ok.then_some(stem)
}

/// The stems this declaration list PROVES are `parameter type` declarations of the
/// unit — the identity test, and the only thing that may exempt a `$` name.
///
/// The producer mints `<stem>$w` AND `<stem>$s` together, always and only for a
/// type parameter (`type_params.rs:169-170`, `:933/:941`), and the dim carriers are
/// minted only for a stem that already got those two. So the PAIR, among this
/// unit's own parameter declarations, is what says "the parser wrote this name".
///
/// ⚠️ This replaced a suffix-GRAMMAR skip, which was the same defect one narrowing
/// later: it exempted every user identifier of that shape, so two `function int
/// f$w()` in one module went from a W3056 warning to NO diagnostic while still
/// answering the second body, and the shipped net-vs-routine pair accepted
/// `wire T$s` + `function int T$s()` — both rejected by iverilog ("'f$w' has
/// already been declared in this scope.") and verilator ("Duplicate declaration of
/// function: 'f$w'"). Under-detection is only the safe side while the diagnostic it
/// replaces still exists, and this slice deleted that one.
pub(crate) fn carrier_stems<'a>(param_names: impl Iterator<Item = &'a str>) -> BTreeSet<&'a str> {
    let (mut w, mut sh) = (BTreeSet::new(), BTreeSet::new());
    for n in param_names {
        if let Some(stem) = n.strip_suffix("$w") {
            w.insert(stem);
        } else if let Some(stem) = n.strip_suffix("$s") {
            sh.insert(stem);
        }
    }
    w.intersection(&sh).copied().collect()
}

/// A name the PARSER minted, by IDENTITY rather than by shape: every internal
/// spelling it and elaborate produce is `$`-PREFIXED (`$break$<lo>`,
/// `$continue$<lo>`, `$enum_name$<E>`, `$unp$<var>$<field>`, and the `$blk$` /
/// `$func$` / `$itask$` / `$pkg$` scope segments), and IEEE §5.6 lets no legal
/// identifier start with `$` — so this half is exact. The CARRIER half is
/// [`carrier_stems`], which needs the unit's own declarations and is applied by the
/// caller.
pub(crate) fn internal_name(name: &str) -> bool {
    name.starts_with('$')
}

fn push<'a>(out: &mut Vec<DeclSite<'a>>, id: &'a ast::Ident, kind: DeclKind, in_region: bool) {
    if internal_name(&id.name) {
        return;
    }
    out.push(DeclSite {
        name: &id.name,
        kind,
        span: id.span,
        in_region,
    });
}

fn param_kind(p: &ast::ParamDecl) -> DeclKind {
    match p.kind {
        ast::ParamKind::Parameter => DeclKind::Parameter,
        ast::ParamKind::Localparam => DeclKind::Localparam,
    }
}

/// Net versus variable, read off the declaration's own kind so the refusal uses
/// the word the user would.
fn netvar_kind(d: &ast::NetVarDecl) -> DeclKind {
    if d.kind.is_net() {
        DeclKind::Net
    } else {
        DeclKind::Variable
    }
}

/// The declarations a MODULE-ITEM sequence contributes to ITS OWN region, in
/// source order. A TRANSPARENT `generate … endgenerate` region contributes at the
/// position of its `generate` item (IEEE §27.2 gives it no scope of its own) —
/// the same flatten [`Elaborator::scope_param_decls`] already performs.
fn collect_items<'a>(
    items: &'a [ast::ModuleItem],
    unit: UnitKind,
    rgn: bool,
    out: &mut Vec<DeclSite<'a>>,
) {
    for it in items {
        collect_item(it, unit, rgn, out);
    }
}

fn collect_item<'a>(
    it: &'a ast::ModuleItem,
    unit: UnitKind,
    rgn: bool,
    out: &mut Vec<DeclSite<'a>>,
) {
    match it {
        ast::ModuleItem::Func(f) => push(out, &f.name, DeclKind::Function, rgn),
        ast::ModuleItem::Task(t) => push(out, &t.name, DeclKind::Task, rgn),
        ast::ModuleItem::Param(p) => push(out, &p.name, param_kind(p), rgn),
        ast::ModuleItem::NetVar(d) => {
            // The parser's header-ARRAY-parameter twin (`ParamItem::ConstArrayVar`)
            // carries the same name as the `ParamDecl` it was desugared beside;
            // counting it would refuse `module top #(parameter int A[2] = '{1,2})`.
            if d.const_param {
                return;
            }
            for n in &d.names {
                push(out, &n.name, netvar_kind(d), rgn);
            }
        }
        // A package body declares TYPE names and enum labels exactly as a module body
        // does (§26.2 makes it one declarative region), so these arms sit ABOVE the
        // package guard: `package pk; typedef enum int { fn = 7 } e_t; function int
        // fn(); endpackage` ran and printed a value where iverilog says "'fn' has
        // already been declared in this scope … as an enum type or value" and
        // verilator "Function has the same name as ENUMITEM 'fn'" — while the MODULE
        // twin of that pair was already refused. [`collides`] keeps a package to its
        // routine pairs, so nothing else opens with them.
        ast::ModuleItem::Typedef(td) => {
            push(out, &td.name, DeclKind::Typedef, rgn);
            // IEEE §6.19: an enum's LABELS are declared in the scope that holds the
            // typedef, not inside the type. `typedef enum int { fn = 7, gg = 8 } e_t;`
            // beside `function int fn(…)` ran and printed a value where iverilog says
            // "'fn' has already been declared in this scope … as an enum type or
            // value" and verilator "Function has the same name as ENUMITEM 'fn'".
            if let ast::TypedefKind::Enum { labels, .. } = &td.kind {
                for l in labels {
                    push(out, &l.name, DeclKind::EnumLabel, rgn);
                }
            }
        }
        ast::ModuleItem::Class(c) => push(out, &c.name, DeclKind::Class, rgn),
        // Everything below is a MODULE/INTERFACE binder: a package body has no ports,
        // genvars, instances, procedural blocks or generate regions.
        _ if unit == UnitKind::Package => {}
        ast::ModuleItem::PortDecl(pd) => {
            for n in &pd.names {
                push(out, n, DeclKind::Port, rgn);
            }
        }
        ast::ModuleItem::Genvar { names, .. } => {
            for n in names {
                push(out, n, DeclKind::Genvar, rgn);
            }
        }
        ast::ModuleItem::Instance(mi) => {
            for inst in &mi.instances {
                push(out, &inst.name, DeclKind::Instance, rgn);
            }
        }
        // The label of a procedural block written DIRECTLY in this region
        // (`initial begin : f`, `initial fork : f`). A block nested inside another
        // block, or inside a routine body, belongs to that block's own scope and is
        // not collected. BOTH block forms: iverilog rejects a `fork : f` beside
        // `function f` with "'f' has already been declared in this scope. : It was
        // declared here as a named block." exactly as it rejects the `begin : f`
        // twin, and verilator says "… same name as FORK 'f'".
        ast::ModuleItem::Proc(p) => {
            if let ast::Stmt::Block { label: Some(l), .. }
            | ast::Stmt::Fork { label: Some(l), .. } = p.body.as_ref()
            {
                push(out, l, DeclKind::NamedBlock, rgn);
            }
        }
        ast::ModuleItem::Generate(g) => {
            collect_gen_items(&g.items, unit, rgn || explicit_generate_region(g), out)
        }
        _ => {}
    }
}

/// Was this `ModuleItem::Generate` written as an explicit `generate … endgenerate`
/// REGION, or is it the parser's wrapper around a bare module-level `if` / `for` /
/// `case` (`module_items.rs:1339-1346`)?
///
/// The AST carries no keyword flag, but the SPANS answer it exactly: the explicit
/// form starts at the `generate` token, strictly before its first gen-item
/// (`generate.rs:38` takes `start` and then bumps), while the bare form's construct
/// span IS its single item's (`let start = self.cur_span()` at the `if`). It matters
/// only for the SENTENCE: `if (1) begin : fn … end` beside `function int fn()` is
/// refused either way and both oracles reject both spellings, but telling the reader
/// about a "`generate … endgenerate` region" the file does not contain sends them
/// looking for one.
fn explicit_generate_region(g: &ast::GenerateConstruct) -> bool {
    match g.items.first() {
        Some(
            ast::GenItem::For { span, .. }
            | ast::GenItem::If { span, .. }
            | ast::GenItem::Case { span, .. }
            | ast::GenItem::Block { span, .. },
        ) => span.lo > g.span.lo,
        // A plain module item at the top of a construct only ever comes from an
        // explicit region: the bare spelling parses exactly one `if`/`for`/`case`.
        Some(ast::GenItem::Item(_)) | None => true,
    }
}

/// The GEN-ITEM twin of [`collect_items`]. TRANSPARENCY is the whole rule and it
/// is the same one `param_dup.rs` reads off `elaborate_gen_item`: a
/// `GenItem::Item` is a plain module item of the enclosing region, a nested
/// `generate` region is another transparent one, and an UNLABELLED `begin…end` in
/// a gen-item list is the anachronistic surround. `for` / `if` / `case` / a
/// LABELLED block each mint a scope of their own (§27.3) and are skipped.
fn collect_gen_items<'a>(
    items: &'a [ast::GenItem],
    unit: UnitKind,
    rgn: bool,
    out: &mut Vec<DeclSite<'a>>,
) {
    for it in items {
        match it {
            ast::GenItem::Item(b) => collect_item(b.as_ref(), unit, rgn, out),
            ast::GenItem::Block {
                label: None, items, ..
            } => collect_gen_items(items, unit, rgn, out),
            // A LABELLED block's CONTENTS are its own declarative region (§27.3) and
            // are never collected here — but its LABEL is a declaration of the
            // ENCLOSING one, and both oracles say so: `generate if (1) begin : fn …`
            // beside `function int fn(…)` ran and printed a value where iverilog says
            // "'fn' has already been declared in this scope … declared here as a
            // function" and verilator "Generate block has the same name as function:
            // 'fn'". Label only, no recursion — `generate for (…) begin : g wire f;`
            // beside a module `function f` runs in both oracles and must keep running.
            // §27.6: a generate block's LABEL is declared in the ENCLOSING scope
            // whether or not a `generate … endgenerate` wraps it, so it carries the
            // enclosing construct's own flag and not a blanket `true`.
            ast::GenItem::Block { label: Some(l), .. }
            | ast::GenItem::For { label: Some(l), .. }
            | ast::GenItem::If { label: Some(l), .. } => push(out, l, DeclKind::GenerateBlock, rgn),
            _ => {}
        }
    }
}

impl Elaborator<'_> {
    /// Refuse every name this unit declares twice with two different binders.
    ///
    /// Per DEFINITION, from `driver.rs::run`, not per instance: the defect is a
    /// property of the source, a module instantiated N times must say it once, and
    /// a module or interface that is never instantiated (or is named only under
    /// `generate if (0)`) is never elaborated at all — which is exactly where the
    /// P25(a) census found vita silent where both oracles reject.
    pub(crate) fn check_decl_name_collisions(&mut self, unit: &ast::ModuleDecl, kind: UnitKind) {
        let mut sites: Vec<DeclSite<'_>> = Vec::new();
        if kind != UnitKind::Package {
            for p in &unit.params {
                push(&mut sites, &p.name, param_kind(p), false);
            }
            match &unit.ports {
                ast::PortList::Ansi(ps) => {
                    for p in ps {
                        push(&mut sites, &p.name, DeclKind::Port, false);
                    }
                }
                // The bare header names of a non-ANSI list, and ONLY those the body
                // does not declare. A non-ANSI port is ONE declaration spelled in two
                // items — the header name and its `input`/`output` `PortDecl` — so
                // pushing both made every non-ANSI port in the corpus a port-vs-port
                // duplicate the moment the matrix put that pair in the refusal set
                // (`module top(a); input a;` refused, where all three tools run it).
                // A header name with no body declaration IS the whole declaration (an
                // implicit `wire`), so it still counts.
                ast::PortList::NonAnsi(ids) => {
                    let declared: BTreeSet<&str> = unit
                        .body
                        .iter()
                        .filter_map(|it| match it {
                            ast::ModuleItem::PortDecl(pd) => Some(pd),
                            _ => None,
                        })
                        .flat_map(|pd| pd.names.iter().map(|n| n.name.as_str()))
                        .collect();
                    for id in ids {
                        if !declared.contains(id.name.as_str()) {
                            push(&mut sites, id, DeclKind::Port, false);
                        }
                    }
                }
                ast::PortList::None => {}
            }
        }
        collect_items(&unit.body, kind, false, &mut sites);
        // §26.4: the parser PREPENDS every `$unit`-scope typedef / parameter /
        // function / task this unit does not itself declare to the front of `body`,
        // and a `$unit` declaration is a SHADOW of a same-named one here, never a
        // duplicate — both oracles run `function int f(); endfunction  module top;
        // initial begin : f … end endmodule`. The parser's own filter keys on the
        // names it can see (`local_decl_names`), which holds no block label, no
        // generate label and no enum label, so a prepended item reached this walk and
        // was refused. The discriminator is the one fact the AST already carries:
        // a declaration written in THIS unit lies inside the unit's own span
        // (`module … endmodule`, `parser: start.to(prev_span)`), a prepended one does
        // not.
        sites.retain(|s| s.span.lo >= unit.span.lo && s.span.hi <= unit.span.hi);
        // The CARRIER half of the skip, by identity: drop `<stem>$…` only where this
        // unit's own parameter declarations prove `<stem>` is a `parameter type`.
        // Done here rather than in `push` because the proof needs the whole list.
        let stems = carrier_stems(
            sites
                .iter()
                .filter(|s| matches!(s.kind, DeclKind::Parameter | DeclKind::Localparam))
                .map(|s| s.name),
        );
        if !stems.is_empty() {
            sites.retain(|s| !carrier_stem(s.name).is_some_and(|st| stems.contains(st)));
        }
        self.report_decl_collisions(&sites, kind);
    }

    /// One report per NAME — the FIRST refusable pair of its declarations, in
    /// source order.
    ///
    /// Per name rather than per pair because a name can be declared three times
    /// (a non-ANSI header name, its `PortDecl` and a function) and the reader
    /// needs the defect once, at the second declaration, with the first one in a
    /// note — which is where both oracles point.
    fn report_decl_collisions(&mut self, sites: &[DeclSite<'_>], unit: UnitKind) {
        let mut by_name: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (i, s) in sites.iter().enumerate() {
            by_name.entry(s.name).or_default().push(i);
        }
        let mut reports: Vec<(usize, usize)> = Vec::new();
        for idxs in by_name.values() {
            'name: for (nth, &j) in idxs.iter().enumerate().skip(1) {
                for &i in &idxs[..nth] {
                    if collides(unit, sites[i].kind, sites[j].kind) {
                        reports.push((i, j));
                        break 'name;
                    }
                }
            }
        }
        // Source order, so a design with two defects reads top to bottom.
        reports.sort_by_key(|&(_, j)| (sites[j].span.lo, sites[j].span.hi));
        let word = unit.word();
        let a_unit = unit.a_word();
        for (first, dup) in reports {
            let d = &sites[dup];
            let f = &sites[first];
            if !self.reported_decl_collisions.insert((d.span.lo, d.span.hi)) {
                continue;
            }
            // The RULE clause. A pair either side of which came through a transparent
            // `generate … endgenerate` needs the §27.2 reason too, or the message
            // cites a line the reader can see is inside a generate and says only
            // "in this module" — which is the same gap `param_dup.rs` closed for its
            // own §27.2 pair, and this walk must not disagree with it about one shape.
            let rule = if f.in_region || d.in_region {
                format!(
                    "a `generate … endgenerate` region with no block label is \
                     TRANSPARENT — its declarations belong to the enclosing {word} \
                     scope (IEEE 1800-2017 §27.2/§27.3), which is ONE name space \
                     (§3.13), so a name is declared there once"
                )
            } else {
                format!(
                    "{a_unit} body is ONE name space (IEEE 1800-2017 §3.13), so a \
                     name is declared there once"
                )
            };
            // Two declarations of ONE kind read as a stutter in the two-kind
            // sentence ("as a port and as a port"), so they get their own.
            let what = if f.kind.word() == d.kind.word() {
                format!(", both times as {}", d.kind.word())
            } else {
                format!(": as {} and as {}", f.kind.word(), d.kind.word())
            };
            let msg = format!(
                "`{}` is declared twice in this {word}{what} — {rule}. Rename one of them",
                d.name
            );
            self.error_at(MsgCode::ElabUnsupported, d.span, &msg);
            self.note_at(
                MsgCode::ElabUnsupported,
                f.span,
                "the first declaration of that name is here",
            );
        }
    }

    /// `recv.member(args)` where `recv` is an INTERFACE INSTANCE and `member` is
    /// one of that interface's MODPORTS — refuse the CALL. Returns whether it did.
    ///
    /// The declaration pair is NOT refused here: `import pk::mp;` (or
    /// `import pk::*;`) beside `modport mp` is accepted by verilator, and the
    /// wildcard spelling is accepted by BOTH oracles (census p22_a2, p22_b2), so a
    /// refusal at the interface body would be a false loud. The CALL is where the
    /// two oracles agree: `w.mp(40)` is "Found definition of 'w.mp' as a MODPORT
    /// but expected a task/function" in verilator and "No function named `w.mp'
    /// found in this context" in iverilog, while vita resolved the dotted name to
    /// the IMPORTED routine and printed `R=44` (census p22_b).
    ///
    /// The interface's OWN `function mp` beside `modport mp` is a different
    /// question and already refused at the declaration (`iface_inst.rs`, §4.5.518):
    /// there both oracles reject the declaration itself.
    pub(crate) fn modport_call_refused(&mut self, path: &ast::HierPath) -> bool {
        if path.segments.len() != 2 {
            return false;
        }
        let recv = path.segments[0].name.clone();
        let member = &path.segments[1].name;
        // The same outward scope walk interface-PORT binding uses, so an instance
        // declared in an enclosing scope resolves exactly as it does there.
        let Some(fq) = self.walk_scopes_key(&recv, |k| self.iface_insts.contains_key(k)) else {
            return false;
        };
        let iface = self.iface_insts[&fq].clone();
        let is_modport = self.ifaces.get(&iface).is_some_and(|d| {
            d.body
                .iter()
                .any(|it| matches!(it, ast::ModuleItem::Modport(m) if &m.name.name == member))
        });
        if !is_modport {
            return false;
        }
        let msg = format!(
            "`{recv}.{member}` names a modport of interface `{iface}`, not a task/function \
             — a modport and a subroutine share one name space (IEEE 1800-2017 §25.5, \
             §3.13), so this call has no callee"
        );
        self.error_at(MsgCode::ElabUnsupported, path.span, &msg);
        true
    }
}
