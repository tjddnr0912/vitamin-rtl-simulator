# 17 · sim-ir IR 백본 동결 (M3)

> **Status:** 동결 sign-off 스펙 (2026-06-04 승인). 구현자가 Rust를 verbatim 전사한다. PR1-B가 `SuspendState` 폐포를 동결했고, M3는 **나머지 sim-ir 백본**(`Expr`/`Stmt`/`Lvalue`/`Terminator` 본문 + net/expr arena + `Process`/`SimIr` 루트)을 동결해 **골든 루트를 `SimIr`로 락**한다.
> **Grounded in:** `01-goals-and-scope.md`(Phase-1 freeze 표), `06-simulation-engine.md`(프로세스 모델 SD1–SD5, line ~129–147), `14-staged-artifacts.md` §1(frozen node·SimIr root, line ~85–164)/§5, `16-schema-hash-spec.md`(SchemaHash 결정성 규칙).
> **검증:** 9-에이전트 설계 스파이크(선행연구 iverilog/Verilator/yosys/CXXRTL + MVP 서브셋 oracle + 제약 oracle) → 설계 → 4-렌즈 적대 검증(커버리지/결정성/프로세스-fit/진화성) → 최종. BLOCKER 2(루트, Fork/Call)·MAJOR 6 해소.

---

## 0. 동결 원칙

- **골든 루트 = `schema_hash::<sim_ir::SimIr>()`** (NOT `Process`). `Process`의 타입-도달 폐포는 모든 cross-arena 엣지가 `u32` 인덱스라 `Expr`/`Stmt`/`NetVar`/`ConstVal`에 **도달하지 못한다** → Process 루트로는 arena가 hash-밖에서 evolvable. `SimIr`가 arena를 `Vec`로 **by-value 보유**하므로 그 폐포가 전 백본 + (`Vec<Process>` 경유) PR1-B 폐포 전체를 덮는다. doc 16의 `Process` vs `SimIrRoot` 불일치도 이로써 `SimIr`로 통일.
- **모든 inter-node 엣지 = `u32`/`u64`/`Option<u32>`/`Vec<u32>` arena 인덱스.** `Box<Self>`/`Vec<Self>` 자기재귀 0 → 타입-도달 그래프 유한 acyclic. 재로드 시 포인터 fixup 0(doc 14:104–109).
- **결정성(3-OS 바이트 동일):** `HashMap`/`HashSet` 일절 금지, `BTreeMap`/`BTreeSet`/`Vec`만. **`usize`/`isize`/`f32`/`f64` 금지**(아래 §8 derive 가드로 강제). order-stable `Vec`. span-free(위치는 노드-인덱스 키 side-table, sim-ir 밖). 전 타입 monomorphic(제네릭/lifetime/const-generic 없음). frozen 타입 serde 속성 0.
- **진화 가능:** Expr/Stmt variant 추가 = 루트 해시 flip = 의도적 re-freeze(전 `.velab` 재생성). M3는 **백본 + MVP variant 집합**을 동결; Phase-2 구문은 §10 re-freeze 슬롯.
- **이미 동결된 형상은 verbatim 재현:** `Process`/`SuspendState` 폐포(PR1-B), `Terminator::Fork{children,join,resume_bb}`·`Call{target,ret_bb}`(doc 14:162, 2026-06-02 RULE-D2 동결).

---

## 1. `Expr` — 식 enum

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum Expr {
    Const     { val: u32 },                                       // -> consts[val] : ConstVal
    Signal    { net: u32, word: Option<u32> },                    // net read; word = unpacked-array 원소 인덱스 식
    Select    { base: u32, offset: u32, width: u32, kind: SelKind }, // bit/part/indexed select
    Concat    { parts: Vec<u32> },                                // {a,b,...} MSB-first, 순서 동결
    Replicate { count: u32, value: u32 },                         // {N{x}} ; count = 상수-elaborated 식 인덱스
    Unary     { op: UnOp,  operand: u32 },
    Binary    { op: BinOp, lhs: u32, rhs: u32 },
    Ternary   { cond: u32, then_e: u32, else_e: u32 },            // ?:
    SysFunc   { which: SysFuncId, args: Vec<u32> },               // $time/$realtime/$signed/$unsigned/$clog2
    Call      { func: u32, args: Vec<u32> },                      // INLINABLE user 함수 호출 -> funcs[func]
}
```

**유효성 불변식(구조 아님 — elaboration 강제):**
- **(I-E1)** `Expr::Call.func`는 elaborator가 inline 가능한 함수여야 한다. 비-inline(재귀/불확실) 함수가 식 위치면 `E-ELAB-UNSUPPORTED`(E3023, doc 06 SD2)로 거부. frame-call(`call_stack`) 실행은 문장 위치 `Terminator::Call`로만 도달.
- **(I-E2)** `Replicate.count`는 elaboration 시 상수-foldable(Verilog 요구). 식 인덱스로 저장(`Select` width와 동형); 비상수는 elaboration 에러.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum UnOp {
    Plus, Minus, LogNot, BitNot,                                  // + - ! ~
    RedAnd, RedNand, RedOr, RedNor, RedXor, RedXnor,              // & ~& | ~| ^ ~^ (reduction)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod, Pow,                                // + - * / % **
    BitAnd, BitOr, BitXor, BitXnor,                              // & | ^ ~^  (^~ -> BitXnor 정규화)
    LogAnd, LogOr,                                               // && ||
    Lt, Le, Gt, Ge,                                             // < <= > >=
    Eq, Ne, CaseEq, CaseNe,                                     // == != === !==   (==? !=? Phase-2)
    Shl, Shr, AShl, AShr,                                       // << >> <<< >>>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum SelKind { Bit, PartConst, PartIdxUp, PartIdxDown }      // x[i] / x[m:l] / x[b+:w] / x[b-:w]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum SysFuncId { Time, Realtime, Signed, Unsigned, Clog2 }   // $random은 Phase-2
```
reduction vs bitwise는 **arity로** 구분(reduction=`UnOp`, bitwise=`BinOp`), 토큰 아님.

---

## 2. `Stmt` — BB 내 straight-line 연산 (제어흐름 없음)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum Stmt {
    BlockingAssign    { lhs: Lvalue, rhs: u32 },                  // =
    NonblockingAssign { lhs: Lvalue, rhs: u32 },                  // <=  (NBA region 스케줄)
    SysTask           { which: SysTaskId, fmt: Option<u32>, args: Vec<u32> }, // $display/.../$dump*
    Disable           { scope_kind: DisableKind, target: u32 },   // 해시된 reap-scope
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum SysTaskId {
    Display, Write, Monitor, Strobe,                             // 텍스트 출력
    Finish, Stop,                                               // sim 제어
    DumpFile, DumpVars, DumpOn, DumpOff, DumpAll,               // VCD dump family
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum DisableKind { Fork, Scope }                            // detached-only vs recursive-children reap
```

- **(D-S1)** ~~intra-assign delay 필드 없음(MVP 제외)~~ **v5 갱신(2026-06-10)**: `NonblockingAssign`이 `delay:Option<u32 ExprId>`를 가짐(transport NBA — 실행 시 평가, t+d NBA region에 값-운반 이벤트). blocking 형 `a = #d b`는 필드 없이 tmp-capture + `Delay` terminator로 lower(둘 다 구현됨). 문장-선두 `#d`/`@()`는 종전대로 terminator.
- **(D-S2)** `SysTask{which,fmt,args}`: `fmt:Option<u32>`는 `consts`의 `ConstVal{repr:StrUtf8}`(format-control 문자열)을 가리킴. `args`는 post-format 식 인덱스. format 없는 태스크(`$finish`/`$dumpvars`)는 `fmt=None`. format 문자열이 수치 operand와 **구조적으로 구분**(별도 필드 + 가리키는 const가 자기-식별)되어 3-OS 바이트 동일 `$display` 충족(doc 01:46). `$monitor`/`$strobe` 영속: 엔진이 (`fmt`,`args`) 인덱스 튜플을 보존(인덱스 안정 → 형상 변화 0).
- **(D-S3)** `Disable{scope_kind,target}`: `target`=`ProcId`(reap할 스코프), `scope_kind`=teardown 의미(doc 14 RULE-D2 inv.2: `Fork`=`detached`만, `Scope`=`children` 재귀). reap-scope는 **해시 필드**(side-metadata 아님 — doc 06:139가 해시 밖 스케줄링 로직 금지). CFG 엣지는 disabling BB의 `Goto`로.
- **(D-S4)** `ContinuousAssign`은 `Stmt` 아님 — top-level `ContAssign` arena(§7).

---

## 3. `Lvalue` — 대입 타깃

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Lvalue { pub chunks: Vec<LvalChunk> }                 // concat-LHS = 다중 chunk; 단순 = 1 chunk

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct LvalChunk {
    pub net:    u32,                                             // -> nets[net]
    pub word:   Option<u32>,                                     // unpacked-array word 인덱스 식 (None=scalar/whole)
    pub offset: Option<u32>,                                     // part-select base 식 (None=offset 0)
    pub width:  Option<u32>,                                     // None = whole net; Some(w) = 명시 width
    pub kind:   SelKind,
}
```
- **(D-L1)** whole-net write 정규형: `{net, word:None, offset:None, width:None, kind:Bit}`. `width:Option<u32>`(None=whole)로 `net_width` denormalize 회피, `SelKind` 4-variant 유지. select 방향/width는 `nets[net].msb/lsb` 조인으로 해소(arena IR이므로 self-contained 아님 — 수용).
- **enum discriminant 없음:** 아래 frozen enum 어느 것도 명시 discriminant 미사용. reorder-안전은 variant 이름+소스 위치로 보장(discriminant 토큰 아님). 명시 discriminant·고정배열 `[T;N]` 미사용 → derive whitespace 경로(F2) 미진입(§8).

---

## 4. `Terminator` — FROZEN-이름·FROZEN-형상 제어흐름 (SD3)

> **`Fork`/`Call`은 RULE-D2 원자 동결 블록(doc 14:162, 2026-06-02)에서 verbatim 재현** — M3가 정하는 게 아님. `Goto`/`Branch`/`Delay`/`Wait`/`Return` 본문만 M3 확정(SD3 §1에서 이름만 줬던 것).

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum Terminator {
    Goto   { target: u32 },                                      // -> blocks[target]
    Branch { cond: u32, then_bb: u32, else_bb: u32 },            // if + case/casez/casex -> Branch chain
    Delay  { amount: u32, region: DelayRegion, resume: u32 },    // region 베이크 (#0 => Inactive)
    Wait   { cond: WaitCause, resume: u32 },                     // 제한 cause (full WakeCond 아님)
    Fork   { children: Vec<u32>, join: u32, resume_bb: u32 },    // FROZEN VERBATIM (14:162). join_kind compile-bake.
    Call   { target: u32, ret_bb: u32 },                         // FROZEN VERBATIM (14:162). args via frame ABI.
    Return,                                                      // body 끝 / task return
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum DelayRegion { Active, Inactive }                        // #d>0 / #0

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum WaitCause {                                             // in-body @()/wait — 시간은 구조적 배제
    Edge  { net: u32, kind: EdgeKind },                          // @(posedge/negedge/edge net)
    Level { nets: Vec<u32> },                                    // @(a or b ...) / @(*)
    Expr  { expr: u32 },                                         // wait(expr)
    Named { ev: u32 },                                           // @(named_event)
}
```

- **(D-T1) Fork**: `kind:JoinKind`는 **필드 아님** — `JoinKind{All,Any,None}`은 elaboration 시 compile-bake(doc 14:163). `resume_bb`=join 충족 후 부모 PC 착지, `join`=자식 rendezvous BB. 부모는 런타임 `WakeCond::Join{join_ref}`로 arm. `join_any`/`join_none`은 lowering-only Phase-2(이름 이미 베이크 → re-freeze 불요).
- **(D-T2) Call**: `func` rename 없음(`target`=동결 이름), `args` 필드 없음. 인자 바인딩은 caller BB에서 `Call` 앞에 `Stmt::BlockingAssign`으로 callee `frame_arena` window(`Frame.locals_base..+locals_len`)에 emit(동결 `Frame` ABI, doc 06:143). 반환값은 ABI상 caller-가시 local. `Return`은 fieldless.
- **(D-T3) Delay**: `region:DelayRegion` elaboration 베이크(엔진 재유도 금지, RULE D2 inv.3). `#0→Inactive`, `#d(d>0)→Active`. suspend 시 엔진이 `WakeCond::TimeAbs{tick=now+eval(amount)}` 계산 + `wake_key.region=region`. `amount`만 런타임 eval, 라우팅은 data-independent.
- **(D-T4) Wait**: `WaitCause`는 full `WakeCond` 아닌 제한 enum → `Wait{TimeAbs}` illegal state 구조적 배제(time=`Delay`). suspend 시 엔진이 정적 `WaitCause`를 `wake_key.cond`로 **복사**(lift: `Edge→Edge`, `Level→Level`, `Expr→WaitTrue`, `Named→NamedEvent`). 프로세스-레벨 `Sensitivity`(always_*)는 §6 별도, 프로세스 entry로 re-arm.

---

## 5. Lowering 표 — 제어 구문 → Terminator

| 소스 구문 | Lowering |
|---|---|
| `if (c) T else E` | BB가 `Branch{cond:c, then_bb:T, else_bb:E}`로 종료 |
| `case/casez/casex` | `Branch` chain; arm마다 `Branch{cond:(sel ===/wildcard item), then_bb:arm, else_bb:next}`; `default`=마지막 `else_bb`. casez/casex don't-care는 `cond` 식에 materialize(item 리터럴의 x/z 자릿수와 `CaseEq`); **`nets`에 저장 안 함** |
| `for(i;c;s){B}` | init-BB `Goto`→test-BB `Branch{c, body, exit}`; body 끝 `Goto`→step-BB `Goto`→test |
| `while(c){B}` | test-BB `Branch{c, body, exit}`; body 끝 `Goto`→test |
| `repeat(n){B}` | elaborated 카운터 local(`SuspendState.locals`, in-body `@`-suspend 생존); 카운터 `Branch`; body 감소, `Goto`→test |
| `forever{B}` | body 끝 `Goto`→body-entry; exit 엣지 없음(Verilog가 내부 `@`/`#` 요구 → same-tick 무한루프 없음) |
| `begin…end` | `Goto`로 직렬 BB 연결; named-block 라벨 → side metadata(진단 전용) |
| `disable scope` | `Stmt::Disable{scope_kind,target}` 실행 후 BB의 `Goto`가 post-disable BB로 |
| `#d` | `Delay{amount:d, region:(d==0?Inactive:Active), resume:next_bb}` |
| `@(...)` / `wait(c)` | `Wait{cond:WaitCause::…, resume:next_bb}` |
| `fork…join[_any/_none]` | `Fork{children, join, resume_bb}`; join flavor는 `join`/`resume_bb` BB 그래프에 compile-bake |
| task call(문장) | `Call{target, ret_bb}` (인자 frame window에 `BlockingAssign` 선바인딩) |
| function call(식) | `Expr::Call{func, args}` — INLINABLE만; 비-inline → E3023 |
| block/body 끝 | `Return` |

**새 terminator 이름 0.** `foreach`/`unique`/`priority`/`do-while`(Phase-2)도 `Goto`/`Branch`로 lower — lowering-only.

---

## 6. `Sensitivity` · net/var 테이블 · 4-state 값 · 상수 풀

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Sensitivity { pub kind: SensKind, pub edges: Vec<EdgeTerm> } // provenance + 해소 trigger 집합

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct EdgeTerm { pub net: u32, pub kind: EdgeKind }         // PR1-B 동결 EdgeKind{Posedge,Negedge,AnyEdge} 재사용

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum SensKind { Initial, Comb, Latch, Edge, Level }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct NetVar {
    pub kind:      NetKind,                                      // wire | reg | logic | integer
    pub width:     u32,                                         // 총 비트폭 (1=scalar)
    pub msb:       u32,                                         // [msb:lsb] 상한 (post-elaboration)
    pub lsb:       u32,                                         // 하한 (msb<lsb = 역순 [0:N])
    pub signed:    bool,                                        // signed 산술 / arithmetic-shift fill
    pub array_len: u32,                                        // 1-D unpacked 원소 수 (1=배열 아님)
    pub dir:       PortDir,                                     // Input/Output/Inout/Internal
    pub init:      BitPacked,                                   // time-0 해소 4-state (reg/logic=x, net=z)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum NetKind { Wire, Reg, Logic, Integer }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum PortDir { Input, Output, Inout, Internal }

// 비트팩 4-state: 2-plane, 2비트/비트. doc 14 벡터 pack 순서는 포맷 일부.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct BitPacked {
    pub val: Vec<u64>,                                          // value plane:  ceil(width/64) words, word0 bit0=LSB
    pub unk: Vec<u64>,                                          // unknown plane: (v,u)=00→0,10→1,01→X,11→Z
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ConstVal {
    pub width:  u32,
    pub signed: bool,                                          // 's 리터럴 => signed 재해석
    pub repr:   ConstRepr,                                     // numeric vs string payload 구분
    pub bits:   BitPacked,                                     // 2-plane (string => UTF-8 바이트 팩)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum ConstRepr { Numeric, StrUtf8 }                        // 수치 4-state 리터럴 vs display format/string 리터럴
```

- **(D-N1)** `Sensitivity`=struct: uniform `edges` + 직교 `kind`. Level/comb/latch는 `EdgeKind::AnyEdge` 엔트리. `Initial`→빈 `edges`.
- **(D-N2)** `EdgeKind`는 PR1-B 동결 enum 재사용(lib.rs) — 레지스트리 단일 엔트리, 재선언 금지.
- **(D-V1)** 2-plane `(val,unk)`, 비트맵 `00→0,10→1,01→X,11→Z`, word0-bit0=LSB 정규(포맷 일부).
- **(D-V2)** `init:BitPacked` inline(자기완결 스냅샷). 자기-엣지 없음 → acyclic.
- **(D-V3)** `BitPacked`에 `width` 필드 없음 — width는 부모(`NetVar.width`/`ConstVal.width`) 소유.
- **(D-V4)** 범위 `(msb,lsb)`; 역순 `[0:N]`은 `msb<lsb`. `array_len:u32`(flat 원소 수); flat `init`이 `width*array_len` 비트. **다차원 UNPACKED 배열은 elaborate에서 row-major 평탄화(`i*s0+…`)로 단일 `array_len`+단일 word ExprId에 매핑 → IR 형상 무변경(골든 불변).** per-dim 크기/lo는 elaborate-로컬 사이드테이블에만. PACKED 다차원·동적/연관 배열만 Phase-2 `dims:Vec<u32>` re-freeze 대상.
- **(D-V5)** `NetKind`=4 MVP kind; `tri/wand/wor` → Phase-2. genvar/param/localparam은 elaboration 시 `consts`로 fold(`NetVar` 아님).
- **(D-V6)** `ConstRepr{Numeric,StrUtf8}`로 format/string 리터럴을 수치 operand와 **구조적** 구분 — `$display` format 표현 명확.

---

## 7. `BasicBlock` · `Process` · top-level arena · `SimIr` 루트

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct BasicBlock {
    pub stmts: Vec<u32>,                                        // -> stmts[..] 직렬 연산, 순서대로
    pub term:  Terminator,                                      // 정확히 1개 terminator (doc 06 불변식)
}

// PR1-B 동결 (lib.rs). 그대로 — 수정 금지.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Process {
    pub sensitivity: Sensitivity,
    pub body:        Vec<BasicBlock>,
    pub entry:       u32,
    pub suspend:     SuspendState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ContAssign { pub lhs: Lvalue, pub rhs: u32, pub delay: Option<u32> } // assign (+옵션 #delay)

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Instance { pub parent: Option<u32>, pub module: u32, pub first_net: u32, pub net_count: u32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct FuncDef { pub entry: u32, pub n_params: u32, pub locals_len: u32, pub is_task: bool }

// 골든 루트. schema_hash::<sim_ir::SimIr>()가 pinned 게이트.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct SimIr {
    pub instances:    Vec<Instance>,
    pub nets:         Vec<NetVar>,
    pub processes:    Vec<Process>,
    pub cont_assigns: Vec<ContAssign>,
    pub funcs:        Vec<FuncDef>,
    pub exprs:        Vec<Expr>,
    pub stmts:        Vec<Stmt>,
    pub blocks:       Vec<BasicBlock>,
    pub consts:       Vec<ConstVal>,
}
```

- **(D-R1)** 골든 게이트는 `schema_hash::<SimIr>()` 루트. `Process` 루트는 **sub-pin** 골든(런타임 클러스터 회귀 신호)으로 유지하되 호환성 게이트 아님.
- **(D-R2)** dump/builtin 마커는 `Stmt::SysTask`, 루트 필드 아님. 헤더 `uses_dump:bool`(doc 14:101)은 해시된 `SimIr` 본문 밖 별도 provenance.
- **(D-R3)** `Instance`=평탄 계층(`parent`, `module` 이름-ref, `[first_net, first_net+net_count)` net slice) — 진단/스코핑, 동작 없음.
- **(D-R4)** `FuncDef{entry,n_params,locals_len,is_task}` — `Call` 바인딩 최소 ABI.

### Arena 레이아웃 (인덱스 해소)

`SimIr`가 flat arena. 모든 엣지는 정확히 한 `Vec`의 인덱스, 로드 시 `vec[idx as usize]`로 해소 — `usize` cast는 resolver의 transient, **저장 필드 아님**(no-`usize` 규칙 유지).

| `SimIr` 의 Vec | 원소 | 인덱스 | 참조처 |
|---|---|---|---|
| `exprs` | `Expr` | ExprId=u32 | rhs, cond, operand, args, `Select.*`, `ContAssign.rhs` |
| `stmts` | `Stmt` | StmtId=u32 | `BasicBlock.stmts:Vec<u32>` |
| `blocks` | `BasicBlock` | BlockId=u32 | `Process.body`/`entry`, 모든 `Terminator` BB 타깃, `FuncDef.entry` |
| `nets` | `NetVar` | NetId=u32 | `Signal.net`, `LvalChunk.net`, `EdgeTerm.net`, `WaitCause` nets, `Instance` slice |
| `consts` | `ConstVal` | ConstId=u32 | `Expr::Const.val`, `SysTask.fmt` |
| `processes` | `Process` | ProcId=u32 | top-level; `Disable.target`, `JoinState.children/detached` |
| `cont_assigns` | `ContAssign` | CaId=u32 | top-level |
| `instances` | `Instance` | InstId=u32 | `Instance.parent` |
| `funcs` | `FuncDef` | FuncId=u32 | `Expr::Call.func`, `Terminator::Call.target` |

node-kind당 append-only `Vec` 1개 = 재로드 시 포인터 fixup 0.

---

## 8. derive 가드 (V-PRIM) — **동결 prerequisite**

> **현 derive는 `usize`/`isize`/`f32`/`f64`를 거부하지 않는다.** `vita-artifact-derive`의 `PRIMITIVES`에 들어 있어 정상 렌더된다. PR1 체크리스트가 주장한 "frozen 클러스터 usize/isize 거부" 가드가 **코드에 없다.** M3 결정성이 여기 걸려 있다.

**필수 수정(M3 deliverable, 골든 pin 전 선착):**
1. `render_path_type`에서 `PRIMITIVES.contains` arm **앞에** `usize`/`isize`/`f32`/`f64` reject arm 추가 → `compile_error!`(HashMap/HashSet 가드 미러). `PRIMITIVES`에서 이 4개 제거.
2. `sim-ir`에 belt-and-suspenders 테스트: `schema_hash::<SimIr>()`의 canonical 문자열이 토큰 `usize`/`isize`/`f32`/`f64`를 **하나도 안 담음** 단언.

미수정 시 stray `usize` 인덱스 필드가 작성자 머신에선 통과하고 런타임 3-OS 바이트를 컴파일 신호 없이 깬다.

**F2 경로(disc/array-len whitespace):** M3 frozen 타입 중 명시 discriminant·고정배열 `[T;N]` 사용 0 → F2 경로 미진입, F2 수정 불요.

---

## 9. vita-artifact 변경

- **루트 해시:** `SuspendState` → **`SimIr`**. 헤더 stamp·골든 게이트 모두 `schema_hash::<sim_ir::SimIr>()`. `Process`는 런타임 클러스터 sub-pin 골든으로 유지.
- **format_version 4 (2026-06-10) — 의도적 re-freeze:** `Terminator::Delay.amount`가 **ExprId 의미**로
  전환(런타임 `#delay`; u32 형상 동일이라 해시 단독으론 못 잡아 컨테이너 버전이 게이트) +
  `SysTaskId::{DumpFlush,DumpLimit}` + `Stmt::{Force,Release}` 형상 선예약 — **semantics도 같은 날 landed**(sample-once 모델,
  per-net forced 플래그가 모든 일반 write 경로를 차단; release는 net=settle 복원/var=값 유지). 루트 해시 재pin(d6d078bc…), Process 서브pin 불변(=정상 sanity),
  canonical/RON 골든 재생성(`REGEN_GOLDEN=1` 스위치 신설), 전 `.velab`/`.vu`는 FORMAT 게이트에서
  exit 2로 거부(의도된 staleness).
- **format_version 5–8 (2026-06-10~14) — 후속 re-freeze:** v5=`NonblockingAssign.delay`(NBA transport delay)
  +`NetKind` Dyn 3종(동적 배열/queue/assoc)+`SysFunc`/`SysTask` 메서드 — v6=queue `insert/delete`·assoc iter·string 키
  — v7=`BinOp::CasezEq/CasexEq`+`SysFuncId`/`SysTaskId` 다수($random·파일 I/O·readmem·bit-vector)+`NetKind::String`
  — **v8=`WaitCause::Fork`(wait fork IR)**. **현재 골든 `format_version` = 8.** SVA(concurrent assert·시퀀스·
  sampled fn·named property/sequence·multi-clock)와 wait fork **기능**은 v8 위에 **순수 IR-0 desugar + AST-flip**으로
  얹혀 `.vu` AST-해시만 재핀(SimIr 골든 무변경).
- **`format_version` bump:** M3가 전 백본 동결 → 루트 해시 by construction 변경 → 이전 모든 `.velab` 무효(decode 시 incompatible-tool 하드 에러, silent misparse 없음). M3 동결로 1회 bump; 새 `schema_hash_is_pinned` EXPECTED + canonical 골든(SimIr 루트) 커밋.
- **decode 게이트:** 정책 불변(version-GATE refuse-and-rebuild).
- **Layer-3 RON 골든:** `serde-reflection` tracer 루트를 `Process` 클러스터 → `SimIr`로 확장.

---

## 10. 잔여 리스크 + Phase-2 re-freeze 슬롯

**잔여(수용):**
1. `Select`/`LvalChunk` 비-self-contained(방향/width는 `nets[net].msb/lsb` 조인). arena IR이므로 수용.
2. `SensKind` provenance 5-variant(런타임은 ~2). 진단 가치로 수용.
3. `Const`→`consts` 간접화(작은 리터럴). uniform pool dedup으로 수용.
4. **derive 가드(§8)가 prerequisite.** `usize/isize/f32/f64` reject arm 착지 전 골든 pin 금지.
5. `with=`-클래스 wire 변경은 Layer-1 사각(doc 16 경계) — Layer-3 RON 전담. M3 frozen 타입 `#[serde(with=)]` 0이라 노출 nil.

**Phase-2 re-freeze (의도적 루트-해시 flip, 전 `.velab` 재생성):**

| Phase-2 구문 | re-freeze 위치 |
|---|---|
| ~~casez/casex 정밀(`==?`/`!=?`)~~ ✅ v7(`BinOp::CasezEq/CasexEq`) · `inside`, 스트리밍 `{>>}/{<<}`(잔여) | `BinOp` 확장 / 새 `Expr` variant |
| ~~`$readmemh`/`$fopen`/파일 I/O, `$random`~~ ✅ v7 | `SysTaskId`/`SysFuncId` 확장(`$random`=`SysFuncId`) |
| ~~`$bits`/`$countones`~~ ✅ v7 · `$signed` 확장(잔여) | `SysFuncId` 확장 |
| ~~intra-assign delay `lhs = #5 rhs`~~ ✅ v4/v5 | blocking=tmp+`Delay` terminator(v4, 필드 불요) · NBA=`NonblockingAssign.delay`(v5) |
| ~~named `event` + `->ev`~~ ✅ 2026-06-11 | **형상 0**: 카운터 desugar(64-bit Reg + `e=e+1` + AnyEdge) — `WaitCause::Named`/`WakeCond::NamedEvent`는 예약-미사용 유지 |
| net types `tri/wand/wor`, drive strength | `NetKind` 확장 / strength 필드 |
| 다차원 PACKED / 동적·연관 array | `NetVar.array_len:u32` → `dims:Vec<u32>` (UNPACKED 다차원은 elaborate 평탄화로 이미 처리됨 — re-freeze 불요) |
| struct/union/enum/typedef | `NetKind` 확장(packed struct는 `BitPacked` fit); unpacked→`dims` |
| interface ref | `Instance`-인접 arena + 새 `Expr`/`Lvalue` leaf |
| ~~SVA(concurrent assert·시퀀스·sampled fn·named property/sequence·multi-clock)·wait fork~~ ✅ v8 | **IR-0 / AST-flip만** — 합성 clocked-always 체커 desugar, `.vu` 재핀(SimIr 골든 무변경) |
| `real`/`realtime` 저장 | **✅ DONE — 의도적 sim-ir re-freeze (format_version 2→3).** 당초 "re-freeze 아님(별도 비해시 lane)" 결정을 **의도적으로 번복**: `NetKind::Real` + `ConstRepr::Real`(둘 다 fieldless) + `SysFuncId::{Rtoi,Itor,RealToBits,BitsToReal}` 4종 추가. f64는 **f64 필드 없이** `f64::to_bits()→u64`로 기존 `BitPacked.val[0]`에 저장(width=64, unk=[0]) → no-float derive 가드(usize/f32/f64 reject) 그대로 만족 + reals가 골든 IR/결정성에 **참여**(side-lane보다 엄격히 깨끗). 루트 해시 1회 flip → `EXPECTED_SIMIR_HASH` 재pin(`EXPECTED_PROCESS_HASH`는 불변=정상 sanity gate), 전 `.velab`/`.vu`는 FORMAT 게이트에서 stale 거부(의도된 staleness). 엔진의 `is_real` 플래그는 비해시 런타임 `Value`/`NetSlot`에만 존재(IR 불침투). 상세: `docs/superpowers/plans/2026-06-04-real-domain-spec.md`. |
| `final`, `foreach`, `unique`/`priority`, `do-while`, `join_any`/`join_none` | **lowering-only** — 기존 `Terminator`/`Branch`/`Goto`/`Fork`, 새 이름 0, re-freeze 0 |

---

## 11. 커버리지 (MVP 그룹 → 타입; in-MVP 누락 0)

| MVP-서브셋 그룹 (doc 01 Phase-1) | 표현 타입 |
|---|---|
| 설계단위: module/port/param/localparam/generate/genvar | `Instance` + `NetVar.dir`(port); param/localparam/genvar는 elaboration 시 `ConstVal` |
| 자료형: wire/reg/logic/integer, 벡터, packed array | `NetVar.kind:NetKind` + width/msb/lsb(벡터) + array_len(1-D unpacked); packed array=더 넓은 `BitPacked` |
| 절차블록: initial/always/always_ff/comb/latch | `Process` + `Sensitivity{kind:SensKind, edges}` |
| 문장: `=` `<=` if case casez casex for while repeat forever begin/end | `Stmt::{BlockingAssign,NonblockingAssign}` + `Terminator::{Branch,Goto}` chain(§5) |
| 타이밍: #delay @(event) wait | `Terminator::Delay`/`Wait{cond:WaitCause}` |
| 연속대입: assign(+delay) | `ContAssign{lhs,rhs,delay}` |
| system tasks: $display/$write/$monitor/$strobe / $time/$realtime / $finish/$stop / $dump* | `Stmt::SysTask{which:SysTaskId,fmt,args}` + `Expr::SysFunc{which:SysFuncId}`($time/$realtime) |
| 식: Verilog-2005 연산자 집합 | `Expr` 10 variant + `UnOp`(6 reduction 포함) + `BinOp`(산술/비트/논리/관계/`==`/`!=`/`===`/`!==`/shift `<<<`/`>>>`) + `Ternary` + `Concat`/`Replicate` + `Select`+`SelKind` + `Signal.word` + signedness |

concat-LHS→`Lvalue.chunks`; casez/casex don't-care→`Branch.cond`(match 패턴, `nets` 미저장); multi-edge→`Sensitivity.edges`/in-body `WaitCause`; comb/latch→`SensKind`+`AnyEdge`; string 리터럴→`ConstVal{repr:StrUtf8}`. **in-MVP 미표현 0.**

---

## Sources
- 01-goals-and-scope.md (Phase-1 freeze 표), 06-simulation-engine.md (프로세스 모델 SD1–SD5), 14-staged-artifacts.md §1/§5, 16-schema-hash-spec.md
- hdl-reference/verilog/{02,03,05,06}, /01-goals
- 선행연구(2026-06-04 검증): Icarus `ivl_expr_t`/`ivl_statement_t`/`ivl_lval_t`, Verilator `V3AstNodeExpr`, Yosys RTLIL `State`/`SigChunk`, CXXRTL
