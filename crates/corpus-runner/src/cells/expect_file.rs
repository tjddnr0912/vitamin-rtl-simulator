//! A reader for the preserved `.expect` files (`crates/testdata/cells/README.md`,
//! "The .expect format"). Read-only: nothing here writes one.

/// One output block, with `identical to block N` already resolved to N's bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub index: usize,
    /// The producer, as the header names it: `oracle iverilog 13.0 (…)`, `vita [PRE]`,
    /// `combined [x] …`.
    pub kind: String,
    /// The capture's path in the slice's scratch directory.
    pub path: String,
    pub body: Vec<u8>,
}

impl Block {
    /// The oracle that produced this block — `iverilog`, `verilator`, `sv2v` (sv2v,
    /// then iverilog), `xcelium` — or `None` for vita and anything combined.
    pub fn oracle(&self) -> Option<&str> {
        let rest = self.kind.strip_prefix("oracle ")?;
        rest.split(' ').next().filter(|t| !t.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectFile {
    /// `key: value` header lines, in order (`vita_binary` repeats).
    pub headers: Vec<(String, String)>,
    pub blocks: Vec<Block>,
}

impl ExpectFile {
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// Parse one `.expect`. Byte-counted, as the format requires: captured text can
/// contain lines that begin with `===`, so a block ends where its header's byte count
/// says, never at the next header-looking line.
pub fn parse(bytes: &[u8]) -> Result<ExpectFile, String> {
    let mut pos = 0usize;
    let mut headers = Vec::new();
    let mut declared = None;
    // Header lines up to and including `blocks: N`.
    while declared.is_none() {
        let line = take_line(bytes, &mut pos).ok_or("no `blocks:` line")?;
        if line.starts_with('#') {
            continue;
        }
        let (k, v) = line
            .split_once(": ")
            .ok_or_else(|| format!("header line without `: `: {line:?}"))?;
        if k == "blocks" {
            declared = Some(v.parse::<usize>().map_err(|e| format!("blocks: {e}"))?);
        }
        headers.push((k.to_string(), v.to_string()));
    }
    let declared = declared.unwrap_or(0);

    let mut blocks: Vec<Block> = Vec::new();
    loop {
        let line = take_line(bytes, &mut pos).ok_or("missing `=== end`")?;
        if line == "=== end" {
            break;
        }
        let head = line
            .strip_prefix("=== block ")
            .ok_or_else(|| format!("expected a block header, got {line:?}"))?;
        let fields: Vec<&str> = head.split(" | ").collect();
        if fields.len() < 4 {
            return Err(format!("short block header: {line:?}"));
        }
        let index: usize = fields[0].parse().map_err(|e| format!("block index: {e}"))?;
        let n: usize = fields[3]
            .strip_suffix(" bytes")
            .ok_or_else(|| format!("no byte count: {line:?}"))?
            .parse()
            .map_err(|e| format!("byte count: {e}"))?;
        let same_as = fields
            .get(4)
            .and_then(|f| f.strip_prefix("identical to block "))
            .map(|i| i.parse::<usize>().map_err(|e| format!("identical to: {e}")))
            .transpose()?;
        let body = match same_as {
            Some(i) => {
                let b = blocks
                    .iter()
                    .find(|b| b.index == i)
                    .ok_or_else(|| format!("block {index} copies missing block {i}"))?;
                if b.body.len() != n {
                    return Err(format!("block {index} copies {i} but sizes differ"));
                }
                b.body.clone()
            }
            None => {
                let end = pos + n;
                let body = bytes
                    .get(pos..end)
                    .ok_or_else(|| format!("block {index} runs past the end"))?
                    .to_vec();
                pos = end;
                // A newline was added after content that does not end in one —
                // including empty content.
                if (n == 0 || body.last() != Some(&b'\n')) && bytes.get(pos) == Some(&b'\n') {
                    pos += 1;
                }
                body
            }
        };
        blocks.push(Block {
            index,
            kind: fields[1].to_string(),
            path: fields[2].to_string(),
            body,
        });
    }
    if blocks.len() != declared {
        return Err(format!(
            "`blocks: {declared}` but {} were read",
            blocks.len()
        ));
    }
    Ok(ExpectFile { headers, blocks })
}

fn take_line<'a>(bytes: &'a [u8], pos: &mut usize) -> Option<&'a str> {
    let rest = bytes.get(*pos..)?;
    if rest.is_empty() {
        return None;
    }
    let nl = rest.iter().position(|&b| b == b'\n')?;
    *pos += nl + 1;
    std::str::from_utf8(&rest[..nl]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A block whose content begins with `===` and an empty, newline-padded block: both
    /// must be read by count.
    #[test]
    fn blocks_are_read_by_byte_count() {
        let text = b"# c\ncell: x\nblocks: 3\n\
=== block 1 | oracle iverilog 13.0 (v) | ./x.ivl | 12 bytes\n=== block 9\n\
=== block 2 | vita [PRE] | ./x.pre | 0 bytes\n\n\
=== block 3 | vita [P2] | ./x.p2 | 12 bytes | identical to block 1\n\
=== end\n";
        let f = parse(text).expect("parses");
        assert_eq!(f.header("cell"), Some("x"));
        assert_eq!(f.blocks.len(), 3);
        assert_eq!(f.blocks[0].body, b"=== block 9\n");
        assert_eq!(f.blocks[0].oracle(), Some("iverilog"));
        assert_eq!(f.blocks[1].body, b"");
        assert_eq!(f.blocks[1].oracle(), None);
        assert_eq!(f.blocks[2].body, b"=== block 9\n");
    }

    #[test]
    fn content_without_a_final_newline_gets_one_after_it() {
        let text = b"blocks: 2\n=== block 1 | oracle sv2v 0.0.13 | ./x.s2v | 3 bytes\nabc\n\
=== block 2 | oracle verilator 5 | ./x.vl | 2 bytes\nd\n=== end\n";
        let f = parse(text).expect("parses");
        assert_eq!(f.blocks[0].body, b"abc");
        assert_eq!(f.blocks[1].body, b"d\n");
    }

    #[test]
    fn a_wrong_count_is_an_error_not_a_guess() {
        let text = b"blocks: 2\n=== block 1 | vita [P] | ./x | 1 bytes\na\n=== end\n";
        assert!(parse(text).is_err());
    }
}
