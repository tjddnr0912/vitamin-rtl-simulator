# Vitamin — RTL Simulation EDA Tool · 설계 명세 (Design Spec)

> **코드네임:** `vitamin` (임시 — 추후 재고) · **CLI 작업명:** `vita` (placeholder)
> **작성일:** 2026-05-26 · **수정:** 2026-05-28 · **상태:** 설계 확정 (브레인스토밍 합의 완료)
> **위치 안내:** 본 문서는 상위 *설계 명세*다. 실제 가이드/참조 문서 세트는 `docs/preview/`에 작성된다. 본 spec은 그 산출물의 청사진이자 단일 진실 공급원(single source of truth)이다.

---

## 1. 개요 / 비전

Ubuntu · RHEL · macOS에서 **소스 빌드로 동작**하는 RTL 시뮬레이션 EDA 툴을 만든다. 상용 레퍼런스는 Synopsys **VCS**, Cadence **Xcelium**이며, 오픈소스 레퍼런스(설계 비교·차등검증 대상)는 **Verilator**, **Icarus Verilog**다.

핵심 가치는 세 가지:

1. **정밀도(precision)** — `timescale` 개념을 충실히 구현해 근소한 타이밍 틀어짐도 잡아낸다.
2. **이식성(portability)** — OS가 교체되어도 원문 소스를 빌드해 동작한다.
3. **성능(performance)** — 저수준 언어(Rust)로 시뮬레이션 속도를 끌어올린다.

기능 범위는 **compile(파싱 + 문법 검사) → elaboration(파라미터 해소 + 계층/연결성/타입 검사) → simulation → VCD 파형 생성(RTL dump 태스크 호출 시)** 의 표준 RTL 시뮬레이터 흐름이다. 검사는 별도 단계가 아니라 각 단계 안에서 수행된다 — 문법 오류는 compile에서, 연결성·타입·다중구동 등 정합성 오류는 elaboration에서 잡는다.

## 2. 목표 / 비목표 / 성공 기준

### 2.1 목표 (in-scope)
- HDL 소스의 **preprocess → lex → parse → elaboration → simulation** 전체 파이프라인.
- **문법 검사**(parse 단계)와 **elaboration 단계 점검 항목**(파라미터 해소, 계층 연결성, 타입/포트 정합, 미연결/다중구동 등).
- **이벤트 구동(event-driven) 시뮬레이션** 커널 + `timescale` 정밀 시간 모델.
- **VCD 파형 생성**(IEEE 1364) — **RTL 내 dump 시스템 태스크 호출 시에만** 활성화. 자동 항상-덤프 아님. (§7)
- **표준 Verilog/SystemVerilog system tasks/functions(`$`로 시작) 전체 지원** — display·I/O·파일I/O·메모리 로드·시뮬레이션 제어·시간·변환·비트벡터·수학·random·VCD dump·assertion 샘플링·introspection 등 전 범주. (§5.2 `hdl-builtins` 크레이트, `hdl-reference/system-tasks/` 섹터 참조)
- **3개 HDL 지원**: SystemVerilog(IEEE 1800) → 그 부분집합인 Verilog(IEEE 1364) → VHDL(IEEE 1076). (로드맵은 §9)
- **3-OS 소스 빌드** (cargo) + CI 매트릭스.

### 2.2 비목표 (out-of-scope, 현 단계)
- 합성(synthesis) 툴 자체 구현 — 단, 참조 문서에는 각 구문의 **합성 가능 여부를 명기**한다.
- 컴파일드(네이티브/JIT) 시뮬레이션 백엔드 — IR 경계만 열어두고 후속 단계로 미룬다(§5.3).
- 파형 GUI 뷰어 — VCD는 GTKWave/Surfer 등 외부 뷰어로 확인.
- FST 등 VCD 외 파형 포맷 — 후속 확장으로만 기록.
- UPF/전력, SDF 타이밍 백애너테이션, DPI-C, 커버리지/UVM 등 고급 검증 기능.

### 2.3 성공 기준 (measurable)
- 대표 RTL 테스트벤치를 **Icarus Verilog / Verilator와 차등검증**했을 때 신호값·천이 시각이 일치.
- 생성 VCD가 표준 뷰어(GTKWave 등)에서 오류 없이 로드되고, **golden VCD와 정규화 diff 일치**.
- 동일 소스가 **Ubuntu · RHEL · macOS에서 동일 결과**로 빌드·실행.
- `timescale`가 다른 모듈이 혼재해도 전역 시간축이 어긋나지 않음(§6.3 정밀도 테스트 통과).
- 표준 system tasks/functions 컴플라이언스 코퍼스(범주별 최소 1개 케이스) 전수 통과.

## 3. 타깃 환경 (OS / 빌드)

- **OS:** Ubuntu (LTS), RHEL (8/9 계열), macOS (Apple Silicon + Intel).
- **빌드 철학:** 원문 소스 → 각 OS에서 빌드. 사전 빌드 바이너리 배포에 의존하지 않는다.
- **순수 Rust 코어 + 최소/제로 C 의존성.** 외부 C 라이브러리 의존을 피해 3-OS 빌드 마찰을 제거한다.
- **MSRV(최소 지원 Rust 버전) 고정** + `rust-toolchain.toml`로 재현성 확보.

## 4. 구현 언어 결정 (Rust)

**결정: Rust.** 후보 비교(설계 회의 합의):

| 후보 | 장점 | 단점 | 판정 |
|---|---|---|---|
| **Rust** | 메모리 안전 + C급 성능; enum/패턴매칭이 lexer·parser·elaborator·AST/IR에 이상적; cargo로 3-OS 재현 빌드; GC 없어 이산사건 시뮬레이터의 결정론적 정밀도 유지 | 러닝커브, 컴파일 시간 | **채택** |
| C / C++ | 검증된 EDA 경로(Verilator=C++, Icarus=C/C++), 최고 이식성/생태계 | 대규모 파서/시뮬레이터를 메모리 안전하게 유지·디버깅하는 부담 | 차점 |
| Go | 단순·빠른 빌드·쉬운 크로스컴파일 | GC가 결정론적 타임스케일 정밀도/처리량에 불리; AST 작업에 sum type 약함 | 코어 부적합 |

**근거 요약:** (a) 시뮬레이터 코어는 타이트한 이벤트 루프 — GC는 결정론적 정밀도/처리량에 부담. (b) 3개 HDL 프론트엔드(lexer/parser/elaborator)는 Rust의 enum + 패턴매칭이 가장 빛나는 영역. (c) 시뮬레이터 버그 = 조용한 오답이므로 메모리 안전성의 가치가 큼. (d) cargo가 3-OS 소스 빌드를 1급으로 지원. (e) 선례: `veryl`, `spade`, `sv-parser` 등.

**생태계 후보(조사·검증 대상, §11 방법론으로 확정):** lexer `logos`, parser `chumsky`/`lalrpop`/수작업 recursive-descent, 진단 `ariadne`/`codespan-reporting`, VCD 참고 `vcd` crate, SystemVerilog 파싱 선례 `sv-parser`.

> **확정 결과(2026-06-01, → `preview/02-implementation-language.md`):** parser = **`winnow` 부트스트랩 → 수작업 RD 이관**(chumsky는 GitHub archived 2026-04-02로 배제), 진단 = **`miette`**(`codespan-reporting` fallback; ariadne 미채택). MSRV = **1.82**(miette). 위 후보 목록은 조사 시점 스냅샷이다.

## 5. 시스템 아키텍처

### 5.1 파이프라인

```
source files
  → preprocess   (`define / `ifdef / `include / `timescale)
  → lex          (토큰 스트림)
  → parse        (언어별 AST)
  → elaborate    (파라미터 해소, 계층 평탄화, 타입/포트/연결성 검사)
  → sim IR       (nets + processes + sensitivity + builtin-call nodes, 언어 중립)
  → sim engine   (event-driven kernel + timescale time wheel + builtin dispatch)
  → VCD writer   (IEEE 1364 정규 포맷 · RTL dump 태스크가 호출될 때만 활성)
```

언어 의존부는 **parse까지**, 그 이후 elaborate→IR→engine→VCD는 **언어 중립**으로 설계한다. 그래야 VHDL 프론트엔드를 공유 IR 위에 얹을 수 있다(§9).

### 5.2 Cargo 워크스페이스 / 크레이트

> **STALE(2026-06-01).** 아래 11-크레이트 표는 초안 스냅샷이다. 권위 목록은 **`preview/04-architecture.md`(14 크레이트)** 와 `preview/03-build-and-portability.md`의 `members` 배열이다 — 이후 `hdl-ast`/`sim-ir`이 `vita-artifact-derive`에 의존하도록 바뀌었고, `vita-artifact`·`vita-artifact-derive`·`vita-log` 3개가 추가됐다. 단계 산출물·해시 결합·진단/로깅 상세는 `preview/14`·`preview/13` 참조.

각 크레이트는 단일 책임 + 명확한 인터페이스로 분리해 독립 테스트 가능하게 둔다.

| 크레이트 | 책임 | 의존 |
|---|---|---|
| `hdl-preprocess` | 컴파일러 지시어 처리, 매크로 전개, include | — |
| `hdl-lexer` | 토큰화 (언어별 토큰 집합) | preprocess |
| `hdl-parser` | 토큰 → AST (언어별) | lexer, ast |
| `hdl-ast` | 언어별 AST 타입 정의 | — |
| `elaborate` | 파라미터 해소·계층 평탄화·타입/연결성 검사 → IR 생성 | ast, sim-ir, diag |
| `sim-ir` | 언어 중립 시뮬레이션 IR (net/process/sensitivity/builtin-call) | — |
| `sim-engine` | 이벤트 구동 커널, 스케줄러, 시간 모델 | sim-ir |
| `hdl-builtins` | **표준 `$`-system tasks/functions 라이브러리** (display·I/O·file·sim ctrl·time·conv·bit·math·random·dump·assertion·introspection 전 범주) — 디스패치 테이블 + 카테고리별 핸들러 | sim-ir, sim-engine, vcd-writer |
| `vcd-writer` | IEEE 1364 VCD 직렬화 (dump 태스크가 호출될 때만 활성) | sim-ir |
| `diag` | 진단/오류 리포팅 (소스 위치, 메시지) | — |
| `cli` | 드라이버 바이너리 — `vita`(원샷) + `vcmp`/`velab`/`vrun`(단계별 compile/elab/sim) | 전부 |

**실행 모델:** `vita`는 compile→elaborate→simulation을 한 번에 도는 원샷 드라이버다. 추가로 단계별 드라이버 `vcmp`(compile) → `velab`(elaborate, `vcmp` 산출물 소비) → `vrun`(simulation, `velab` 산출물 소비)을 제공해 단계를 분리 실행한다. 이 3단계는 Cadence(`xmvlog`/`xmelab`/`xmsim`)·Synopsys(`vlogan`/`vcs`/`simv`) 흐름에 대응하며, 단계별 독립 빌드·디버깅과 변경 없는 단계 스킵(산출물 재사용)을 가능하게 한다.

### 5.3 시뮬레이션 전략 — 하이브리드 (인터프리터 우선)

- **MVP:** IR-walking **인터프리터** 방식 이벤트 구동 시뮬레이터(Icarus 계열). 정확성·VCD·타이밍 정밀도를 먼저 확보.
- **후속:** `sim-ir`를 깨끗한 경계로 유지해, 추후 **컴파일드/JIT 백엔드**(Verilator 계열)를 재작성 없이 추가.
- **이유:** "올바른" 시뮬레이터를 먼저 만들고 속도는 IR 뒤에서 최적화. SV-first·정밀도 중심 목표에 부합하며 MVP가 현실적.

### 5.4 IR 설계 원칙

- `sim-ir`는 **언어 비의존**: net(폭/4-state), process(트리거 조건/본문), continuous assign, 계층 인스턴스, 초기값을 표현.
- 4-state 논리(`0/1/x/z`)를 1급으로 표현(2-state 최적화는 후속).
- 인터프리터/컴파일드 백엔드 양쪽이 동일 IR을 소비하도록 표면적을 좁게 유지.
- **system tasks 호출은 IR에서 named builtin-call 노드**로 표현 → `hdl-builtins` 디스패치 테이블이 해소. dump 태스크는 별도 표식으로 vcd-writer에 라우팅.

## 6. 시뮬레이션 엔진 (정밀도의 핵심)

### 6.1 이벤트 구동 커널 / stratified event queue

Verilog/SV 표준 스케줄링의 **계층화 이벤트 영역**을 구현한다. 최소 핵심: **Active → Inactive(`#0`) → NBA(`<=`) → Monitor**. SV 확장(Preponed/Observed/Reactive/Postponed 등 — assertion·program block용)은 후속 단계에서 추가. 이 영역 분리가 blocking(`=`) vs non-blocking(`<=`)의 결정론적 의미를 보장한다.

### 6.2 delta cycle

동일 시뮬레이션 시각에서 신호가 안정될 때까지 0-time 반복(delta cycle)을 수행. 조합 논리 전파와 race 회피의 기반.

### 6.3 timescale / time precision — 64-bit 정수 시간

- **전역 시간 = 64-bit 정수**, 단위는 설계 전체에서 가장 **미세한 precision**. 부동소수 시간 누적 오차 없음.
- 모듈별 `timescale <unit>/<precision>`의 unit/precision을 전역 precision 기준 **정수 배율**로 환산.
- 지연(`#delay`)은 모듈 unit → 전역 precision으로 정수 변환 후 스케줄.
- precision 반올림 규칙을 표준에 맞춰 명문화(예: `#1.55` + precision 환산).
- **정밀도 테스트:** 서로 다른 `timescale` 모듈을 혼재시킨 설계에서 천이 시각이 1 precision 단위까지 정확히 일치하는지 회귀로 검증.

## 7. VCD 파형 생성 (IEEE 1364) — RTL-driven

**중요: VCD 덤프는 항상 자동 수행되지 않는다.** RTL 코드에서 명시적으로 호출한 dump 시스템 태스크를 시뮬레이션 런타임이 식별·실행해 VCD를 생성한다. 단일 실행에서 dump 태스크가 한 번도 호출되지 않으면 VCD는 생성되지 않는다(no-op).

**인식·지원할 dump 시스템 태스크 (IEEE 1364 §18 기반):**
- `$dumpfile("name.vcd")` — 출력 파일 지정.
- `$dumpvars` — 덤프 대상 신호 선언 (인자 없으면 전체, `(level, scope)` 인자 시 depth + 지정 scope/신호).
- `$dumpon` / `$dumpoff` — 덤프 일시 재개/정지.
- `$dumpall` — 현재 모든 값 강제 기록.
- `$dumpflush` — 버퍼 플러시.
- `$dumplimit(size)` — 파일 크기 제한.

CLI 편의 플래그(예: `vrun --force-dump`)는 선택적 후속 기능으로만 검토 — **기본은 RTL이 주도**한다.

**파일 포맷 — IEEE 1364:**
- 헤더: `$date` · `$version` · `$timescale` · `$scope`/`$upscope`(계층) · `$var`(신호 선언·식별자 코드) · `$enddefinitions`.
- 초기 덤프: `$dumpvars` 호출 시점의 신호 상태.
- 값 변화부: `#<time>` 시각 마커 + 스칼라(`0/1/x/z`+id) / 벡터(`b<bits> <id>`) / 실수(`r<real> <id>`) 변화.

정확한 토큰·문법은 §11 방법론으로 표준 대조 후 `07-vcd-format.md`에 **golden 포맷 명세**로 고정. 확장 VCD(`$dumpports*`)·FST는 비목표(후속 기록만).

## 8. 검증 전략 (differential testing)

- **차등검증:** 동일 소스를 **Icarus Verilog(`iverilog`+`vvp`)** 와 **Verilator**로 돌려 신호값·천이 시각 비교.
- **VCD golden diff:** 생성 VCD를 정규화 후 골든과 비교(식별자 코드 차이를 흡수하는 정규화기 포함).
- **컴플라이언스 서브셋:** 언어 기능별 + **system tasks 범주별** 소형 테스트 코퍼스 누적.
- **단위 테스트:** 각 크레이트(lexer/parser/elaborate/engine/builtins/vcd) 독립 테스트. TDD 지향.

## 9. 로드맵 (SystemVerilog-first 단계)

- **Phase 1 — MVP:** SystemVerilog **합성가능 RTL 서브셋**(= Verilog-2005 RTL 전부 포함). preprocess→lex→parse→elaborate→event-driven sim→VCD. 인터프리터 백엔드. timescale 정밀도 + VCD는 1일차부터. **system tasks 핵심 셋**: `$display`/`$write`/`$monitor`/`$strobe`, `$time`/`$realtime`, `$finish`/`$stop`, dump 패밀리(`$dumpfile`/`$dumpvars`/`$dumpon`/`$dumpoff`/`$dumpall`).
- **Phase 2 — SV 확장:** interface/modport, package, struct/enum/typedef, always_comb/_ff/_latch, 추가 구문. **system tasks 확장**:
  - 파일 I/O: `$fopen`/`$fclose`/`$fwrite`/`$fdisplay`/`$fread`/`$fscanf`/`$fgets`/`$sscanf`/`$sformat`/`$sformatf`
  - 메모리 로드: `$readmemh`/`$readmemb`/`$writememh`/`$writememb`
  - 변환: `$signed`/`$unsigned`/`$rtoi`/`$itor`/`$bitstoreal`/`$realtobits`
  - 비트벡터: `$bits`/`$clog2`/`$countones`/`$countbits`/`$onehot`/`$onehot0`/`$isunknown`
  - 수학: `$pow`/`$ln`/`$log10`/`$exp`/`$sqrt`/`$sin`/`$cos`/`$tan`
  - random: `$random`/`$urandom`/`$urandom_range`/`$dist_*`
  - assertion 샘플링: `$past`/`$rose`/`$fell`/`$stable`/`$changed`/`$sampled`
  - introspection: `$typename`/`$cast`/`$size`/`$left`/`$right`/`$low`/`$high`/`$increment`
  - 기타: `$value$plusargs`/`$test$plusargs`/`$system`
- **Phase 3 — VHDL:** IEEE 1076 프론트엔드를 **공유 IR 위에** 별도 구축(std_logic_1164/numeric_std 포함).
- **상시 횡단:** timescale 정밀도, VCD, 차등검증, 진단 품질, system tasks 컴플라이언스 코퍼스.
- **후속(비목표→여력 시):** 컴파일드/JIT 백엔드, FST, SV assertion 영역 확장, 확장 VCD(`$dumpports*`).

## 10. 산출물: `docs/preview/` 구조

상위 기획 문서(00–12) + 섹터 분할 HDL 참조(`hdl-reference/`). 각 문서 하단에 자체 `## Sources` 섹션 유지.

```
docs/preview/
├── 00-overview.md               # 비전, VCS/Xcelium/Verilator/Icarus 비교, 범위
├── 01-goals-and-scope.md        # 목표/비목표, 성공기준, 타깃 OS, MVP 정의
├── 02-implementation-language.md # Rust 선정 근거 + 대안 비교 + 크레이트 생태계
├── 03-build-and-portability.md  # cargo workspace, 3-OS 소스빌드, CI 매트릭스, MSRV
├── 04-architecture.md           # 파이프라인 + 크레이트 + IR 설계 + builtin 디스패치
├── 05-strategy-and-roadmap.md   # SV-first 단계별 로드맵, 마일스톤, 리스크
├── 06-simulation-engine.md      # event-driven 커널, stratified queue, delta cycle
├── 07-vcd-format.md             # VCD 정규 명세 + 생성기 설계 (golden 포맷, RTL-driven)
├── 08-timescale-and-timing.md   # timescale/precision, 64-bit 정수 시간, 이벤트 영역
├── 09-testing-and-verification.md # iverilog/verilator 차등검증, VCD diff, 컴플라이언스
├── 10-glossary.md               # 용어집
├── 11-sources-and-citations.md  # 출처·저작권·인용 정책 (WHERE / 귀속)
├── 12-research-methodology.md   # 조사 방법론 (HOW / research 스킬: WebSearch + WebFetch 다라운드, 1차 source 검증)
├── research-log/                # 라운드별 WebSearch 쿼리·WebFetch URL·요약 (재현성·투명성)
│   └── README.md                # 로그 작성 규약
└── hdl-reference/               # 섹터별 분할 (용량 큼)
    ├── README.md                # 얇은 네비게이션 인덱스 (각 항목 1줄 + 읽기 순서)
    ├── 00-standards-map.md      # IEEE 1800/1364/1076/1164 매핑·버전·상호관계
    ├── 01-synthesizability-legend.md # ✅가능 / ⚠️조건부 / ❌비합성 표기 범례 (공통)
    ├── system-tasks/            # ★ 표준 $-system tasks/functions 카테고리별 참조
    │   ├── 00-index.md          # 전 system tasks 인벤토리 + Phase 1/2/3 커버리지 매트릭스
    │   ├── 01-display-io.md     # $display/$write/$monitor/$strobe (+ b/o/h 변형)
    │   ├── 02-file-io.md        # $fopen/$fclose/$fwrite/$fdisplay/$fread/$fscanf/$fgets/$sscanf/$sformat/$sformatf
    │   ├── 03-memory-load.md    # $readmemb/$readmemh/$writememb/$writememh
    │   ├── 04-simulation-control.md # $finish/$stop/$exit
    │   ├── 05-time-functions.md # $time/$stime/$realtime
    │   ├── 06-conversion.md     # $signed/$unsigned/$rtoi/$itor/$bitstoreal/$realtobits
    │   ├── 07-bit-vector.md     # $bits/$clog2/$countones/$countbits/$onehot/$onehot0/$isunknown
    │   ├── 08-math.md           # $pow/$ln/$log10/$exp/$sqrt/$sin/$cos/$tan 등
    │   ├── 09-random.md         # $random/$urandom/$urandom_range/$dist_*
    │   ├── 10-vcd-dump.md       # $dumpfile/$dumpvars/$dumpon/$dumpoff/$dumpall/$dumpflush/$dumplimit (+ 확장 $dumpports*)
    │   ├── 11-assertion-sampling.md # $past/$rose/$fell/$stable/$changed/$sampled/$assertoff/$asserton/$assertkill
    │   ├── 12-introspection.md  # $typename/$cast/$isunbounded/$size/$left/$right/$low/$high/$increment
    │   └── 13-misc.md           # $value$plusargs/$test$plusargs/$system 등
    ├── verilog/
    │   ├── 00-index.md
    │   ├── 01-lexical.md        # 토큰·식별자·숫자 리터럴·문자열·주석
    │   ├── 02-data-types.md     # net/wire/tri, reg, integer, real, time, vector, parameter
    │   ├── 03-expressions-operators.md
    │   ├── 04-modules-hierarchy.md  # module/port/instantiation/parameter/generate
    │   ├── 05-behavioral.md     # initial/always, blocking(=)/non-blocking(<=)
    │   ├── 06-procedural-statements.md # if/case/for/while/repeat/forever, fork-join
    │   ├── 07-tasks-functions.md
    │   ├── 08-gate-level.md     # primitives, UDP, drive strength
    │   ├── 09-compiler-directives.md # `timescale/`define/`ifdef/`include/`default_nettype
    │   ├── 10-system-tasks.md   # 언어별 개요 + ../system-tasks/ 카테고리 docs로 cross-link
    │   └── 11-synthesizability.md
    ├── systemverilog/
    │   ├── 00-index.md
    │   ├── 01-data-types.md     # logic/bit/int, enum/struct/union/typedef, 2-state vs 4-state
    │   ├── 02-arrays.md         # packed/unpacked, dynamic, associative, queue
    │   ├── 03-procedural.md     # always_comb/_ff/_latch, unique/priority, foreach
    │   ├── 04-interfaces.md     # interface, modport, clocking block
    │   ├── 05-packages.md       # package, import/export, $unit
    │   ├── 06-classes-oop.md    # class/상속 (검증용, 비합성)
    │   ├── 07-assertions-sva.md # immediate/concurrent assertion, property, sequence
    │   ├── 08-functions-tasks.md  # SV 추가 system tasks 개요 + ../system-tasks/ cross-link
    │   └── 09-synthesizability.md
    └── vhdl/
        ├── 00-index.md
        ├── 01-lexical.md
        ├── 02-types.md          # scalar/composite, std_logic_1164, numeric_std
        ├── 03-objects.md        # signal/variable/constant/generic
        ├── 04-design-units.md   # entity/architecture/package/configuration/library
        ├── 05-concurrent.md     # process, concurrent assign, component, generate
        ├── 06-sequential.md     # if/case/loop, wait, variable assign
        ├── 07-subprograms.md    # function/procedure
        ├── 08-packages-libraries.md # ieee, std_logic_1164, numeric_std, std
        └── 09-synthesizability.md
```

## 11. 조사 방법론 (`research` 스킬 기반)

**원칙: 다라운드 + 다각도 + 1차 source 직접 확인.** 단일 소스에 의존하지 않고 라운드마다 의도적으로 다른 각도(쿼리·언어·시각)에서 검색·검증한다. 이 방법론은 `12-research-methodology.md`에 명문화하며, 실제 조사 시점에 `research` 스킬을 호출해 수행한다.

**사용 도구 (research 스킬 백엔드):** Anthropic **WebSearch** + **WebFetch** 기반 다라운드 조사. `gemini -p` 서브에이전트 방식은 **폐기**(이전 백엔드).

1. **Claude 내부 지식** — 빠른 초안·골격(baseline).
2. **WebSearch (다각도)** — 라운드마다 의도적으로 다른 쿼리/언어(영/한)/시각으로 검색해 편향 회피.
3. **WebFetch — 1차 source 직접 확인** — IEEE LRM 참조 페이지·표준 본문·Verilator/Icarus 공식 문서 등 핵심 원문을 **직접 읽어** hallucination을 차단(Phase 1.5 source 검증).

**워크플로우:** 다각도 WebSearch → WebFetch로 핵심 source 검증 → Claude **5차원 체크리스트**로 gap 점검 → 후속 라운드(다른 각도) **최대 4라운드** → 한국어 narrative로 수렴. 소스 충돌 시 양쪽 기록 + **IEEE LRM(1차 표준) 우선**. 검색 쿼리·fetch URL·라운드 요약은 `research-log/`에 기록(재현성).

**헤더 규약:** research 스킬 규약 준수 — 본문 헤더에 한자/추상 명사(예: 起承轉結·기승전결·"도입/본론/결론")는 **사용하지 않고**, 주제에서 따온 구체 헤더만 쓴다.

## 12. 출처 / 저작권 / 인용 정책

- IEEE 표준은 저작권/유료다. **요약·인용은 하되 원문 verbatim 복제는 금지.**
- 자유 이용 가능 자료(공개 LRM 참조, 공개 BNF/문법, IEEE Get 프로그램, Verilator/Icarus 문서, 대학 강의자료 등)를 우선.
- 각 문서 하단 `## Sources`에 URL·표준 번호·접근일 명시. 전역 정책은 `11-sources-and-citations.md`.

## 13. 용어 (요약, 상세는 `10-glossary.md`)

- **Elaboration:** 파라미터 확정·계층 인스턴스화·연결 해소로 시뮬레이션 가능한 모델을 구성하는 단계.
- **Delta cycle:** 동일 시각에서 신호 안정화를 위한 0-time 반복.
- **NBA:** Non-Blocking Assignment(`<=`)의 갱신을 모으는 이벤트 영역.
- **VCD:** Value Change Dump, IEEE 1364 정의 파형 텍스트 포맷.
- **timescale:** `unit/precision`. 지연 단위와 반올림 분해능.
- **system task/function:** `$`로 시작하는 표준 빌트인(예: `$display`/`$dumpvars`/`$bits`/`$urandom`). 본 프로젝트의 `hdl-builtins` 크레이트가 구현.

## 14. 리스크 / 미해결 사항

- **표준 정확도:** VCD 문법·이벤트 영역·timescale 반올림은 §11 방법론으로 표준 대조 필수(가장 정밀도에 민감).
- **SV 범위 통제:** SystemVerilog는 방대 — Phase 1은 합성가능 RTL 서브셋으로 **엄격히 제한**.
- **차등검증 도구 차이:** Icarus와 Verilator의 의미가 미묘히 다를 수 있음 → 표준을 최종 권위로.
- **system tasks 의미 일치:** `$urandom` seed/스트림, `$random` 분포, `$readmemh` 주소 해석 등 도구별 비결정 영역 — **표준 + Icarus 의미**를 기준으로 명시.
- **문서 언어:** 본 spec/가이드는 한국어 + 영문 기술용어. 영문 전환 원하면 spec 리뷰에서 지정.
- **CLI 이름 `vita`·`vcmp`·`velab`·`vrun`:** placeholder. 코드네임 `vitamin`과 함께 추후 확정.

## Sources (본 spec의 근거)

- 설계 회의(브레인스토밍) 합의 — 2026-05-26 세션 + 2026-05-28 반영(조사 방법론 변경·VCD RTL-driven·system tasks 전수 지원).
- 표준 식별(상세 대조는 작성 단계 §11에서 수행): Verilog **IEEE 1364**, SystemVerilog **IEEE 1800**, VHDL **IEEE 1076**, **IEEE 1164**(std_logic_1164), VCD(IEEE 1364 §18 정의), dump 시스템 태스크(IEEE 1364 §18).
- 레퍼런스 도구: Synopsys VCS, Cadence Xcelium(상용); Verilator, Icarus Verilog(오픈소스).
- 환경 확인: `research` 스킬 사용 가능 (Anthropic WebSearch + WebFetch 다라운드 백엔드, 5차원 체크리스트, 최대 4라운드).
