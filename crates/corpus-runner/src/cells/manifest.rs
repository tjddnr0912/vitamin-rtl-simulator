//! The curated manifest: `crates/testdata/cells/MANIFEST.txt`, written only by
//! `corpus-runner cells pin`.
//!
//! A text file rather than a Rust table (`corpus.rs` is one) because it is generated:
//! a few thousand entries whose pinned lines are arbitrary design output. Its shape
//! follows the `.expect` files beside it — `key value` lines, then counted blocks —
//! with every pinned line prefixed by `| `, so a design that prints `cell x` or a blank
//! line cannot be mistaken for structure, and a diff shows the line as printed.
//!
//! ```text
//! cell AE/s588__g__a2__a43_ret4_read_before.sv
//! name a43_ret4_read_before.sv
//! oracles iverilog,sv2v,verilator
//! expect known-wrong 0          (or `runs 0`, or `refused`, below)
//! oracle 1
//! | P=xxxx
//! vita 1                        (known-wrong only: vita's pinned wrong answer)
//! | P=0001
//! ```
//!
//! A refused cell pins vita's first error line (its file name stripped), the sorted
//! codes of every error it printed and the sorted positions of every error, all three
//! compared exactly:
//!
//! ```text
//! expect refused
//! refusal 3:29: error[VITA-E3009] E-ELAB-UNSUPPORTED: a function call that … [in top]
//! codes VITA-E3009
//! at 3:29
//! ```
//!
//! Entries are separated by a blank line and sorted by `cell`. Lines starting with
//! `#` before the first entry are a generated header.

use super::admit::{row_of, Excluded, Reason};
use crate::Expect;

/// One manifest entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// `<row>/<cell>.sv`, relative to the cells directory.
    pub path: String,
    /// The file name to run it under.
    pub name: String,
    /// The oracles whose answers agreed, sorted.
    pub oracles: Vec<String>,
    /// Their normalized answer, lines joined by `\n`.
    pub oracle: String,
    /// What vita did with it when it was pinned. `Runs`, `KnownWrong` or `Refused`;
    /// a cell is never `Split` — no ruling stands behind one.
    pub expect: Expect<String>,
}

impl Cell {
    pub fn row(&self) -> &str {
        row_of(&self.path)
    }

    /// The expectation's manifest keyword.
    pub fn kind(&self) -> &'static str {
        kind_of(&self.expect)
    }
}

pub fn kind_of<S>(e: &Expect<S>) -> &'static str {
    match e {
        Expect::Runs { .. } => "runs",
        Expect::KnownWrong { .. } => "known-wrong",
        Expect::Refused { .. } => "refused",
        Expect::Split { .. } => "split",
    }
}

/// Render the manifest. `header` lines are written as `# ` comments.
pub fn render(header: &[String], cells: &[Cell]) -> String {
    let mut s = String::new();
    for h in header {
        s.push_str("# ");
        s.push_str(h);
        s.push('\n');
    }
    for c in cells {
        s.push('\n');
        s.push_str(&format!("cell {}\nname {}\n", c.path, c.name));
        s.push_str(&format!("oracles {}\n", c.oracles.join(",")));
        match &c.expect {
            Expect::Runs { exit } => s.push_str(&format!("expect runs {exit}\n")),
            Expect::KnownWrong { exit, .. } => s.push_str(&format!("expect known-wrong {exit}\n")),
            Expect::Refused { diag } => {
                // `refusal_pin`'s three lines: the first is prefixed, the other two
                // already read `codes …` and `at …`.
                let (first, rest) = diag.split_once('\n').unwrap_or((diag, ""));
                s.push_str(&format!("expect refused\nrefusal {first}\n{rest}\n"));
            }
            Expect::Split { .. } => unreachable!("a cell is never pinned as a ruled split"),
        }
        push_block(&mut s, "oracle", &c.oracle);
        if let Expect::KnownWrong { vita, .. } = &c.expect {
            push_block(&mut s, "vita", vita);
        }
    }
    s
}

fn push_block(s: &mut String, key: &str, joined: &str) {
    let lines: Vec<&str> = if joined.is_empty() {
        Vec::new()
    } else {
        joined.split('\n').collect()
    };
    s.push_str(&format!("{key} {}\n", lines.len()));
    for l in lines {
        s.push_str("| ");
        s.push_str(l);
        s.push('\n');
    }
}

/// Parse a manifest. Strict: anything out of shape is an error naming its line, so a
/// hand edit that breaks the format is caught rather than half-read.
pub fn parse(text: &str) -> Result<Vec<Cell>, String> {
    let mut cur = Cursor {
        lines: text.split('\n').collect(),
        at: 0,
    };
    // A final newline leaves one empty piece; it is not a separator.
    if cur.lines.last() == Some(&"") {
        cur.lines.pop();
    }
    while cur.lines.get(cur.at).is_some_and(|l| l.starts_with('#')) {
        cur.at += 1;
    }
    let mut cells: Vec<Cell> = Vec::new();
    while cur.at < cur.lines.len() {
        cur.blank()?;
        let at = cur.at + 1;
        let path = cur.field("cell")?.to_string();
        let name = cur.field("name")?.to_string();
        let oracles = cur
            .field("oracles")?
            .split(',')
            .map(str::to_string)
            .collect();
        let eline = cur.at + 1;
        let exp = cur.field("expect")?.to_string();
        let refusal = if exp == "refused" {
            let first = cur.field("refusal")?;
            let codes = cur.field("codes")?;
            let at = cur.field("at")?;
            Some(format!("{first}\ncodes {codes}\nat {at}"))
        } else {
            None
        };
        let oracle = cur.block("oracle")?;
        let (kind, arg) = exp.split_once(' ').unwrap_or((&exp, ""));
        let exit = || {
            arg.parse::<i32>()
                .map_err(|e| format!("line {eline}: exit code {arg:?}: {e}"))
        };
        let expect = match kind {
            "runs" => Expect::Runs { exit: exit()? },
            "refused" => Expect::Refused {
                diag: refusal.unwrap_or_default(),
            },
            "known-wrong" => Expect::KnownWrong {
                exit: exit()?,
                vita: cur.block("vita")?,
            },
            other => return Err(format!("line {eline}: unknown expectation {other:?}")),
        };
        if cells.last().is_some_and(|p| p.path >= path) {
            return Err(format!("line {at}: {path} is out of order or repeated"));
        }
        cells.push(Cell {
            path,
            name,
            oracles,
            oracle,
            expect,
        });
    }
    Ok(cells)
}

struct Cursor<'a> {
    lines: Vec<&'a str>,
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, want: &str) -> Result<&'a str, String> {
        let l = self
            .lines
            .get(self.at)
            .copied()
            .ok_or_else(|| format!("unexpected end of the manifest, expected {want}"))?;
        self.at += 1;
        Ok(l)
    }

    fn blank(&mut self) -> Result<(), String> {
        let at = self.at + 1;
        match self.take("a blank line")? {
            "" => Ok(()),
            l => Err(format!("line {at}: expected a blank line, got {l:?}")),
        }
    }

    fn field(&mut self, key: &str) -> Result<&'a str, String> {
        let at = self.at + 1;
        let l = self.take(key)?;
        l.strip_prefix(key)
            .and_then(|r| r.strip_prefix(' '))
            .ok_or_else(|| format!("line {at}: expected `{key} …`, got {l:?}"))
    }

    /// `key N` followed by N `| `-prefixed lines, joined by `\n`.
    fn block(&mut self, key: &str) -> Result<String, String> {
        let at = self.at + 1;
        let n = self.field(key)?;
        let n: usize = n
            .parse()
            .map_err(|e| format!("line {at}: line count {n:?}: {e}"))?;
        let mut out: Vec<&str> = Vec::with_capacity(n);
        for _ in 0..n {
            let at = self.at + 1;
            let l = self.take("a `| ` line")?;
            out.push(
                l.strip_prefix("| ")
                    .ok_or_else(|| format!("line {at}: expected `| …`, got {l:?}"))?,
            );
        }
        Ok(out.join("\n"))
    }
}

/// Render the exclusion list: `path<TAB>reason<TAB>detail`, sorted by path.
pub fn render_excluded(header: &[String], excluded: &[Excluded]) -> String {
    let mut s = String::new();
    for h in header {
        s.push_str("# ");
        s.push_str(h);
        s.push('\n');
    }
    s.push_str("path\treason\tdetail\n");
    for x in excluded {
        let detail = x.detail.replace(['\t', '\n'], " ");
        s.push_str(&format!("{}\t{}\t{detail}\n", x.path, x.reason.label()));
    }
    s
}

/// Parse [`render_excluded`]'s output.
pub fn parse_excluded(text: &str) -> Result<Vec<Excluded>, String> {
    let mut out = Vec::new();
    for (i, l) in text.lines().enumerate() {
        if l.starts_with('#') || l == "path\treason\tdetail" || l.is_empty() {
            continue;
        }
        let mut f = l.splitn(3, '\t');
        let (path, reason, detail) = (f.next(), f.next(), f.next());
        let (Some(path), Some(reason), Some(detail)) = (path, reason, detail) else {
            return Err(format!("line {}: expected three fields", i + 1));
        };
        out.push(Excluded {
            path: path.to_string(),
            reason: Reason::from_label(reason)
                .ok_or_else(|| format!("line {}: unknown reason {reason:?}", i + 1))?,
            detail: detail.to_string(),
        });
    }
    Ok(out)
}
