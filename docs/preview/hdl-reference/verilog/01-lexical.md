# 01 · Verilog Lexical Conventions

Per IEEE 1364-2005 §3. The lexical layer is the first processing stage: it turns
source text into a token stream.

---

## Token classes

Verilog source is made of these seven kinds of token.

| Token | Example |
|------|------|
| whitespace | space, tab, newline (CR/LF) |
| comment | `// ...` , `/* ... */` |
| operator | `+`, `~`, `===` |
| number | `8'hAB`, `3.14` |
| string | `"hello"` |
| identifier | `clk`, `\sys.clk ` |
| keyword | `module`, `wire`, `always` |

Whitespace decides token boundaries, but outside a string literal it carries no
meaning of its own.

---

## Comments

```verilog
// line comment — runs to the end of the line (\n)
/* block comment
   may span several lines
   no nesting: /* inside a /* */ is an error */
```

Block comments do not nest. A block delimiter written inside a line comment, as in
`// /* */`, has no effect — the comment still ends at the end of the line.

---

## Identifiers

### Simple identifier

```
first character: [A-Za-z_]
remainder:       [A-Za-z0-9_$]*
```

- Case sensitive (`clk` ≠ `CLK`)
- Cannot start with `$` (reserved for system tasks)
- Cannot start with a digit

### Escaped identifier

```
\<any characters><whitespace>
```

Starts with `\` and is terminated by a single whitespace character (space, tab or
newline). Every printable character in between becomes part of the name. Those
characters are literal — `\n` and friends are not escape sequences here.

```verilog
\x+y            // identifier: x+y
\sys.clk        // identifier: sys.clk
\a[0]           // identifier: a[0]  (not an array access)
wire \reset-n ; // a valid identifier (contains a hyphen)
```

The leading backslash and the terminating whitespace are not part of the name.
Written as `\reset-n `, the simulator sees a name spelled `reset-n`.

---

## Number literals

### Integer literal form

```
[size] ' [s|S] base_specifier digits
```

| Field | Meaning | Default |
|------|------|--------|
| `size` | bit width (a positive integer) | 32 (unsized) |
| `s` / `S` | marks the literal signed (1364-2001+) | unsigned |
| base_specifier | `d` / `b` / `o` / `h` (case insensitive) | `d` |
| digits | digits of that base + x/X + z/Z + `_` | — |

### Examples

```verilog
// sized literals
8'hAB          // 8-bit hex 0xAB (decimal 171)
4'b1010        // 4-bit binary 10
12'd255        // 12-bit decimal 255
6'o63          // 6-bit octal 51

// signed literals (1364-2001+)
4'sd5          // 4-bit signed +5
8'sb1111_0000  // 8-bit signed binary −16 (two's complement)
12'sh800       // 12-bit signed hex −2048
'sd9           // unsized signed decimal 9 (32 bits)

// x / z digits
4'bx           // 4 bits all unknown (= 4'bxxxx)
8'hzz          // 8 bits all high-Z (= 8'hzzzz)
8'b1010_xxxx   // upper 4 bits known, lower 4 bits unknown
4'bz0          // MSB=z, LSB=0

// underscores for readability
32'hDEAD_BEEF  // hex grouping
20'b0001_1010_0011_0100_0101

// unsized (no width given)
42             // 32-bit signed decimal 42
'b1101         // 32-bit binary
'hFF           // 32-bit hex
```

### Sizing rules

- **Truncation**: `size` smaller than the value needs → the MSBs are cut (possibly
  without a warning)
- **Zero extension**: an unsigned unsized literal assigned into a wider context →
  the upper bits fill with 0
- **Sign extension**: a signed literal assigned into a wider signed context → the
  MSB repeats

### Real number literals

Only two forms are legal:

```
fixed point:  <integer part>.<fraction>            e.g. 3.14, 0.5, 1.0
exponential:  <integer part>[.<fraction>]e[+|-]<exponent>   e.g. 1.5e3, 2.5E-4, 64e0
```

At least one digit is required on both sides of the decimal point (`1.` or `.5`
alone is a syntax error).

```verilog
real clk_freq = 1.0e9;  // 1 GHz
real tau      = 1.5e-9; // 1.5 ns
```

---

## Keywords

Verilog-2005 reserves **140** keywords — all lowercase. Written with a capital they
become ordinary identifiers (`Module` ≠ `module`), but by convention a user
identifier that merely re-cases a keyword is avoided.

```
always        and           assign        automatic
begin         buf           bufif0        bufif1
case          casex         casez         cell
cmos          config        deassign      default
defparam      design        disable       edge
else          end           endcase       endconfig
endfunction   endgenerate   endmodule     endprimitive
endspecify    endtable      endtask       event
for           force         forever       fork
function      generate      genvar        highz0
highz1        if            ifnone        incdir
include       initial       inout         input
instance      integer       join          large
liblist       library       localparam    macromodule
medium        module        nand          negedge
nmos          nor           noshowcancelled  not
notif0        notif1        or            output
parameter     pmos          posedge       primitive
pull0         pull1         pulldown      pullup
pulsestyle_onevent  pulsestyle_ondetect   rcmos
real          realtime      reg           release
repeat        rnmos         rpmos         rtran
rtranif0      rtranif1      scalared      showcancelled
signed        small         specify       specparam
strong0       strong1       supply0       supply1
table         task          time          tran
tranif0       tranif1       tri           tri0
tri1          triand        trior         trireg
unsigned      use           uwire         vectored
wait          wand          weak0         weak1
while         wire          wor           xnor
xor
```

`automatic`, `generate`, `genvar`, `localparam`, `signed`, `unsigned` and `uwire`
were added by Verilog-2001/2005 (they are not keywords in Verilog-1995).

---

## Keyword version directives

```verilog
`begin_keywords "1364-2005"
  // inside this region only 1364-2005 keywords are reserved
`end_keywords
```

Naming the version lets a SystemVerilog parser process Verilog source without user
identifiers colliding with SystemVerilog-only keywords.

---

## Sources

- IEEE 1364-2005 §3 (Lexical conventions)
- IEEE 1800-2017 §5 (Lexical conventions — Verilog-compat)
- portal.cs.umbc.edu/help/VHDL/verilog/reserved.html (keyword list)
- vlsiverify.com/verilog/lexical-conventions/
- chipverify.com/verilog/verilog-syntax
- projectf.io/posts/numbers-in-verilog/
