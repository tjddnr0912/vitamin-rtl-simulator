//! velab artifact header (doc-14 §1) — written/decoded independently of the body.
use serde::{Deserialize, Serialize};

use crate::gate::ArtifactError;

/// 8-byte magic prefix (doc-14 §1 "VELAB\0", padded to 8).
pub const MAGIC_VELAB: [u8; 8] = *b"VELAB\0\0\0";

/// 8-byte magic prefix for the compiled `.vu` artifact (parse output).
/// Distinct from `MAGIC_VELAB` so a `.vu` fed to `vrun` fails the FORMAT gate,
/// not the schema gate.
pub const MAGIC_VU: [u8; 8] = *b"VU\0\0\0\0\0\0";

/// Container format version. Bumped whenever the header layout changes.
pub const CURRENT_FORMAT_VERSION: u32 = 19; // v19 (2026-06-25): N6 real-math system functions — +SysFuncId×21 ($ln/$log10/$exp/$sqrt/$pow/$floor/$ceil/$sin/$cos/$tan/$asin/$acos/$atan/$atan2/$hypot/$sinh/$cosh/$tanh/$asinh/$acosh/$atanh; IEEE §20.8.2, computed via vendored pure-Rust libm → 3-OS byte-identical) + SysFuncId×2 non-uniform $dist_* ids (DistT/DistErlang; DistNormal/Exponential/Poisson/ChiSquare existed inert since v9, now emitted). FROZEN-ROOT change → SimIr schema hash / canonical / RON goldens re-pinned. // v18 (2026-06-24): ⓑ-breadth string conversion methods — +SysFuncId×5 (StrAtoi/StrAtohex/StrAtooct/StrAtobin/StrAtoreal; §6.16.9-13) + SysTaskId×4 (StrItoa/StrHextoa/StrOcttoa/StrBintoa; §6.16.14-17). FROZEN-ROOT change → SimIr goldens re-pinned. // v17 (2026-06-24): ⓑ-breadth array locator methods + `with` iterator — +Expr::ArrayItem (with-clause iterator) + SysTaskId::ArrLocator (min/max/unique/find* → queue; IEEE §7.12.1). FROZEN-ROOT change (Expr + SysTaskId in SimIr) → SimIr schema hash / canonical / RON goldens re-pinned. // v16 (2026-06-24): ⓑ-breadth array ordering methods — +SysTaskId×3 (ArrSort/ArrRsort/ArrReverse; IEEE §7.12.2 in-place mutators). FROZEN-ROOT change (SysTaskId in SimIr) → SimIr schema hash / canonical / RON goldens re-pinned. // v15 (2026-06-24): ⓑ-breadth array reduction methods — +SysFuncId×5 (ArrSum/ArrProduct/ArrAnd/ArrOr/ArrXor; IEEE §7.12.3). FROZEN-ROOT change (SysFuncId in SimIr) → SimIr schema hash / canonical / RON goldens re-pinned. // v14 (2026-06-24): N7-REST B-CRV final — inline `randomize() with {…}` per-call constraints (`randomize_with` field in the 14th `StagedExtraSidecars` trailer; artifact-only, sim-ir UNCHANGED). // v13 (2026-06-24): N7-REST B2 `randc` cyclic fields (class_randc sidecar; artifact-only, sim-ir unchanged). // v12 (2026-06-24): N7-REST B2 `dist` weighted distributions (class_dist sidecar in the 14th trailer; artifact-only, sim-ir unchanged). // v11 (2026-06-23): N7-REST B2 constraint solver — `class_constraints` predicates added to the 14th `StagedExtraSidecars` trailer (artifact wire shape change; sim-ir UNCHANGED, so the SimIr schema hash / canonical / RON goldens are untouched — only the staged trailer + this constant). // v10 (2026-06-23): +SysTaskId::ClassRandomize (N7-REST `obj.randomize()`). Phase B1 constrained-random; rand-field bounds ride the `class_rand` sidecar (IR-0), only the randomize() call node is in the frozen IR. // v9 (2026-06-18): +SysFuncId×13 (file-read + $dist_* + $cast) + SysTaskId×5 ($writememb/h, $cast, $monitoron/off).

/// Build provenance (Layer 2). Stamped for traceability, NEVER a staleness key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub tool_version: String,
    pub git_sha: Option<String>,
    pub dirty: bool,
    pub profile: String,
}

impl Provenance {
    /// Capture from build-time env (no build.rs — option_env!/env!/cfg!).
    pub fn capture() -> Self {
        Provenance {
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            git_sha: option_env!("VITA_GIT_SHA").map(str::to_string),
            dirty: option_env!("VITA_GIT_DIRTY").is_some_and(|v| v == "1" || v == "true"),
            profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
            .to_string(),
        }
    }
}

/// velab header (doc-14 §1). Decodable before the body.
///
/// `composite_input_hash`/`consumed`/`worklib_manifest_hash` are stamped and
/// round-tripped here, but their RULE-V live-recheck gate (`E-ART-STALE-UPSTREAM`)
/// is deferred to a later PR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VelabHeader {
    pub format_version: u32,
    pub schema_hash: [u8; 32],
    pub composite_input_hash: [u8; 32],
    pub global_time_precision: i64,
    pub consumed: Vec<(String, [u8; 32])>,
    pub worklib_manifest_hash: [u8; 32],
    pub uses_dump: bool,
    pub tool_semver_major: u32,
    pub provenance: Provenance,
}

/// Internal: serialize as `magic ++ postcard(header) ++ body`. The body is opaque.
fn write_with_magic(magic: &[u8; 8], header: &VelabHeader, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + 64 + body.len());
    out.extend_from_slice(magic);
    let header_bytes =
        postcard::to_stdvec(header).expect("postcard header encode is infallible for owned data");
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(body);
    out
}

/// Internal: check `magic`, decode the header ALONE, return the untouched body slice.
/// A bad magic or undecodable header is a hard `E-ART-FORMAT-MISMATCH` (doc-15) —
/// the body is never deserialized.
fn read_with_magic<'a>(
    magic: &[u8; 8],
    label: &str,
    bytes: &'a [u8],
) -> Result<(VelabHeader, &'a [u8]), ArtifactError> {
    if bytes.len() < magic.len() || bytes[..magic.len()] != *magic {
        return Err(ArtifactError::format(&format!(
            "bad or missing {label} magic"
        )));
    }
    let after_magic = &bytes[magic.len()..];
    let (header, body) = postcard::take_from_bytes::<VelabHeader>(after_magic)
        .map_err(|e| ArtifactError::format(&format!("undecodable {label} header: {e}")))?;
    Ok((header, body))
}

/// Serialize as `MAGIC_VELAB ++ postcard(header) ++ body`.
pub fn write_velab(header: &VelabHeader, body: &[u8]) -> Vec<u8> {
    write_with_magic(&MAGIC_VELAB, header, body)
}

/// Check magic, then decode the header ALONE (body untouched). Returns the header
/// and the trailing body slice. A bad magic or undecodable header is a hard
/// `E-ART-FORMAT-MISMATCH` (doc-15) — the body is never deserialized.
pub fn read_velab(bytes: &[u8]) -> Result<(VelabHeader, &[u8]), ArtifactError> {
    read_with_magic(&MAGIC_VELAB, "velab", bytes)
}

/// Serialize a `.vu` (compiled `SourceUnit`) artifact:
/// `MAGIC_VU ++ postcard(header) ++ body`. `body` is `postcard(hdl_ast::SourceUnit)`
/// (the CLI owns that encode — the container stays language-neutral).
pub fn write_vu(header: &VelabHeader, body: &[u8]) -> Vec<u8> {
    write_with_magic(&MAGIC_VU, header, body)
}

/// Header-only decode of a `.vu`. Bad magic / undecodable header →
/// `E-ART-FORMAT-MISMATCH`; the body slice is returned untouched.
pub fn read_vu(bytes: &[u8]) -> Result<(VelabHeader, &[u8]), ArtifactError> {
    read_with_magic(&MAGIC_VU, "VU", bytes)
}
