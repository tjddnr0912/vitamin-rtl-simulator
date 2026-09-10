# 01 · VHDL Lexical Elements

Per IEEE 1076-2008 §13.

---

## Identifiers

**Basic identifier**: letters, digits and underscores, starting with a letter. Consecutive
underscores (`__`) and a trailing underscore are illegal. **Case-insensitive** — `Signal`,
`SIGNAL` and `signal` are the same identifier.

**Extended identifier**: written between backslashes.

```vhdl
\MySignal\      -- valid
\end\           -- a reserved word may be used as an identifier
\My Signal\     -- a space is allowed
```

Extended identifiers are **case-sensitive**: `\MySignal\ /= \mysignal\`.
A backslash itself is escaped as `\\`.

---

## Comments

```vhdl
-- single-line comment (since VHDL-87)

/* block comment
   may span several lines
   added in VHDL-2008 */
```

Block comments `/* ... */` do not nest.

---

## Reserved Words

### VHDL-1993 (92 words)

| Group | Keywords |
|------|--------|
| Design units | `entity` `architecture` `package` `configuration` `library` `use` `context` |
| Types and objects | `type` `subtype` `constant` `signal` `variable` `file` `shared` `alias` `attribute` `generic` `port` |
| Sequential statements | `process` `begin` `end` `if` `then` `elsif` `else` `case` `when` `loop` `for` `while` `next` `exit` `return` `wait` `null` |
| Concurrent statements | `block` `generate` `component` `map` `open` `others` |
| Operators | `and` `or` `nand` `nor` `xor` `xnor` `not` `mod` `rem` `abs` `rol` `ror` `sla` `sll` `sra` `srl` |
| Delay and driving | `after` `transport` `inertial` `reject` `guarded` `disconnect` `unaffected` |
| Type keywords | `array` `record` `access` `range` `downto` `to` `of` `units` `group` `label` `literal` `in` `out` `inout` `buffer` `linkage` |
| Subprograms | `function` `procedure` `pure` `impure` `return` `body` |
| Reporting and assertion | `assert` `report` `severity` |
| Other | `all` `new` `on` `select` `with` `is` `register` `postponed` |
| Condition | `bus` |

The full list in alphabetical order:
`abs` `access` `after` `alias` `all` `and` `architecture` `array` `assert` `attribute`
`begin` `block` `body` `buffer` `bus`
`case` `component` `configuration` `constant`
`disconnect` `downto`
`else` `elsif` `end` `entity` `exit`
`file` `for` `function`
`generate` `generic` `group` `guarded`
`if` `impure` `in` `inertial` `inout` `is`
`label` `library` `linkage` `literal` `loop`
`map` `mod`
`nand` `new` `next` `nor` `not` `null`
`of` `on` `open` `or` `others` `out`
`package` `port` `postponed` `procedure` `process` `pure`
`range` `record` `register` `reject` `rem` `report` `return` `rol` `ror`
`select` `severity` `signal` `shared` `sla` `sll` `sra` `srl` `subtype`
`then` `to` `transport` `type`
`unaffected` `units` `until` `use`
`variable`
`wait` `when` `while` `with`
`xnor` `xor`

### Added in VHDL-2008 (PSL integration plus new features, ~15 words)

`assume` `assume_guarantee` `context` `cover` `default`
`force` `parameter` `property` `release` `restrict`
`restrict_guarantee` `sequence` `vmode` `vprop` `vunit`

> `context` introduces a context declaration, `force`/`release` appear in forced signal
> assignments, and the PSL keywords (`property`, `sequence`, `cover`, `assume` and the rest)
> are used when writing a verification unit.

---

## Literals

### Integer and real literals

```vhdl
42          -- integer
1_000_000   -- underscores allowed as separators (readability only, no meaning)
3.14        -- real (the decimal point is mandatory)
1.0e-5      -- real in exponent form
```

### Based literals

Form: `base#value#` or `base#value#Eexponent`

```vhdl
16#FF#        -- 255 (hexadecimal)
16#ff#        -- 255 (case-insensitive)
2#1010_1100#  -- 172 (binary)
8#377#        -- 255 (octal)
16#E#E1       -- 16#E# × 16¹ = 224
```

The base ranges from 2 to 16. The digit characters are case-insensitive too.

### Character and string literals

```vhdl
'A'           -- character literal
' '           -- a space
'"'           -- the double-quote character (enclosed in single quotes)

"Hello"       -- string
"say ""hi"""  -- an embedded double quote is escaped by doubling it
""            -- empty string
```

### Bit string literals

```vhdl
B"1010_1100"    -- binary, 8 bits
b"1010"         -- lower case is allowed too
O"377"          -- octal → 9 bits (3 bits per digit)
X"FF"           -- hexadecimal → 8 bits (4 bits per digit)
x"deadbeef"     -- 32 bits
D"255"          -- decimal → width chosen automatically (VHDL-2008+)
```

| Prefix | Base | Bits per digit | Introduced in |
|--------|------|--------------|----------|
| `B`/`b` | binary | 1 | 1993 |
| `O`/`o` | octal | 3 | 1993 |
| `X`/`x` | hexadecimal | 4 | 1993 |
| `D`/`d` | decimal | automatic | 2008 |

**VHDL-2008 width specifier**: a bit width may be written in front of the prefix.

```vhdl
8X"FF"    -- hexadecimal FF in exactly 8 bits
12X"FF"   -- zero-extended to 12 bits: "000011111111"
4X"FF"    -- truncated to 4 bits: "1111"
8D"255"   -- 255 as 8-bit binary: "11111111"
```

---

## Delimiters

```
:=   <=   =>   <>   --   /*   */
+  -  *  /  =  /=  <  <=  >  >=
&  |  .  ,  ;  :  (  )  '
```

Also `**` (exponentiation) and `??` (condition conversion, 2008+).

---

## Sources

- IEEE 1076-2008 §13 (Lexical elements) — UMBC portal.cs.umbc.edu ✓
- Doulos, VHDL-2008 Easier to Use
- SourceForge Scintilla vhdl_2008_keywords.txt ✓
