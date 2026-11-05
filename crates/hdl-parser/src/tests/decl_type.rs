//! The data type every declaration carries (`hdl_ast::DataType`, §5.2 Q-DECLTYPE S12):
//! what each spelling records, per container family.
//!
//! A parameter records the signing keyword its sign fold read and its declared kind
//! (no reader trusts either yet: Q-PARAM2STATE); an associative key records its signing
//! keyword; every other container records neither the keyword nor anything new, and its
//! kind, effective sign and packed dims are the parser's as before. Each family's pin
//! also asserts the states its constructor admits (`DataType::new_*`).
use super::*;

fn unit(src: &str) -> SourceUnit {
    let (su, errs) = p(src);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    su.unwrap()
}

fn gen_params<'a>(items: &'a [GenItem], out: &mut Vec<&'a ParamDecl>) {
    for g in items {
        match g {
            GenItem::For { body, .. } | GenItem::Block { items: body, .. } => gen_params(body, out),
            GenItem::If { then_b, else_b, .. } => {
                gen_params(then_b, out);
                gen_params(else_b, out);
            }
            GenItem::Case { items, .. } => {
                for c in items {
                    match c {
                        GenCaseItem::Match { body, .. } | GenCaseItem::Default { body, .. } => {
                            gen_params(body, out)
                        }
                    }
                }
            }
            GenItem::Item(it) => body_params(std::slice::from_ref(it), out),
        }
    }
}

fn body_params<'a>(body: &'a [ModuleItem], out: &mut Vec<&'a ParamDecl>) {
    for it in body {
        match it {
            ModuleItem::Param(p) => out.push(p),
            ModuleItem::Generate(g) => gen_params(&g.items, out),
            _ => {}
        }
    }
}

/// Every `ParamDecl` of the unit: module / interface / package headers and bodies,
/// generate blocks included.
fn all_params(su: &SourceUnit) -> Vec<&ParamDecl> {
    let mut out = Vec::new();
    for t in &su.items {
        if let TopItem::Module(m) | TopItem::Interface(m) | TopItem::Package(m) = t {
            out.extend(m.params.iter());
            body_params(&m.body, &mut out);
        }
    }
    out
}

/// The `(kind, signing)` record of parameter `name`.
fn rec(su: &SourceUnit, name: &str) -> (TypeKind, Option<Signing>) {
    let ps = all_params(su);
    let p = ps.iter().find(|p| p.name.name == name).unwrap_or_else(|| {
        let names: Vec<_> = ps.iter().map(|p| p.name.name.as_str()).collect();
        panic!("no parameter `{name}` among {names:?}")
    });
    (p.dtype.kind(), p.dtype.signing())
}

use Signing::{Implicit as SI, Signed as SS, Unsigned as SU};
use TypeKind::{Implicit as KI, NotRecorded as KN};

fn k(n: NetVarKind) -> TypeKind {
    TypeKind::Kind(n)
}

/// The parameter family's invariant: a signing keyword is always recorded, the packed
/// dims never are (the range is the product range).
fn assert_param_family(su: &SourceUnit) {
    for p in all_params(su) {
        assert!(
            p.dtype.signing().is_some(),
            "`{}` records its signing",
            p.name.name
        );
        assert_eq!(
            p.dtype.packed_record(),
            None,
            "`{}` packed dims",
            p.name.name
        );
    }
}

#[test]
fn a_keyword_parameter_records_its_kind_and_the_signing_its_fold_read() {
    let su = unit(
        "module m;\n\
         parameter P0 = 1; parameter [3:0] P1 = 1; parameter signed P2 = 1; parameter unsigned P3 = 1;\n\
         localparam logic L0 = 1; localparam bit L1 = 1; localparam reg L2 = 1; localparam int L3 = 1;\n\
         localparam integer L4 = 1; localparam byte L5 = 1; localparam shortint L6 = 1;\n\
         localparam longint L7 = 1; localparam time L8 = 1; localparam real L9 = 1.0;\n\
         localparam realtime L10 = 1.0; localparam string L11 = \"a\";\n\
         localparam int unsigned U0 = 1; localparam integer signed U1 = 1;\n\
         localparam logic signed [3:0] U2 = 1; localparam unsigned int U3 = 1; localparam bit [3:0] U4 = 1;\n\
         localparam signed string S2 = \"cd\";\n\
         specparam signed SP = 4'hF; specparam [3:0] SR = 4'hF;\n\
         parameter signed real R = 1.5; parameter signed time T2 = 7;\n\
         parameter signed int unsigned D = 32'hFFFFFFFF; parameter unsigned integer signed E = -1;\n\
         endmodule\n",
    );
    use NetVarKind as N;
    for (name, want) in [
        ("P0", (KI, SI)),
        ("P1", (KI, SI)),
        ("P2", (KI, SS)),
        ("P3", (KI, SU)),
        ("L0", (k(N::Logic), SI)),
        ("L1", (k(N::Bit), SI)),
        ("L2", (k(N::Reg), SI)),
        ("L3", (k(N::Int), SI)),
        ("L4", (k(N::Integer), SI)),
        ("L5", (k(N::Byte), SI)),
        ("L6", (k(N::Shortint), SI)),
        ("L7", (k(N::Longint), SI)),
        ("L8", (k(N::Time), SI)),
        ("L9", (k(N::Real), SI)),
        ("L10", (k(N::Realtime), SI)),
        ("L11", (k(N::String), SI)),
        ("U0", (k(N::Int), SU)),
        ("U1", (k(N::Integer), SS)),
        ("U2", (k(N::Logic), SS)),
        ("U3", (k(N::Int), SU)),
        ("U4", (k(N::Bit), SI)),
        // `string` is a kind, never Implicit, and keeps the keyword its fold read.
        ("S2", (k(N::String), SS)),
        // `specparam` parses as a localparam.
        ("SP", (KI, SS)),
        ("SR", (KI, SI)),
        // Spellings outside IEEE that the parser accepts record the token the fold read:
        // a signing keyword before `real` / `time`, and a keyword on both sides (the
        // leading one wins).
        ("R", (k(N::Real), SS)),
        ("T2", (k(N::Time), SS)),
        ("D", (k(N::Int), SS)),
        ("E", (k(N::Integer), SU)),
    ] {
        assert_eq!(rec(&su, name), (want.0, Some(want.1)), "{name}");
    }
    assert_param_family(&su);
}

#[test]
fn a_header_parameter_records_like_a_body_one() {
    let su = unit(
        "module m #(parameter unsigned A = 1, B = 2, parameter int C = 3, D = 4) (); endmodule\n\
         module n #(signed P = 1, Q = 2) (); endmodule\n\
         module t #(parameter int AR [2] = '{1, 2}, parameter logic unsigned [3:0] BR [2] = '{1, 2}) ();\n\
         endmodule\n",
    );
    use NetVarKind as N;
    for (name, want) in [
        ("A", (KI, SU)),
        // A continuation inherits the whole prefix: kind and keyword.
        ("B", (KI, SU)),
        ("C", (k(N::Int), SI)),
        ("D", (k(N::Int), SI)),
        ("P", (KI, SS)),
        ("Q", (KI, SS)),
        // The header array twin records what the scalar would.
        ("AR", (k(N::Int), SI)),
        ("BR", (k(N::Logic), SU)),
    ] {
        assert_eq!(rec(&su, name), (want.0, Some(want.1)), "{name}");
    }
    assert_param_family(&su);
}

#[test]
fn every_container_of_a_parameter_records_the_keyword() {
    let su = unit(
        "localparam unsigned UK = 1;\n\
         package pk; parameter unsigned PK = 1; endpackage\n\
         interface ifc; localparam int unsigned IK = 1; endinterface\n\
         program pg; localparam bit signed GK = 1; endprogram\n\
         module g; if (1) begin : b localparam integer unsigned GN = 1; end endmodule\n",
    );
    use NetVarKind as N;
    for (name, want) in [
        ("UK", (KI, SU)),
        ("PK", (KI, SU)),
        ("IK", (k(N::Int), SU)),
        ("GK", (k(N::Bit), SS)),
        ("GN", (k(N::Integer), SU)),
    ] {
        assert_eq!(rec(&su, name), (want.0, Some(want.1)), "{name}");
    }
    assert_param_family(&su);
}

#[test]
fn a_typedef_typed_parameter_records_the_family_of_its_type() {
    let su = unit(
        "module m;\n\
         typedef logic [3:0] lv_t; localparam lv_t A1 = 1;\n\
         typedef int i_t; localparam i_t A2 = 1;\n\
         typedef lv_t lv2_t; localparam lv2_t A3 = 1;\n\
         typedef string str_t; localparam str_t S = \"ab\"; localparam signed str_t S3 = \"ef\";\n\
         typedef enum bit [1:0] { EA, EB } e_t; localparam e_t E1 = EB;\n\
         typedef enum byte { BA, BB } eb_t; localparam eb_t E2 = BB;\n\
         typedef enum { NA, NB } en_t; localparam en_t E3 = NB;\n\
         typedef e_t e2_t; localparam e2_t E4 = EA;\n\
         typedef struct packed { logic [3:0] a; bit b; } s_t; localparam s_t P1 = 5'd5;\n\
         typedef s_t s2_t; localparam s2_t P2 = 5'd5;\n\
         typedef union packed { logic [3:0] a; bit [3:0] b; } u_t; localparam u_t U1 = 4'd3;\n\
         endmodule\n\
         module sy; parameter W = 4;\n\
         typedef struct packed { logic [W-1:0] a; logic b; } ss_t; localparam ss_t Q1 = 5'd5;\n\
         endmodule\n\
         module tp #(parameter type T = logic [3:0]) ();\n\
         typedef T t2_t; localparam t2_t TP1 = 4'd5; localparam T TP2 = 4'd6;\n\
         endmodule\n",
    );
    use NetVarKind as N;
    for (name, want) in [
        ("A1", (k(N::Logic), SI)),
        ("A2", (k(N::Int), SI)),
        // A chained alias of a keyword type carries the base's record.
        ("A3", (k(N::Logic), SI)),
        ("S", (k(N::String), SI)),
        // The keyword written on the parameter, not the typedef's.
        ("S3", (k(N::String), SS)),
        // An enum (a vector base, an atom base, the default base, a chained alias), a
        // struct (exact, or a chained alias of one) and a union record no kind.
        ("E1", (KN, SI)),
        ("E2", (KN, SI)),
        ("E3", (KN, SI)),
        ("E4", (KN, SI)),
        ("P1", (KN, SI)),
        ("P2", (KN, SI)),
        ("U1", (KN, SI)),
        // A symbolic struct (a member bound is a parameter) and a type parameter, used
        // directly or through an alias, record no kind either.
        ("Q1", (KN, SI)),
        ("TP1", (KN, SI)),
        ("TP2", (KN, SI)),
    ] {
        assert_eq!(rec(&su, name), (want.0, Some(want.1)), "{name}");
    }
    // The parameters the type parameter desugars to are written untyped.
    let desugared: Vec<_> = all_params(&su)
        .into_iter()
        .filter(|p| p.name.name.contains('$'))
        .collect();
    assert!(!desugared.is_empty());
    for p in desugared {
        assert_eq!(
            (p.dtype.kind(), p.dtype.signing()),
            (KI, Some(SI)),
            "{}",
            p.name.name
        );
    }
    assert_param_family(&su);
}

#[test]
fn a_composed_alias_parameter_records_logic_like_its_twin() {
    // `v_t` is an alias composed of a packed struct of `logic` members; `e2_t` a
    // chained alias of a struct.
    let su = unit(
        "package q;\n\
         typedef struct packed { logic [1:0] a; } e_t;\n\
         typedef e_t [3:0] v_t; parameter v_t D = '0;\n\
         parameter logic [3:0][1:0] TW = '0;\n\
         typedef e_t e2_t; parameter e2_t E = 2'd1;\n\
         endpackage\nmodule top; endmodule\n",
    );
    assert_eq!(rec(&su, "D"), (k(NetVarKind::Logic), Some(SI)));
    assert_eq!(rec(&su, "D"), rec(&su, "TW"));
    assert_eq!(rec(&su, "E"), (KN, Some(SI)));
    assert_param_family(&su);
}

/// Spellings the first pins left out (lens S-F7), each asserting what the parser
/// records today -- `(kind, signing, effective sign)`: a typedef-prefixed header array
/// twin and its scalar sibling, a `signed` continuation that opens a new prefix,
/// interface and program header parameters, and a package-scoped typedef.
#[test]
fn the_remaining_parameter_spellings_record_what_the_parser_kept() {
    let su = unit(
        "package pk; typedef logic signed [3:0] s_t; typedef int unsigned u_t; endpackage\n\
         module m import pk::*; #(parameter s_t AR [2] = '{4'hF, 4'h1}, parameter s_t S = 4'hF) ();\n\
         endmodule\n\
         module w #(parameter W = 4, signed X = -1) (); endmodule\n\
         interface ih #(parameter int unsigned IH = 1, IC = 2) (); endinterface\n\
         program ph #(parameter bit signed PH = 1) (); endprogram\n\
         module top; localparam pk::u_t P = 32'hFFFFFFFF; localparam pk::s_t Q = 4'hF;\n\
         import pk::*; localparam s_t R = 4'hE; endmodule\n",
    );
    let got: Vec<_> = ["AR", "S", "W", "X", "IH", "IC", "PH", "P", "Q", "R"]
        .into_iter()
        .map(|name| {
            let p = all_params(&su)
                .into_iter()
                .find(|p| p.name.name == name)
                .unwrap_or_else(|| panic!("no parameter `{name}`"));
            (name, p.dtype.kind(), p.dtype.signing(), p.dtype.signed)
        })
        .collect();
    use NetVarKind as N;
    let want = [
        // The typedef-prefixed header array twin records what its scalar sibling does:
        // the typedef's kind, no keyword, the typedef's sign.
        ("AR", k(N::Logic), Some(SI), true),
        ("S", k(N::Logic), Some(SI), true),
        ("W", KI, Some(SI), false),
        // `signed X` after `W` opens a new prefix (the keyword is X's own).
        ("X", KI, Some(SS), true),
        ("IH", k(N::Int), Some(SU), false),
        ("IC", k(N::Int), Some(SU), false),
        ("PH", k(N::Bit), Some(SS), true),
        // A package-scoped typedef records as the imported name does.
        ("P", k(N::Int), Some(SI), false),
        ("Q", k(N::Logic), Some(SI), true),
        ("R", k(N::Logic), Some(SI), true),
    ];
    assert_eq!(got, want);
    assert_param_family(&su);
}

/// A port's or formal's kind reader refuses a record no port family can hold.
#[test]
#[should_panic(expected = "a port data type records its keyword")]
fn a_port_kind_reader_refuses_a_not_recorded_kind() {
    let _ = DataType::new_param(TypeKind::NotRecorded, Signing::Implicit, false, None).port();
}

#[test]
fn a_class_typed_parameter_is_refused() {
    let (_, errs) = p("class C; endclass\nmodule m; localparam C P = null; endmodule\n");
    assert!(!errs.is_empty());
}

fn module_body(su: &SourceUnit, name: &str) -> Vec<ModuleItem> {
    su.items
        .iter()
        .find_map(|t| match t {
            TopItem::Module(m) if m.name.name == name => Some(m.body.clone()),
            _ => None,
        })
        .unwrap()
}

fn net(body: &[ModuleItem], name: &str) -> NetVarDecl {
    body.iter()
        .find_map(|it| match it {
            ModuleItem::NetVar(d) if d.names.iter().any(|n| n.name.name == name) => Some(d.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no declaration of `{name}`"))
}

/// The net / variable / member / alias family's invariant: a kind is always recorded,
/// the signing keyword never is, the packed dims always are.
fn assert_net_family(d: &DataType) {
    assert!(matches!(d.kind(), TypeKind::Kind(_)), "{d:?}");
    assert_eq!(d.signing(), None, "{d:?}");
    assert!(d.packed_record().is_some(), "{d:?}");
}

#[test]
fn an_associative_key_records_the_signing_keyword_written() {
    let su = unit(
        "module m; typedef logic [7:0] addr_t;\n\
         int a0 [int]; int a1 [int unsigned]; int a2 [bit signed [7:0]]; int a3 [addr_t];\n\
         endmodule\n",
    );
    let body = module_body(&su, "m");
    for (name, want) in [("a0", SI), ("a1", SU), ("a2", SS), ("a3", SI)] {
        let d = net(&body, name);
        let Dim::Assoc(AssocKey::Typed(kt)) = &d.names[0].unpacked[0] else {
            panic!("{name}: {:?}", d.names[0].unpacked)
        };
        assert_eq!(kt.dtype.signing(), Some(want), "{name}");
        // The key family: a kind and a keyword always recorded, packed dims kept.
        assert!(matches!(kt.dtype.kind(), TypeKind::Kind(_)), "{name}");
        assert!(kt.dtype.packed_record().is_some(), "{name}");
    }
}

#[test]
fn a_variable_keeps_its_effective_sign_where_no_keyword_is_written() {
    // `v` writes no keyword and is signed: `signed` is not a function of the keyword
    // and the kind (`crates/cli/tests/decl_type_sign.rs` runs it).
    let su = unit(
        "module top; typedef logic signed [3:0] st; st v = 4'hF; logic [1:0][3:0] w;\n\
         typedef struct packed { logic [1:0][3:0] m; bit b; } s_t;\n\
         typedef logic [7:0] b_t; endmodule\n",
    );
    let body = module_body(&su, "top");
    let v = net(&body, "v");
    assert!(v.dtype.signed);
    assert_eq!(v.dtype.signing(), None);
    assert_eq!(v.dtype.kind(), k(NetVarKind::Logic));
    assert_net_family(&v.dtype);
    let w = net(&body, "w");
    assert_eq!(w.dtype.packed_dims().len(), 1);
    assert_net_family(&w.dtype);
    for it in &body {
        if let ModuleItem::Typedef(td) = it {
            match &td.kind {
                TypedefKind::Struct { members } => {
                    for m in members {
                        assert_net_family(&m.dtype);
                    }
                    assert_eq!(members[0].dtype.packed_dims().len(), 1);
                    assert_eq!(members[1].dtype.kind(), k(NetVarKind::Bit));
                }
                TypedefKind::Alias { dtype } => assert_net_family(dtype),
                TypedefKind::Enum { .. } => {}
            }
        }
    }
}

#[test]
fn a_port_records_its_net_or_var_keyword_or_none() {
    let su = unit(
        "module m(input a, input wire b, output logic c, input logic [1:0][3:0] d); endmodule\n\
         module n(x, y); input [3:0] x; output reg y; endmodule\n\
         module f; function logic [3:0] fn(input int i, input logic [1:0][3:0] j, k);\n\
         return 4'd0; endfunction endmodule\n",
    );
    let TopItem::Module(m) = &su.items[0] else {
        panic!()
    };
    let PortList::Ansi(ports) = &m.ports else {
        panic!()
    };
    let kinds: Vec<_> = ports.iter().map(|p| p.dtype.kind()).collect();
    assert_eq!(
        kinds,
        [
            KI,
            k(NetVarKind::Wire),
            k(NetVarKind::Logic),
            k(NetVarKind::Logic)
        ]
    );
    assert_eq!(ports[3].dtype.packed_dims().len(), 1);
    for p in ports {
        // The port family: no NotRecorded kind, no keyword, packed dims kept.
        assert_ne!(p.dtype.kind(), KN);
        assert_eq!(p.dtype.signing(), None);
        assert!(p.dtype.packed_record().is_some());
    }
    let body = module_body(&su, "n");
    let decls: Vec<_> = body
        .iter()
        .filter_map(|it| match it {
            ModuleItem::PortDecl(d) => Some(d),
            _ => None,
        })
        .collect();
    assert_eq!(
        decls.iter().map(|d| d.dtype.kind()).collect::<Vec<_>>(),
        [KI, k(NetVarKind::Reg)]
    );
    for d in decls {
        assert_eq!(d.dtype.signing(), None);
        assert_eq!(d.dtype.packed_record(), Some(&[][..]));
    }
    let body = module_body(&su, "f");
    let ModuleItem::Func(fd) = body
        .iter()
        .find(|it| matches!(it, ModuleItem::Func(_)))
        .unwrap()
    else {
        panic!()
    };
    // The return family: nothing but the sign and the range is recorded.
    assert_eq!(fd.dtype.kind(), KN);
    assert_eq!(fd.dtype.signing(), None);
    assert_eq!(fd.dtype.packed_record(), None);
    // The formal family: the keyword or none, the packed dims flattened (not recorded).
    let kinds: Vec<_> = fd.ports.iter().map(|p| p.dtype.kind()).collect();
    assert_eq!(
        kinds,
        [
            k(NetVarKind::Int),
            k(NetVarKind::Logic),
            k(NetVarKind::Logic)
        ]
    );
    for p in &fd.ports {
        assert_eq!(p.dtype.signing(), None);
        assert_eq!(p.dtype.packed_record(), None);
    }
}
