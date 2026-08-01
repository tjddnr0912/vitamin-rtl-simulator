//! Parity sweep for `write_lvalue`'s whole-scalar fast path.
//!
//! The fast path is a shortcut THROUGH `write_lvalue_general`, not a second
//! implementation of it, and this is what makes that claim checkable: every case runs
//! both, on two independently built states, and compares everything a caller or the
//! scheduler can observe afterwards —
//!
//! - the returned `changed`,
//! - the destination net's stored words (`cur.val` / `cur.unk`), all of them,
//! - the dirty channel (`dirty` list and `dirty_flag`),
//! - the accumulated edge mask (`slot_edge`), which is how a clock net wakes processes.
//!
//! Missing any one of those would be silent: a fast path that stores the right bits but
//! forgets `note_change` leaves the value correct and the design frozen.
//!
//! The sweep is over the two axes the fast path actually reasons about — the destination
//! net's WIDTH (which sets the word count and the top-word mask) and the incoming
//! VALUE's width/sign/pattern (which sets what `resize` does before the store, including
//! sign extension and X/Z propagation).

use super::*;
use crate::exec::Offsets;
use sim_ir::{LvalChunk, SelKind};

#[derive(Default)]
struct NullSink;
impl LogSink for NullSink {
    fn emit(&self, _e: LogEvent) {}
}

/// Net widths chosen to straddle every boundary the store loop has: sub-word, exact
/// word, word+1, the two-inline-word limit, and past it into the heap representation.
const NET_WIDTHS: &[u32] = &[1, 2, 7, 8, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 200];

fn source() -> String {
    let mut s = String::from("module t;\n");
    for (i, w) in NET_WIDTHS.iter().enumerate() {
        s.push_str(&format!("  reg [{}:0] u{i};\n", w - 1));
        s.push_str(&format!("  reg signed [{}:0] s{i};\n", w - 1));
    }
    s.push_str("endmodule\n");
    s
}

/// The values swept into each net. Widths straddle the same boundaries; patterns cover
/// all-zero, all-one, alternating, a high-bit-set negative (so sign extension differs
/// from zero extension) and both unknown states.
fn values() -> Vec<Value> {
    let mut out = Vec::new();
    for &w in &[1u32, 4, 8, 32, 64, 65, 100, 128] {
        for &signed in &[false, true] {
            out.push(Value::zeros(w, signed));
            let mut ones = Value::zeros(w, signed);
            for i in 0..w {
                ones.set_vu(i, 1, 0);
            }
            out.push(ones);
            let mut alt = Value::zeros(w, signed);
            for i in (0..w).step_by(2) {
                alt.set_vu(i, 1, 0);
            }
            out.push(alt);
            // high bit only — sign extension vs zero extension diverge here
            let mut hi = Value::zeros(w, signed);
            hi.set_vu(w - 1, 1, 0);
            out.push(hi);
            // X (val 0 / unk 1) and Z (val 1 / unk 1) in the low bits
            let mut xs = Value::zeros(w, signed);
            for i in 0..w.min(3) {
                xs.set_vu(i, 0, 1);
            }
            out.push(xs);
            let mut zs = Value::zeros(w, signed);
            for i in 0..w.min(3) {
                zs.set_vu(i, 1, 1);
            }
            out.push(zs);
        }
    }
    out
}

fn whole_net_lvalue(net: u32) -> Lvalue {
    Lvalue {
        chunks: vec![LvalChunk {
            net,
            word: None,
            offset: None,
            width: None,
            kind: SelKind::Bit,
        }],
    }
}

/// What a caller and the scheduler can see after a write.
#[derive(PartialEq, Eq, Debug)]
struct Observed {
    changed: bool,
    val: Vec<u64>,
    unk: Vec<u64>,
    dirty: Vec<u32>,
    dirty_flag: bool,
    slot_edge: u8,
}

#[test]
fn the_whole_scalar_fast_path_matches_the_general_path() {
    let src = source();
    let (toks, le) = hdl_lexer::lex(&src);
    assert!(le.is_empty(), "lex: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, &src);
    assert!(pe.is_empty(), "parse: {pe:?}");
    let sink = NullSink;
    let ir = elaborate::elaborate(&su.expect("unit"), &sink).expect("elaborate");

    let vals = values();
    let mut cases = 0usize;

    for net in 0..ir.nets.len() as u32 {
        // Only nets the fast path claims; the rest are the general path either way and
        // would make the sweep look bigger than it is.
        let lv = whole_net_lvalue(net);
        for v in &vals {
            for &track_edge in &[false, true] {
                let build = |fast: bool| -> Observed {
                    let mut st = SimState::new(
                        &ir,
                        Box::new(std::io::sink()),
                        &sink,
                        "1ns".to_string(),
                        "test".to_string(),
                        None,
                    );
                    st.build_plain_scalar();
                    st.is_edge_target[net as usize] = track_edge;
                    // Pre-load a non-zero pattern so a same-value store is exercised as
                    // well as a changing one.
                    let nw = st.nets[net as usize].width;
                    let mut pre = Value::zeros(nw.max(1), false);
                    for i in (0..nw).step_by(3) {
                        pre.set_vu(i, 1, 0);
                    }
                    st.store_words(net as usize, 0, nw, &pre);
                    st.dirty.clear();
                    st.dirty_flag.iter_mut().for_each(|f| *f = false);
                    st.slot_edge[net as usize] = 0;

                    let offs = Offsets::Inline {
                        buf: [(0, 0); 2],
                        len: 1,
                    };
                    let changed = if fast {
                        st.write_lvalue(&lv, v.clone(), &offs)
                    } else {
                        st.write_lvalue_general(&lv, v.clone(), &offs)
                    };
                    Observed {
                        changed,
                        val: st.nets[net as usize].cur.val.clone(),
                        unk: st.nets[net as usize].cur.unk.clone(),
                        dirty: st.dirty.clone(),
                        dirty_flag: st.dirty_flag[net as usize],
                        slot_edge: st.slot_edge[net as usize],
                    }
                };
                let fast = build(true);
                let general = build(false);
                assert_eq!(
                    fast, general,
                    "net {net} (width {}), value width {} signed {}, track_edge {track_edge}",
                    ir.nets[net as usize].width, v.width, v.signed,
                );
                cases += 1;
            }
        }
    }
    // Not vacuous: the sweep must actually have run, and the fast path must actually
    // have CLAIMED these nets (otherwise both sides are the general path and the
    // comparison proves nothing).
    assert!(cases > 5000, "sweep collapsed to {cases} cases");
    let mut st = SimState::new(
        &ir,
        Box::new(std::io::sink()),
        &sink,
        "1ns".to_string(),
        "test".to_string(),
        None,
    );
    st.build_plain_scalar();
    assert!(
        st.plain_scalar.iter().filter(|&&p| p).count() >= NET_WIDTHS.len() * 2,
        "the fast path claimed none of the swept nets — the comparison was vacuous"
    );
}

/// A real-valued source into an integer net must NOT take the fast path: the general
/// path rounds it (IEEE 1364-2005 §6.2, half away from zero) and the fast path would
/// store the raw IEEE-754 bits. This is the same class of defect the native-eval type
/// guard closed, one layer down, so it gets its own pin.
#[test]
fn a_real_value_still_rounds_through_the_general_path() {
    let src = "module t; reg [63:0] w; endmodule";
    let (toks, _) = hdl_lexer::lex(src);
    let (su, _) = hdl_parser::parse(&toks, src);
    let sink = NullSink;
    let ir = elaborate::elaborate(&su.expect("unit"), &sink).expect("elaborate");
    let mut st = SimState::new(
        &ir,
        Box::new(std::io::sink()),
        &sink,
        "1ns".to_string(),
        "test".to_string(),
        None,
    );
    st.build_plain_scalar();
    let lv = whole_net_lvalue(0);
    let offs = Offsets::Inline {
        buf: [(0, 0); 2],
        len: 1,
    };
    st.write_lvalue(&lv, Value::from_f64(2.75), &offs);
    assert_eq!(
        st.nets[0].cur.val[0], 3,
        "a real source must round to 3, not deliver its IEEE bit pattern"
    );
}
