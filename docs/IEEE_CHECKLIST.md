# IEEE 1800-2017 checklist

The whole language, top-down in the standard's clause order, as short titles with check boxes.
Read from the docs only (`docs/manual/`, `docs/ROADMAP.md` and `docs/roadmap/`, `docs/PROBE_CATALOG.md`)
on 2026-11-17 at commit 085df910 (branch `roadmap-s1`); nothing was built or run for it.

The rule of a box:

- `[x]` = the manual records the item as supported and no open roadmap entry records a refusal or a wrong value on it.
- `[ ]` = anything else — refused, partly supported, an open entry against it, or not recorded — followed by the reason
  in a few words and the roadmap id where one exists (`— §5.2 Q-X`, `— §2 <slug>`, `— not filed`, `— status not recorded`).
- Never check a box by guess: no record = empty. A partly supported subclause is split into leaves until each leaf is
  one or the other, by the manual's own distinctions or by the form an entry names.

How it is kept: a landing's docs commit re-reads the lines of the clauses its rows touch; the counts below are
recomputed then, from the boxes of this file. General, overview, informative and example subclauses carry no leaf;
3.3–3.11, Annex A and Annex F carry none either, since their leaves are in the clauses they name. Batch 5
(PARSE-1's piece, Q-WREXT) landed on 2026-11-18 (§4.5.654), structure only: its re-read of the 5.7.1, 6.5, 6.20, 9.2.2.2,
10.3, 10.4, 23.3.3.2 and 26.3 lines moved no box. Batch 6's docs (§4.5.655, 2026-11-21) re-read 3.12.1, 5.7.1, 6.5, 6.16,
6.19, 6.20.6, 6.23, 8.25, 9.2.2.2–9.2.2.4, 9.3.1, 9.4.5, 10.3.2, 10.6.1, 10.6.2, 11.12, 13.3, 20.7, 23.3.2, 23.9 and 29,
then 6.9.1, 6.12, 7.5, 8.4, 8.5, 9.4.2.3, 10.4.2, 10.9.1, 11.3.1, 11.4.12, 11.5.1, 12.7.1 and 23.3.2.2: ten boxes emptied
on PRE == POST filings' cells (9.4.5, 11.12, 29.3.1, 29.7, 29.8, 6.19.1, 8.25, 13.3, 10.4.2, 10.9.1), none checked.
Batch 6 (Q-WEXTPOS alone) landed on 2026-11-21 (§4.5.655), structure only; Q-MDRVMIX (2026-11-20, behind Q-SCOPEBIND) and Q-LOSSYSPELL (2026-11-21, behind
Q-FILLCTX) are parked: an item only one of them completes stays empty. 23.3.2.2 and 28.3.3 stay checked, each with a reason line
naming its illegal accept (an accept is neither a refusal nor a wrong value; main, W/batch6/RULINGS.txt L2-78 (e)).

| clause | title | checked / leaves |
|---|---|---|
| 3 | Design and verification building blocks | 2 / 12 |
| 4 | Scheduling semantics | 13 / 26 |
| 5 | Lexical conventions | 16 / 29 |
| 6 | Data types | 32 / 116 |
| 7 | Aggregate data types | 22 / 69 |
| 8 | Classes | 8 / 30 |
| 9 | Processes | 19 / 49 |
| 10 | Assignment statements | 6 / 25 |
| 11 | Operators and expressions | 22 / 57 |
| 12 | Procedural programming statements | 12 / 26 |
| 13 | Tasks and functions (subroutines) | 4 / 20 |
| 14 | Clocking blocks | 3 / 15 |
| 15 | Interprocess synchronization and communication | 2 / 8 |
| 16 | Assertions | 11 / 64 |
| 17 | Checkers | 0 / 3 |
| 18 | Constrained random value generation | 10 / 40 |
| 19 | Functional coverage | 8 / 27 |
| 20 | Utility system tasks and system functions | 10 / 32 |
| 21 | Input/output system tasks and system functions | 21 / 53 |
| 22 | Compiler directives | 5 / 15 |
| 23 | Modules and hierarchy | 12 / 42 |
| 24 | Programs | 0 / 7 |
| 25 | Interfaces | 4 / 16 |
| 26 | Packages | 3 / 15 |
| 27 | Generate constructs | 3 / 9 |
| 28 | Gate-level and switch-level modeling | 6 / 21 |
| 29 | User-defined primitives | 10 / 14 |
| 30 | Specify blocks | 0 / 5 |
| 31 | Timing checks | 0 / 7 |
| 32 | Backannotation using the standard delay format | 0 / 6 |
| 33 | Configuring the contents of a design | 0 / 4 |
| 34 | Protected envelopes | 0 / 3 |
| 35 | Direct programming interface | 0 / 7 |
| 36 | Programming language interface (PLI/VPI) overview | 0 / 5 |
| 37 | VPI object model diagrams | 0 / 1 |
| 38 | VPI routine definitions | 0 / 1 |
| 39 | Assertion API | 0 / 3 |
| 40 | Code coverage control and API | 0 / 3 |
| 41 | Data read API | 0 / 1 |
| Annex | Annexes | 0 / 12 |
| total | | 264 / 898 |

## 3 Design and verification building blocks

**3.3–3.11 Building blocks** (no leaf here: modules 23, programs 24, interfaces 25, checkers 17, primitives 28–29,
subroutines 13, packages 26, configurations 33, hierarchy 23)

**3.12 Compilation and elaboration**
- [ ] 3.12.1 Compilation-unit scope declarations — unit variables, forward constants; §3.a ⑤ⓝ
- [ ] 3.12.1 Compilation-unit routines — fold at the calling module; §2 🆕 AF, §5.2 4, §3.b unit-routine-bare-name
- [ ] 3.12.1 `$unit::` scope prefix — absent; §3.a ⑤ⓝ
- [ ] 3.12.1 Compilation-unit and module imports — one import scope; §2 unit-module-import-scope

**3.13 Name spaces**
- [ ] 3.13 Declaration name collisions in one scope — some pairs accepted; §2 duplicate-value-name-accepted, §5.2 Q-TDREDECL

**3.14 Simulation time units and precision**
- [ ] 3.14.1 Time value rounding — rounding precision across timescales; §2 multi-timescale-rounding
- [x] 3.14.2.1 `` `timescale `` directive
- [ ] 3.14.2.1 Some units without a timescale — warns where IEEE requires an error; §3.b timescale
- [ ] 3.14.2.2 `timeunit` and `timeprecision` — absent; not filed
- [ ] 3.14.2.3 Precedence of timeunit, timeprecision and `` `timescale `` — no timeunit; not filed
- [x] 3.14.3 Simulation time unit (global precision)
- [ ] 3.14.3 `1step` outside a clocking skew — status not recorded

## 4 Scheduling semantics

**4.3 Event simulation**
- [x] 4.3 Update events and evaluation events

**4.4 Stratified event scheduler**
- [ ] 4.4.1 Reactive region set — not modelled; not filed
- [ ] 4.4.2.1 Preponed region — clocking sampling only; not filed
- [x] 4.4.2.2 Active events region
- [x] 4.4.2.3 Inactive events region
- [x] 4.4.2.4 NBA events region
- [x] 4.4.2.5 Observed events region
- [ ] 4.4.2.6 Reactive events region — program processes run in Active; not filed
- [ ] 4.4.2.7 Re-Inactive events region — not modelled; not filed
- [ ] 4.4.2.8 Re-NBA events region — not modelled; not filed
- [x] 4.4.2.9 Postponed events region
- [ ] 4.4.3 PLI regions — no PLI; no roadmap row

**4.5 SystemVerilog simulation reference algorithm**
- [ ] 4.5 Reference algorithm — seven of seventeen regions modelled; not filed

**4.6–4.7 Determinism**
- [x] 4.6 Determinism (statement order, NBA order)
- [x] 4.7 Nondeterminism (one fixed order chosen)

**4.9 Scheduling implication of assignments**
- [x] 4.9.1 Continuous assignment
- [ ] 4.9.1 Continuous-assign update order in a batch — lands after the batch; §2 ca-hop-after-batch
- [ ] 4.9.2 Procedural continuous assignment — `force` samples once; §5.2 Q-RELEVAL, §5.2 Q-RELIMM
- [x] 4.9.3 Blocking assignment
- [x] 4.9.4 Nonblocking assignment
- [ ] 4.9.5 Switch (transistor) processing — no switch primitives; not filed
- [x] 4.9.6 Port connections: input and output
- [ ] 4.9.6 Port connections: inout — one-directional, parent to child; not filed
- [x] 4.9.7 Subroutines
- [ ] 4.9.7 Subroutine activations sharing state — §5.2 Q-CAPLIFE, §5.2 Q-STATICFRAME-WAIT

**4.10 PLI callback control points**
- [ ] 4.10 PLI callback control points — no PLI; no roadmap row

## 5 Lexical conventions

**5.2–5.5 Tokens**
- [x] 5.2 Lexical tokens
- [x] 5.3 White space
- [x] 5.4 Comments
- [x] 5.5 Operators

**5.6 Identifiers, keywords, and system names**
- [x] 5.6 Simple identifiers
- [ ] 5.6.1 Escaped identifiers — backslash kept in the name; §5.2 Q-ESCSCOPE, §5.2 Q-ESCTDKEEP
- [x] 5.6.2 Keywords (case-sensitive, reserved)
- [x] 5.6.3 System tasks and system functions (names)
- [x] 5.6.4 Compiler directives (names)

**5.7 Numbers**
- [x] 5.7.1 Sized based literals
- [ ] 5.7.1 White space inside a based literal — `64'sh FFFF` refused; §3.b based-ws
- [ ] 5.7.1 Unsized based literals — x / z not extended past 32 bits; §5.2 Q-UNSZXPAD
- [ ] 5.7.1 Unbased unsized `'0 '1 'x 'z` — widths in some positions; `'s1` accepted; §2 🆕 J, §2 🆕 M, §3.b signed-unbased-literal
- [x] 5.7.1 Unsized decimal literals
- [x] 5.7.1 `x`, `z` and `?` digits
- [ ] 5.7.1 Underscore separators — one reader keeps them; §2 sized-literal-separator-read
- [x] 5.7.2 Real literal constants

**5.8 Time literals**
- [ ] 5.8 Time literals in delays — sub-precision rounding; §2 delay-rounding-leaf
- [ ] 5.8 Time literals in other expressions — fold as integers; §5.2 Q-TIMELITCONST, §2 runtime-time-literal

**5.9 String literals**
- [ ] 5.9 String literal value and width — empty literal is 1 bit, NUL kept raw; §5.2 Q-STRWIDTH, §2 string-nul-byte
- [x] 5.9.1 Special characters in strings (Table 5-1 escapes)
- [ ] 5.9.1 Non-standard escapes (`\r`, `\X`) — warn, tools differ; not filed
- [ ] 5.9 Non-UTF-8 bytes in a source string — refused; §5.2 Q-SRCBYTES

**5.10–5.13 Literals, attributes, methods**
- [x] 5.10 Structure literals (positional, named, default)
- [ ] 5.10 Structure literal with no target record — refused; §5.2 Q-PATNOREC
- [x] 5.11 Array literals (positional, default)
- [ ] 5.11 Replication inside an array literal — refused at parse; §5.2 35
- [ ] 5.12 Attributes — ignored; not filed
- [x] 5.13 Built-in method call syntax

## 6 Data types

**6.3 Value set**
- [x] 6.3.1 Logic values (0, 1, x, z)
- [ ] 6.3.2.1 Charge strength — trireg out of scope; §8 non-goal-list
- [ ] 6.3.2.2 Drive strength — refused at parse; §3.b decl-var-net-kw

**6.5 Nets and variables**
- [x] 6.5 Whole-net continuous drivers resolved
- [ ] 6.5 Overlapping partial or delayed net drivers — false E3001; §3.b E3001-delayed, §7 60, §5.2 Q-MDPORTCOL
- [ ] 6.5 Variable written by one continuous driver only — rules partly read (whole writes only; the partial rule parked); §5.2 Q-MDRVMIX, §5.2 Q-MIXCAPROC, §3.b mdrv-unlisted-writers
- [ ] 6.5 Procedural write to a net — an input-port net accepted; §3.b proc-write-input-net
- [ ] 6.5 Net-array word with two drivers — neither refused nor resolved; §5.2 Q-ARRWORDMD

**6.6 Net types**
- [x] 6.6.1 `wire` nets
- [ ] 6.6.1 `tri` nets — no strength model; not filed
- [ ] 6.6.2 Unresolved nets (`uwire`) — behaves as wire; not filed
- [x] 6.6.3 Wired nets `wand`, `wor`
- [ ] 6.6.3 Wired nets `triand`, `trior` — refused; not filed
- [ ] 6.6.4 Trireg net — §8 non-goal-list
- [ ] 6.6.5 Tri0 and tri1 nets — refused; not filed
- [ ] 6.6.6 Supply nets — refused; not filed
- [ ] 6.6.7 User-defined nettypes — absent; not filed
- [ ] 6.6.8 Generic interconnect — status not recorded

**6.7 Net declarations**
- [x] 6.7.1 Net declarations with built-in net types
- [ ] 6.7.1 Net with an explicit data type (`wire logic`) — refused at parse; §3.b decl-var-net-kw
- [ ] 6.7.1 Net declaration delay without an assignment — ignored silently; §5.2 Q-NETDLY
- [ ] 6.7.2 Net declarations with user-defined nettypes — absent; not filed
- [ ] 6.7.3 Initialization of nets with user-defined nettypes — absent; not filed

**6.8 Variable declarations**
- [x] 6.8 Variable declarations with a data type
- [ ] 6.8 The `var` keyword — refused at parse; §3.b decl-var-net-kw

**6.9 Vector declarations**
- [x] 6.9.1 Vectors with literal non-negative bounds
- [ ] 6.9.1 Negative range bounds — §0 11, §3.b neg-ascending, §3.b neg-bound-part
- [ ] 6.9.2 Vector net accessibility (`vectored`, `scalared`) — refused at parse; not filed

**6.10 Implicit declarations**
- [x] 6.10 Implicit nets at module scope (assign target, port connection)
- [ ] 6.10 Implicit nets inside a generate block — refused; §3.b implicit-net-generate

**6.11 Integer data types**
- [x] 6.11.1 Integral types
- [ ] 6.11.2 2-state types — stored 4-state in places; §5.2 Q-PARAM2STATE, §5.2 Q-2STINVSEL, §5.2 8
- [x] 6.11.2 4-state types
- [x] 6.11.3 Signed and unsigned integer types (variables)

**6.12 Real, shortreal, and realtime data types**
- [x] 6.12 `real` and `realtime` variables
- [ ] 6.12 `shortreal` — §5.2 Q-SHORTREAL
- [ ] 6.12 `real unsigned` — accepted; §2 real-unsigned-accepted
- [ ] 6.12 Part-select write to a scalar `real` — ignored silently; §3.b real-part-write
- [ ] 6.12.1 Operators and real numbers — `$signed` of a real accepted; §2 signed-of-real-accepted
- [ ] 6.12.2 Conversion — integral to real at a wrong width; §5.2 Q-CAREALCTX, §5.2 Q-RDYNELEM, §5.2 Q-FRAMEREAL

**6.13–6.15 Void, chandle, class**
- [x] 6.13 Void data type
- [ ] 6.14 Chandle data type — absent; §7 61
- [ ] 6.15 Class handle type — two routes skip the handle check; §5.2 Q-HANDLERET

**6.16 String data type**
- [ ] 6.16 String variables: declare, assign, compare — interior NUL kept; integral taken without a cast; §5.2 Q-STRNUL, §3.b string-integral-assign-no-cast
- [ ] 6.16 String concatenation and replication — §5.2 Q-NESTCAT, §5.2 Q-STRREPNC, §5.2 Q-STRREPEVAL
- [ ] 6.16 String `?:` — not typed as a string; §5.2 Q-STRTERN
- [ ] 6.16 String ports and arrays — §5.2 Q-STRPORT, §3.b string-array
- [x] 6.16 Byte index `s[i]` of a string variable
- [ ] 6.16.1 Len() — no self width in a constant region; §2 string-len-self-width
- [x] 6.16.2 Putc()
- [x] 6.16.3 Getc()
- [x] 6.16.4 Toupper()
- [x] 6.16.5 Tolower()
- [x] 6.16.6 Compare()
- [ ] 6.16.7 Icompare() — status not recorded
- [x] 6.16.8 Substr()
- [ ] 6.16.9 Atoi(), atohex(), atooct(), atobin() — constant fold stops at `_`; §2 const-atoi-underscore
- [ ] 6.16.10 Atoreal() — Unicode whitespace skipped; §5.2 Q-ATOREAL-WS
- [x] 6.16.11 Itoa()
- [x] 6.16.12 Hextoa()
- [x] 6.16.13 Octtoa()
- [x] 6.16.14 Bintoa()
- [ ] 6.16.15 Realtoa() — status not recorded

**6.17 Event data type**
- [x] 6.17 Named event declaration
- [ ] 6.17 Event arrays, initializers, assignment — refused; not filed

**6.18 User-defined types**
- [ ] 6.18 Typedef of a vector or packed type — bounds re-fold where used; §5.2 68, §5.2 71
- [ ] 6.18 Forward typedef (`typedef class`, incomplete types) — §3.b class-typedef
- [ ] 6.18 Typedef redeclared in one package — accepted; §5.2 Q-TDREDECL

**6.19 Enumerations**
- [x] 6.19.1 Enum with the default base type
- [ ] 6.19.1 Enum with an explicit 4-state base — a label with an x / z digit refused; §3.b enum-label-domain
- [ ] 6.19.1 Enum with a 2-state base — stored 4-state; §5.2 8, §2 enum-bit-four-state
- [ ] 6.19.1 Enum base wider than 2^64 bits — runs with a wrapped width; §2 enum-base-overflow-silent
- [ ] 6.19.1 Label wider than its base — §3.b enum-label, §5.2 49
- [ ] 6.19.2 Enumerated type ranges (`name[N]`) — status not recorded
- [ ] 6.19.3 Type checking — implicit conversion to an enum accepted; §5.2 Q-ENUMCONV
- [x] 6.19.4 Enumerated types in numerical expressions
- [ ] 6.19.5.1 First() — result 32 bits wide; §2 enum-method-width
- [ ] 6.19.5.2 Last() — result 32 bits wide; §2 enum-method-width
- [ ] 6.19.5.3 Next() — result 32 bits wide; §2 enum-method-width
- [ ] 6.19.5.4 Prev() — result 32 bits wide; §2 enum-method-width
- [x] 6.19.5.5 Num() on an enum variable
- [x] 6.19.5.6 Name() on an enum variable
- [ ] 6.19.5 Methods on a nested struct member (`o.i.e.name()`) — §3.a ⑤ⓓ
- [ ] 6.19.5 Methods on a label or a parameter-valued label — refused; §0 10ⓑ

**6.20 Constants**
- [x] 6.20.1 Parameter declaration syntax
- [ ] 6.20.2 Untyped value parameters — wide copies, wraps; §3.b wide-untyped-consumer, §2 untyped-64bit-wrap
- [ ] 6.20.2 Typed integral value parameters — sign and 2-state lost; §2 signed-param-reads-unsigned, §5.2 Q-PARAM2STATE
- [ ] 6.20.2 Real value parameters — §0 8ⓐ, §0 8ⓑ, §0 8ⓒ
- [ ] 6.20.2 String value parameters — §5.2 50, §3.b string-default-numeric-override
- [ ] 6.20.2 Array and multi-dimensional packed parameters — partly; §3.a ⑤ⓐ, §3.a ⑤ⓒ
- [ ] 6.20.2.1 `$` as a parameter value — status not recorded
- [ ] 6.20.3 Type parameters — integral subset; §3.a ⑤ⓠ, §3.a ⑤ⓟ, §5.2 Q-TPCARRIER
- [ ] 6.20.4 `localparam` at module and package scope — typed of unfolded width unconverted; §5.2 Q-TLPCONV
- [ ] 6.20.4 `localparam` in a procedural block or task body — refused at parse; §2 🆕 Q, §3.b tf-localparam
- [ ] 6.20.5 Specify parameters — parsed as localparam anywhere; §2 specparam-data-type
- [ ] 6.20.6 Const constants — a module-level `const` refused; §3.b module-const-decl

**6.21 Scope and lifetime**
- [x] 6.21 Static variables at module scope
- [ ] 6.21 Static variables in subroutines — run-time initializers; §2 static-local-runtime-init, §2 static-frame-init-net
- [ ] 6.21 Automatic variables in subroutines — §2 auto-local-fork-delay, §2 auto-local-input-slot
- [ ] 6.21 Automatic variables in procedural blocks — one flattened net; §3.b blk-automatic-lifetime

**6.22 Type compatibility**
- [ ] 6.22.1 Matching types — status not recorded
- [ ] 6.22.2 Equivalent types — associative copy across key types accepted; §2 assoc-copy-key-types
- [ ] 6.22.3 Assignment compatible — implicit enum conversion accepted; §5.2 Q-ENUMCONV
- [ ] 6.22.4 Cast compatible — class down-casts refused; §4 class-static-cast
- [ ] 6.22.5 Type incompatible — status not recorded
- [ ] 6.22.6 Matching nettypes — no nettypes; not filed

**6.23–6.25 Type operator, casting, parameterized types**
- [ ] 6.23 Type operator `type(…)` — `var type(x)` declaration refused; §3.b var-type-op-decl
- [ ] 6.24.1 Primitive type casts (`int'`, `byte'`, …) — constant folds; §5.2 Q-PRIMCAST, §2 byte-cast-type-width
- [ ] 6.24.1 Size casts `N'(e)` — §2 Call-leaf sign, §2 Double call, §2 Nested cast
- [x] 6.24.1 Signedness casts `signed'`, `unsigned'`
- [ ] 6.24.1 Narrowing 4-state cast — drops x; §2 narrowing-cast-drops-x
- [ ] 6.24.1 Typedef casts — atom and `bit` typedefs refused; §3.b typedef-atom-cast
- [ ] 6.24.1 `string'(…)` cast — literal lowers bare; §5.2 Q-STRCASTLIT
- [ ] 6.24.2 `$cast` dynamic casting — direct right side only, integral; §4 class-static-cast
- [ ] 6.24.3 Bit-stream casting — status not recorded
- [ ] 6.25 Parameterized data types (class type parameters) — refused; not filed

## 7 Aggregate data types

**7.2 Structures**
- [x] 7.2.1 Packed struct: declare, whole read and write
- [x] 7.2.1 Packed struct member read and write by name
- [ ] 7.2.1 Member part-select with named bounds — §5.2 24
- [ ] 7.2.1 Member indexed or variable-index write — refused; §5.2 Q-MEMBIDXWR, §5.2 Q-MEMBOOBWR
- [ ] 7.2.1 Member widths from parameters — laid out wrong; §5.2 Q-SYMMIX, §5.2 42
- [ ] 7.2.1 Member select after a two-level array index — §5.2 38
- [ ] 7.2.1 Packed dimensions on a typedef'd member — §5.2 41, §3.b typedef-packed-dims
- [ ] 7.2 Unpacked structs — partly (records); §3.a ⑤ⓓ, §5.2 Q-XAGG
- [ ] 7.2.2 Assigning to structures (member defaults) — §3.a ⑤ⓓ

**7.3 Unions**
- [x] 7.3.1 Packed unions: declare, read, write
- [ ] 7.3.1 Untyped pattern into a packed-union member — §2 union-member-pattern
- [ ] 7.3 Unpacked unions — refused at parse; not filed
- [ ] 7.3.2 Tagged unions — absent; not filed

**7.4 Packed and unpacked arrays**
- [x] 7.4.1 Packed arrays: one dimension
- [ ] 7.4.1 Multi-dimensional packed arrays: element selects and writes — §3.b md-packed-write, §5.2 Q-PKRDLSB, §5.2 Q-VARBASEWR
- [ ] 7.4.1 Packed dimension size cap — §5.2 Q-PKCAP
- [x] 7.4.2 Fixed-size unpacked arrays: declare, element read and write
- [x] 7.4.3 Procedural whole-array assignment of one shape
- [ ] 7.4.3 Continuous whole-array assignment — partly; §3.b cont-array-residue, §3.b cont-array-typedef-residue
- [ ] 7.4.3 Whole-array equality and whole arrays in expressions — refused; not filed
- [x] 7.4.4 Memories
- [x] 7.4.5 Multidimensional unpacked arrays at module scope
- [x] 7.4.6 Element indexing
- [ ] 7.4.6 Unpacked-array slices — partial slices refused; §3.b pattern-slice-arith-bound

**7.5 Dynamic arrays**
- [x] 7.5.1 New[ ] as an assignment
- [ ] 7.5.1 New[ ] in other positions, in a function — refused; §3.b V2A/V5
- [x] 7.5.2 Size()
- [x] 7.5.3 Delete()
- [ ] 7.5 Dynamic array element read and write — out-of-range 2-state reads x; a fill into a `real` element; §2 dyn-oob-reads-x, §2 queue-dyn-index-unsealed, §2 real-dyn-elem-fill

**7.6–7.7 Array assignments and arguments**
- [x] 7.6 Array assignments between fixed-size arrays
- [ ] 7.6 Dynamic-array self pattern — no snapshot; §5.2 Q-DYNPAT
- [ ] 7.7 Arrays as arguments to subroutines — partly; §3.b array-formal, §3.b dyn-formal-pos

**7.8 Associative arrays**
- [ ] 7.8.1 Wildcard index type — refused; §7 MVP-CUT
- [ ] 7.8.2 String index — skips the string-key funnel; §2 string-key-funnel
- [ ] 7.8.3 Class index — status not recorded
- [ ] 7.8.4 Integral index — widened, not cast to the key; §2 assoc-index-widened, §2 time-key-signed
- [ ] 7.8.5 Other user-defined types — some keys refused; §3.b assoc-typed-key-loud
- [ ] 7.8.6 Accessing invalid indices — status not recorded
- [ ] 7.8.7 Allocating elements — sub-select writes dropped; §5.2 Q-DYNELEMSUBWR, §2 assoc-struct-member-write

**7.9 Associative array methods**
- [ ] 7.9.1 Num() and size() — enum elements answer label count; §2 assoc-enum-num
- [x] 7.9.2 Delete()
- [x] 7.9.3 Exists()
- [ ] 7.9.4 First() — direct right side only; §2 iter-var-type-mismatch
- [ ] 7.9.5 Last() — direct right side only; §2 iter-var-type-mismatch
- [ ] 7.9.6 Next() — direct right side only; §2 iter-var-type-mismatch
- [ ] 7.9.7 Prev() — direct right side only; §2 iter-var-type-mismatch
- [ ] 7.9.8 Arguments to traversal methods — §2 iter-var-type-mismatch
- [ ] 7.9.9 Associative array assignment — §2 assoc-copy-key-types
- [ ] 7.9.10 Associative array arguments — status not recorded
- [ ] 7.9.11 Associative array literals — keyed `'{0: a}` refused; not filed

**7.10 Queues**
- [ ] 7.10.1 Queue operators (`q[i]`, `q[$]`, slices) — signed index read unsigned; §2 queue-dyn-index-unsealed
- [ ] 7.10.1 Element writes — part-select and nonblocking; §2 queue-elem-part-write, §5.2 Q-QNBA
- [ ] 7.10 Signed x / z element bound to a formal — zero-extends; §2 signed-xz-queue-formal
- [x] 7.10.2.1 Size()
- [x] 7.10.2.2 Insert()
- [x] 7.10.2.3 Delete()
- [ ] 7.10.2.4 Pop_front() — direct right side only; not filed
- [ ] 7.10.2.5 Pop_back() — direct right side only; not filed
- [x] 7.10.2.6 Push_front()
- [x] 7.10.2.7 Push_back()
- [ ] 7.10.3 Persistence of references to elements — status not recorded
- [ ] 7.10.4 Updating a queue by assignment and concatenation — status not recorded
- [x] 7.10.5 Bounded queues

**7.11–7.12 Array querying and manipulation**
- [ ] 7.11 Array querying functions — `$size` with a dimension argument; §2 size-dim-arg
- [ ] 7.12.1 Array locator methods — queue-handle target only; §5.2 Q-LOCSTALE
- [x] 7.12.2 Array ordering methods sort, rsort, reverse
- [ ] 7.12.2 Shuffle() — status not recorded
- [ ] 7.12.3 Array reduction methods — real `with` typing; §5.2 Q-WITHREAL, §5.2 Q-INLITEM
- [ ] 7.12.4 Iterator index querying — position, not key; §2 assoc-index-locators

## 8 Classes

**8.3–8.12 Class basics**
- [ ] 8.3 Class declaration — body names bind where parsed; §2 class-scope forward use
- [ ] 8.4 Objects (handles, null, identity) — §5.2 Q-HANDLERET
- [ ] 8.5 Object properties — integral only; in a concatenation target; §3.b class-real-members, §2 class-field-default-width, §2 class-prop-concat-native
- [ ] 8.6 Object methods — class-typed return is one bit; §5.2 Q-CLSRET
- [x] 8.7 Constructors
- [ ] 8.8 Typed constructor calls — status not recorded
- [ ] 8.9 Static class properties — refused; §3.b class-real-members
- [ ] 8.10 Static methods — refused; §3.b class-real-members
- [x] 8.11 This
- [x] 8.12 Handle assignment
- [ ] 8.12 Shallow copy (`new h`) — status not recorded

**8.13–8.24 Inheritance and access**
- [ ] 8.13 Inheritance and subclasses — base from another module accepted; §2 extends-other-module
- [ ] 8.14 Overridden members — status not recorded
- [x] 8.15 Super
- [ ] 8.16 Casting — down-casts refused; §4 class-static-cast
- [x] 8.17 Chaining constructors
- [x] 8.18 Data hiding and encapsulation (`local`, `protected`)
- [ ] 8.19 Constant class properties — refused at parse; not filed
- [x] 8.20 Virtual methods
- [ ] 8.21 Abstract classes and pure virtual methods — absent; not filed
- [x] 8.22 Polymorphism: dynamic method lookup
- [ ] 8.23 Class scope resolution operator `::` — status not recorded
- [ ] 8.24 Out-of-block declarations (`extern`) — refused at parse; not filed

**8.25–8.29 Parameterized classes and the rest**
- [ ] 8.25 Parameterized classes: value parameters — a `signed` value parameter refused; §3.b class-param-signing
- [ ] 8.25 Value-parameter override not converted to its type — §5.2 Q-CLSPARAMTYPE
- [ ] 8.25 Parameterized classes: type parameters — refused; not filed
- [ ] 8.25.1 Class scope resolution for parameterized classes — status not recorded
- [ ] 8.26 Interface classes — absent; not filed
- [ ] 8.27 Typedef class — §3.b class-typedef
- [ ] 8.29 Memory management — no garbage collection; not filed

## 9 Processes

**9.2 Structured procedures**
- [x] 9.2.1 Initial procedures
- [x] 9.2.2.1 `always` with an event control
- [x] 9.2.2.1 `always` with in-body timing
- [ ] 9.2.2.1 `always` whose only timing is an intra-assignment delay — §5.2 Q-ALWINTRA, §5.2 Q-ALWRESIDUE
- [ ] 9.2.2.1 `always` with no timing control — inert with a warning; §5.2 Q-ALWBUDGET
- [ ] 9.2.2.2 `always_comb` time-0 pass and order — §2 🆕 AA, §2 comb-t0-pass-order
- [ ] 9.2.2.2.1 Implicit `always_comb` sensitivities — longest static prefix; §5.2 ENG-3
- [ ] 9.2.2.2 `always_comb` written by another process — whole writes only; the partial rule is parked: §5.2 Q-MDRVMIX
- [ ] 9.2.2.3 `always_latch` — as `always_comb`; §2 🆕 AA, §3.b mdrv-ff-latch-pairs
- [x] 9.2.2.4 `always_ff` with an event-control header
- [ ] 9.2.2.4 `always_ff` with no header — a missable event control refused; §7 FF-MISSED-AT
- [ ] 9.2.2.4 `always_ff` single-writer restriction — §3.b mdrv-ff-latch-pairs
- [x] 9.2.3 Final procedures

**9.3 Block statements**
- [x] 9.3.1 Sequential blocks
- [ ] 9.3.1 Block-local declarations — share flattened nets; §2 shared-flat-local-net, §2 blk-scope-name-leak, §5.2 Q-SCOPEBIND
- [ ] 9.3.2 Parallel blocks (`fork … join`, `join_any`, `join_none`) — §5.2 Q-FORKARM-HIDDEN, §5.2 Q-ARMWRITE
- [ ] 9.3.2 Nested fork, fork-arm declarations — refused or hoisted; §2 fork-declarations-flatten
- [x] 9.3.3 Statement block start and finish times
- [x] 9.3.4 Block names
- [x] 9.3.5 Statement labels

**9.4 Procedural timing controls**
- [ ] 9.4.1 Delay control — negative and over-u32 delays; §2 negative-delay-fires, §5.b DELAY-CLAMP
- [x] 9.4.2 Edge event control in a process header
- [ ] 9.4.2 In-body edge wait — sees an earlier edge of its batch; §2 edge-wait-earlier-edge
- [ ] 9.4.2 `edge` keyword — refused at parse; §3.b edge-event
- [x] 9.4.2 Level event control on a whole net or variable
- [ ] 9.4.2 Level event on a variable-index select — refused; §3.b level-select-var-index
- [ ] 9.4.2 Constant-index select that never wakes — §2 const-select-never-wakes, §5.2 Q-EVSELT0
- [ ] 9.4.2 Hierarchical name in an event control — refused; §3.b hier-event
- [x] 9.4.2 Named event and real variable events
- [x] 9.4.2.1 Event OR operator
- [ ] 9.4.2.1 Mixed level and edge terms — level wakes on bit 0; §5.2 Q-MIX, §2 level-edge-bit0-wake
- [ ] 9.4.2.2 Implicit event_expression list `@*` — §5.2 Q-FRAMESTAR, §2 at-star-dyn-handle
- [ ] 9.4.2.3 Conditional event controls (`iff`) — a loop variable read E3010; §5.2 ENG-4, §3.b iff-loop-variable
- [ ] 9.4.2.4 Sequence events — status not recorded
- [x] 9.4.3 Level-sensitive `wait` on integral expressions
- [ ] 9.4.3 `wait` on strings and heap containers — never wakes; §5.2 Q-STRWAIT, §5.2 Q-WAITDYN, §2 wait-heap-container
- [ ] 9.4.4 Level-sensitive sequence controls — status not recorded
- [x] 9.4.5 Intra-assignment delay (`= #d`, `<= #d`) in module processes
- [ ] 9.4.5 Intra-assignment delay in overlapping automatic task activations — §5.2 Q-CAPLIFE
- [ ] 9.4.5 Intra-assignment event control (`= @(ev)`, `<= @(ev)`) — a lone unsized literal loses its width; §2 event-control-unsized-fill
- [ ] 9.4.5 `repeat (n) @(ev)` with a run-time count — refused; not filed
- [ ] 9.4.5 Intra-assignment delay on a string concatenation — §5.2 Q-DLYCAT

**9.5–9.7 Process control**
- [x] 9.5 Process execution threads
- [x] 9.6.1 Wait fork statement
- [x] 9.6.2 Disable of an enclosing named block
- [ ] 9.6.2 Disable across processes, of a task, by hierarchical path — §3.b xproc-disable, §7 MVP-CUT
- [x] 9.6.3 Disable fork at process level
- [ ] 9.6.3 Disable fork inside a subroutine frame body — refused; not filed
- [ ] 9.7 Fine-grain process control (`process` class) — absent; not filed

## 10 Assignment statements

**10.3 Continuous assignments**
- [x] 10.3.1 The net declaration assignment
- [x] 10.3.2 Continuous assignment to a whole net or a constant select
- [ ] 10.3.2 Target with a non-constant select — accepted; §5.2 Q-CALHSEL, §2 net-variable-select-assign
- [ ] 10.3.2 Right side calling a function — re-evaluated per settle; §2 ca-function-reruns, §3.b real-cont-assign
- [ ] 10.3.2 Continuous assignment to a variable — partly; §3.b cont-var-partial, §3.b cont-var-gate, §3.b ustruct-cont-assign
- [ ] 10.3.3 Continuous assignment delays — §5.2 Q-DCASETTLE, §5.2 Q-DCAPARTX, §5.2 N-R01, §2 multidriven-delay-dropped
- [ ] 10.3.4 Continuous assignment strengths — refused at parse; §3.b decl-var-net-kw

**10.4 Procedural assignments**
- [x] 10.4.1 Blocking procedural assignments
- [ ] 10.4.2 Nonblocking procedural assignments — a class property's handle read when applied; §2 nba-class-handle-time

**10.5 Variable declaration assignment**
- [x] 10.5 Initializer of a module-scope variable
- [ ] 10.5 Initializer in a subroutine or block — §2 static-local-runtime-init, §5.2 Q-PKGBLKLEAK

**10.6 Procedural continuous assignments**
- [ ] 10.6.1 `assign` and `deassign` — deassign over a continuous assign accepted; a concatenation target refused; §5.2 Q-DEASSIGNCA, §3.b deep
- [ ] 10.6.2 `force` — samples once; §5.2 Q-FORCEDYN, §5.2 Q-FORCEPORT, §5.2 Q-FORCEAUTO
- [ ] 10.6.2 `release` — §5.2 Q-RELIMM, §5.2 Q-RELUNDRV, §5.2 Q-RELZERODLY
- [ ] 10.6.2 Force or release of a select — refused; §3.b deep

**10.7–10.8 Extension and assignment-like contexts**
- [x] 10.7 Assignment extension and truncation
- [ ] 10.8 Assignment-like contexts — status not recorded

**10.9 Assignment patterns**
- [ ] 10.9.1 Array patterns: positional and `default:` on fixed-size arrays — a fill element of a `real` array 64 bits; §2 real-pattern-port-fill
- [ ] 10.9.1 Replication, index keys and type keys — refused at parse; §5.2 35
- [ ] 10.9.1 Patterns into multi-dimensional packed arrays — §5.2 69, §3.b md-packed-pattern
- [x] 10.9.2 Structure patterns: positional, named, `default:` on a packed struct
- [ ] 10.9.2 Typed patterns `T'{…}` beyond an unsigned packed struct — §3.b typed-pattern-residue
- [ ] 10.9 `default:` onto a function-local array — refused; §5.2 33

**10.10–10.11 Unpacked array concatenation and aliasing**
- [ ] 10.10 Unpacked array concatenation — fills a frame-local array in reverse; §5.2 Q-FRAMECONCAT
- [ ] 10.11 Net aliasing — refused at parse; §3.b decl-var-net-kw

## 11 Operators and expressions

**11.2 Overview**
- [ ] 11.2.1 Constant expressions — many folds decline or bind wrong; §5.2 76, §2 🆕 AE, §2 🆕 H
- [ ] 11.2.2 Aggregate expressions — whole unpacked arrays have no value; not filed

**11.3 Operators**
- [ ] 11.3.1 Operators with real operands — a `real` concatenation target accepted; §2 signed-of-real-accepted, §0 8ⓔ, §5.2 Q-CAREALCAT
- [x] 11.3.2 Operator precedence
- [x] 11.3.3 Using integer literals in expressions
- [x] 11.3.4 Operations on 4-state and 2-state types
- [ ] 11.3.5 Operator expression short circuiting — an operand call named twice; §2 operator-call-twice
- [ ] 11.3.6 Assignment within an expression — status not recorded

**11.4 Operator descriptions**
- [ ] 11.4.1 Assignment operators (`+=` …) — index evaluated twice; §5.2 Q-CMPIDXTWICE, §5.2 Q-HIERCOMPOUND
- [x] 11.4.2 Increment and decrement as statements
- [ ] 11.4.2 Increment and decrement inside expressions — status not recorded
- [x] 11.4.3 Arithmetic operators
- [x] 11.4.3.1 Arithmetic with unsigned and signed types
- [x] 11.4.4 Relational operators
- [x] 11.4.5 Equality operators
- [x] 11.4.6 Wildcard equality with a constant pattern, at run time
- [ ] 11.4.6 Wildcard equality in a constant expression — compares at the left width; §2 🆕 S, §3.b compound-==?
- [ ] 11.4.6 Wildcard equality with a run-time pattern — refused; §3.b deep
- [x] 11.4.7 Logical operators
- [x] 11.4.8 Bitwise operators
- [x] 11.4.9 Reduction operators
- [x] 11.4.10 Shift operators
- [x] 11.4.11 Conditional operator over integral operands
- [ ] 11.4.11 Conditional operator over string, real or aggregate arms — §5.2 Q-XNONINT, §5.2 Q-STRTERN
- [ ] 11.4.12 Concatenation operators — unsized operand accepted; a fill into hierarchical pieces; §2 unsized-concat-operand, §2 hier-concat-fill, §2 concat-unsized-literal
- [ ] 11.4.12.1 Replication operator — zero-count rule; §5.2 Q-ZEROREPL, §2 nested-zero-replication
- [ ] 11.4.12.2 String concatenation — §5.2 Q-NESTCAT, §5.2 Q-STRCATSEL
- [x] 11.4.13 Set membership `inside` at run time with constant elements
- [ ] 11.4.13 `inside` in a constant expression — compares at the left width; §2 🆕 S
- [ ] 11.4.13 `inside` with an element whose x / z exist at run time — compared with `==`; §2 🆕 S
- [x] 11.4.14 Streaming `{<<N{…}}`, `{>>N{…}}` into a module variable
- [ ] 11.4.14 Streaming into a subroutine local — right-justified; §2 local-stream-justify
- [ ] 11.4.14.1 Type slice size and `with` — refused at parse; not filed
- [ ] 11.4.14.3 Streaming concatenation as an assignment target — status not recorded
- [ ] 11.4.14.4 Streaming dynamically sized data — status not recorded

**11.5 Operands**
- [x] 11.5.1 Constant bit-select and part-select in range
- [x] 11.5.1 Indexed part-select `+:` `-:`
- [ ] 11.5.1 Select partly or wholly out of range — refused or wrong; §5.2 Q-PSELOOR, §5.2 Q-VMSELSHIFT, §2 packed-element-overhang, §5.2 Q-LITLANE, §2 fill-over-u32-width
- [ ] 11.5.1 Indexed width 0, negative or over 2^20 — §2 zero-width-part-select, §2 dead-arm-zero-width, §3.b over-max-width
- [ ] 11.5.1 Bit-select of a scalar — folds; §5.2 Q-SCALARSEL
- [ ] 11.5.1 Nested selects in a write target — refused; §5.2 29
- [ ] 11.5.1 Nested part-select read of a bit-select — accepted; §2 nested-select-of-bit
- [ ] 11.5.2 Array and memory addressing — wide index truncated; §5.2 Q-IDX64TRUNC, §5.2 Q-IDXSIGNCTX
- [ ] 11.5.3 Longest static prefix — §5.2 ENG-3

**11.6–11.8 Bit lengths, signedness, evaluation**
- [ ] 11.6.1 Rules for expression bit lengths — §5.2 74, §2 late-sibling-operand-width
- [ ] 11.7 Signed expressions — §2 Call-leaf sign, §2 🆕 AI
- [ ] 11.8.1 Rules for expression types — §5.2 74
- [ ] 11.8.2 Steps for evaluating an expression — §2 🆕 AI
- [x] 11.8.3 Steps for evaluating an assignment
- [x] 11.8.4 Handling x and z in signed expressions

**11.9–11.12 Tagged unions, strings, min:typ:max, let**
- [ ] 11.9 Tagged union expressions and member access — absent; not filed
- [x] 11.10.1 String literal operations
- [x] 11.10.2 String literal value padding
- [ ] 11.10.3 Empty string literal handling — 1 bit wide; §5.2 Q-STRWIDTH, §5.2 Q-STREMPTY
- [ ] 11.11 Min, typical, and maximum delay expressions — typ always taken; not filed
- [ ] 11.12 Let construct with untyped formals and an integral body — a lone unsized body loses its width; outside a module refused; §2 let-fill, §3.b let-decl-scopes
- [ ] 11.12 Let with typed formals or a string body — §5.2 Q-LETCAST, §5.2 Q-LETSTR

## 12 Procedural programming statements

**12.4 Conditional if–else statement**
- [x] 12.4 if–else
- [x] 12.4.1 if–else–if construct
- [ ] 12.4.2 unique-if, unique0-if, priority-if — misses in subroutine bodies; §3.b unique-if-chain, §3.b unique-if-text
- [ ] 12.4.2.1 Overlap (multiple-match) violation reports — §8 non-goal-list
- [ ] 12.4.2.2 If violation reports and multiple processes — status not recorded

**12.5 Case statement**
- [ ] 12.5 Case statement — signed shift scrutinee, call scrutinee; §2 🆕 B, §2 🆕 X, §2 🆕 Y
- [x] 12.5.1 Case statement with do-not-cares (`casez`, `casex`)
- [x] 12.5.2 Constant expression in case statement
- [x] 12.5.3 unique-case, unique0-case, priority-case: no-match report
- [ ] 12.5.3.1 Overlap (multiple-match) violation reports — §8 non-goal-list
- [ ] 12.5.3.2 Case violation reports and multiple processes — status not recorded
- [ ] 12.5.4 Set membership case statement (`case … inside`) — partly; §3.b case-inside-residue, §3.b inside-name-use

**12.6 Pattern matching conditional statements**
- [ ] 12.6 Pattern matching in case, if and `?:` — absent; not filed

**12.7 Loop statements**
- [x] 12.7.1 The for-loop with one init and one step
- [ ] 12.7.1 Comma-separated for-init or step, or an empty step — refused at parse; §3.b for-header-lists
- [ ] 12.7.1 Loop variable shadowing a constant — §2 loop-var-const-shadow
- [x] 12.7.2 The repeat loop
- [x] 12.7.3 The foreach-loop over fixed, dynamic and queue arrays
- [ ] 12.7.3 The foreach-loop over associative arrays — index typed `integer`; §2 foreach-assoc-index-type
- [ ] 12.7.3 Foreach over packed vectors, empty leading index — refused; not filed
- [x] 12.7.4 The while-loop
- [ ] 12.7.5 The do…while-loop — body parsed twice; §2 do-while-parsed-twice
- [x] 12.7.6 The forever-loop

**12.8 Jump statements**
- [x] 12.8 `break` and `continue`
- [x] 12.8 `return` in a function or task body
- [ ] 12.8 `return` inside a timing-control body in a task — refused; §3.b timing-return

## 13 Tasks and functions (subroutines)

**13.3 Tasks**
- [ ] 13.3 Task declaration and enable — a select write to an output formal refused; §3.b task-output-select-write
- [ ] 13.3.1 Static tasks — input substituted over the body; §2 static-task-arg-subst, §2 static-task-auto-local
- [ ] 13.3.1 Automatic tasks — §5.2 Q-PTAIL, §5.2 Q-FRAMEWAKE
- [ ] 13.3.2 Task memory usage and concurrent activation — §5.2 Q-STATICFRAME-WAIT

**13.4 Functions**
- [x] 13.4 Function declaration and call
- [ ] 13.4 Function body restrictions — timing control or task enable accepted; §5.2 Q-FNTIMING
- [x] 13.4.1 Void functions
- [ ] 13.4.1 Return values — a static function reading its return variable; §2 static-fn-return-read
- [ ] 13.4.2 Static and automatic functions — §3.b §3.11, §3.b static-local
- [ ] 13.4.3 Constant functions — every `case` refused, many folds decline; §5.2 5, §3.b const-fn-case
- [ ] 13.4.4 Background processes spawned by function calls — status not recorded

**13.5 Subroutine calls and argument passing**
- [x] 13.5.1 Pass by value
- [ ] 13.5.2 Pass by reference — copy-in, copy-out; §5.2 Q-REFALIAS, §5.2 Q-REFILLEGAL
- [ ] 13.5.3 Default argument values — §3.b pkg-default, §3.b hier-default-arg
- [x] 13.5.4 Argument binding by name in a direct call
- [ ] 13.5.4 Named arguments in a hierarchical call — refused; not filed
- [ ] 13.5.5 Optional argument list (empty positional argument) — §3.b empty-positional-argument

**13.6–13.8 DPI names, task and function names, parameterized subroutines**
- [ ] 13.6 Import and export functions (DPI) — §3.a ⑮
- [ ] 13.7 Task and function names (hierarchical calls) — §3.b hier-task, §3.b hier-fn-inline-callee
- [ ] 13.8 Parameterized tasks and functions (class static methods) — refused; §3.b class-real-members

## 14 Clocking blocks

**14.3–14.16 Clocking**
- [ ] 14.3 Clocking block declaration — name resolution misses shadowing; §5.2 Q-CLKSHADOW
- [ ] 14.4 Input and output skews other than `#1step` — refused; §4 n4-clocking-skew
- [ ] 14.5 Hierarchical expressions (`@(inst.cb)`, hierarchical items) — refused; not filed
- [ ] 14.6 Signals in multiple clocking blocks — mangled names collide; §2 23
- [ ] 14.7 Clocking block scope and lifetime — status not recorded
- [ ] 14.9 Interfaces and clocking blocks — clocking in an interface refused; not filed
- [x] 14.10 Clocking block events `@(cb)`
- [ ] 14.11 Cycle delay `##` — status not recorded
- [x] 14.12 Default clocking
- [x] 14.13 Input sampling of integral and real items
- [ ] 14.13 Input sampling of strings, queues, unpacked arrays — §5.2 Q-CLKHEAP
- [ ] 14.14 Global clocking — status not recorded
- [ ] 14.15 Synchronous events — status not recorded
- [ ] 14.16 Synchronous drives — output clobbered to x; §5.2 Q-CLKDRV, §2 24
- [ ] 14.16 `inout` clocking items — refused; not filed

## 15 Interprocess synchronization and communication

**15.3–15.5**
- [ ] 15.3 Semaphores — absent; not filed
- [ ] 15.4 Mailboxes — absent; not filed
- [x] 15.5.1 Triggering an event `->`
- [ ] 15.5.1 Nonblocking trigger `->>` — status not recorded
- [x] 15.5.2 Waiting for an event `@`
- [ ] 15.5.3 Persistent trigger: `triggered` — status not recorded
- [ ] 15.5.4 Event sequencing: `wait_order()` — absent; not filed
- [ ] 15.5.5 Operations on named event variables (assign, merge, compare) — refused; not filed

## 16 Assertions

**16.3–16.4 Immediate and deferred assertions**
- [x] 16.3 Immediate `assert` and `assume`
- [ ] 16.3 Immediate `cover` — refused at parse; §3.b cover-immediate, §3.b immediate-cover
- [ ] 16.4.1 Deferred assertion reporting — non-print action runs at reach; §3.b deferred-inline-action
- [x] 16.4.2 Deferred assertion flush points
- [ ] 16.4.3 Deferred assertions outside procedural code — declaring action block; §2 deferred-assert-block
- [ ] 16.4.4 Disabling deferred assertions — refused beside assertion control; not filed
- [ ] 16.4.5 Deferred assertions and multiple processes — status not recorded

**16.5–16.6 Concurrent assertions overview**
- [ ] 16.5.1 Sampling — previous-value registers, no Preponed sampling; §5.2 Q-DSV
- [ ] 16.5.2 Assertion clock — single clock required; not filed
- [ ] 16.6 Boolean expressions — names bind the wrong object; §5.2 Q-SVASCOPE, §5.2 Q-SVALET

**16.7–16.9 Sequences**
- [x] 16.7 Sequences with `##n` and `##[m:n]`
- [ ] 16.7 Unbounded `##[m:$]` fused with `##0` — §4 fused-zero-unbounded-delay
- [x] 16.8 Declaring sequences with untyped formals
- [ ] 16.8.1 Typed formal arguments in sequence declarations — status not recorded
- [ ] 16.8.2 Local variable formal arguments — §4 n2c-seq-local-vars
- [x] 16.9.2 Consecutive repetition `[*n]`, `[*m:n]`, `[+]`
- [ ] 16.9.2 Unbounded `[*m:$]` over a non-boolean — refused; not filed
- [ ] 16.9.2 Goto `[->n]` and nonconsecutive `[=n]` — single count, boolean only; not filed
- [ ] 16.9.3 Sampled value functions in sequences — `$past` one argument; §5.2 Q-DSV
- [ ] 16.9.4 Global clocking past and future functions — status not recorded
- [ ] 16.9.5 AND operation — booleans of one clock only; not filed
- [ ] 16.9.6 Intersection — absent; not filed
- [ ] 16.9.7 OR operation — booleans of one clock only; not filed
- [ ] 16.9.8 First_match operation — absent; not filed
- [ ] 16.9.9 Conditions over sequences (`throughout`) — boolean left operand only; not filed
- [ ] 16.9.10 Sequence contained within another (`within`) — top-level antecedent only; not filed

**16.10–16.11 Local variables and subroutines on match**
- [ ] 16.10 Local variables — one capture; §4 n2c-seq-local-vars, §5.2 Q-SVALOCALKIND
- [ ] 16.11 Calling subroutines on match of a sequence — status not recorded

**16.12 Declaring properties**
- [ ] 16.12.1 Property instantiation — some named-property consequents refused; not filed
- [ ] 16.12.2 Sequence property (no implication) — refused at parse; not filed
- [ ] 16.12.3 Negation property — same-clock operand only; not filed
- [ ] 16.12.4 Disjunction property — same-clock booleans only; not filed
- [ ] 16.12.5 Conjunction property — same-clock booleans only; not filed
- [ ] 16.12.6 If-else property — status not recorded
- [x] 16.12.7 Implication `|->`, `|=>`
- [x] 16.12.8 Implies and iff properties
- [ ] 16.12.9 Followed-by property — status not recorded
- [ ] 16.12.10 Nexttime property — bounded form refused; not filed
- [ ] 16.12.11 Always property — top level only, `s_always` refused; not filed
- [x] 16.12.12 Until property `until`, `s_until`
- [ ] 16.12.12 `until_with`, `s_until_with` — status not recorded
- [ ] 16.12.13 Eventually property — unbounded `s_eventually` only; not filed
- [ ] 16.12.14 Abort properties — status not recorded
- [ ] 16.12.15 Weak and strong operators — status not recorded
- [ ] 16.12.16 Case property — status not recorded
- [ ] 16.12.17 Recursive properties — legal only as a `|=>` consequent; not filed
- [ ] 16.12.18 Typed formal arguments in property declarations — status not recorded
- [ ] 16.12.19 Local variable formal arguments in property declarations — §4 n2c-seq-local-vars

**16.13 Multiclock support**
- [x] 16.13.2 Two-clock property `@(c1) a |=> @(c2) b`
- [ ] 16.13.1 Multiclocked sequences — `##1` boundaries only; not filed
- [ ] 16.13.6 Sequence methods (`triggered`, `matched`) — status not recorded

**16.14 Concurrent assertions**
- [ ] 16.14.1 Assert statement — a corpus design's properties refused; §3.b eh1-assert-on, §5.2 73
- [ ] 16.14.2 Assume statement — status not recorded
- [ ] 16.14.3 Cover statement — action block refused; not filed
- [ ] 16.14.4 Restrict statement — absent; not filed
- [x] 16.14.5 Concurrent assertions outside procedural code
- [ ] 16.14.6 Embedding concurrent assertions in procedural code — status not recorded
- [ ] 16.14.7 Inferred value functions — status not recorded
- [ ] 16.14.8 Nonvacuous evaluations — pass action on vacuous success; not filed

**16.15–16.18**
- [x] 16.15 `disable iff` and `default disable iff`
- [ ] 16.15 `disable iff` with a property-level `and` / `or` — refused; not filed
- [ ] 16.16 Clock resolution — no clock in scope refused, multi-clock refused; not filed
- [ ] 16.17 Expect statement — absent; not filed
- [ ] 16.18 Clocking blocks and concurrent assertions — status not recorded

## 17 Checkers

**17.2–17.8**
- [ ] 17.2 Checker declaration — absent; no roadmap row
- [ ] 17.3 Checker instantiation — absent; no roadmap row
- [ ] 17.4–17.8 Context inference, procedures, covergroups, variables, functions in checkers — absent; no roadmap row

## 18 Constrained random value generation

**18.4 Random variables**
- [x] 18.4.1 `rand` on integral members
- [ ] 18.4.1 `rand` on class-handle members — refused; not filed
- [ ] 18.4.2 `randc` — 16 bits, outside the solver; not filed

**18.5 Constraint blocks**
- [x] 18.5 Boolean constraints over rand fields and constants
- [ ] 18.5 Names in a class body binding — §2 class-scope forward use
- [ ] 18.5.1 External constraint blocks — status not recorded
- [ ] 18.5.2 Constraint inheritance — status not recorded
- [x] 18.5.3 Set membership
- [x] 18.5.4 Distribution
- [ ] 18.5.5 Uniqueness constraints — status not recorded
- [x] 18.5.6 Implication
- [ ] 18.5.7 if–else constraints — refused; §3.b constraint-if, §7 57
- [ ] 18.5.8.1 foreach iterative constraints — absent; not filed
- [ ] 18.5.8.2 Array reduction iterative constraints — status not recorded
- [ ] 18.5.9 Global constraints — status not recorded
- [ ] 18.5.10 Variable ordering (`solve … before`) — absent; not filed
- [ ] 18.5.11 Static constraint blocks — status not recorded
- [ ] 18.5.12 Functions in constraints — operators only; not filed
- [ ] 18.5.13 Constraint guards — status not recorded
- [x] 18.5.14 Soft constraints

**18.6–18.12 Randomization methods and control**
- [x] 18.6.1 Randomize()
- [ ] 18.6.2 Pre_randomize() and post_randomize() — absent; not filed
- [x] 18.7 In-line constraints `randomize() with` over class fields
- [ ] 18.7 In-line constraint names — bind at the call, not in the class; §2 inline-constraint-caller-scope
- [ ] 18.7.1 `local::` scope resolution — status not recorded
- [ ] 18.8 Disabling random variables with `rand_mode()` — absent; not filed
- [ ] 18.9 Controlling constraints with `constraint_mode()` — absent; not filed
- [ ] 18.10 Dynamic constraint modification — status not recorded
- [ ] 18.11 In-line random variable control `randomize(v)` — refused; not filed
- [ ] 18.11.1 In-line constraint checker — status not recorded
- [ ] 18.12 `std::randomize()` — absent; §3.b std-randomize-nested, §7 58

**18.13–18.17 Random numbers, stability, randcase, randsequence**
- [x] 18.13.1 `$urandom`
- [x] 18.13.2 `$urandom_range()`
- [ ] 18.13.3 Srandom() — status not recorded
- [ ] 18.13.4 Get_randstate() — status not recorded
- [ ] 18.13.5 Set_randstate() — status not recorded
- [ ] 18.14 Random stability (thread and object) — status not recorded
- [ ] 18.15 Manually seeding randomize — status not recorded
- [ ] 18.16 Random weighted case `randcase` — absent; not filed
- [ ] 18.17 Random sequence generation `randsequence` — absent; not filed

## 19 Functional coverage

**19.3–19.4 Covergroups**
- [x] 19.3 Covergroup declaration, clocking event, `new`
- [ ] 19.3 Covergroup-typed block-local with an initializer — refused; §7 55, §3.b cg-block-local
- [ ] 19.4 Using covergroup in classes — status not recorded

**19.5 Defining coverage points**
- [ ] 19.5 Coverpoint expression — constant part-select domain width; §5.2 Q-COVSELDOM, §2 coverpoint-pow-width
- [x] 19.5.1 Value bins `bins`, `bins b[]`
- [ ] 19.5.1 Fixed-size array bins `bins b[N]` — refused; not filed
- [ ] 19.5.1.1 Coverpoint bin with covergroup expressions — status not recorded
- [ ] 19.5.1.2 Coverpoint bin set covergroup expressions — status not recorded
- [ ] 19.5.2 Bins for transitions — refused at parse; not filed
- [x] 19.5.3 Automatic bin creation
- [ ] 19.5.4 Wildcard bins — refused at parse; not filed
- [x] 19.5.5 Excluding values (`ignore_bins`)
- [x] 19.5.6 Illegal values (`illegal_bins`)
- [ ] 19.5.7 Value resolution — status not recorded

**19.6 Defining cross coverage**
- [x] 19.6 Cross of coverpoints within 64 bins
- [ ] 19.6.1 Cross bins (`binsof`, `intersect`) — refused at parse; not filed
- [ ] 19.6.2 Excluding cross products — refused at parse; not filed
- [ ] 19.6.3 Illegal cross products — refused at parse; not filed

**19.7–19.11 Options, methods, computation**
- [x] 19.7 Options `at_least`, `weight`
- [ ] 19.7 Other coverage options — status not recorded
- [ ] 19.7.1 Covergroup type options — status not recorded
- [x] 19.8 Predefined method `sample()`
- [ ] 19.8 `get_coverage()` — integer, IEEE gives a real; not filed
- [ ] 19.8 Other predefined coverage methods — refused; not filed
- [ ] 19.8.1 Overriding the built-in sample method — status not recorded
- [ ] 19.9 Coverage system tasks and functions — status not recorded
- [ ] 19.11 Coverage computation — status not recorded

## 20 Utility system tasks and system functions

**20.2 Simulation control**
- [x] 20.2 `$finish`
- [ ] 20.2 `$finish` inside a subroutine body — fatal at run time; §3.a ⑧ⓑ, §3.a ⑧ⓒ
- [ ] 20.2 `$stop` — ends the run, no interactive mode; not filed
- [ ] 20.2 `$exit` — same as `$finish`; not filed

**20.3–20.4 Time**
- [ ] 20.3.1 `$time` — a continuous assign in another unit; §2 ca-time-scale
- [x] 20.3.2 `$stime`
- [x] 20.3.3 `$realtime`
- [ ] 20.4.1 `$printtimescale` — not recognised, skipped; not filed
- [x] 20.4.2 `$timeformat`

**20.5–20.7 Conversion and queries**
- [ ] 20.5 `$rtoi` — inside a constant-function body; §0 8ⓖ
- [ ] 20.5 `$itor` — no constant fold; §3.b itor-const-fold
- [ ] 20.5 `$realtobits`, `$bitstoreal` — any width accepted; §2 realtobits-width
- [ ] 20.5 `$signed`, `$unsigned` — real and string arguments accepted; §2 signed-of-real-accepted, §2 signed-string-arg-accepted
- [ ] 20.6.1 `$typename` — enums and structs as their base; §3.b $typename
- [ ] 20.6.2 `$bits` — sized from a folded value; §5.2 Q-BITSTYPE, §3.b dyn-bits-count
- [x] 20.6.3 `$isunbounded`
- [ ] 20.7 Array query functions — `$size` with a dimension argument, of a hierarchical name; §2 size-dim-arg, §3.b dyn-size-spellings, §3.b hier-array-query
- [ ] 20.7.1 Queries over multiple variable dimensions — §2 size-dim-arg

**20.8–20.11 Math, bit vector, severity, elaboration tasks**
- [ ] 20.8.1 Integer math `$clog2` — no argument panics; §2 sysfn-zero-arg-panic
- [x] 20.8.2 Real math functions
- [x] 20.9 Bit vector system functions (`$countones`, `$onehot`, `$onehot0`, `$isunknown`, `$countbits`)
- [x] 20.10 Severity tasks `$info`, `$warning`, `$error`
- [ ] 20.10 `$fatal` — drops its step's pending NBA; §2 fatal-drops-nba
- [x] 20.11 Elaboration system tasks

**20.12–20.18 Assertion control, sampled values, coverage, random, misc**
- [ ] 20.12 Assertion control tasks — global form only, immediate checks not reached; §2 assertoff-immediate
- [ ] 20.13 Sampled value system functions — `$past` one argument; §5.2 Q-DSV
- [ ] 20.14 Coverage system functions — not recognised; not filed
- [x] 20.15.1 `$random`
- [ ] 20.15.2 Distribution functions — normal and chi-square differ; §5.2 Q-DISTN
- [ ] 20.16 Stochastic analysis tasks (`$q_*`) — not recognised, skipped; not filed
- [ ] 20.17 Programmable logic array tasks — not recognised; not filed
- [ ] 20.18.1 `$system` — not recognised, skipped; not filed

## 21 Input/output system tasks and system functions

**21.2 Display system tasks**
- [ ] 21.2.1 `$display` and `$write` argument model — a string variable first is a format; §5.2 Q-STRFMTARG
- [x] 21.2.1.1 Escape sequences for special characters
- [x] 21.2.1.2 Format `%d %h %x %o %b`
- [ ] 21.2.1.2 Format `%c` — x / z prints a NUL byte; §5.2 Q-PCTCXZ
- [ ] 21.2.1.2 Format `%f %e %g` — negative NaN unsigned; §5.2 Q-FMTNEGNAN, §5.2 Q-NANCANON
- [x] 21.2.1.2 Format `%t`
- [ ] 21.2.1.2 Format `%u %z` — write nothing; §5.2 P-PCTU, §5.2 Q-PCTUREC
- [ ] 21.2.1.2 Format `%l` — §3.c l-format-choice
- [x] 21.2.1.3 Size of displayed data
- [x] 21.2.1.4 Unknown and high-impedance values
- [ ] 21.2.1.5 Strength format `%v` — no strength model; not filed
- [ ] 21.2.1.6 Hierarchical name format `%m` — §5.2 Q-PCTM, §5.2 Q-ESCSCOPE
- [ ] 21.2.1.7 Assignment pattern format `%p` — §3.b %p-ⓐ, §3.b %p-ⓒ
- [x] 21.2.1.8 String format `%s`
- [ ] 21.2.2 Strobed monitoring `$strobe` — automatic task variable panics; §2 strobe-automatic-panic
- [ ] 21.2.3 Continuous monitoring `$monitor` — automatic task variable panics; §2 strobe-automatic-panic
- [x] 21.2.3 `$monitoron`, `$monitoroff`

**21.3 File input/output**
- [x] 21.3.1 `$fopen` as an assignment, `$fclose`, multichannel descriptors
- [ ] 21.3.1 `$fopen` in other expression positions — refused; not filed
- [x] 21.3.2 File output tasks (`$fdisplay`, `$fwrite`, `$fmonitor`, `$fstrobe`)
- [x] 21.3.3 Formatting to a string `$sformat`, `$swrite`
- [ ] 21.3.3 `$sformatf` as a task argument or in `?:` — refused; §5.2 26, §3.b sformatf
- [x] 21.3.4.1 Reading a character at a time (`$fgetc`, `$ungetc`)
- [ ] 21.3.4.2 Reading a line `$fgets` — `return $fgets(…)` refused; §3.b fgets-rhs
- [ ] 21.3.4.3 Formatted reads `$fscanf`, `$sscanf` — `%u %z %C`, x / z source; §5.2 Q-SCANXZSRC, §5.2 Q-PCTC-ENG
- [x] 21.3.4.4 Reading binary data `$fread`
- [ ] 21.3.4.4 `$fread` into a memory with a negative base — reads nothing; not filed
- [ ] 21.3.4 Placement of statement-effect reads — `&&`, `||`, `?:`, loop conditions; §3.a ③ⓐ, §3.a ③ⓑ
- [ ] 21.3.5 File positioning `$ftell`, `$fseek`, `$rewind` — §0-C A, §3.b filepos
- [x] 21.3.6 Flushing output `$fflush`
- [ ] 21.3.7 I/O error status `$ferror` — §0-C A
- [ ] 21.3.8 Detecting EOF `$feof` — §5.b FD-EOF + FEOF, §3.a ③ⓒ

**21.4–21.6 Memory files and command line**
- [ ] 21.4 `$readmemb`, `$readmemh` into a memory — bad characters skipped; §2 readmem-bad-token
- [ ] 21.4 `$readmemh` into an associative array — some addresses refused; §3.b readmem-assoc-loud
- [x] 21.4.1 Reading packed data
- [x] 21.4.2 Reading 2-state types
- [ ] 21.4.3 Multidimensional unpacked arrays — status not recorded
- [ ] 21.5 `$writememb`, `$writememh` — task-local array refused; §3.b writemem-local
- [x] 21.5.1 Writing packed data
- [x] 21.5.2 Writing 2-state types
- [ ] 21.5.3 Writing addresses to the output file — status not recorded
- [ ] 21.6 `$test$plusargs` — non-literal query; not filed
- [ ] 21.6 `$value$plusargs` — `%0d` refused, non-UTF-8 argv panics; §3.b plusargs-%0d, §5.2 P-ARGV

**21.7 Value change dump (VCD) files**
- [ ] 21.7.1.1 `$dumpfile` — a parameter name names the file; §2 dumpfile-param-name
- [ ] 21.7.1.2 `$dumpvars` — scope names and selection; §2 🆕 N
- [x] 21.7.1.3 `$dumpoff`, `$dumpon`
- [x] 21.7.1.4 `$dumpall`
- [x] 21.7.1.5 `$dumplimit`
- [x] 21.7.1.6 `$dumpflush`
- [ ] 21.7.2 Format of 4-state VCD file — cosmetic encoding differences; §3.b vcd
- [ ] 21.7.3 Extended VCD tasks (`$dumpports*`) — §7 VCD-EXT
- [ ] 21.7.4 Format of extended VCD file — §7 VCD-EXT
- [ ] 21.7.5 VCD SystemVerilog type mappings — generate scopes typed `module`; §3.b vcd

## 22 Compiler directives

**22.3–22.14**
- [ ] 22.3 `` `resetall `` — ignored; not filed
- [x] 22.4 `` `include ``
- [ ] 22.5.1 `` `define `` — macro formal gaps; §3.a ⑤ⓞ
- [x] 22.5.2 `` `undef ``
- [ ] 22.5.3 `` `undefineall `` — absent; not filed
- [x] 22.6 `` `ifdef ``, `` `else ``, `` `elsif ``, `` `endif ``, `` `ifndef ``
- [ ] 22.7 `` `timescale `` — partial-timescale diagnostics; §3.b timescale
- [x] 22.8 `` `default_nettype none ``
- [ ] 22.8 `` `default_nettype `` other net types — always a 1-bit wire; not filed
- [ ] 22.9 `` `unconnected_drive ``, `` `nounconnected_drive `` — ignored; not filed
- [ ] 22.10 `` `celldefine ``, `` `endcelldefine `` — ignored; not filed
- [ ] 22.11 `` `pragma `` and standard pragmas — ignored; not filed
- [ ] 22.12 `` `line `` — ignored in locations; §3.b line-directive
- [x] 22.13 `` `__FILE__ `` and `` `__LINE__ ``
- [ ] 22.14 `` `begin_keywords ``, `` `end_keywords `` — version does not narrow keywords; §3.b begin-keywords-version

## 23 Modules and hierarchy

**23.2 Module definitions**
- [x] 23.2.1 Module header definition
- [ ] 23.2.2.1 Non-ANSI style port declarations — signed redeclared unsigned; §2 🆕 AK, §3.b non-ansi-md-port
- [x] 23.2.2.2 ANSI style ports: input and output
- [ ] 23.2.2.2 `inout` ports — one-directional; not filed
- [ ] 23.2.2.3 Rules for port kind, data type and direction — `var` ports refused; §3.b decl-var-net-kw
- [ ] 23.2.2.4 Default port values — parsed, never lowered; §5.2 Q-PORTDEFAULT
- [x] 23.2.3 Parameterized modules
- [ ] 23.2.3 Empty parameter port list `#()` — refused; §7 56, §3.b empty-param-list

**23.3 Module instances**
- [x] 23.3.1 Top-level modules
- [ ] 23.3.1 `$root` — status not recorded
- [x] 23.3.2.1 Connecting ports by ordered list
- [x] 23.3.2.2 Connecting ports by name — stays checked by the box rule (an illegal accept is neither a refusal nor a wrong value): §2 port-named-twice, RECORD
- [x] 23.3.2.3 Implicit named port connections `.name`
- [x] 23.3.2.4 Wildcard named port connections `.*`
- [ ] 23.3.3.1 Port coercion — status not recorded
- [ ] 23.3.3.2 Port connection rules for variables — port drivers escape the check; §3.b mdrv-ca-port
- [ ] 23.3.3.3 Port connection rules for nets — disjoint port and assign drivers refused; §5.2 Q-MDPORTCOL
- [ ] 23.3.3.4 Port connection rules for interfaces — §3.b iface-port-forward, §7 54
- [ ] 23.3.3.5 Unpacked array ports and arrays of instances — §5.2 37, §3.b nonansi-child-array
- [ ] 23.3.3.5 Instance array header binding — §5.2 1, §2 🆕 AH
- [ ] 23.3.3.6 Single source nets (`uwire`) — not enforced; not filed
- [ ] 23.3.3.7 Dissimilar net types (net and port collapsing) — status not recorded
- [x] 23.3.3.8 Connecting signed values via ports

**23.4–23.5 Nested and extern modules**
- [ ] 23.4 Nested modules — absent; not filed
- [ ] 23.5 Extern modules — absent; not filed

**23.6–23.9 Hierarchical names and scopes**
- [ ] 23.6 Hierarchical reads — leaves lack the child's environment; §2 hier-leaf-inline-region, §5.2 75
- [x] 23.6 Hierarchical procedural write of a whole variable
- [ ] 23.6 Hierarchical element, part-select or whole-array write — refused; §7 59
- [ ] 23.6 Hierarchical `force` and `release` — §5.2 Q-HIERFORCE
- [ ] 23.6 Hierarchical function and task calls — §3.b hier-task, §3.b hier-default-arg
- [ ] 23.7 Member selects and hierarchical names — §5.2 70, §2 md-param-hier-element
- [ ] 23.7.1 Package or class scope prefixes — §3.b pkg-scoped-task, §5.2 Q-PKGPMEMB
- [x] 23.8 Upwards name referencing
- [ ] 23.8.1 Task and function name resolution — status not recorded
- [ ] 23.9 Scope rules — names bound in the wrong scope; §2 declaration-order-accept, §2 struct-binding-by-name, §5.2 Q-SCOPEBIND

**23.10–23.11 Overriding parameters, bind**
- [ ] 23.10.1 `defparam` — direct child, constant only; §8 defparam-depth, §0 14-b, §3.b defparam-iface
- [ ] 23.10.2.1 Parameter value assignment by ordered list — x / z plane lost; §5.2 Q-OVRXPLANE, §2 16
- [ ] 23.10.2.2 Parameter value assignment by name — x / z plane lost; §5.2 Q-OVRXPLANE, §2 17
- [x] 23.10.3 Parameter dependence
- [ ] 23.10.4.1 Order of elaboration — generate-scope parameters across walks; §5.2 77, §2 🆕 V
- [ ] 23.10.4.2 Early resolution of hierarchical names — §5.2 75
- [ ] 23.11 Binding auxiliary code (`bind`) — actual's struct layout; §2 bind-actual-lexical-layout

## 24 Programs

**24.3–24.7**
- [ ] 24.3 The program construct — parsed as a module; not filed
- [ ] 24.3.1 Scheduling semantics of program code — Active, not Reactive; not filed
- [ ] 24.3.2 Program port connections without clocking blocks — status not recorded
- [ ] 24.4 Eliminating testbench races — not filed
- [ ] 24.5 Blocking tasks in cycle/event mode — status not recorded
- [ ] 24.6 Programwide space and anonymous programs — status not recorded
- [ ] 24.7 Program control tasks (`$exit`) — same as `$finish`; not filed

## 25 Interfaces

**25.3–25.10**
- [ ] 25.3 Interface declaration and instance — member widths through ports; §2 interface-member-port-width
- [ ] 25.3 Generate, typedef, nested instances, defparam in an interface — refused; §3.b iface-generate
- [x] 25.4 ANSI interface header ports
- [ ] 25.4 Non-ANSI or interface-typed header ports, interface instance arrays — refused; not filed
- [x] 25.5 Modport declaration with directions enforced
- [ ] 25.5 Modport names and modport actuals — §2 modport-import-name, §3.b modport-port-actual
- [ ] 25.5.4 Modport expressions — §7 54
- [ ] 25.5.5 Clocking blocks and modports — status not recorded
- [ ] 25.6 Interfaces and specify blocks — §3.b specify
- [x] 25.7 Tasks and functions declared in an interface
- [ ] 25.7 Modport import and export of routines — §3.b iface-modport-formal
- [x] 25.8 Parameterized interfaces
- [ ] 25.8 Interface parameter read through an interface port in a constant — §3.b interface-port-parameter-const
- [ ] 25.9 Virtual interfaces — static alias only; not filed
- [ ] 25.9.1 Virtual interfaces and clocking blocks — status not recorded
- [ ] 25.10 Access to interface objects — §2 interface-facts-declined

## 26 Packages

**26.2–26.7**
- [ ] 26.2 Package parameters — a value calling a function refused; §5.2 45, §3.b pkg-param-own-fn
- [ ] 26.2 Package typedefs — named dimensions fold where used; §5.2 46
- [ ] 26.2 Package functions and tasks — text binds the caller's names; §5.2 Q-RTNTEXT, §5.2 2
- [x] 26.2 Package variables read and written from a module
- [ ] 26.2 Package string, real, event, class and dynamic items — refused; not filed
- [ ] 26.2 Duplicate names in a package body — accepted; §5.2 Q-PKGDUPNAME
- [x] 26.3 `pkg::name` of a parameter, enum label or variable
- [ ] 26.3 `pkg::t()` task enable — refused at parse; §3.b pkg-scoped-task
- [ ] 26.3 Explicit import — two of one type or function name take the second; §2 explicit-import-conflict
- [ ] 26.3 Wildcard import — ambiguity by adjacency; §2 import-ambiguity-adjacency, §5.2 Q-AMBIMPORT
- [ ] 26.3 Imported routines — free names bind the caller; §2 imported-routine-free-name, §2 reimported-routine-binding
- [x] 26.4 Using packages in module headers
- [ ] 26.5 Search order rules — §2 unit-module-import-scope
- [ ] 26.6 Exporting imported names from packages — refused at parse; §5.2 39
- [ ] 26.7 The std built-in package — absent; not filed

## 27 Generate constructs

**27.3–27.6**
- [x] 27.3 Generate regions and the optional `generate` keyword
- [ ] 27.3 Genvar declaration — reusing a variable's name accepted; §5.2 Q-GENVARALIAS
- [ ] 27.3 Items in generate blocks (parameters, typedefs, routines) — §5.2 77, §3.b gen-enum-uncarried, §3.b gen-rtn-edges
- [x] 27.4 Loop generate constructs
- [x] 27.5 Conditional generate `if`
- [ ] 27.5 Conditional generate `case` — label compares; §5.2 7, §2 gen-case-label-unfoldable
- [ ] 27.5 String-valued parameters in a condition — §5.2 50, §3.b string-literal-condition-residue
- [ ] 27.5 Constant calls in a generate block — bind the module's function; §5.2 3, §2 🆕 AG
- [ ] 27.6 External names for unnamed generate blocks — scope naming; §2 🆕 N

## 28 Gate-level and switch-level modeling

**28.3–28.10 Gates and switches**
- [x] 28.3.1 Gate type specification
- [ ] 28.3.2 Drive strength specification — refused at parse; not filed
- [x] 28.3.3 Delay specification — stays checked by the box rule (an illegal accept is neither a refusal nor a wrong value): `#1'bx` accepted, §3.b gate-delay-based-literal
- [x] 28.3.4 Primitive instance identifier
- [ ] 28.3.5 Range specification (gate instance arrays) — status not recorded
- [x] 28.3.6 Primitive instance connection list
- [x] 28.4 `and`, `nand`, `nor`, `or`, `xor`, `xnor` gates
- [ ] 28.5 `buf` and `not` gates — a `z` input at time 0; §2-N buf-z-input
- [x] 28.6 `bufif1`, `bufif0`, `notif1`, `notif0` gates
- [ ] 28.5 Gate driving a non-`logic` variable — refused; §3.b cont-var-gate
- [ ] 28.7 MOS switches — absent; not filed
- [ ] 28.8 Bidirectional pass switches — absent; not filed
- [ ] 28.9 CMOS switches — absent; not filed
- [ ] 28.10 `pullup` and `pulldown` sources — absent; not filed

**28.11–28.16 Strengths and delays**
- [ ] 28.11 Logic strength modeling — no strength model; not filed
- [ ] 28.12 Strengths and values of combined signals — no strength model; not filed
- [ ] 28.13 Strength reduction by nonresistive devices — no switches; not filed
- [ ] 28.14 Strength reduction by resistive devices — no switches; not filed
- [ ] 28.15 Strengths of net types — refused net types; §8 non-goal-list
- [ ] 28.16.1 min:typ:max delays — typ always taken; not filed
- [ ] 28.16.2 Trireg net charge decay — §8 non-goal-list

## 29 User-defined primitives

**29.3–29.10**
- [ ] 29.3.1 UDP header — ANSI port declarations refused; §3.b udp-ansi-header
- [x] 29.3.2 UDP port declarations
- [x] 29.3.3 Sequential UDP initial statement
- [x] 29.3.4 UDP state table
- [x] 29.3.5 Z values in UDP
- [x] 29.3.6 Summary of symbols: level, edge, next-state
- [ ] 29.3.6 `b` and `*` inside an explicit `(vw)` edge — refused; not filed
- [x] 29.4 Combinational UDPs
- [x] 29.5 Level-sensitive sequential UDPs
- [x] 29.6 Edge-sensitive sequential UDPs
- [ ] 29.7 Sequential UDP initialization — time 0 missed with declaration-initialized inputs; §2 udp-decl-init-t0
- [ ] 29.8 UDP instances — an unnamed instance refused; §3.b udp-unnamed-instance
- [x] 29.9 Mixing level-sensitive and edge-sensitive descriptions
- [x] 29.10 Level-sensitive dominance

## 30 Specify blocks

**30.3–30.7**
- [ ] 30.3 Specify block declaration — refused at parse; §3.b specify
- [ ] 30.4 Module path declarations — refused at parse; §3.b specify
- [ ] 30.5 Assigning delays to module paths — refused at parse; §3.b specify
- [ ] 30.6 Mixing module path delays and distributed delays — refused at parse; §3.b specify
- [ ] 30.7 Detailed control of pulse filtering behavior — refused at parse; §3.b specify

## 31 Timing checks

**31.3–31.9**
- [ ] 31.3 Stability-window checks (`$setup`, `$hold`, `$setuphold`, `$removal`, `$recovery`, `$recrem`) — §3.b specify
- [ ] 31.4 Clock and control checks (`$skew`, `$timeskew`, `$fullskew`, `$width`, `$period`, `$nochange`) — §3.b specify
- [ ] 31.5 Edge-control specifiers — §3.b specify
- [ ] 31.6 Notifiers — §3.b specify
- [ ] 31.7 Conditioned events — §3.b specify
- [ ] 31.8 Vector signals in timing checks — §3.b specify
- [ ] 31.9 Negative timing checks — §3.b specify

## 32 Backannotation using the standard delay format

**32.3–32.9**
- [ ] 32.3 The SDF annotator — out of scope; §8 non-goal-list
- [ ] 32.4 Mapping of SDF constructs to SystemVerilog — out of scope; §8 non-goal-list
- [ ] 32.5 Multiple annotations and 32.6 multiple SDF files — out of scope; §8 non-goal-list
- [ ] 32.7 Pulse limit annotation — out of scope; §8 non-goal-list
- [ ] 32.8 SDF to SystemVerilog delay value mapping — out of scope; §8 non-goal-list
- [ ] 32.9 Loading timing data `$sdf_annotate` — skipped with a warning; §8 non-goal-list

## 33 Configuring the contents of a design

**33.3–33.7**
- [ ] 33.3 Libraries — refused at parse; not filed
- [ ] 33.4 Configurations — refused at parse; not filed
- [ ] 33.5 Using libraries and configs — refused at parse; not filed
- [ ] 33.7 Displaying library binding information — status not recorded

## 34 Protected envelopes

**34.3–34.5**
- [ ] 34.3 Processing protected envelopes — no roadmap row
- [ ] 34.4 Protect pragma directives — `` `pragma `` ignored; no roadmap row
- [ ] 34.5 Protect pragma keywords — no roadmap row

## 35 Direct programming interface

**35.3–35.9**
- [ ] 35.3 Two layers of DPI — absent; §3.a ⑮
- [ ] 35.4 Global name space of imported and exported functions — absent; §3.a ⑮
- [ ] 35.5 Imported tasks and functions — absent; §7 61, §7 62
- [ ] 35.6 Calling imported functions — absent; §7 61
- [ ] 35.7 Exported functions — absent; §7 62
- [ ] 35.8 Exported tasks — absent; §7 62
- [ ] 35.9 Disabling DPI tasks and functions — absent; §7 62

## 36 Programming language interface (PLI/VPI) overview

**36.3–36.11**
- [ ] 36.3–36.5 User-defined system tasks and functions — no roadmap row
- [ ] 36.6 User-supplied PLI applications — no roadmap row
- [ ] 36.8 VPI sizetf, compiletf and calltf routines — no roadmap row
- [ ] 36.9 PLI mechanism — no roadmap row
- [ ] 36.10 VPI access to SystemVerilog and simulation objects — no roadmap row

## 37 VPI object model diagrams

**37.2–37.83**
- [ ] 37 VPI object model — no roadmap row

## 38 VPI routine definitions

**38.2–38.38**
- [ ] 38 VPI routines — no roadmap row

## 39 Assertion API

**39.3–39.5**
- [ ] 39.3 Static information — no roadmap row
- [ ] 39.4 Dynamic information — no roadmap row
- [ ] 39.5 Control functions — no roadmap row

## 40 Code coverage control and API

**40.3–40.5**
- [ ] 40.3 Real-time coverage access — no roadmap row
- [ ] 40.4 FSM recognition — no roadmap row
- [ ] 40.5 VPI coverage extensions — no roadmap row

## 41 Data read API

**41**
- [ ] 41 Data read API — no roadmap row

## Annexes

**Annex A–N** (A: leaves in clauses 3–41 ; F: leaves in 16 ; O–Q informative)
- [ ] B Keywords — status not recorded
- [ ] C Deprecation — status not recorded
- [ ] D Optional system tasks and system functions — not recognised, skipped; not filed
- [ ] E Optional compiler directives — absent; not filed
- [ ] G Std package — absent; not filed
- [ ] H DPI C layer — §7 61, §7 62
- [ ] I `svdpi.h` — §7 61
- [ ] J Inclusion of foreign language code — §7 61
- [ ] K `vpi_user.h` — no roadmap row
- [ ] L `vpi_compatibility.h` — no roadmap row
- [ ] M `sv_vpi_user.h` — no roadmap row
- [ ] N Algorithm for probabilistic distribution functions — §5.2 Q-DISTN
