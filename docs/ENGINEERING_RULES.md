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
- **제약이 "머신러리 부재"로 보이면 대개 "가정"이다** — 기존 코드가 이미 일반형을 계산하는데 호출부가 특수형을 *가정*해 좁혀둔 경우가 흔하다. 새로 만들기 전에 **일반 경로가 이미 무엇을 정규화하는지** 확인하고, 가정 대신 **조회**로 바꿔라(특수형은 그 조회가 항등이 되게 해서 IR byte-identical 유지).
- **masking loud-guard를 걷어내면 그 밑의 pre-existing silent-wrong이 드러난다 — 그것도 loud→silent 하강이다.** 게이트 제거는 반드시 PRE 3-way로 "가려져 있던 형태"를 전수하고, 아직 못 고치는 형태는 **실제 이유로 문구를 바꾼 loud를 유지**하라(원래 문구를 남기면 다음 사람이 잘못된 근인을 물려받는다).
- **source가 destination과 aliasing될 수 있으면 capture→mutate→install 순서로** — 재귀에서 caller net == callee net이 되는 경우(자기 formal을 자기 actual로 넘김·copy-out) in-place 순서는 조용히 값을 잃는다. 두 net이 다를 때와 같을 때 **양쪽 다 옳은** 순서를 고르고 doc에 두 경우를 다 적어라.
- **"executor가 X를 못 한다"는 대개 거짓** — (a) 저장소 interior-mutability (b) 그 경로로 보낸 분류/라우팅이 틀린 것. 재작성 전에 **가장 단순한 형태를 fresh-probe**. 깊다고 판정한 기능도 **Case 분할**하면 대부분이 기존 모델로 공짜 동작하고 일부만 신규 인프라가 필요.

- **한 술어로 두 resolver 를 못 섬긴다** — 값이 두 표현을 가지면 subsystem 마다 lookup 순서가 달라진다(const-fold=`params` vs lower=`real_param_val` 우선). consumer 를 resolver 별로 묶고 **술어를 갈라라**.
- **가드를 구문(리터럴 모양)으로 세우지 마라** — 새 슬라이스가 그 구문 밖의 값을 도달시키는 순간 뚫린다(`expr_is_real_literal` 가드가 `R`·`R+1` 을 흘려 자식이 조용히 잘못된 param 으로 실행). **값 기반**으로.
- **새 저장 클래스로 재분류하면 기존 클래스의 능력을 상속시켜라** — real 재분류만으로 정수 능력 8형이 통째 false-loud된 사례 有. **두 표현이 정확히 일치할 때는 양쪽 등록이 정답**(근사면 등록 금지).
- **shape(AST walker) 판정보다 값(lower된 IR/전 하위식) 판정이 구조적으로 완전** — walker는 새 shape를 놓치지만 값은 못 숨는다(`_`-free 열거보다 강함). 폭 가드를 리프로 세웠다가 중간값 오버플로를 놓친 §4.5.229 가 같은 교훈.

- **loud 를 silent 로 넓히지 마라** — 근인이 pre-existing 이어도 내 변경이 그 표면을 loud→silent 로 확대하면 내 회귀다(근인은 defer, 표면은 loud 유지).
- **"이 래퍼가 모든 site 를 덮는다" 는 주석은 테스트로 고정하라** — 6 중 4 만 참이었고 남은 2 가 폭주였다.
- **loud gate 를 추가하면 그 gate 를 우회하는 간접 경로(함수 호출·계층 이름·메서드)를 전수하라** — "이 gate 가 유일한 그물"이라고 썼으면 실제로 유일한지 측정하라.

### 영역별 레퍼런스 (그 영역을 건드릴 때만)

- **width/type 축**: self-width table(`width.rs`)·eval 일치. **width-분기는 width-0 HANDLE(string/dyn/queue) 오분기**→NetKind discriminator를 width 前·is_str 라우팅=설정처 grep 단일소스. target-width fill=`lower_ctx_or_plain`. 4-state raw=`val&!unk`. resize=RHS 부호 extend·TARGET 부호 stamp. **real→int 가드=strict `<2^N`**. **2-state X/Z→0=per-WRITE-path·per-STORAGE**. string/dyn HANDLE formal=사이드카 마스크. **타입-signedness=全 decl 대칭**·**signedness fidelity가 全 consumer 도달**·**compare/case=COLLECTIVE**(§11.8.1)·**untyped param=값이 타입 결정**(§6.20.2·fail-open). **const-fold=단일-Const만 provably-safe**. 상세=ARCHIVE.
- **name/scope 축**: comma-list sticky 속성 스레드. flat map+nested scope=lazy snapshot/restore(TYPE+VAR·ALL decl-region). alias/copy=이름 keyed ALL 사이드맵+**set-or-CLEAR**. **flat 레지스트리+scoped resolution=scope PRECEDENCE 미모델→wrong-shadow silent→dedicated infra**. 새 var-binding=decl-binding 미러+enclosing snapshot/restore 격리. collect→apply=consumption-tracking(leftover=loud). **symbols alias=중앙 퍼널(resolve_net)**. **sub-select offset 정규화=선언 base `dbase=min(msb,lsb)` 차감**(clamp=silent→loud).
- **인프라 선례**: **systask 사다리**=부작용無→elaborate None·엔진 state만→no-op Display+StmtId 사이드테이블·엔진효과+직렬화→frozen SysTaskId=format bump. side-effect sysfunc expr=statement-form desugar(single-eval). 엔진-facing 사이드카=`StagedExtraSidecars` append-only(`#[serde(default)]`·신규 필드=format bump ②). 공유 버퍼 재사용=`mem::take`/restore 격리. **1 parse fn이 N item emit=pending-queue+drain at collection-LOOP top**(종료조건에 `!pending.empty`). **persistent 사이드맵은 scope-restore 안 됨→pollution**(save/restore·set-or-CLEAR).

### 무오라클 능력 (§4.5.235 · §4.5.236)

- **오라클이 미지원하는 스펙은 결함이 있어도 영원히 안 보인다.** iverilog 가 `%p` 를 아예 구현하지 않아 차분이 침묵했고, 실제로 real 이 정수로 반올림돼(2.5→3) 값이 사라지고 있었다. **테스트 0건인 스펙/포맷을 찾는 것** 자체가 유효한 silent-wrong 탐색 전략이다(`grep -rl '%p' tests/` 가 비면 그 자리가 후보).

- **오라클이 거부하는 영역에서 우리가 앞서 있으면, 테스트가 유일한 방어선이다.** iverilog 가 구문을 거부하는 기능(modport 타입 포트·함수 결과 part-select 등)은 **차분으로 회귀를 감지할 수 없다** — 핀이 없으면 리팩터 한 번에 조용히 사라진다. fresh 스윕이 clean 으로 끝나도 결론은 "할 일 없음"이 아니라 **"핀 없는 무오라클 능력을 찾아 핀하라"**.

### 두 술어 봉인 (§4.5.234)

- **값 술어가 둘이 되는 걸 피할 수 없으면, 두 구현이 반드시 일치하는 부분집합으로 좁혀라.** 규칙이 같기를 바라지 말고 **불일치가 가능한 입력을 거부**하라 — 파서의 리터럴 폴드는 *절단이 필요한* 리터럴을 아예 안 받는다(`'h1FFFFFFFF` 를 elaborate 는 33비트로 키우고 파서는 32비트로 마스킹했다). 남는 것은 정의상 안전하고, 거부된 것은 이전 동작(loud) 그대로다.
- **두 술어의 teeth 는 결과를 한 줄에 같이 찍는 것**(`x.name()` 과 `x` 값). 어느 한쪽만 보면 불일치가 "이름만 빈 문자열" 같은 조용한 형태로 숨는다.
- **타입이 있는 값의 부호는 "리터럴이 뭐라 쓰였나"가 아니라 "어떤 타입의 값인가"가 정한다.** enum 라벨은 §6.19 상 **base 타입의 값**이므로 `32'hDEADBEEF` 는 `enum integer` 에서 −559038737 이다 — 리터럴의 `s` 마커로 부호를 정하면 한쪽은 **false-loud**, 반대쪽은 **이름만 조용히 빈 문자열**이 된다. 폴드는 **패턴+폭**만 내놓고 **해석은 타입을 아는 호출부**에서.
- **범위/유효성 검사의 판별자가 정말 "폭"인지 의심하라** — enum 라벨 검사는 폭이 아니라 **출처**(명시 값 vs 자동증가)가 기준이었다. 명시적 `-1` 은 오류지만 `64'sh7FFF…` 다음의 wrap 은 합법이다. 값만 보면 둘 다 "음수 i64" 로 같아 보인다.

### 크기 추정 (§4.5.240)

- **"loud 하다"를 갭으로 적기 전에 그 loud 가 어느 패밀리의 규칙인지 보라.** side-effect sysfunc 는 single-eval 보장을 위해 **statement-form 으로 lower** 되므로 임의 expression 위치의 loud 는 **고칠 대상이 아니라 지켜야 할 불변식**이다. 게다가 관용적 배치(대입 rhs·if 조건)는 이미 동작하고 있었다 — **갭의 크기를 재기 전에 실제로 못 쓰는 형태가 무엇인지부터 세라**.
- **내가 큐에 적은 크기 추정도 다음 반복을 오도한다** — 오판을 발견하면 항목을 지우지 말고 **정정 사유와 함께 다시 쓰라**(지우면 다음 사람이 같은 오판을 반복한다).

### 크기 추정 (§4.5.233)

- **"작아 보이는 loud→supported" 는 값이 필요한 TIME 을 먼저 물어라.** enum 라벨 폴드는 30줄짜리로 보였지만 값이 **파스 타임**에 필요했고, 그 값을 만드는 함수는 파서에 **의존하는** 크레이트에 있었다(순환). 근인이 한 줄이어도 **그 한 줄이 사는 레이어**가 슬라이스 크기를 정한다.
- **같은 값을 두 번 파싱하게 되면 그것은 "두 술어" 함정이다** — 어긋나는 순간 조용히 틀린다(여기선 `.name()` 표와 상수가 다른 라벨을 가리킴). 불가피하면 teeth 는 반드시 **내부 차분**(두 술어의 결과가 같은 소스에서 일치하는지)으로.

### 능력 확장 (§4.5.232)

- **철자 비대칭을 없애려는 "능력 확장"이 규칙의 전제를 무너뜨릴 수 있다.** 어떤 규칙(§11.8.1 실수 우선 순서)을 적용하는 site 가 **하나뿐**인데 그 규칙이 막고 있던 능력(정수 twin)을 전역에 열면, 규칙을 모르는 **모든 consumer** 가 조용히 틀린다 — generate 분기 오선택 등 5건이 한 번에 열렸다. 확장 전에 **"이 능력을 소비하는 site 가 몇 개이고 각자 이 규칙을 아는가"** 를 세라. 셋 이상이면 규칙을 먼저 공통 퍼널로 올린 뒤에 확장하라.
- **핵심 성과와 확장을 분리해서 평가하라** — 실수 산술 폴드(핵심)는 twin 과 무관해 철회해도 100% 남았다. 리뷰가 blocking 을 내면 **확장만 떼어내 슬라이스를 살리는** 선택지가 있는지 먼저 보라.

### 오라클 검증 (§4.5.231)

- **"오라클과 다르다"를 결함으로 접수하기 前에 오라클의 자기일관성을 먼저 측정하라.** 같은 하위식을 `+0` 으로 감싸 값이 바뀌는지, 형제 연산자(`+`/`*` vs `<<`)가 같은 문맥 폭을 쓰는지 — **한 모델로 오라클의 답 전부를 재현할 수 있는지**를 물어라. 재현 못 하면 그건 갭이 아니라 오라클 결함이고, 쫓아가면 우리 쪽이 비일관이 된다.
- **오라클이 없을 때의 teeth = 자기 대 자기 항등식.** "값 보존 래퍼가 값을 바꾸면 안 된다", "연산자끼리 문맥 폭을 두고 갈리면 안 된다" 는 오라클 없이도 검증 가능하고, 나중에 모델을 바꿔도 **여전히 참이어야 하는** 성질이라 회귀 테스트로 오래 산다.

### 상수 접기 (§4.5.230)

- **오라클이 없거나 모호하면 자기 엔진이 오라클이다.** elaborate 인터프리터와 런타임이 **같은 소스**를 다르게 계산하면 그 자체가 결함이다 — 이 차분이 슬라이스 전체를 이끌었고(9형 중 6형 불일치), iverilog 는 사후 확인이었을 뿐이다.
- **값·폭·부호는 세 개의 술어이고 함께 움직인다.** 접기를 넓히면 `const_expr_signed` 와 `param_decl_width` 도 같은 arm 집합을 가져야 한다 — `Cast` 에서 배운 교훈이 `Call` 에서 **그대로 반복**됐다(반환 타입을 몰라 −56 이 4294967240 으로).
- **폭 문맥에서 부호는 문맥당 한 번 정하고 내려보내라**(IEEE §11.8.1). 노드마다 다시 계산하면 부호 있는 하위식이 부호 없는 부모 밑에서 sign-extend 돼 **정답이 오답으로 하강**한다(`(b+b)/u` 100→228). 자기결정 위치(비교 피연산자 등)만 자기들끼리 다시 통일한다.
- **재진입하는 헬퍼는 깊이를 리셋한다.** 폭을 구하려고 `const_eval_in_scope` 를 부르면 그 안의 call 깊이가 0 부터 다시 세어져 `bit [f()-1:0]` 이 스택을 넘겼다. 깊이 캡보다 **그 형태를 아예 접지 않는** 구조적 제거가 낫다.
- **변환은 리프의 선언 경계에서** — 좁은 signed 로컬은 env 에 이미 sign-extend 된 i64 로 들어있다. 문맥이 unsigned 면 §11.6.1 은 **자기 폭에서 zero-extend** 하라고 하는데, 리프를 그대로 두고 연산자 폭에서 마스킹하면 sign 비트가 살아남아 비교가 뒤집힌다. (§4.5.229 의 "문맥 경계에서 변환" 과 짝 — 경계는 **양쪽**에 있다.)
- **"거부(None)"는 "모름"이어야지 기본값이 되면 안 된다** — 폭을 못 구한 선언을 읽는 쪽이 `(32, unsigned)` 로 추측하는 순간, 거부의 안전성(=이전 동작 유지)이 사라지고 64비트 값이 조용히 잘린다. 모름은 **마스킹 안 함**으로 전파하라.
- **폭을 알게 되면 "모르니 거부"였던 규칙을 재검토하라** — 음수의 논리 `>>` 는 폭 의존이라 거부하고 있었는데, 문맥 폭이 생긴 순간 **비트패턴으로 정확히 계산**할 수 있게 됐다(거부를 남겨두면 correct→loud 하강).

### 상수 접기 (§4.5.229)

- **폭 정확성 가드는 리프가 아니라 값으로 세워라.** "모든 리프가 ≥32비트면 안전"은 두 방향으로 틀렸다 — `(32'd1<<32'd33)>>32'd30` 은 리프가 전부 32비트인데 **중간값이 32비트를 넘었다 돌아와** SV 와 갈리고(그 자리는 PRE 가 **정답**이었으므로 correct→silent-wrong 하강), 반대로 `4비트 param * 2` 는 SV 가 max-폭으로 32비트에 계산하는데 과잉거부된다. 판정은 **모든 하위식의 값**이 안전 범위에 있는지로 하고, 그 traversal 은 세 조건이 **하나를 공유**하게 하라.
- **"이 잔차는 기존 경로도 갖고 있다"는 정당화는 측정 전엔 거짓으로 취급하라.** `*`·`<<`·`%` 는 엔진이 애초에 접지 않아 폭 1 로 떨어졌고 **그 폭 1 이 정답**이었다. 기존 경로가 접는 연산(`+`/`-`)만 그 잔차를 갖고 있었다.
- **약한 folder 를 강한 것으로 바꿀 때, 약한 쪽의 "이상한 동작"에 의존하던 곳을 먼저 찾아라.** 리터럴 folder 의 의도적 `wrapping_neg`(음수 리터럴 → 0xFFFF_FFFF)는 방향 검사가 읽던 신호였는데, 그 값으로 폭을 계산하자 **u32 오버플로 패닉**(release 는 0폭)이 됐다. 폭 산술은 u64 checked + `MAX_NET_WIDTH` 상한.
- **fold 를 넓히면 그 값의 SIGNEDNESS 를 읽는 형제 술어도 같이 넓혀라.** `const_eval_in_scope` 에 `Cast` arm 을 더하자 `localparam P = int'(-300)` 이 **접히기는 하는데 `const_expr_signed` 에 Cast arm 이 없어 unsigned 로 바인딩**됐다(loud→silent-wrong). 값 술어와 부호 술어는 같은 arm 집합을 가져야 한다.
- **인터프리터를 신뢰 경계로 쓸 거면 그 내부 폭도 확인하라.** 상수함수 호출을 접기 전에 **반환 폭만** 보면 부족하다 — 인터프리터가 대입을 선언 폭으로 coerce 하지 않으므로 narrow 한 **로컬/포멀 하나**로 발산한다(`bit [3:0] t = 4'd15+4'd15` = SV 14, i64 30).

### round-20 (§4.5.228) — 기록된 전제를 먼저 재측정하라

- **"deep 이라 못 한다"는 기록은 근거가 아니라 가설이다.** ROADMAP 이 "깊은 스케줄러 rework" 라 적어둔 fork 건은 **bb 번호공간 충돌 한 곳**이었다. 근인을 지목한 것은 코드 읽기가 아니라 **판별 실험**(task CFG 에 dead `if` 9개 → resume bb 를 밀면 같은 설계가 통과 ⇒ 충돌이 원인). 가설이 "무엇을 바꾸면 증상이 사라지는가"로 표현되면 바로 실험이 된다.
- **"loud 로 기록됨"은 loud 라는 뜻이 아니다.** 음수 하한 배열은 `a[-1]` 을 **명시적으로 건드릴 때만** E4002 였고, 순수 `foreach` 는 **무성**이었다. multi-packed 음수 bound 도 "warn+clamp" 로 적혀 있었지만 그 경고는 **형제 선언**에서 나오고 있었다. 분류를 신뢰하지 말고 **그 항목만 있는 최소 설계**로 재현하라.
- **플래그가 이유를 설명하는 것처럼 보이면 의심하라.** `allow_string_init=false` 는 "그 스코프는 flush 를 안 돈다"고 읽혔지만 flush 는 있었다. 진짜 이유는 선언 시점 쓰기가 **bare-name 으로 모듈 리스트에 새는 것**. 플래그는 결함의 *설명*이 아니라 *대역*이었고, 이유를 고치자 flag 4개가 한꺼번에 열렸다.
- **수명 문제는 이미 그 수명을 가진 것에 붙여라.** 동시 활성화 dyn 배열은 새 arena 가 아니라 **AUTOMATIC window 의 수명**(같은 두 지점의 park/unpark)으로 풀렸다. 단, **공유와 부재는 다르다** — window 는 핸들로 공유되지만 park 된 배열은 힙에서 *사라진다*. 그래서 arm 이 부모 배열을 x 로 읽는 회귀가 났고, **16k 설계 스윕만 그것을 잡았다**(이 슬라이스용으로 쓴 프로브는 전부 통과했다).
- **폭과 정규화는 함께 켜라.** 음수 packed bound 에서 폭만 넓히고 선택 정규화를 빠뜨리면 "넓지만 잘못된 비트를 짚는 net" = silent-wrong 이 된다. 그래서 opt-in 파라미터로 묶고 **기록할 수 있는 호출부만** 켰다(나머지는 리터럴 `false` 로 바이트 동일·loud 유지). 비대칭이 남지만 **loud 한 비대칭**이다.
- **동결 enum 에 변종을 더하기 전에 사이드카를 보라.** `$fmonitor`/`$fstrobe` 는 `Monitor`/`Strobe` id 재사용 + StmtId 사이드카로 끝났다(SysTaskId 변종이었다면 SimIr 해시 flip → 전 골든 재핀). 그리고 **fd 위치를 판단하는 술어는 하나여야 한다** — 분할과 기록이 각자 판단하면 언젠가 어긋난다.
- **오라클이 자기 자신과 모순되면 그것은 결함이다.** iverilog 는 같은 fd 에 `$fmonitor` 를 누적하면서 `$monitor` 는 싱글턴으로 둔다. 측정>주장이지만, **측정된 것이 오라클의 내부 모순일 때는** hand-IEEE 로 가고 그 측정값을 테스트에 기록하라.
