# 04 · 시스템 아키텍처

> 설계 명세 §5 전체 기반. 크레이트 책임·IR 설계·builtin dispatch 경로를 상세 기술.

---

## 파이프라인

```
source files
  → preprocess   (`define / `ifdef / `include / `timescale)
  → lex          (토큰 스트림)
  → parse        (언어별 AST)          ← 문법 검사 수행
  → elaborate    (파라미터 해소, 계층 평탄화, 타입/포트/연결성 검사)
  → sim IR       (nets + processes + sensitivity + builtin-call nodes, 언어 중립)
  → sim engine   (event-driven kernel + timescale time wheel + builtin dispatch)
  → VCD writer   (IEEE 1364 정규 포맷 · RTL dump 태스크가 호출될 때만 활성)
```

**언어 의존부는 parse까지, 그 이후는 언어 중립이다.** elaborate, sim IR, sim engine, VCD writer는 Verilog/SystemVerilog/VHDL을 구분하지 않는다. VHDL 프론트엔드(Phase 3)를 공유 IR 위에 얹을 수 있는 근거가 여기에 있다(§9 로드맵).

각 단계는 독립 크레이트에 대응하고 내부 검사를 담당한다.

**preprocess:** `` `define / `ifdef / `ifndef / `include / `timescale `` 등 컴파일러 지시어를 처리해 순수 HDL 토큰 스트림의 전처리된 소스를 생성한다. 매크로 전개와 include 파일 추적도 이 단계 책임이다. 이후 단계는 전처리된 소스만 본다.

**lex:** 전처리된 소스를 언어별 토큰 스트림으로 변환한다. 키워드, 식별자, 리터럴, 연산자를 구분하며 소스 위치(파일/라인/컬럼) 정보를 토큰에 첨부해 이후 진단 메시지에 사용한다.

**parse:** 언어별 문법 규칙에 따라 토큰 스트림을 AST로 변환한다. **문법 검사는 이 단계 내부에서 수행된다** — parse가 실패하면 elaboration으로 진행하지 않는다. 구문 오류의 소스 위치와 회복(recovery) 힌트는 diag 크레이트를 통해 보고된다.

**elaborate:** parse 결과 AST를 소비해 언어 중립 sim IR을 생성한다. **연결성·타입·다중구동 등 정합성 검사는 이 단계 내부에서 수행된다.** 구체적으로: 파라미터 오버라이드 평가(generate 스킴과 인터리빙), 모듈 인스턴스 재귀 해소와 계층 평탄화, 포트/타입 정합 검증, 미연결 net·다중구동(multiple driver) 감지가 포함된다. 오류는 diag 크레이트로 보고된다.

**sim IR:** elaborate가 출력하는 언어 중립 중간 표현이다. net(폭/4-state), process(트리거 조건/본문), continuous assign, 계층 인스턴스, builtin-call 노드를 담는다. 인터프리터와 컴파일드 백엔드 양쪽이 동일 IR을 소비한다.

**sim engine:** sim IR을 실행하는 이벤트 구동 커널이다. stratified event queue(Active → Inactive(`#0`) → NBA(`<=`) → Monitor), timescale 기반 64비트 정수 시간 모델, delta cycle, builtin dispatch를 담당한다.

**VCD writer:** IEEE 1364 VCD 포맷으로 신호 변화를 직렬화한다. **RTL 코드에서 dump 시스템 태스크(`$dumpfile`, `$dumpvars` 등)가 호출될 때만 활성화된다.** dump 태스크가 한 번도 불리지 않으면 VCD 파일은 생성되지 않는다(no-op). 자동 항상-덤프가 아니다.

**진단·로깅(횡단):** 모든 단계의 운영 출력 — 읽는 파일·라이브러리 해소·elaborate 진행·런 요약, 그리고 error/warning/fatal — 은 단일 이벤트 스트림으로 흐른다. 진단 *렌더링*(file:line:col + caret)은 `diag`, 운영 *transcript·로그파일(tee)·severity·메시지 코드·exit-code·`$error`/`$fatal` 연동*은 `vita-log`가 담당한다. 권위 문서는 [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md).

---

## 실행 모델 — 원샷과 단계별 실행

파이프라인은 두 가지 방식으로 구동된다.

**원샷 — `vita`**
preprocess → lex → parse → elaborate → sim → VCD 전 과정을 한 명령으로 실행한다. 중간 산출물을 디스크에 남기지 않고 메모리에서 곧장 다음 단계로 흘려보낸다. 일상적인 시뮬레이션의 기본 진입점이다.

**단계별 — `vcmp` / `velab` / `vrun`**
같은 파이프라인을 세 개의 독립 실행 단계로 쪼갠다. 각 단계는 앞 단계가 디스크에 남긴 산출물(artifact)을 읽어 이어받는다.

| 명령 | 단계 | 담당 크레이트 | 입력 | 출력(artifact) |
|---|---|---|---|---|
| `vcmp` | compile | hdl-preprocess · hdl-lexer · hdl-parser | HDL 소스 | 분석된 설계 단위 (work 라이브러리) |
| `velab` | elaborate | elaborate | `vcmp` 산출물 | elaborated sim-ir 스냅샷 |
| `vrun` | simulation | sim-engine · vcd-writer · hdl-builtins | `velab` 산출물 | 시뮬레이션 실행 + VCD (dump 호출 시) |

`vita`(원샷)는 이 세 단계를 디스크 왕복 없이 연결한 것과 의미상 동일하다.

**상용 EDA 흐름과의 매핑**
이 3단계 분리는 상용 시뮬레이터의 표준 흐름과 1:1 대응한다.

| vitamin | Cadence Xcelium | Synopsys VCS |
|---|---|---|
| `vcmp` (compile) | `xmvlog` / `xmvhdl` | `vlogan` / `vhdlan` |
| `velab` (elaborate) | `xmelab` | `vcs` (elab → `simv` 빌드) |
| `vrun` (simulation) | `xmsim` | `simv` |
| `vita` (원샷) | `xrun` | — |

**단계 분리의 이점**
- **독립 빌드** — 네 명령은 프로덕션에서 단일 multicall 바이너리(argv[0] 베이스네임 디스패치)로 배포되지만, 단계별 디버깅용 실제 `[[bin]]` 타깃을 dev 전용 `separate-bins` 피처 뒤에 둬 `vcmp`/`velab`/`vrun`을 독립적으로 빌드·호출·디버깅할 수 있다.
- **단계별 디버깅** — compile·elaborate·simulation 중 어디서 문제가 났는지 단계 경계의 산출물을 `--dump`(RON 뷰)로 직접 들여다보며 좁힐 수 있다.
- **불필요한 단계 스킵 (건전성 보장)** — 소스가 그대로면 `vcmp`/`velab` 산출물을 재사용해 `vrun`만 반복 실행한다. 단, 이 스킵이 *건전*하려면 `vrun`이 상류 체인 전체를 **라이브 소스에 대해 재검증**해야 한다(내용 해시 대조, mtime 금지). Xcelium `-R`/`-r`의 검사-생략 패스트패스와 의도적으로 다르다.

**산출물 포맷 · 해시 결합 · CLI 표면.** 단계 간 산출물의 온디스크 포맷(vcmp work 라이브러리 디렉터리 / velab `<top>.velab` 스냅샷), staleness 해시 결합 규칙, 멀티 라이브러리 주소화, 전체 CLI 플래그 표면은 **[14-staged-artifacts.md](14-staged-artifacts.md)** 에 권위 있게 정의한다. 핵심 법칙: 플래그는 *어느 바이너리가 파싱하느냐*가 아니라 *어느 단계의 출력을 교란하느냐*로 분류되어 staleness 해시에 결합된다 — 전처리 교란(`+define`/`+incdir`/`-y`/`--std`)→vcmp 소스 해시, elaborate 교란(top/파라미터/`--multi-driver`/라이브러리 바인딩)→velab 합성 해시, 런타임 전용(`+plusargs`/`--log`/seed)→해시 무관. `` `timescale ``·`` `default_nettype `` 같은 sticky 디렉티브의 파일 간 캐리오버 때문에, vcmp 단위 해시는 *상속 반영 후* 전처리 바이트 + 정렬된 파일 목록 위에서 계산한다.

(`vita`·`vcmp`·`velab`·`vrun` 이름은 코드네임 `vitamin`과 함께 현재 placeholder다.)

---

## Cargo 워크스페이스 / 크레이트

15개 크레이트가 단일 cargo workspace를 구성한다. 각 크레이트는 단일 책임 + 명확한 인터페이스를 가져 독립적으로 테스트 가능하다.

| 크레이트 | 책임 | 의존 |
|---|---|---|
| `hdl-preprocess` | 컴파일러 지시어 처리, 매크로 전개, include | — |
| `hdl-lexer` | 토큰화 (언어별 토큰 집합) | preprocess |
| `hdl-parser` | 토큰 → AST (언어별) | lexer, ast |
| `hdl-ast` | 언어별 AST 타입 정의 | vita-artifact-derive (serde·SchemaHash derive) |
| `elaborate` | 파라미터 해소·계층 평탄화·타입/연결성 검사 → IR 생성 | ast, sim-ir, diag |
| `sim-ir` | 언어 중립 시뮬레이션 IR (net/process/sensitivity/builtin-call) | vita-artifact-derive (serde·SchemaHash derive) |
| `sim-engine` | 이벤트 구동 커널, 스케줄러, 시간 모델 | sim-ir |
| `hdl-builtins` | 표준 `$`-system tasks/functions 라이브러리 — 디스패치 테이블 + 카테고리별 핸들러 | sim-ir, sim-engine, vcd-writer |
| `vcd-writer` | IEEE 1364 VCD 직렬화 (dump 태스크가 호출될 때만 활성) | sim-ir |
| `diag` | 진단 *렌더링* (file:line:col + caret) + `Severity`/`MsgCode`/`Frame`/`Diagnostic`/`LogSink`/`LogEvent` 데이터 모델 (IO 없음 → leaf) | — |
| `vita-artifact` | 단계 산출물 (역)직렬화 + 헤더(magic/format_version/schema_hash/빌드지문) + staleness 검사(D3 트리플 대조) + `--dump` RON 뷰 | hdl-ast, sim-ir, hdl-preprocess, diag, vita-artifact-derive |
| `vita-artifact-derive` | `#[derive(SchemaHash)]` proc-macro — 타입별 local shape 문자열을 컴파일 타임 방출 (leaf, syn/quote) | — |
| `vita-schema` | `SchemaShape` trait + `ShapeRegistry` — 참여 타입 폐포를 정렬 합성해 blake3 `SCHEMA_HASH` 런타임 산출 (leaf; trait를 `vita-artifact`에 두면 순환이라 분리 — 16) | blake3 |
| `vita-log` | 운영 로깅 — **구현됨(2026-06-10): `-Wno-*`/`-Werror=` suppress/promote 게이트(`GatePolicy`/`GatedSink`)**; transcript·로그파일 tee·배너는 Phase-1.x 잔여 | diag *(tracing은 tee 랜딩 시)* |
| `cli` | 드라이버 바이너리 — `vita`(원샷) + `vcmp`/`velab`/`vrun`(단계별); 프로덕션은 단일 multicall 바이너리 | 전부 + vita-artifact + vita-log |

크레이트별 책임과 분리 이유:

**`hdl-preprocess`** 는 언어 파싱 이전 단계를 완전히 격리한다. include 파일 탐색, 매크로 전개, conditional compilation이 여기서 끝난다. 이후 크레이트는 "전처리가 완료된" 토큰 스트림만 받는다.

**`hdl-lexer`** 는 언어별 토큰 집합을 담당한다. SystemVerilog/Verilog/VHDL은 키워드 집합이 다르기 때문에 언어별 변형을 이 크레이트 내부에 격리한다. `logos` 크레이트가 후보 구현이다.

**`hdl-parser`** 와 **`hdl-ast`** 를 분리하는 이유는 AST 타입 정의가 parser와 elaborate 양쪽에서 참조되기 때문이다. 순환 의존을 피하려면 AST 타입을 독립 크레이트에 두어야 한다.

**`elaborate`** 는 AST와 sim-ir 양쪽을 알고 있는 유일한 크레이트다. 언어 지식(AST 구조)과 시뮬레이션 지식(IR 구조)의 변환 지점을 한 곳에 모은다. diag 의존은 연결성·타입 오류를 리포팅하기 위해서다.

**`sim-ir`** 는 어떤 HDL도, 어떤 백엔드도 의존하지 않는다. 이 크레이트가 좁은 표면적을 유지할수록 인터프리터와 컴파일드 백엔드 교체가 쉬워진다.

**`sim-engine`** 은 IR을 실행하는 이벤트 루프 코어다. 시간 모델, stratified queue, delta cycle이 모두 여기에 있다. hdl-builtins에 의존하지 않는다 — builtin 실행은 hdl-builtins가 엔진 위에서 동작하는 구조다.

**`hdl-builtins`** 는 표준 `$`-system tasks/functions 전 범주를 구현하는 라이브러리다. display·I/O·file I/O·sim ctrl·time·변환·비트벡터·수학·random·dump·assertion 샘플링·introspection을 카테고리별 핸들러로 구현하고 디스패치 테이블로 묶는다. sim-engine, vcd-writer 양쪽을 의존해 dump 패밀리 호출을 vcd-writer로 라우팅한다. (`hdl-reference/system-tasks/` 참조)

**`vcd-writer`** 는 직렬화 책임만 갖는다. VCD 헤더·$scope 계층·$var 선언·값 변화 기록이 모두 이 크레이트다. sim-engine이나 hdl-builtins의 실행 로직과 섞이지 않는다.

**`diag`** 는 소스 위치 정보와 오류/경고 메시지를 일관된 형식으로 생성하는 공유 라이브러리다. 렌더러는 `miette`(`default-features = false`로 leaf 순수성 유지; `codespan-reporting`을 fallback로 교체 가능)다. 어느 단계에서든 같은 방식으로 진단을 보고할 수 있게 한다. (크레이트 결정 근거는 [02-implementation-language.md](02-implementation-language.md))

**`vita-artifact`** 는 단계 간 디스크 산출물의 (역)직렬화·헤더·버전·staleness를 한곳에 격리하는 크레이트다(D1). work 라이브러리 매니페스트와 `<unit>.vu`/`<top>.velab` 헤더 프레이밍, 전처리-소스 해시 대조, `--dump` RON 뷰가 모두 여기에 있다. 직렬화는 이 크레이트의 선택적 경계로만 일어나므로 원샷 `vita` 경로는 이 코드를 호출하지 않는다. `hdl-ast`·`sim-ir`의 루트 타입을 알아야 그 형상 해시를 stamp할 수 있어 둘에 의존하고, 라이브 재해시를 위해 `hdl-preprocess`에, 디코드/staleness 오류 보고를 위해 `diag`에 의존한다. 상세는 [14-staged-artifacts.md](14-staged-artifacts.md).

**`vita-artifact-derive`** 는 `#[derive(SchemaHash)]` proc-macro만 제공하는 빌드그래프 leaf 크레이트다(syn/quote 의존). 직렬화 타입 형상(필드·variant + serde 속성)의 구조적 해시를 컴파일 타임에 산출해, 타입 레이아웃이 바뀌면 이전 산출물이 silent misparse 대신 깨끗한 버전 오류로 거부되게 한다(D2). proc-macro는 rustc 안에서 돌아 cargo-native이므로 03의 "별도 빌드 스크립트/codegen 없음" 원칙을 지킨다. `hdl-ast`·`sim-ir`가 이 derive를 적용하므로 두 크레이트는 더 이상 순수 leaf가 아니다.

**`cli`** 는 드라이버 바이너리의 진입점이다. 원샷 `vita`와 단계별 `vcmp`(compile)·`velab`(elaborate)·`vrun`(simulation)을 제공하되, **프로덕션은 단일 multicall 바이너리**(argv[0] 베이스네임 디스패치; clap `multicall`이 아니라 손수 구현 — positional 기본 applet과 양립 위해)로 빌드하고 dev 전용 `separate-bins` 피처에서만 4개 `[[bin]]`을 따로 낸다. `vita-artifact`를 통해 산출물을 읽고 쓰며, 나머지 크레이트를 조합하는 글루다. 단계별 실행·산출물 흐름·CLI 표면은 위 "실행 모델" 절과 [14-staged-artifacts.md](14-staged-artifacts.md)를 참조한다.

---

## 하이브리드 시뮬레이션 전략

**MVP는 인터프리터다.** sim-ir를 직접 walk하는 이벤트 구동 인터프리터 방식으로 시뮬레이터를 구축한다. 이 방식은 Icarus Verilog의 vvp 런타임 접근과 유사하다 — 중간 표현을 생성하고 런타임에 해석한다. 인터프리터로 출발하는 이유는 명확하다: 정확성과 표준 준수를 먼저 확보한 다음 속도를 최적화해야 하기 때문이다.

**sim-ir가 경계 역할을 한다.** sim-ir를 좁고 안정된 표면으로 유지하면, MVP 인터프리터 이후 컴파일드 또는 JIT 백엔드를 재작성 없이 추가할 수 있다. 프론트엔드 전체(preprocess → lex → parse → elaborate)는 그대로 두고 실행 백엔드만 교체하는 구조다. Verilator의 접근(C++ 코드 생성)이 후속 단계의 모델이다.

**왜 컴파일드 방식을 MVP로 선택하지 않는가.** Verilator는 정적 macro-task 스케줄링 방식을 채택하며 동적 macro-dataflow 모델을 명시적으로 거부한다([Verilator internals.rst](https://github.com/verilator/verilator/blob/master/docs/internals.rst)). 그 정적 구조는 IEEE 1800 stratified event scheduling을 완전히 모델링하기 어렵다. 컴파일드 방식은 표준 IEEE 1800 스케줄링 의미론을 정확히 구현하기 어렵다. vitamin은 timescale 정밀도와 VCD 정확성을 1일차부터 확보해야 하므로 인터프리터 방식이 올바른 출발점이다.

---

## IR 설계 원칙

**sim-ir는 언어 비의존이다.** Verilog 구문도, SystemVerilog 타입도, VHDL 엔터티도 알지 못한다. net(폭/4-state), process(트리거 조건/본문), continuous assign, 계층 인스턴스, 초기값만 표현한다. 언어별 특성은 elaborate 단계에서 모두 소화된다.

**4-state(0/1/x/z)를 1급으로 표현한다.** Icarus의 functor가 0/1/x/z를 각 2비트로 인코딩하듯, sim-ir의 net과 process 값은 4-state를 기본 표현으로 갖는다. 시뮬레이션 초기화 시 x 상태, 멀티드라이버 z 해소, x-propagation이 모두 이 표현에 의존한다. 2-state 최적화(성능)는 후속 개선으로만 검토한다.

**builtin-call은 IR 노드 타입이다.** `$display(...)`, `$dumpvars` 같은 system task 호출은 AST에서 언어별 builtin call 노드로 표현되고, elaborate 단계에서 IR의 `builtin-call` 노드로 변환된다. 이 노드는 이름(예: `$display`), 타입이 확인된 인자 목록, 반환 타입을 담는다. sim-engine은 이 노드를 실행할 때 hdl-builtins의 디스패치 테이블을 조회한다.

**dump 태스크는 별도 표식으로 vcd-writer에 라우팅한다.** `$dumpfile`, `$dumpvars`, `$dumpon`/`$dumpoff`, `$dumpall`, `$dumpflush`, `$dumplimit` 는 IR에서 dump-family 표식을 갖는 builtin-call 노드로 표현된다. hdl-builtins의 디스패치 경로가 이 표식을 감지해 vcd-writer로 라우팅한다. 이 연결이 없으면 VCD는 생성되지 않는다.

**표면적을 좁게 유지한다.** sim-ir가 노출하는 타입과 trait을 최소화해 인터프리터와 컴파일드 백엔드 양쪽이 동일 IR을 소비할 수 있도록 계약을 단순하게 유지한다. Yosys RTLIL이 "all frontends must transform to RTLIL-compatible representation"을 강제하는 것과 같은 원칙이다.

**sim-ir는 span-free다(D4).** 소스 위치(파일 경로·바이트 범위)는 프론트엔드 잔재이므로 언어 중립 IR 노드에 넣지 않는다. 런타임 진단이 소스를 가리켜야 할 때는, IR 노드 인덱스로 키잉된 **선택적·독립 버전 사이드테이블**(`node_index → {file_id, byte_range}`)로 위치를 운반하고 `file_id→path` 맵은 work 매니페스트에 둔다(Yosys RTLIL이 `src`를 어트리뷰트로 오버레이하는 선례). 이로써 `SchemaHash` derive가 sim-ir의 중립 코어만 해시하고, 진단 위치가 백엔드 교체 경계를 넓히지 않는다. 상세는 [14-staged-artifacts.md](14-staged-artifacts.md) §7.

---

## Builtin Dispatch (hdl-builtins)

system task 호출 하나가 파이프라인을 어떻게 통과하는지 단계별로 따라간다.

**1. parser → AST:** HDL 소스의 `$xxx(arg1, arg2)` 구문을 parser가 인식해 언어별 builtin call AST 노드를 만든다. 이름(`$xxx`)과 파싱된 인자 표현식 목록을 담는다.

**2. elaborate → IR builtin-call:** elaborate 크레이트가 AST builtin call 노드를 소비하고 IR의 `BuiltinCall` 노드를 생성한다. 이 변환 과정에서 이름 검증(알려진 system task인지), 인자 타입 검사(개수·타입 정합), 반환 타입 결정이 수행된다. 알 수 없는 system task나 타입 불일치는 diag 크레이트를 통해 오류로 보고된다.

**3. sim-engine → hdl-builtins dispatch:** sim-engine이 이벤트 실행 중 `BuiltinCall` 노드를 만나면 hdl-builtins의 중앙 디스패치 테이블에 이름을 키로 조회한다. 디스패치 테이블은 이름 → 카테고리별 핸들러 함수 매핑이다.

**4. 카테고리별 핸들러 실행:** 핸들러는 범주별로 구분된다: display/I/O, 파일 I/O, 시뮬레이션 제어(`$finish`/`$stop`), 시간(`$time`/`$realtime`), 변환, 비트벡터, 수학, random, dump 패밀리, assertion 샘플링, introspection. 각 핸들러는 sim-engine에서 현재 시뮬레이션 컨텍스트(시간, net 상태)를 인자로 받는다.

**5. dump 패밀리 → vcd-writer 라우팅:** `$dumpfile`/`$dumpvars`/`$dumpon`/`$dumpoff`/`$dumpall`/`$dumpflush`/`$dumplimit` 핸들러는 vcd-writer를 직접 호출한다. 이 경로가 열려야만 VCD 파일이 생성된다. RTL 코드가 dump 태스크를 한 번도 호출하지 않으면 vcd-writer는 활성화되지 않는다.

system task 전 범주 참조: `hdl-reference/system-tasks/`.

---

## 레퍼런스 비교 (research 반영)

### Icarus Verilog — 인터프리터 선례

Icarus Verilog는 `iverilog`(컴파일러) + `vvp`(런타임)의 두 바이너리 구조다. `iverilog` 내부는 flex/bison 기반 lex/parse로 pform(장식된 파스 트리)을 만들고, elaborate 단계에서 netlist form으로 변환한 뒤 ivl_target API를 통해 code generator(tgt-vvp)에 전달한다. tgt-vvp는 텍스트 바이트코드 형식의 vvp 파일을 생성한다.

vvp 런타임은 구조층(functor net)과 행동층(thread)의 이중 구조다. functor는 4-state를 2비트로 인코딩(0→00, 1→01, x→10, z→11)하고 64바이트 진리표로 조합 논리를 표현한다. `.thread` 문이 initial/always 블록에 대응하는 스레드를 만들고, `%set`/`%assign`/`%load`/`%wait` 명령어로 functor net과 상호작용한다. 이벤트 큐는 skip list 기반이며 전파·대입·스레드 스케줄 세 종류 이벤트를 처리한다.

vitamin과의 유사점: 프론트엔드(iverilog)와 런타임(vvp) 분리, 4-state 1급 표현, 이벤트 구동 인터프리터. vitamin의 MVP 접근과 가장 유사한 출발점이다.

### Verilator — 컴파일드 방식과 그 트레이드오프

Verilator는 Verilog/SV를 C++로 변환해 일반 컴파일러가 최적화하게 한다. 파이프라인은 Flex/Bison 파스 → ~20단계 V3 AST pass → V3Order 정적 스케줄 계산 → V3EmitC C++ 출력이다. 결과로 나온 `_eval()` 함수는 Active/NBA 영역을 위에서 아래로 순서대로 실행한다 — 동적 이벤트 큐 없이.

핵심 트레이드오프: Verilator는 정적 macro-task 스케줄링을 채택하며 동적 macro-dataflow 모델을 명시적으로 거부한다 — internals.rst는 "Sarkar describes two options: you can dynamically schedule tasks at runtime... Verilator does not support this... The other option is to statically assign macro-tasks to threads... Verilator takes this static approach"라고 설명한다. 그 정적 구조는 IEEE 1800 stratified event scheduling을 완전히 모델링하기 어렵고, 컴파일드 방식은 속도를 얻지만 표준 IEEE 1800 스케줄링 의미론을 완전히 구현하지 않는다. 멀티스레딩(MTask)은 V3Partition이 의존성 그래프를 거칠게 합쳐 매크로태스크를 생성하고 정적으로 스케줄한다.

vitamin과의 관계: 후속 단계(Phase 3+)에서 컴파일드/JIT 백엔드를 추가할 때 Verilator 방식이 모델이 된다. sim-ir 경계가 프론트엔드 재작성 없이 이 교체를 가능하게 한다.

### vitamin의 위치

vitamin은 두 선례 사이에 의도적으로 위치한다. MVP는 Icarus처럼 인터프리터 방식으로 표준 준수를 먼저 확보한다. 그러나 sim-ir를 IR 경계로 두어 Verilator처럼 컴파일드 백엔드도 후속에 수용할 수 있다. 이 경계가 둘 다 가능하게 하는 핵심 설계 결정이다.

---

## Sources

- 설계 명세 §5 전체 (`docs/superpowers/specs/2026-05-26-vitamin-rtl-simulator-design.md`)
- research-log: [`eda-architectures-2026-05-28.md`](research-log/eda-architectures-2026-05-28.md)
- Icarus Verilog Developer Guide: https://steveicarus.github.io/iverilog/developer/guide/index.html
- Icarus VVP Simulation Engine: https://steveicarus.github.io/iverilog/developer/guide/vvp/vvp.html
- Verilator Internals: https://github.com/verilator/verilator/blob/master/docs/internals.rst
- Yosys RTLIL: https://yosyshq.readthedocs.io/projects/yosys/en/stable/yosys_internals/formats/rtlil_rep.html
