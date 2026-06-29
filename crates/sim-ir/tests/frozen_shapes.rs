use sim_ir::*;
use vita_schema::SchemaShape;

#[test]
fn schema_names_are_crate_root_fq() {
    assert_eq!(ProcFlags::schema_name(), "sim_ir::ProcFlags");
    assert_eq!(EdgeKind::schema_name(), "sim_ir::EdgeKind");
    assert_eq!(SuspendState::schema_name(), "sim_ir::SuspendState");
}

#[test]
fn four_state() {
    assert_eq!(
        FourState::local_shape(),
        "repr=@#[]enum{#[]Zero,#[]One,#[]X,#[]Z}"
    );
}
#[test]
fn edge_kind() {
    assert_eq!(
        EdgeKind::local_shape(),
        "repr=@#[]enum{#[]Posedge,#[]Negedge,#[]AnyEdge}"
    );
}
#[test]
fn proc_flags() {
    assert_eq!(ProcFlags::local_shape(), "repr=@#[]newtype(#[]u8)");
}
#[test]
fn region_tag() {
    assert_eq!(
        RegionTag::local_shape(),
        "repr=@#[]enum{#[]Active,#[]Inactive,#[]Nba,#[]Monitor}"
    );
}
#[test]
fn frame() {
    assert_eq!(
        Frame::local_shape(),
        "repr=@#[]struct{#[]return_pc:u32,#[]callee_entry:u32,#[]locals_base:u32,#[]locals_len:u32,#[]is_automatic:bool}"
    );
}
#[test]
fn join_state() {
    assert_eq!(
        JoinState::local_shape(),
        "repr=@#[]struct{#[]parent:Option<u32>,#[]children:Vec<u32>,#[]detached:Vec<u32>,#[]flags:sim_ir::ProcFlags}"
    );
}
#[test]
fn wake_cond() {
    assert_eq!(
        WakeCond::local_shape(),
        "repr=@#[]enum{#[]Edge{#[]net:u32,#[]kind:sim_ir::EdgeKind},#[]Level{#[]nets:Vec<u32>},#[]WaitTrue{#[]expr:u32},#[]TimeAbs{#[]tick:u64},#[]NamedEvent{#[]ev:u32},#[]Join{#[]join_ref:u32}}"
    );
}
#[test]
fn wake_key() {
    assert_eq!(
        WakeKey::local_shape(),
        "repr=@#[]struct{#[]cond:sim_ir::WakeCond,#[]region:sim_ir::RegionTag,#[]tie_break:u32}"
    );
}
#[test]
fn suspend_state() {
    assert_eq!(
        SuspendState::local_shape(),
        "repr=@#[]struct{#[]resume_pc:u32,#[]locals:Vec<sim_ir::FourState>,#[]join_state:sim_ir::JoinState,#[]wake_key:sim_ir::WakeKey,#[]call_stack:Vec<sim_ir::Frame>,#[]frame_arena:Vec<sim_ir::FourState>}"
    );
}
