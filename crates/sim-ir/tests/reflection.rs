//! Layer 3 (16): trace the actual serde derives with serde-reflection and diff the
//! resulting Registry (RON) against a committed golden. Catches wire drift that the
//! syn-attr Layer-1 path cannot see.
use serde_reflection::{Tracer, TracerConfig};

fn traced_registry_ron() -> String {
    let mut tracer = Tracer::new(TracerConfig::default());
    // Scalar leaf enums.
    tracer.trace_simple_type::<sim_ir::FourState>().unwrap();
    tracer.trace_simple_type::<sim_ir::EdgeKind>().unwrap();
    tracer.trace_simple_type::<sim_ir::RegionTag>().unwrap();
    tracer.trace_simple_type::<sim_ir::WakeCond>().unwrap();
    tracer.trace_simple_type::<sim_ir::UnOp>().unwrap();
    tracer.trace_simple_type::<sim_ir::BinOp>().unwrap();
    tracer.trace_simple_type::<sim_ir::SelKind>().unwrap();
    tracer.trace_simple_type::<sim_ir::SysFuncId>().unwrap();
    tracer.trace_simple_type::<sim_ir::SysTaskId>().unwrap();
    tracer.trace_simple_type::<sim_ir::DisableKind>().unwrap();
    tracer.trace_simple_type::<sim_ir::DelayRegion>().unwrap();
    tracer.trace_simple_type::<sim_ir::WaitCause>().unwrap();
    tracer.trace_simple_type::<sim_ir::Terminator>().unwrap();
    tracer.trace_simple_type::<sim_ir::SensKind>().unwrap();
    tracer.trace_simple_type::<sim_ir::NetKind>().unwrap();
    tracer.trace_simple_type::<sim_ir::PortDir>().unwrap();
    tracer.trace_simple_type::<sim_ir::ConstRepr>().unwrap();
    tracer.trace_simple_type::<sim_ir::Expr>().unwrap();
    tracer.trace_simple_type::<sim_ir::Stmt>().unwrap();
    // Struct types.
    tracer.trace_simple_type::<sim_ir::ProcFlags>().unwrap();
    tracer.trace_simple_type::<sim_ir::Frame>().unwrap();
    tracer.trace_simple_type::<sim_ir::JoinState>().unwrap();
    tracer.trace_simple_type::<sim_ir::WakeKey>().unwrap();
    tracer.trace_simple_type::<sim_ir::SuspendState>().unwrap();
    tracer.trace_simple_type::<sim_ir::LvalChunk>().unwrap();
    tracer.trace_simple_type::<sim_ir::Lvalue>().unwrap();
    tracer.trace_simple_type::<sim_ir::EdgeTerm>().unwrap();
    tracer.trace_simple_type::<sim_ir::Sensitivity>().unwrap();
    tracer.trace_simple_type::<sim_ir::BitPacked>().unwrap();
    tracer.trace_simple_type::<sim_ir::NetVar>().unwrap();
    tracer.trace_simple_type::<sim_ir::ConstVal>().unwrap();
    tracer.trace_simple_type::<sim_ir::BasicBlock>().unwrap();
    tracer.trace_simple_type::<sim_ir::Process>().unwrap();
    tracer.trace_simple_type::<sim_ir::ContAssign>().unwrap();
    tracer.trace_simple_type::<sim_ir::Instance>().unwrap();
    tracer.trace_simple_type::<sim_ir::FuncDef>().unwrap();
    tracer.trace_simple_type::<sim_ir::SimIr>().unwrap();
    let registry = tracer.registry().unwrap();
    // PrettyConfig::default() picks the PLATFORM newline (\r\n on Windows),
    // which broke the byte-compared golden on windows-latest (3-OS CI run 3).
    // Pin "\n" explicitly — the golden must be byte-identical on every OS.
    let cfg = ron::ser::PrettyConfig::default().new_line("\n".to_string());
    ron::ser::to_string_pretty(&registry, cfg).unwrap()
}

#[test]
fn serde_reflection_ron_golden() {
    // Sanctioned regen for an INTENTIONAL format_version bump (same switch as
    // schema_hash.rs): REGEN_GOLDEN=1 rewrites the committed RON golden.
    if std::env::var("REGEN_GOLDEN").is_ok() {
        std::fs::write("../testdata/sim_ir_registry.ron", traced_registry_ron())
            .expect("write RON golden");
        return;
    }
    let golden = include_str!("../../testdata/sim_ir_registry.ron");
    assert_eq!(
        traced_registry_ron().trim_end(),
        golden.trim_end(),
        "serde wire format drifted (Layer 3). Update sim_ir_registry.ron only if intentional."
    );
}
