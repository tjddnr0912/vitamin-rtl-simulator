//! M3 backbone acceptance: representative types render their canonical shape with
//! FQ sibling refs (sim_ir::Foo), and the SimIr root registry builds.
use sim_ir::*;
use vita_schema::{schema_hash, SchemaShape, ShapeRegistry};

#[test]
fn binop_fieldless_enum() {
    assert_eq!(
        BinOp::local_shape(),
        "repr=@#[]enum{#[]Add,#[]Sub,#[]Mul,#[]Div,#[]Mod,#[]Pow,#[]BitAnd,#[]BitOr,#[]BitXor,#[]BitXnor,#[]LogAnd,#[]LogOr,#[]Lt,#[]Le,#[]Gt,#[]Ge,#[]Eq,#[]Ne,#[]CaseEq,#[]CaseNe,#[]Shl,#[]Shr,#[]AShl,#[]AShr,#[]CasezEq,#[]CasexEq}"
    );
}

#[test]
fn expr_has_fq_child_refs() {
    assert_eq!(
        Expr::local_shape(),
        "repr=@#[]enum{#[]Const{#[]val:u32},#[]Signal{#[]net:u32,#[]word:Option<u32>},#[]Select{#[]base:u32,#[]offset:u32,#[]width:u32,#[]kind:sim_ir::SelKind},#[]Concat{#[]parts:Vec<u32>},#[]Replicate{#[]count:u32,#[]value:u32},#[]Unary{#[]op:sim_ir::UnOp,#[]operand:u32},#[]Binary{#[]op:sim_ir::BinOp,#[]lhs:u32,#[]rhs:u32},#[]Ternary{#[]cond:u32,#[]then_e:u32,#[]else_e:u32},#[]SysFunc{#[]which:sim_ir::SysFuncId,#[]args:Vec<u32>},#[]Call{#[]func:u32,#[]args:Vec<u32>},#[]ArrayItem{#[]index:bool,#[]width:u32,#[]signed:bool}}"
    );
}

#[test]
fn terminator_fq_and_frozen_fork_call() {
    assert_eq!(
        Terminator::local_shape(),
        "repr=@#[]enum{#[]Goto{#[]target:u32},#[]Branch{#[]cond:u32,#[]then_bb:u32,#[]else_bb:u32},#[]Delay{#[]amount:u32,#[]region:sim_ir::DelayRegion,#[]resume:u32},#[]Wait{#[]cond:sim_ir::WaitCause,#[]resume:u32},#[]Fork{#[]children:Vec<u32>,#[]join:u32,#[]resume_bb:u32},#[]Call{#[]target:u32,#[]ret_bb:u32},#[]Return}"
    );
}

#[test]
fn simir_root_fq() {
    assert_eq!(
        SimIr::local_shape(),
        "repr=@#[]struct{#[]instances:Vec<sim_ir::Instance>,#[]nets:Vec<sim_ir::NetVar>,#[]processes:Vec<sim_ir::Process>,#[]cont_assigns:Vec<sim_ir::ContAssign>,#[]funcs:Vec<sim_ir::FuncDef>,#[]exprs:Vec<sim_ir::Expr>,#[]stmts:Vec<sim_ir::Stmt>,#[]blocks:Vec<sim_ir::BasicBlock>,#[]consts:Vec<sim_ir::ConstVal>}"
    );
    assert_eq!(SimIr::schema_name(), "sim_ir::SimIr");
}

#[test]
fn simir_registry_builds_and_hashes() {
    // Whole closure registers without a name collision; hash is stable.
    let mut reg = ShapeRegistry::new();
    SimIr::register(&mut reg);
    let h1 = schema_hash::<SimIr>();
    assert_eq!(h1, schema_hash::<SimIr>());
    // sanity: closure contains the PR1-B types too (reached via Vec<Process>)
    let canon = reg.canonical_string();
    assert!(canon.contains("sim_ir::SuspendState="));
    assert!(canon.contains("sim_ir::Expr="));
}
