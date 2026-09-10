# Vitamin `docs/preview/` Set Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 설계 spec `docs/superpowers/specs/2026-05-26-vitamin-rtl-simulator-design.md`에 따라 `docs/preview/` 가이드/참조 문서 세트(약 60개 markdown 파일)를 `research` 스킬 다라운드 조사를 거쳐 작성한다.

**Architecture:** 7 그룹 순차 실행 — (A) 스캐폴드 → (B) 12 기획 문서 → (C) HDL 참조 골격 3 → (D) system-tasks 14 → (E) Verilog 참조 12 → (F) SystemVerilog 참조 10 → (G) VHDL 참조 10. 각 그룹은 그 자체로 사용 가능한 자료를 산출하므로 자연스러운 체크포인트. spec에서 직접 도출 가능한 문서와 research 필수 문서를 task마다 명시.

**Tech Stack:** Markdown · `research` 스킬(Anthropic WebSearch + WebFetch 다라운드, 1차 source 검증, 5차원 체크리스트, 최대 4라운드) · git(`Add/Fix/Update/Refactor` 접두사 + Co-Authored-By 푸터).

**Branch:** `feat/vitamin-docs-preview`. main 브랜치 직접 작업 금지.

**문서 언어:** 한국어 + 영문 기술용어 (spec 언어와 일치). 헤더에 한자/추상명사(起承轉結·기승전결·"도입/본론/결론") 금지 — 주제에서 따온 구체 헤더만.

**공통 규칙 (모든 task 적용):**
1. 모든 문서는 하단 `## Sources` 섹션 필수 (URL · 표준 번호 · 접근일).
2. IEEE 표준 원문 verbatim 복제 금지. 요약·인용만.
3. research 스킬 사용 시 출력을 `docs/preview/research-log/<topic-slug>-YYYY-MM-DD.md`로 보관.
4. 커밋 푸터:
   ```
   Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
   ```
5. 한 task 완료 = 한 커밋. push는 사용자 명시 지시 시에만.

---

## Group A — 스캐폴드 (Task 0~1)

### Task 0: 작업 브랜치 생성

**Files:** (브랜치 작업만)

- [ ] **Step 1: 현재 git 상태 확인**

```bash
cd /Users/seongwookjang/project/git/violet_sw/016_claude_rtl
git status
git branch --show-current
```
예상: `main` 브랜치.

- [ ] **Step 2: feature 브랜치 생성·전환**

```bash
git checkout -b feat/vitamin-docs-preview
```
예상 출력: `Switched to a new branch 'feat/vitamin-docs-preview'`.

- [ ] **Step 3: 확인**

```bash
git branch --show-current
```
예상 출력: `feat/vitamin-docs-preview`.

---

### Task 1: 디렉터리 스캐폴드 + research-log 규약

**Files:**
- Create: `docs/preview/research-log/README.md`
- Implicit dirs: `docs/preview/{research-log,hdl-reference/{system-tasks,verilog,systemverilog,vhdl}}/`

- [ ] **Step 1: 디렉터리 일괄 생성**

```bash
mkdir -p docs/preview/research-log
mkdir -p docs/preview/hdl-reference/system-tasks
mkdir -p docs/preview/hdl-reference/verilog
mkdir -p docs/preview/hdl-reference/systemverilog
mkdir -p docs/preview/hdl-reference/vhdl
```

- [ ] **Step 2: `docs/preview/research-log/README.md` 작성**

다음 내용 그대로 사용:

````markdown
# Research Log

본 폴더는 `research` 스킬 호출 시점의 1차 자료를 라운드별로 보관해 재현성·투명성을 확보한다. spec §11 방법론의 산물.

## 파일 명명
`<topic-slug>-YYYY-MM-DD.md` — 예: `vcd-format-2026-05-29.md`, `sv-data-types-2026-06-02.md`.

## 파일 구조
각 로그는 다음 머리말로 시작한다:

```yaml
---
topic: <짧은 영문 슬러그>
date: YYYY-MM-DD
rounds: <실행 라운드 수, 1~4>
primary_sources_fetched:
  - https://...
queries:
  - "Round 1 영문 쿼리"
  - "Round 1 한국어 쿼리"
  - "Round 2 ..."
---
```

본문은 research 스킬의 narrative 출력을 그대로 첨부하고, 하단 `## Sources` 섹션에 인용 URL을 다시 정리한다.

## 사용 규칙
- 동일 topic 재조사 시 새 날짜 파일 추가, 이전 파일은 **보존**(라운드 이력).
- 학술/상용 paywall(IEEE 표준 등) 내용 **verbatim 복제 금지** — 요약·인용만.
- 라운드별 쿼리 다양성(영/한 언어 혼용, 다른 각도)을 의도적으로 확보.
````

- [ ] **Step 3: 커밋**

```bash
git add docs/preview/research-log/README.md
git commit -m "Add docs/preview scaffolding + research-log convention

- Create docs/preview/ subtree (research-log/, hdl-reference/{system-tasks,verilog,systemverilog,vhdl}/)
- Add research-log README defining filename + YAML header + citation convention per spec §11

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Group B — 12 기획 문서 (Task 2~9)

### Task 2: 00-overview.md + 01-goals-and-scope.md

**Files:**
- Create: `docs/preview/00-overview.md`
- Create: `docs/preview/01-goals-and-scope.md`

두 문서 모두 **spec §1·§2·§3·§9에서 직접 도출** — research 불필요.

- [ ] **Step 1: `00-overview.md` 작성 — 다음 섹션 구조**

```
# 00 · Vitamin 개요

## 비전
(spec §1 요약)

## 무엇을 만드나
HDL → compile → elaboration → simulation → (RTL dump 시) VCD. 검사는 각 단계 내부.

## 레퍼런스 도구 비교
표: Synopsys VCS · Cadence Xcelium · Icarus Verilog · Verilator (카테고리/강점/본 프로젝트 위치)

## 본 프로젝트의 차별점
- Rust 코어 (메모리 안전 + C급 + GC 없음)
- 3-OS 소스 빌드 (cargo)
- 하이브리드 (인터프리터 MVP → IR 경계 너머 컴파일드 옵션)

## Sources
- 본 spec §1·§2 (path: docs/superpowers/specs/2026-05-26-vitamin-rtl-simulator-design.md)
- Synopsys VCS: https://www.synopsys.com/verification/simulation/vcs.html
- Cadence Xcelium: https://www.cadence.com/.../xcelium-simulator.html
- Icarus Verilog: https://steveicarus.github.io/iverilog/
- Verilator: https://www.veripool.org/verilator/
```

- [ ] **Step 2: `01-goals-and-scope.md` 작성 — 다음 섹션 구조**

```
# 01 · 목표 · 범위 · 성공 기준

## 목표 (in-scope)
(spec §2.1 그대로 + 가이드 독자 친화적 부연)

## 비목표 (out-of-scope, 현 단계)
(spec §2.2)

## 성공 기준 (측정 가능)
(spec §2.3)

## 타깃 환경
Ubuntu(LTS) · RHEL(8/9) · macOS(AS+Intel). 순수 Rust 코어 + 최소/제로 C 의존성. MSRV 고정.

## Phase 1 (MVP) 정의
SV 합성가능 RTL 서브셋 = Verilog-2005 RTL 전부 포함. preprocess→lex→parse→elaborate→sim→VCD. 인터프리터 백엔드. system tasks 핵심 셋(`$display`/`$write`/`$monitor`/`$strobe`, `$time`/`$realtime`, `$finish`/`$stop`, dump 패밀리).

## Sources
- 본 spec §2·§3·§9
```

- [ ] **Step 3: 커밋**

```bash
git add docs/preview/00-overview.md docs/preview/01-goals-and-scope.md
git commit -m "Add 00-overview + 01-goals-and-scope

Direct from spec §1, §2, §3, §9. Vision, tool comparison, scope, success criteria, MVP.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: 02-implementation-language.md + 03-build-and-portability.md

**Files:**
- Create: `docs/preview/02-implementation-language.md`
- Create: `docs/preview/03-build-and-portability.md`
- Create: `docs/preview/research-log/rust-hdl-ecosystem-YYYY-MM-DD.md` (research 산출물)

`02-language`는 spec §4 + Rust HDL 생태계 1 라운드 research. `03-build`는 spec §3·§5.2.

- [ ] **Step 1: research 1 라운드 호출 — Rust HDL/EDA 생태계**

`research` 스킬 호출 프롬프트(그대로 사용):

```
research Rust HDL/EDA ecosystem crates 2025-2026: (1) sv-parser by dalance — current version, IEEE 1800 coverage, maintenance status; (2) veryl HDL transpiler current; (3) spade HDL current; (4) vcd crate (Kevin Mehall) — current API + alternatives; (5) logos lexer current; (6) chumsky vs lalrpop comparison for HDL parsing; (7) ariadne / codespan-reporting for diagnostics; (8) precedent simulators in Rust (Marlin? others). Cite crates.io URLs and GitHub repos with stars/last-commit dates. 한국어 narrative로 정리.
```

결과를 `docs/preview/research-log/rust-hdl-ecosystem-<today>.md`에 YAML 머리말 + narrative + Sources 형식으로 저장.

- [ ] **Step 2: `02-implementation-language.md` 작성 — 다음 섹션**

```
# 02 · 구현 언어 결정 (Rust)

## 결정
Rust 채택.

## 후보 비교
(spec §4 표 + 가이드 부연 1~2줄/행)

## 채택 근거
(spec §4의 a~e 5개)

## Rust HDL/EDA 생태계 (research 반영)
<!-- SUPERSEDED 2026-06-01: 아래는 작성 시점 템플릿. 확정 결정은 preview/02 참조 —
     parser = winnow 부트스트랩→hand-RD(chumsky archived 배제), 진단 = miette(ariadne 미채택), MSRV = 1.82 -->
- lexer: `logos` (버전: research에서 가져옴)
- parser 후보: `chumsky` vs `lalrpop` vs 수작업 RD — 본 프로젝트 권장 (선정 근거 1~2줄)
- 진단: `ariadne` · `codespan-reporting`
- 참조 선례: sv-parser (dalance), veryl, spade — 각 1줄 설명
- VCD: `vcd` crate (참고용)

## MSRV / Toolchain
- `rust-toolchain.toml` 사용 (channel, components)
- MSRV 정책: ...

## Sources
- 본 spec §4
- research-log: `rust-hdl-ecosystem-<date>.md`
- crates.io / GitHub URLs from research
```

- [ ] **Step 3: `03-build-and-portability.md` 작성 — 다음 섹션**

```
# 03 · 빌드 · 이식성

## 빌드 철학
원문 소스 → 각 OS에서 빌드. 사전 빌드 바이너리 의존 금지.

## Cargo Workspace 구조
(spec §5.2 크레이트 표 + 워크스페이스 Cargo.toml 예시)

```toml
[workspace]
members = ["crates/hdl-preprocess", "crates/hdl-lexer", "crates/hdl-parser", ... ]
resolver = "2"

[workspace.package]
edition = "2024"
rust-version = "1.80"  # 예시 MSRV
```

## MSRV · Toolchain 고정
- `rust-toolchain.toml` 예시 코드 블록
- MSRV 정책: ...

## 3-OS 매트릭스
| OS | 패키지 매니저 | Rust | 비고 |
| Ubuntu LTS | apt | rustup | 기준 |
| RHEL 8/9 | dnf | rustup | glibc 호환 |
| macOS AS+Intel | brew | rustup | universal 빌드 가이드 |

## CI 매트릭스
GitHub Actions YAML 예시 (matrix: ubuntu-latest, macos-latest, RHEL self-hosted/UBI 컨테이너).

## 외부 의존성 정책
순수 Rust crate 우선. C 라이브러리 의존은 회피, 불가피하면 build.rs 명시 + 3-OS 검증.

## Sources
- 본 spec §3·§5.2
```

- [ ] **Step 4: 커밋**

```bash
git add docs/preview/02-implementation-language.md docs/preview/03-build-and-portability.md docs/preview/research-log/rust-hdl-ecosystem-*.md
git commit -m "Add 02-implementation-language + 03-build-and-portability

- 02: Rust 채택 근거 + 크레이트 생태계 검토 (research 1 라운드)
- 03: cargo workspace, MSRV, 3-OS 매트릭스, CI

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: 04-architecture.md

**Files:**
- Create: `docs/preview/04-architecture.md`
- Create: `docs/preview/research-log/eda-architectures-YYYY-MM-DD.md`

spec §5 전체 + Icarus/Verilator 아키텍처 비교 research.

- [ ] **Step 1: research 1~2 라운드 — EDA 시뮬레이터 내부 아키텍처**

프롬프트:

```
research open-source HDL simulator internal architecture: (1) Icarus Verilog pipeline — lex/parse/elaborate/vvp bytecode interpretation, IR structure; (2) Verilator — how it generates C++, the internal mt task graph and stratified scheduling; (3) Yosys (synthesis but instructive) IR. Focus on lessons for an event-driven interpreter Rust simulator: IR design, hierarchy flattening, separation between language frontend and simulation core. Cite primary docs (Icarus iverilog docs, Verilator docs, Yosys manual). 한국어 narrative.
```

결과 → `research-log/eda-architectures-<today>.md`.

- [ ] **Step 2: `04-architecture.md` 작성 — 다음 섹션**

```
# 04 · 시스템 아키텍처

## 파이프라인
(spec §5.1 다이어그램 그대로 + 단계별 1~2 단락 설명)

## Cargo 워크스페이스 / 크레이트
(spec §5.2 표 + 각 크레이트별 책임 1 단락)

## 하이브리드 시뮬레이션 전략
(spec §5.3) — 인터프리터 MVP → IR 경계 너머 컴파일드 옵션. 이유: 정확성·VCD·timescale 정밀도 먼저.

## IR 설계 원칙
(spec §5.4) — 언어 비의존, 4-state 1급, builtin-call 노드, 양 백엔드 동일 IR 소비.

## Builtin Dispatch (hdl-builtins)
시스템 태스크 호출 경로:
- parser: `$xxx(...)` → AST builtin call 노드
- elaborate: AST → IR builtin-call (이름·인자 타입·반환 타입 검증)
- engine: IR builtin-call 실행 시 hdl-builtins의 디스패치 테이블 조회 → 카테고리별 핸들러
- dump 패밀리는 별도 표식 → vcd-writer 라우팅

## 레퍼런스 비교 (research 반영)
- Icarus: 인터프리터, vvp 바이트코드. 본 프로젝트와 유사 출발점.
- Verilator: 컴파일드 (C++ 생성). 후속 phase의 모델.
- 본 프로젝트의 위치: IR 경계가 둘 다 가능하게 함.

## Sources
- 본 spec §5
- research-log: `eda-architectures-<date>.md`
- Icarus / Verilator 공식 문서 URL
```

- [ ] **Step 3: 커밋**

```bash
git add docs/preview/04-architecture.md docs/preview/research-log/eda-architectures-*.md
git commit -m "Add 04-architecture

Pipeline + 10-crate workspace + hybrid simulation + IR + builtin dispatch + EDA reference comparison (research)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: 05-strategy-and-roadmap.md

**Files:**
- Create: `docs/preview/05-strategy-and-roadmap.md`

spec §9 직접 도출. research 불필요.

- [ ] **Step 1: 문서 작성 — 다음 섹션**

```
# 05 · 전략 · 로드맵 (SystemVerilog-first)

## 전략 요지
SV(IEEE 1800)이 Verilog(IEEE 1364)를 흡수. 단일 SV 프론트엔드로 두 언어 동시 커버. VHDL은 별도 프론트엔드를 공유 IR 위에.

## Phase 1 — MVP
- 범위: SV 합성가능 RTL 서브셋 (= Verilog-2005 RTL 전부 포함)
- 산출물: preprocess→lex→parse→elaborate→event-driven sim→VCD, 인터프리터 백엔드
- timescale 정밀도 + VCD: 1일차부터
- system tasks 핵심 셋: `$display`/`$write`/`$monitor`/`$strobe`, `$time`/`$realtime`, `$finish`/`$stop`, dump 패밀리
- 마일스톤: ① 단일 모듈 + always 블록 / clk 토글 + $display 동작, ② 다계층 모듈 + parameter resolution, ③ `$dumpvars`로 VCD 생성, ④ Icarus와 차등검증 PASS

## Phase 2 — SV 확장
- interface/modport, package, struct/enum/typedef, always_comb/_ff/_latch, foreach, unique/priority
- system tasks 확장 (spec §9 Phase 2 목록 옮김)
- 마일스톤: ① package + typedef ② interface로 신호 그룹 전달 ③ assertion 샘플링 함수 동작

## Phase 3 — VHDL
- IEEE 1076 프론트엔드 (별도 lexer/parser)
- 공유 IR 위에 얹기 (sim-ir, sim-engine, hdl-builtins, vcd-writer 재사용)
- std_logic_1164 / numeric_std 빌트인 패키지
- 마일스톤: ① entity/architecture + signal assignment ② process + wait ③ ieee 패키지

## 상시 횡단
timescale 정밀도, VCD, 차등검증, 진단 품질, system tasks 컴플라이언스 코퍼스.

## 후속 (여력 시)
- 컴파일드/JIT 백엔드 (IR 경계 활용)
- FST 파형 (LZ4 압축, 대용량)
- SV assertion 영역 확장 (Preponed/Observed/Reactive/Postponed)
- 확장 VCD (`$dumpports*`)

## 리스크 / 의존성
- SV 방대 → Phase 1 엄격 제한
- 차등검증 도구 차이 → 표준 권위 기준
- system tasks 비결정 영역(`$urandom` 등) → 표준+Icarus 의미 기준

## Sources
- 본 spec §9
```

- [ ] **Step 2: 커밋**

```bash
git add docs/preview/05-strategy-and-roadmap.md
git commit -m "Add 05-strategy-and-roadmap

3-phase SV-first roadmap with milestones, 상시 횡단, 후속 확장, risks. Direct from spec §9.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: 06-simulation-engine.md + 08-timescale-and-timing.md

**Files:**
- Create: `docs/preview/06-simulation-engine.md`
- Create: `docs/preview/08-timescale-and-timing.md`
- Create: `docs/preview/research-log/sv-scheduling-2026-MM-DD.md`
- Create: `docs/preview/research-log/timescale-precision-2026-MM-DD.md`

두 문서는 IEEE 1800 §4 (scheduling) + IEEE 1364 §6/§17 (timing) research 필수.

- [ ] **Step 1: research 다라운드 — SV scheduling event regions**

프롬프트:

```
research IEEE 1800 SystemVerilog stratified event scheduling regions in detail: (1) full list — Preponed, Pre-Active, Active, Inactive, Pre-NBA, NBA, Post-NBA, Observed, Post-Observed, Reactive, Post-Re-Active, Re-Inactive, Pre-Re-NBA, Re-NBA, Post-Re-NBA, Postponed; (2) IEEE 1364 minimal regions (Active/Inactive/NBA/Monitor); (3) delta cycle semantics; (4) blocking vs non-blocking assignment ordering; (5) `#0` semantics and Inactive region; (6) examples that distinguish region behaviors. Primary sources: IEEE 1800-2017/2023 §4, IEEE 1364-2005 §5. Also reference Verilator/Icarus implementation notes. 한국어 narrative — 도표/표 포함 권장.
```

결과 → `research-log/sv-scheduling-<date>.md`.

- [ ] **Step 2: research 다라운드 — timescale precision**

프롬프트:

```
research IEEE 1364 / 1800 \`timescale directive precisely: (1) syntax `\`timescale unit/precision`, allowed units (s/ms/us/ns/ps/fs) and values (1, 10, 100); (2) per-module vs global precision: how mixed-precision designs are scheduled; (3) rounding rules for delays smaller than precision (toward nearest, half-to-even?); (4) `$time` vs `$realtime` semantics; (5) Icarus and Verilator behavior on mixed timescale; (6) numerical pitfalls — why floating-point time accumulates error vs integer time. Primary: IEEE 1800 / 1364 LRM. 한국어 narrative.
```

결과 → `research-log/timescale-precision-<date>.md`.

- [ ] **Step 3: `06-simulation-engine.md` 작성 — 다음 섹션**

```
# 06 · 시뮬레이션 엔진

## 이벤트 구동 모델 개요
(왜 이벤트 구동인가, 클록드 시뮬레이션과의 차이)

## Stratified Event Queue
- 최소 핵심: Active → Inactive(`#0`) → NBA(`<=`) → Monitor (IEEE 1364)
- SV 확장: Preponed/Observed/Reactive/Postponed 등 — 후속 phase
- 영역 분리가 blocking vs non-blocking의 결정론 보장
- (research에서 가져온 다이어그램/표)

## Delta Cycle
- 동일 시각 0-time 반복으로 신호 안정화
- 조합 논리 전파 + race 회피
- 무한 delta 검출 + 진단

## Process & Sensitivity
- always / initial / continuous assign이 IR의 process 노드로
- sensitivity list (`@(...)`, `@*`, `always_comb`)
- 트리거 → Active queue 삽입

## Builtin-Call 처리
- IR의 builtin-call 노드 만나면 hdl-builtins 디스패치
- dump 태스크는 vcd-writer로 라우팅 (§7 참조)

## 성능 고려
- 인터프리터 핫 루프 — IR을 캐시 친화적 SoA로
- 64-bit 정수 time wheel (§08 참조)
- 후속 컴파일드/JIT 백엔드의 분기점

## Sources
- 본 spec §6
- research-log: `sv-scheduling-<date>.md`
- IEEE 1800 §4, IEEE 1364 §5
```

- [ ] **Step 4: `08-timescale-and-timing.md` 작성 — 다음 섹션**

```
# 08 · Timescale · 정밀 시간

## 왜 정밀도가 핵심인가
시뮬레이터 버그 = 조용한 오답. 1 precision unit의 천이 어긋남도 검증 실패로 드러나야 함.

## `\`timescale unit/precision`
- 문법, 허용 단위 (s/ms/us/ns/ps/fs), 허용 값 (1/10/100)
- 모듈별 적용 vs 컴파일 단위 적용 (IEEE 1800 §3 참고)

## 64-bit 정수 시간 모델
- 전역 시간 = 설계 전체에서 가장 미세한 precision의 정수 카운트
- 부동소수 사용 안 함 — 누적오차 0
- 모듈별 unit/precision → 전역 precision 기준 정수 배율
- `#delay` 환산 예시:
  ```
  // ModuleA: `timescale 1ns/100ps  → unit=10p, precision=1p (가정 전역=1p)
  #1.55  // 1.55 ns × 1000p/ns = 1550 ps = 15500 (전역 100fs precision일 때)
  ```
  실제 반올림 규칙: IEEE 표준 (research 결과 박제)

## $time vs $realtime
- `$time`: 64-bit 정수, 호출 모듈의 unit 단위
- `$realtime`: real, precision 반영한 소수
- 구현: 내부는 정수, 출력 시 변환

## 정밀도 회귀 테스트
- 서로 다른 timescale 모듈 혼재 설계 → 천이 시각이 1 precision까지 일치 확인
- Icarus / Verilator와 차등검증
- 테스트 케이스 예시 3~5개 (작성 시 채움)

## Sources
- 본 spec §6.3
- research-log: `timescale-precision-<date>.md`
- IEEE 1800 §3, IEEE 1364 §17 / §6
```

- [ ] **Step 5: 커밋**

```bash
git add docs/preview/06-simulation-engine.md docs/preview/08-timescale-and-timing.md docs/preview/research-log/sv-scheduling-*.md docs/preview/research-log/timescale-precision-*.md
git commit -m "Add 06-simulation-engine + 08-timescale-and-timing

- 06: event-driven kernel, stratified event queue, delta cycle, process/sensitivity, builtin dispatch (research)
- 08: 64-bit 정수 시간 모델, precision 반올림, \$time vs \$realtime, 정밀도 회귀 테스트 (research)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: 07-vcd-format.md

**Files:**
- Create: `docs/preview/07-vcd-format.md`
- Create: `docs/preview/research-log/vcd-format-2026-MM-DD.md`

VCD 포맷 정확도가 핵심. 다라운드 research 필수.

- [ ] **Step 1: research 다라운드 — VCD 포맷 명세**

프롬프트:

```
research IEEE 1364 §18 Value Change Dump (VCD) text format with full BNF/grammar accuracy: (1) header tokens — `$date $version $timescale $scope $upscope $var $enddefinitions` — argument syntax, allowed scope types (module/task/function/begin/fork), var types (wire/reg/integer/real/parameter/etc.); (2) identifier code encoding — printable ASCII chars 33-126, multi-char codes; (3) `$dumpvars` initial dump — value format including initial state; (4) value change sections — scalar (`0/1/x/z` + identifier code, no space), vector (`b<bits> <id>`), real (`r<value> <id>`); (5) `#<time>` markers — non-decreasing constraint; (6) `$dumpoff`/`$dumpon`/`$dumpall`/`$dumpflush`/`$dumplimit` semantics; (7) `$comment` blocks; (8) extended VCD (`$dumpports*` — out of scope here but for completeness); (9) GTKWave/Surfer parser compatibility caveats. Primary: IEEE 1364-2005 §18 (also IEEE 1800-2017 by reference). Secondary: GTKWave docs, vcd crate Rust source. 한국어 narrative — BNF 표기 포함.
```

결과 → `research-log/vcd-format-<date>.md`. 충분한 정확도 확보까지 최대 4라운드.

- [ ] **Step 2: `07-vcd-format.md` 작성 — 다음 섹션 (golden 포맷 명세)**

```
# 07 · VCD 포맷 · 생성기 설계

## 원칙: RTL-driven
VCD는 RTL의 dump 시스템 태스크가 호출될 때만 생성. dump 태스크 무호출 = VCD 미생성 (no-op).

## 인식·지원할 dump 시스템 태스크
- `$dumpfile("name.vcd")` — 출력 파일 지정
- `$dumpvars` — 무인자(전체) / `(level, scope)` (depth + 지정)
- `$dumpon` / `$dumpoff` — 일시 재개/정지
- `$dumpall` — 현재 모든 값 강제 기록
- `$dumpflush` — 버퍼 플러시
- `$dumplimit(size)` — 파일 크기 제한

## 파일 구조 (golden 명세, IEEE 1364 §18)

### 헤더
```
$date <YYYY-MM-DD HH:MM:SS> $end
$version Vitamin vX.Y.Z $end
$timescale 1 ns $end       # 전역 precision 표현
$scope module top $end
  $var wire 1 ! clk $end
  $var wire 8 " data $end
  $scope module child $end
    ...
  $upscope $end
$upscope $end
$enddefinitions $end
```

### 초기 덤프 + 값 변화
```
$dumpvars
0!
b00000000 "
$end
#10
1!
#20
0!
b11110000 "
```

### 식별자 코드 (research 결과 박제)
- printable ASCII 33-126 (`!` ~ `~`)
- 다문자: `!`, `"`, ..., `!!`, ...
- 인코딩 알고리즘 박제 (vcd-writer 구현 가이드)

### 값 표기
- scalar: `0|1|x|z` + id (공백 없음) — 예: `1!`
- vector: `b<bits> <id>` — 예: `b1010 "` (선행 0 생략 가능, x/z 포함 가능)
- real: `r<value> <id>` — 예: `r3.14 #`

### `$comment` 블록
임의 텍스트, `$end`로 종료.

## 생성기 설계 (vcd-writer 크레이트)

### 인터페이스
```rust
pub struct VcdWriter { ... }
impl VcdWriter {
    pub fn new(path: &Path, timescale: Timescale) -> Result<Self>;
    pub fn declare_scope(&mut self, kind: ScopeKind, name: &str);
    pub fn declare_var(&mut self, kind: VarKind, width: u32, name: &str) -> IdCode;
    pub fn end_definitions(&mut self);
    pub fn dump_initial(&mut self, values: &[(IdCode, Value)]);
    pub fn time(&mut self, t: u64);
    pub fn change_scalar(&mut self, id: IdCode, bit: Bit);
    pub fn change_vector(&mut self, id: IdCode, bits: &[Bit]);
    pub fn dump_off(&mut self);
    pub fn dump_on(&mut self);
    pub fn flush(&mut self);
}
```

### dump 태스크 라우팅
hdl-builtins의 dump 카테고리 핸들러가 VcdWriter 메서드 호출. 단일 인스턴스(시뮬 전역).

## 검증 전략
1. 생성 VCD를 GTKWave / Surfer로 로드 → 오류 없음.
2. Icarus iverilog의 VCD와 정규화 diff 일치 (식별자 코드 차 흡수 정규화기).
3. Verilator VCD와도 비교 (옵션).

## 비목표 (현 단계)
- 확장 VCD (`$dumpports*`)
- FST 포맷 (LZ4 압축 대용량)

## Sources
- 본 spec §7
- research-log: `vcd-format-<date>.md`
- IEEE 1364-2005 §18 (primary)
- GTKWave docs
```

- [ ] **Step 3: 커밋**

```bash
git add docs/preview/07-vcd-format.md docs/preview/research-log/vcd-format-*.md
git commit -m "Add 07-vcd-format

RTL-driven semantics + 6 dump tasks + IEEE 1364 §18 golden file format (header, identifier codes, value notations) + vcd-writer crate interface + verification (research multi-round)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 8: 09-testing-and-verification.md + 10-glossary.md

**Files:**
- Create: `docs/preview/09-testing-and-verification.md`
- Create: `docs/preview/10-glossary.md`
- Create: `docs/preview/research-log/iverilog-verilator-behaviors-2026-MM-DD.md`

09는 차등검증 도구 동작 차이 가벼운 research. 10은 spec §13 + 추가 용어.

- [ ] **Step 1: research 1 라운드 — Icarus/Verilator 동작 차이**

프롬프트:

```
research practical differences between Icarus Verilog (iverilog + vvp) and Verilator for differential testing of an RTL simulator: (1) what subsets of IEEE 1800 does each support fully vs partially; (2) typical semantic differences (NBA timing, $display ordering, x-propagation, multi-driver); (3) command-line invocation for headless batch (iverilog -o; vvp ; verilator --binary --trace); (4) VCD output options and quirks (Verilator default uses FST unless --trace; format differences); (5) recommended canonical testcases for cross-verification (CHIPS Alliance, OpenTitan, etc.); (6) hosted test corpora (e.g., zoo of small RTL designs). Cite Icarus iverilog GitHub, Verilator docs, any comparison studies. 한국어 narrative.
```

결과 → `research-log/iverilog-verilator-behaviors-<date>.md`.

- [ ] **Step 2: `09-testing-and-verification.md` 작성 — 다음 섹션**

```
# 09 · 테스트 · 검증 전략

## 검증 계층
1. **단위 테스트** — 크레이트별 (lexer/parser/elaborate/sim-ir/sim-engine/hdl-builtins/vcd-writer/diag).
2. **통합 테스트** — 작은 RTL 입력 → 시뮬레이션 → VCD 비교.
3. **차등검증** — Icarus / Verilator와 동일 입력 → 신호값·천이 시각 비교.
4. **컴플라이언스 코퍼스** — 언어 기능별 + system tasks 범주별.

## 단위 테스트 정책
- TDD 지향 — 새 기능은 실패 테스트 먼저.
- 각 크레이트는 자체 `tests/` 디렉터리.
- 커버리지 목표: lex/parse 95%+, elaborate 90%+, engine 핵심 90%+.

## 차등검증 워크플로우
1. RTL 입력 + 테스트벤치 준비
2. 세 도구 동시 실행: `vita sim`, `iverilog -o ... && vvp ...`, `verilator --trace ... --binary`
3. 각 도구의 VCD 추출
4. 정규화 diff 도구로 비교 (식별자 코드 차 흡수)
5. 첫 차이 발견 시 표준이 권위 — 표준 어김인지 도구 버그인지 판별 (research에서 가져온 차이 패턴 박제)

## VCD Golden Diff 도구
- 정규화 규칙 (식별자 매핑, 공백/주석 무시, 동일 시각 동일 신호 값만 비교)
- diff 출력 포맷

## 컴플라이언스 코퍼스 구조
```
tests/corpus/
├── verilog-2005/
│   ├── lexical/
│   ├── modules/
│   ├── procedural/
│   └── ...
├── sv-extensions/
└── system-tasks/
    ├── display/
    ├── file-io/
    ├── dump/
    └── ...
```

각 케이스: `<name>.sv` (RTL) + `<name>.expected.vcd` (golden).

## 추천 외부 테스트벤치 (research 반영)
- (research에서 가져온 RTL zoo / OpenTitan 등)

## CI 통합
- 빌드 매트릭스 (§03) 위에서 corpus 전체 수행
- VCD diff PASS/FAIL을 PR 게이트로

## Sources
- 본 spec §8
- research-log: `iverilog-verilator-behaviors-<date>.md`
- Icarus / Verilator 공식 문서 URL
```

- [ ] **Step 3: `10-glossary.md` 작성 — 다음 섹션**

```
# 10 · 용어집

## 시뮬레이션 핵심
- **Compile** — preprocess + lex + parse + 문법 검사. 결과: AST.
- **Elaboration** — 파라미터 해소, 계층 인스턴스화, 타입/연결성/다중구동 검사. 결과: sim-ir.
- **Event-driven simulation** — 신호 변화(이벤트)에 반응해 process를 실행하는 방식. 클록드 시뮬레이션 대비 표준.
- **Delta cycle** — 동일 시뮬 시각 내 신호 안정화를 위한 0-time 반복.
- **Stratified event queue** — IEEE 1364/1800 표준 스케줄링 영역 분리(Active/Inactive/NBA/Monitor 등).
- **NBA** — Non-Blocking Assignment(`<=`)의 갱신을 모으는 이벤트 영역.

## 시간
- **timescale** — `\`timescale unit/precision`. 모듈별 단위/분해능.
- **Time wheel** — 미래 이벤트를 시각별로 보관하는 자료구조.
- **`$time` / `$realtime`** — 정수 vs precision-반영 실수 시간.

## 파형
- **VCD** — Value Change Dump (IEEE 1364 §18). 신호 변화 텍스트 파형 포맷.
- **Identifier code** — VCD에서 신호를 짧게 식별하는 ASCII 코드.
- **FST** — Fast Signal Trace (GTKWave/Verilator 압축 포맷). 본 프로젝트 비목표.

## 언어
- **HDL** — Hardware Description Language.
- **System task / function** — `$`로 시작하는 표준 빌트인 (`$display`, `$dumpvars`, `$bits`, `$urandom` 등).
- **Synthesizability** — 합성(synthesis) 도구가 게이트로 변환 가능한지 여부.

## 본 프로젝트
- **Vitamin** — 본 프로젝트 코드네임 (임시).
- **vita** — CLI 작업명 (placeholder).
- **hdl-builtins** — `$`-system tasks/functions 구현 크레이트.
- **sim-ir** — 언어 비의존 시뮬레이션 IR.

## Sources
- 본 spec §13 (요약) + IEEE 1800/1364 표준 용어.
```

- [ ] **Step 4: 커밋**

```bash
git add docs/preview/09-testing-and-verification.md docs/preview/10-glossary.md docs/preview/research-log/iverilog-verilator-behaviors-*.md
git commit -m "Add 09-testing-and-verification + 10-glossary

- 09: 4-tier verification (unit/integration/differential/compliance), Icarus/Verilator quirks (research), VCD golden diff, corpus structure
- 10: project + standard glossary

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 9: 11-sources-and-citations.md + 12-research-methodology.md

**Files:**
- Create: `docs/preview/11-sources-and-citations.md`
- Create: `docs/preview/12-research-methodology.md`

둘 다 spec §11·§12에서 직접 도출. research 불필요.

- [ ] **Step 1: `11-sources-and-citations.md` 작성 — 다음 섹션**

```
# 11 · 출처 · 저작권 · 인용 정책

## 원칙
- IEEE 표준은 저작권 보호 + 유료. **요약·인용 OK, 원문 verbatim 복제 금지.**
- 자유 이용 자료 우선: 공개 LRM 참조 페이지, 공개 BNF/grammar, IEEE Get 프로그램, Verilator/Icarus 공식 문서, 대학 강의자료.
- 영리/유료 자료는 정당한 인용 범위만.

## 인용 규칙
- 각 문서 하단 `## Sources` 섹션 필수.
- 항목 형식: `- <설명>: <URL or 표준번호>, 접근일 YYYY-MM-DD`.
- IEEE 표준 식별: `IEEE 1800-2017`, `IEEE 1364-2005` 같이 정확한 번호.
- 절(section) 인용: `IEEE 1800-2017 §4.4.2` 형식.

## 자료 카탈로그
- **표준**: IEEE 1800 (SV), IEEE 1364 (Verilog, withdrawn but referenced), IEEE 1076 (VHDL), IEEE 1164 (std_logic_1164).
- **참조 LRM**: ChipVerify, AsicWorld, Verification Academy(요약), HDLBits.
- **오픈소스 도구 문서**: iverilog (steveicarus.github.io), Verilator (veripool.org), Yosys, GTKWave.
- **저장소**: GitHub iverilog, verilator, yosys, sv-parser.

## 위반 시 처리
- verbatim 복제 발견 → 즉시 요약·재작성으로 교체 + 커밋 로그에 "Fix copyright: ..." 기록.

## Sources
- 본 spec §12.
```

- [ ] **Step 2: `12-research-methodology.md` 작성 — 다음 섹션**

```
# 12 · 조사 방법론 (research 스킬 기반)

## 원칙
**다라운드 + 다각도 + 1차 source 직접 확인.** 단일 소스 의존 없이 라운드마다 의도적으로 다른 각도(쿼리·언어·시각)에서 검색·검증해 hallucination을 거른다.

## 사용 도구
**`research` 스킬** = Anthropic WebSearch + WebFetch 다라운드.
- `gemini -p` 서브에이전트 방식은 **폐기** (이전 백엔드, 기록만 보존).

## 3 소스
1. **Claude 내부 지식** — 빠른 초안·골격(baseline). 단독 사용 금지(반드시 라이브 검증과 결합).
2. **WebSearch (다각도)** — 라운드마다 의도적으로 다른 쿼리/언어(영/한)/시각.
3. **WebFetch — 1차 source 직접 확인** — IEEE LRM 페이지, 표준 본문, 도구 공식 문서를 **직접 읽어** hallucination 차단 (Phase 1.5 검증).

## 워크플로우
```
다각도 WebSearch
   ↓
WebFetch로 핵심 source 검증
   ↓
Claude 5차원 체크리스트 gap 점검 (covered? sourced? precise? balanced? Korean clarity?)
   ↓
gap 있으면 라운드 추가 (다른 각도) — 최대 4
   ↓
한국어 narrative 수렴
```

## 충돌 처리
소스 간 정보 불일치 시:
- 양쪽 기록 + 차이 명시
- **IEEE LRM(1차 표준) 우선**
- 도구별(Icarus/Verilator) 동작 차이는 별도 표기

## 재현성
모든 조사는 `research-log/<topic-slug>-YYYY-MM-DD.md`에 기록:
- 머리말 YAML: topic / date / rounds / primary_sources_fetched / queries
- 본문: research 스킬의 narrative 출력
- 하단 Sources: 인용 URL 재정리

## 헤더 규약
research 스킬 규약 준수 — 본문 헤더에 한자/추상 명사(起承轉結·기승전결·"도입/본론/결론") 금지. 주제에서 따온 구체 헤더만.

## 호출 예시
```
research IEEE 1364 §18 VCD format: exact BNF, identifier code encoding, scalar/vector/real value notation, $dumpvars semantics. Primary: IEEE 1364-2005 §18. 한국어 narrative with BNF.
```

## Sources
- 본 spec §11.
- 본 plan 공통 규칙.
- research 스킬 정의 (Claude Code skills).
```

- [ ] **Step 3: 커밋**

```bash
git add docs/preview/11-sources-and-citations.md docs/preview/12-research-methodology.md
git commit -m "Add 11-sources-and-citations + 12-research-methodology

- 11: copyright/citation policy, verbatim 복제 금지, IEEE 표준 인용 규칙
- 12: research 스킬 기반 방법론 (WebSearch+WebFetch 다라운드, 5차원 체크리스트, 최대 4라운드, 1차 source 검증)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

**Checkpoint:** Group B 종료. 12개 기획 문서 완성 → 프로젝트 방향 lock. 이후 그룹은 reference 문서.

---

## Group C — HDL 참조 골격 (Task 10)

### Task 10: hdl-reference/{README, 00-standards-map, 01-synthesizability-legend}

**Files:**
- Create: `docs/preview/hdl-reference/README.md`
- Create: `docs/preview/hdl-reference/00-standards-map.md`
- Create: `docs/preview/hdl-reference/01-synthesizability-legend.md`
- Create: `docs/preview/research-log/hdl-standards-versions-2026-MM-DD.md`

표준 버전 매핑은 정확도 중요 → research 필수.

- [ ] **Step 1: research 1~2 라운드 — IEEE 표준 버전 매핑**

프롬프트:

```
research IEEE HDL standards version map with exact dates and relationships: (1) IEEE 1364 — versions 1995, 2001, 2005 (last standalone Verilog); status (withdrawn? merged?); (2) IEEE 1800 — 2005, 2009, 2012, 2017, 2023 — when did SV subsume 1364 (2009 merger)?; what's new in each revision; (3) IEEE 1076 — 1987, 1993, 2002, 2008, 2019 — major changes per revision; status of 2019; (4) IEEE 1164 (std_logic_1164) — 1993 standalone, then absorbed into 1076-2008; (5) IEEE 1666 SystemC (mention only, out-of-scope); (6) IEEE Get program — which of these are freely accessible? Primary: ieee.org, IEEE Xplore abstracts (no paywall content), Wikipedia (for cross-reference only). 한국어 narrative — 표 형식 권장.
```

결과 → `research-log/hdl-standards-versions-<date>.md`.

- [ ] **Step 2: `hdl-reference/README.md` 작성 (얇은 인덱스)**

```
# HDL Reference

본 폴더는 Verilog · SystemVerilog · VHDL의 문법·패키지·합성가능성을 섹터별로 정리한 참조 문서다. 시뮬레이터 구현 시 표준 준수 확인용.

## 폴더
| 폴더/파일 | 설명 |
|---|---|
| [00-standards-map.md](00-standards-map.md) | IEEE 1800/1364/1076/1164 버전·관계 매핑 |
| [01-synthesizability-legend.md](01-synthesizability-legend.md) | ✅/⚠️/❌ 합성 표기 범례 (전 참조문서 공통) |
| [system-tasks/](system-tasks/) | 표준 `$`-system tasks/functions 카테고리별 |
| [verilog/](verilog/) | Verilog (IEEE 1364) 문법·구조 |
| [systemverilog/](systemverilog/) | SystemVerilog (IEEE 1800) 확장 |
| [vhdl/](vhdl/) | VHDL (IEEE 1076) 문법·패키지 |

## 권장 읽기 순서
1. 00-standards-map → 02-synthesizability-legend (전체 규약)
2. system-tasks/00-index (커버리지 매트릭스)
3. 관심 언어 폴더의 00-index → 항목별

## Sources
- 본 spec §10 (구조)
```

- [ ] **Step 3: `hdl-reference/00-standards-map.md` 작성 (research 결과 박제)**

```
# 00 · IEEE HDL 표준 매핑

## 빠른 참조 표
(research에서 가져온 표 — 표준번호, 버전, 발표연도, 주요 변경, 상태)

| 표준 | 버전 | 발표 | 주요 변경 | 상태 |
|---|---|---|---|---|
| IEEE 1364 (Verilog) | 1995 | 1995 | 초판 | superseded |
| IEEE 1364 | 2001 | 2001 | generate, signed, ... | superseded |
| IEEE 1364 | 2005 | 2005 | 마지막 standalone | merged into 1800 (2009) |
| IEEE 1800 (SV) | 2005 | 2005 | SV 초판 (Verilog 별도) | superseded |
| IEEE 1800 | 2009 | 2009 | **1364 흡수** | superseded |
| IEEE 1800 | 2012 | 2012 | corrigenda | superseded |
| IEEE 1800 | 2017 | 2017 | 정리 + 작은 변경 | active |
| IEEE 1800 | 2023 | 2023 | 최신 | latest |
| IEEE 1076 (VHDL) | 1987 | 1987 | 초판 | superseded |
| IEEE 1076 | 1993 | 1993 | | superseded |
| IEEE 1076 | 2002 | 2002 | | superseded |
| IEEE 1076 | 2008 | 2008 | **1164 흡수**, 큰 개정 | active |
| IEEE 1076 | 2019 | 2019 | 최신 | latest |
| IEEE 1164 (std_logic_1164) | 1993 | 1993 | | merged into 1076-2008 |

## 언어 간 관계
- **SystemVerilog ⊃ Verilog (2009 이후)**: SV는 Verilog의 슈퍼셋. 본 프로젝트 단일 SV 프론트엔드.
- **VHDL은 독립 언어**: 별도 프론트엔드, 공유 IR.

## 본 프로젝트의 타겟 버전
- SV: IEEE 1800-2017 (시작 기준), 2023 이슈는 후속.
- Verilog: 1800에 흡수된 부분 사용 (= 1364-2005 RTL 전부).
- VHDL: IEEE 1076-2008 (시작), 2019 후속.

## 자유 접근 자료
(research 결과 — 어떤 표준이 IEEE Get 등으로 접근 가능한지)

## Sources
- 본 spec §10
- research-log: `hdl-standards-versions-<date>.md`
- IEEE 1800 / 1364 / 1076 / 1164 (표준 번호 인용)
```

- [ ] **Step 4: `hdl-reference/01-synthesizability-legend.md` 작성**

```
# 01 · 합성 가능성 표기 범례

본 폴더의 모든 참조 문서는 각 구문/기능마다 다음 마커를 사용한다.

## 범례
| 마커 | 의미 |
|---|---|
| ✅ | **합성 가능** — 표준 합성 도구가 게이트로 변환 가능 |
| ⚠️ | **조건부** — 특정 형식만 합성 가능 / 도구 의존 / 합성 가능하나 권장 안 됨 |
| ❌ | **비합성** — 시뮬레이션·검증 전용 (예: `class`, assertion, `wait`, dynamic memory) |

## 사용 예
```
### `always_ff @(posedge clk)`
✅ 합성 가능. 표준 클록드 레지스터로 합성.

### `initial`
⚠️ 시뮬레이션용. FPGA 합성은 일부 지원(초기값), ASIC은 일반적으로 비합성.

### `class`
❌ 비합성. 검증 전용(SV 확장).
```

## 본 프로젝트 범위와의 관계
본 프로젝트는 **시뮬레이터**다 — 합성은 비목표. 그러나 참조 문서는 합성 가능 여부를 명기해, 사용자가 RTL 작성 시 합성 친화적 코드를 식별할 수 있게 돕는다.

## Sources
- 본 spec §2.1 (참조 문서에 합성 가능 여부 명기 요구사항)
- IEEE 1800 / 1364 / 1076 합성 가능 부분집합 (도구별 가이드: Synopsys DC, Xilinx Vivado, Cadence Genus 가이드 참고)
```

- [ ] **Step 5: 커밋**

```bash
git add docs/preview/hdl-reference/README.md docs/preview/hdl-reference/00-standards-map.md docs/preview/hdl-reference/01-synthesizability-legend.md docs/preview/research-log/hdl-standards-versions-*.md
git commit -m "Add hdl-reference scaffold (README + standards-map + synthesizability-legend)

- README: 얇은 네비게이션 인덱스
- 00-standards-map: IEEE 1800/1364/1076/1164 버전·관계 (research)
- 01-synthesizability-legend: ✅/⚠️/❌ 공통 범례

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

**Checkpoint:** Group C 종료. HDL 참조 골격 lock.

---

## Group D — system-tasks 14 문서 (Task 11~15)

각 task는 카테고리별로 3~4개 docs를 묶는다. **공통 패턴**: 각 doc은 (1) 카테고리 설명, (2) 함수별 시그니처/의미/예시/합성가능성/지원 Phase, (3) Sources. 카테고리별 research 1~2 라운드.

각 task의 **공통 작성 템플릿**(모든 system-tasks doc에 적용):

````
# NN · <카테고리 이름>

## 개요
이 카테고리는 ... (역할, 어떤 RTL 상황에서 쓰는지)

## 지원 Phase (본 프로젝트)
- Phase 1: <목록>
- Phase 2: <목록>
- Phase 3: <목록>

## 항목 상세
### `$name`
- **시그니처**: `$name(arg1, arg2[, arg3])`
- **표준**: IEEE 1800-2017 §X.Y / IEEE 1364-2005 §A.B
- **의미**: ...
- **반환 타입**: ...
- **예시**:
  ```sv
  $name(...)
  ```
- **Icarus / Verilator 동작**: 동일 / 차이 (있다면 명시)
- **합성 가능성**: ❌ (system tasks는 대부분 비합성, 일부 예외 명시)
- **본 프로젝트 구현 메모**: hdl-builtins의 어느 핸들러가 처리

## Sources
- IEEE 1800-2017 §<해당 절>
- research-log: `<topic-slug>-<date>.md`
````

### Task 11: system-tasks 00-index + 01-display-io + 04-simulation-control + 05-time-functions

**Phase 1 핵심 셋** — research 1~2 라운드 충분.

**Files:**
- Create: `docs/preview/hdl-reference/system-tasks/00-index.md`
- Create: `docs/preview/hdl-reference/system-tasks/01-display-io.md`
- Create: `docs/preview/hdl-reference/system-tasks/04-simulation-control.md`
- Create: `docs/preview/hdl-reference/system-tasks/05-time-functions.md`
- Create: `docs/preview/research-log/system-tasks-display-time-2026-MM-DD.md`

- [ ] **Step 1: research 1~2 라운드 — display/IO/sim-control/time**

프롬프트:

```
research SystemVerilog/Verilog system tasks (IEEE 1800-2017 §20 + IEEE 1364-2005 §17) — categories: (A) display & monitoring — $display, $write, $monitor, $strobe and their b/o/h variants ($displayb/o/h, $writeb/o/h, $monitorb/o/h, $strobeb/o/h), $monitoron/$monitoroff; (B) simulation control — $finish, $stop, $exit (Verilog vs SV); (C) time — $time, $stime, $realtime. For each: signature, semantics, format string specifiers (%d/%b/%h/%o/%c/%s/%t/%v/%e/%f/%g/%m/etc.), example RTL, Icarus and Verilator behavior, exit codes for $finish. Primary: IEEE 1800-2017 §20.x, IEEE 1364-2005 §17.x. 한국어 narrative — 각 시스템 태스크별 상세 항목.
```

결과 → `research-log/system-tasks-display-time-<date>.md`.

- [ ] **Step 2: `00-index.md` 작성 — 전 system-tasks 인벤토리 + 커버리지 매트릭스**

```
# 00 · System Tasks · Functions 인덱스

`$`로 시작하는 표준 빌트인은 본 프로젝트 `hdl-builtins` 크레이트가 구현한다. 본 폴더는 카테고리별 상세 참조.

## 카테고리 인덱스
| # | 파일 | 주요 항목 | Phase |
|---|---|---|---|
| 01 | display-io | $display/$write/$monitor/$strobe + b/o/h | 1 |
| 02 | file-io | $fopen/$fclose/$fwrite/$fdisplay/$fread/$fscanf/$sscanf/$sformat | 2 |
| 03 | memory-load | $readmemb/$readmemh/$writememb/$writememh | 2 |
| 04 | simulation-control | $finish/$stop/$exit | 1 |
| 05 | time-functions | $time/$stime/$realtime | 1 |
| 06 | conversion | $signed/$unsigned/$rtoi/$itor/$bitstoreal/$realtobits | 2 |
| 07 | bit-vector | $bits/$clog2/$countones/$countbits/$onehot/$onehot0/$isunknown | 2 |
| 08 | math | $pow/$ln/$log10/$exp/$sqrt/$sin/$cos/$tan | 2 |
| 09 | random | $random/$urandom/$urandom_range/$dist_* | 2 |
| 10 | vcd-dump | $dumpfile/$dumpvars/$dumpon/$dumpoff/$dumpall/$dumpflush/$dumplimit | 1 |
| 11 | assertion-sampling | $past/$rose/$fell/$stable/$changed/$sampled/$assertoff/$asserton/$assertkill | 2 |
| 12 | introspection | $typename/$cast/$isunbounded/$size/$left/$right/$low/$high/$increment | 2 |
| 13 | misc | $value$plusargs/$test$plusargs/$system | 2 |

## Phase 1 핵심 셋 (MVP 1일차부터)
- display: `$display`, `$write`, `$monitor`, `$strobe`
- time: `$time`, `$realtime`
- control: `$finish`, `$stop`
- dump: `$dumpfile`, `$dumpvars`, `$dumpon`, `$dumpoff`, `$dumpall`

## Phase 2 확장
(전 항목)

## Phase 3 / 후속
일부 SV-only assertion sampling, 확장 VCD dumpports* (현재 비목표)

## Sources
- 본 spec §9 (Phase별 system tasks 목록)
- IEEE 1800-2017 §20, IEEE 1364-2005 §17
```

- [ ] **Step 3: `01-display-io.md` 작성** — 공통 작성 템플릿에 따라 각 항목:
  - `$display` / `$displayb` / `$displayo` / `$displayh`
  - `$write` / `$writeb` / `$writeo` / `$writeh`
  - `$monitor` / `$monitorb` / `$monitoro` / `$monitorh` / `$monitoron` / `$monitoroff`
  - `$strobe` / `$strobeb` / `$strobeo` / `$strobeh`
  
  포맷 specifier 표 (`%d/%b/%h/%o/%c/%s/%t/%v/%e/%f/%g/%m/%0d/...`) 포함.

- [ ] **Step 4: `04-simulation-control.md` 작성** — `$finish`, `$stop`, `$exit`. exit codes / IEEE 정의 박제.

- [ ] **Step 5: `05-time-functions.md` 작성** — `$time`, `$stime`, `$realtime`. 모듈 unit vs 전역 precision 변환 예시 + 08-timescale doc cross-link.

- [ ] **Step 6: 커밋**

```bash
git add docs/preview/hdl-reference/system-tasks/00-index.md docs/preview/hdl-reference/system-tasks/01-display-io.md docs/preview/hdl-reference/system-tasks/04-simulation-control.md docs/preview/hdl-reference/system-tasks/05-time-functions.md docs/preview/research-log/system-tasks-display-time-*.md
git commit -m "Add system-tasks: 00-index + 01-display-io + 04-sim-control + 05-time

Phase 1 핵심 셋. Coverage matrix + 카테고리별 상세 (research 1~2 라운드).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 12: system-tasks 10-vcd-dump + 02-file-io + 03-memory-load

**VCD dump (Phase 1) + 파일/메모리 I/O (Phase 2 진입).**

**Files:**
- Create: `docs/preview/hdl-reference/system-tasks/10-vcd-dump.md`
- Create: `docs/preview/hdl-reference/system-tasks/02-file-io.md`
- Create: `docs/preview/hdl-reference/system-tasks/03-memory-load.md`
- Create: `docs/preview/research-log/system-tasks-io-memory-2026-MM-DD.md`

- [ ] **Step 1: research 1~2 라운드 — file I/O / memory load / dump**

프롬프트:

```
research SystemVerilog/Verilog system tasks — categories: (A) VCD dump — $dumpfile, $dumpvars (signatures: no-arg, (level, scope), variadic), $dumpoff/$dumpon, $dumpall, $dumpflush, $dumplimit; arguments and effects per IEEE 1364 §18; (B) file I/O — $fopen (modes "r/w/a/r+/w+/a+"), $fclose, $fwrite/$fdisplay/$fmonitor/$fstrobe (variants with mcd vs fd), $fread, $fscanf, $fgets, $sscanf, $sformat, $sformatf — semantics and return values; (C) memory load — $readmemh, $readmemb (address syntax including @<hex>, comments //, /* */ — exact rules), $writememh, $writememb. For each: signature, IEEE section, semantics, Icarus/Verilator quirks, example. 한국어 narrative.
```

결과 → `research-log/system-tasks-io-memory-<date>.md`.

- [ ] **Step 2: `10-vcd-dump.md` 작성**

각 dump 태스크 상세 (공통 템플릿). 07-vcd-format doc과 cross-link. `$dumpvars`의 `(level, scope)` 인자 의미 박제 (level=0 무제한, scope=identifier).

- [ ] **Step 3: `02-file-io.md` 작성**

각 파일 I/O 함수 상세. `$fopen`의 mode 문자열 + 반환 fd / mcd 차이. `$fread` 바이트/벡터 모드 차이.

- [ ] **Step 4: `03-memory-load.md` 작성**

`$readmemh/b`의 정확한 입력 포맷 (주소 `@HEX`, 데이터, 주석, 공백). 다중구동 시 동작. `$writememh/b` 출력 포맷.

- [ ] **Step 5: 커밋**

```bash
git add docs/preview/hdl-reference/system-tasks/10-vcd-dump.md docs/preview/hdl-reference/system-tasks/02-file-io.md docs/preview/hdl-reference/system-tasks/03-memory-load.md docs/preview/research-log/system-tasks-io-memory-*.md
git commit -m "Add system-tasks: 10-vcd-dump + 02-file-io + 03-memory-load

dump 패밀리 (Phase 1) + 파일/메모리 I/O (Phase 2) — IEEE 1364 §18 + §17 (research)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 13: system-tasks 06-conversion + 07-bit-vector + 08-math

**Files:**
- Create: `docs/preview/hdl-reference/system-tasks/06-conversion.md`
- Create: `docs/preview/hdl-reference/system-tasks/07-bit-vector.md`
- Create: `docs/preview/hdl-reference/system-tasks/08-math.md`
- Create: `docs/preview/research-log/system-tasks-conversion-math-2026-MM-DD.md`

- [ ] **Step 1: research 1~2 라운드**

프롬프트:

```
research SystemVerilog system functions: (A) conversion — $signed, $unsigned (SV both as cast operators and system functions), $rtoi, $itor, $bitstoreal, $realtobits — exact return types, sign-extension/truncation rules; (B) bit-vector queries — $bits (parameterizable on type or expr), $clog2 (return type, edge cases for 0 input), $countones, $countbits, $onehot, $onehot0, $isunknown — semantics on x/z; (C) math functions — $pow, $ln, $log10, $exp, $sqrt, $sin, $cos, $tan, $asin/$acos/$atan/$atan2, $sinh/$cosh/$tanh, $floor, $ceil — real-valued. IEEE 1800-2017 §20.8, §20.9. 한국어 narrative.
```

결과 → `research-log/system-tasks-conversion-math-<date>.md`.

- [ ] **Step 2: `06-conversion.md` 작성** — 각 변환 함수 + 부호확장 규칙 + 실수 ↔ 정수.

- [ ] **Step 3: `07-bit-vector.md` 작성** — `$bits` (타입/표현식 인자), `$clog2`(0/1 입력 처리), 카운팅 함수들, x/z 처리.

- [ ] **Step 4: `08-math.md` 작성** — 실수 수학 함수, IEEE 754 동작, 도메인 외 입력(`$sqrt(-1)` 등) 처리.

- [ ] **Step 5: 커밋**

```bash
git add docs/preview/hdl-reference/system-tasks/06-conversion.md docs/preview/hdl-reference/system-tasks/07-bit-vector.md docs/preview/hdl-reference/system-tasks/08-math.md docs/preview/research-log/system-tasks-conversion-math-*.md
git commit -m "Add system-tasks: 06-conversion + 07-bit-vector + 08-math

부호변환, 비트벡터 쿼리, 실수 수학 (Phase 2). IEEE 1800-2017 §20.8/§20.9 (research).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 14: system-tasks 09-random + 11-assertion-sampling

**Files:**
- Create: `docs/preview/hdl-reference/system-tasks/09-random.md`
- Create: `docs/preview/hdl-reference/system-tasks/11-assertion-sampling.md`
- Create: `docs/preview/research-log/system-tasks-random-assertion-2026-MM-DD.md`

- [ ] **Step 1: research 1~2 라운드**

프롬프트:

```
research SystemVerilog system functions: (A) random — $random (seeded, 32-bit signed), $urandom (32-bit unsigned, IEEE 1800-defined RNG state), $urandom_range(max [, min]), $dist_uniform, $dist_normal, $dist_exponential, $dist_poisson, $dist_chi_square, $dist_t, $dist_erlang — exact algorithms IEEE 1800 mandates (Mersenne Twister? LCG?), seed handling, thread-local vs global state, reproducibility across tools; (B) assertion sampling — $past(expr [, n]), $rose, $fell, $stable, $changed, $sampled — clocked sampling semantics (Observed region), $assertoff/$asserton/$assertkill/$assertcontrol. Primary: IEEE 1800-2017 §18 (random), §16 (SVA). Verilator/Icarus support. 한국어 narrative.
```

결과 → `research-log/system-tasks-random-assertion-<date>.md`.

- [ ] **Step 2: `09-random.md` 작성** — `$random` vs `$urandom` 차이, seed 정책, 분포 함수들.

- [ ] **Step 3: `11-assertion-sampling.md` 작성** — clocked sampling (Observed), 각 함수의 정확한 의미, n-cycle lookback.

- [ ] **Step 4: 커밋**

```bash
git add docs/preview/hdl-reference/system-tasks/09-random.md docs/preview/hdl-reference/system-tasks/11-assertion-sampling.md docs/preview/research-log/system-tasks-random-assertion-*.md
git commit -m "Add system-tasks: 09-random + 11-assertion-sampling

Random + 분포 함수, SVA sampling functions (\$past/\$rose/\$fell/\$stable/\$changed). IEEE 1800-2017 §18/§16 (research).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 15: system-tasks 12-introspection + 13-misc

**Files:**
- Create: `docs/preview/hdl-reference/system-tasks/12-introspection.md`
- Create: `docs/preview/hdl-reference/system-tasks/13-misc.md`
- Create: `docs/preview/research-log/system-tasks-introspection-misc-2026-MM-DD.md`

- [ ] **Step 1: research 1~2 라운드**

프롬프트:

```
research SystemVerilog system functions: (A) introspection — $typename(expr), $cast(dest, src), $isunbounded, $size(arr [, dim]), $left/$right/$low/$high/$increment/$dimensions(arr) — type/range queries on arrays/types; (B) misc — $value$plusargs("fmt=%d", var) and $test$plusargs("flag"), $system("shell cmd"), $error/$warning/$info/$fatal (elaboration messages and assertion severity), $exit (test phase control). Primary: IEEE 1800-2017 §20.6/§20.11/§20.7/§20.10. Icarus and Verilator support level. 한국어 narrative.
```

결과 → `research-log/system-tasks-introspection-misc-<date>.md`.

- [ ] **Step 2: `12-introspection.md` 작성** — 타입/배열 쿼리, `$cast` 동적 형변환.

- [ ] **Step 3: `13-misc.md` 작성** — `$value$plusargs` 포맷 패턴, `$test$plusargs` 플래그, `$system`(보안 경고 포함), severity tasks.

- [ ] **Step 4: 커밋**

```bash
git add docs/preview/hdl-reference/system-tasks/12-introspection.md docs/preview/hdl-reference/system-tasks/13-misc.md docs/preview/research-log/system-tasks-introspection-misc-*.md
git commit -m "Add system-tasks: 12-introspection + 13-misc

Type/array queries + plusargs + system shell + severity tasks. IEEE 1800-2017 §20.6/§20.7/§20.10/§20.11 (research).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

**Checkpoint:** Group D 종료. system-tasks 14 문서 완성.

---

## Group E — Verilog 참조 12 문서 (Task 16~19)

각 task는 3~4 파일을 묶는다. **공통 작성 템플릿**: 각 doc은 (1) 개요, (2) 문법(BNF 또는 예시), (3) 의미·예시, (4) 합성가능성, (5) 본 프로젝트 구현 메모(어느 크레이트에서 처리), (6) Sources. 각 task당 research 1~2 라운드.

### Task 16: verilog/{00-index, 01-lexical, 02-data-types, 03-expressions-operators}

**Files:**
- Create: `docs/preview/hdl-reference/verilog/00-index.md`
- Create: `docs/preview/hdl-reference/verilog/01-lexical.md`
- Create: `docs/preview/hdl-reference/verilog/02-data-types.md`
- Create: `docs/preview/hdl-reference/verilog/03-expressions-operators.md`
- Create: `docs/preview/research-log/verilog-lexical-types-expr-2026-MM-DD.md`

- [ ] **Step 1: research 1~2 라운드 — Verilog lexical/types/expr**

프롬프트:

```
research Verilog (IEEE 1364-2005 / subset within IEEE 1800-2017) precisely: (A) lexical — whitespace, comments (//, /* */), identifiers (simple + escaped \id ;), keywords list, number literals (decimal/binary/octal/hex, signed/unsigned, x/z digits, sized vs unsized, real); (B) data types — net types (wire, tri, wand, wor, tri0, tri1, supply0, supply1, trireg), reg/integer/real/time, packed/unpacked vectors, parameter/localparam, memory (reg [N:0] mem [0:M]); (C) expressions/operators — full operator precedence table, bit/logical/reduction/shift/equality/comparison/conditional, signed arithmetic rules (1364-2001+), x/z propagation. Primary: IEEE 1800-2017 §5/§6 (Verilog-compat subset) + IEEE 1364-2005 §2/§3/§4. 한국어 narrative.
```

결과 → `research-log/verilog-lexical-types-expr-<date>.md`.

- [ ] **Step 2: `00-index.md` 작성 (verilog 폴더 인덱스)**

```
# 00 · Verilog (IEEE 1364) Reference

본 폴더는 Verilog (IEEE 1364-2005) — SV에 흡수된 부분집합 — 의 문법/구조 참조.

## 파일
| # | 파일 | 주제 |
|---|---|---|
| 01 | lexical | 토큰·식별자·숫자 리터럴·주석 |
| 02 | data-types | net/wire/reg/integer/real/vector/parameter |
| 03 | expressions-operators | 연산자·우선순위·signed/x/z |
| 04 | modules-hierarchy | module/port/instantiation/parameter/generate |
| 05 | behavioral | initial/always/blocking(=)/non-blocking(<=) |
| 06 | procedural-statements | if/case/for/while/repeat/forever/fork-join |
| 07 | tasks-functions | task/function/automatic |
| 08 | gate-level | primitives/UDP/drive strength |
| 09 | compiler-directives | `\`timescale/\`define/\`ifdef/\`include` |
| 10 | system-tasks | 본 폴더 개요 + ../system-tasks/ cross-link |
| 11 | synthesizability | 합성 가능/조건부/비합성 매핑 |

## 본 프로젝트 입장
SV 프론트엔드가 Verilog를 완전 흡수 — 별도 Verilog 프론트엔드 없음. 본 폴더는 RTL 사용자가 Verilog 서브셋만 쓸 때의 참조.

## Sources
- 본 spec §10
- IEEE 1364-2005, IEEE 1800-2017
```

- [ ] **Step 3: `01-lexical.md` 작성** — 공통 템플릿. 토큰 BNF, 키워드 표(Verilog-2005 키워드), 숫자 리터럴 (`8'hAB`, `4'b1010`, `12'd255`, `'sd5` 등) 상세, escaped identifier.

- [ ] **Step 4: `02-data-types.md` 작성** — net 타입 9종 표(default 동작), reg vs wire 의미, vector 선언, parameter/localparam, memory.

- [ ] **Step 5: `03-expressions-operators.md` 작성** — 연산자 우선순위 표 전체, signed arithmetic 규칙 (`signed`/`$signed`/`$unsigned`), x/z propagation 표, reduction 연산자.

- [ ] **Step 6: 커밋**

```bash
git add docs/preview/hdl-reference/verilog/00-index.md docs/preview/hdl-reference/verilog/01-lexical.md docs/preview/hdl-reference/verilog/02-data-types.md docs/preview/hdl-reference/verilog/03-expressions-operators.md docs/preview/research-log/verilog-lexical-types-expr-*.md
git commit -m "Add verilog refs: 00-index + 01-lexical + 02-data-types + 03-expressions-operators

IEEE 1364-2005 §2-§4 + IEEE 1800-2017 §5-§6 부분집합 (research).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 17: verilog/{04-modules-hierarchy, 05-behavioral, 06-procedural-statements}

**Files:**
- Create: `docs/preview/hdl-reference/verilog/04-modules-hierarchy.md`
- Create: `docs/preview/hdl-reference/verilog/05-behavioral.md`
- Create: `docs/preview/hdl-reference/verilog/06-procedural-statements.md`
- Create: `docs/preview/research-log/verilog-modules-behavioral-2026-MM-DD.md`

- [ ] **Step 1: research 1~2 라운드 — modules/behavioral/procedural**

프롬프트:

```
research Verilog modules and behavioral constructs: (A) modules — module declaration (ANSI vs non-ANSI port lists), ports (input/output/inout, port direction inference), parameter/defparam (defparam deprecated), instantiation (named vs positional), generate blocks (for/if/case generate), hierarchical names; (B) behavioral — initial vs always blocks, always @(...) sensitivity (level vs edge), blocking (=) vs non-blocking (<=) timing semantics with examples that show NBA region effects; (C) procedural statements — if/else, case/casex/casez (casez vs casex pitfalls, full_case/parallel_case directives), for/while/repeat/forever, disable, fork-join (and ;join_any/join_none in SV), delay #d and event @(...). Primary: IEEE 1800-2017 §23/§9/§12. 한국어 narrative.
```

결과 → `research-log/verilog-modules-behavioral-<date>.md`.

- [ ] **Step 2: `04-modules-hierarchy.md` 작성** — ANSI/non-ANSI 포트, parameter/defparam, instantiation, generate, 계층 이름.

- [ ] **Step 3: `05-behavioral.md` 작성** — initial/always 차이, sensitivity list, blocking vs NBA 의미 (스케줄링 영역 cross-link to §06-engine).

- [ ] **Step 4: `06-procedural-statements.md` 작성** — 제어 구조 + 반복 + casex/casez 함정 + fork-join.

- [ ] **Step 5: 커밋**

```bash
git add docs/preview/hdl-reference/verilog/04-modules-hierarchy.md docs/preview/hdl-reference/verilog/05-behavioral.md docs/preview/hdl-reference/verilog/06-procedural-statements.md docs/preview/research-log/verilog-modules-behavioral-*.md
git commit -m "Add verilog refs: 04-modules-hierarchy + 05-behavioral + 06-procedural-statements

Modules, ports, generate, initial/always, blocking/NBA timing, control flow (research).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 18: verilog/{07-tasks-functions, 08-gate-level, 09-compiler-directives}

**Files:**
- Create: `docs/preview/hdl-reference/verilog/07-tasks-functions.md`
- Create: `docs/preview/hdl-reference/verilog/08-gate-level.md`
- Create: `docs/preview/hdl-reference/verilog/09-compiler-directives.md`
- Create: `docs/preview/research-log/verilog-tasks-gates-directives-2026-MM-DD.md`

- [ ] **Step 1: research 1~2 라운드 — tasks/functions, gate-level, directives**

프롬프트:

```
research Verilog: (A) tasks and functions — declaration syntax, input/output/inout args, automatic vs static, return value (function return statement vs name assignment), recursion, system tasks ($display etc.) vs user tasks; (B) gate-level primitives — and/or/nand/nor/xor/xnor/buf/not/bufif0/bufif1/notif0/notif1/nmos/pmos/cmos/rnmos/rpmos/rcmos/tran/tranif0/tranif1/rtran/rtranif0/rtranif1/pullup/pulldown; drive strengths (supply/strong/pull/weak/highz); UDP (user-defined primitives) — combinational and sequential UDPs; (C) compiler directives — \`define (with macro args), \`undef, \`ifdef/\`ifndef/\`else/\`elsif/\`endif, \`include, \`timescale, \`default_nettype (none vs wire), \`begin_keywords/\`end_keywords, \`pragma, \`line, \`resetall, \`celldefine/\`endcelldefine. Primary: IEEE 1800-2017 §13/§28/§22. 한국어 narrative.
```

결과 → `research-log/verilog-tasks-gates-directives-<date>.md`.

- [ ] **Step 2: `07-tasks-functions.md` 작성** — task/function 차이, automatic, 재귀, 반환 방식.

- [ ] **Step 3: `08-gate-level.md` 작성** — 전체 게이트 프리미티브 표, drive strength, UDP (combinational + sequential UDP table 예시).

- [ ] **Step 4: `09-compiler-directives.md` 작성** — 모든 지시어, 매크로 인자, `\`default_nettype none` 권장.

- [ ] **Step 5: 커밋**

```bash
git add docs/preview/hdl-reference/verilog/07-tasks-functions.md docs/preview/hdl-reference/verilog/08-gate-level.md docs/preview/hdl-reference/verilog/09-compiler-directives.md docs/preview/research-log/verilog-tasks-gates-directives-*.md
git commit -m "Add verilog refs: 07-tasks-functions + 08-gate-level + 09-compiler-directives

Tasks/functions, gate primitives + UDP, compiler directives (research).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 19: verilog/{10-system-tasks, 11-synthesizability}

**Files:**
- Create: `docs/preview/hdl-reference/verilog/10-system-tasks.md`
- Create: `docs/preview/hdl-reference/verilog/11-synthesizability.md`
- Create: `docs/preview/research-log/verilog-synthesizability-2026-MM-DD.md`

- [ ] **Step 1: research 1~2 라운드 — Verilog 합성 가능 서브셋**

프롬프트:

```
research Verilog synthesizable subset — what Verilog-2005 constructs are universally synthesizable (Synopsys DC, Xilinx Vivado, Cadence Genus 공통), conditionally synthesizable (tool-specific), and non-synthesizable: (1) initial blocks (FPGA: 초기값 OK / ASIC: 일반 안 됨), (2) delays #d in always (대부분 무시 또는 에러), (3) UDP (대부분 안 됨), (4) full_case/parallel_case (deprecated, 권장 안 됨), (5) reg/wire 데이터 타입, (6) integer/real (real 비합성), (7) gate-level vs RTL, (8) tasks (보통 합성, 인자 자동/static 차이), (9) functions (합성 가능), (10) recursion (보통 안 됨), (11) loops (synthesizable when bound), (12) generate (합성 권장), (13) defparam (deprecated). Cite Vivado UG901, Synopsys synthesis guides, IEEE 1364.1 (synthesis subset standard, if applicable). 한국어 narrative.
```

결과 → `research-log/verilog-synthesizability-<date>.md`.

- [ ] **Step 2: `10-system-tasks.md` 작성** — 본 폴더 개요 + `../system-tasks/` 카테고리 docs로 cross-link 표.

- [ ] **Step 3: `11-synthesizability.md` 작성** — Verilog 구문별 ✅/⚠️/❌ 매핑 + 권장 패턴 + 도구별 차이 박스.

- [ ] **Step 4: 커밋**

```bash
git add docs/preview/hdl-reference/verilog/10-system-tasks.md docs/preview/hdl-reference/verilog/11-synthesizability.md docs/preview/research-log/verilog-synthesizability-*.md
git commit -m "Add verilog refs: 10-system-tasks + 11-synthesizability

System tasks cross-link + Verilog 합성 가능/조건부/비합성 매핑 (research, IEEE 1364.1 + Vivado/Synopsys guides).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

**Checkpoint:** Group E 종료. Verilog 참조 12 문서 완성.

---

## Group F — SystemVerilog 참조 10 문서 (Task 20~22)

### Task 20: systemverilog/{00-index, 01-data-types, 02-arrays, 03-procedural}

**Files:**
- Create: `docs/preview/hdl-reference/systemverilog/00-index.md`
- Create: `docs/preview/hdl-reference/systemverilog/01-data-types.md`
- Create: `docs/preview/hdl-reference/systemverilog/02-arrays.md`
- Create: `docs/preview/hdl-reference/systemverilog/03-procedural.md`
- Create: `docs/preview/research-log/sv-types-arrays-procedural-2026-MM-DD.md`

- [ ] **Step 1: research 1~2 라운드 — SV types/arrays/procedural**

프롬프트:

```
research SystemVerilog (IEEE 1800-2017) extensions over Verilog: (A) data types — 2-state (bit, byte, shortint, int, longint) vs 4-state (logic, reg, integer, time), enum (with explicit values + first/last/next/prev), struct (packed vs unpacked, randc/rand fields), union (packed/tagged), typedef, string, chandle, virtual interface; (B) arrays — packed multi-dim, unpacked, dynamic ([]) with new[N]/delete, associative ([key_type]/[*]/[string]) with exists/first/last/next/prev/num, queue ([$]) with push_back/push_front/pop_back/pop_front/insert/delete, array methods (.size/.delete/.sort/.rsort/.shuffle/.reverse/.find/.find_first/.sum/.product/.and/.or/.xor); (C) procedural — always_comb/always_ff/always_latch semantics + checks, unique/priority case/if, foreach, do-while, void function with return, ref/const ref args. Primary: IEEE 1800-2017 §6/§7/§9/§12/§13. 한국어 narrative.
```

결과 → `research-log/sv-types-arrays-procedural-<date>.md`.

- [ ] **Step 2: `00-index.md` 작성 (SV 폴더 인덱스)**

```
# 00 · SystemVerilog (IEEE 1800) Reference

본 폴더는 SystemVerilog (IEEE 1800-2017) — Verilog 흡수 + 확장 — 의 신규/확장 기능 참조.

## 파일
| # | 파일 | 주제 |
|---|---|---|
| 01 | data-types | logic/bit/int, enum/struct/union/typedef, 2-state vs 4-state |
| 02 | arrays | packed/unpacked/dynamic/associative/queue + 배열 메소드 |
| 03 | procedural | always_comb/_ff/_latch, unique/priority, foreach |
| 04 | interfaces | interface/modport/clocking block |
| 05 | packages | package/import/export/$unit |
| 06 | classes-oop | class/상속 (검증용, 비합성) |
| 07 | assertions-sva | immediate/concurrent assertion, property, sequence |
| 08 | functions-tasks | SV 추가 시스템 태스크 cross-link |
| 09 | synthesizability | SV 합성 가능/조건부/비합성 매핑 |

## Verilog와의 관계
SV는 Verilog (IEEE 1364) 슈퍼셋. Verilog 문서(`../verilog/`)는 부분집합 정리, 본 폴더는 SV 추가/변경 사항.

## Sources
- 본 spec §10
- IEEE 1800-2017
```

- [ ] **Step 3: `01-data-types.md` 작성** — 2-state vs 4-state 표, logic vs reg, enum/struct/union/typedef, packed vs unpacked.

- [ ] **Step 4: `02-arrays.md` 작성** — 4 종 배열, 각 메소드, 사용 시나리오, 합성 가능 부분집합.

- [ ] **Step 5: `03-procedural.md` 작성** — always_comb/_ff/_latch elaboration 검사, unique/priority, foreach 동작.

- [ ] **Step 6: 커밋**

```bash
git add docs/preview/hdl-reference/systemverilog/00-index.md docs/preview/hdl-reference/systemverilog/01-data-types.md docs/preview/hdl-reference/systemverilog/02-arrays.md docs/preview/hdl-reference/systemverilog/03-procedural.md docs/preview/research-log/sv-types-arrays-procedural-*.md
git commit -m "Add SV refs: 00-index + 01-data-types + 02-arrays + 03-procedural

IEEE 1800-2017 §6/§7/§9/§12 extensions: 2-state vs 4-state, arrays, always_comb/_ff/_latch (research).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 21: systemverilog/{04-interfaces, 05-packages, 06-classes-oop}

**Files:**
- Create: `docs/preview/hdl-reference/systemverilog/04-interfaces.md`
- Create: `docs/preview/hdl-reference/systemverilog/05-packages.md`
- Create: `docs/preview/hdl-reference/systemverilog/06-classes-oop.md`
- Create: `docs/preview/research-log/sv-interfaces-packages-classes-2026-MM-DD.md`

- [ ] **Step 1: research 1~2 라운드**

프롬프트:

```
research SystemVerilog: (A) interfaces — interface declaration, signals/parameters, modport (input/output/inout/import/export tasks/functions), clocking block (default input/output skew, programmable assertions), interface parameter, virtual interface for class-based verification; (B) packages — package declaration, import (explicit, wildcard *::), export, $unit (compilation unit), import precedence rules; (C) classes — class declaration, constructors (new), inheritance (extends, super.new), polymorphism (virtual methods), parameterized class, static members, scope resolution (::), this, randomization (rand/randc, constraint, randomize() method) — note: classes are 100% non-synthesizable; (D) virtual interface — bridge between class verification and module hierarchy. Primary: IEEE 1800-2017 §25/§26/§8/§18. 한국어 narrative.
```

결과 → `research-log/sv-interfaces-packages-classes-<date>.md`.

- [ ] **Step 2: `04-interfaces.md` 작성** — interface 선언, modport, clocking block, virtual interface 연결.

- [ ] **Step 3: `05-packages.md` 작성** — package 선언, import 패턴, $unit, 충돌 해결.

- [ ] **Step 4: `06-classes-oop.md` 작성** — 클래스 모든 OOP 기능, randomization, ❌ 합성불가 표기 강조.

- [ ] **Step 5: 커밋**

```bash
git add docs/preview/hdl-reference/systemverilog/04-interfaces.md docs/preview/hdl-reference/systemverilog/05-packages.md docs/preview/hdl-reference/systemverilog/06-classes-oop.md docs/preview/research-log/sv-interfaces-packages-classes-*.md
git commit -m "Add SV refs: 04-interfaces + 05-packages + 06-classes-oop

Interface/modport/clocking, package/import/\$unit, class OOP + randomization (비합성). IEEE 1800-2017 §25/§26/§8/§18 (research).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 22: systemverilog/{07-assertions-sva, 08-functions-tasks, 09-synthesizability}

**Files:**
- Create: `docs/preview/hdl-reference/systemverilog/07-assertions-sva.md`
- Create: `docs/preview/hdl-reference/systemverilog/08-functions-tasks.md`
- Create: `docs/preview/hdl-reference/systemverilog/09-synthesizability.md`
- Create: `docs/preview/research-log/sv-sva-synthesis-2026-MM-DD.md`

- [ ] **Step 1: research 1~2 라운드**

프롬프트:

```
research SystemVerilog: (A) SVA (SystemVerilog Assertions) — immediate (assert/assume/cover) vs concurrent assertions, property declaration (always/eventually/until/implies/throughout), sequence declaration (cycle delay ##N, ##[m:n], repetition [*N] [=N] [->N], goto), sampling (Observed region), clocking block in assertions, action blocks (pass/fail) — exact IEEE 1800-2017 §16 grammar; (B) SV functions/tasks additions over Verilog — let (compile-time inlining), void function, automatic by default in some contexts, ref args; (C) SV synthesizable subset for RTL — what's universally synthesizable (logic, always_comb/_ff/_latch, enum, struct/typedef, packed arrays, interface/modport with restrictions), conditionally (foreach, unique/priority, dynamic arrays for parameter), non-synthesizable (class, randomization, dynamic memory, queues for non-FIFO use, assertions, virtual interfaces). Primary: IEEE 1800-2017 §16/§13 + Vivado UG901 SV chapter. 한국어 narrative.
```

결과 → `research-log/sv-sva-synthesis-<date>.md`.

- [ ] **Step 2: `07-assertions-sva.md` 작성** — immediate vs concurrent, property/sequence 문법, action blocks, clocking 의존.

- [ ] **Step 3: `08-functions-tasks.md` 작성** — SV 추가 시스템 태스크 cross-link to `../system-tasks/`, let, void function, ref args.

- [ ] **Step 4: `09-synthesizability.md` 작성** — SV 구문별 ✅/⚠️/❌ + 권장 RTL 패턴 + 도구 비교.

- [ ] **Step 5: 커밋**

```bash
git add docs/preview/hdl-reference/systemverilog/07-assertions-sva.md docs/preview/hdl-reference/systemverilog/08-functions-tasks.md docs/preview/hdl-reference/systemverilog/09-synthesizability.md docs/preview/research-log/sv-sva-synthesis-*.md
git commit -m "Add SV refs: 07-assertions-sva + 08-functions-tasks + 09-synthesizability

SVA grammar + property/sequence, let/void function/ref args, SV 합성 가능 서브셋 (research).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

**Checkpoint:** Group F 종료. SV 참조 10 문서 완성.

---

## Group G — VHDL 참조 10 문서 (Task 23~25)

### Task 23: vhdl/{00-index, 01-lexical, 02-types, 03-objects}

**Files:**
- Create: `docs/preview/hdl-reference/vhdl/00-index.md`
- Create: `docs/preview/hdl-reference/vhdl/01-lexical.md`
- Create: `docs/preview/hdl-reference/vhdl/02-types.md`
- Create: `docs/preview/hdl-reference/vhdl/03-objects.md`
- Create: `docs/preview/research-log/vhdl-lexical-types-2026-MM-DD.md`

- [ ] **Step 1: research 다라운드 — VHDL lexical/types/objects**

프롬프트:

```
research VHDL (IEEE 1076-2008) precisely: (A) lexical — case-insensitive identifiers (extended \id\ are case-sensitive), comments (-- single line, /* block */ in 2008+), reserved words list (96+ keywords), literals (decimal/based 16#FF# / 2#1010#, character 'c', string "abc", bit_string "10101"), bit_string b/o/x/d prefixes; (B) types — scalar (integer, natural, positive, real, time with units, enumeration like bit, boolean, character, severity_level, file_open_kind), composite (array — constrained vs unconstrained, record), access (pointer), file, std_logic_1164 (std_ulogic 9-value 'U/X/0/1/Z/W/L/H/-', std_logic resolution), numeric_std (signed/unsigned arithmetic); (C) objects — signal (drives net), variable (process-local), constant, generic (parameter), file. Object modes for ports (in/out/inout/buffer/linkage). Primary: IEEE 1076-2008 §5/§6/§9 + Doulos VHDL refs. 한국어 narrative.
```

결과 → `research-log/vhdl-lexical-types-<date>.md`.

- [ ] **Step 2: `00-index.md` 작성 (VHDL 폴더 인덱스)**

```
# 00 · VHDL (IEEE 1076) Reference

본 폴더는 VHDL (IEEE 1076-2008) 문법·패키지·합성가능성 참조.

## 파일
| # | 파일 | 주제 |
|---|---|---|
| 01 | lexical | 토큰·식별자·주석·리터럴 |
| 02 | types | scalar/composite, std_logic_1164, numeric_std |
| 03 | objects | signal/variable/constant/generic + 포트 modes |
| 04 | design-units | entity/architecture/package/configuration/library |
| 05 | concurrent | process, concurrent assignment, component, generate |
| 06 | sequential | if/case/loop, wait, variable assignment |
| 07 | subprograms | function/procedure |
| 08 | packages-libraries | ieee, std_logic_1164, numeric_std, std |
| 09 | synthesizability | VHDL 합성 가능/조건부/비합성 매핑 |

## 본 프로젝트 위치
Phase 3 진입 시 VHDL 프론트엔드를 별도 추가 (공유 IR 위에). std_logic_1164 / numeric_std는 hdl-builtins의 VHDL 패키지 모듈로.

## Sources
- 본 spec §10
- IEEE 1076-2008
```

- [ ] **Step 3: `01-lexical.md` 작성** — case-insensitive 규칙, 키워드 표, 리터럴 종류, bit_string 접두사.

- [ ] **Step 4: `02-types.md` 작성** — scalar/composite, std_logic_1164 9-value 표 + resolution, numeric_std signed/unsigned.

- [ ] **Step 5: `03-objects.md` 작성** — signal vs variable 의미 차이 (delta cycle), constant/generic, file, 포트 modes.

- [ ] **Step 6: 커밋**

```bash
git add docs/preview/hdl-reference/vhdl/00-index.md docs/preview/hdl-reference/vhdl/01-lexical.md docs/preview/hdl-reference/vhdl/02-types.md docs/preview/hdl-reference/vhdl/03-objects.md docs/preview/research-log/vhdl-lexical-types-*.md
git commit -m "Add VHDL refs: 00-index + 01-lexical + 02-types + 03-objects

IEEE 1076-2008 §5/§6/§9: lexical, types (std_logic_1164, numeric_std), signal/variable/constant/generic (research).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 24: vhdl/{04-design-units, 05-concurrent, 06-sequential}

**Files:**
- Create: `docs/preview/hdl-reference/vhdl/04-design-units.md`
- Create: `docs/preview/hdl-reference/vhdl/05-concurrent.md`
- Create: `docs/preview/hdl-reference/vhdl/06-sequential.md`
- Create: `docs/preview/research-log/vhdl-design-units-statements-2026-MM-DD.md`

- [ ] **Step 1: research 다라운드 — VHDL design units / concurrent / sequential**

프롬프트:

```
research VHDL (IEEE 1076-2008): (A) design units — entity (with generics, ports), architecture (multiple per entity, configuration to select), package (declaration + body), library/use clauses (library ieee; use ieee.std_logic_1164.all;), configuration (binding component to entity); (B) concurrent statements — process (with sensitivity list or wait), concurrent signal assignment (simple, conditional, selected), component instantiation (positional vs named, generic map, port map), generate (for-generate, if-generate, case-generate in 2008+), block, concurrent procedure call, concurrent assertion; (C) sequential statements (within process) — if/case (matching case? in 2008), loop (for, while, infinite), wait (until, on, for), variable assignment (:=), signal assignment (<= with after delay), return, next, exit, assert, report, severity. Primary: IEEE 1076-2008 §3/§11/§10. 한국어 narrative.
```

결과 → `research-log/vhdl-design-units-statements-<date>.md`.

- [ ] **Step 2: `04-design-units.md` 작성** — entity/architecture, package decl vs body, library/use, configuration.

- [ ] **Step 3: `05-concurrent.md` 작성** — process sensitivity vs wait, 동시 신호 할당 (simple/conditional/selected), component instantiation, generate, block.

- [ ] **Step 4: `06-sequential.md` 작성** — process 내부 제어 + wait의 정확한 의미 (sensitivity list 동등 변환), variable vs signal assignment.

- [ ] **Step 5: 커밋**

```bash
git add docs/preview/hdl-reference/vhdl/04-design-units.md docs/preview/hdl-reference/vhdl/05-concurrent.md docs/preview/hdl-reference/vhdl/06-sequential.md docs/preview/research-log/vhdl-design-units-statements-*.md
git commit -m "Add VHDL refs: 04-design-units + 05-concurrent + 06-sequential

Entity/architecture/package/library, process + concurrent assigns, sequential (wait, control flow). IEEE 1076-2008 §3/§10/§11 (research).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 25: vhdl/{07-subprograms, 08-packages-libraries, 09-synthesizability}

**Files:**
- Create: `docs/preview/hdl-reference/vhdl/07-subprograms.md`
- Create: `docs/preview/hdl-reference/vhdl/08-packages-libraries.md`
- Create: `docs/preview/hdl-reference/vhdl/09-synthesizability.md`
- Create: `docs/preview/research-log/vhdl-subprograms-pkg-synth-2026-MM-DD.md`

- [ ] **Step 1: research 다라운드 — VHDL subprograms / pkg / synth**

프롬프트:

```
research VHDL (IEEE 1076-2008): (A) subprograms — function (pure vs impure), procedure, parameter modes (in/out/inout, default in for function), overloading, operator overloading, subprogram body, recursion; (B) standard packages — std.standard (built-in, no use clause needed), std.textio (file I/O for testbench), ieee.std_logic_1164 (std_ulogic/std_logic + resolution + conversion to_bit/to_stdulogic/to_stdlogic + operators), ieee.numeric_std (signed/unsigned + arithmetic + conversion to_integer/to_signed/to_unsigned + shift_left/right/rotate), ieee.std_logic_arith (deprecated, avoid), ieee.numeric_std_unsigned, ieee.math_real, ieee.fixed_pkg / float_pkg (2008+); (C) synthesizable subset — universally synthesizable (signal of std_logic/_vector, integer with range, process with full sensitivity, if/case, for-loop with bound, std_logic_1164/numeric_std), conditionally (record types for ports — tool support varies, generate, recursion 일부), non-synthesizable (file I/O, access types, wait for, after delays, std.textio, real arithmetic 대부분, infinite loops, dynamic memory). Primary: IEEE 1076-2008 §4/§15/§16 + Doulos guides + Vivado UG901 VHDL chapter. 한국어 narrative.
```

결과 → `research-log/vhdl-subprograms-pkg-synth-<date>.md`.

- [ ] **Step 2: `07-subprograms.md` 작성** — function/procedure, pure/impure, overloading.

- [ ] **Step 3: `08-packages-libraries.md` 작성** — 표준 패키지 전체 (std/ieee), std_logic_arith 경고, math_real 비합성 메모.

- [ ] **Step 4: `09-synthesizability.md` 작성** — VHDL 구문별 ✅/⚠️/❌ + 권장 패턴.

- [ ] **Step 5: 커밋**

```bash
git add docs/preview/hdl-reference/vhdl/07-subprograms.md docs/preview/hdl-reference/vhdl/08-packages-libraries.md docs/preview/hdl-reference/vhdl/09-synthesizability.md docs/preview/research-log/vhdl-subprograms-pkg-synth-*.md
git commit -m "Add VHDL refs: 07-subprograms + 08-packages-libraries + 09-synthesizability

Function/procedure + overloading, std/ieee packages (std_logic_1164, numeric_std, math_real), VHDL 합성 서브셋 (research, IEEE 1076-2008 §4/§15 + Vivado UG901).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

**Checkpoint:** Group G 종료. VHDL 참조 10 문서 완성. **전체 docs/preview/ 세트 (~60 files) 완성.**

---

## 종료 시 점검

### 최종 정합성 검증
- [ ] **모든 문서 하단에 `## Sources` 섹션 존재** — `grep -L "^## Sources$" docs/preview/**/*.md`로 누락 확인.
- [ ] **research-log/ 의 각 파일이 YAML 머리말 + Sources 푸터 갖춤** — `head -10 docs/preview/research-log/*.md` 점검.
- [ ] **한자/추상 명사 헤더 부재** — `grep -E "^#+ (起|承|轉|結|기승전결|도입|본론|결론)" docs/preview/`로 확인.
- [ ] **IEEE 표준 verbatim 복제 없음** — `git diff main` 검토 시 IEEE 본문 인용으로 의심되는 긴 블록 추출 후 요약으로 재작성.
- [ ] **spec과의 일관성** — 코드네임 `vitamin`, CLI `vita`, 크레이트 10개 (포함 `hdl-builtins`), Phase 1/2/3 범위, VCD RTL-driven 표현 통일.

### PR 단계 (사용자 명시 시)
- [ ] **PR 생성** — `gh pr create --base main --head feat/vitamin-docs-preview --title "Add vitamin docs/preview set" --body "..."`. 본문에 spec 링크 + 25 task 요약.
- [ ] **사용자 명시 시 push** — `git push -u origin feat/vitamin-docs-preview`.

---

## Self-Review (작성자 본인이 점검)

**1. Spec coverage** — spec의 각 섹션이 plan의 task로 매핑:
- spec §1 비전 → Task 2 (00-overview)
- spec §2 목표/비목표/성공기준 → Task 2 (01-goals)
- spec §3 타깃 환경 → Task 3 (03-build)
- spec §4 Rust 결정 → Task 3 (02-language)
- spec §5 아키텍처 → Task 4 (04-architecture)
- spec §6 시뮬 엔진 → Task 6 (06-simulation-engine)
- spec §7 VCD → Task 7 (07-vcd-format)
- spec §8 검증 → Task 8 (09-testing-and-verification)
- spec §9 로드맵 → Task 5 (05-roadmap) + 시스템 태스크 Phase 정보가 Group D 인덱스에 반영
- spec §10 산출물 구조 → Group A·C·D·E·F·G 전체
- spec §11 조사 방법론 → Task 9 (12-research-methodology) + 모든 research-필요 task의 research 스킬 호출
- spec §12 출처/저작권 → Task 9 (11-sources-and-citations) + 모든 doc Sources 섹션
- spec §13 용어 → Task 8 (10-glossary)
- spec §14 리스크 → Task 5 (05-roadmap 리스크/의존성 섹션)
- 모든 spec 섹션이 task로 커버됨. ✅

**2. Placeholder scan** — "TBD/TODO/fill in later" 패턴 없음. "Add appropriate error handling" 같은 모호 지시 없음. 각 task의 모든 step은 구체 명령/문서 구조/research 프롬프트를 갖춤. ✅

**3. Type consistency** — 크레이트 이름(`hdl-builtins`), 코드네임(`vitamin`), CLI(`vita`), Phase 번호(1/2/3) 일관. system-tasks 카테고리 번호(01~13) 전 doc 일관. ✅

**4. 누락 위험 영역**
- Phase 1 핵심 system tasks가 Task 11(display/sim-control/time)과 Task 12(dump) 두 task에 걸쳐 있음 — 의도된 분할(파일 묶기).
- Verilog/SV 합성가능성 doc은 각각 Task 19/22에서 별개 research — 의도된 분리.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-28-vitamin-docs-preview-set.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration. 적합한 이유: 25+ tasks의 독립성·검증 필요.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints. 적합한 이유: 빠른 전체 진행.

**Which approach?**
