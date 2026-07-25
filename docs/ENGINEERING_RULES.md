# 엔지니어링 규칙 — silent-wrong 재발 방지 (vitamin)

> 누적 슬라이스의 **적대 리뷰에서 실제로 silent-wrong을 한 번씩 막았던** 규칙만 모았다.
> 일화·§번호·SHA는 [ROADMAP_ARCHIVE](ROADMAP_ARCHIVE.md)에 있고 여기엔 **규칙만** 둔다.
> 구현 전에 읽고, 새 교훈은 해당 절에 1줄로 병합한다.

## 리뷰 방법론 (구현·설계 이후)

- **적대적 코드 리뷰** — 라이브 차분(iverilog 등)으로 silent-wrong을 실제 재현해 검증.
- **Fagan Inspection** — 역할(Sub-agent)을 **Author / Moderator / Reviewer / Recorder** 로 분리해 진행.
  - 사전 정의 체크리스트는 **Spec 문서**(`docs/preview/`)로 대체. 별도 체크리스트가 있으면 대체가 아니라 **합산**.
  - 코드의 **논리적 오류**를 검증.
- 리뷰 관점 4축: **Architecture & System Integration** · **Performance & Efficiency** · **Maintainability & Readability** · **Robustness & Testability**.

## 작성 원칙

> 누적 슬라이스의 적대 리뷰에서 실제로 silent-wrong을 한 번씩 막았던 규칙들. 일화·§번호는 [ROADMAP_ARCHIVE](docs/ROADMAP_ARCHIVE.md)에 있고 여기엔 **규칙만** 둔다. 새 교훈은 이 절에 1줄로 병합.

### 정확도 서열 (최우선 불변)

- **정의/서열**: silent-wrong(틀린 출력·무에러)=최악 > honest-loud(명시 거부)=항상 안전. **검증 불가면 구현하지 말 것**(오라클도 전제조건도 없으면 loud 유지).
- **오라클 없는 영역**(SVA·OOP·CRV·clocking·array-param 등 iverilog 거부분)= hand-IEEE 핀 + **vita-내부 등가 차분**(신규 형태 ≡ 검증된 기존 형태 byte-identical)이 teeth.
- **G2 확장**: 관찰 rail(JSONL)도 **틀린 로그=silent-wrong**(LLM 오도). 관찰값=엔진 단일소스 파생(이중계산 금지·재도출=EXACT 미러)·**VALUE export=formatter 지원 kind만 allow-list**(non-bit-vector=loud)·미해석 probe=loud·teeth=3-way+결정성 골든.
- soundness와 differential이 충돌하면 **differential이 이긴다**(측정>주장).

### 정확도·게이트

- 가능한 IR-0. 공통경로 funnel(인터프리터+VM 단일 청크포인트) 먼저 확인. 비대상 디자인 byte-identical(가드/사이드카=값 다를 때만).
- **공유 분류기/게이트 확장 = ALL consumer 전수**: accept-gate walker는 conservative or `_`-free-exhaustive(under-detecting=반복 silent 원천)·**분류기는 자기가 분류하는 표현식의 LOWERING과 동일 resolver를 써야** 한다(다르면 shadowing에서 조용히 갈림·IR twin의 AST-레벨 projection이 되면 불일치 구조적 불가).
- **READ 경로를 넓히면 WRITE twin을 같은 반복에 전수하라** — read 하나를 고치면 대개 write 쪽에 같은-클래스 silent가 여러 형태로 잠복해 있다(select 3형·concat). twin 판정 기준=**scalar/fixed 쌍둥이가 loud인데 이 경로만 조용하면 그 경로가 이상한 것**.
- **guard는 문서화된 단일 퍼널에 두고 全 site가 술어 하나를 공유하라** — 별도 site에 두면 형제 축(concat 등)이 열린 채 남고, 술어를 둘로 나누면 (string/real처럼) 축마다 커버리지가 갈린다. 술어 이름은 **금지 사유**(bit-addressable 아님)로 짓고 타입 열거로 짓지 마라.
- **공유 기구(walk·분류기·퍼널)에 semantics를 추가할 땐 default가 아니라 OPT-IN 파라미터로** — consumer마다 순서 의존성·안전 전제가 다르다(한 곳을 위해 14개 전부에 리스크를 지우지 마라). opt-in 함수 doc에 **양성 전제조건**(언제 켜도 되는지)을 반드시 적어라(금지 조건만 적으면 다음 사람이 같은 함정에 빠진다).
- **mutable elaboration state(`symbols` 등)에 name resolution/분류를 걸면 phase 재실행과 충돌**한다 — 진단이 특정 phase에만 있으면 **결과가 조용히 삭제**된다(generate body 통째 소멸·exit 0). 순서 의존이 의심되면 **AST-gathered pure-function 집합**으로 판정하라.
- **ALL-sites 전수(최다 재발 패턴)**: 공유 함수/desugar가 도는 **모든** 스코프·caller·parser 변종·assign-site(≥7)·net RESERVE 경로(`grep add_net`: module·frame·inline·class-method 별개 예약)·statement-dispatch·decl-level 검증=공유 헬퍼 추출로 全 decl+type-spec site 배선. **RESERVE site=frame reserve와 FULL parity**(packed_dims·dim_desc·intro_kind co-register). **eligibility-set ≡ process-set**(차집합=silent-drop). 미검증 스코프=`allow_*` gate loud 격리. dispatch hook=최상단(detection-FIRST). **deny 훅도 ALL write-path 전수**.
- **scope/safety guard=ENUMERATION보다 ALLOW-LIST**: '위험한 것' 열거는 category 누락 반복·'증명가능 안전'만 허용+나머지 reject=construction상 완전(§S⑤). 재귀 allow-list=全 compound가 全 value sub-expr **AND-recurse**(미방문 1개=escape). **가드가 대상 subset에 실제 발화 검증**(vacuous guard 주의·robust=직접 per-net 카운트).

### 확장·라우팅

- **fix/routing이 잠복 gap과 상호작용=값 맞아도 리그레션**: (a) 공유 경로+deferred gap 경유 (b) 정확-라우팅이 목적지 갭을 CONSUMING 컨텍스트서 노출 (c) masking loud-guard 제거→pre-existing 노출. **深 residual=loud-guard>부분정규화**. 게이트 술어=목적지 CONSUMER set과 EXACT 일치. 안전형=**strictly ADDITIVE/fail-closed 부분집합**·나머지 old 경로 유지. **blanket-reject RELAX=숨기던 全 shape 노출**→old-reject 전수 enum+live-oracle differential. teeth=vs main sweep regress 0.
- 확장=discriminator-BRANCH(기존 경로 verbatim)·신규 eligibility set은 disjoint 증명. 1-D→N-D 전 offset/stride N-D 처리 확인(DIRECTION 1곳=double-flip 방지). **deferred 미러(write→read)=offset+방향-의존 KIND(±:)도 resolution서 결정**(lowering-baked=반대silent). **nested/packed select WRITE=read의 flatten-prefix 미러**(leaf=stride·fail-closed desc-zero-lsb·var-packed=loud).
- **정확도 사다리 = silent-wrong ≪ loud ≪ correct-support · 올라가되 절대 내려가지 마라**(동작하던 걸 loud화=회귀·loud는 "못 하는 것"에만 정당). **silent-wrong을 다른 silent-wrong과 맞바꾸지도 마라** — 타입 변환은 **leaf가 아니라 문맥 경계**에서. 문맥 도메인을 못 만들면 **변환 말고 loud**.
- **경로 A가 경로 B보다 능력이 좁으면 그 비대칭 자체가 정확도 갭**(우회 말고 정공법으로 동등화). 사다리는 **순서대로** 올라야 안전(전제 슬라이스 먼저).
- **loud verdict도 재검증 대상**: 직접 테스트 없이 mental model로 gate한 것은 과보수일 수 있다(인접 동작 사실과 대조·distinct-value/non-square로 경험 확인). 단, 상호작용이 예측 불가면 **cleanly-verifiable subset만 지원하고 나머지는 loud**(억지 지원=silent).
- **pre-resolve(elaborate) vs post-resolve(engine) compute divergence**는 sidecar flag로 over-approximate(양측이 동일 소스에서 derive→divergence 무의미화).
- **defer→resolve 머신**: defer 시점에 미지인 것(callee shape)은 resolve로 미루고, caller-scope 의존(actual net)은 defer 시 미리 resolve해 사이드카에 저장. 방향 등 미지 정보는 각 arg를 필요한 표현 전부(value+lvalue)로 lower해두고 resolve 시 sidecar로 선택.
- **"executor가 X를 못 한다"는 대개 거짓** — (a) 저장소 interior-mutability (b) 그 경로로 보낸 분류/라우팅이 틀린 것. 재작성 전에 **가장 단순한 형태를 fresh-probe**. 깊다고 판정한 기능도 **Case 분할**하면 대부분이 기존 모델로 공짜 동작하고 일부만 신규 인프라가 필요.

### 영역별 레퍼런스 (그 영역을 건드릴 때만)

- **width/type 축**: self-width table(`width.rs`)·eval 일치. **width-분기는 width-0 HANDLE(string/dyn/queue) 오분기**→NetKind discriminator를 width 前·is_str 라우팅=설정처 grep 단일소스. target-width fill=`lower_ctx_or_plain`. 4-state raw=`val&!unk`. resize=RHS 부호 extend·TARGET 부호 stamp. **real→int 가드=strict `<2^N`**. **2-state X/Z→0=per-WRITE-path·per-STORAGE**. string/dyn HANDLE formal=사이드카 마스크. **타입-signedness=全 decl 대칭**·**signedness fidelity가 全 consumer 도달**·**compare/case=COLLECTIVE**(§11.8.1)·**untyped param=값이 타입 결정**(§6.20.2·fail-open). **const-fold=단일-Const만 provably-safe**. 상세=ARCHIVE.
- **name/scope 축**: comma-list sticky 속성 스레드. flat map+nested scope=lazy snapshot/restore(TYPE+VAR·ALL decl-region). alias/copy=이름 keyed ALL 사이드맵+**set-or-CLEAR**. **flat 레지스트리+scoped resolution=scope PRECEDENCE 미모델→wrong-shadow silent→dedicated infra**. 새 var-binding=decl-binding 미러+enclosing snapshot/restore 격리. collect→apply=consumption-tracking(leftover=loud). **symbols alias=중앙 퍼널(resolve_net)**. **sub-select offset 정규화=선언 base `dbase=min(msb,lsb)` 차감**(clamp=silent→loud).
- **인프라 선례**: **systask 사다리**=부작용無→elaborate None·엔진 state만→no-op Display+StmtId 사이드테이블·엔진효과+직렬화→frozen SysTaskId=format bump. side-effect sysfunc expr=statement-form desugar(single-eval). 엔진-facing 사이드카=`StagedExtraSidecars` append-only(`#[serde(default)]`·신규 필드=format bump ②). 공유 버퍼 재사용=`mem::take`/restore 격리. **1 parse fn이 N item emit=pending-queue+drain at collection-LOOP top**(종료조건에 `!pending.empty`). **persistent 사이드맵은 scope-restore 안 됨→pollution**(save/restore·set-or-CLEAR).
