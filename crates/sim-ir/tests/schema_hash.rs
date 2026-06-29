//! Golden #1: pinned SimIr root hash (M3 backbone, 2-platform determinism contract).
//! Golden #2: canonical-string diff. Plus a Process sub-pin (runtime-cluster regression).
use vita_schema::{schema_hash, SchemaShape, ShapeRegistry};

/// blake3 of the full SimIr-closure canonical string. Locked at
/// format_version 19 (2026-06-25: N6 real-math — 21 SysFuncId variants
/// `Ln`/`Log10`/`Exp`/`Sqrt`/`Pow`/`Floor`/`Ceil`/`Sin`/`Cos`/`Tan`/`Asin`/
/// `Acos`/`Atan`/`Atan2`/`Hypot`/`Sinh`/`Cosh`/`Tanh`/`Asinh`/`Acosh`/`Atanh`,
/// IEEE §20.8.2 — plus 2 non-uniform `$dist_*` ids `DistT`/`DistErlang`. All
/// reached from SimIr via the Expr arena, so the root hash flips; the Process
/// cluster reaches Expr only via arena INDICES (u32), so its sub-pin is
/// UNCHANGED. Earlier: format_version 18 (2026-06-24: 5 SysFuncId `StrAtoi`/`StrAtohex`/`StrAtooct`/
/// `StrAtobin`/`StrAtoreal` + 4 SysTaskId `StrItoa`/`StrHextoa`/`StrOcttoa`/
/// `StrBintoa` for the ⓑ-breadth string conversion methods, IEEE §6.16.9-17).
/// (2026-06-24 v17: `Expr::ArrayItem` (the with-clause iterator)
/// plus the `SysTaskId::ArrLocator` variant for the ⓑ-breadth array locator
/// methods, IEEE §7.12.1). Both are reached from SimIr via the Expr/Stmt arenas,
/// so the root hash flips; the Process cluster reaches them only through arena
/// INDICES (u32), so its sub-pin is UNCHANGED this bump. (2026-06-24 v16: 3
/// SysTaskId variants `ArrSort`/`ArrRsort`/`ArrReverse` for the array ordering
/// methods, §7.12.2. 2026-06-24 v15: 5 SysFuncId variants `ArrSum`/`ArrProduct`/
/// `ArrAnd`/`ArrOr`/`ArrXor` for the array reductions, §7.12.3. 2026-06-23 v10:
/// one extra SysTaskId variant `ClassRandomize` for N7-REST `obj.randomize()`.
/// 2026-06-18 v9: 13 SysFuncId and 5 SysTaskId for the file-read/$dist_*/$cast/
/// $writemem*/$monitoron-off family.)
const EXPECTED_SIMIR_HASH: &str =
    "37fa4f1f37d433ad94a8a6f03d4ee6dd9d03317ac322b4bdb885a4030194aad6";
/// Sub-pin: the runtime Process cluster (cheap regression signal; NOT the gate).
const EXPECTED_PROCESS_HASH: &str =
    "61db2e207ed69c2ff1dbf3fc0473b7ed9906fbeb6c42128ef9edf382b081f277";

const GOLDEN_CANON: &str = include_str!("../../testdata/sim_ir_canonical.txt");

#[test]
fn schema_hash_is_pinned() {
    assert_eq!(
        hex::encode(schema_hash::<sim_ir::SimIr>()),
        EXPECTED_SIMIR_HASH,
        "SCHEMA_HASH changed — a frozen sim-ir shape/serde-attr moved.\n\
         If intentional: all .velab invalid -> bump format_version + update both goldens."
    );
}

#[test]
fn process_subpin() {
    assert_eq!(
        hex::encode(schema_hash::<sim_ir::Process>()),
        EXPECTED_PROCESS_HASH
    );
}

#[test]
fn canonical_string_golden() {
    let mut reg = ShapeRegistry::new();
    sim_ir::SimIr::register(&mut reg);
    // Sanctioned regen switch for an INTENTIONAL format_version bump:
    //   REGEN_GOLDEN=1 cargo test -p sim-ir --test schema_hash -- --nocapture
    // rewrites the canonical golden and prints the two hashes to paste above.
    if std::env::var("REGEN_GOLDEN").is_ok() {
        std::fs::write("../testdata/sim_ir_canonical.txt", reg.canonical_string())
            .expect("write canonical golden");
        println!(
            "REGEN SimIr   = {}",
            hex::encode(schema_hash::<sim_ir::SimIr>())
        );
        println!(
            "REGEN Process = {}",
            hex::encode(schema_hash::<sim_ir::Process>())
        );
        return;
    }
    assert_eq!(reg.canonical_string(), GOLDEN_CANON);
}
