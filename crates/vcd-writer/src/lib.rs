//! vcd-writer — IEEE 1364 §18 Value Change Dump serialization.
//!
//! A *passive* [`VcdWriter`]: it does NOT decide when to dump. The simulation
//! engine and `hdl-builtins` dump-task handlers (`$dumpfile`, `$dumpvars`,
//! value changes, `$dumpoff`/`$dumpon`/`$dumpall`/`$dumpflush`, `$dumplimit`)
//! drive it through the methods below.
//!
//! Pipeline a caller follows:
//! 1. header — [`VcdWriter::write_preamble`], [`VcdWriter::push_scope`],
//!    [`VcdWriter::declare_var`], [`VcdWriter::pop_scope`],
//!    [`VcdWriter::write_header`].
//! 2. initial dump — [`VcdWriter::dump_initial`] (the `$dumpvars` block).
//! 3. value changes — [`VcdWriter::set_time`] then [`VcdWriter::value_change`].
//! 4. dump control — [`VcdWriter::dump_off`] / [`VcdWriter::dump_on`] /
//!    [`VcdWriter::dump_all`].
//! 5. [`VcdWriter::flush`] at end-of-file.
//!
//! ## Design choices
//! - **Vector width is kept full** (no leading `0`/`x`/`z` collapse). IEEE 1364
//!   permits the compression but full width keeps the golden output trivially
//!   predictable; a reader expands collapsed forms identically anyway.
//! - **`IdCode`** is base-94 over printable ASCII `!`..=`~` (33..=126); the
//!   first var is `!`, exactly as the format doc specifies.
//! - The writer is generic over [`std::io::Write`], so tests target an in-memory
//!   `Vec<u8>` and production targets a `File` with zero code difference.
//! - Every record method terminates its line with `\n`, so a well-formed stream
//!   ends with a newline by construction (GTKWave issue #336).

use std::collections::HashMap;
use std::io::{self, Write};

use sim_ir::BitPacked;

/// VCD identifier code: a base-94 encoding over printable ASCII `!`..=`~`.
///
/// First var = `!`, then `"`, …, 94th = `~`, 95th = `!!`, etc. A variable keeps
/// its code for the whole file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IdCode(u32);

impl IdCode {
    /// The first id code (`!`).
    pub const FIRST: Self = IdCode(0);

    /// The next id code in allocation order.
    #[must_use]
    pub fn next(self) -> Self {
        IdCode(self.0 + 1)
    }

    /// The raw ordinal (0-based allocation index).
    #[must_use]
    pub fn ordinal(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for IdCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Bijective base-94: each "digit" maps to 33..=126. The leading-digit
        // decrement makes the sequence ! " … ~ !! !" … with no gaps.
        let mut n = u64::from(self.0);
        let mut chars = Vec::new();
        loop {
            chars.push(char::from_u32(33 + (n % 94) as u32).unwrap());
            n /= 94;
            if n == 0 {
                break;
            }
            n -= 1;
        }
        for c in chars.iter().rev() {
            write!(f, "{c}")?;
        }
        Ok(())
    }
}

/// VCD scope type (IEEE 1364 §18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeType {
    Module,
    Task,
    Function,
    Begin,
    Fork,
}

impl ScopeType {
    fn keyword(self) -> &'static str {
        match self {
            ScopeType::Module => "module",
            ScopeType::Task => "task",
            ScopeType::Function => "function",
            ScopeType::Begin => "begin",
            ScopeType::Fork => "fork",
        }
    }
}

/// VCD variable type (IEEE 1364 §18). Types beyond `wire`/`reg` are recorded
/// verbatim in the header but behave identically for value encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarType {
    Event,
    Integer,
    Parameter,
    Real,
    Realtime,
    Reg,
    Supply0,
    Supply1,
    Time,
    Tri,
    Triand,
    Trior,
    Trireg,
    Tri0,
    Tri1,
    Wand,
    Wire,
    Wor,
}

impl VarType {
    fn keyword(self) -> &'static str {
        match self {
            VarType::Event => "event",
            VarType::Integer => "integer",
            VarType::Parameter => "parameter",
            VarType::Real => "real",
            VarType::Realtime => "realtime",
            VarType::Reg => "reg",
            VarType::Supply0 => "supply0",
            VarType::Supply1 => "supply1",
            VarType::Time => "time",
            VarType::Tri => "tri",
            VarType::Triand => "triand",
            VarType::Trior => "trior",
            VarType::Trireg => "trireg",
            VarType::Tri0 => "tri0",
            VarType::Tri1 => "tri1",
            VarType::Wand => "wand",
            VarType::Wire => "wire",
            VarType::Wor => "wor",
        }
    }
}

/// A counting wrapper so `$dumplimit` can observe the byte total.
struct Counting<W: Write> {
    inner: W,
    bytes: u64,
}

impl<W: Write> Write for Counting<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.bytes += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// One declared variable's metadata, retained for dump-off replays.
#[derive(Clone, Copy)]
struct VarMeta {
    id: IdCode,
    width: u32,
    /// `real`/`realtime` vars emit `r<%.16g> <id>` value changes (IEEE 1364
    /// §18.x), not the 4-state `b…`/scalar form.
    is_real: bool,
}

/// vitamin VCD generator — one instance per simulation.
///
/// Generic over the sink: `VcdWriter<Vec<u8>>` for tests, `VcdWriter<File>` for
/// production.
pub struct VcdWriter<W: Write> {
    out: Counting<W>,
    next_id: IdCode,
    /// declared variables in declaration order (for `$dumpoff` replay).
    vars: Vec<VarMeta>,
    by_id: HashMap<IdCode, usize>,
    scope_depth: u32,
    current_time: u64,
    time_written: bool,
    /// `false` between `$dumpoff` and `$dumpon`.
    dumping: bool,
    byte_limit: Option<u64>,
    limit_hit: bool,
    /// VCD-SCRATCH: reused per-change encode buffer. Each value change builds its
    /// record bytes here (cleared, capacity retained) instead of allocating a
    /// fresh `String` per change — the dump hot path no longer allocates.
    scratch: Vec<u8>,
}

impl<W: Write> VcdWriter<W> {
    /// Create a writer over `sink`.
    pub fn new(sink: W) -> Self {
        VcdWriter {
            out: Counting {
                inner: sink,
                bytes: 0,
            },
            next_id: IdCode::FIRST,
            vars: Vec::new(),
            by_id: HashMap::new(),
            scope_depth: 0,
            current_time: 0,
            time_written: false,
            dumping: true,
            byte_limit: None,
            limit_hit: false,
            scratch: Vec::new(),
        }
    }

    /// Total bytes written so far.
    #[must_use]
    pub fn bytes_written(&self) -> u64 {
        self.out.bytes
    }

    /// Set the `$dumplimit` byte budget. Once exceeded, a one-time
    /// `$comment Dump limit reached $end` is emitted and further records drop.
    pub fn set_limit(&mut self, bytes: u64) {
        self.byte_limit = Some(bytes);
    }

    /// Whether the byte limit has been reached.
    #[must_use]
    pub fn is_limit_reached(&self) -> bool {
        self.limit_hit
    }

    /// The declared width recorded for an id, if known.
    #[must_use]
    pub fn width_of(&self, id: IdCode) -> Option<u32> {
        self.by_id.get(&id).map(|&i| self.vars[i].width)
    }

    // ── limit gate ────────────────────────────────────────────────────────────

    /// Returns `true` if the record should be suppressed (limit reached).
    /// Emits the limit comment exactly once on the transition.
    fn check_limit(&mut self) -> io::Result<bool> {
        if self.limit_hit {
            return Ok(true);
        }
        if let Some(limit) = self.byte_limit {
            if self.out.bytes >= limit {
                self.limit_hit = true;
                writeln!(self.out, "$comment Dump limit reached $end")?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    // ── header ────────────────────────────────────────────────────────────────

    /// Write the fixed header preamble: `$date`, `$version` (vitamin-sim crate
    /// version), `$comment`, `$timescale`. `unit` is e.g. `"1ns"`, `"10ps"`.
    ///
    /// `date` is taken verbatim (the caller decides whether to stamp wall-clock
    /// time); this keeps the writer deterministic and testable.
    pub fn write_preamble(&mut self, date: &str, unit: &str) -> io::Result<()> {
        writeln!(self.out, "$date\n   {date}\n$end")?;
        writeln!(
            self.out,
            "$version\n   vitamin-sim {}\n$end",
            env!("CARGO_PKG_VERSION")
        )?;
        writeln!(
            self.out,
            "$comment\n   Generated by vitamin RTL simulator\n$end"
        )?;
        writeln!(self.out, "$timescale {unit} $end")?;
        Ok(())
    }

    /// Open a hierarchy level: `$scope <type> <name> $end`.
    pub fn push_scope(&mut self, scope_type: ScopeType, name: &str) -> io::Result<()> {
        writeln!(self.out, "$scope {} {name} $end", scope_type.keyword())?;
        self.scope_depth += 1;
        Ok(())
    }

    /// Close the current hierarchy level: `$upscope $end`.
    pub fn pop_scope(&mut self) -> io::Result<()> {
        debug_assert!(
            self.scope_depth > 0,
            "pop_scope without matching push_scope"
        );
        writeln!(self.out, "$upscope $end")?;
        self.scope_depth = self.scope_depth.saturating_sub(1);
        Ok(())
    }

    /// Declare a variable: `$var <type> <width> <idcode> <reference> $end`.
    /// Allocates and returns a fresh [`IdCode`] that is stable for the file.
    pub fn declare_var(
        &mut self,
        var_type: VarType,
        width: u32,
        reference: &str,
    ) -> io::Result<IdCode> {
        let id = self.next_id;
        self.next_id = self.next_id.next();
        let idx = self.vars.len();
        let is_real = matches!(var_type, VarType::Real | VarType::Realtime);
        self.vars.push(VarMeta { id, width, is_real });
        self.by_id.insert(id, idx);
        writeln!(
            self.out,
            "$var {} {width} {id} {reference} $end",
            var_type.keyword()
        )?;
        Ok(id)
    }

    /// Close the definitions section: `$enddefinitions $end`.
    pub fn write_header(&mut self) -> io::Result<()> {
        writeln!(self.out, "$enddefinitions $end")?;
        Ok(())
    }

    /// Write a free-form `$comment` block.
    pub fn write_comment(&mut self, text: &str) -> io::Result<()> {
        writeln!(self.out, "$comment\n   {text}\n$end")?;
        Ok(())
    }

    // ── value encoding ──────────────────────────────────────────────────────────

    /// Map bit `i` of a [`BitPacked`] to its VCD char.
    /// `(v=0,u=0)→'0'`, `(v=1,u=0)→'1'`, `(v=0,u=1)→'x'`, `(v=1,u=1)→'z'`.
    /// word0 bit0 = LSB.
    fn bit_char(bits: &BitPacked, i: u32) -> char {
        let word = (i / 64) as usize;
        let shift = i % 64;
        let v = bits.val.get(word).map_or(0, |w| (w >> shift) & 1);
        let u = bits.unk.get(word).map_or(0, |w| (w >> shift) & 1);
        match (v, u) {
            (0, 0) => '0',
            (1, 0) => '1',
            (0, 1) => 'x',
            (1, 1) => 'z',
            _ => unreachable!(),
        }
    }

    /// Whether `id` was declared as a `real`/`realtime` var.
    fn is_real(&self, id: IdCode) -> bool {
        self.by_id.get(&id).is_some_and(|&i| self.vars[i].is_real)
    }

    /// Append a `real` value change `r<%.16g> <id>` (no space after `r`, space
    /// before the id), e.g. `r3.14159 !`, to `buf`. The 64-bit `bits` carry the
    /// IEEE-754 pattern in word 0 (vitamin stores reals as raw bits).
    fn encode_real_into(buf: &mut Vec<u8>, id: IdCode, bits: &BitPacked) {
        let x = f64::from_bits(bits.val.first().copied().unwrap_or(0));
        // `Vec<u8>` as `io::Write` is infallible.
        let _ = write!(buf, "r{} {id}", fmt_g(x, 16));
    }

    /// Append a single integral change to `buf` (no trailing newline): scalar
    /// `1!`, vector `b1010 !` (MSB..LSB, space before id).
    fn encode_value_into(buf: &mut Vec<u8>, id: IdCode, bits: &BitPacked, width: u32) {
        if width <= 1 {
            // scalar: <char><id>, NO space.
            buf.push(Self::bit_char(bits, 0) as u8);
            let _ = write!(buf, "{id}");
        } else {
            // vector: b<MSB..LSB> <id>, space before id.
            buf.push(b'b');
            for i in (0..width).rev() {
                buf.push(Self::bit_char(bits, i) as u8);
            }
            buf.push(b' ');
            let _ = write!(buf, "{id}");
        }
    }

    /// Append a real (`r…`) or integral change to `buf`. `real` is precomputed by
    /// the caller so this needs no `&self`, letting the hot path borrow
    /// `self.scratch` mutably alongside an immutable `is_real` lookup.
    fn encode_change_into(buf: &mut Vec<u8>, real: bool, id: IdCode, bits: &BitPacked, width: u32) {
        if real {
            Self::encode_real_into(buf, id, bits);
        } else {
            Self::encode_value_into(buf, id, bits, width);
        }
    }

    /// The value string for a single integral change (no trailing newline).
    /// Thin `String` wrapper over [`Self::encode_value_into`] — the byte writer
    /// is the single source of truth, so tests and the hot path can't diverge.
    #[cfg(test)]
    fn encode_value(id: IdCode, bits: &BitPacked, width: u32) -> String {
        let mut buf = Vec::new();
        Self::encode_value_into(&mut buf, id, bits, width);
        String::from_utf8(buf).expect("value encoding is ASCII")
    }

    /// Build one change record into the reused `scratch` buffer (no per-change
    /// `String` allocation) and write `<value>\n` to `out`.
    fn write_change(&mut self, id: IdCode, bits: &BitPacked, width: u32) -> io::Result<()> {
        let real = self.is_real(id);
        self.scratch.clear();
        Self::encode_change_into(&mut self.scratch, real, id, bits, width);
        self.scratch.push(b'\n');
        self.out.write_all(&self.scratch)
    }

    /// Emit a value-change record `<value>\n` for `id`.
    /// Suppressed while dumping is off or once the limit is hit.
    pub fn value_change(&mut self, id: IdCode, bits: &BitPacked, width: u32) -> io::Result<()> {
        if !self.dumping || self.check_limit()? {
            return Ok(());
        }
        self.write_change(id, bits, width)
    }

    // ── time + dump ─────────────────────────────────────────────────────────────

    /// Advance simulation time, emitting `#<t>` once per distinct timestamp.
    /// The first value change must occur under `#0` (per the format doc); call
    /// `set_time(0)` before the first post-dumpvars change.
    pub fn set_time(&mut self, t: u64) -> io::Result<()> {
        if self.time_written && t == self.current_time {
            return Ok(());
        }
        if self.check_limit()? {
            self.current_time = t;
            self.time_written = true;
            return Ok(());
        }
        writeln!(self.out, "#{t}")?;
        self.current_time = t;
        self.time_written = true;
        Ok(())
    }

    /// Write the `$dumpvars` initial-values block (no `#time` prefix).
    /// `vars` is the full set of tracked `(id, bits, width)` at time 0.
    pub fn dump_initial<'a, I>(&mut self, vars: I) -> io::Result<()>
    where
        I: IntoIterator<Item = (IdCode, &'a BitPacked, u32)>,
    {
        writeln!(self.out, "$dumpvars")?;
        for (id, bits, width) in vars {
            self.write_change(id, bits, width)?;
        }
        writeln!(self.out, "$end")?;
        Ok(())
    }

    /// `$dumpall`: force-write every tracked variable's current value as a
    /// checkpoint at the current time. `vars` must cover every declared id.
    pub fn dump_all<'a, I>(&mut self, vars: I) -> io::Result<()>
    where
        I: IntoIterator<Item = (IdCode, &'a BitPacked, u32)>,
    {
        if self.check_limit()? {
            return Ok(());
        }
        writeln!(self.out, "$dumpall")?;
        for (id, bits, width) in vars {
            self.write_change(id, bits, width)?;
        }
        writeln!(self.out, "$end")?;
        Ok(())
    }

    /// `$dumpoff`: write every tracked variable as `x`/`bx…`, then stop dumping
    /// until [`dump_on`](Self::dump_on). Widths come from the declared table, so
    /// no value snapshot is needed.
    pub fn dump_off(&mut self) -> io::Result<()> {
        if self.check_limit()? {
            self.dumping = false;
            return Ok(());
        }
        writeln!(self.out, "$dumpoff")?;
        let metas: Vec<VarMeta> = self.vars.clone();
        for m in metas {
            if m.width <= 1 {
                writeln!(self.out, "x{}", m.id)?;
            } else {
                let xs: String = "x".repeat(m.width as usize);
                writeln!(self.out, "b{xs} {}", m.id)?;
            }
        }
        writeln!(self.out, "$end")?;
        self.dumping = false;
        Ok(())
    }

    /// `$dumpon`: resume dumping and re-emit every tracked variable's current
    /// value. `vars` supplies the live values at resume time.
    pub fn dump_on<'a, I>(&mut self, vars: I) -> io::Result<()>
    where
        I: IntoIterator<Item = (IdCode, &'a BitPacked, u32)>,
    {
        self.dumping = true;
        if self.check_limit()? {
            return Ok(());
        }
        writeln!(self.out, "$dumpon")?;
        for (id, bits, width) in vars {
            self.write_change(id, bits, width)?;
        }
        writeln!(self.out, "$end")?;
        Ok(())
    }

    /// Flush the sink. Every record method already ends its line with `\n`, so a
    /// well-formed stream terminates with a newline (GTKWave issue #336); this
    /// does not append a spurious blank line.
    pub fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

impl VcdWriter<Vec<u8>> {
    /// Convenience for tests: take the in-memory buffer as a UTF-8 `String`.
    #[must_use]
    pub fn into_string(self) -> String {
        String::from_utf8(self.out.inner).expect("VCD output is ASCII/UTF-8")
    }
}

/// C `printf %.{p}g` of a finite f64: `p` significant digits, `%e`/`%f` chosen by
/// the decimal exponent, trailing zeros stripped. The SINGLE deterministic `%g`
/// shared (REALG-DEDUP) by the VCD real value-change format (`p = 16`) and
/// `$display %g` (`prec.unwrap_or(6).max(1)`). Deterministic across OSes — uses
/// only Rust's `{:e}`/`{:.*}`, never a libm transcendental.
pub fn fmt_g(x: f64, p: i32) -> String {
    if !x.is_finite() {
        // C / iverilog spell NaN lowercase ("nan", any sign); ±inf already match
        // Rust's "inf"/"-inf". Rust's default Display gives "NaN" — a silent-wrong
        // for `$display` parity until coerced here.
        return if x.is_nan() {
            "nan".to_string()
        } else {
            format!("{x}") // "inf" / "-inf"
        };
    }
    if x == 0.0 {
        return "0".to_string();
    }
    let p = p.max(1);
    let sci = format!("{:.*e}", (p - 1) as usize, x); // p-1 fractional → p sig digits
    let exp: i32 = sci
        .split_once('e')
        .and_then(|(_, e)| e.parse().ok())
        .unwrap_or(0);
    if exp < -4 || exp >= p {
        let (mant, e) = sci.split_once('e').unwrap();
        let mant = strip_trailing_zeros(mant);
        let (sgn, dig) = match e.strip_prefix('-') {
            Some(d) => ('-', d),
            None => ('+', e),
        };
        let dig = if dig.len() < 2 {
            format!("{dig:0>2}")
        } else {
            dig.to_string()
        };
        format!("{mant}e{sgn}{dig}")
    } else {
        let prec = (p - 1 - exp).max(0) as usize;
        strip_trailing_zeros(&format!("{x:.prec$}"))
    }
}

/// Strip insignificant trailing zeros after a decimal point (and a bare `.`).
fn strip_trailing_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bp(val: u64, unk: u64) -> BitPacked {
        BitPacked {
            val: vec![val],
            unk: vec![unk],
        }
    }

    #[test]
    fn idcode_sequence() {
        // first var = '!', then '"', up to 94th = '~', 95th = '!!'.
        let mut id = IdCode::FIRST;
        assert_eq!(id.to_string(), "!");
        id = id.next();
        assert_eq!(id.to_string(), "\"");
        // 94th (ordinal 93) is '~'.
        let mut id93 = IdCode::FIRST;
        for _ in 0..93 {
            id93 = id93.next();
        }
        assert_eq!(id93.to_string(), "~");
        // 95th (ordinal 94) is '!!'.
        assert_eq!(id93.next().to_string(), "!!");
    }

    #[test]
    fn four_state_mapping() {
        // bit0 of each: 0,1,x,z.
        assert_eq!(VcdWriter::<Vec<u8>>::bit_char(&bp(0, 0), 0), '0');
        assert_eq!(VcdWriter::<Vec<u8>>::bit_char(&bp(1, 0), 0), '1');
        assert_eq!(VcdWriter::<Vec<u8>>::bit_char(&bp(0, 1), 0), 'x');
        assert_eq!(VcdWriter::<Vec<u8>>::bit_char(&bp(1, 1), 0), 'z');
    }

    #[test]
    fn scalar_has_no_space_vector_has_space() {
        let id = IdCode::FIRST;
        // scalar
        assert_eq!(VcdWriter::<Vec<u8>>::encode_value(id, &bp(1, 0), 1), "1!");
        assert_eq!(VcdWriter::<Vec<u8>>::encode_value(id, &bp(0, 1), 1), "x!");
        // vector MSB..LSB. value=0b0101 (bit0=1,bit2=1) width 4 -> "0101"
        assert_eq!(
            VcdWriter::<Vec<u8>>::encode_value(id, &bp(0b0101, 0), 4),
            "b0101 !"
        );
        // vector with x and z bits: bit0 x (v0,u1), bit1 z (v1,u1) width 2
        //  -> MSB(bit1)=z LSB(bit0)=x -> "bzx"
        assert_eq!(
            VcdWriter::<Vec<u8>>::encode_value(id, &bp(0b10, 0b11), 2),
            "bzx !"
        );
    }

    /// VCD-SCRATCH (2026-06-22 audit): pin the EXACT bytes `value_change` emits
    /// for scalar + multi-id vector changes, so the scratch-buffer refactor
    /// (no per-change String alloc) stays byte-identical. Characterization test:
    /// green before AND after the refactor.
    #[test]
    fn value_change_byte_exact_scalar_and_vector() {
        let mut w = VcdWriter::new(Vec::new());
        w.write_preamble("d", "1ns").unwrap();
        w.push_scope(ScopeType::Module, "t").unwrap();
        let a = w.declare_var(VarType::Wire, 1, "a").unwrap(); // scalar, id "!"
        let b = w.declare_var(VarType::Wire, 4, "b").unwrap(); // vector, id """
        w.pop_scope().unwrap();
        w.write_header().unwrap();
        w.set_time(0).unwrap();
        w.value_change(a, &bp(1, 0), 1).unwrap(); // "1!\n"
        w.value_change(b, &bp(0b0101, 0), 4).unwrap(); // "b0101 \"\n"
        let s = w.into_string();
        let tail = &s[s.find("#0").unwrap()..];
        assert_eq!(
            tail,
            format!("#0\n1{a}\nb0101 {b}\n"),
            "value_change byte stream must be exact"
        );
    }

    #[test]
    fn real_var_emits_r_format() {
        // SW4 (2026-06-22 audit): a `real`/`realtime` var's value change must be
        // `r<%.16g> <id>` (spec 07-vcd-format.md:164-169), NOT a 64-bit binary
        // `b...` vector of the IEEE-754 bit pattern (silent waveform corruption in
        // GTKWave/Surfer). The initial $dumpvars value must use r-format too.
        let mut w = VcdWriter::new(Vec::new());
        w.write_preamble("d", "1ns").unwrap();
        w.push_scope(ScopeType::Module, "t").unwrap();
        let r = w.declare_var(VarType::Real, 64, "r").unwrap();
        w.pop_scope().unwrap();
        w.write_header().unwrap();
        let zero = bp(0.0_f64.to_bits(), 0);
        w.dump_initial(std::iter::once((r, &zero, 64))).unwrap();
        // 0.1 is NOT exactly representable → exercises %.16g rounding + zero-strip.
        let bits = bp(0.1_f64.to_bits(), 0);
        w.set_time(1).unwrap();
        w.value_change(r, &bits, 64).unwrap();
        let neg = bp((-2.5_f64).to_bits(), 0);
        w.value_change(r, &neg, 64).unwrap();
        let s = w.into_string();
        assert!(s.contains(&format!("r0 {r}")), "initial r0 missing:\n{s}");
        assert!(s.contains(&format!("r0.1 {r}")), "got:\n{s}");
        assert!(s.contains(&format!("r-2.5 {r}")), "got:\n{s}");
        assert!(
            !s.contains("b010") && !s.contains("b110") && !s.contains("b00000"),
            "real must not emit a binary vector:\n{s}"
        );
    }

    #[test]
    fn golden_vcd() {
        let mut w = VcdWriter::new(Vec::new());
        w.write_preamble("2026-05-28 09:00:00", "1ns").unwrap();
        w.push_scope(ScopeType::Module, "tb").unwrap();
        let data = w.declare_var(VarType::Wire, 8, "data").unwrap();
        let valid = w.declare_var(VarType::Wire, 1, "data_valid").unwrap();
        let state = w.declare_var(VarType::Reg, 4, "state").unwrap();
        w.pop_scope().unwrap();
        w.write_header().unwrap();

        // initial dump: data=bxxxxxxxx, valid=x, state=bxxxx (all unknown)
        let x8 = BitPacked {
            val: vec![0],
            unk: vec![0xFF],
        };
        let x1 = bp(0, 1);
        let x4 = BitPacked {
            val: vec![0],
            unk: vec![0xF],
        };
        w.dump_initial([(data, &x8, 8), (valid, &x1, 1), (state, &x4, 4)])
            .unwrap();

        // #0: data=0, valid=1, state=0
        w.set_time(0).unwrap();
        w.value_change(data, &bp(0, 0), 8).unwrap();
        w.value_change(valid, &bp(1, 0), 1).unwrap();
        w.value_change(state, &bp(0, 0), 4).unwrap();

        // #10: data=0x81, valid=x, state bit0=z -> "000z"
        w.set_time(10).unwrap();
        w.value_change(data, &bp(0x81, 0), 8).unwrap();
        w.value_change(valid, &bp(0, 1), 1).unwrap(); // x
        w.value_change(state, &bp(0b0001, 0b0001), 4).unwrap(); // bit0 z

        w.flush().unwrap();
        let got = w.into_string();

        let expected = "\
$date
   2026-05-28 09:00:00
$end
$version
   vitamin-sim 0.0.0
$end
$comment
   Generated by vitamin RTL simulator
$end
$timescale 1ns $end
$scope module tb $end
$var wire 8 ! data $end
$var wire 1 \" data_valid $end
$var reg 4 # state $end
$upscope $end
$enddefinitions $end
$dumpvars
bxxxxxxxx !
x\"
bxxxx #
$end
#0
b00000000 !
1\"
b0000 #
#10
b10000001 !
x\"
b000z #
";
        assert_eq!(got, expected);
        // trailing newline present (GTKWave #336)
        assert!(got.ends_with('\n'));
    }

    #[test]
    fn dumpoff_writes_x_then_suppresses_dumpon_resumes() {
        let mut w = VcdWriter::new(Vec::new());
        w.write_preamble("d", "1ns").unwrap();
        w.push_scope(ScopeType::Module, "t").unwrap();
        let a = w.declare_var(VarType::Wire, 1, "a").unwrap();
        let b = w.declare_var(VarType::Wire, 4, "b").unwrap();
        w.pop_scope().unwrap();
        w.write_header().unwrap();
        w.set_time(0).unwrap();
        w.value_change(a, &bp(1, 0), 1).unwrap();

        // id codes: a -> '!', b -> '"'.
        let ia = a.to_string();
        let ib = b.to_string();
        assert_eq!(ia, "!");
        assert_eq!(ib, "\"");

        w.dump_off().unwrap();
        // suppressed while off
        w.value_change(a, &bp(0, 0), 1).unwrap();
        w.dump_on([(a, &bp(1, 0), 1), (b, &bp(0b1010, 0), 4)])
            .unwrap();
        w.value_change(b, &bp(0b1111, 0), 4).unwrap();
        w.flush().unwrap();
        let got = w.into_string();

        assert!(got.contains(&format!("$dumpoff\nx{ia}\nbxxxx {ib}\n$end\n")));
        // the suppressed change `0!` must NOT appear between dumpoff and dumpon
        let off = got.find("$dumpoff").unwrap();
        let on = got.find("$dumpon").unwrap();
        assert!(!got[off..on].contains(&format!("0{ia}")));
        assert!(got.contains(&format!("$dumpon\n1{ia}\nb1010 {ib}\n$end\n")));
        assert!(got.contains(&format!("b1111 {ib}\n")));
    }

    #[test]
    fn dumpall_checkpoint() {
        let mut w = VcdWriter::new(Vec::new());
        w.write_preamble("d", "1ns").unwrap();
        w.push_scope(ScopeType::Module, "t").unwrap();
        let a = w.declare_var(VarType::Wire, 1, "a").unwrap();
        let b = w.declare_var(VarType::Reg, 4, "b").unwrap();
        w.pop_scope().unwrap();
        w.write_header().unwrap();
        w.set_time(5).unwrap();
        w.dump_all([(a, &bp(1, 0), 1), (b, &bp(0b0011, 0), 4)])
            .unwrap();
        w.flush().unwrap();
        let got = w.into_string();
        // id codes: a -> '!', b -> '"'.
        assert!(got.contains(&format!("#5\n$dumpall\n1{}\nb0011 {}\n$end\n", a, b)));
    }

    #[test]
    fn dump_limit_inserts_comment_and_suppresses() {
        let mut w = VcdWriter::new(Vec::new());
        w.write_preamble("d", "1ns").unwrap();
        w.push_scope(ScopeType::Module, "t").unwrap();
        let a = w.declare_var(VarType::Wire, 1, "a").unwrap();
        w.pop_scope().unwrap();
        w.write_header().unwrap();
        w.set_limit(1); // already exceeded by header
        w.set_time(0).unwrap();
        w.value_change(a, &bp(1, 0), 1).unwrap();
        w.flush().unwrap();
        assert!(w.is_limit_reached());
        let got = w.into_string();
        assert!(got.contains("$comment Dump limit reached $end"));
        // value change `1!` suppressed (and no `#0` either, both past the limit)
        assert!(!got.contains("1!"));
        assert!(!got.contains("#0"));
    }

    #[test]
    fn set_time_dedups_same_timestamp() {
        let mut w = VcdWriter::new(Vec::new());
        w.set_time(0).unwrap();
        w.set_time(0).unwrap();
        w.set_time(10).unwrap();
        w.flush().unwrap();
        assert_eq!(w.into_string(), "#0\n#10\n");
    }

    #[test]
    fn multiword_vector() {
        // 80-bit vector: bit79 set, bit0 set.
        let bits = BitPacked {
            val: vec![1u64, 1u64 << 15],
            unk: vec![0, 0],
        };
        let id = IdCode::FIRST;
        let s = VcdWriter::<Vec<u8>>::encode_value(id, &bits, 80);
        // MSB (bit79) = 1; LSB (bit0) = 1; everything between is 0.
        assert!(s.starts_with("b1"));
        assert!(s.ends_with("1 !"));
        // total length: 'b' + 80 chars + ' ' + '!' = 83
        assert_eq!(s.len(), 1 + 80 + 1 + 1);
    }

    // REALG-DEDUP: an adversarial determinism table for the SINGLE shared `fmt_g`
    // (the VCD `%g` has no external oracle, so this golden table is the oracle
    // surrogate). Exercises both the VCD precision (p=16) and `$display` default
    // (p=6), incl. the exponent-form boundary (`exp >= p`), subnormal-ish small
    // values, and the non-finite / signed-zero specials.
    #[test]
    fn fmt_g_determinism_golden() {
        let cases: &[(f64, i32, &str)] = &[
            (9.9999e5, 6, "999990"),
            (999999.0, 6, "999999"),
            (1234567.0, 6, "1.23457e+06"), // rounds up into exp form (exp 6 >= p 6)
            (123456.0, 6, "123456"),
            (1500.0, 6, "1500"),
            (1.0, 6, "1"),
            (0.0001, 6, "0.0001"),
            (0.00001, 6, "1e-05"), // exp -5 < -4 → exp form
            (6.022e23, 16, "6.022e+23"),
            (1.0e16, 16, "1e+16"), // exp 16 >= p 16 → exp form
            (0.1, 16, "0.1"),
            (2.5, 16, "2.5"),
            (-0.0, 6, "0"),
            (f64::INFINITY, 6, "inf"),
            (f64::NEG_INFINITY, 6, "-inf"),
            (f64::NAN, 6, "nan"), // C/iverilog lowercase (was "NaN", a $display silent-wrong)
        ];
        for &(x, p, want) in cases {
            assert_eq!(fmt_g(x, p), want, "fmt_g({x:e}, {p})");
        }
    }
}
