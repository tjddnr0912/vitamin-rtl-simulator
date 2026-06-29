//! The three header gates fire their exact MsgCode; the happy path passes; a
//! schema_hash stamped with the current build verifies, a tampered one fails.
use diag::MsgCode;
use vita_artifact::{verify_header, Provenance, ToolContext, VelabHeader, CURRENT_FORMAT_VERSION};

fn header_for(tool: &ToolContext) -> VelabHeader {
    VelabHeader {
        format_version: tool.format_version,
        schema_hash: tool.schema_hash,
        composite_input_hash: [0; 32],
        global_time_precision: 0,
        consumed: vec![],
        worklib_manifest_hash: [0; 32],
        uses_dump: false,
        tool_semver_major: tool.semver_major,
        provenance: Provenance::capture(),
    }
}

#[test]
fn current_build_header_verifies() {
    let tool = ToolContext::current();
    assert!(verify_header(&header_for(&tool), &tool).is_ok());
}

#[test]
fn format_version_mismatch() {
    let tool = ToolContext::current();
    let mut h = header_for(&tool);
    h.format_version = CURRENT_FORMAT_VERSION + 1;
    assert_eq!(
        verify_header(&h, &tool).unwrap_err().code,
        MsgCode::ArtFormatMismatch
    );
}

#[test]
fn schema_hash_mismatch() {
    let tool = ToolContext::current();
    let mut h = header_for(&tool);
    h.schema_hash[0] ^= 0xFF; // tamper one byte
    assert_eq!(
        verify_header(&h, &tool).unwrap_err().code,
        MsgCode::ArtSchemaMismatch
    );
}

#[test]
fn semver_major_mismatch() {
    let tool = ToolContext::current();
    let mut h = header_for(&tool);
    h.tool_semver_major = tool.semver_major + 1;
    assert_eq!(
        verify_header(&h, &tool).unwrap_err().code,
        MsgCode::ArtVersionGate
    );
}

#[test]
fn stamped_schema_hash_is_the_sim_ir_root() {
    let tool = ToolContext::current();
    let expected = vita_schema::schema_hash::<sim_ir::SimIr>(); // M3 root (was SuspendState)
    assert_eq!(tool.schema_hash, expected);
}
