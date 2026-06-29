//! Decode-time staleness gates (doc-14 §2 RULE D2, doc-15 9xxx).
//! Policy is version-GATE (refuse-and-rebuild), never silent migration.
use diag::MsgCode;

use crate::header::{VelabHeader, CURRENT_FORMAT_VERSION};

/// A gate rejection, tagged with the stable diagnostic code to emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactError {
    pub code: MsgCode,
    pub message: String,
}

impl ArtifactError {
    pub fn format(msg: &str) -> Self {
        ArtifactError {
            code: MsgCode::ArtFormatMismatch,
            message: msg.to_string(),
        }
    }
    pub fn schema(msg: &str) -> Self {
        ArtifactError {
            code: MsgCode::ArtSchemaMismatch,
            message: msg.to_string(),
        }
    }
    pub fn version(msg: &str) -> Self {
        ArtifactError {
            code: MsgCode::ArtVersionGate,
            message: msg.to_string(),
        }
    }
}

/// This build's identity, compared against an artifact header.
pub struct ToolContext {
    pub format_version: u32,
    pub schema_hash: [u8; 32],
    pub semver_major: u32,
}

impl ToolContext {
    /// Generic: build a tool context for ANY artifact body with the caller-supplied
    /// expected structural hash (e.g. `schema_hash::<SourceUnit>()` for a `.vu`).
    /// `format_version` + `semver_major` are fixed for this build, so the container
    /// stays language-neutral — the CLI passes the per-stage expected hash.
    pub fn new(schema_hash: [u8; 32]) -> Self {
        ToolContext {
            format_version: CURRENT_FORMAT_VERSION,
            schema_hash,
            semver_major: env!("CARGO_PKG_VERSION_MAJOR")
                .parse()
                .expect("CARGO_PKG_VERSION_MAJOR is a valid u32"),
        }
    }

    /// The running tool's expected values. `schema_hash` is the structural hash of
    /// the frozen sim-ir root (M3: `SimIr`). Convenience for the `.velab` gate.
    pub fn current() -> Self {
        Self::new(vita_schema::schema_hash::<sim_ir::SimIr>())
    }
}

/// Header gates, lower->higher: format (magic/version) is the lowest gate, then the
/// tool semver-major, then the structural schema hash. Any mismatch is a hard error
/// with a rebuild hint — never silent reuse (doc-15 9xxx, doc-14 §5).
pub fn verify_header(h: &VelabHeader, tool: &ToolContext) -> Result<(), ArtifactError> {
    if h.format_version != tool.format_version {
        return Err(ArtifactError::format(&format!(
            "format_version={} but this tool expects {}; regenerate with `velab`",
            h.format_version, tool.format_version
        )));
    }
    if h.tool_semver_major != tool.semver_major {
        return Err(ArtifactError::version(&format!(
            "produced by vitamin {}.x, this tool is {}.x; regenerate or install a matching vitamin",
            h.tool_semver_major, tool.semver_major
        )));
    }
    if h.schema_hash != tool.schema_hash {
        return Err(ArtifactError::schema(
            "sim-ir type shape changed between builds; rerun `velab`",
        ));
    }
    Ok(())
}
