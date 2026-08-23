//! The manifest.
//!
//! Deliberately a Rust table rather than a data file: the workspace builds with
//! `--locked` for 3-OS reproducibility, and a TOML/JSON manifest would mean a new
//! dependency to parse a file that changes a few times a year. A `const` table costs
//! nothing, and the compiler checks it.

/// Where the RTL comes from, and what we are allowed to do with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Written in this repository and committed. Reproducible without a network.
    FirstParty,
    /// Cloned from upstream at a pinned commit. Never redistributed.
    Upstream {
        repo: &'static str,
        /// Full 40-char SHA. A tag or branch would let the workload drift under us,
        /// which would silently invalidate every number measured against it.
        sha: &'static str,
        license: &'static str,
    },
}

/// What the design stresses. The corpus is only useful if these differ — four CPUs
/// measure one shape four times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Instruction fetch/decode/execute: branchy control, a register file, a bus.
    Cpu,
    /// Wide fixed datapath, few branches, heavy bit manipulation.
    Crypto,
    /// Streaming/handshake pipelines — many small always blocks, high event churn.
    Stream,
    /// Parameterised interconnect: elaboration-heavy, generate-heavy.
    Fabric,
}

impl Shape {
    pub fn label(self) -> &'static str {
        match self {
            Shape::Cpu => "cpu",
            Shape::Crypto => "crypto",
            Shape::Stream => "stream",
            Shape::Fabric => "fabric",
        }
    }
}

/// What vita is currently expected to do with this design.
///
/// A corpus containing only designs vita already runs can never tell anyone
/// anything. The refusals are the point: `Refused` records the diagnostic vita emits
/// today, so the harness can distinguish three cases that look identical from the
/// outside — a *known* gap (not a failure, and the count is the coverage metric), a
/// *new* refusal on a design that used to run (a regression, red), and a design that
/// started running (a slice landed; the tool says so and asks for the row to move).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    /// Runs to completion and prints [`Workload::digest`], exiting with `exit`.
    ///
    /// The exit code lives HERE rather than beside it as a free field, because a
    /// free field can be set on a refused row — and it was. Pinning `exit: 1` on a
    /// refused workload (which is what vita returns *while refusing*) made the
    /// promotion it was waiting for impossible to observe: the day the gap closed,
    /// vita would exit 0, miss the equality test, and grade as
    /// "was loud, now crashes". The single event the corpus exists to detect was
    /// wired to report itself as a regression. Making the code reachable only from
    /// `Runs` removes the shape of that mistake.
    Runs { exit: i32 },
    /// Declines at elaborate. `diag` is a distinctive fragment of the message —
    /// matched as a substring so rewording a diagnostic does not break the gate,
    /// which is the same reason the compliance corpus asserts on message codes.
    Refused { diag: &'static str },
}

/// One measurable design.
#[derive(Debug, Clone, Copy)]
pub struct Workload {
    pub name: &'static str,
    pub origin: Origin,
    pub shape: Shape,
    /// The workload's own directory under `bench/`. The upstream clone lands in
    /// `bench/<root>/src`, and the testbench written for the bench sits beside it.
    pub root: &'static str,
    /// The working directory the simulators run in, under `bench/`. Usually the same
    /// as [`Workload::root`] — but `darkriscv` reads `../src/darksocv.mem` relative
    /// to the CWD from inside upstream's own `sim/` directory, so the two differ and
    /// pretending otherwise silently loads an empty memory.
    pub dir: &'static str,
    /// Flags that must precede the sources. They differ per tool because the two
    /// spell the same thing differently (`--top` vs `-s`), and because a workload
    /// that needs `-D`/`-I` needs it identically on both sides or the comparison is
    /// between two different designs.
    pub vita_args: &'static [&'static str],
    pub iverilog_args: &'static [&'static str],
    /// Sources in the order the simulators must see them, relative to `dir`.
    pub files: &'static [&'static str],
    /// Files the workload READS at run time — firmware images, memory contents —
    /// relative to `dir`. Listed separately from `files` because the simulators are
    /// never told about them, but their absence is just as fatal, and a workload
    /// whose image is missing must report `absent` rather than blame vita for the
    /// `$readmemh` that fails.
    pub data: &'static [&'static str],
    /// Plusargs that set the workload size.
    pub plusargs: &'static [&'static str],
    /// The digest line an oracle produced, verbatim. `run` compares against this,
    /// which is what lets a machine with no simulator installed still gate. Present
    /// even for a [`Expect::Refused`] workload: the oracle answered, and pinning what
    /// it said is what makes the eventual promotion checkable.
    pub digest: &'static str,
    /// What vita does with it today.
    pub expect: Expect,
    /// Which oracle produced `digest`, and at what version.
    pub oracle: &'static str,
    pub note: &'static str,
}

impl Workload {
    pub fn is_first_party(&self) -> bool {
        matches!(self.origin, Origin::FirstParty)
    }

    pub fn is_refused(&self) -> bool {
        matches!(self.expect, Expect::Refused { .. })
    }
}

/// How much of the corpus vita runs today. This is the number the corpus exists to
/// move, and it is deliberately reported as a fraction of the whole corpus rather
/// than of the designs that happen to work.
pub fn coverage() -> (usize, usize) {
    (
        CORPUS.iter().filter(|w| !w.is_refused()).count(),
        CORPUS.len(),
    )
}

/// The corpus. Ordered small-to-large so a filtered run reaches a verdict quickly.
///
/// The `note` fields deliberately carry NO timings. Numbers written in two places
/// rot in one of them, and they already did: an earlier draft of this table quoted a
/// different measurement run than `docs/study/03-workload-corpus.md` and the two
/// disagreed in the third digit on six of seven rows. Timings live in study/03;
/// `corpus-runner run --compare` reproduces them.
pub static CORPUS: &[Workload] = &[
    Workload {
        name: "sha256",
        origin: Origin::Upstream {
            repo: "https://github.com/secworks/sha256",
            sha: "837c5cc396f001d18f2c765721c585716eb439ae",
            license: "BSD-2-Clause",
        },
        shape: Shape::Crypto,
        root: "sha256",
        dir: "sha256",
        vita_args: &[],
        iverilog_args: &[],
        files: &[
            "tb.v",
            "src/src/rtl/sha256_core.v",
            "src/src/rtl/sha256_w_mem.v",
            "src/src/rtl/sha256_k_constants.v",
        ],
        data: &[],
        plusargs: &["+N=2000"],
        digest: "DIGEST=e75e29e81cff3c66de9e0f419baa516ea08e6414fa1f9f62a757538288351724",
        expect: Expect::Runs { exit: 0 },
        oracle: "iverilog 13.0 (verilator 5.050 agrees)",
        note: "the corpus's widest vita margin",
    },
    Workload {
        name: "aes",
        origin: Origin::Upstream {
            repo: "https://github.com/secworks/aes",
            sha: "80dc4718e1dcbbdb4b0dd1bdb393d8f7b98981dc",
            license: "BSD-2-Clause",
        },
        shape: Shape::Crypto,
        root: "aes",
        dir: "aes",
        vita_args: &[],
        iverilog_args: &[],
        files: &[
            "src/src/rtl/aes_core.v",
            "src/src/rtl/aes_encipher_block.v",
            "src/src/rtl/aes_decipher_block.v",
            "src/src/rtl/aes_key_mem.v",
            "src/src/rtl/aes_sbox.v",
            "src/src/rtl/aes_inv_sbox.v",
            "tb.v",
        ],
        data: &[],
        plusargs: &["+N=200"],
        digest: "DIGEST=cfaa46dd896b2275ade662d344f5e251",
        // Correct digest, and STILL exit 1: vita reports the out-of-range array read
        // in `aes_key_mem.v` as an error, where IEEE 1364-2005 §5.2.1 defines the
        // behaviour (read x, write ignored) and both oracles stay silent. Pinning the
        // code keeps that visible instead of grading the workload as a crash.
        expect: Expect::Runs { exit: 1 },
        oracle: "iverilog 13.0 (verilator 5.050 agrees)",
        note: "exits 1 on an IEEE-defined out-of-range read; digest still correct",
    },
    Workload {
        name: "picorv32",
        origin: Origin::Upstream {
            repo: "https://github.com/YosysHQ/picorv32",
            sha: "a473fc8fca393771d83b0ffcf0b14db3393339d8",
            license: "ISC",
        },
        shape: Shape::Cpu,
        root: "picorv32",
        dir: "picorv32",
        vita_args: &[],
        iverilog_args: &[],
        // `tbd.v`, not the older `tb.v`: that one prints final state only, which is
        // blind to any divergence the core later overwrites.
        files: &["tbd.v", "src/picorv32.v"],
        data: &[],
        plusargs: &["+N=400000"],
        digest: "DIGEST=8bdb7a2fb9b56280",
        expect: Expect::Runs { exit: 0 },
        // Not verilator: picorv32's register file starts uninitialised, so the design
        // genuinely depends on 4-state semantics and verilator's 2-state
        // approximation disagrees (17b6f447736ac50d). Recorded, not hidden.
        oracle: "iverilog 13.0",
        note: "the reference RISC-V workload; tbd.v accumulates, tb.v does not",
    },
    Workload {
        name: "darkriscv",
        origin: Origin::Upstream {
            repo: "https://github.com/darklife/darkriscv",
            sha: "4aa437997cd35253c9111f10a449de13ccaeee78",
            license: "BSD-3-Clause",
        },
        shape: Shape::Cpu,
        // Upstream's own sim directory: `darkram.v` opens `../src/darksocv.mem`
        // relative to the working directory, so this is not a free choice.
        root: "darkriscv",
        dir: "darkriscv/src/sim",
        vita_args: &[
            "--top",
            "tb2",
            "-DSIMULATION=1",
            "-D__WAITSTATE__=7",
            "-I",
            "../rtl",
        ],
        iverilog_args: &[
            "-s",
            "tb2",
            "-DSIMULATION=1",
            "-D__WAITSTATE__=7",
            "-I",
            "../rtl",
        ],
        files: &["../../tb2.v", "../rtl/darkriscv.v", "../rtl/darkram.v"],
        data: &["../src/darksocv.mem"],
        plusargs: &["+N=600000"],
        digest: "DIGEST=59370cf8b1d0503d",
        expect: Expect::Runs { exit: 0 },
        oracle: "iverilog 13.0 (verilator 5.050 agrees)",
        // The full SoC (darksocv + darkuart + …) is REFUSED — see ROADMAP §3 ③.
        note: "core only — the full SoC is refused (ROADMAP §3 ③). One of two rows vita loses",
    },
    Workload {
        name: "biriscv",
        origin: Origin::Upstream {
            repo: "https://github.com/ultraembedded/biriscv",
            sha: "6af9c4be5a0807d368eaad5e49af52322e31d073",
            license: "Apache-2.0",
        },
        shape: Shape::Cpu,
        root: "biriscv",
        dir: "biriscv",
        vita_args: &["--top", "tb_top", "-I", "src/src/core", "-D", "TRACE=0"],
        iverilog_args: &["-s", "tb_top", "-I", "src/src/core", "-D", "TRACE=0"],
        files: &[
            "src/src/core/biriscv_alu.v",
            "src/src/core/biriscv_csr_regfile.v",
            "src/src/core/biriscv_csr.v",
            "src/src/core/biriscv_decode.v",
            "src/src/core/biriscv_decoder.v",
            "src/src/core/biriscv_defs.v",
            "src/src/core/biriscv_divider.v",
            "src/src/core/biriscv_exec.v",
            "src/src/core/biriscv_fetch.v",
            "src/src/core/biriscv_frontend.v",
            "src/src/core/biriscv_issue.v",
            "src/src/core/biriscv_lsu.v",
            "src/src/core/biriscv_mmu.v",
            "src/src/core/biriscv_multiplier.v",
            "src/src/core/biriscv_npc.v",
            "src/src/core/biriscv_pipe_ctrl.v",
            "src/src/core/biriscv_regfile.v",
            "src/src/core/biriscv_trace_sim.v",
            "src/src/core/biriscv_xilinx_2r1w.v",
            "src/src/core/riscv_core.v",
            "src/tb/tb_core_icarus/tcm_mem.v",
            "src/tb/tb_core_icarus/tcm_mem_ram.v",
            "tb.v",
        ],
        // Regenerated by bench/biriscv/prepare.sh — never committed.
        data: &["prog.hex"],
        plusargs: &["+N=50000"],
        digest: "DIGEST=22481d1cacf87584",
        expect: Expect::Runs { exit: 0 },
        oracle: "iverilog 13.0",
        note: "dual-issue, the largest design that runs (8.8k lines)",
    },
    Workload {
        name: "serv",
        origin: Origin::Upstream {
            repo: "https://github.com/olofk/serv",
            sha: "41e8aeedfd1e9ad5f95902c5b0dfc83d1c99e5d2",
            license: "ISC",
        },
        shape: Shape::Cpu,
        root: "serv",
        dir: "serv",
        // `-s tb` is not optional: this file list has three uninstantiated roots
        // (`tb`, `serv_rf_top`, `servile_rf_mem_if`), so without it the oracle
        // elaborates three designs where vita elaborates one — and the comparison
        // stops being between the same thing.
        vita_args: &["--top", "tb"],
        iverilog_args: &["-s", "tb"],
        files: &[
            "tb.v",
            "src/rtl/serv_bufreg.v",
            "src/rtl/serv_bufreg2.v",
            "src/rtl/serv_alu.v",
            "src/rtl/serv_csr.v",
            "src/rtl/serv_ctrl.v",
            "src/rtl/serv_decode.v",
            "src/rtl/serv_immdec.v",
            "src/rtl/serv_mem_if.v",
            "src/rtl/serv_rf_if.v",
            "src/rtl/serv_rf_ram_if.v",
            "src/rtl/serv_rf_ram.v",
            "src/rtl/serv_state.v",
            "src/rtl/serv_debug.v",
            "src/rtl/serv_top.v",
            "src/rtl/serv_rf_top.v",
            "src/rtl/serv_aligner.v",
            "src/rtl/serv_compdec.v",
            "src/servile/servile_arbiter.v",
            "src/servile/servile_mux.v",
            "src/servile/servile_rf_mem_if.v",
            "src/servile/servile.v",
            "src/servant/servant_timer.v",
            "src/servant/servant_gpio.v",
            "src/servant/servant_mux.v",
            "src/servant/servant_ram.v",
            "src/servant/servant.v",
        ],
        data: &["src/sw/blinky.hex"],
        plusargs: &["+N=500000"],
        digest: "DIGEST=f3f45af36093b2b1",
        expect: Expect::Refused {
            diag: "the override of parameter `RESET_STRATEGY` is not a constant",
        },
        // Not verilator: SERV reads an uninitialised register file and drives x
        // deliberately, so its 2-state result (e7e8b5e6c1276563) is a different
        // design's answer. iverilog and vita's probe build agree.
        oracle: "iverilog 13.0",
        note: "bit-serial, ~35 clk/insn — a scheduler-throughput workload. ROADMAP §3 ①",
    },
    Workload {
        name: "verilog-axi",
        origin: Origin::Upstream {
            repo: "https://github.com/alexforencich/verilog-axi",
            sha: "516bd5dadc3365b7f9e225d2af8fe0b8d804fe53",
            license: "MIT",
        },
        shape: Shape::Fabric,
        root: "verilog-axi",
        dir: "verilog-axi",
        vita_args: &[],
        iverilog_args: &[],
        files: &[
            "src/rtl/axi_crossbar.v",
            "src/rtl/axi_crossbar_rd.v",
            "src/rtl/axi_crossbar_wr.v",
            "src/rtl/axi_crossbar_addr.v",
            "src/rtl/axi_register_rd.v",
            "src/rtl/axi_register_wr.v",
            "src/rtl/arbiter.v",
            "src/rtl/priority_encoder.v",
            "src/rtl/axi_ram.v",
            "tb.v",
        ],
        data: &[],
        plusargs: &["+N=5000"],
        digest: "DIGEST=3b9321d5ea42f302",
        expect: Expect::Refused {
            diag: "parameter `S_THREADS` value is not a foldable constant expression",
        },
        oracle: "iverilog 13.0",
        note: "2x2 crossbar — elaboration- and generate-heavy. ROADMAP §3 ②",
    },
    Workload {
        name: "verilog-ethernet",
        origin: Origin::Upstream {
            repo: "https://github.com/alexforencich/verilog-ethernet",
            sha: "77320a9471d19c7dd383914bc049e02d9f4f1ffb",
            license: "MIT",
        },
        shape: Shape::Stream,
        root: "verilog-ethernet",
        dir: "verilog-ethernet",
        vita_args: &[],
        iverilog_args: &[],
        files: &[
            "tb.v",
            "src/rtl/eth_mac_1g.v",
            "src/rtl/axis_gmii_rx.v",
            "src/rtl/axis_gmii_tx.v",
            "src/rtl/lfsr.v",
        ],
        data: &[],
        plusargs: &["+N=1000"],
        digest: "DIGEST=ca4945d0044f74d8",
        expect: Expect::Refused {
            diag: "parameter `STYLE_INT` value is not a foldable constant expression",
        },
        oracle: "iverilog 13.0 (verilator 5.050 agrees)",
        note: "GMII loopback — the only streaming shape. ROADMAP §3 ①",
    },
    Workload {
        name: "keccak",
        origin: Origin::FirstParty,
        shape: Shape::Crypto,
        root: "keccak",
        dir: "keccak",
        vita_args: &[],
        iverilog_args: &[],
        files: &["tb.sv", "keccak_f.sv"],
        data: &[],
        plusargs: &["+N=2000"],
        digest: "perms=2000 lane0=54aa20c46ef0e0f6 lane1=b19e9f995e1f41d3 acc=767c5ab6776c4bde",
        expect: Expect::Runs { exit: 0 },
        oracle: "iverilog 13.0, verilator 5.050 and a Python reference all agree",
        note: "first-party; the readable core, with rho as a user function",
    },
    Workload {
        name: "keccak-arr",
        origin: Origin::FirstParty,
        shape: Shape::Crypto,
        root: "keccak",
        dir: "keccak",
        vita_args: &[],
        iverilog_args: &[],
        // Identical to `keccak` except that `rho` builds a 25-element array on every
        // call. That one difference is the whole reason this row exists: it is the
        // only workload in the corpus where vita loses badly, and every arena
        // estimate to date was computed from it.
        files: &["tb.sv", "keccak_f_arr.sv"],
        data: &[],
        plusargs: &["+N=2000"],
        digest: "perms=2000 lane0=54aa20c46ef0e0f6 lane1=b19e9f995e1f41d3 acc=767c5ab6776c4bde",
        expect: Expect::Runs { exit: 0 },
        oracle: "iverilog 13.0, verilator 5.050 and a Python reference all agree",
        note: "first-party; the corpus worst case, and the only design the arena estimate used",
    },
];
