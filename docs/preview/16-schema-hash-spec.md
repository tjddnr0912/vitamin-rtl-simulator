# 16 · SchemaHash 명세 (G2)

> **Status:** PR1-구현 확정 설계 (verbatim target). `#[derive(SchemaHash)]` proc-macro 크레이트 `vita-artifact-derive` + 런타임 trait 크레이트 `vita-schema`.
> **Grounded in:** `14-staged-artifacts.md` §1(lines 117–164), §5(lines 472–532); `03-build-and-portability.md`(deps 61–82, dep graph 87–108); `13-diagnostics-and-logging.md`(line 182–184).
> **Ecosystem (검증 2026-06-02):** postcard-schema(#276), borsh `BorshSchema`(#92), scale-info(`TypeId` 비안정), facet/abi_stable, serde-reflection, typeshare. `blake3::hash`는 **`const fn` 아님**(docs.rs 확인) → `OnceLock` 런타임 산출. `module_path!()`는 토큰 텍스트 위치에서 전개 → 생성 코드에 토큰으로 방출해야 user-crate 경로가 됨.

---

## 목적 / 범위

`SCHEMA_HASH`는 sim-ir/hdl-ast serde 타입들의 **구조적 형상(structural shape)** 전체를 하나의 blake3 32-byte 값으로 응축한다. 보장하는 단 하나의 명제:

> **루트(`sim_ir::SimIr`)에서 타입-도달 가능한 형상이 한 군데라도 바뀌면 — 필드 추가/삭제/재배열/타입변경, enum variant 추가/삭제/재배열, serde 속성 추가/변경 — 루트 해시가 반드시 flip 하고, 그 결과 기존 모든 `.velab`/`.vu`가 즉시 무효(incompatible-tool 하드 에러)가 된다.**

> **루트 = `sim_ir::SimIr` (M3 재루트, doc 17).** G2 설계 시점엔 `Process`를 예시 루트로 썼으나, `Process`의 타입-도달 폐포는 cross-arena 엣지가 전부 `u32` 인덱스라 `Expr`/`Stmt`/`NetVar`/`ConstVal` arena에 **도달하지 못한다**. arena를 `Vec`로 by-value 보유하는 `SimIr`만이 전 백본을 동결한다. 아래 본문의 `<Process>` 예시는 SuspendState-클러스터 전파 트레이스(여전히 유효한 sub-pin)이며, **호환성 게이트 루트는 `SimIr`**다. `SimIrRoot`는 곧 `sim_ir::SimIr`.

범위 안:
- **형상**: 필드명 + 필드 타입 + enum variant 형태 + 명시 discriminant + array 길이 `N`.
- **serde 속성**: `rename`/`rename_all`/`skip`/`skip_serializing_if`/`with`/`default`/`flatten`/`tag`/`content`/`untagged`/`transparent`/`deny_unknown_fields`/`alias`/`other`. (serde 속성은 Rust 형상을 안 바꾸고 postcard **wire**만 바꿀 수 있으므로 반드시 포함.)

범위 밖 (의도적):
- **값(value) 그래프** — 런타임 cyclic, 형상과 무관.
- **`diag` 계열** — `Severity`/`MsgCode`/`diag::Frame`/`Diagnostic`/`LogEvent`/span/`SourceLoc` (line 102, 183: sim-ir 코어는 span-free·SchemaHash-clean). 레지스트리에 등장하지 않음.
- **`with=` 모듈 내부 인코딩 변경** (경로는 그대로인데 내부만 바뀌는 경우) — Layer 1이 못 잡음. **Layer 3 serde-reflection이 전담** (아래 명시).

> **경계 명시 (critique GAP #3 반영):** Layer 1(이 derive)은 **완전한 wire 오라클이 아니다.** `with=` 모듈의 내부 인코딩이 경로 변경 없이 바뀌면 Layer 1은 침묵한다. 그 클래스는 Layer 3 RON 골든에서만 잡힌다. §serde 속성 커버리지는 postcard-schema #276 갭의 *대부분*을 닫지만 *전부*는 아니다.

---

## 타입 그래프 = acyclic DAG

**주장: `Process` 루트의 타입-도달 그래프는 유한 acyclic DAG다. (값 그래프는 cyclic.)**

근거 — §1 lines 126–164의 모든 필드를 전수 보행:

| 타입 | inter-node 엣지 | 자기참조? |
|---|---|---|
| `Process` | `body:Vec<BasicBlock>`, `entry:u32`, `suspend:SuspendState`, `sensitivity:Sensitivity` | 없음 |
| `SuspendState` | `resume_pc:u32`, `locals:Vec<FourState>`, `join_state:JoinState`, `wake_key:WakeKey`, `call_stack:Vec<Frame>`, `frame_arena:Vec<FourState>` | 없음 |
| `JoinState` | `parent:Option<u32>`, `children:Vec<u32>`, `detached:Vec<u32>`, `flags:ProcFlags` | 없음 |
| `Frame` | `return_pc:u32`, `callee_entry:u32`, `locals_base:u32`, `locals_len:u32`, `is_automatic:bool` | **없음** (`Box<Frame>`/`Vec<Frame>` 부재 — call stack은 `SuspendState`가 `Vec<Frame>`로 보유) |
| `WakeKey` | `cond:WakeCond`, `region:RegionTag`, `tie_break:u32` | 없음 |
| `WakeCond` | `Edge{net:u32,kind:EdgeKind}`, `Level{nets:Vec<u32>}`, `WaitTrue{expr:u32}`, `TimeAbs{tick:u64}`, `NamedEvent{ev:u32}`, `Join{join_ref:u32}` | 없음 |
| `ProcFlags(u8)`, `RegionTag` | (leaf) | 없음 |

**모든 inter-node 엣지가 `u32`/`u64`/`Option<u32>`/`Vec<u32>` arena/table 인덱스다 — 노드 타입에 대한 by-value 중첩 참조가 단 하나도 없다.** 따라서 타입-레벨 back-edge = 0, 타입-도달 그래프는 유한 acyclic. (반면 값 그래프는 "프로세스가 다른 프로세스를 테이블 인덱스로 깨움" → cyclic. 형상 해시는 값과 무관하므로 영향 없음.)

**함의:** scale-info/facet/abi_stable가 진짜 재귀를 위해 쓰는 fn-pointer 간접화가 **불필요**하다 — 부모 const에 자식 *Shape*를 박지 않고 자식 *이름 문자열*만 박기 때문. const/registry 합성이 유한 종료한다.

**그래도 visited-set 가드를 유지한다 (보험, critique 승인).** 두 가지 비-장식적 이유:
1. **dedup이 cycle 없이도 load-bearing.** `FourState`는 `locals`·`frame_arena` 양쪽에서 도달, `u32`/`ProcFlags`는 다수 부모에서 도달. `insert_once`의 조기 반환이 DFS를 bound 한다.
2. **미동결 타입의 문서화된 escape hatch.** `BasicBlock`/`Stmt`/`Expr`/`FourState`/`Sensitivity`/`EdgeKind` 본문은 §1에 미명세(line 162는 `Terminator`/`JoinKind` 이름만 줌). 만약 `Expr::Binop{lhs:Box<Expr>}`로 자기재귀가 되면 `visited`가 보험→load-bearing으로 승격. 지금 `BTreeSet` 가드를 두는 것은 알려진-대기 freeze에 대한 싼 보험이며 over-engineering 아님.

---

## SchemaShape trait + 합성 메커니즘

### 중심 긴장과 해소

proc-macro는 derive 한 번에 **한 타입의 syn AST만** 본다 — 필드명 + 필드 타입 *경로 토큰*(`JoinState`, `Vec<u32>`, `Option<u32>`)을 보지, 가리키는 타입의 *형상*은 못 본다. 따라서 순수-syn derive는 `SuspendState` 전개 시 `JoinState`의 필드를 inline 할 수 **없다**. 교차-타입 합성은 반드시 런타임/trait 메커니즘을 거쳐야 한다(§5 "참여 타입 레지스트리"). 모든 조사 대상 크레이트가 동의: borsh(`add_definitions_recursively`), scale-info(`MetaType` fn-ptr), postcard-schema(`<F as Schema>::SCHEMA` static), facet(fn-ptr Shape).

**채택 모델 = borsh의 정렬 레지스트리 쓰기 (반환 문자열 ✗, 직접 hasher feed ✗).** 각 타입은 (a) 자기 *local* 형상 const 문자열, (b) 각 필드 타입의 `register`로 재귀하는 메서드를 방출. 합성은 공유 정렬 레지스트리에 intern. 이유:
1. **dedup이 필수이고 결정적이어야 함.** 반환-문자열 모델은 DAG에서 `FourState`/`u32`를 매 등장마다 재전개 → super-linear + 두 개의 동등-유효 정규형(drift 유발). 레지스트리는 타입명 사전순(BTreeMap)으로 한 번만 intern (§5 line 488 "레지스트리는 타입명 사전식 정렬로 정규화" verbatim).
2. **직접 hasher feed는 *문자열* 자체의 cross-platform diff를 막음.** §5는 정규 문자열이 byte-identical이길 원하고 Layer 3은 human-reviewable 산출물을 원함 → 전체 정규 문자열을 먼저 materialize 후 해시.
3. **Occupied-entry assert = 공짜 충돌 탐지** (borsh `add_definition`). 두 타입이 같은 FQ 이름에 다른 형상으로 등록하면 panic (borsh-rs #92 failure mode 차단).

leaf-up Merkle 해싱(각 타입 해시 = `blake3(local ++ child_hashes)`)은 **기각**: 3-OS 안정·자연 dedup이긴 하나 (a) human-reviewable 문자열 상실, (b) 자식 해시 flip이 불투명 cascade — 개발자가 *어느 필드*가 바뀌었는지 못 봄. flat-registry-string은 `--dump`로 정확한 diff 라인을 보여줌.

### 크레이트 토폴로지 (critique FATAL #1 반영 — **수정됨**)

> **수정 (치명적이었던 dependency cycle 제거):** trait를 `vita-artifact`에 두면 안 된다. `03` line 92가 `vita-artifact ──► sim-ir`를 명시하므로, `sim-ir`가 `SchemaShape`를 impl 하려고 `vita-artifact`에 의존하면 `sim-ir → vita-artifact → sim-ir` **cargo 순환** → 컴파일 불가. trait는 **새 leaf 크레이트 `vita-schema`**에 두고, `sim-ir`/`hdl-ast`가 이를 의존한다(`03` line 98/108의 leaf 패턴과 동형). **워크스페이스 production 크레이트 14 → 15** (03/04 동기 필요).

```
hdl-ast / sim-ir     ──► vita-artifact-derive   (the derive; 03 line 98)
hdl-ast / sim-ir     ──► vita-schema            (SchemaShape trait + ShapeRegistry + blake3)
vita-artifact        ──► hdl-ast, sim-ir, vita-schema, vita-artifact-derive, ...
vita-schema          ──► blake3                 (leaf — workspace 의존 0; 03 line 108 leaf 목록에 합류)
vita-artifact-derive ──► syn / quote / proc-macro2  (leaf — 불변)
```

- `vita-artifact-derive`(proc-macro)는 trait를 정의할 수 없다(proc-macro 크레이트는 매크로만 export). 그래서 trait는 별도 런타임 크레이트.
- `blake3`는 **`vita-schema`**에 산다 (`vita-artifact` 아님).
- `vita-artifact`는 stamp 시점에 `vita_schema::schema_hash::<SimIrRoot>()`를 읽어 헤더에 박는다.

### trait 시그니처 (크레이트 `vita-schema`)

```rust
//! crate vita-schema — leaf, deps: blake3 only.
//! `#[derive(SchemaHash)]`가 모든 참여 sim-ir / hdl-ast 타입에 구현.

pub trait SchemaShape {
    /// 이 타입의 rename-STABLE 정규 레지스트리 키.
    /// = concat!(module_path!(), "::", "<TypeIdent>") — FQ인 이유·Rust ident(serde rename 아님)인 이유는 결정성 규칙 참조.
    fn schema_name() -> &'static str;

    /// 이 타입의 LOCAL 형상만 (자기 필드/variant + 자기 serde 속성), 자식 타입은 schema_name()으로 *참조*(inline ✗).
    /// derive가 컴파일 타임에 구운 `const &'static str`. 런타임 할당 0.
    fn local_shape() -> &'static str;

    /// 이 타입을 등록하고 모든 필드/variant-payload 타입으로 재귀.
    /// borsh add_definitions_recursively 미러. 유일하게 제어흐름 있는 메서드. (본문은 derive 생성)
    fn register(reg: &mut ShapeRegistry);
}

/// order-stable 레지스트리. HashMap/HashSet 일절 없음 (§5 line 489 하드 금지).
pub struct ShapeRegistry {
    /// name -> local_shape. BTreeMap ⇒ 사전순 순회 ⇒ 3-OS 안정.
    entries: alloc::collections::BTreeMap<&'static str, &'static str>,
    /// DFS 종료 + cycle 가드. 절대 HashSet 아님.
    visited: alloc::collections::BTreeSet<&'static str>,
}

impl ShapeRegistry {
    /// 루트에서 도달 폐포를 몰고, canonicalize + 해시. 루트당 한 번 (lazy).
    pub fn schema_hash<T: SchemaShape>() -> [u8; 32] {
        let mut reg = ShapeRegistry::new();
        T::register(&mut reg);                  // acyclic 타입 DAG DFS
        let canonical = reg.canonical_string();  // 결정적 concat
        blake3::hash(canonical.as_bytes()).into()
    }
}
```

### 생성되는 `register` 본문 (타입당)

```rust
fn register(reg: &mut ShapeRegistry) {
    // 1. 이 타입 local 형상 삽입(가드). 이미 visited면 false 반환 → 즉시 종료(dedup + cycle 가드).
    if !reg.insert_once(Self::schema_name(), Self::local_shape()) {
        return;
    }
    // 2. 각 참여 필드/variant-payload 타입으로 SOURCE ORDER 재귀.
    //    매크로 타임에 이 타입 AST의 distinct user-type마다 1줄 방출.
    //    prim/Option/Vec/BTreeMap/array/tuple of prims는 종료(재귀 ✗).
    <sim_ir::ProcFlags as SchemaShape>::register(reg);   // 예: JoinState용
    // ... distinct 자식 user 타입마다 1줄, 매크로 타임 Vec-dedup ...
}
```

```rust
fn insert_once(&mut self, name: &'static str, shape: &'static str) -> bool {
    if !self.visited.insert(name) {              // BTreeSet::insert — 이미 있으면 false
        // 충돌 감사: 같은 이름 재등록은 IDENTICAL 형상이어야 함. 아니면 두 타입이 이름 충돌(borsh-rs #92).
        // release 빌드에서도 살아남도록 plain assert_eq! (debug_assert ✗).
        assert_eq!(self.entries.get(name), Some(&shape),
            "SchemaHash 이름 충돌: {name} 가 서로 다른 두 형상으로 등록됨");
        return false;
    }
    self.entries.insert(name, shape);            // BTreeMap — 이름 정렬
    true
}
```

> **충돌 체크는 `assert_eq!`** (release에서 컴파일 아웃되는 `debug_assert_eq!` ✗). 두 `Frame` 충돌은 correctness invariant이므로 release CI에서도 panic 해야 한다.

**매크로가 매크로 타임에 계산하는 것:** 타입 syn AST에서 임의 필드/variant-payload `<texpr>`에 등장하는 *user 타입 경로* 집합 수집(`Option`/`Vec`/`BTreeMap`/`BTreeSet`/array/tuple을 peel 해 leaf user 이름 도달), **first-appearance source order**로 dedup(`Vec` + 선형스캔 멤버십 — 매크로 안에서도 `HashSet` 금지), distinct 자식마다 `::register(reg)` 1줄 방출. prim/`str`/`String`/std 컨테이너는 재귀 종료.

### 중첩 변경이 루트로 전파되는 트레이스 (핵심 명제 검증)

`WakeCond::Edge{net:u32}` → `{net:u64}` 변경 시:

```
schema_hash::<Process>()
└─ Process::register        → entries[sim_ir::Process] 삽입, 자식 재귀
   └─ SuspendState::register → entries[sim_ir::SuspendState] 삽입
      └─ WakeKey::register   → entries[sim_ir::WakeKey] 삽입  (SuspendState.wake_key)
         └─ WakeCond::register → entries[sim_ir::WakeCond] = "...Edge{#[]net:u64,...}"  ← 변경된 라인
```
`WakeCond`의 local_shape가 `u32`→`u64`로 바뀜 → `canonical_string()`이 그 레지스트리 엔트리를 concat → blake3 flip. 경로 `Process→SuspendState→WakeKey→WakeCond`는 §1에 실재(검증됨). `WakeKey` 라인은 `sim_ir::WakeCond` *토큰*만 담으므로 불변이지만, 본문이 자기 엔트리에서 바뀌어 루트가 flip — "이름 참조, never inline" 규칙이 전파를 가능케 하면서 유한성도 보장.

---

## 정규 shape-string 문법 (EBNF)

문법 2계층: **타입식 `<texpr>`**(필드 타입이 어떻게 렌더되는가 — derive가 syn 경로에서 읽는 leaf/container)과 **컨테이너 정의 `<cdef>`**(타입의 `local_shape()`에 굽는 본문).

### 어휘/간격 규칙

- UTF-8, 실무상 ASCII. **무의미 공백 0 — 공백/개행/탭 절대 방출 안 함**(cross-platform `\n` vs `\r\n` drift #1 원인 제거). 모든 바이트가 유의미.
- 고정 단일 ASCII 구분자: `< > ( ) { } [ ] , ; : = @ # |`. 타입명은 `::` literal.
- 분리자 `,`. 후행 분리자 절대 없음 (0-필드 struct = `{}`, 1-필드 = `{name:texpr}`).
- 정수(array 길이 `N`, discriminant)는 **10진, leading-zero 없음, underscore 없음, unsigned는 부호 없음**(`repr(i…)` discriminant는 leading `-` 가능).
- JSON/RON 중간표현 없음 — RON은 Layer-3 골든에만, Layer-1 해시 입력엔 절대 없음.

### 타입식 `<texpr>`

```ebnf
texpr        ::= prim | option | seq | map | set | array | tuple | uname

prim         ::= "u8" | "u16" | "u32" | "u64" | "u128"
               | "i8" | "i16" | "i32" | "i64" | "i128"
               | "bool" | "char" | "str" | "String"
               | "f32" | "f64"
               | "usize" | "isize"            (* 표현 가능하나 엔진/CI 거부 — 결정성 RULE 4 *)

option       ::= "Option" "<" texpr ">"
seq          ::= "Vec" "<" texpr ">"
map          ::= "BTreeMap" "<" texpr "," texpr ">"   (* 유일 허용 order-stable map *)
set          ::= "BTreeSet" "<" texpr ">"             (* GAP #3: BTreeSet 명시 production — order-stable·wire-legal·허용 *)
array        ::= "[" texpr ";" uint "]"               (* 고정 [T;N] *)
tuple        ::= "(" texpr ("," texpr)* ")"           (* (A,B,...); 0-ary는 unit "()" *)

uname        ::= fqname        (* 참여 user 타입 — 이름으로만 참조 *)
fqname       ::= ident ("::" ident)*   (* fully-qualified, 예: sim_ir::JoinState *)
uint         ::= "0" | [1-9][0-9]*
ident        ::= [A-Za-z_][A-Za-z0-9_]*
```

> **결정적 규칙 — user 타입은 `<texpr>`에서 `fqname`로 *참조*, never inline.** `SuspendState.join_state: JoinState` 렌더 시 필드 타입식은 문자열 `sim_ir::JoinState`이지, `JoinState`의 본문이 아니다. 본문은 레지스트리 키 `sim_ir::JoinState`에 한 번 산다. postcard-schema `Format::TypeName` / borsh `Declaration` 간접화와 동일 — *이것이 `JoinState` 내부 변경이 루트를 flip 시키는 이유*다.

> **`HashMap`/`HashSet` 은 production 없음** — 필드 타입 경로가 literal `HashMap`/`HashSet`이면 `<texpr>` 렌더러가 하드 `compile_error!`(§5 iteration ban을 타입 레벨로 확장). `BTreeSet`은 GAP #3 반영해 명시 허용(결정적이므로) — production 없으면 `uname`로 fall-through 해 존재하지 않는 `BTreeSet::register`로 재귀하는 버그가 났을 것.

### 컨테이너 정의 `<cdef>`

```ebnf
cdef         ::= cattrs "@" body

cattrs       ::= "#" "[" cattr_item ("," cattr_item)* "]"   (* 없으면 "#[]" *)
cattr_item   ::= "rename_all=" str_lit | "tag=" str_lit | "content=" str_lit
               | "untagged" | "transparent" | "deny_unknown_fields" | "default"

body         ::= unit_struct | newtype_struct | tuple_struct | named_struct | enum_def

unit_struct    ::= "unit"
newtype_struct ::= "newtype" "(" fattrs texpr ")"
tuple_struct   ::= "tuple" "(" elem ("," elem)* ")"
named_struct   ::= "struct" "{" field ("," field)* "}"   (* 0-필드는 "struct{}" *)

elem         ::= fattrs texpr
field        ::= fattrs ident ":" texpr            (* ident = Rust 필드 ident — RULE 5 *)

enum_def     ::= "enum" "{" variant ("," variant)* "}"  (* variant는 SOURCE order *)
variant      ::= vattrs ident disc vbody
disc         ::= "=" int_lit | ""                  (* 명시 discriminant *)
int_lit      ::= "-"? uint
vbody        ::= "" | "(" elem ("," elem)* ")" | "{" field ("," field)* "}"

fattrs       ::= "#" "[" fattr_item ("," fattr_item)* "]"   (* 없으면 "#[]" *)
vattrs       ::= "#" "[" vattr_item ("," vattr_item)* "]"
fattr_item   ::= "rename=" str_lit | "skip" | "skip_serializing_if"
               | "with=" str_lit | "default" | "flatten" | "alias=" str_lit
vattr_item   ::= fattr_item | "other"              (* GAP #3: serde(other) catch-all variant *)

str_lit      ::= '"' <utf8, " 와 \ 백슬래시 이스케이프> '"'
repr_tag     ::= "repr=" ("u8"|"u16"|"u32"|"u64"|"i8"|"i16"|"i32"|"i64"|"C"|"")
```

타입 `T`의 full `local_shape()`:

```
fqname "=" repr_tag cdef
```
예: `sim_ir::JoinState=repr=@#[]struct{...}`. 선두 `fqname "="`로 각 레지스트리 라인이 self-identifying → `--dump`/Layer-3 라인 diff 가능.

### 실제 frozen 타입 렌더

**`ProcFlags(u8)` — newtype, line 155 "bare u8과 구분" 요건:**
```
sim_ir::ProcFlags=repr=@#[]newtype(#[]u8)
```
필드 타입으로서의 bare `u8`은 `u8`로 렌더되지만, `ProcFlags`는 자기 키 아래 full `newtype(#[]u8)` 본문 → postcard 바이트가 동일해도 구조적으로 distinct (§1 line 155 요구).

**`RegionTag` — 4 unit variant, 재배열 민감:**
```
sim_ir::RegionTag=repr=@#[]enum{#[]Active,#[]Inactive,#[]Nba,#[]Monitor}
```
variant source order. `Nba`/`Monitor` swap → 부분문자열 재배열 → 해시 flip (§5 line 483).

**`JoinState` — Option/Vec/user-type 필드:**
```
sim_ir::JoinState=repr=@#[]struct{#[]parent:Option<u32>,#[]children:Vec<u32>,#[]detached:Vec<u32>,#[]flags:sim_ir::ProcFlags}
```
`flags`는 `sim_ir::ProcFlags`를 이름 참조; 본문은 별도 엔트리. `ProcFlags(u8)`→`ProcFlags(u16)`이면 `JoinState` 라인은 불변이나 `ProcFlags` 라인이 flip → 루트 flip.

**`WakeCond` — 혼합 struct-variant payload + 중첩 user enum (hard case):**
```
sim_ir::WakeCond=repr=@#[]enum{#[]Edge{#[]net:u32,#[]kind:sim_ir::EdgeKind},#[]Level{#[]nets:Vec<u32>},#[]WaitTrue{#[]expr:u32},#[]TimeAbs{#[]tick:u64},#[]NamedEvent{#[]ev:u32},#[]Join{#[]join_ref:u32}}
```
`kind:sim_ir::EdgeKind`는 이름 참조(`EdgeKind` 본문은 별도 등록). `tick:u64` vs 도처의 `u32`는 distinct 토큰 — 3-OS varint-width 계약상 반드시 달라야 함(§5 line 496).

**`SuspendState` — frozen 원자 루트 클러스터, 중복 `Vec<FourState>`:**
```
sim_ir::SuspendState=repr=@#[]struct{#[]resume_pc:u32,#[]locals:Vec<sim_ir::FourState>,#[]join_state:sim_ir::JoinState,#[]wake_key:sim_ir::WakeKey,#[]call_stack:Vec<sim_ir::Frame>,#[]frame_arena:Vec<sim_ir::FourState>}
```
`locals`·`frame_arena` 둘 다 `Vec<sim_ir::FourState>` 렌더 — 필드 타입식은 동일하나 `FourState`는 레지스트리에 **한 번만** intern. 두 필드 등장은 의도적으로 보존(필드 수·이름이 load-bearing); dedup 되는 건 `FourState`의 *정의*뿐.

---

## serde 속성 커버리지

§5 line 505–509("wire 민감성")가 단일 최중요 correctness 절. postcard-schema #276이 `#[serde(skip)]`를 빠뜨려 descriptor가 실제 postcard wire와 불일치(`UnexpectedEndOfData`)하는 버그를 보임 — vitamin은 이를 *런타임 해시 경로*에서 닫는다(typeshare 체크리스트로 교차검증).

| serde 속성 | 레벨 | wire 효과 | 렌더 슬롯 | 렌더 값 |
|---|---|---|---|---|
| `rename="X"` | field/variant | wire 필드/variant 이름(아래 note) | `fattr`/`vattr` `rename="X"` | rename 문자열 |
| `rename_all="..."` | container | 전체 필드/variant 규칙 rename | `cattr` `rename_all="snake_case"` | 규칙 문자열 verbatim |
| `skip`(+`skip_serializing`/`_deserializing`) | field/variant | **wire에서 필드 제거** — #276 버그 | `fattr` `skip` | presence |
| `skip_serializing_if="path"` | field | 조건부 wire 제거 → 값-의존 | `fattr` `skip_serializing_if` | **presence만**; 술어 경로 미해시 |
| `with="mod"` | field | (de)serialize impl 교체 → 임의 wire 변경 | `fattr` `with="mod"` | 모듈 경로 문자열 |
| `default`(+`default="path"`) | field/container | decode 시 결측 허용 → wire/compat 변경 | `fattr`/`cattr` `default` | presence (경로 미해시) |
| `flatten` | field | 부모로 inline → wire 재구조 | `fattr` `flatten` | presence |
| `tag="t"` | enum container | internally-tagged → tag 필드 추가 | `cattr` `tag="t"` | tag 문자열 |
| `tag="t",content="c"` | enum container | adjacently-tagged → 2 wire 필드 | `cattr` `tag="t",content="c"` | 두 문자열 |
| `untagged` | enum container | discriminant tag 없음 | `cattr` `untagged` | presence |
| `transparent` | container | newtype passthrough → wire=inner | `cattr` `transparent` | presence |
| `deny_unknown_fields` | container | decode strictness | `cattr` `deny_unknown_fields` | presence |
| `alias="X"` | field | decode 시 추가 이름 수용 | `fattr` `alias="X"` | alias 문자열 |
| `other` | **variant** | **catch-all variant → deserialize 의미 변경 (GAP #3)** | `vattr` `other` | presence |

**"presence만" rationale (`skip_serializing_if`/`default="path"`):** 술어/default *함수*는 코드 참조이지 *형상*이 아니다 — 하지만 *존재*는 wire-relevant(필드를 optional/조건부로 만듦). 그래서 presence + kind만 해시, 해소된 함수 본문은 미해시(매크로가 못 보고, wire 문법 안 바꾸고 바뀔 수 있음). **`with`는 경로 문자열을 해시** — `with="mod_a"` vs `"mod_b"`는 전혀 다른 wire를 만들 수 있고 경로가 매크로가 가진 유일 신호.

**postcard 하에서 `rename` note.** postcard는 non-self-describing 바이너리 — 필드 *이름*이 wire에 없으므로 순수 `rename`은 postcard 바이트를 안 바꿈. 그래도 캡처(싸고, JSON/RON `--dump` 경로 + `rename`↔`alias`/`flatten` 상호작용 future-proof). *바이너리* 임계 속성(`skip`/`with`/`flatten`/`tag`/`content`/`untagged`/`transparent`/`default`/`other`)이 절대 놓치면 안 되는 것 — 모두 커버.

> **frozen wire 타입 하드 규칙 (vitamin 정책).** §1 line 122: frozen `SuspendState` 클러스터는 "postcard 바이트 동일"·**serde 속성 없음** — 모든 필드가 `#[]` 렌더. **감사 결과:** §1 lines 125–164 전수 — `ProcFlags`/`RegionTag`/`WakeCond`/`JoinState`/`Frame`/`WakeKey`/`SuspendState`/`Process` 모두 속성-free. 문법은 production을 *유지*해 frozen 타입에 *나중에* serde 속성을 붙이면 해시가 flip 하게 함(의도적 wire 변경 → 마땅히 flip). PR1은 frozen 타입 `local_shape()`가 오직 `#[]` 슬롯만 담는지 단언하는 `cargo test` 추가(아무도 frozen 클러스터에 serde 속성을 몰래 안 붙였는지 가드).

---

## 결정성 규칙 (3-OS 바이트 동일)

1. **`HashMap`/`HashSet` 일절 금지** — `ShapeRegistry`(`BTreeMap`/`BTreeSet`), 매크로 자식-dedup(`Vec`+선형스캔) 모두. `<texpr>` 렌더러가 필드 타입 경로 literal `HashMap`/`HashSet`에 `compile_error!`. (§5 line 489 ban을 타입 레벨로 확장.) `BTreeSet`은 허용(결정적).
2. **교차-타입 순서 = `fqname` 사전순**(`str` 바이트 `Ord`, BTreeMap 순회). **`core::any::TypeId`로 절대 정렬·해시 안 함** — `Hash`/`Ord`가 rustc 릴리즈 간 비안정(scale-info 회피, Bevy #32). `TypeId`는 설계 어디에도 없음 — 정체성은 `&'static str` fqname, plain 바이트-lexicographic. (이름 순서는 어느 루트가 walk를 몰든 동일한 sub-string 산출 → encounter-order보다 strictly stronger.)
3. **타입 내 순서 = source order**, `local_shape()` literal에 매크로 타임 baked(syn 필드/variant order = `Vec` walk). 재배열 → 부분문자열 재배열 → flip (§5 line 483).
4. **primitive 폭은 literal 토큰**; `u32`≠`u64`≠`usize` as 문자열. `usize`/`isize`는 *표현 가능*(불법 도입 시 flip)하나 엔진/CI lint + frozen 클러스터엔 derive `compile_error!`로 **거부**(§5 line 496: 32/64-bit 폭 차이 → 3-OS 바이트 깸). `Vec` length-prefix는 postcard varint → 길이는 안전, 원소 타입만 해시.
5. **타입명 = fully-qualified `module_path!()::Ident`, Rust ident 사용 — serde rename 이름 절대 아님.**
   - *FQ(bare ident 아님)*: borsh-rs #92/postcard-schema 충돌 회피 — `sim_ir::Frame`(call-frame) vs `diag::Frame`(`13` line 102 데이터 모델)이 bare `Frame` 키로 충돌하면 안 됨. (`diag::Frame`은 레지스트리 밖이지만 — RULE 7 — FQ는 belt-and-suspenders.)
   - *Rust ident(serde rename 아님)*: 레지스트리 **키**가 rename-stable 해야 `#[serde(rename)]`이 키를 silently re-key/reorder(=재배열로 해시 flip, double-count)하지 않음. rename의 wire 효과는 §serde 속성의 `rename=` 슬롯에 정확히 한 번 캡처. 키는 구조적 Rust 경로 유지.
   - **⚠ 구현 필수 (트랩):** `module_path!()`는 생성 코드의 `const`에 **토큰으로 방출**(`quote!`)해야 하며, **매크로 본문에서 평가하면 안 됨**. proc-macro가 자기 안에서 `module_path!()`를 호출하면 매 타입에 `vita_artifact_derive` 경로가 박혀 wrong-but-stable FQ 키 + `Frame`/`Frame` 충돌이 silently 발생. `module_path!()`는 *토큰 텍스트 위치*(생성된 impl이 착지하는 타입의 모듈)에서 전개되어 user-crate 경로가 됨(std 문서 확인). proc-macro의 caller-modpath 직접 읽기(unstable `proc_macro_span`)와 혼동 금지.
6. **float·locale·time 미진입.** discriminant/array 길이는 10진 literal. 해시되는 "값"은 ident, 타입 토큰, serde 속성 문자열 literal, discriminant 정수, array 길이뿐.
7. **`diag` 타입은 레지스트리 밖** (`13` line 102/182–184: sim-ir 코어 span-free·SchemaHash-clean; dep graph line 98은 `hdl-ast`/`sim-ir`만 derive). `MsgCode`/`Severity`/`Diagnostic`/`LogEvent`/`diag::Frame`/span/`SourceLoc`는 참여 타입 아님 — 어떤 sim-ir 필드도 참조 안 하므로 `register` walk에 등장 안 함. span은 독립 버전드 side-table로 hash 밖.
8. **frozen set에 generic 없음.** 모든 frozen 노드 monomorphic(타입 파라미터·lifetime·const generic 없음 — span-free). **PR1 결정: generic 타입에 derive `compile_error!`** (generic 지원 미구현-예약). attack surface를 frozen set이 필요로 하는 것으로 축소. **⚠ 잔여:** `hdl-ast`는 *다른* SchemaHash deriver(line 98) — `hdl-ast` 루트가 non-generic인지 PR1 freeze 전 확인. 아니면 reject-all-generics 규칙이 `hdl-ast`를 깸. (현 frozen sim-ir set은 전부 concrete — 확인됨.)

### 정규 concat

```rust
fn canonical_string(&self) -> String {
    let mut s = String::new();
    s.push_str("vita-schema-v1\n");              // 포맷-버전 sentinel (의도적 grammar 버전 bump용 + 빈 레지스트리도 고정 baseline)
    for (_name, shape) in &self.entries {        // BTreeMap = fqname 사전순, 3-OS 동일
        s.push_str(shape);                       // 각 shape는 self-identifying (fqname "=" ...)
        s.push('\n');                            // 레코드 분리자 = '\n'(0x0A) literal, never CRLF (writeln! ✗)
    }
    s
}
```

---

## blake3 산출 + CI 골든 게이트

### 최종 해시 입력

단일 concat된 `canonical_string()`(materialized UTF-8 문자열)을 한 번 해시: `blake3::hash(canonical.as_bytes())`. leaf-up Merkle 아님(불투명 diff·reviewable 문자열 상실로 기각). 3-OS byte-identical인 이유 — 모든 byte-결정 요인이 platform-invariant:

| 결정 요인 | platform-invariant 이유 |
|---|---|
| 참여 타입 집합 | source AST + `--locked` deps 고정 |
| 교차-타입 순서 | 사전순 `str` `Ord` — 순수 바이트 비교, `TypeId` 없음, locale 없음 |
| 타입 내 순서 | syn source order, 컴파일 타임 baked |
| 타입 토큰(`u32` vs `u64`) | literal 문자열; `usize`/`isize` 폭 미진입(ban + 토큰만 등장 가능하고 거부됨) |
| 공백 | 0 방출; `\n`은 literal 0x0A |
| discriminant/array `N` | 10진 literal, platform 포맷 없음 |
| serde 속성 문자열 | syn meta에서 `str_lit`로 verbatim 복사 |
| 해시 알고리즘 | `blake3` 1.x 순수 Rust, C 없음, deterministic, `--locked` 핀 |

### blake3 = 런타임 산출, literal const 아님 (정직하게 명시)

> **§5 line 481의 letter에서 의도적으로 벗어남(intent는 보존).** §5는 `const SCHEMA_HASH: [u8;32] = blake3(shape)`를 *바라지만*, **`blake3::hash`는 `const fn`이 아니다**(docs.rs 확인 — `pub fn hash(input:&[u8]) -> Hash`). 따라서 literal `const` blake3은 오늘 컴파일 불가. 정직한 설계:
> - `local_shape()`/`schema_name()`는 진짜 컴파일 타임 `const &'static str`(단일 타입 정보는 매크로 타임에 전부 알려짐). ✔ const.
> - **해시**는 첫 접근 시 `std::sync::OnceLock<[u8;32]>`로 산출(`vita-schema`의 `static`을 `schema_hash::<SimIrRoot>()`가 first-read에 채움). `vita-artifact`가 header-stamp 시 한 번 읽어 stamp. 비용은 수-KB 문자열 1회 blake3.
> - 이는 §5의 *semantics*(빌드당 안정 32-byte 구조 키)를 보존하되 "no codegen/cargo-only" 규칙을 안 깨고 blake3-const 거짓말도 안 함. **G2는 "§5 line 481을 만족"이라 주장하지 않음 — *intent*를 만족하고 *letter*에서 벗어남.**
> - **예약 upgrade 경로:** literal 컴파일 타임 const가 후일 강제되면 blake3을 *같은 정규 문자열* 위의 `const fn` 해셔로 교체 가능. 단 이는 **순수 drop-in 아님** — 정규 문자열 자체의 const-fn 조립이 `const fn` BTreeMap-정렬 `&'static str` concat을 요구하는데, 현 stable Rust에 const 힙 할당·const BTreeMap 순회가 없어 const-sortable 이름 리스트도 필요. *plausible 하나 deferred.* 정규 문자열이 계약, 해시 함수는 그 뒤 구현 디테일.

### CI 골든 게이트

**골든 #1 — 2-플랫폼 `SCHEMA_HASH` 동일성 (§5 line 490–491).** 특수 harness 없는 plain `cargo test`:
```rust
#[test]
fn schema_hash_is_pinned() {
    const EXPECTED: &str = "…64 hex…";  // frozen sim-ir 루트의 커밋된 기대 해시
    let got = vita_schema::ShapeRegistry::schema_hash::<sim_ir::SimIr>();   // M3 루트 (doc 17); Process는 sub-pin
    assert_eq!(hex::encode(got), EXPECTED,
        "SCHEMA_HASH 변경 — frozen sim-ir 타입의 형상/serde 속성이 이동.\n\
         의도적이면: 모든 .velab 무효 → format_version bump + 골든 갱신.");
    // (현재 골든 컨테이너 format_version = 8. v2→8 사이 의도적 re-freeze: real(v3)·#delay ExprId(v4)·
    //  NBA transport delay+dyn array/queue/assoc(v5)·queue insert/assoc iter/string key(v6)·
    //  casez/casex·$random·file I/O·readmem·package·string(v7)·WaitCause::Fork wait-fork(v8).)
}
```
모든 CI OS/arch(x86_64/aarch64 × linux-gnu/apple-darwin)에서 실행. 정규 문자열이 byte-identical(위)이므로 해시 동일 → *같은* `EXPECTED` literal이 전 플랫폼 통과 — 그것이 2-플랫폼 계약(§5 line 491). platform-의존 해시면 최소 한 runner 실패.

**골든 #2 — 정규-*문자열* 골든 (더 강한 diff).** full `canonical_string()`을 골든 텍스트 파일로 커밋, `cargo test`에서 diff. frozen 필드 이동 시 개발자가 정확히 바뀐 레지스트리 엔트리의 **라인 diff**를 봄(예: `sim_ir::JoinState=…flags:sim_ir::ProcFlags` → `…flags:sim_ir::ProcFlags2`), 두 32-byte blob이 아님. leaf-up Merkle 대신 materialized 문자열을 택한 이유.

**Layer 3 — `serde-reflection` RON 골든 (§5 line 521–525, 교차검증).** `dev-dependencies` 전용(`serde-reflection=0.6` — 0.4는 stale, `03` 참조) `cargo test` — 런타임 tracer를 sim-ir 타입에 돌려 산출 `Registry`(RON)를 커밋 골든과 diff. Layer 1이 *못 잡는* 클래스를 잡음: syn-속성 캡처가 놓친 wire 변경(§serde 체크리스트 밖 serde 동작, 또는 *`with=` 모듈 내부* 인코딩이 경로 변경 없이 바뀐 경우). serde-reflection은 *실제 serde derive*를 돌려 `rename`/`skip`/`with`/`default`/`flatten`/`tag`를 자동 반영 → human review의 authoritative wire 오라클. Layer 1 될 수 없음(샘플 값 필요, const stamp 불가, codegen 필요 → "no codegen" 위반). 역할 분담(§5 line 525): Layer 1 = 런타임 staleness 게이트, Layer 3 = 의도적-포맷 human review. 함께 shape 편집 + wire 편집 커버.

**개발자가 frozen 필드를 바꿨을 때 보는 것** (`Frame.return_pc: u32`→`u64`):
1. `cargo test schema_hash_is_pinned` **실패**(해시 flip).
2. 정규-문자열 골든 **실패** — `sim_ir::Frame=…struct{#[]return_pc:u32,…}` → `…return_pc:u64,…` 정확 라인.
3. Layer 3 RON 골든 **실패** — `Frame` `return_pc` `U32`→`U64`.
4. 셋 다 무시하고 ship 하면, 신 바이너리가 구 `.velab` decode 시 **clean "incompatible tool, recompile" 에러**(Layer 1 런타임 게이트), never silent misparse(§5 line 484–485). 합법 진행은 의도적 atomic re-freeze 처리(§1 line 121/§5 line 495): `format_version` bump, 전 `.velab` 재생성, 두 골든 갱신 — **재생성은 sanctioned 스위치 `REGEN_GOLDEN=1 cargo test -p sim-ir`로**(canonical txt·serde-reflection RON을 다시 쓰고 새 핀 해시를 출력; 2026-06-10 v4 bump에서 신설).

**런타임 게이트 (Layer 1).** `vita-artifact`가 `schema_hash::<SimIrRoot>()`를 stamp 시 한 번 읽어 `.velab`/`.vu` 헤더에 박음. decode 시 헤더 `schema_hash` ≠ 현 빌드 해시 → **하드 에러, silent reuse 없음**(§5 line 528). 정책은 version-GATE(refuse-and-rebuild), migrate 아님(§5 line 531–532).

---

## PR1 구현 체크리스트

- `vita-artifact-derive`: `[lib] proc-macro = true`; deps `syn=2(full)`/`quote=1`/`proc-macro2=1`만(빌드그래프 leaf, `03` line 99/108). **`blake3` 없음**(derive는 *문자열 const* 방출, 해시 아님).
- **신규 leaf 크레이트 `vita-schema`** (FATAL #1): `SchemaShape` trait + `ShapeRegistry`(`BTreeMap`/`BTreeSet`) + `schema_hash<T>()` + `OnceLock` 캐시. dep `blake3=1`. `sim-ir`/`hdl-ast`가 이를 의존. **`vita-artifact`에 trait 두지 말 것**(`sim-ir → vita-artifact → sim-ir` 순환). **워크스페이스 production 14→15** (03/04 동기 필요).
- Derive 전개: 타입당 `schema_name()`(`concat!(module_path!(),"::","Ident")` — **토큰 방출, 매크로 내 평가 금지**), `local_shape()`(AST + serde 속성으로 빌드한 §정규문법 const 문자열), `register()`(source-order 자식 `register` 호출 DFS, 매크로 타임 `Vec`-dedup).
- Compile-error 가드: (1) generic 타입, (2) `HashMap`/`HashSet` 필드 타입, (3) frozen 클러스터의 `usize`/`isize`.
- `BTreeSet<T>` production 포함(GAP #3) — 없으면 user-type fall-through 버그.
- `serde(other)` variant 속성 캡처(GAP #3).
- 충돌 체크는 `assert_eq!`(release 생존), **`debug_assert_eq!` 아님**.
- `#[derive(SchemaHash)]`를 full frozen 폐포에: `Process`/`SuspendState`/`JoinState`/`Frame`/`WakeKey`/`WakeCond`/`RegionTag`/`ProcFlags`/`Terminator` + (확정 시) `BasicBlock`/`Stmt`/`Expr`/`FourState`/`Sensitivity`/`EdgeKind`.
- 테스트: `schema_hash_is_pinned`(해시 골든), 정규-문자열 골든, frozen-타입-serde속성-없음 가드, `dev-dependencies` `serde-reflection` RON 골든(Layer 3).

---

## 잔여 리스크

1. **미동결 타입 freeze 게이트 (하드, 비-deferrable).** `BasicBlock`/`Stmt`/`Expr`/`FourState`/`Sensitivity`/`EdgeKind` 본문이 §1에 미명세(line 162는 `Terminator`/`JoinKind` 이름만). frozen blast radius 안(`body`의 `Terminator`, line 494가 `BasicBlock`→`Stmt`→`Expr`를 끌어들임). **PR1은 이들이 명세 + index-edge(`expr:u32`/`net:u32`) 사용 검증 전까지 골든 해시 freeze 불가.** 만약 하나라도 `Box<Self>`/`Vec<Self>`(예: `Expr::Binop{lhs:Box<Expr>}`)면 진짜 자기재귀 → cycle 가드가 보험→load-bearing 승격. 전부 `#[derive(SchemaHash)]` 필수.
2. **`hdl-ast` generic 확인.** reject-all-generics 규칙(RULE 8)이 sim-ir엔 OK(전부 monomorphic 검증)이나, `hdl-ast`는 *다른* SchemaHash deriver(line 98). `hdl-ast` 루트가 non-generic인지 PR1 freeze 전 확인 — 아니면 PR1이 `hdl-ast`를 깸.
3. **Layer 1 ≠ 완전 wire 오라클.** `with=` 모듈 내부 인코딩이 경로 변경 없이 바뀌면 Layer 1 침묵 → Layer 3 RON 골든에서만 잡힘. §serde 커버리지는 #276 갭의 대부분을 닫되 전부는 아님. 경계를 문서·리뷰에서 crisp 하게 유지.
4. **`module_path!()` 방출 트랩.** 매크로 본문에서 `module_path!()`를 호출하면 모든 타입에 `vita_artifact_derive` 경로 → wrong-but-stable FQ 키 + `Frame`/`Frame` 충돌이 silently 발생. 반드시 생성 `const`에 토큰으로 방출. (RULE 5에 명시했으나 잔여 리스크로 재핀.)
5. **`OnceLock` ≠ literal const.** §5 line 481 letter에서 의도적 deviation. const-fn upgrade 경로는 plausible 하나 const-fn 문자열 조립(const BTreeMap 정렬 concat) 필요 → deferred. 정규 문자열이 계약.
6. **충돌 assert가 PR1에서 누락되면** 두 distinct 타입이 같은 FQ 이름에 다른 형상으로 silently alias(borsh-rs #92). `assert_eq!` 필수 — release 생존.

---

### 검증 확인 (소스 대조)
- **타입-도달 그래프 acyclic** — `Process` 루트 모든 inter-node 엣지가 `u32`/`u64`/`Option<u32>`/`Vec<u32>` arena 인덱스; `Frame` self-edge 없음 (§1 lines 126–164 전수).
- **frozen sim-ir 타입 generic 없음** (전부 concrete).
- **frozen 타입 serde 속성 없음** (전부 `#[]` 렌더).
- **`diag`/`MsgCode`/span 타입은 레지스트리 밖** (`13` line 102/183).
- **`vita-artifact` trait-home은 순환** (`03` line 92 `vita-artifact ──► sim-ir`) → **`vita-schema` leaf로 수정** (line 98/108 패턴).
- **blake3 1.x는 `const fn` 아님** (`03` line 69 `blake3="1"`; docs.rs) → `OnceLock` 산출; const-fn 해셔는 예약 upgrade.

## Sources
- 14-staged-artifacts.md §1(117–164, 동결 노드), §5(472–532, schema 해시 메커니즘)
- 03-build-and-portability.md (deps 61–82, dep graph 87–108 — vita-schema leaf 추가 필요)
- 13-diagnostics-and-logging.md (line 102, 182–184 — diag는 hash 밖)
- 생태계(2026-06-02 검증): borsh `BorshSchema`(rust #92), postcard-schema(#276), scale-info(`TypeId` 비안정), serde-reflection 0.6(0.4는 stale), typeshare, facet/abi_stable
