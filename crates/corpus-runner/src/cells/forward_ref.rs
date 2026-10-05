//! A conservative textual detector for a parameter used before it is declared.
//!
//! ROADMAP's oracle-split ruling gives forward references in a scope to iverilog and
//! IEEE 1800 §26.3: sv2v (which reorders declarations) and verilator are disqualified
//! on that axis. So a cell whose admission rests on those two alone — iverilog absent,
//! for example stopped by a `sorry:` — must not be pinned when the design refers to a
//! `parameter` / `localparam` above its declaration. This finds that shape in the
//! source text. It over-reports by design: a name declared in one scope and used
//! earlier in another (a legal shadow) is reported too, and excluding such a cell only
//! costs coverage.

/// One identifier, with the line it starts on.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Token<'a> {
    text: &'a str,
    line: u32,
    /// An identifier, as opposed to punctuation, a number, a string, a `$name` or a
    /// `` `directive ``.
    ident: bool,
}

/// A parameter name read on a line above any of its declarations: `(name, line of
/// the first use, line of the first declaration)`. `None` when there is none.
pub fn forward_parameter_reference(src: &str) -> Option<(String, u32, u32)> {
    let toks = tokens(src);
    // Every name declared after `parameter` / `localparam`: `NAME [dims] = …`, and the
    // names after commas in the same declaration, at the keyword's bracket depth.
    let mut first_decl: std::collections::BTreeMap<&str, u32> = std::collections::BTreeMap::new();
    let mut i = 0;
    while i < toks.len() {
        if !(toks[i].ident && matches!(toks[i].text, "parameter" | "localparam")) {
            i += 1;
            continue;
        }
        let mut depth = 0i32;
        let mut j = i + 1;
        while j < toks.len() {
            let t = &toks[j];
            match t.text {
                ";" if depth == 0 => break,
                "(" | "{" => depth += 1,
                ")" | "}" => {
                    depth -= 1;
                    if depth < 0 {
                        break;
                    }
                }
                "parameter" | "localparam" if t.ident => break,
                _ => {}
            }
            if t.ident && depth == 0 && assigned(&toks, j) {
                let line = first_decl.entry(t.text).or_insert(t.line);
                *line = (*line).min(t.line);
            }
            j += 1;
        }
        i = j.max(i + 1);
    }
    // The first use of each such name above its first declaration. A name after a `.`
    // is a member of another scope (`u.K`) or a named override (`.K(3)`), not a use.
    toks.iter()
        .enumerate()
        .filter(|(k, t)| t.ident && !(*k > 0 && toks[k - 1].text == "."))
        .filter_map(|(_, t)| {
            let d = *first_decl.get(t.text)?;
            (t.line < d).then(|| (t.text.to_string(), t.line, d))
        })
        .min_by_key(|(_, used, _)| *used)
}

/// Whether the identifier at `j` is followed by optional `[ … ]` groups and a single
/// `=` (not `==`).
fn assigned(toks: &[Token], j: usize) -> bool {
    let mut k = j + 1;
    while toks.get(k).is_some_and(|t| t.text == "[") {
        let mut d = 0;
        while let Some(t) = toks.get(k) {
            match t.text {
                "[" => d += 1,
                "]" => {
                    d -= 1;
                    if d == 0 {
                        break;
                    }
                }
                _ => {}
            }
            k += 1;
        }
        k += 1;
    }
    toks.get(k).is_some_and(|t| t.text == "=") && toks.get(k + 1).is_none_or(|t| t.text != "=")
}

/// The source as tokens, comments and strings skipped.
fn tokens(src: &str) -> Vec<Token<'_>> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut line = 1u32;
    let mut i = 0;
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'$';
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            line += 1;
            i += 1;
        } else if c.is_ascii_whitespace() {
            i += 1;
        } else if b[i..].starts_with(b"//") {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if b[i..].starts_with(b"/*") {
            i += 2;
            while i < b.len() && !b[i..].starts_with(b"*/") {
                if b[i] == b'\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 2;
        } else if c == b'"' {
            i += 1;
            while i < b.len() && b[i] != b'"' {
                if b[i] == b'\\' {
                    i += 1;
                }
                if i < b.len() && b[i] == b'\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 1;
        } else if c.is_ascii_digit() || c == b'\'' {
            // A number or a based literal: `8'hFC`, `'0`, `1.5`. `'{` is punctuation.
            if c == b'\'' && b.get(i + 1) == Some(&b'{') {
                out.push(Token {
                    text: &src[i..i + 1],
                    line,
                    ident: false,
                });
                i += 1;
                continue;
            }
            i += 1;
            while i < b.len() && (word(b[i]) || b[i] == b'\'' || b[i] == b'.' || b[i] == b'?') {
                i += 1;
            }
        } else if c == b'$' || c == b'`' {
            let s = i;
            i += 1;
            while i < b.len() && word(b[i]) {
                i += 1;
            }
            out.push(Token {
                text: &src[s..i],
                line,
                ident: false,
            });
        } else if c == b'\\' {
            // An escaped identifier runs to the next whitespace.
            let s = i;
            while i < b.len() && !b[i].is_ascii_whitespace() {
                i += 1;
            }
            out.push(Token {
                text: &src[s..i],
                line,
                ident: true,
            });
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let s = i;
            while i < b.len() && word(b[i]) {
                i += 1;
            }
            out.push(Token {
                text: &src[s..i],
                line,
                ident: true,
            });
        } else {
            let len = src[i..].chars().next().map_or(1, char::len_utf8);
            out.push(Token {
                text: &src[i..i + len],
                line,
                ident: false,
            });
            i += len;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `AI/s594__i__r2__C5.sv`'s shape: a localparam whose initializer reads an array
    /// parameter declared on the next line.
    #[test]
    fn a_parameter_read_above_its_declaration_is_found() {
        let src = "module t;\n  function automatic int fl(input int x); return x; endfunction\n\
                   localparam int L = fl(0) + ((AS[0] + 8'd0) == 8'hFC);\n\
                   localparam logic [7:0] AS [0:1] = '{8'hFC, 8'h01};\n\
                   initial $display(\"L=%0d\", L);\nendmodule\n";
        assert_eq!(forward_parameter_reference(src), Some(("AS".into(), 3, 4)));
    }

    #[test]
    fn declarations_before_use_comments_strings_and_members_are_not_uses() {
        let src = "module t #(parameter int W = 8, K = W + 1);\n\
                   // K and P in a comment\n  /* P */\n\
                   initial $display(\"P=%0d K\", u.P);\n  sub #(.P(3)) u();\n\
                   localparam P = K == 9;\nendmodule\n";
        assert_eq!(forward_parameter_reference(src), None);
        // `==` is not a declaration's `=`.
        assert_eq!(
            forward_parameter_reference(
                "module t;\n initial if (Q == 1) ;\n localparam Q = 1;\nendmodule\n"
            ),
            Some(("Q".into(), 2, 3))
        );
    }

    /// A legal shadow is reported too: the detector over-excludes on purpose.
    #[test]
    fn a_name_used_above_an_inner_declaration_is_reported() {
        let src = "module t;\n  initial $display(\"%0d\", N);\n  if (1) begin : g\n    localparam N = 2;\n  end\nendmodule\n";
        assert_eq!(forward_parameter_reference(src), Some(("N".into(), 2, 4)));
    }
}
