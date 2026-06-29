//! Acceptance: derived schema_name()/local_shape()/register() render the canonical
//! grammar. Toy types live in a submodule so module_path! resolves to a known string.
//! NOTE: the derive renders the SPELLED field path, not the child's FQ schema_name —
//! see crate docs. So body references use the spelling (bare `ProcFlags`), while
//! schema_name() is the module_path-based FQ key.
use vita_artifact_derive::SchemaHash;
use vita_schema::{SchemaShape, ShapeRegistry};

mod sim_ir {
    use super::*;

    #[derive(SchemaHash)]
    #[allow(dead_code)]
    pub struct ProcFlags(pub u8);

    #[derive(SchemaHash)]
    #[allow(dead_code)]
    pub enum RegionTag {
        Active,
        Inactive,
        Nba,
        Monitor,
    }

    #[derive(SchemaHash)]
    #[allow(dead_code)]
    pub struct JoinState {
        pub parent: Option<u32>,
        pub children: Vec<u32>,
        pub detached: Vec<u32>,
        pub flags: ProcFlags, // spelled bare -> rendered as "ProcFlags"
    }
}

#[test]
fn procflags_newtype_shape() {
    assert_eq!(
        sim_ir::ProcFlags::schema_name(),
        "render::sim_ir::ProcFlags"
    );
    assert_eq!(sim_ir::ProcFlags::local_shape(), "repr=@#[]newtype(#[]u8)");
}

#[test]
fn regiontag_enum_shape() {
    assert_eq!(
        sim_ir::RegionTag::local_shape(),
        "repr=@#[]enum{#[]Active,#[]Inactive,#[]Nba,#[]Monitor}"
    );
}

#[test]
fn joinstate_struct_shape_and_children() {
    assert_eq!(
        sim_ir::JoinState::local_shape(),
        "repr=@#[]struct{#[]parent:Option<u32>,#[]children:Vec<u32>,#[]detached:Vec<u32>,#[]flags:ProcFlags}"
    );
    // register() interns the ProcFlags child exactly once under its FQ schema_name.
    let mut reg = ShapeRegistry::new();
    sim_ir::JoinState::register(&mut reg);
    let s = reg.canonical_string();
    assert_eq!(s.matches("render::sim_ir::ProcFlags=").count(), 1);
}
