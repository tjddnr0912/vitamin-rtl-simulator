//! `.vu` container = MAGIC_VU ++ postcard(header) ++ body; header-only decode preserves the body.
use vita_artifact::{read_vu, write_vu, Provenance, VelabHeader, MAGIC_VU};

fn hdr(schema: [u8; 32]) -> VelabHeader {
    VelabHeader {
        format_version: vita_artifact::CURRENT_FORMAT_VERSION,
        schema_hash: schema,
        composite_input_hash: [0; 32],
        global_time_precision: 0,
        consumed: Vec::new(),
        worklib_manifest_hash: [0; 32],
        uses_dump: false,
        tool_semver_major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        provenance: Provenance::capture(),
    }
}

// TEST 1: VU magic prefixes the stream and the body is preserved byte-for-byte.
#[test]
fn vu_magic_and_body_preserved() {
    let bytes = write_vu(&hdr([0xCD; 32]), b"VU-BODY-BYTES");
    assert_eq!(&bytes[..8], &MAGIC_VU);
    let (got, body) = read_vu(&bytes).expect("decode");
    assert_eq!(got.schema_hash, [0xCD; 32]);
    assert_eq!(
        body, b"VU-BODY-BYTES",
        "header-only decode must leave the body untouched"
    );
}

// TEST 2: a `.vu` fed to read_velab fails the FORMAT gate (magic mismatch), not the schema gate.
#[test]
fn vu_read_as_velab_is_format_mismatch() {
    let bytes = write_vu(&hdr([0xCD; 32]), b"x");
    let err = vita_artifact::read_velab(&bytes).unwrap_err();
    assert_eq!(err.code, diag::MsgCode::ArtFormatMismatch);
}
