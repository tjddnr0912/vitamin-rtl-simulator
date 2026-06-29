//! vita-artifact — staged-artifact container: header (de)serialize, version/schema
//! staleness gates. (Body serialization + RULE-V live re-hash land in later PRs.)
mod gate;
mod header;

pub use gate::{verify_header, ArtifactError, ToolContext};
pub use header::{
    read_velab, read_vu, write_velab, write_vu, Provenance, VelabHeader, CURRENT_FORMAT_VERSION,
    MAGIC_VELAB, MAGIC_VU,
};
