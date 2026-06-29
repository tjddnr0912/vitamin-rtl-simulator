//! P2-A worklib (doc-14 §1/§3 v1 subset): the `work/` library directory.
//!
//! Layout: `<dir>/lib.toml` (canonical, machine-written manifest) +
//! `<dir>/units/cu_<hex16>.vu` (content-addressed compilation-unit blobs —
//! byte-identical to what `vcmp -o` would write, so the frozen `.vu` format
//! is untouched).
//!
//! v1 deviations from the doc-14 sketch (documented there): blobs are
//! per-COMPILATION-UNIT, not per-unit (vita's single-CU concat model — every
//! unit entry points at its CU blob), and `src_sha256` digests are RAW file
//! bytes + the recorded `-D`/`-I` surface + the include closure, which has
//! the same staleness detection power as post-preprocess digests while
//! per-file preprocessed bytes stay undefined under CU concatenation.
//!
//! Determinism contract: the manifest is emitted in ONE canonical text form
//! (fixed key order, one array element per line, `\n` only) and parsed by a
//! STRICT reader that accepts exactly that form — `blake3(lib.toml bytes)` is
//! the staleness key recorded into `.velab` snapshots (RULE V).

use serde::{Deserialize, Serialize};

/// One compilation unit recorded in a library manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cu {
    /// Blob path relative to the library dir (`units/cu_<hex16>.vu`).
    pub blob: String,
    /// `-D NAME[=VAL]` surface in argv order (preprocessing input).
    pub defines: Vec<String>,
    /// `-I` dirs in argv order (preprocessing input).
    pub incdirs: Vec<String>,
    /// Source files as given to vcmp, with raw-byte blake3 digests.
    pub sources: Vec<(String, [u8; 32])>,
    /// `\`include` closure (paths the preprocessor actually opened).
    pub includes: Vec<(String, [u8; 32])>,
    /// Design units this CU defines: (kind, name) in declaration order.
    pub units: Vec<(String, String)>,
}

/// A parsed `lib.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    pub cus: Vec<Cu>,
}

/// What a lib-mode `.velab` records about its upstream (the 9th trailer).
/// Tolerant-empty on read: a legacy `.velab` simply has no work gate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkConsumed {
    /// (logical name, dir as given, blake3 of lib.toml bytes) per `-L`.
    pub libs: Vec<(String, String, [u8; 32])>,
    /// (path as resolved at velab time, blake3 of blob bytes) per consumed CU.
    pub blobs: Vec<(String, [u8; 32])>,
    /// (path, raw blake3) for every source+include of every consumed CU.
    pub files: Vec<(String, [u8; 32])>,
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn unhex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

/// Minimal string escaping for the canonical form (`\` and `"`).
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn unesc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            if let Some(n) = it.next() {
                out.push(n);
            }
        } else {
            out.push(c);
        }
    }
    out
}

impl Manifest {
    /// Canonical text form — the ONLY form `parse` accepts and the byte domain
    /// of the staleness hash.
    pub fn emit(&self) -> String {
        let mut s = String::new();
        s.push_str(
            "# vitamin work library manifest (canonical v1; machine-written by `vcmp --work`)\n",
        );
        s.push_str("format_version = 1\n");
        s.push_str(&format!("name = \"{}\"\n", esc(&self.name)));
        for cu in &self.cus {
            s.push_str("\n[[cu]]\n");
            s.push_str(&format!("blob = \"{}\"\n", esc(&cu.blob)));
            emit_str_array(&mut s, "defines", cu.defines.iter().map(|d| esc(d)));
            emit_str_array(&mut s, "incdirs", cu.incdirs.iter().map(|d| esc(d)));
            emit_str_array(
                &mut s,
                "sources",
                cu.sources
                    .iter()
                    .map(|(p, h)| format!("{}  {}", hex(h), esc(p))),
            );
            emit_str_array(
                &mut s,
                "includes",
                cu.includes
                    .iter()
                    .map(|(p, h)| format!("{}  {}", hex(h), esc(p))),
            );
            emit_str_array(
                &mut s,
                "units",
                cu.units
                    .iter()
                    .map(|(k, n)| format!("{} {}", esc(k), esc(n))),
            );
        }
        s
    }

    /// Strict parser for the canonical form. Any deviation is an error with a
    /// 1-based line number (surfaced as `E-WORK-MANIFEST`).
    pub fn parse(text: &str) -> Result<Manifest, String> {
        let mut lines = text.lines().enumerate().peekable();
        let mut next_content = |lines: &mut std::iter::Peekable<
            std::iter::Enumerate<std::str::Lines>,
        >|
         -> Option<(usize, String)> {
            for (i, l) in lines.by_ref() {
                let t = l.trim_end();
                if t.is_empty() || t.starts_with('#') {
                    continue;
                }
                return Some((i + 1, t.to_string()));
            }
            None
        };
        let (ln, l) = next_content(&mut lines).ok_or("empty manifest")?;
        if l != "format_version = 1" {
            return Err(format!("not a canonical work manifest (line {ln})"));
        }
        let (ln, l) = next_content(&mut lines).ok_or("missing `name`")?;
        let name =
            parse_str_kv(&l, "name").ok_or(format!("expected `name = \"…\"` (line {ln})"))?;
        let mut cus = Vec::new();
        while let Some((ln, l)) = next_content(&mut lines) {
            if l != "[[cu]]" {
                return Err(format!("expected `[[cu]]` (line {ln})"));
            }
            let (ln, l) = next_content(&mut lines).ok_or("truncated [[cu]]")?;
            let blob =
                parse_str_kv(&l, "blob").ok_or(format!("expected `blob = \"…\"` (line {ln})"))?;
            let defines = parse_str_array(&mut lines, &mut next_content, "defines")?;
            let incdirs = parse_str_array(&mut lines, &mut next_content, "incdirs")?;
            let sources = parse_digest_array(&mut lines, &mut next_content, "sources")?;
            let includes = parse_digest_array(&mut lines, &mut next_content, "includes")?;
            let units_raw = parse_str_array(&mut lines, &mut next_content, "units")?;
            let mut units = Vec::new();
            for u in &units_raw {
                let (k, n) = u.split_once(' ').ok_or(format!("bad unit entry `{u}`"))?;
                units.push((k.to_string(), n.to_string()));
            }
            cus.push(Cu {
                blob,
                defines,
                incdirs,
                sources,
                includes,
                units,
            });
        }
        Ok(Manifest { name, cus })
    }

    /// Every (kind, name) unit across all CUs, with its CU index.
    pub fn unit_index(&self) -> Vec<(&str, &str, usize)> {
        let mut out = Vec::new();
        for (ci, cu) in self.cus.iter().enumerate() {
            for (k, n) in &cu.units {
                out.push((k.as_str(), n.as_str(), ci));
            }
        }
        out
    }
}

fn emit_str_array(s: &mut String, key: &str, items: impl Iterator<Item = String>) {
    let v: Vec<String> = items.collect();
    if v.is_empty() {
        s.push_str(&format!("{key} = []\n"));
        return;
    }
    s.push_str(&format!("{key} = [\n"));
    for e in v {
        s.push_str(&format!("  \"{e}\",\n"));
    }
    s.push_str("]\n");
}

fn parse_str_kv(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.strip_prefix(" = \"")?;
    let inner = rest.strip_suffix('"')?;
    Some(unesc(inner))
}

type LineIter<'a> = std::iter::Peekable<std::iter::Enumerate<std::str::Lines<'a>>>;

fn parse_str_array<'a>(
    lines: &mut LineIter<'a>,
    next_content: &mut impl FnMut(&mut LineIter<'a>) -> Option<(usize, String)>,
    key: &str,
) -> Result<Vec<String>, String> {
    let (ln, l) = next_content(lines).ok_or(format!("truncated before `{key}`"))?;
    if l == format!("{key} = []") {
        return Ok(Vec::new());
    }
    if l != format!("{key} = [") {
        return Err(format!("expected `{key} = [` (line {ln})"));
    }
    let mut out = Vec::new();
    loop {
        let (ln, l) = next_content(lines).ok_or(format!("unterminated `{key}` array"))?;
        if l == "]" {
            return Ok(out);
        }
        let e = l
            .trim_start()
            .strip_prefix('"')
            .and_then(|r| r.strip_suffix("\","))
            .ok_or(format!("bad array element (line {ln})"))?;
        out.push(unesc(e));
    }
}

fn parse_digest_array<'a>(
    lines: &mut LineIter<'a>,
    next_content: &mut impl FnMut(&mut LineIter<'a>) -> Option<(usize, String)>,
    key: &str,
) -> Result<Vec<(String, [u8; 32])>, String> {
    let raw = parse_str_array(lines, next_content, key)?;
    let mut out = Vec::new();
    for e in &raw {
        let (h, p) = e
            .split_once("  ")
            .ok_or(format!("bad digest entry `{e}`"))?;
        let d = unhex32(h).ok_or(format!("bad digest hex in `{e}`"))?;
        out.push((p.to_string(), d));
    }
    Ok(out)
}

/// Outcome of [`add_cu`].
pub enum AddOutcome {
    /// Manifest mutated (or already byte-identical) and blob present.
    Ok,
    /// A unit in the new CU is already defined by an UNRELATED CU (different
    /// source paths) — E-DUP-UNIT; nothing was written.
    DupUnit(String),
}

/// Record one freshly-compiled CU into the library at `dir`, writing the blob
/// (content-addressed) and the canonical manifest atomically.
///
/// Replacement rule (incremental recompile): any existing CU sharing AT LEAST
/// ONE source path with the new CU is superseded — its entries drop and the
/// new CU takes the first such position (order churn ⇒ spurious staleness is
/// avoided for the common recompile-in-place flow; an UNCHANGED recompile is
/// a byte-identical no-op). After supersession a remaining duplicate unit
/// name is a real redefinition → `DupUnit`, manifest untouched.
pub fn add_cu(
    dir: &std::path::Path,
    lib_name: &str,
    blob_bytes: &[u8],
    mut cu: Cu,
    write_atomic: &dyn Fn(&str, &[u8]) -> std::io::Result<()>,
) -> Result<AddOutcome, String> {
    let mpath = dir.join("lib.toml");
    let mut manifest = if mpath.exists() {
        let text =
            std::fs::read_to_string(&mpath).map_err(|e| format!("{}: {e}", mpath.display()))?;
        let m = Manifest::parse(&text).map_err(|e| format!("{}: {e}", mpath.display()))?;
        if m.name != lib_name {
            return Err(format!(
                "{}: directory already holds library `{}` (requested `{lib_name}`)",
                mpath.display(),
                m.name
            ));
        }
        m
    } else {
        Manifest {
            name: lib_name.to_string(),
            cus: Vec::new(),
        }
    };

    // Canonical form is line-oriented: a newline in any recorded string
    // would corrupt it (loudly, but only on the NEXT parse) — reject now.
    let has_nl = |t: &str| t.contains('\n') || t.contains('\r');
    if has_nl(lib_name)
        || cu.defines.iter().any(|d| has_nl(d))
        || cu.incdirs.iter().any(|d| has_nl(d))
        || cu.sources.iter().any(|(p, _)| has_nl(p))
        || cu.includes.iter().any(|(p, _)| has_nl(p))
    {
        return Err(
            "a recorded path/define contains a newline — unsupported in the canonical manifest"
                .into(),
        );
    }
    let blob_hash = blake3::hash(blob_bytes);
    cu.blob = format!("units/cu_{}.vu", &hex(blob_hash.as_bytes())[..32]);

    // Supersede CUs sharing any source path; remember the earliest slot.
    let new_paths: std::collections::BTreeSet<&str> =
        cu.sources.iter().map(|(p, _)| p.as_str()).collect();
    let mut slot = None;
    let mut retained = Vec::with_capacity(manifest.cus.len() + 1);
    for (i, old) in manifest.cus.into_iter().enumerate() {
        if old
            .sources
            .iter()
            .any(|(p, _)| new_paths.contains(p.as_str()))
        {
            slot.get_or_insert(retained.len());
            let _ = i;
        } else {
            retained.push(old);
        }
    }
    // Real redefinition check against what remains.
    let new_names: std::collections::BTreeSet<&str> =
        cu.units.iter().map(|(_, n)| n.as_str()).collect();
    for old in &retained {
        for (_, n) in &old.units {
            if new_names.contains(n.as_str()) {
                return Ok(AddOutcome::DupUnit(n.clone()));
            }
        }
    }
    let at = slot.unwrap_or(retained.len());
    retained.insert(at, cu);
    manifest.cus = retained;

    // Write blob (content-addressed: an existing identical file needs no write),
    // then the manifest, both atomically; GC unreferenced blobs best-effort.
    let units_dir = dir.join("units");
    std::fs::create_dir_all(&units_dir).map_err(|e| format!("{}: {e}", units_dir.display()))?;
    let blob_path = dir.join(&manifest.cus[at].blob);
    if !blob_path.exists() {
        write_atomic(&blob_path.to_string_lossy(), blob_bytes)
            .map_err(|e| format!("{}: {e}", blob_path.display()))?;
    }
    let text = manifest.emit();
    write_atomic(&mpath.to_string_lossy(), text.as_bytes())
        .map_err(|e| format!("{}: {e}", mpath.display()))?;
    let live: std::collections::BTreeSet<&str> =
        manifest.cus.iter().map(|c| c.blob.as_str()).collect();
    if let Ok(rd) = std::fs::read_dir(&units_dir) {
        for ent in rd.flatten() {
            let name = format!("units/{}", ent.file_name().to_string_lossy());
            if name.ends_with(".vu") && !live.contains(name.as_str()) {
                let _ = std::fs::remove_file(ent.path());
            }
        }
    }
    Ok(AddOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Manifest {
        Manifest {
            name: "work".into(),
            cus: vec![Cu {
                blob: "units/cu_0011223344556677.vu".into(),
                defines: vec!["W=8".into()],
                incdirs: vec![],
                sources: vec![("rtl/a b\".sv".into(), [0xab; 32])],
                includes: vec![("inc/h.vh".into(), [0x01; 32])],
                units: vec![("module".into(), "top".into())],
            }],
        }
    }

    #[test]
    fn emit_parse_roundtrip_is_identity() {
        let m = sample();
        let text = m.emit();
        let back = Manifest::parse(&text).expect("canonical form parses");
        assert_eq!(m, back);
        // Emit is a fixed point (canonical = deterministic bytes).
        assert_eq!(text, back.emit());
    }

    #[test]
    fn parse_rejects_non_canonical() {
        assert!(Manifest::parse("garbage\n").is_err());
        assert!(Manifest::parse("format_version = 2\n").is_err());
        let mut t = sample().emit();
        t = t.replace("sources = [", "sources= [");
        assert!(Manifest::parse(&t).is_err(), "strict spacing");
    }

    #[test]
    fn digest_entries_preserve_paths_with_spaces() {
        let m = sample();
        let back = Manifest::parse(&m.emit()).unwrap();
        assert_eq!(back.cus[0].sources[0].0, "rtl/a b\".sv");
        assert_eq!(back.cus[0].sources[0].1, [0xab; 32]);
    }
}
