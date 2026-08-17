# 00 · Vitamin 개요

## 비전

**Vitamin**(`vita` CLI placeholder)은 Ubuntu · RHEL · macOS에서 **소스 빌드만으로 동작**하는 RTL 시뮬레이션 EDA 툴이다.
설계 목표를 세 가지로 압축한다:

1. **정밀도(precision)** — `timescale` 개념을 충실히 구현해 근소한 타이밍 틀어짐도 시뮬레이션에서 잡아낸다.
2. **이식성(portability)** — OS가 교체되어도 동일한 원문 소스를 빌드해 동일한 결과를 낸다.
3. **성능(performance)** — GC 없는 저수준 언어(Rust)로 시뮬레이션 처리 속도를 끌어올린다.

코드네임 `vitamin`, CLI 작업명 `vita`(및 단계별 `vcmp`/`velab`/`vrun`)는 현재 임시 placeholder다.

## 무엇을 만드나

표준 RTL 시뮬레이터 처리 흐름을 구현한다:

```
HDL 소스
  → preprocess   (`define / `ifdef / `include / `timescale)
  → lex          (토큰 스트림)
  → parse        (언어별 AST · 문법 검사)
  → elaborate    (파라미터 해소 · 계층 연결성 · 타입/포트 정합 · 다중구동 검사)
  → sim engine   (이벤트 구동 커널 · timescale 시간 모델)
  → VCD writer   (RTL dump 태스크 호출 시에만 활성 · 파형 = VCD + FST(`.fst` 확장자 디스패치, VCD→FST 트랜스코드))
```

검사는 별도 단계가 아니라 각 단계 내부에서 수행된다.
문법 오류는 parse에서, 연결성·타입·다중구동 등 정합성 오류는 elaboration에서 잡는다.
VCD 파형은 자동 항상-덤프가 아니며, RTL 코드가 dump 시스템 태스크(`$dumpfile`, `$dumpvars` 등)를
명시적으로 호출할 때에만 생성된다. 파형 출력은 VCD + FST를 지원한다 —
`$dumpfile("x.fst")`처럼 확장자가 `.fst`(대소문자 무관)면 FST(VCD→FST 트랜스코드), 그 외는 VCD다.

실행은 두 방식이다. `vita` 하나로 compile→elaborate→simulation을 한 번에 돌리거나,
단계별 명령 `vcmp`(compile)·`velab`(elaborate)·`vrun`(simulation)으로 나눠 돌릴 수 있다.
단계별 방식은 상용 EDA(Cadence `xmvlog`/`xmelab`/`xmsim`, Synopsys `vlogan`/`vcs`/`simv`)의
단계 분리에 대응하며, 단계별 독립 빌드·디버깅과 변경 없는 단계 스킵을 가능하게 한다 (상세는 §4 아키텍처).

## 레퍼런스 도구 비교

| 도구 | 카테고리 | 강점 | 본 프로젝트 위치 |
|---|---|---|---|
| **Synopsys VCS** | 상용 컴파일드 시뮬레이터 | 업계 표준 정확도, full SV/UVM, 높은 시뮬레이션 처리량 | 정확도 기준점(레퍼런스). 오픈소스 범위에서의 동등성 목표 |
| **Cadence Xcelium** | 상용 컴파일드 시뮬레이터 | 멀티-언어(SV/VHDL/e), 병렬 컴파일, 고급 검증 환경 | 상용 생태계 참조. 언어 커버리지 로드맵 설정 기준 |
| **Icarus Verilog** | 오픈소스 인터프리터 | 무료·경량·VCD 출력, Verilog-2005/일부 SV 지원, 차등검증 도구 | Phase 1 MVP의 직접 비교 대상 + 차등검증 기준 도구 |
| **Verilator** | 오픈소스 C++ 트랜스파일 | 고속 cycle-accurate 2-state 시뮬레이터, 대형 설계에 적합 | 2-state 정확도 비교 대상; 컴파일드 백엔드 후속 단계의 설계 참조 |
| **Vitamin** | **오픈소스 Rust 인터프리터** | 메모리 안전 · GC 없음 · 3-OS 소스 빌드(cargo) · 4-state · 정확성·이식성 우선, 후속 컴파일드 옵션 보유 | 본 프로젝트 |

## 본 프로젝트의 차별점

- **Rust 코어** — 메모리 안전 + C급 성능 + GC 없음. 이산사건 시뮬레이터의 결정론적 타임스케일 정밀도를 GC 없이 유지한다. Rust의 enum + 패턴매칭은 lexer · parser · elaborator · AST/IR 작성에 이상적이며, 조용한 오답이 치명적인 시뮬레이터에서 메모리 안전성의 가치가 크다.
- **3-OS 소스 빌드 (cargo)** — Ubuntu(LTS) · RHEL(8/9) · macOS(Apple Silicon + Intel)에서 `cargo build` 한 명령으로 재현. 사전 빌드 바이너리 배포에 의존하지 않으며 C 외부 라이브러리 의존을 최소화해 크로스 플랫폼 빌드 마찰을 제거한다.
- **하이브리드 전략 (인터프리터 MVP → IR 경계 너머 컴파일드 옵션)** — Phase 1은 정확성과 VCD 정밀도를 먼저 확보하는 인터프리터 방식. `sim-ir`를 언어 중립 경계로 설계해 재작성 없이 컴파일드/JIT 백엔드를 후속 단계에서 추가할 수 있다. *(2026-08-16 현재: 실행기가 **셋**이고 **기본은 ③이다** — ①`interp`(의미의 정본·오라클) ②`vm` 바이트코드 ③**`native`** 평면 아레나(기본). ③층은 **전체 코퍼스의 100.00% 를 바이트 정확하게 실행**하고(6,470 중 거부 0 · flip 발산 0 · ROADMAP §5.1-ap), 제품 표면은 **`oracle` feature**(기본 ON)로 갈린다 — `--no-default-features` 빌드는 **native 하나**이고 거기선 게이트 거부가 **치명**이다(Phase B 완료 · 삭제한 줄 0 · 구조 = [04 §실행 백엔드 아키텍처](04-architecture.md)). **Phase D 완료(2026-08-17)**: 성능은 벤치 8형태 **전부에서 native < vm** 이고(착수 때 셋에서 졌다 · 최악 2.52×), **기계어 생성은 지어서·배선해서·재서 기각했다**(`jit` feature 로 남아 있으나 **기본 OFF** — 런의 ~38% 가 shim 이고 천장은 op 디스패치 8.9~11.3% · ROADMAP §5.1-be). 해설 = [study/02](../study/02-v1-native-coverage.md))*

## Sources

- 본 spec §1 · §2 — `docs/superpowers/specs/2026-05-26-vitamin-rtl-simulator-design.md`
- Synopsys VCS: https://www.synopsys.com/verification/simulation/vcs.html (접근일: 2026-05-28)
- Cadence Xcelium: https://www.cadence.com/en_US/home/tools/system-design-and-verification/simulation-and-testbench-verification/xcelium-simulator.html (접근일: 2026-05-28)
- Icarus Verilog: https://steveicarus.github.io/iverilog/ (접근일: 2026-05-28)
- Verilator: https://www.veripool.org/verilator/ (접근일: 2026-05-28)
