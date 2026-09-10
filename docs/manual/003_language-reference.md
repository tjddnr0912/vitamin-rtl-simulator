# Language Reference

This chapter answers one question for every construct of IEEE 1364 (Verilog) and
IEEE 1800 (SystemVerilog): does vitamin support it, refuse it loudly, or not
accept it at all. It is organised as matrices, one per language area, so a
construct can be looked up directly.

Related chapters: [Installation](001_installation.md) ·
[Quick Start](002_quickstart.md) · [CLI Reference](004_cli-reference.md) ·
[System Tasks](005_system-tasks.md) · [Limitations](006_limitations.md) ·
[Error Codes](007_error-codes.md).

## 1. Status vocabulary and refusal channels

Every row in this chapter carries one of five statuses.

| Status | Meaning |
|---|---|
| **Supported** | Parses, elaborates and simulates with the semantics the standard defines. |
| **Partial** | Accepted, with the simplification or restricted shape named in the row. |
| **Loud** | Refused with a diagnostic and exit 1. The construct is never simulated as something else. |
| **Ignored** | Accepted and stripped. No semantics, no diagnostic. |
| **Absent** | No grammar arm. The text lexes as an ordinary identifier and the parse fails. |

A refusal arrives through one of three channels. The message is always specific
about what was refused and why.

| Channel | Code | Stage | Effect |
|---|---|---|---|
| `E-PARSE-UNEXPECTED-TOKEN` | `VITA-E2002` | parse | `expected <what>, found '<token>'`; exit 1 |
| `E-ELAB-UNSUPPORTED` | `VITA-E3009` | elaborate | the design is not simulated; exit 1 |
| `W-ELAB-FEATURE-LIMIT` | `VITA-W3056` | elaborate | legal construct accepted but simplified; the run continues |

Other codes named in the matrices — `VITA-E3001` (multi-driver), `VITA-E3002`
(port or modport mismatch), `VITA-E3010` (undeclared name), `VITA-E3018`
(assignment kind), `VITA-E4001` (assertion failure), `VITA-E4002` (runtime index
range) — are listed in full in [Error Codes](007_error-codes.md). Each code has
three interchangeable spellings for `vita explain`, `-Wno-` and `-Werror=`: the
mnemonic (`E-ELAB-UNSUPPORTED`), the printed number (`VITA-E3009`) and the bare
number (`E3009`).

Platforms are Linux and macOS. See [Installation](001_installation.md).

## 2. Design units

| Construct | Status | Notes |
|---|---|---|
| `module` … `endmodule` | Supported | The primary design unit. |
| `macromodule` … `endmodule` | Supported | Treated as `module`; the keyword is recorded on the declaration. |
| ANSI port header | Supported | `module m (input logic a, output reg [7:0] q);` |
| Non-ANSI port header | Supported | `module m (a, q); input a; output [7:0] q;` |
| `module m;` (no port list) | Supported | |
| `primitive` … `endprimitive` | Supported | User-defined primitive; see §17. |
| `interface` / `endinterface` / `modport` | Partial | See §16. |
| `package` / `endpackage`, `import` | Supported | See §16. |
| `program` / `endprogram` | Partial | Parses into the module form and elaborates as a top-level container. IEEE 1800 §24 Reactive-region scheduling of program processes is approximated as the Active region. |
| `class` / `endclass` | Supported | Top level, module body or package body; see §11. |
| `bind <target> <checker> <inst> (…)` | Supported | A contextual keyword. The bound instance is an ordinary module instance; binds written in a body are hoisted to top level. A checker with an `output`/`inout` port is Loud (`VITA-E3009`) so a bound instance can never drive a target net. |
| Compilation-unit scope `typedef` / `parameter` / `localparam` / `function` / `task` / `import` before the first `module` | Supported | Visible to every unit that follows; a same-named declaration inside a design unit shadows it. A unit `parameter` is a `localparam`. |
| `$unit::name` scope prefix | Absent | `VITA-E2002` |
| `config` / `endconfig`, `library`, `liblist`, `cell`, `design`, `use`, `incdir`, `instance` | Loud | Lexed as keywords, dispatched nowhere: `VITA-E2002` `expected 'module'`. |
| `specify` / `endspecify` (path delays, timing checks) | Absent | `VITA-E2002` |
| Nested modules, `extern module` | Absent | `VITA-E2002` |
| `import "DPI-C" function …` | Absent | `VITA-E2002` cascade beginning `expected identifier, found '"DPI-C"'`. |

### 2.1 Ports

```systemverilog
// ANSI
module adder #(parameter W = 8)
              (input  logic [W-1:0] a, b,
               output logic [W-1:0] sum);
  assign sum = a + b;
endmodule

// Non-ANSI
module adder (a, b, sum);
  parameter W = 8;
  input  [W-1:0] a, b;
  output [W-1:0] sum;
  assign sum = a + b;
endmodule
```

The first token inside `(` decides the header style: a direction keyword makes
it ANSI, a bare identifier makes it non-ANSI, and `Ident Ident` or
`Ident . Ident` makes it an interface-typed ANSI port.

| Construct | Status | Notes |
|---|---|---|
| Packed dimensions on an ANSI port (`output logic [3:0][7:0] o`) | Supported | |
| Unpacked dimensions after the port name (`output logic [7:0] o [4]`) | Supported | |
| `signed` on a port | Supported | |
| Interface-typed ANSI port (`intf p`, `intf.mp p`) | Supported | Bound by symbol aliasing, not by wiring. |
| Unpacked-array port | Partial | Wired element by element. Both sides must be arrays of the same per-dimension shape and direction; a mismatch is Loud (`VITA-E3009`), because IEEE 1800 §7.6 pairs elements positionally and a flat-index connection would reverse the order. |
| `inout` port | Partial | Approximated as one-directional, parent to child (`VITA-W3056`). |
| Unconnected port | Supported | A warning names the direction; an unconnected input floats at `z` and every value derived from it is unknown. |
| Drive strength on a port, assign or gate (`(strong1, strong0)`) | Loud | `VITA-E2002` |
| `vectored` / `scalared` | Loud | `VITA-E2002` |
| Port declaration inside `generate` | Loud | `VITA-E3009` `port declaration not allowed inside generate` |

### 2.2 Instantiation and hierarchy

| Construct | Status | Notes |
|---|---|---|
| Positional connections `u (a, b, c)` | Supported | |
| Named connections `.p(e)` | Supported | |
| Empty named connection `.p()` | Supported | Treated as unconnected. |
| `.name` shorthand | Supported | Expands to `.name(name)`. The same-named signal must be declared: a missing one is a hard error, never an implicit net. |
| `.*` wildcard | Supported | Every unlisted port connects to the same-named signal in the instantiating scope. A same-named constant, or a missing name, is Loud. |
| Instance array `dff u[3:0] (…)` | Partial | The child must have ANSI ports and exactly one constant `[msb:lsb]` range; the cap is 4096 elements. A connection must be a plain identifier of the port width or of the whole array width. Interface-typed ports on the child are Loud. |
| `.*` on an instance array | Loud | `VITA-E3009` |
| Recursive module instantiation | Loud | `VITA-E3009`, naming the cycle. |
| Unresolvable module | Loud | `E-ELAB-UNRESOLVED-INSTANCE` / `VITA-E3003` |
| Incompatible port binding | Loud | `E-ELAB-PORT-MISMATCH` / `VITA-E3002` |

### 2.3 Hierarchical references

| Form | Status | Notes |
|---|---|---|
| Read of a net or parameter (`u.x`, `u.g.P`, `u.v.P`) | Supported | A parameter read folds to its constant, including one wider than 64 bits. A bit, part or indexed-part select of a parameter and `$bits(u.P)` / `$bits(u.net)` fold too. |
| Procedural write (`u.x = v;`, `u.mem[i] = v;`, `u.x[3:0] = v;`) | Supported | In any direction: into a child, up into a parent, or through the module's own full path. |
| Continuous write (`assign u.x = v;`) | Supported | On a `wire`. A procedural write to a hierarchical `wire` is Loud (`VITA-E3018`), as Icarus refuses it. |
| Function call (`u1.f(x)`), task enable | Supported | Resolved in a deferred pass. |
| Function call with named arguments | Loud | `VITA-E3009` `hierarchical function call with named arguments (deferred)` |
| Element or part-select write | Loud | `VITA-E3009`. A whole-net hierarchical write is the supported shape. |
| Read or write of a named event, a dynamic handle, or a whole unpacked array | Loud | `VITA-E3009` — none of them has a plain whole-net value. |
| Select of an ascending (`[0:7]`) hierarchical parameter, or of one sized from its value | Loud | `VITA-E3009`; read the whole parameter. |
| Hierarchical name in an event control | Loud | `VITA-E3009`. Reading is supported, so `always @(*) local = u.x;` and a trigger on `local` work. |
| Reference to an `automatic` block-local | Loud | `VITA-E3009` — IEEE 1800 §23.9 gives per-entry storage no hierarchical address. |
| `disable` by hierarchical path | Loud | `VITA-E3009`; see §9. |

### 2.4 Parameters and overrides

| Construct | Status | Notes |
|---|---|---|
| `parameter` in a body or an ANSI `#( … )` header | Supported | |
| `localparam` | Supported | |
| `specparam` | Partial | Parsed as a `localparam`, and not restricted to a `specify` block. |
| Comma list sharing one type prefix (`localparam [3:0] A = 1, B = 2;`) | Supported | |
| Positional override `#(8)` | Supported | |
| Named override `#(.W(8))` | Supported | |
| `defparam inst.param = const;` | Partial | Direct-child target only (exactly two path segments), last write wins per IEEE 1800 §23.10.1, and it beats a `#()` override. A longer path, a non-constant value, or a `defparam` inside `generate` is Loud (`VITA-E3009`). A target matching no instance warns and is ignored. |
| CLI top-level override `-G NAME=VALUE` | Supported | Applies to the top module only; see [CLI Reference](004_cli-reference.md). |
| Override carrying a real value, a non-constant, or an x/z-bearing value | Loud | `VITA-E3009` — the declared default would otherwise be used silently, which is a different design. |
| Type parameter `parameter type T = logic [7:0]` | Partial | Integral vector subset: `logic`/`reg`/`bit` with one packed range, the 2-state atoms, `time`, an integral vector typedef, another type parameter, or an unpacked-array typedef. `T` declares variables, ports, struct members, subroutine formals and returns, `typedef T u_t;`, `T'(e)` and `$bits(T)`, and is overridden by `.T(logic [15:0])`, positionally, by a typedef, by `pkg::t` or by `.T(T)`. An override may change the width; one that changes the signedness, the 2-state kind or the dimension count is refused. A struct, enum, union, real, string, class or multi-dimensional type as the default or the override is Loud (`VITA-E2002`), as is a class `#(type T)`. |
| Parameter typed by a `typedef` | Supported | Vector, signed, atom, packed struct, packed union, enum, and the scoped `pkg::t` spelling; bindings follow `import pkg::*`. |
| Array parameter (`localparam st_t P[N] = '{ … }`) | Partial | A body `localparam`, a package `parameter` or `localparam`, a generate-scope declaration, and an ANSI header array parameter with instance override. A module-body overridable `parameter` array, a multi-dimensional array parameter, a multi-dimensional packed element type, and a `defparam` or `-G` onto an array parameter are Loud. |
| Multi-dimensional packed parameter (`parameter logic [N-1:0][M-1:0] P`) | Supported | Body, ANSI header (default and instance override), and package. Reads `P[i]`, `P[i][j]`, `P[i][a:b]`, `P[i][o+:w]`, `P[a:b]`, `$bits(P)`, `$bits(P[i])`. A `'{…}` value for such a parameter, and `$size`/`$left`/`$dimensions` on it, are Loud. |
| Class-handle parameter | Loud | `VITA-E2002` |

An UNTYPED parameter takes its type from its value (IEEE 1800 §6.20.2), and when
that value is an operator the width is the operator's own (Table 11-21), not the
magnitude of the result: `~8'h5A` is 8 bits, `8'hFF << 8'd9` is 8 bits, a
comparison is one bit, and a bitwise pair takes its wider operand. `+`, `-` and
`*` follow the same rule, so an untyped sum wraps at its operand width —
`localparam Q = 8'd200 + 8'd100` is 44, and `parameter A = 1 << 32` is 0.
Declare a width (`localparam [31:0] Q = …`) to keep the wider value. A declared
width wins, and so does an override's.

### 2.5 Generate

| Construct | Status | Notes |
|---|---|---|
| `generate` / `endgenerate` | Supported | The keywords are optional (IEEE 1800 §27.3): a bare `if`, `for` or `case` at module-item position is a generate construct. |
| `genvar i, j;` | Supported | |
| `for (genvar i = 0; i < 4; i++)` header declaration | Supported | IEEE 1800 §27.4. |
| Generate `if` / `case` / `for`, labelled `begin : name` | Supported | |
| Hierarchical reference into a generate scope (`g[0].z`, `cg.c`) | Supported | |
| `function` / `task` inside `generate` | Supported | IEEE 1800 §27.3. The routine belongs to the block's scope: only the taken branch of a generate-`if` declares one, a generate-`for` body declares one per iteration (its own genvar value), a bare call resolves innermost-first (so it shadows a same-named module routine) and `%m` names the declaring block (`t.u.g.show`). |
| Calling a generate-scoped routine from OUTSIDE its block | Loud | Not visible by bare name (`VITA-E3010` `call to undeclared function`/`task`); a hierarchical `u.g.f(x)` is `VITA-E3009` `unsupported hierarchical function call`. |
| Generate-scoped routine in a constant expression | Loud | `localparam W = f(N)` inside the block is `VITA-E3009` `… value is not a constant` — the elaborate-time const-function interpreter reads module-body declarations only. |
| `defparam` inside `generate` | Loud | `VITA-E3009` ``a `defparam` inside a generate block is deferred`` |
| `import` inside `generate` | Loud | `VITA-E3009` unless it is redundant with a module-scope import of the same package. |
| Non-advancing genvar step | Loud | `VITA-E3009` `generate-for genvar does not advance (step leaves it unchanged)` |
| Unroll beyond the caps | Loud | See §19. |

## 3. Preprocessor

The preprocessor is a text-to-text transform with byte-offset provenance. It
recognises exactly nineteen directive names; every other backtick word is a
macro use, and an undefined one is `E-PP-BAD-DIRECTIVE` / `VITA-E1013`
(``undefined macro use `NAME``).

| Directive | Status | Behaviour |
|---|---|---|
| `` `define `` object-like | Supported | Never consumes a following `(`. |
| `` `define `` function-like | Supported | Function-like only when `(` immediately follows the name. Named formals and IEEE 1800 §22.5.1 default argument values (`` `define M(a, b = 5) ``); a default may be a literal, an expression, parenthesised text, a string, a replication or another macro, and binds at use time. |
| `` `undef `` | Supported | Removing an undefined macro warns `W-PP-UNDEF-UNDEFINED` / `VITA-W1008`. |
| `` `include "path" `` | Supported | The path is macro-expanded first; the search order is the including file's own directory, then each `-I` directory in order. |
| `` `ifdef `` / `` `ifndef `` / `` `elsif `` / `` `else `` / `` `endif `` | Supported | Nested; at file level and inside a macro body, where the group is evaluated when the body expands. `` `elsif `` after `` `else ``, and a duplicate `` `else ``, are errors. |
| `` `timescale unit/precision `` | Supported | See §18. |
| `` `default_nettype `` | Partial | Only `none` changes behaviour. Every other argument means "implicit nets allowed"; the named type is not used, and an implicit net is always a 1-bit `wire`. |
| `` `__FILE__ `` | Supported | The using file's name as given on the command line, as a string literal. |
| `` `__LINE__ `` | Supported | The line of the use; inside a macro body, the line where the use's argument list closes. |
| `` `line `` | Ignored | The whole logical line is consumed. Line numbers are not re-mapped. |
| `` `pragma `` | Ignored | The whole logical line is consumed. |
| `` `celldefine `` / `` `endcelldefine `` | Ignored | The directive token is stripped. |
| `` `resetall `` | Ignored | The token is stripped. Macros, `` `timescale `` and `` `default_nettype `` keep their state. |
| `` `begin_keywords "spec"`` / `` `end_keywords `` | Ignored | One logical line is consumed; the full keyword set applies regardless. |
| `` `unconnected_drive `` / `` `nounconnected_drive `` | Ignored | Drive state is not modelled. |
| Any other backtick word (`` `undefineall ``, `` `protect ``, `` `delay_mode_path ``, …) | Absent | `VITA-E1013` |

Macro semantics:

| Rule | Behaviour |
|---|---|
| Function-like macro used without `(` | Loud `E-PP-MACRO-ARITY` / `VITA-E1002` |
| Too many actuals, or an omitted actual whose formal has no default | Loud `VITA-E1002` |
| Blank actual (whitespace and comments only) | Takes the formal's default. |
| Actual pre-expansion | Each actual expands to completion with the macro out of the active set; the substituted body is re-scanned with the name held active. |
| Recursion | Loud `E-PP-RECURSIVE-MACRO` / `VITA-E1004` |
| Token paste `` `` `` | Deletes the operator so the tokens abut; adjacent whitespace is not trimmed. |
| Stringify `` `" … `" `` | Emits a real `"` and substitutes parameters inside; `` `\`" `` becomes `\"`. |
| Line continuation in a body | The `\` is dropped and the newline is kept, so a body stays one directive per line. |
| Trailing `// comment` in a body | Dropped; a comment line ending in `\` continues the body. |
| Commas inside parens or strings | Not argument separators. |
| Backtick inside a string literal | Not expanded. |
| Redefinition with different text | `W-PP-MACRO-REDEFINED` / `VITA-W1007`; identical text is silent. |
| Defining or undefining a directive keyword | Loud `E-PP-BAD-DIRECTIVE` / `VITA-E1013` |

Caps and command-line input: macro depth 256, include depth 64, expanded output
256 MiB (exceeding it is `VITA-E1004`). An include cycle is
`E-PP-RECURSIVE-INCLUDE` / `VITA-E1005`; a missing include is
`E-PP-INCLUDE-NOT-FOUND` / `VITA-E1001`. An unterminated `` `ifdef `` at end of
file produces one `VITA-E1013` per unclosed frame, anchored at its own opening
line. `-D NAME[=text]`, `-I dir` and the plusarg spellings `+define+A+B` and
`+incdir+d1+d2` feed the preprocessor; `-D NAME` with no `=` defines an empty
body.

## 4. Lexical elements

| Element | Status | Rule |
|---|---|---|
| Simple identifier | Supported | `[A-Za-z_][A-Za-z0-9_$]*`. `$` is legal in the interior, never first. |
| Keywords | Supported | Case-sensitive, lowercase only: `Module` and `Wire1` are identifiers. |
| Escaped identifier `\name<ws>` | Partial | Hand-scanned: `\` plus a printable run, terminated by one whitespace character. The backslash is kept in the name, so `\foo ` and `foo` are different names where IEEE 1364 §3.7.1 spells one identifier. |
| Escaped identifier with an empty body | Loud | `VITA-E2002` |
| System task or function `$name` | Supported | The `$` is part of the name. |
| Bare `$` | Supported | The queue last-index token (`q[$]`). |
| Bare backtick | Loud | `VITA-E2002` |
| `// … <newline>` comment | Supported | |
| `/* … */` comment | Supported | Non-nesting: the first `*/` closes. An unterminated one is Loud (`VITA-E2002`). |
| Attribute instance `(* … *)` | Ignored | Recognised over the token stream and deleted; `(` and `*` must be adjacent, so `@(*)` is never an attribute opener. An opener with no closer is Loud (`VITA-E2002`). |

Number literals:

| Form | Regular shape | Examples |
|---|---|---|
| Sized based | `\d[\d_]*[ \t]*'[sS]?[dDbBoOhH][ \t]*[0-9a-fA-FxXzZ?_]+` | `8'hAB`, `4'sd5`, `32'h 0000_0000` |
| Unsized based | `'[sS]?([dDbBoOhH][ \t]*digits\|[01xXzZ])` | `'hFF`, `'sd9`, `'0`, `'1`, `'x`, `'z` |
| Decimal | `\d[\d_]*` | `42`, `1_000` |
| Real, exponent | `\d[\d_]*(\.\d[\d_]*)?[eE][+-]?\d[\d_]*` | `1.5e3`, `2.5E-4` |
| Real, fixed | `\d[\d_]*\.\d[\d_]*` | `3.14`, `0.5` |

White space between size and base and between base and value is admitted for
spaces and tabs only, never newlines (IEEE 1800 §5.7.1). The unsized fill forms
`'0 '1 'x 'z` admit no space after the apostrophe. `1.` and `.5` are not reals —
both sides need at least one digit — and the parser refuses them. `?` is an
alias for `z` in every based-digit class. A decimal-based literal accepts `x` or
`z` only as the whole value. A zero size (`0'h1`) is malformed. A literal whose
resolved width exceeds the net-width cap, or a decimal magnitude longer than
315 656 digits, is Loud (`VITA-E3009`).

String literals are UTF-8 bytes in IEEE 1800 §5.9 order — the first character is
the most significant byte, the width is `8 × nbytes`, and the value is 2-state,
so `"ab"` is `16'h6162`. Table 5-1 escapes (`\n \t \v \f \a \\ \"`), octal
`\ddd` (one to three digits) and hex `\xhh` (one or two digits) are supported.
`\r` yields 0x0D and any other `\X` keeps both characters; both spellings warn
once per literal and escape with `W-ELAB-STR-ESCAPE` / `VITA-W3059`, because
other tools read them differently. An unterminated string is Loud
(`VITA-E2002`).

## 5. Data types

### 5.1 Net and variable kinds

The parser accepts all twenty-four declaration keywords; elaborate supports the
subset below. A keyword marked Loud parses and is then refused with
`VITA-E3009` `unsupported net/var kind (v1)`.

| Keyword | Standard | Status | Notes |
|---|---|---|---|
| `wire` | Verilog | Supported | 4-state; time 0 is all-`z`. |
| `tri` | Verilog | Partial | Behaves exactly as `wire`; there is no strength model. |
| `uwire` | Verilog | Partial | Behaves as `wire`. |
| `wand` | Verilog | Supported | True IEEE 4-state wired-AND resolution on multiple drivers. |
| `wor` | Verilog | Supported | True wired-OR resolution. |
| `triand`, `trior`, `tri0`, `tri1`, `supply0`, `supply1`, `trireg` | Verilog | Loud | `VITA-E3009` |
| `reg` | Verilog | Supported | 4-state; time 0 is all-`x`. |
| `logic` | SystemVerilog | Supported | 4-state; accepts both continuous and procedural writes. |
| `integer` | Verilog | Supported | 32 bits, signed by default, 4-state. |
| `time` | Verilog | Supported | 64 bits, always unsigned — an explicit `signed` is dropped. As a parameter the width and unsignedness come from the declaration, never from the initializer: `localparam time A = 8'd5` is 64 bits, and `localparam time B = -8'sd2` reads `18446744073709551614`. |
| `real` / `realtime` | Verilog | Supported | IEEE-754 double, always signed, 2-state; time 0 is `+0.0`. `realtime` is a synonym. |
| `event` | Verilog | Supported | Desugars to a 64-bit unsigned counter starting at 0; see §9.2. |
| `bit`, `byte`, `shortint`, `int`, `longint` | SystemVerilog | Partial | 2-state, widths 1/8/16/32/64, time 0 is 0. `byte`/`shortint`/`int`/`longint` are signed by default; `bit` is unsigned. A write coerces x/z to 0 in the one-shot flow; the staged `vrun` initialises to 0 without the on-write coercion. |
| `string` | SystemVerilog | Supported | See §5.4. |
| Class handle | SystemVerilog | Supported | A 32-bit object id; 0 is `null`. See §11. |
| `virtual` interface | SystemVerilog | Partial | See §16. |
| `chandle`, `shortreal` | SystemVerilog | Absent | `VITA-E2002` |

`signed` and `unsigned` qualifiers are supported (`reg signed [7:0] x;`, `'sd5`).
A packed range or dimension is accepted only on a net kind or on
`logic`/`reg`/`bit`; a fixed-width atom, `real`, `string` or `event` takes none
(IEEE 1800 §6.11), and writing one is a parse error naming that rule.

| Type | Width | Default sign |
|---|---|---|
| `integer` | 32 | signed |
| `byte` / `shortint` / `int` / `longint` | 8 / 16 / 32 / 64 | signed |
| `real` / `realtime` | 64 | signed (forced) |
| `time` | 64 | unsigned (forced) |
| `event` | 64 | unsigned (forced) |
| class handle | 32 | unsigned (forced) |
| `wire` / `reg` / `logic` / `bit`, no range | 1 | unsigned |
| `wire` / `reg` / `logic` / `bit`, `[m:l]` | `abs(m−l)+1` | unsigned unless `signed` is written |

Real values convert through `$rtoi`, `$itor`, `$realtobits` and `$bitstoreal`.
`$bits` of a real variable or of a `real`/`realtime` parameter is 64 (IEEE 1800
§6.12.1) in every context — a range bound, a `localparam` initializer, a
`generate if` condition, a constant function. A real parameter reached through a
wildcard `import p::*` answers 32; the `p::P` spelling answers 64.

Real operands are refused where the standard gives them no meaning, each with
its own `VITA-E3009` message: `%` (`modulo (%) not defined on real operand`),
bitwise, shift and reduction operators, bit and part selects, `==?`/`!=?`,
membership in a concatenation (`use $realtobits`), `%b`/`%h`/`%o` formatting, a
bit-vector system function argument, a width or range bound, a replication
count, and a parameter override.

### 5.2 Implicit nets and multiple drivers

| Position | Behaviour |
|---|---|
| Left side of a continuous assignment | Implicit 1-bit `wire` plus `W-PARSE-IMPLICIT-NET` / `VITA-W2003`. |
| Terminal list of a gate or module instance, either side | Same. |
| Any other position (ordinary right side, procedural lvalue) | Loud `E-ELAB-UNRESOLVED-NAME` / `VITA-E3010`. |
| Under `` `default_nettype none `` | Loud everywhere. |
| A wider assignment onto an implicit net | An extra `VITA-W3056` naming the discarded top bits. |

`-Werror=W-PARSE-IMPLICIT-NET` turns implicit nets into a project-wide error.

Multiple continuous drivers on one net are legal when every driver writes the
whole net without a delay; the value is resolved by 4-state wire resolution at
settle time. When any driver is delayed, multi-chunk, an array element or a
partial select, and the bit intervals overlap, the design is refused with
`E-ELAB-MULTIDRIVER` / `VITA-E3001`. Dynamic (non-constant offset) selects and
array-element writes are not counted, which is the conservative side of the cut.

### 5.3 User-defined types

| Construct | Status | Notes |
|---|---|---|
| `typedef enum { … } t;` | Supported | Labels lower to integer constants: first 0, then incrementing, with explicit `= expr` overrides. An explicit packed base (`enum logic [1:0] { … }`) or atom base (`enum byte { … }`) is supported and the base's declared signedness is preserved. A label that does not fit its base type is Loud (IEEE 1800 §6.19). |
| Enum methods `first` `last` `next` `prev` `name` `num` | Supported | They work on a variable of the enum type. Calling one on a package-scoped label (`pk::LA.name()`) is Loud, with a message showing the variable form. |
| `typedef <type> t;` alias, chained alias `typedef t2 t;` | Supported | Net kind, signedness and packed dimensions all carry. |
| `typedef struct packed { … } t;` | Supported | Members are packed MSB-first into one flat vector, and `s.field` lowers to a constant part-select. `struct packed signed` sets the whole-struct signedness. A member may be another packed struct or union typedef at any depth (`s.a.b.c`, `arr[i].a.b`), and a member width may name a constant, including a parameter overridden per instance. |
| `typedef struct { … } t;` unpacked record | Partial | Members keep their own types and become independent nets. A scalar record is supported, and so is a one-dimensional fixed array or unbounded queue of one (`t a [0:3];`, `t q [$];`), with `a[i].field` selecting a member: a packable record lowers to a packed element vector, and a record with a `string` or `real` member lowers to one array per member. A second unpacked dimension, a bounded queue (`[$:N]`), a declaration initializer on such an array, and a packed-struct member inside a record are Loud. |
| `typedef union packed { … } t;` | Supported | Overlay semantics: every member shares bit 0 and the width is the widest member. `union packed signed` is supported. |
| `typedef union { … }` unpacked | Loud | `VITA-E2002` `packed` after `union` (unpacked union unsupported in v1) |
| `tagged union` | Absent | `VITA-E2002` |
| Any other `typedef <x>` | Loud | `VITA-E2002`, naming the accepted forms. |

```systemverilog
typedef enum { RED, GREEN, BLUE } color_t;   // RED=0, GREEN=1, BLUE=2
typedef logic [7:0] byte_t;

typedef struct packed {
  logic [3:0] hi;
  logic [3:0] lo;
} nibble_pair_t;

nibble_pair_t p;
initial begin
  p.hi = 4'hA;
  p.lo = 4'h5;   // p is 8'hA5
end
```

A typedef carries its dimensions to every declaration that names it — a
variable, an ANSI port, a package or compilation-unit type, a block-local or
generate-scope declaration, a `'{…}` initializer, a chained alias, an interface
member, a subroutine formal and a type-parameter default. `$bits` of the bare
type name answers, in the package-scoped spelling as well. A dimension may name
a constant, including an overridable `parameter`, and is then computed per
instance; a package type's dimensions always read the package's own constants.

Loud on a typedef: use as a function return type, as an enum base, or as a
packed-struct member; `a_t'(e)`; `$bits` of a `[]`, `[$]` or `[string]` typedef;
and dimensions written on both the typedef and the declarator, because the
reference tools disagree about the resulting dimension order.

### 5.4 Strings

`string` variables are heap-backed and dynamic. Supported operations: `len`,
`getc`, `putc`, `substr`, `toupper`, `tolower`, `compare`, the `atoi` family
(`atoi`, `atohex`, `atooct`, `atobin`, `atoreal`), the `itoa` family (`itoa`,
`hextoa`, `octtoa`, `bintoa`), element indexing `s[i]` including a byte write
inside an `automatic` subroutine, comparisons, and `{a, b}` concatenation on
assignment. A chained call is supported on a string-returning method result
(`s.substr(2,4).atoi()`).

The `ato*` scan takes only leading digits and underscores per IEEE 1800 §6.16.9:
no whitespace skipping, no sign, and `_` is skipped rather than terminating, so
`" 3".atoi()` and `"-7".atoi()` are both `0` and `"1_0".atoi()` is `10`.

Loud on strings (`VITA-E3009` unless noted): a `string` port, a `string`
variable with packed or unpacked dimensions, a declaration initializer inside a
block (assign it in an `initial` block), a runtime index into a string array, a
non-blocking or delayed write to a string element, a string inside a
concatenation lvalue, a non-constant replication count, a real value as a
concatenation element, an unknown method or arity, and the `string'(24'h610062)`
cast (`VITA-E2002`).

### 5.5 Type and array query functions

| Function | Status | Notes |
|---|---|---|
| `$bits` | Supported | A variable, a type name, `pkg::T`, or a hierarchical net or parameter. |
| `$size` `$left` `$right` `$low` `$high` `$increment` | Supported | `$low` is `min(left,right)`, `$high` is `max`, `$size` is `abs(left−right)+1`. |
| `$dimensions` `$unpacked_dimensions` | Supported | No dimension argument. |
| `$typename` | Supported | |
| `$isunbounded` | Supported | |

## 6. Arrays and dynamic storage

### 6.1 Declared dimensions

| Spelling | Status | Notes |
|---|---|---|
| Packed vector `[msb:lsb]` | Supported | Descending or ascending; the width is `abs(msb−lsb)+1`. |
| Multi-dimensional packed `[3:0][7:0]` | Supported | One flat vector of the product of the widths; `m[i]` is a bit slice. |
| Negative bounds (`[3:-2]`, `[-3:0]`) | Partial | Stored normalized with the declared low bound in a side map. |
| Unpacked `[msb:lsb]` and `[N]` | Supported | Flattened, with per-dimension extents recorded when the addressing is not plain 0-based. |
| Multi-dimensional unpacked | Supported | |
| Dynamic array `[]` | Supported | `new[n]`, `new[n](src)`, `.size()`, `.delete()`, element read and write, whole-handle copy. |
| Queue `[$]`, bounded `[$:N]` | Supported | Push and pop at both ends, `.insert()`, `.delete()`, `q[$]`, bounded truncation, whole-handle copy, and the slice read `q[a:b]` (a partial out-of-range slice clamps; a reversed, fully out-of-range or x/z-bounded slice yields the empty queue). A non-constant or negative bound is Loud. |
| Associative `[integer]` `[time]` `[int]` `[longint]` `[shortint]` `[byte]` | Supported | Every integral spelling shares one signed 64-bit key domain; keys are not truncated to the declared width. |
| Associative `[string]` | Supported | Byte-string keys. |
| Associative `[*]` wildcard | Loud | `VITA-E2002`, naming the accepted key types. |
| Partial unpacked slice (fewer indices than dimensions) | Loud | `VITA-E3009` `partial unpacked-array slice (v1: index every dimension)` |

A dynamic dimension cannot be mixed with other unpacked dimensions; dynamic
storage must be a variable kind, cannot be a port, and cannot appear in an event
control. `real` and `event` elements in dynamic storage are Loud.

### 6.2 Runtime index behaviour

| Situation | Behaviour | Diagnostic |
|---|---|---|
| Known index past the end | Read yields all-`x`; the write is dropped, not clamped. | `E-RUN-RANGE` / `VITA-E4002`, rate-limited to 8 reports per kind |
| Unknown (x/z) index | Read yields all-`x`; the write is a no-op. | `W-RUN-RANGE-UNKNOWN` / `VITA-W4029`, separate budget |

A sub-dimension over-index of a multi-dimensional unpacked array is bounds
checked and reported the same way.

Arithmetic is exact for every operand inside the declarable width regime, in
both signednesses and at any width. The one bound is `WIDE_ARITH_CAP`, 1 048 576
bits, which equals `MAX_NET_WIDTH` (§19): an operand above it — only a
replication such as `{16{a}}` can inflate one that far — makes `*`, `/`, `%` and
`**` poison to `x` rather than stall the kernel, and the run warns once with
`W-RUN-WIDE-ARITH` / `VITA-W4025`. `+` and `-` stay exact at any width.

### 6.3 Array methods

| Method | Legal receivers | Position | Status |
|---|---|---|---|
| `.size()` | any dynamic handle | expression | Supported, no arguments |
| `.num()`, `.exists(k)` | associative | expression | Supported |
| `.delete()` / `.delete(k)` / `.delete(i)` | dynamic, queue, associative | statement | Supported |
| `.push_back(v)` / `.push_front(v)` | queue | statement | Supported |
| `.insert(i, v)` | queue | statement | Supported |
| `.pop_back()` / `.pop_front()` | queue | direct right side of a blocking assignment | Partial — elsewhere Loud (`VITA-E3009`) |
| `.first(k)` / `.next(k)` / `.last(k)` / `.prev(k)` | associative | direct right side of a blocking assignment | Partial — they write their key argument; elsewhere Loud |
| `.sort()` `.rsort()` `.reverse()` | dynamic array, queue, 1-D fixed unpacked | statement, no arguments | Supported |
| `.sum()` `.product()` `.and()` `.or()` `.xor()` | dynamic array, queue, associative, 1-D fixed unpacked | expression | Supported, bare or with `with (expr)` |
| `.min()` `.max()` `.unique()` `.unique_index()` | dynamic array, queue, associative | `dst = src.method()` where `dst` is a queue handle | Supported |
| `.find()` `.find_index()` `.find_first()` `.find_last()` `.find_first_index()` `.find_last_index()` | same | same | Supported; a `with (condition)` clause is required |

A locator used in expression position, or whose result is not a queue handle, is
Loud (`VITA-E3009`), and so is any unknown or kind-mismatched method. Reduction
and ordering methods take no arguments — the `with` clause is the only key. The
`with` iterator variable defaults to `item`, a named iterator
(`find(x) with (x > 2)`) is supported, and `item.index` is the declared low
index, so `int a[-1:1]` iterates -1, 0, 1.

On a fixed-size unpacked array the methods above work on 1-D receivers
(`int a[4]`, `int a[3:0]`, `int a[-1:1]`). A multi-dimensional receiver, a
non-simple receiver, `real`/`string`/class-handle elements, a packed vector
(`logic [3:0] v; v.sum()` is not an array method), a subroutine-local array, and
`.sort()` on a `wire` array (a procedural net write, `VITA-E3018`) are Loud.

Whole-array operations: an unpacked-array assignment requires identical element
types and the same number and size of dimensions (IEEE 1800 §7.6); a whole
unpacked array has no value in an expression, as a subroutine argument, or as a
port; and a whole-handle copy needs the same dynamic-storage kind and matching
element types on both sides.

### 6.4 `foreach`

`foreach` is a parse-time desugar over 1-D fixed unpacked arrays, dynamic
arrays, queues and associative arrays, plus the multi-dimensional form
`foreach (m[i,j])`. The index is renamed to a synthetic unique name so it cannot
clobber an outer variable, and `break` and `continue` work inside it. An empty
leading index slot (`foreach (a[,j])`) is Loud (`VITA-E2002`). Multi-dimensional
`foreach` is supported only on fixed-size unpacked arrays, and an index
dimension beyond the array's unpacked dimensions is Loud. A packed-vector
`foreach` is not supported.

### 6.5 Assignment patterns

```systemverilog
typedef struct packed { logic [3:0] mode; logic en; logic [7:0] len; } cfg_t;

cfg_t c;
int   a [0:3];
initial begin
  c = '{4'h3, 1'b1, 8'd7};                  // positional      (§10.9)
  c = '{len: 8'd7, mode: 4'h3, en: 1'b1};   // named           (§10.9.2)
  c = '{mode: 4'h5, default: 1'b0};         // named + default (§10.9.1)
  a = '{default: 5};                        // default on a whole array
end
```

| Spelling | Status | Notes |
|---|---|---|
| Positional `'{e0, …}` | Supported | Packed struct or union, fixed-size unpacked array (1-D and multi-dimensional, nested), dynamic array, queue. |
| Named `'{name: v, …}` | Supported | For a packed struct and a packable record, in a procedural assignment, a declaration initializer, and a `push_back`/`push_front`/`insert` actual. Field order comes from the declaration. Every member must be named exactly once. |
| `'{default: v}` | Supported | For a packed struct and for a fixed-size unpacked array of any bounds. The value is applied once per filled slot, so it takes each member's own width: `'{default: 1'b1}` on `cfg_t` is `13'h0301`, not all-ones. At most one `default:`. |
| Mixed `'{name: v, default: v}` | Supported | `default` covers whatever no name gave. |
| Integer key `'{0: a}`, type key `'{int: 0}`, replication `'{N{e}}` | Loud | `VITA-E2002` |
| Mixing positional and keyed | Loud | `VITA-E2002` — IEEE 1800 §10.9 does not allow it. |
| A call as the `default:` value | Loud | The value is duplicated into every slot it fills, so a call would run once per member. |
| A keyed pattern on a dynamic array, queue, packed array, subroutine argument or continuous assign, or nested inside a positional multi-dimensional pattern | Loud | `VITA-E3009` — the keys cannot be resolved against that target. |

Prefer the named spelling for a struct: a positional pattern is coupled to the
declaration order, so inserting a member shifts every later value.

The element count of an unpacked-array pattern must match the dimension exactly
(IEEE 1800 §10.9.1), and each element of a multi-dimensional pattern must itself
be a nested pattern.

### 6.6 Streaming operators

| Form | Status | Notes |
|---|---|---|
| `{<<N{…}}` | Supported | Cuts into N-bit blocks and reverses; the default slice size is 1. |
| `{>>N{…}}` | Supported | IEEE 1800 §11.4.14.3 pads a streaming right side on the RIGHT, so `{>>{32'hAABBCCDD}}` into 64 bits is `aabbccdd00000000` where a plain concatenation is `00000000aabbccdd`. |
| `{<<byte{…}}` (a type as the slice size) | Loud | `VITA-E2002`; write the bit count (`{<<8{…}}`). |
| `{<<8{a with [3:0]}}` | Loud | `VITA-E2002` |
| A stream with no assignment width, or an operand that calls a function | Loud | `VITA-E3009` — §11.4.14.3 needs a width to know where to pad, and an operand call would be recomputed once per output block. |

## 7. Expressions and operators

### 7.1 Precedence

The full table is implemented by a Pratt parser. Higher levels bind tighter.

| Level | Operators | Associativity |
|---|---|---|
| 1 | unary `+ - ! ~ & ~& \| ~\| ^ ~^ ^~` | right; tighter than `**`, so `-2**2` is `(-2)**2` |
| 2 | `**` | left — `2**2**3` is `(2**2)**3` |
| 3 | `* / %` | left |
| 4 | `+ -` | left |
| 5 | `<< >> <<< >>>` | left |
| 6 | `< <= > >=`, and the contextual `inside` / `dist` | left |
| 7 | `== != === !== ==? !=?` | left |
| 8 | `&` | left |
| 9 | `^ ~^ ^~` | left |
| 10 | `\|` | left |
| 11 | `&&` | left |
| 12 | `\|\|` | left |
| 13 | `?:` | right |
| 14 | `->` (constraint and property implication) | right |

A unary-only operator left in infix position (`a ~& b`) is a parse error, not a
silent truncation.

### 7.2 Operators

| Class | Operators | Status |
|---|---|---|
| Arithmetic | `+` `-` `*` `/` `%` `**` | Supported |
| Bitwise | `&` `\|` `^` `~` `~^` `^~` | Supported |
| Reduction (unary) | `&` `~&` `\|` `~\|` `^` `~^` | Supported |
| Logical | `&&` `\|\|` `!` | Supported |
| Relational | `<` `<=` `>` `>=` | Supported |
| Equality | `==` `!=` `===` `!==` | Supported |
| Wildcard equality | `==?` `!=?` | Supported — lowered as a constant-right-side mask and compare (IEEE 1800 §11.4.6) |
| Shift | `<<` `>>` `<<<` `>>>` | Supported |
| Conditional | `?:` | Supported |
| Concatenation | `{a, b, c}` | Supported |
| Replication | `{N{x}}` | Supported — `{0{x}}` has width 0 and is legal only as a direct operand of a concatenation; a negative or non-constant count is Loud |
| Bit select | `x[i]` | Supported |
| Part select | `x[m:l]` | Supported |
| Indexed part select | `x[b+:w]` / `x[b-:w]` | Supported — the width must be a constant ≥ 1 |
| `inside` | `a inside { … }` | Supported — desugared to an OR of equality and range tests, so it works in constraints and in ordinary conditions alike |
| `dist` | `v dist { … }` | Supported inside a constraint only; see §12 |
| Implication | `a -> b` | Supported — desugared to `!a \|\| b` |
| `min:typ:max` | `a:b:c` | Partial — accepted inside `#( … )` only, and the typ value is always taken |

### 7.3 Selects

A bit or part select attaches to any primary, so `((a^b)>>8)[7:0]`, `f(a)[7:0]`,
`{a,b}[7:0]` and `16'hABCD[7:0]` all parse and evaluate. When the select base did
not begin at a name, or when a non-final select in a chain is a range
(`a[7:0][3:0]`), a non-fatal `W-PARSE-SELECT-BASE` / `VITA-W2004` is emitted,
because other tools reject those forms.

Loud select forms, each with its own `VITA-E3009` message: a nested lvalue
select; a bit select of a bit select on a multi-dimensional array element; a
part-select range that exceeds the declared bounds; part-select bounds whose
direction disagrees with the net's; a real index, bound or size (IEEE 1800
§11.5.1); a non-constant or non-positive indexed part-select width; and more
indices than the packed array has dimensions.

### 7.4 Self-determined width and sign

Every operand has a self-determined width computed by one rule table. The
result is clamped at 2^24 bits.

| Node | Self width | Self sign |
|---|---|---|
| Numeric constant | `max(width, 1)` | as spelled; a string constant is never signed |
| Real constant | 64 | signed |
| Signal | the net's width | the net's signedness |
| Bit select | 1 | unsigned always (IEEE 1800 §5.4.1) |
| Part / indexed-part select | the constant width operand | unsigned always |
| Concatenation | sum of the part self-widths | unsigned always; parts are self-determined |
| Replication | `count × width(value)` | unsigned always |
| Unary `+ - ~` | operand width | operand sign |
| Unary `! & ~& \| ~\| ^ ~^` | 1 | unsigned; the operand is self-determined |
| Binary `+ - * / % & \| ^ ~^` | `max(L, R)` | signed only when BOTH are signed |
| `**` | the base width only | the base sign only; the exponent is self-determined |
| `< <= > >= == != === !== && \|\|` | 1 | unsigned |
| `<< >> <<< >>>` | the left operand's width | the left operand's sign; the amount is self-determined |
| `?:` | `max(then, else)` | signed only when BOTH branches are signed |
| `$time` / `$realtime` | 64 | unsigned |
| `$signed(x)` / `$unsigned(x)` | operand width | forced signed / forced unsigned |

Context-determined positions, where the assignment or operator width propagates
inward: both operands of `+ - * / % & | ^ ~^`, the left operand of a shift and of
`**`, both branches of `?:`, and the operand of unary `+ - ~`. Positions that
are self-determined and stop the context: concatenation and replication parts,
comparison and logical operands, reduction and `!` operands, a shift amount, a
power's exponent, and a `?:` condition.

### 7.5 Literal widths

| Form | Width | Signed | Extension |
|---|---|---|---|
| Plain decimal `42` | `max(32, nbits+1)` | signed | zero-extend |
| Unsized `'h…` / `'b…` / `'o…` | `max(32, digit span)` — the digit span, not the value's MSB, so `'h1FFFFFFFF` is 36 | as spelled | see below |
| Unsized `'d…` | `max(32, value bits)`, one more for `'sd` | as spelled | see below |
| Sized `W'…` | exactly `W` | as spelled | see below |
| Fill `'0 '1 'x 'z` | the context width when a context supplies one, else 32 | unsigned | replicate the fill bit |

The extension fill is the state of the supplied MSB: `x` extends with `x`, `z`
extends with `z`, and `0` or `1` zero-extends — a `1` MSB never replicates. Bits
above the declared width are truncated from the top.

Measured values: `$bits(42)` is 32, `$bits('hFF)` is 32, `$bits('h1FFFFFFFF)` is
36, `$bits(8'hAB)` is 8, `4'bx` is `xxxx`, `8'hzz` is `zzzzzzzz`, `8'b1` is
`00000001`, `8'bx1` is `xxxxxxx1`, `-4'sd5` is `-5`, `3'b111 + 1` is 8,
`4'sd5 / -4'sd2` is `-2`, `$bits(1.5)` is 64.

### 7.6 Casts

| Cast | Status | Notes |
|---|---|---|
| `int'(e)` `integer'(e)` `byte'(e)` `shortint'(e)` `longint'(e)` `bit'(e)` `logic'(e)` `reg'(e)` `time'(e)` `real'(e)` `realtime'(e)` | Supported | The complete primitive set. |
| `signed'(e)` / `unsigned'(e)` | Supported | The width is preserved; only the sign interpretation flips. Loud on a real operand. |
| `N'(e)` / `(W+1)'(e)` size cast | Supported | The result is N bits and inherits the operand's signedness. The width must be a positive constant expression. |
| `name'(e)` typedef cast | Supported | Numeric typedefs and packed struct or union types. |
| `Base'(derived)` class up-cast | Supported | Identity on the handle; only the static type narrows, and virtual dispatch reads the dynamic class. A down-cast and an unrelated cast are Loud. |
| Real to integer wider than 64 bits | Loud | `VITA-E3009` |
| `$cast(dest, source)` | Partial | Supported as the direct right side of a blocking assignment; elsewhere Loud. The destination must be a plain integral variable. |
| `void'(call);` | Supported | The discard-cast statement. |

## 8. Procedural blocks

| Block | Status | Notes |
|---|---|---|
| `initial` | Supported | Runs once at time 0. |
| `always` with `@( … )` | Supported | |
| `always` with no `@` but in-body `#` or `@` | Supported | The clock-generator shape; starts at time 0. |
| `always` with neither | Partial | Unschedulable; lowered as an inert process with a warning. |
| `always_ff` / `always_comb` / `always_latch` | Supported | |
| `final` | Supported | Runs once after the main loop ends, whatever the finish reason. A timing control inside it is Loud (IEEE 1800 §9.2.3 makes a `final` block zero-time). |

### 8.1 Sensitivity and event control

| Form | Status | Notes |
|---|---|---|
| `@*` / `@(*)` | Supported | A level list over the inferred read set. `always @*` has no implicit execution at time 0, unlike `always_comb` and `always_latch`. |
| `@(posedge clk or negedge rst_n)` | Supported | `or` and `,` both separate terms. |
| `@(a or b or sel)`, `@(a, b, c)` | Supported | |
| `@e`, `@clk`, `@u.s`, `@a[2]` without parentheses | Supported | A single level term. |
| `@(posedge clk iff en)` | Supported | Desugared to `@(posedge clk) if (en) S` for a single guarded term. An `iff` guard on a multi-term event control is Loud. |
| `@a+b`, `@posedge clk` without parentheses, `@()` | Loud | `VITA-E2002` |
| `edge` event control | Absent | `VITA-E2002` |
| In-body `@(*)` | Supported | Infers the read set of the statement it controls. An empty read set warns that it can never wake. |
| In-body multi-term edge wait | Loud | `VITA-E3009` — move it to a block header or split the wait. |
| Single-bit level (non-edge) event control | Loud | `VITA-E3009` — use `posedge`/`negedge`, or the whole signal. |
| Edge event control on a non-LSB bit select | Loud | `VITA-E3009` — a constant LSB bit select is the supported shape. |

## 9. Statements

| Statement | Status | Notes |
|---|---|---|
| `;` (null) | Supported | |
| `begin` / `end`, `begin : name` | Supported | Block-local declarations, including `automatic` ones, and sibling blocks reusing a name at any depth. A block that encloses another and redeclares the same name is Loud. |
| Blocking assign `=`, non-blocking assign `<=` | Supported | |
| `if` / `else` | Supported | |
| `case` / `casez` / `casex` … `endcase`, with `default` | Supported | |
| `case ( … ) inside`, `x inside { … }` | Supported | Desugared to an OR of equality and range tests. |
| `for` | Supported | Exactly one init and one step; a comma list is a parse error. |
| `while`, `repeat`, `forever` | Supported | |
| `do … while ( … );` | Supported | A parse-time desugar. |
| `foreach` | Supported | See §6.4. |
| `unique` / `priority` on `if` and `case` | Supported | A runtime no-match injects a report: `W-RUN-UNIQUE-VIOLATION` / `VITA-W4031`, text `value is unhandled for priority or unique case statement`. Multi-match checking is a documented cut — the lowered cascade is first-match-wins, so an overlap is unobservable. |
| `unique0` / `priority0` | Supported | They keep the qualifier's intent and suppress the no-match report. |
| A `unique`/`priority` qualifier on anything but `if`/`case` | Loud | `VITA-E2002` |
| `break;` / `continue;` | Supported | Contextual: recognised only in that exact shape, so a legacy net named `break` keeps working. |
| `return [expr];` | Supported | `return <expr>` in a void task, or `return` outside a subroutine body, is Loud. |
| Statement label `L: stmt` | Supported | IEEE 1800 §9.3.5: the label names a block around the statement, so `L: begin … end` is `begin : L … end`, `disable L` ends `L: for (…)`, and `%m` inside prints the label chain. A statement label plus a block label on the same `begin` is a parse error. |
| `disable name;` | Supported | Aborts the named lexically-enclosing block; `break` and `continue` lower onto this machinery. |
| `disable fork;` | Supported | Cancels the calling process's forked children. Inside a subroutine frame body it is Loud. |
| `disable` of a cross-process block, a task body, across a fork barrier, or by hierarchical path | Loud | `VITA-E3009` |
| `++i` `--i` `i++` `i--`, and `+= -= *= /= %= &= \|= ^= <<= >>= <<<= >>>=` | Supported | |
| `#delay` statement, `#delay stmt` | Supported | See §18. |
| `@(event) stmt` | Supported | |
| `wait (expr) [stmt]` | Supported | Level-sensitive wait. |
| `wait fork;` | Supported | |
| `-> ev;` | Supported | See §9.2. |
| `assert` / `assume` / `cover property` | Supported | See §13. |
| `void'(call);` | Supported | |
| `$systask(...)`, `task_name(args);` | Supported | See [System Tasks](005_system-tasks.md). |
| Any other keyword in statement position | Loud | `VITA-E2002` |

### 9.1 `casez` and `casex`

The IEEE split is implemented exactly: a `casez` bit is don't-care when either
side is `z` or `?` (an `x` never matches), and a `casex` bit is don't-care when
either side is `x` or `z`. Every remaining position compares 4-state exact. So
`casez(4'b10x1)` matches `4'b1??1`, `casex(4'b10x1)` matches `4'b1??1`, and
`casez(4'b1zz1)` matches `4'b1001`.

### 9.2 Named events

| Construct | Status | Notes |
|---|---|---|
| `event e;` | Supported | Desugars to a 64-bit unsigned counter starting at 0. |
| `-> e;` | Supported | Lowered as an increment, so two triggers in one slot go 0→2 and no toggle is lost. |
| `@(e)`, an event in a mixed sensitivity list | Supported | Every waiter sees the change. |
| `-> x` where `x` is not a named event | Loud | `VITA-E3009` |
| `event e[3];`, `event e = …;`, `event [7:0] e;` | Loud | `VITA-E3009` `a named event takes no range, initializer or array dimensions` |
| Assigning to, or reading, a named event | Loud | `VITA-E3009` — only `->e` and `@(e)` touch it. |

### 9.3 Assignment timing

| Construct | Status | Notes |
|---|---|---|
| `= #d rhs` / `<= #d rhs` | Supported | Intra-assignment delay with capture-now, write-later semantics. |
| `= @(ev) rhs` / `<= @(ev) rhs` | Supported | On `=` the process blocks; on `<=` it does not, and a detached helper performs the non-blocking write. |
| `= repeat(n) @(ev) rhs` | Supported | Unrolled; a count above 1024 is Loud, and a runtime (non-constant) count is Loud. |
| Intra-assignment delay on an unpacked-array or assignment-pattern assignment, on a string concatenation, on `new`, or on any dynamic-storage operation | Loud | `VITA-E3009` |

### 9.4 Continuous assignment, `force` and procedural `assign`

| Construct | Status | Notes |
|---|---|---|
| `assign lhs = rhs;` | Supported | |
| `assign #d lhs = rhs;` | Supported | Inertial: a pulse narrower than the delay is absorbed. |
| `assign #(rise, fall[, turnoff])` | Supported | Distinct rise, fall and turnoff delays; turnoff defaults to `min(rise, fall)`. |
| Net-declaration delay `wire #3 w = a;`, `wire #(2,3) w = a;` | Supported | On a net kind at module-item scope only. |
| Procedural `assign lv = e;` / `deassign lv;` | Supported | Lowered at a weaker rank than a plain procedural write. The target must be a whole variable. |
| `force lv = e;` | Partial | Sample-once: the right side is evaluated when the statement executes, not re-evaluated continuously. |
| `release lv;` | Supported | |
| Any of the four on a bit or part select | Loud | `VITA-E3009` — the target must be a whole net or variable. |
| `assign` / `deassign` targeting a net | Loud | `E-ELAB-LVALUE-KIND` / `VITA-E3018` |

Assignment-kind legality (`VITA-E3018`):

| lvalue kind | procedural `=` / `<=` | continuous `assign` |
|---|---|---|
| `wire` family | Loud — `procedural assignment to net` | allowed |
| `reg` / `integer` / `real` / `string` | allowed | Loud — `continuous assign drives variable` |
| `logic` | allowed | allowed (IEEE 1800 admits either) |

Port bindings and declaration initializers are synthetic continuous assignments
and are exempt.

## 10. Functions and tasks

| Feature | Status | Notes |
|---|---|---|
| `function [signed] [range] name (…); … endfunction` | Supported | ANSI and non-ANSI formal lists; range and `signed` qualifiers. |
| `task name (…); … endtask` | Supported | May consume time. |
| `function void f(...)` | Supported | Desugars to a task. |
| `function string f(...)` | Supported | The return net is a `string`. |
| 2-state return type | Supported | The return assignment coerces x/z to 0 (IEEE 1800 §6.11.3). |
| Body-local declarations, including `typedef enum` | Supported | |
| `automatic` on a function or task | Supported | Per-call frame storage; recursion works and the depth is capped loudly. |
| `automatic` / `static` per-declaration lifetime | Supported | On a frame subroutine's body declarations. |
| `automatic` on a procedural block-local | Partial | Accepted where the flattening is indistinguishable from per-entry storage: a declaration initializer re-runs on each block entry, and a read that may precede the first write is Loud. |
| Default argument values `f(int a, int b = 10)` | Supported | ANSI formals only. A default that references another formal is Loud. |
| Named arguments `.formal(v)` / `.formal()` | Supported | Reordered to positional before lowering. A named argument outside a user subroutine call is Loud. |
| `ref` / `const ref` formals | Partial | Mapped onto copy-in/copy-out (`inout`). The source spelling is kept for diagnostics. |
| Unpacked-array formals | Supported | The actual must be a bare whole-array name of the same per-dimension shape and direction. |
| Multi-dimensional packed formal | Supported | Element and part reads, element writes and the `$size` family inside the body. One that is also an unpacked array is Loud. |
| A `function` with an `output` or `inout` formal | Loud | `VITA-E3009` — illegal per the standard. |
| A function body that is not reducible to an expression when the frame lowering is unavailable | Loud | `VITA-E3009`, naming the reason. |
| Recursive subroutine when the frame-call lowering is unavailable | Loud | `VITA-E3009`, naming the cycle. |
| Argument-count mismatch | Loud | `VITA-E3009` `function/task f: N args for M formals` |
| `$finish` / `$stop` inside a subroutine body | Loud at runtime | `F-RUN-FATAL` / `VITA-F4004` — a body that stops half-way owes its calling expression a value, and the reference simulators disagree about which one. Move the call to the caller. |
| `let NAME [(formals)] = expr;` | Supported | Substituted with positional binding at each use. Arity mismatch and recursion are Loud (IEEE 1800 §11.13). |

## 11. Classes and object orientation

| Feature | Status | Notes |
|---|---|---|
| `class C; … endclass` | Supported | Top level, module body, package body. |
| Single inheritance `extends B` | Supported | One base class. A cyclic or unknown base is Loud. |
| Data members | Supported for integral kinds | IEEE 1800 §8.8 defaults are applied at `new`: 0 for 2-state, `x` for 4-state, `null` for handles. A folded constant declaration initializer is applied at `new`; a non-constant one is Loud. |
| `real`, `string`, array, and array-of-handle members | Loud | `VITA-E3009` |
| Methods `function` / `task` | Supported | A method with an `output`/`inout` port, or a `string` return type, is Loud. |
| `virtual` methods | Supported | Dispatched through a per-class vtable; non-virtual methods take no slot. |
| Constructor `new`, default arguments | Supported | An arity mismatch is Loud; a non-literal default argument value is Loud, because it would resolve in the caller's scope. |
| Implicit `super.new()` | Supported | Inserted into a derived constructor that does not call it. |
| `this` / `super` | Supported | |
| `local` / `protected` | Supported | Enforced; the violation message names the scope that may reach the member. |
| `static` / `const` / `pure` / `extern` members | Loud | `VITA-E2002` |
| `virtual class`, `pure virtual`, abstract classes | Absent | `VITA-E2002` |
| Parameterized class `class C #(int W = 8)` | Supported | Value parameters only, monomorphized at parse time: each distinct `C #(args)` becomes a concrete class. A `type` parameter on a class is Loud. |
| Class handles | Supported | A 32-bit object id where 0 is `null`; the object lives on the engine's class heap. `p == q` and `p == null` compare identity. |
| Handle mixed with an integral value, or used in any operator but `==`/`!=` | Loud | `VITA-E3009` (IEEE 1800 §8.4). |
| `new` not assigned to a handle | Loud | `VITA-E3009` |
| Class handle as a port, with dimensions, or with a declaration initializer | Loud | `VITA-E3009` |
| Unknown member | Loud | `VITA-E3009` `class C has no member f` |
| Object lifetime | No garbage collection | The class heap is never collected. The budget is 1 000 000 objects; exceeding it is `F-RUN-CLASS-LIMIT` / `VITA-F4024`. |
| `semaphore`, `mailbox`, the `process` class, `std::` | Absent | `VITA-E2002` |

## 12. Randomisation

| Feature | Status | Notes |
|---|---|---|
| `rand` member | Supported for integral members | A `rand` class-handle member is Loud. |
| `randc` member | Partial | Cycles over its full type range, outside the solver. A `randc` field wider than 16 bits, a constraint on one, and an inline `with` referencing one are Loud. |
| `constraint NAME { expr; … }` | Supported | A list of boolean expressions, each terminated by `;`, with an optional `soft` prefix. |
| `soft` constraints | Supported | Phase 1 solves hard and soft together; if that is infeasible, the soft ones are dropped and phase 2 retries hard-only. |
| `randomize()` | Supported | Returns 1 or 0. A null or x handle returns 0; a class with no rand fields returns 1; otherwise rejection sampling within a cap of 10 000 tries, with fields left unchanged on failure. |
| `randomize() with { … }` | Supported | As an expression and as a statement. A single-field range narrows that field's domain; everything else compiles to a predicate. The engine intersects inline domains with the class domains and ANDs the predicates. |
| Constraint operators | Supported | `+ - * / %`, `< <= > >=`, `== !=`, `&&`, `\|\|`, unary `!` and `-`, and parentheses. Anything else is Loud. |
| Constraint operands | Supported | The class's own and inherited rand fields, and constants. Any other name is Loud. |
| Width of a general (non-range) constraint | ≤ 63 bits of either sign, or exactly 64-bit signed | Predicates evaluate in signed 64-bit; wider or unsigned-64 is Loud. A pure single-field range constraint on such a field is supported. |
| Contradictory constraint | Loud at elaborate | `VITA-E3009` `contradictory constraint on rand field f (empty solution set)` |
| `x inside { … }` in a constraint | Supported | A top-level OR over a single field narrows that field's domain. |
| `v dist { … }` | Supported | `:=` is per value, `:/` spreads across the range. The weighted draw replaces the domain draw for that field. Non-constant bounds or weights, and a `dist` over a non-rand name, are Loud. |
| `if`/`else` constraints, `foreach` constraints, `solve … before` | Absent | `VITA-E2002` |
| `constraint_mode()`, `rand_mode()`, `pre_randomize()`, `post_randomize()`, `std::randomize()` | Absent | |
| `$urandom [(seed)]` | Supported | The seed argument is input-only: it re-seeds and is not written back. |
| `$urandom_range(max[, min])` | Supported | Inclusive; swapped bounds auto-correct; an x/z bound yields an x result. |
| `$random [(seed)]` | Supported | The IEEE 1364 Annex N generator. The seed must be a plain integral variable, and there is at most one. |
| `$dist_uniform` `$dist_normal` `$dist_exponential` `$dist_poisson` `$dist_chi_square` `$dist_t` `$dist_erlang` | Supported | Direct right side of a blocking assignment only. The seed must be a plain integral variable of at least 32 bits. |

Random draws are reproducible: `$urandom` and `$urandom_range` run a fixed
generator from a fixed initial state, `$random` and `$dist_*` use the Annex N
kernel in pure IEEE-754 arithmetic, and `randomize()` consumes its seed in a
fixed draw order. The same design produces byte-identical output on every
supported platform.

## 13. Assertions

### 13.1 Immediate and deferred

| Form | Status | Notes |
|---|---|---|
| `assert (c);` | Supported | With no `else`, the failure action reports `E-RUN-ASSERT-FAIL` / `VITA-E4001`. |
| `assert (c) pass; else fail;` | Supported | The `else`-only form gives a null pass action. |
| `assume (c);` | Supported | Parsed and checked exactly as `assert`. |
| `assert #0 (c)` | Supported | Matures in the Observed region, flushing when the statement is re-reached. |
| `assert final (c)` | Supported | Matures in the Reactive region. |
| `assert #N (c)`, N ≠ 0 | Loud | `VITA-E2002` — `#0` is the accepted deferred form. |
| `expect`, `restrict` | Absent | `VITA-E2002` |
| `$assertoff` / `$asserton` / `$assertkill` / `$assertcontrol` | Partial | A global fire gate. A levels or scope argument is Loud, because a scoped control would silently over-disable. |

### 13.2 Concurrent assertions

| Feature | Status | Notes |
|---|---|---|
| `assert property (@(clk) a \|-> b);` and `\|=>` | Supported | |
| A bare sequence property (no implication) | Loud | `VITA-E2002` |
| `disable iff (expr)` | Supported | Aborts the attempt with no verdict and clears in-flight pipeline state. |
| `default disable iff (expr);` | Supported | One per scope (IEEE 1800 §16.15); a second is Loud. |
| Action blocks `pass` / `else fail` | Supported | `pass` runs on non-vacuous success; `fail` replaces the default report. |
| `cover property (@(clk) seq);` | Supported | Lowered to a clocked match counter plus a `final` report of the hit count. A cover action block is Loud. |
| `sequence NAME; … endsequence` | Supported | Inlined at each use, with formals. An arity mismatch, recursion (IEEE 1800 §16.8) and an unknown name are Loud. |
| `property NAME; … endproperty` | Supported | Spliced at `assert property(NAME)`. Recursion (§16.12) is Loud, and a recursive reference is legal only as the consequent of `\|=>`. |
| Labelled assertion `name : assert property …` | Supported | |
| A clocking event | Required | An assertion with no clock in scope is Loud, with a message showing both spellings — an explicit `@(posedge clk)` or a `default clocking`. A multi-clock or OR-of-clocks property clock is Loud. |

### 13.3 Sequence operators

| Operator | Status | Notes |
|---|---|---|
| Boolean leaf | Supported | |
| `##n`, `##[m:n]`, leading `##n` | Supported | The ranged form is a sliding-OR window. |
| `[*n]`, `[*m:n]` consecutive repetition | Supported | |
| `[*m:$]` unbounded repetition | Partial | Requires a boolean operand. |
| `seq[+]` | Supported | Sugar for `[*1:$]`. |
| `[->n]` goto, `[=n]` nonconsecutive | Partial | Boolean operand, single count only; a range or a non-positive count is Loud. |
| `cond throughout seq` | Partial | Boolean left operand; over an unbounded, goto or nonconsecutive sequence it is Loud. |
| `seq1 within seq2` | Partial | Top-level antecedent only, over bounded boolean sequences. |
| `@(clk) seq` re-clocking | Supported | At a `##` boundary. |
| Match item `(b, x = e)` | Partial | One fixed-delay capture; a ranged delay in a sequence carrying a local variable is Loud, because two attempts would converge on one data stage. |
| Sequence and property local variables | Partial | Fixed-width integral types only, no initializer, one capture, read in a later antecedent term or in the consequent. Every other shape is Loud with a message naming it. |
| An empty-match repetition as the leading or standalone term | Loud | `VITA-E3009`; place it after a term. |
| `intersect`, `first_match` | Absent | `VITA-E2002` |

### 13.4 Property operators

Precedence, loosest to tightest: `implies` / `iff` < `until` / `s_until` < `or`
< `and` < unary prefix (`not`, `always`, `s_eventually`, `nexttime`) <
implication `|->` / `|=>` < primary.

| Operator | Status | Notes |
|---|---|---|
| `and` / `or` | Supported | Operands must be booleans of the same clock skew; a multi-term, re-clocked or named-sequence operand is Loud, and so is mixing a `\|=>` operand with a same-clock one. |
| `not p` | Supported | The operand must reduce to a same-clock verdict. |
| `p implies q` | Supported | Desugared to `(not p) or q`. |
| `p iff q` | Supported | Desugared to `(not p or q) and (not q or p)`. |
| `p until q` / `p s_until q` | Supported | `s_until` adds a liveness obligation checked at end of simulation. |
| `s_eventually p` | Supported | Unbounded only; reported at end of simulation. The bounded form `s_eventually [m:n] p` is Loud. |
| `eventually p` (weak, unbounded) | Loud | `VITA-E2002` — a weak unbounded `eventually` has no bounded-simulation verdict; use `s_eventually`. |
| `nexttime p` / `s_nexttime p` | Supported | Desugared to `1'b1 \|=> p`. The bounded form `nexttime [n] p` is Loud. |
| `always p` | Partial | Top level only; a nested `always` (`a \|-> always b`) is Loud. |
| `s_always` | Loud | `VITA-E2002` |

### 13.5 Multi-clock

| Form | Status | Notes |
|---|---|---|
| `@(c1) ante \|=> @(c2) cons` | Supported | Two-process handoff. A consequent clocking event requires `\|=>`, and it must be a single edge. |
| Cross-clock antecedent (`a ##1 @(c2) b`) | Partial | Every segment after a `##1` must be re-clocked, only `##1` connects a boundary, each segment clock must be a single edge, the consequent must be boolean, and a fourth clock boundary is Loud. |

### 13.6 Sampled-value functions

| Function | Status | Notes |
|---|---|---|
| `$sampled(e)` | Supported | Accepts any expression and recurses. |
| `$past(sig)` | Partial | One signal argument; no `[n]` delay and no clocking or gating arguments. |
| `$stable(sig)` | Supported | `prev === e` |
| `$changed(sig)` | Supported | `prev !== e` |
| `$rose(sig)` / `$fell(sig)` | Supported | On bit 0. |

Each takes exactly one simple, non-hierarchical signal; anything else is Loud.
There is no Preponed region in the region model — the sampled value is a
previous-value register.

## 14. Functional coverage

| Feature | Status | Notes |
|---|---|---|
| `covergroup NAME [@(event)]; … endgroup` | Supported | An optional clock auto-samples each instance. |
| `cg c = new;` | Supported | `new` must be spelled exactly. |
| `[LABEL:] coverpoint EXPR;` | Supported | |
| Automatic bins (empty bin body) | Supported | 64 bins when the sampled width is 6 or more, otherwise `1 << width`. |
| `bins` / `ignore_bins` / `illegal_bins` | Supported | |
| `bins b[]` unsized array bin | Supported | Beyond 64 bins it is Loud. |
| `bins b[N]` fixed-size array bin | Loud | `VITA-E3009` |
| `default` catch-all bin | Supported | |
| `$` range endpoint | Supported | Clamped to the coverpoint domain. |
| `[LABEL:] cross a, b;` | Supported | A cross product beyond 64 bins, and a cross of an `iff`-guarded coverpoint, are Loud. |
| Cross select body (`binsof` / `intersect`) | Loud | `VITA-E2002` |
| `iff (G)` guard on a coverpoint or bin | Supported | An `iff` on `ignore_bins`/`illegal_bins` is Loud. |
| `option.at_least = N` | Supported | At covergroup and coverpoint level. A value above 1 on an automatic-bin coverpoint is Loud; declare explicit bins. |
| `option.weight = N` | Supported | Per coverpoint; crosses weigh 1. |
| Transition bins (`=>`), `wildcard bins`, default sequence bins | Loud | `VITA-E2002` |
| Non-constant bin or option value | Loud | `VITA-E3009` |
| More than 64 explicit bins | Loud | `VITA-E3009` — the tracker is a 64-bit bitmap. |
| `c.sample()` | Supported | As a statement, no arguments. |
| `c.get_coverage()` | Supported | An integer percentage; the result must be used. |
| Any other covergroup method | Loud | `VITA-E3009` |

With `--obs-dir`, a design with at least one covergroup instance also writes
`coverage.json`, whose percentages mirror `get_coverage()` exactly. See
[CLI Reference](004_cli-reference.md).

## 15. Processes and fork/join

| Construct | Status | Notes |
|---|---|---|
| `fork … join` | Supported | |
| `fork … join_any` | Supported | |
| `fork … join_none` | Supported | |
| `wait fork;` | Supported | |
| `disable fork;` | Supported | At process level; inside a subroutine frame body it is Loud. |
| Fork arm with block-local declarations | Partial | The declarations are hoisted into the enclosing scope, with a warning naming the simplification. |
| `fork` label (`F: fork … join`) | Supported | The label is a `%m` scope. |
| Nested `fork` (a fork inside a fork child) | Loud | `VITA-E3009` |
| A fork child disabling a block outside its own body | Loud | `VITA-E3009` |
| A process that runs past the body-step budget without suspending | Loud at runtime | `F-RUN-BODY-STEP-LIMIT` / `VITA-F4027` |

## 16. Interfaces, packages, programs and clocking

| Construct | Status | Notes |
|---|---|---|
| `interface` / `endinterface` | Partial | An instance is flattened into plain nets plus symbol aliases. The accepted body items are: net and variable declarations, continuous assigns, procedural blocks (including block-locals inside them, which behave exactly as they do in a module), `modport`, `parameter`, port declarations, `genvar` and `import`. A nested instance, a `generate` block, a function or task, a `typedef` and a `defparam` inside an interface are Loud. |
| Interface header ports | Partial | ANSI, non-interface-typed only. Interface instance arrays are Loud. |
| `modport mp (input a, output b);` | Supported | Directions are enforced: writing through a modport `input` is `E-ELAB-PORT-MISMATCH` / `VITA-E3002` `cannot write X through a modport input (read-only)`. |
| `virtual [interface] IFACE vif;` | Partial | A static alias: the handle is bound once (`vif = bif;`), after which every `vif.member` aliases that instance's net. Never binding it, binding it to something that is not an interface instance, binding it to the wrong interface type, and dynamic or conditional re-binding are all Loud. |
| `package` / `endpackage` | Supported | A package body holds parameters, typedefs, functions, tasks and plain variables. A net declaration in a package is Loud (IEEE 1800 §26.2), and so are dynamic storage and the exotic variable kinds (event, string, real, class, virtual interface). |
| `import pkg::*;` / `import pkg::sym;` | Supported | At compilation-unit scope, module scope, and in an ANSI module or interface header, where the imported names are visible to the header's parameter defaults, ranges and port list. A body import applies after the header. An unknown package or symbol is Loud, an explicit import colliding with a local declaration is Loud (IEEE 1800 §26.3), and a name exported by two wildcards is unbound — Loud at the use site, silent when unused. |
| `export` | Absent | `VITA-E2002` |
| `pkg::name` scoped reference | Supported | For a parameter, an enum label and a plain variable. A part select of a package array element is Loud; read the whole element or import the name. |
| `program` / `endprogram` | Partial | See §2. |
| `clocking NAME @(event); … endclocking` | Partial | Default-skew input sampling: elaborate synthesizes preponed-sampled holding nets, `@(cb)` is a clocking event, and `cb.sig` reads the holding net. Anonymous `clocking @(clk);` and `default clocking` are supported. A clocking `inout` driver, a non-net input or output bind, an explicit skew other than `#1step`, a level or `@*` clocking event, and a cross-hierarchy `@(inst.cb)` are Loud. Writing a clocking input is Loud. |
| `timeunit` / `timeprecision` | Absent | `VITA-E2002`. `` `timescale `` is the supported channel; see §18. |
| `nettype`, `alias`, `checker`, `interface class`, `implements` | Absent | `VITA-E2002` |

## 17. Gate primitives and user-defined primitives

### 17.1 Gate primitives

The syntax is `gate_type [#delay] [name] ( terminals ) [, …] ;`, and each gate
lowers to a continuous assignment whose terminals are §3.5 implicit-net
positions on both sides.

| Primitive | Status | Lowering |
|---|---|---|
| `and`, `nand` | Supported | The first terminal is the output; the inputs fold with `&`, and `nand` inverts. Any number of inputs. |
| `or`, `nor` | Supported | Fold with `\|`; `nor` inverts. |
| `xor`, `xnor` | Supported | Fold with `^`; `xnor` inverts. |
| `buf` | Supported | The LAST terminal is the input and every preceding terminal is an output; `z` is coerced to `x` per IEEE 1364 §7.3. |
| `not` | Supported | The same shape, inverting. |
| `bufif0`, `bufif1`, `notif0`, `notif1` | Supported | Exactly `(out, data, control)`; the non-driving side drives `1'bz`. |
| Gate `#delay`, `#(rise, fall)`, `#(rise, fall, turnoff)` | Supported | The same delay model as `assign #d`. |
| `pullup`, `pulldown` | Absent | `VITA-E2002` |
| `cmos`, `rcmos`, `nmos`, `pmos`, `rnmos`, `rpmos` | Absent | `VITA-E2002` |
| `tran`, `tranif0`, `tranif1`, `rtran`, `rtranif0`, `rtranif1` | Absent | `VITA-E2002` |
| Drive strength on a gate | Loud | `VITA-E2002` |
| Wrong terminal count | Loud | `VITA-E2002`, naming the required shape. |

### 17.2 User-defined primitives

`primitive … endprimitive` is desugared into an ordinary module, so a UDP
instantiates like any other design unit.

| Feature | Status | Notes |
|---|---|---|
| Combinational UDP | Supported | One `always @(*)` with an if/else-if cascade. |
| Sequential UDP | Supported | `output reg`, two-colon rows, edge columns, `-` hold and `initial q = 1'bN;`. The IEEE 1364 §29 state table is evaluated literally: level rows first, then edge rows, no match yields `x`, and `-` holds. |
| Input level symbols | Supported | `0 1 x X ? b B` |
| Edge symbols | Supported | `r R` = `(01)`, `f F` = `(10)`, `p P` = `{(01),(0x),(x1)}`, `n N` = `{(10),(1x),(x0)}`, `*` = any change, and explicit `(vw)` pairs. |
| `(vw)` endpoints | Supported | `0 1 x X ?` only; `b`, `z` and `*` are Loud. |
| Next-state symbols | Supported | `0 1 x`, and `-` (hold) in a sequential table. |
| Matching semantics | Supported | 4-state exact, not `casez`: `0`/`1`/`x` match only that value, `?` matches anything including `z`, `b` matches 0 or 1, and the `x` symbol matches both `x` and `z` (§29.3.4). |
| Combinational conflict resolution | Supported | Order-independent priority 0 > 1 > x. |
| Optional `: name` end label | Ignored | |

UDP shape errors are parse errors (`VITA-E2002`) naming the rule: a vector
output, more than one output, a missing `output` declaration, an output that is
not the first port, a port and `input` declaration mismatch, fewer than two
ports, an initial value other than 0/1/x, more than two colons in a row, more
than one edge column in a row, a wrong column count, an illegal input symbol, an
empty table, a sequential table with a non-`reg` output, and a `reg`/`initial`
marker over a purely combinational table.

## 18. Timescale and timing

`` `timescale `` is a preprocessor directive written at file top level, outside
any design unit.

```systemverilog
`timescale 1ns / 1ps
```

| Feature | Status | Notes |
|---|---|---|
| `` `timescale unit/precision `` | Supported | The mantissa must be exactly `1`, `10` or `100`; the units are `s`, `ms`, `us`, `ns`, `ps`, `fs`. A precision coarser than the unit is Loud (`VITA-E1013`). A module is governed by the last region declared before it in file order. |
| No `` `timescale `` anywhere | Supported | The base is `1ns/1ns`, with `W-PP-TIMESCALE-DEFAULT` / `VITA-W1017`. |
| Some modules with a timescale and some without | Supported | `W-PP-TIMESCALE-MIXED` / `VITA-W1018`. IEEE 1800 §3.14.2.2 makes this an error and other tools refuse it. |
| Mixed timescales across modules | Supported | One global integer time axis, based on the finest precision in the design. |
| `#delay` scaling | Supported | Two stages: round at the declaring module's own precision, then scale to the global tick base. |
| `$time` | Supported | Scaled to the calling module's unit, per process. |
| `$realtime` | Supported | Real-valued time in the calling module's unit. |

Delay expressions follow one rule in every lane — a statement delay, a statement
prefix, an intra-assignment `=` or `<=`, a task body, `always`, a `fork` arm,
`repeat`, `for`, a structural `assign #d`, and a net-declaration delay:

- A time literal is rounded at the module's time precision, so under `10ns/1ns`
  `#(25ns)` delays 25 ns, and under `1ns/1ns` `#(2.5ns)` delays 3 ns.
- A literal whose unit is finer than the design precision is rounded where it is
  written rather than dropped: `#(2500ps)` under `1ns/1ns` is 3 ns, and
  `#(1250fs + 1250fs)` under `1ns/1ps` is 2 ps.
- A literal smaller than one tick rounds to no delay: `#(2.5ps)` under `1ns/1ns`
  does not delay.
- Division keeps the time domain: `#(3ns / 2)` under `1ns/1ns` is 2 ns, not the
  1 ns an integer division would give.
- A negated sized literal is a self-determined unsigned value, so `#(-4'd1)` is
  15 ticks.
- A delay that evaluates negative never fires in a procedural lane
  (`#(1ns - 5ns)`).
- A negative time literal (`#(-2500ps)`) is Loud, as is a time literal that is
  non-constant, real, negative or sub-precision in an ordinary expression
  position.

Waveform output is produced only when the design calls `$dumpvars`; there is no
always-on dumping. `$dumpfile` alone records a pending path and creates no file,
and a `-o` path alone produces no waveform either — both only choose the path
that the first `$dumpvars` opens. See [System Tasks](005_system-tasks.md) and
[CLI Reference](004_cli-reference.md).

## 19. Elaboration caps

A design that exceeds one of these is refused loudly, never truncated silently.

| Cap | Value | What it bounds |
|---|---|---|
| `MAX_NET_WIDTH` | 1 048 576 bits | One net's declared width, and a literal's resolved width |
| `MAX_TOTAL_NETS` | 131 072 | The whole net arena |
| `MAX_ARRAY_LEN` | 16 777 216 | Unpacked-array elements |
| `GENERATE_UNROLL_CAP` | 4096 | Generate-for iterations |
| `GENERATE_DEPTH_CAP` | 32 | Generate nesting |
| `INST_ARRAY_CAP` | 4096 | Instance-array elements |
| `REPEAT_UNROLL_CAP` | 1024 | Intra-assignment `repeat(n)` unroll |
| `MAX_EXPR_DEPTH` | 128 | Expression nesting |
| `MAX_STMT_DEPTH` | 256 | Statement nesting |
| `MAX_AST_NODES` | 2 097 152 | Total AST expression nodes |
| `MAX_DECIMAL_DIGITS` | 315 656 | Digits in one decimal literal |
| `SVA_SEQ_ALT_CAP` | 256 | Sequence alternatives, and property-level `and`/`or` nesting |
| Class-object budget | 1 000 000 | Live class objects; the heap is not collected |
| `MAX_ELAB_ERRORS` | 200 | Emitted elaborate diagnostics, after which one suppression notice is printed |

The parser stops recording after 50 diagnostics.

## 20. How this matrix is produced and verified

Every row is derived from the source at the current commit and pinned by a test
in the workspace suite. The gate is
`cargo nextest run --workspace --locked`: 7352 tests, all passing, 15 skipped.

Where an external oracle exists, a construct is settled by differential testing
against it:

- `crates/sim-engine/tests/differential.rs` runs 27 designs through vita and
  through Icarus Verilog live, compiling with `iverilog -g2012` and running
  `vvp`, and compares `$display` output verbatim. It runs when both tools are on
  `PATH` and skips otherwise, so it is a developer-machine gate.
- The CLI suite records oracle provenance in each test's header, naming the tool
  and version that produced the pinned value. Icarus Verilog is named in 534 of
  564 targets and Verilator in 182.
- `crates/corpus-runner` compiles each workload of the ten-design corpus with
  Icarus Verilog before the timed rounds, so a corpus row that vita and Icarus
  disagree about is a graded failure rather than a silent pass.

Where no oracle exists — SVA, classes, randomisation, parameterized classes and
virtual interfaces are unsupported by Icarus Verilog, and the two reference
tools contradict each other on a further set of cells — the expected value is
derived by reading IEEE 1364 and IEEE 1800 and written into the test as a
literal, with the test header recording why no tool arbitrates. Where a
soundness argument and a measurement disagree, the measurement wins. Where there
is neither an oracle nor a precondition, the construct stays Loud rather than
being implemented on a guess.

Divergences that survive this process, and the cells where the reference tools
split, are listed construct by construct in [Limitations](006_limitations.md).
When this chapter and the code disagree, the code is the ground truth and this
matrix is the defect.
