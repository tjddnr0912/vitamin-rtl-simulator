//! hdl-lexer — the tokenizer (LEX stage of preprocess -> LEX -> parse -> ...).
//!
//! Turns raw source text into a `Vec<Spanned>` token stream that `hdl-parser`
//! consumes, plus a `Vec<LexError>` of recoverable lex failures. The lexer
//! NEVER panics: every failure becomes a `TokenKind::Error` token in the stream
//! AND a `LexError` in the side list (span + reason). The driver maps each
//! `LexError` onto a `diag::Diagnostic` (`MsgCode::ParseUnexpectedToken`,
//! VITA-E2002, or a lex-specific unterminated path).
//!
//! ## Design decisions (logos 0.16)
//! - **Span lives in the lexer, not the token.** We iterate with `.spanned()`
//!   (yields `(Result<TokenKind, LexError>, Range<usize>)`) and pair kind+span
//!   into `Spanned`. This honors the doc-13 span-free sim-ir split: `TokenKind`
//!   stays payload-light (`Copy`), spans ride alongside in the stream/side-table.
//! - **Keyword strategy = one `Ident` regex + a `match` lookup** (NOT 124+
//!   `#[token]` variants, NOT a `phf`/`HashMap` dep). One place encodes
//!   case-sensitivity (lowercase-only table => `Module` stays `Ident`), the enum
//!   surface stays small, and maximal-munch is trivially correct (`module2`
//!   lexes as the whole identifier, then the lookup fails -> `Ident`).
//! - **Numbers/strings capture the raw slice only.** 4-state x/z, sizing,
//!   sign-extension are *semantic* and belong in elaborate. The token carries no
//!   value; downstream re-slices via the span. Interning is deferred.
//! - **Comments/whitespace are boundaries.** Line comments + whitespace use
//!   `#[logos(skip ...)]`. Block comments use a *callback token* so an
//!   unterminated `/*` is a real diagnostic (a skip-regex can't diagnose that).
//!
//! ## Ambiguities resolved here
//! - `<=` is ONE token (`LtEq`): less-equal vs nonblocking-assign is the
//!   parser's arity/context call. Same policy for reduction-vs-binary operators
//!   (`&` `|` `^` `~&` `~|` `~^` `^~`): one spelling, one token, parser resolves.
//! - `'` is never a standalone token: it only introduces a based literal
//!   (`'h`, `'b`, `4'sd5`, `'0`). SV cast `'(...)` is out of MVP scope.
//! - `3.14` is ONE `RealFixed` (maximal munch beats `3` `.` `14`); `1.5e3` is
//!   ONE `RealExp`.
//! - `1.` and `.5` alone are NOT reals (regex requires >=1 digit each side);
//!   they fall through to `IntDecimal`+`Dot` / `Dot`+`IntDecimal`, which the
//!   parser rejects. (A lone `.5` therefore lexes as `Dot` then `5`.)
//! - Escaped identifier `\...<ws>`: hand-scanned; backslash + terminating
//!   whitespace are not part of the name; the ws is left for the skip rule.
//! - Unterminated string / unterminated block comment => `LexError`, not panic.

use logos::{Lexer, Logos, Skip};
use std::ops::Range;

/// Half-open byte range `[start, end)` into the source string.
pub type Span = Range<usize>;

// ---------------------------------------------------------------------------
// Public token surface
// ---------------------------------------------------------------------------

/// A token kind paired with its source byte span. This is the unit the parser
/// consumes; `kind` is `Copy`, the span is the only location info (sim-ir is
/// span-free, so locations live here and in the AST side-table).
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned {
    pub kind: TokenKind,
    pub span: Span,
}

impl Spanned {
    #[inline]
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// A recoverable lex failure: a reason + the span it covers. Deliberately does
/// NOT depend on the `diag` crate — the driver maps `reason` to a `MsgCode`
/// (see [`LexErrorKind::msg_code_hint`]) and builds the `Diagnostic`.
#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: Span,
}

/// The structural reason a lex failed. Must satisfy logos's associated-error
/// contract: `Clone + PartialEq + Default`. logos emits `Default` on a byte that
/// matches no rule; callbacks return the specific variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LexErrorKind {
    /// A stray byte/char matching no token rule (logos default failure).
    #[default]
    UnexpectedChar,
    /// `"..."` with no closing quote before newline/EOF.
    UnterminatedString,
    /// `/*` with no closing `*/` before EOF.
    UnterminatedBlockComment,
    /// `\` not followed by any printable char (`\` then immediate whitespace/EOF).
    EmptyEscapedIdent,
    /// A bare `$` or backtick not followed by an identifier body (produced by the
    /// `lone_sigil` callback; the single sigil byte is the error span).
    LoneSigil,
}

impl LexErrorKind {
    /// Hint for the driver: which stable diag code best fits this reason.
    /// Returns the `(mnemonic, code_num)` pair so this crate need not link
    /// `diag`. The driver looks up the matching `diag::MsgCode`.
    ///
    /// All current variants map to `E-PARSE-UNEXPECTED-TOKEN` / `VITA-E2002`;
    /// the distinct variants exist so a future lex-specific code (e.g. an
    /// "unterminated" path) can split them out without changing the lexer API.
    pub const fn msg_code_hint(self) -> (&'static str, &'static str) {
        match self {
            LexErrorKind::UnexpectedChar
            | LexErrorKind::UnterminatedString
            | LexErrorKind::UnterminatedBlockComment
            | LexErrorKind::EmptyEscapedIdent
            | LexErrorKind::LoneSigil => ("E-PARSE-UNEXPECTED-TOKEN", "VITA-E2002"),
        }
    }
}

// logos requires the associated error type to be the enum-level `Error`.
// We use `LexErrorKind` as that type (it is `Clone + PartialEq + Default`), then
// re-attach the span at the iterator via `.spanned()` to build a `LexError`.
//
// (We deliberately do NOT bake the span into the error via
// `#[logos(error(Range<usize>, ...))]`: keeping `LexErrorKind` span-free lets it
// stay `Copy + Eq` and keeps the span as the single source of truth at the
// iterator. `From<E>` for each callback `E` is satisfied because every callback
// returns `LexErrorKind` directly.)

// ---------------------------------------------------------------------------
// The Logos token enum
// ---------------------------------------------------------------------------

/// The exhaustive MVP token kind. `Copy` because no variant carries an owned
/// payload — text-bearing kinds (numbers, strings, idents, system tasks,
/// directives, escaped idents) are recovered by re-slicing the source with the
/// token's span; keyword identity rides in `WordKind`/`Kw` (both `Copy`).
#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(error = LexErrorKind)]
// Boundaries with no value (token separators):
#[logos(skip r"[ \t\r\n\u{000C}]+")]
// whitespace incl. form-feed \f (\u{000C})
// Line comment to end-of-line. logos 0.16 lints `[^\n]*` as an "unbounded greedy
// dot" (it re-reads to EOF per token); for a line comment that scan-to-newline IS
// the intended behavior, so we opt in with `allow_greedy = true`. The newline is
// left unconsumed for the whitespace skip above.
#[logos(skip("//[^\n]*", allow_greedy = true))]
pub enum TokenKind {
    // ---- identifiers / keywords (one regex, keyword resolved in callback) ----
    /// Simple identifier OR a keyword (distinguished by the inner `WordKind`).
    /// `[A-Za-z_][A-Za-z0-9_$]*` — cannot start with `$` or a digit.
    /// `always_ff`/`always_comb`/`always_latch` match here too (underscores are
    /// legal mid-identifier); only the lookup table marks them as keywords.
    #[regex(r"[A-Za-z_][A-Za-z0-9_$]*", |lex| Kw::classify(lex.slice()))]
    Word(WordKind),

    /// Escaped identifier: `\` + printable run, terminated by ONE whitespace.
    /// The leading `\` and the terminating whitespace are not part of the name
    /// (downstream uses `&src[span][1..]`, trimmed). Hand-scanned in a callback
    /// because the "any printable until ws, ws excluded" rule is not a clean
    /// regex class.
    #[token("\\", escaped_ident)]
    EscapedIdent,

    /// System task/function: `$` + simple-identifier body. Distinct from `Word`
    /// because identifiers cannot *start* with `$`. The lexer does not whitelist
    /// the core set ($display/$time/$finish/...); the elaborator resolves which
    /// are supported. A bare `$` not followed by an ident-start matches the
    /// shorter `#[token("$", lone_sigil)]` rule below instead (=> `LoneSigil`).
    #[regex(r"\$[A-Za-z_][A-Za-z0-9_$]*")]
    SystemTask,

    /// Compiler directive: backtick + simple-identifier (`` `timescale ``,
    /// `` `define ``, `` `include ``). Emitted as one token; the (currently
    /// passthrough) preprocess stage consumes these later. A bare backtick with
    /// no ident body matches the shorter `lone_sigil` rule below (=> `LoneSigil`).
    #[regex(r"`[A-Za-z_][A-Za-z0-9_$]*")]
    Directive,

    // Bare backtick with no identifier body — shorter than the `Directive`
    // regex above, so maximal-munch picks that when an ident follows; only a
    // lone backtick reaches the callback, which returns `LoneSigil`. The
    // variant exists solely to host the rule (the callback always errors, so
    // it never appears as an `Ok` token). A lone `$` is a REAL token since
    // v5 ⑥ (queue last-index `q[$]`) — see `Dollar` above.
    #[token("`", lone_sigil)]
    LoneSigilTok,

    // ---- number literals (capture slice only; value parse deferred) ----
    // ORDER OF DECLARATION DOES NOT MATTER for correctness — logos picks the
    // LONGEST match; priority only breaks ties at equal length, and these five
    // never collide at equal length because their prefixes diverge. Comments
    // give the maximal-munch reasoning.
    /// Real, exponent form: `1.5e3`, `2.5E-4`, `64e0`, `1.0e9`, `1.5e-9`.
    /// The optional `(\.\d[\d_]*)?` covers both `64e0` and `1.0e9`. Beats
    /// `RealFixed`/`IntDecimal` by length whenever an exponent is present.
    #[regex(r"\d[\d_]*(\.\d[\d_]*)?[eE][+-]?\d[\d_]*")]
    RealExp,

    /// Real, fixed-point: `3.14`, `0.5`, `1.0`. BOTH sides require >=1 digit, so
    /// `1.` and `.5` do NOT match (they split into `IntDecimal`+`Dot` /
    /// `Dot`+`IntDecimal`, which the parser rejects — the spec calls them errors).
    #[regex(r"\d[\d_]*\.\d[\d_]*")]
    RealFixed,

    /// Sized based integer: `8'hAB`, `4'b1010`, `12'd255`, `6'o63`, `4'sd5`,
    /// `12'sh800`, `4'bx`, `8'hzz`, `32'hDEAD_BEEF`. Base letter is
    /// case-insensitive; digits include `x X z Z ? _`.
    #[regex(r"\d[\d_]*'[sS]?[dDbBoOhH][0-9a-fA-FxXzZ?_]+")]
    IntSized,

    /// Unsized based integer: `'hFF`, `'b1101`, `'sd9`, and unsized fills
    /// `'0 '1 'x 'z` (the `?_` class also admits `'?`).
    #[regex(r"'[sS]?([dDbBoOhH][0-9a-fA-FxXzZ?_]+|[01xXzZ])")]
    IntUnsizedBased,

    /// Plain unsized decimal: `42`, `0`, `1_000`. Underscores allowed (not first).
    #[regex(r"\d[\d_]*")]
    IntDecimal,

    /// String literal `"..."` with C-style escapes. Scanned in a callback so an
    /// unterminated string (no close before newline/EOF) is a real diagnostic.
    /// The span covers the opening and closing quotes.
    #[token("\"", lex_string)]
    Str,

    // ---- block comment (callback token; emits nothing on success) ----
    /// Non-nesting `/* ... */`. On success the callback returns `Skip` (no token
    /// is produced — it is a boundary like whitespace). On EOF-before-`*/` it
    /// returns an `UnterminatedBlockComment` error. This variant therefore never
    /// actually appears in the output stream; it exists only to host the rule.
    #[token("/*", block_comment)]
    BlockComment,

    // ---- operators & punctuators (fixed spellings; maximal munch) ----
    // Longest-match makes prefix families self-ordering: `===` beats `==` beats
    // `=`; `<<<` beats `<<` beats `<`; `~^`/`~&`/`~|` beat `~`; `->`/`-:` beat
    // `-`; `+:` beats `+`; `>=`/`>>`/`>>>` beat `>`. No manual priority needed.
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("[*")]
    LBracketStar, // SVA consecutive-repetition `[*n]` (Phase-3, slice S4)
    #[token("[->")]
    LBracketArrow, // SVA goto-repetition `[->n]` (Phase-3, slice S8)
    #[token("[=")]
    LBracketEq, // SVA nonconsecutive-repetition `[=n]` (Phase-3, slice S8)
    // SVA-REST `[+]` consecutive-repetition sugar = `[*1:$]` (one-or-more). An ATOMIC
    // 3-char token so it can NEVER mis-lex a `[+x]` indexed expression (logos matches
    // `[+]` only when a `]` immediately follows the `+`; `[+x]` stays `[` `+` `x` `]`).
    #[token("[+]")]
    BracketPlus,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(";")]
    Semi,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("@")]
    At,
    #[token("#")]
    Hash,
    #[token("##")]
    HashHash, // SVA sequence cycle-delay `##n` (Phase-3, slice S4)
    #[token("?")]
    Question,
    #[token(":")]
    Colon,
    /// SV package-scope operator `pkg::sym` (v7).
    #[token("::")]
    ColonColon,
    #[token("+:")]
    PlusColon, // ascending part-select [base+:width]
    #[token("-:")]
    MinusColon, // descending part-select [base-:width]
    #[token("->")]
    Arrow, // event trigger
    #[token("$")]
    Dollar, // bare `$` — queue last-index (`q[$]`) / `[$]` queue dim (v5 ⑥)

    // Standalone `'` — ONLY reachable for SV cast `type'(...)` / `N'(...)` and the
    // (still-unsupported) assignment-pattern `'{...}`. Logos longest-match keeps
    // `8'hFF`→IntSized and `'0/'1/'x/'z/'hFF`→IntUnsizedBased intact: those regexes
    // consume ≥2 chars while this token is 1 char, so the literal always wins. A
    // bare `'` followed by `(` or `{` (which neither literal regex matches) was a
    // lex error before; this token upgrades it to a clean Apostrophe. No valid
    // construct regresses (see `cast_apostrophe_is_additive` test below).
    #[token("'")]
    Apostrophe,

    #[token("=")]
    Eq, // blocking assign / param assign / net init
    #[token("<=")]
    LtEq, // less-equal AND nonblocking-assign — ONE token

    #[token("+")]
    Plus, // binary add / unary plus (arity by parser)
    #[token("-")]
    Minus, // binary sub / unary minus
    #[token("*")]
    Star,
    #[token("/")]
    Slash, // reached only after // and /* rules fail
    #[token("%")]
    Percent,
    #[token("**")]
    StarStar, // power

    #[token("&")]
    Amp, // bitwise AND / reduction-AND
    #[token("~&")]
    TildeAmp, // reduction NAND
    #[token("|")]
    Pipe, // bitwise OR / reduction-OR
    #[token("~|")]
    TildePipe, // reduction NOR
    #[token("^")]
    Caret, // bitwise XOR / reduction-XOR
    #[token("~^")]
    TildeCaret, // XNOR (form 1)
    #[token("^~")]
    CaretTilde, // XNOR (form 2) — distinct spelling kept for span fidelity
    #[token("~")]
    Tilde, // bitwise NOT

    #[token("!")]
    Bang, // logical NOT
    #[token("&&")]
    AmpAmp, // logical AND
    #[token("||")]
    PipePipe, // logical OR
    #[token("|->")]
    PipeArrow, // SVA overlapping implication (v8, Phase-3)
    #[token("|=>")]
    PipeEqArrow, // SVA non-overlapping implication (v8, Phase-3)

    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token(">=")]
    GtEq,
    #[token("==")]
    EqEq, // logical equality
    #[token("!=")]
    BangEq, // logical inequality
    #[token("===")]
    EqEqEq, // case equality
    #[token("!==")]
    BangEqEq, // case inequality
    // §11.4.6 wildcard equality. logos longest-match keeps every prefix-sharing
    // stream intact: `===?` still lexes `===` `?` (no 4-char token exists),
    // `a==b?c:d` lexes `==` (next char is `b`), and a SPACED `== ?` stays two
    // tokens — only the previously-unlexable contiguous `==?` is new.
    #[token("==?")]
    EqEqQ, // wildcard equality
    #[token("!=?")]
    BangEqQ, // wildcard inequality

    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,
    #[token("<<<")]
    ShlA, // arithmetic left shift
    #[token(">>>")]
    ShrA, // arithmetic right shift

    // ---- increment / decrement + compound assignment (SV §11.4.1/§11.4.2) ----
    // logos longest-match keeps these distinct from `+`/`=`/`<<`/`|=>` etc.
    #[token("++")]
    PlusPlus,
    #[token("--")]
    MinusMinus,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,
    #[token("%=")]
    PercentEq,
    #[token("&=")]
    AmpEq,
    #[token("|=")]
    PipeEq,
    #[token("^=")]
    CaretEq,
    #[token("<<=")]
    ShlEq,
    #[token(">>=")]
    ShrEq,
    #[token("<<<=")]
    ShlAEq,
    #[token(">>>=")]
    ShrAEq,

    // ---- error sentinel (never produced by logos directly; built at iterator) ----
    /// Placed into the stream wherever a `LexError` occurred so the parser sees a
    /// concrete token at that position and can recover. Carries the reason; the
    /// span is on the enclosing `Spanned`.
    Error(LexErrorKind),
}

/// A matched `Word` is either a reserved keyword (`Keyword(Kw)`) or a plain
/// identifier. `Copy` so `TokenKind` stays `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordKind {
    Keyword(Kw),
    Ident,
}

// ---------------------------------------------------------------------------
// Callbacks
// ---------------------------------------------------------------------------

/// Scan an escaped identifier body after the leading `\`. Consumes printable
/// non-whitespace chars; stops at (and leaves) the first whitespace so the
/// `skip` rule eats it as a boundary. Empty body (`\` then ws/EOF) is an error.
fn escaped_ident(lex: &mut Lexer<TokenKind>) -> Result<(), LexErrorKind> {
    let rem = lex.remainder(); // source after the backslash
    let end = rem.find(|c: char| c.is_whitespace()).unwrap_or(rem.len());
    if end == 0 {
        return Err(LexErrorKind::EmptyEscapedIdent);
    }
    lex.bump(end); // consume the name bytes; terminating ws stays for the skip
    Ok(())
}

/// Scan a string body after the opening `"`. Handles escape pairs, stops at the
/// closing `"`. A newline or EOF before the close is an unterminated string.
///
/// SOUNDNESS (no-panic guarantee): logos `bump` asserts the new offset is
/// in-bounds AND on a UTF-8 char boundary. The escape branch therefore must
/// never let `i` run past `bytes.len()` (a trailing `\` at EOF) and the final
/// `bump` is clamped to a char boundary as defense-in-depth.
fn lex_string(lex: &mut Lexer<TokenKind>) -> Result<(), LexErrorKind> {
    let rem = lex.remainder();
    let bytes = rem.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                // Escape pair (\" \\ \n \t ...). A `\` immediately before a
                // newline, or a `\` that is the LAST byte (EOF), is part of a
                // broken string (Verilog strings are single-line; `\<newline>`
                // is not a defined escape). Consume the backslash itself (it
                // belongs to the unterminated run) but do NOT skip past it into
                // the newline / past EOF — that would swallow the newline or
                // overrun `i` past `bytes.len()` and trip logos `bump`'s assert.
                if i + 1 >= bytes.len() {
                    i += 1; // include the trailing `\`; loop then exits at EOF
                } else if bytes[i + 1] == b'\n' {
                    i += 1; // include the `\`; next iter breaks on the newline
                } else {
                    i += 2; // skip backslash + the escaped byte
                }
            }
            b'"' => {
                lex.bump(i + 1); // include the closing quote in the token
                return Ok(());
            }
            b'\n' => break, // newline before close => unterminated
            _ => i += 1,
        }
    }
    // No closing quote was found. `i` is the index of the offending newline
    // (break) or == bytes.len() (loop fell off the end), so it is already
    // <= bytes.len(). Clamp to a char boundary defensively before bumping so a
    // multibyte char in the unterminated run can never trip the boundary assert.
    let mut end = i.min(bytes.len());
    while end > 0 && !rem.is_char_boundary(end) {
        end -= 1;
    }
    lex.bump(end);
    Err(LexErrorKind::UnterminatedString)
}

/// Recovery callback for a bare sigil that is NOT followed by an identifier
/// body: `$` (not `$ident`, which is `SystemTask`) or a backtick (not
/// `` `ident ``, which is `Directive`). Emits the `LoneSigil` reason so the
/// error taxonomy is honest (these would otherwise surface as `UnexpectedChar`).
/// The single sigil byte is the token; the parser recovers from the next byte.
fn lone_sigil(_lex: &mut Lexer<TokenKind>) -> Result<(), LexErrorKind> {
    Err(LexErrorKind::LoneSigil)
}

/// Scan a non-nesting block comment after the opening `/*`. First `*/` closes.
/// On success returns `Skip` (produces no token — it's a boundary). EOF before
/// `*/` is an unterminated block comment.
fn block_comment(lex: &mut Lexer<TokenKind>) -> Result<Skip, LexErrorKind> {
    let rem = lex.remainder();
    match rem.find("*/") {
        Some(end) => {
            lex.bump(end + 2); // consume through the closing */
            Ok(Skip)
        }
        None => {
            lex.bump(rem.len()); // consume to EOF so the span covers it
            Err(LexErrorKind::UnterminatedBlockComment)
        }
    }
}

// ---------------------------------------------------------------------------
// Keyword table — case-SENSITIVE, lowercase-only (so `Module` stays `Ident`).
// 124 Verilog-2005 reserved words + 4 SystemVerilog-subset additions = 128.
// (The lexical doc prose says "140"; that is the IEEE-1800 count and a known
//  mislabel — the printed reserved block holds exactly 124 words.)
//
// A `match` on `&str` is compiled to an efficient dispatch by rustc; no `phf`,
// no `HashMap` — keeping the dep set minimal and behavior deterministic.
// ---------------------------------------------------------------------------

/// Reserved keyword identity. The lexer emits `Word(Keyword(Kw::X))`; the
/// IN-MVP vs reserved-but-unused split is a *parser* concern, not a lex filter —
/// every reserved word tokenizes so the parser can give precise errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[rustfmt::skip]
pub enum Kw {
    // --- 124 Verilog-2005 reserved words ---
    Always, And, Assign, Automatic, Begin, Buf, Bufif0, Bufif1,
    Case, Casex, Casez, Cell, Cmos, Config, Deassign, Default,
    Defparam, Design, Disable, Edge, Else, End, Endcase, Endconfig,
    Endfunction, Endgenerate, Endmodule, Endprimitive, Endspecify, Endtable,
    Endtask, Event, For, Force, Forever, Fork, Function, Generate,
    Genvar, Highz0, Highz1, If, Ifnone, Incdir, Include, Initial,
    Inout, Input, Instance, Integer, Join, Large, Liblist, Library,
    Localparam, Macromodule, Medium, Module, Nand, Negedge, Nmos, Nor,
    Noshowcancelled, Not, Notif0, Notif1, Or, Output, Parameter, Pmos,
    Posedge, Primitive, Pull0, Pull1, Pulldown, Pullup,
    PulsestyleOnevent, PulsestyleOndetect, Rcmos, Real, Realtime, Reg,
    Release, Repeat, Rnmos, Rpmos, Rtran, Rtranif0, Rtranif1, Scalared,
    Showcancelled, Signed, Small, Specify, Specparam, Strong0, Strong1,
    Supply0, Supply1, Table, Task, Time, Tran, Tranif0, Tranif1,
    Tri, Tri0, Tri1, Triand, Trior, Trireg, Unsigned, Use,
    Uwire, Vectored, Wait, Wand, Weak0, Weak1, While, Wire,
    Wor, Xnor, Xor,
    // --- 4 SystemVerilog-subset additions (doc 01 Phase-1) ---
    Logic, AlwaysFf, AlwaysComb, AlwaysLatch,
    // --- SV user-defined types (Phase-2) ---
    Typedef, Enum, Struct, Union, Packed,
    // --- SV immediate assertion (Phase-2 gateway item E) ---
    Assert,
    // --- SV interface (v5 ⑥, Phase-2 gateway item D) ---
    Interface, Endinterface, Modport,
    // --- SV program block (ⓑ-breadth, §24 — parsed into the module AST) ---
    Program, Endprogram,
    // --- SV foreach (v5 ⑥ follow-on — parse-desugar, no AST/IR change) ---
    Foreach,
    // --- SV package + string type (v7 P2-C/P2-D) ---
    Package, Endpackage, Import, String,
    // --- SV procedural advanced (P2-E) ---
    Do, Unique, Priority, Unique0, Priority0, Final,
    // --- SVA concurrent assertion subset (v8, Phase-3) ---
    Property,
    // --- SV functional coverage subset (N5, Phase-3) ---
    Covergroup, Endgroup, Coverpoint,
    // --- SV 2-state integer types (SVPART) ---
    Bit, Byte, Shortint, Int, Longint,
    // --- SV class/OOP subset (N7) ---
    Class, Endclass, Extends, Virtual, Null,
    // --- SVA-REST ---
    Assume,
    // --- SV void return type (typed-param/void close) ---
    Void,
    // --- SV clocking block (N4, §14) ---
    Clocking, Endclocking,
}

impl Kw {
    /// Classify a matched simple-identifier slice. Lowercase-only table, so
    /// case-sensitivity is enforced here: anything not exactly matching is an
    /// `Ident` (e.g. `Module`, `MODULE`, `Wire1`).
    #[rustfmt::skip]
    fn classify(s: &str) -> WordKind {
        use Kw::*;
        let kw = match s {
            "always" => Always, "and" => And, "assign" => Assign,
            "automatic" => Automatic, "begin" => Begin, "buf" => Buf,
            "bufif0" => Bufif0, "bufif1" => Bufif1, "case" => Case,
            "casex" => Casex, "casez" => Casez, "cell" => Cell,
            "cmos" => Cmos, "config" => Config, "deassign" => Deassign,
            "default" => Default, "defparam" => Defparam, "design" => Design,
            "disable" => Disable, "edge" => Edge, "else" => Else,
            "end" => End, "endcase" => Endcase, "endconfig" => Endconfig,
            "endfunction" => Endfunction, "endgenerate" => Endgenerate,
            "endmodule" => Endmodule, "endprimitive" => Endprimitive,
            "endspecify" => Endspecify, "endtable" => Endtable,
            "endtask" => Endtask, "event" => Event, "for" => For,
            "force" => Force, "forever" => Forever, "fork" => Fork,
            "function" => Function, "generate" => Generate, "genvar" => Genvar,
            "highz0" => Highz0, "highz1" => Highz1, "if" => If,
            "ifnone" => Ifnone, "incdir" => Incdir, "include" => Include,
            "initial" => Initial, "inout" => Inout, "input" => Input,
            "instance" => Instance, "integer" => Integer, "join" => Join,
            "large" => Large, "liblist" => Liblist, "library" => Library,
            "localparam" => Localparam, "macromodule" => Macromodule,
            "medium" => Medium, "module" => Module, "nand" => Nand,
            "negedge" => Negedge, "nmos" => Nmos, "nor" => Nor,
            "noshowcancelled" => Noshowcancelled, "not" => Not,
            "notif0" => Notif0, "notif1" => Notif1, "or" => Or,
            "output" => Output, "parameter" => Parameter, "pmos" => Pmos,
            "posedge" => Posedge, "primitive" => Primitive, "pull0" => Pull0,
            "pull1" => Pull1, "pulldown" => Pulldown, "pullup" => Pullup,
            "pulsestyle_onevent" => PulsestyleOnevent,
            "pulsestyle_ondetect" => PulsestyleOndetect, "rcmos" => Rcmos,
            "real" => Real, "realtime" => Realtime, "reg" => Reg,
            "release" => Release, "repeat" => Repeat, "rnmos" => Rnmos,
            "rpmos" => Rpmos, "rtran" => Rtran, "rtranif0" => Rtranif0,
            "rtranif1" => Rtranif1, "scalared" => Scalared,
            "showcancelled" => Showcancelled, "signed" => Signed,
            "small" => Small, "specify" => Specify, "specparam" => Specparam,
            "strong0" => Strong0, "strong1" => Strong1, "supply0" => Supply0,
            "supply1" => Supply1, "table" => Table, "task" => Task,
            "time" => Time, "tran" => Tran, "tranif0" => Tranif0,
            "tranif1" => Tranif1, "tri" => Tri, "tri0" => Tri0,
            "tri1" => Tri1, "triand" => Triand, "trior" => Trior,
            "trireg" => Trireg, "unsigned" => Unsigned, "use" => Use,
            "uwire" => Uwire, "vectored" => Vectored, "wait" => Wait,
            "wand" => Wand, "weak0" => Weak0, "weak1" => Weak1,
            "while" => While, "wire" => Wire, "wor" => Wor,
            "xnor" => Xnor, "xor" => Xor,
            // SystemVerilog-subset:
            "logic" => Logic, "always_ff" => AlwaysFf,
            "always_comb" => AlwaysComb, "always_latch" => AlwaysLatch,
            "typedef" => Typedef, "enum" => Enum, "struct" => Struct,
            "union" => Union, "packed" => Packed,
            "assert" => Assert,
            "interface" => Interface, "endinterface" => Endinterface,
            "program" => Program, "endprogram" => Endprogram,
            "modport" => Modport, "foreach" => Foreach,
            "package" => Package, "endpackage" => Endpackage,
            "import" => Import, "string" => String,
            "do" => Do, "unique" => Unique, "priority" => Priority,
            "unique0" => Unique0, "priority0" => Priority0,
            "final" => Final,
            "property" => Property,
            "covergroup" => Covergroup, "endgroup" => Endgroup,
            "coverpoint" => Coverpoint,
            "bit" => Bit, "byte" => Byte, "shortint" => Shortint,
            "int" => Int, "longint" => Longint,
            // --- SV class/OOP subset (N7) ---
            "class" => Class, "endclass" => Endclass, "extends" => Extends,
            "virtual" => Virtual, "null" => Null,
            // --- SVA-REST: assumption property (sim-checked like assert) ---
            "assume" => Assume,
            // --- SV void return type ---
            "void" => Void,
            // --- SV clocking block (N4) ---
            "clocking" => Clocking, "endclocking" => Endclocking,
            _ => return WordKind::Ident,
        };
        WordKind::Keyword(kw)
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Tokenize `src` into a token stream + a list of recoverable lex errors.
///
/// - Whitespace, line comments, and (successful) block comments are dropped as
///   boundaries.
/// - Every lex failure produces BOTH a `Spanned { kind: Error(reason), span }`
///   in the returned vec (so the parser sees a concrete token and can recover)
///   AND a `LexError { kind, span }` in the error list (so the driver can emit a
///   diagnostic). The function never panics.
pub fn lex(src: &str) -> (Vec<Spanned>, Vec<LexError>) {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();
    for (result, span) in TokenKind::lexer(src).spanned() {
        match result {
            Ok(kind) => tokens.push(Spanned::new(kind, span)),
            Err(kind) => {
                errors.push(LexError {
                    kind,
                    span: span.clone(),
                });
                tokens.push(Spanned::new(TokenKind::Error(kind), span));
            }
        }
    }
    (tokens, errors)
}

/// Streaming variant: yields `Spanned` only (errors surface as `Error` tokens).
/// Useful when the parser drives lazily and inspects `TokenKind::Error` itself.
pub fn lex_iter(src: &str) -> impl Iterator<Item = Spanned> + '_ {
    TokenKind::lexer(src)
        .spanned()
        .map(|(result, span)| match result {
            Ok(kind) => Spanned::new(kind, span),
            Err(kind) => Spanned::new(TokenKind::Error(kind), span),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: kinds only (drop spans) for compact assertions.
    fn kinds(src: &str) -> Vec<TokenKind> {
        lex(src).0.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn ex1_continuous_assign() {
        use TokenKind::*;
        // `assign y = a & 8'hFF;`
        let k = kinds("assign y = a & 8'hFF;");
        assert_eq!(
            k,
            vec![
                Word(WordKind::Keyword(Kw::Assign)),
                Word(WordKind::Ident), // y
                Eq,
                Word(WordKind::Ident), // a
                Amp,
                IntSized, // 8'hFF
                Semi,
            ]
        );
        // span fidelity: the literal `8'hFF` is bytes 15..20
        let (toks, errs) = lex("assign y = a & 8'hFF;");
        assert!(errs.is_empty());
        let lit = toks.iter().find(|t| t.kind == IntSized).unwrap();
        assert_eq!(&"assign y = a & 8'hFF;"[lit.span.clone()], "8'hFF");
    }

    #[test]
    fn cast_apostrophe_is_additive() {
        use TokenKind::*;
        // Sized/unsized/fill literals must STILL lex as one atomic token
        // (logos longest-match beats the new 1-char Apostrophe).
        assert_eq!(kinds("8'hFF"), vec![IntSized]);
        assert_eq!(kinds("4'sd5"), vec![IntSized]);
        assert_eq!(kinds("'hFF"), vec![IntUnsizedBased]);
        for fill in ["'0", "'1", "'x", "'z"] {
            assert_eq!(kinds(fill), vec![IntUnsizedBased], "{fill}");
        }
        // Cast forms: a bare `'` followed by `(` is now a clean Apostrophe token.
        assert_eq!(kinds("8'("), vec![IntDecimal, Apostrophe, LParen]);
        assert_eq!(
            kinds("int'("),
            vec![Word(WordKind::Keyword(Kw::Int)), Apostrophe, LParen]
        );
        assert_eq!(
            kinds("signed'("),
            vec![Word(WordKind::Keyword(Kw::Signed)), Apostrophe, LParen]
        );
        // Assignment-pattern `'{` is NOT fused — it splits into Apostrophe + LBrace
        // and stays a parser-level loud reject (unsupported), never a cast.
        assert_eq!(kinds("'{"), vec![Apostrophe, LBrace]);
    }

    #[test]
    fn ex2_nonblocking_and_shift() {
        use TokenKind::*;
        // `q <= d << 2;`  — `<=` is ONE token (LtEq), `<<` is Shl.
        assert_eq!(
            kinds("q <= d << 2;"),
            vec![
                Word(WordKind::Ident), // q
                LtEq,
                Word(WordKind::Ident), // d
                Shl,
                IntDecimal, // 2
                Semi,
            ]
        );
    }

    #[test]
    fn ex3_real_vs_int_and_dot() {
        use TokenKind::*;
        // `real f = 3.14; x = .5;` — 3.14 is ONE RealLiteral; `.5` is NOT a real
        // (it splits to Dot + IntDecimal, which the parser rejects).
        assert_eq!(
            kinds("real f = 3.14;"),
            vec![
                Word(WordKind::Keyword(Kw::Real)),
                Word(WordKind::Ident), // f
                Eq,
                RealFixed, // 3.14
                Semi,
            ]
        );
        // exponent form wins by maximal munch over fixed/decimal:
        assert_eq!(kinds("1.5e3"), vec![RealExp]);
        assert_eq!(kinds("64e0"), vec![RealExp]);
        // `.5` alone: Dot then IntDecimal (malformed real -> parser's problem)
        assert_eq!(kinds(".5"), vec![Dot, IntDecimal]);
        // `1.` alone: IntDecimal then Dot
        assert_eq!(kinds("1."), vec![IntDecimal, Dot]);
    }

    #[test]
    fn ex4_keywords_case_sensitive_and_sv() {
        use TokenKind::*;
        // lowercase `module` is a keyword; `Module` is an identifier.
        assert_eq!(
            kinds("module Module always_ff logic"),
            vec![
                Word(WordKind::Keyword(Kw::Module)),
                Word(WordKind::Ident), // Module (capital M)
                Word(WordKind::Keyword(Kw::AlwaysFf)),
                Word(WordKind::Keyword(Kw::Logic)),
            ]
        );
        // `module2` is one identifier (maximal munch), not keyword + `2`.
        assert_eq!(kinds("module2"), vec![Word(WordKind::Ident)]);
    }

    #[test]
    fn ex5_system_task_directive_escaped_and_event() {
        use TokenKind::*;
        // `$display`, `` `timescale ``, escaped id `\a[0] `, event `@(posedge clk)`
        assert_eq!(kinds("$display"), vec![SystemTask]);
        assert_eq!(kinds("`timescale"), vec![Directive]);
        // escaped ident: backslash + body, terminated by the space (space dropped)
        let (toks, errs) = lex("\\a[0] x");
        assert!(errs.is_empty());
        assert_eq!(toks[0].kind, EscapedIdent);
        assert_eq!(&"\\a[0] x"[toks[0].span.clone()], "\\a[0]"); // ws not included
        assert_eq!(toks[1].kind, Word(WordKind::Ident)); // x
                                                         // event control
        assert_eq!(
            kinds("@(posedge clk)"),
            vec![
                At,
                LParen,
                Word(WordKind::Keyword(Kw::Posedge)),
                Word(WordKind::Ident), // clk
                RParen,
            ]
        );
    }

    #[test]
    fn ex6_unterminated_string_and_block_comment_are_errors() {
        use TokenKind::*;
        // unterminated string -> Error token + a LexError, no panic
        let (toks, errs) = lex("\"oops");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, Error(LexErrorKind::UnterminatedString));
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].kind, LexErrorKind::UnterminatedString);
        // good string is one Str token
        assert_eq!(kinds("\"hello \\\"world\\\"\""), vec![Str]);
        // unterminated block comment
        let (_t, errs) = lex("/* never closed");
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].kind, LexErrorKind::UnterminatedBlockComment);
        // closed block comment is a boundary -> no tokens
        assert!(kinds("/* c */").is_empty());
    }

    #[test]
    fn ex7_string_backslash_edge_cases_never_panic() {
        use TokenKind::*;
        // BLOCKER regression: a string ending in a trailing backslash at EOF
        // must NOT panic (logos bump overrun). It is ONE unterminated string;
        // the trailing `\` is consumed into the broken-string token.
        let (toks, errs) = lex("\"abc\\");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, Error(LexErrorKind::UnterminatedString));
        assert_eq!(&"\"abc\\"[toks[0].span.clone()], "\"abc\\");
        assert_eq!(errs.len(), 1);
        // Multiple trailing backslashes: `"\\\` (quote + 3 backslashes) is one
        // unterminated string, no panic. (Rust `"\"\\\\\\"` == `"` `\` `\` `\`.)
        let (toks, errs) = lex("\"\\\\\\");
        assert_eq!(toks.len(), 1);
        assert_eq!(errs[0].kind, LexErrorKind::UnterminatedString);
        // MAJOR regression: `\` immediately before a newline must NOT swallow the
        // newline — the string is unterminated at the newline, not multi-line.
        // The broken-string token spans `"abc\` (through the backslash, up to but
        // not including the newline); `def"` then lexes as its own line.
        let (toks, errs) = lex("\"abc\\\ndef\"");
        assert_eq!(toks[0].kind, Error(LexErrorKind::UnterminatedString));
        assert_eq!(&"\"abc\\\ndef\""[toks[0].span.clone()], "\"abc\\");
        assert_eq!(errs[0].kind, LexErrorKind::UnterminatedString);
        // mid-UTF-8 hazard: `\` before a multibyte char, then EOF -> clamped, no panic
        let (toks, errs) = lex("\"x\\ä");
        assert_eq!(toks[0].kind, Error(LexErrorKind::UnterminatedString));
        assert_eq!(errs[0].kind, LexErrorKind::UnterminatedString);
        // a normal escaped quote inside a closed string still works
        assert_eq!(kinds("\"a\\\"b\""), vec![Str]);
    }

    #[test]
    fn ex8_lone_sigil_recovers_with_lonesigil_reason() {
        use TokenKind::*;
        // bare `$` is a REAL token since v5 ⑥ (queue last-index `q[$]`)
        let (toks, errs) = lex("$ x");
        assert_eq!(errs.len(), 0);
        assert_eq!(toks[0].kind, Dollar);
        assert_eq!(toks[1].kind, Word(WordKind::Ident)); // x
                                                         // bare backtick -> LoneSigil; `$display`/`` `define `` still munch longer
        let (_t, errs) = lex("`");
        assert_eq!(errs[0].kind, LexErrorKind::LoneSigil);
        assert_eq!(kinds("$display"), vec![SystemTask]);
        assert_eq!(kinds("`define"), vec![Directive]);
    }

    #[test]
    fn wildcard_eq_tokens_munch_longest() {
        use TokenKind::*;
        assert_eq!(kinds("==?"), vec![EqEqQ]);
        assert_eq!(kinds("!=?"), vec![BangEqQ]);
        // no 4-char operator: `===?` stays case-eq + ternary-question.
        assert_eq!(kinds("===?"), vec![EqEqEq, Question]);
        assert_eq!(kinds("!==?"), vec![BangEqEq, Question]);
        // spaced forms are unchanged (two tokens).
        assert_eq!(kinds("== ?"), vec![EqEq, Question]);
        // a ternary right after an equality COMPARAND is unaffected.
        assert_eq!(
            kinds("a==b?c:d"),
            vec![
                Word(WordKind::Ident),
                EqEq,
                Word(WordKind::Ident),
                Question,
                Word(WordKind::Ident),
                Colon,
                Word(WordKind::Ident)
            ]
        );
    }

    #[test]
    fn operator_prefix_families_munch_longest() {
        use TokenKind::*;
        assert_eq!(kinds("==="), vec![EqEqEq]);
        assert_eq!(kinds("=="), vec![EqEq]);
        assert_eq!(kinds("="), vec![Eq]);
        assert_eq!(kinds("<<<"), vec![ShlA]);
        assert_eq!(kinds("<<"), vec![Shl]);
        assert_eq!(kinds("<="), vec![LtEq]);
        assert_eq!(kinds("<"), vec![Lt]);
        assert_eq!(kinds("~^"), vec![TildeCaret]);
        assert_eq!(kinds("^~"), vec![CaretTilde]);
        assert_eq!(kinds("->"), vec![Arrow]);
        assert_eq!(
            kinds("[a+:b]"),
            vec![
                LBracket,
                Word(WordKind::Ident),
                PlusColon,
                Word(WordKind::Ident),
                RBracket,
            ]
        );
    }
}
