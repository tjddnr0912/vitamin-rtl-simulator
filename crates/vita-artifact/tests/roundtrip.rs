//! A written velab = MAGIC ++ postcard(header) ++ body; reading decodes the header
//! alone (postcard::take_from_bytes) and returns the untouched body slice.
use vita_artifact::{read_velab, write_velab, Provenance, VelabHeader, MAGIC_VELAB};

fn sample_header() -> VelabHeader {
    VelabHeader {
        format_version: 1,
        schema_hash: [0xAB; 32],
        composite_input_hash: [0x11; 32],
        global_time_precision: -12,
        consumed: vec![("work:top".to_string(), [0x22; 32])],
        worklib_manifest_hash: [0x33; 32],
        uses_dump: true,
        tool_semver_major: 0,
        provenance: Provenance {
            tool_version: "0.0.0".to_string(),
            git_sha: Some("deadbeef".to_string()),
            dirty: false,
            profile: "debug".to_string(),
        },
    }
}

#[test]
fn magic_prefixes_the_stream() {
    let bytes = write_velab(&sample_header(), b"BODYBYTES");
    assert_eq!(&bytes[..8], &MAGIC_VELAB);
}

#[test]
fn header_roundtrips_and_body_is_preserved() {
    let h = sample_header();
    let bytes = write_velab(&h, b"BODYBYTES");
    let (got, body) = read_velab(&bytes).expect("decode");
    assert_eq!(got, h);
    assert_eq!(
        body, b"BODYBYTES",
        "header-only decode must leave the body untouched"
    );
}

#[test]
fn wrong_magic_is_format_mismatch() {
    let mut bytes = write_velab(&sample_header(), b"X");
    bytes[1] ^= 0xFF; // corrupt magic
    let err = read_velab(&bytes).unwrap_err();
    assert_eq!(err.code, diag::MsgCode::ArtFormatMismatch);
}
