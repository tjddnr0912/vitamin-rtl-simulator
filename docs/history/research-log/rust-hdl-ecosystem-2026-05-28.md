---
topic: rust-hdl-ecosystem
date: 2026-05-28
rounds: 2
primary_sources_fetched:
  - https://crates.io/api/v1/crates/sv-parser
  - https://crates.io/api/v1/crates/logos
  - https://crates.io/api/v1/crates/chumsky
  - https://crates.io/api/v1/crates/lalrpop
  - https://crates.io/api/v1/crates/ariadne
  - https://crates.io/api/v1/crates/codespan-reporting
  - https://crates.io/api/v1/crates/vcd
  - https://crates.io/api/v1/crates/veryl
  - https://github.com/dalance/sv-parser
  - https://gitlab.com/spade-lang/spade
  - https://github.com/veryl-lang/veryl
  - https://github.com/samitbasu/rust-hdl
  - https://github.com/zesterer/chumsky
  - https://github.com/lalrpop/lalrpop
  - https://spade-lang.org/
queries:
  - "Round 1: sv-parser rust crate IEEE 1800 SystemVerilog dalance crates.io 2025"
  - "Round 1: veryl HDL transpiler rust crate 2025 2026 version"
  - "Round 1: spade HDL rust language compiler version 2025"
  - "Round 1: chumsky parser combinator rust vs lalrpop HDL grammar comparison 2025"
  - "Round 1: logos lexer rust crate version 2025 crates.io"
  - "Round 1: ariadne codespan-reporting rust diagnostic crate 2025 comparison"
  - "Round 1: vcd crate rust Kevin Mehall crates.io version 2025"
  - "Round 1: Rust Verilog simulator open source github 2024 2025"
  - "Round 2: rust-hdl open source RTL simulator written Rust 2024 2025 github stars"
---

# Research: Rust HDL/EDA Ecosystem (2025-2026)

2026-05-28 기준. crates.io API 직접 조회 + GitHub README 교차검증으로 버전·날짜를 확정했다.
WebSearch 2라운드(영문 + 다른 각도) → WebFetch 15개 primary source 직접 확인.

---

## sv-parser (dalance) — SystemVerilog 파서 선례

- **현재 버전:** 0.13.5 (2026-03-30 업데이트)
- **GitHub 스타:** 472 stars
- **라이선스:** MIT OR Apache-2.0
- **저장소:** https://github.com/dalance/sv-parser
- **crates.io 총 다운로드:** 185,611회 (최근 9,064회)

**IEEE 1800 커버리지:** README에서 "IEEE 1800-2017 완전 준수(fully compliant)"를 표방한다. 파서 출력은 IEEE 1800-2017 Annex A의 공식 문법 규칙 이름을 그대로 따른 `SyntaxTree`(구체 구문 트리)를 반환한다. `parse_sv()` API 하나로 SV 소스 파일을 파싱하고, `RefNode` 열거형의 변형 이름이 표준 문법 규칙명과 1:1로 대응한다.

**유지보수 상태:** 2026년 3월까지 업데이트 이력 확인. 활발히 유지됨. `svlint`(SystemVerilog 린터) 등 파생 도구들이 이 크레이트를 의존성으로 사용하고 있어 다운스트림 압력이 유지보수를 뒷받침한다.

**한계:** IEEE 1800-2017 기준이므로 1800-2023 신규 추가 구문은 보장하지 않는다. CST(구체 구문 트리) 중심이라 AST로 변환하려면 추가 작업이 필요하다.

---

## veryl HDL — 현대적 HDL 트랜스파일러

- **현재 버전:** 0.20.0 (2026-05-01 업데이트)
- **GitHub 스타:** 942 stars
- **라이선스:** MIT OR Apache-2.0
- **저장소:** https://github.com/veryl-lang/veryl
- **crates.io 총 다운로드:** 62,686회

**프로젝트 상태:** 활발한 개발 중. 마스터 브랜치 커밋 4,496개, 62개 릴리스. SystemVerilog를 대상 언어(transpile target)로 삼는 현대적 HDL로, Rust/SystemVerilog에서 문법을 차용하고 패키지 매니저·LSP 서버·IDE 통합을 내장한다. ISCA 2025 OSCAR 워크숍(2025년 6월)에서 발표되는 등 학술계에서도 활동 중이다.

**vitamin 프로젝트와의 관계:** veryl은 SV 코드를 생성하는 새 HDL이지, SV 시뮬레이터가 아니다. 직접 의존성은 아니지만, Rust로 HDL 파이프라인을 구축하는 선례로서 아키텍처 참고 가치가 크다.

---

## spade HDL — Rust 영감을 받은 HDL 컴파일러

- **GitHub 스타:** 65 stars (GitHub mirror; 주 저장소는 GitLab)
- **주 저장소:** https://gitlab.com/spade-lang/spade (GitHub mirror: https://github.com/spade-lang/spade)
- **최신 버전:** 정확한 버전 번호 미확인 (GitLab 태그 19개 확인, 공식 사이트에 명시 없음)
- **최근 활동:** 커밋 1,460개, 2025년 8월 Frans Skarman의 PhD 논문("Improved Tooling for Digital Hardware Development: Spade, Surfer, and more") 발표. 2026년 1월 ACM TRETS 저널 논문 게재.

**프로젝트 상태:** 활발하나 규모가 작다. Rust + Clash에서 영감을 받은 HDL로, Rust와 유사한 타입 시스템(enum, sum type, immutable 변수)과 파이프라인 추상화를 제공한다. 뮌헨 응용과학대 AEMY 그룹이 유지관리하며 NLNet/NGI Zero 지원을 받는다.

**vitamin과의 관계:** spade는 HDL 컴파일러 구현 사례(Rust 코어). 동반 도구 Surfer(파형 뷰어, Rust 구현)도 생태계 맥락에서 참고할 만하다.

---

## vcd 크레이트 (Kevin Mehall)

- **현재 버전:** 0.7.0
- **마지막 업데이트:** 2023-07-15
- **라이선스:** MIT
- **저장소:** https://github.com/kevinmehall/rust-vcd
- **crates.io 총 다운로드:** 184,721회 (최근 33,217회)

**API 특징:** IEEE 1364 VCD 포맷 읽기/쓰기를 모두 지원. `Writer` 타입으로 헤더(`$date`/`$timescale`/`$var`/`$scope`)/초기 덤프/값 변화(`Value::V0`, `V1`, `X`, `Z`) 직렬화를 제공한다.

**유지보수 현황:** 마지막 배포가 2023-07-15로 약 2년 이상 업데이트 없음. 다운로드는 꾸준하지만 사실상 유지보수 정체(dormant) 상태로 볼 수 있다. `vcd-ng` 등 포크도 존재한다.

**vitamin 방침:** vcd 크레이트의 구현 방식을 참고하되, vitamin은 `vcd-writer` 크레이트를 자체 구현한다. 이 크레이트에서 IEEE 1364 VCD 직렬화 API 설계 선례를 학습하는 것이 주목적.

---

## logos 렉서

- **현재 버전:** 0.16.1 (2026-01-30 업데이트)
- **라이선스:** MIT OR Apache-2.0
- **저장소:** https://github.com/maciejhirsz/logos
- **crates.io 총 다운로드:** 53,889,579회 (최근 18,047,321회)

**특징:** 정규식 + 단순 매칭 기반 토큰 정의를 프로시저 매크로(`#[derive(Logos)]`)로 처리해 단일 결정적 상태 기계(DFA)로 컴파일한다. "수작업 렉서보다 빠른 생성 렉서"를 표방. 대규모 토큰 집합에도 성능이 일정하게 유지된다. Rust 생태계에서 컴파일러/언어 툴 분야에서 사실상의 표준 렉서 크레이트로 자리잡았다.

**버전 이력:** v0.15.1(2025-08-08) → v0.16.0(2025-12-07) → v0.16.1(2026-01-30)로 꾸준히 개발 중.

---

## chumsky vs. lalrpop — HDL 파서 후보 비교

### chumsky

- **현재 버전:** 0.13.0 (2026-05-06 업데이트)
- **GitHub 스타:** ~4,500 stars (저장소 Codeberg 이전: https://codeberg.org/zesterer/chumsky)
- **라이선스:** MIT
- **총 다운로드:** 21,461,093회

**특성:** 파서 콤비네이터 라이브러리. 모든 맥락 자유 문법(CFG)과 맥락 의존 문법 일부를 지원한다. 오류 복구(error recovery)를 1급으로 지원해 파서가 구문 오류를 만난 후 파싱을 계속할 수 있다. 재귀 하강(recursive descent) 방식 + opt-in 좌재귀/메모이제이션. Pratt 파서 지원으로 표현식 연산자 우선순위 처리가 편리하다. 진단 연동을 위한 `Label` 타입 제공.

**HDL/SV 문법 적합성:** SV 문법은 Annex A 기준으로 약 1,800개 이상의 규칙을 가진 대형 BNF다. 재귀적 계층 구조, 파라미터화 인스턴스, 표현식 우선순위 등이 복잡하다. chumsky의 콤비네이터 방식은 그 규모의 문법을 Rust 타입 시스템으로 점진적으로 쌓을 수 있다. 오류 복구가 내장돼 있어 여러 오류를 한 번에 보고하는 개발 UX에 유리하다.

### lalrpop

- **현재 버전:** 0.23.1 (2026-03-11 업데이트)
- **GitHub 스타:** ~3,500 stars
- **라이선스:** Apache-2.0 OR MIT
- **총 다운로드:** 55,731,021회

**특성:** LR(1) 파서 생성기. `.lalrpop` 파일에 BNF 유사 문법을 작성하면 Rust 코드를 생성한다. 타입 추론, 문법 매크로, 매개변수화된 서브셋 지원. LR(1) 기반이므로 직접 좌재귀를 허용한다(콤비네이터 방식은 기본 금지).

**HDL/SV 문법 적합성:** LR(1) 방식은 SV 문법의 일부 모호성(예: `[*]` 반복 표현)에서 충돌(conflict)을 일으킬 수 있어 문법 재작성이 필요하다. 또한 1,800여 규칙 크기의 문법 파일은 lalrpop의 컴파일 시간을 매우 길게 만들 수 있다(빌드 타임 코드 생성). 오류 복구 기능이 없어 첫 번째 오류에서 중단된다.

### 권장 선택

**vitamin 프로젝트 권장 파서: chumsky**.

SV 규모의 문법(복잡한 재귀 + 모호성 + 우선순위)을 다루고, 여러 오류를 한 번에 보고하는 진단 UX를 목표로 할 때 chumsky가 명확히 유리하다. lalrpop은 오류 복구 부재와 대형 문법에서의 컴파일 타임 코드 생성 비용이 단점이다. 단, chumsky v0.13은 API가 안정화 과정에 있으므로 API 변경에 대한 주의가 필요하다.

---

## ariadne vs. codespan-reporting — 진단 라이브러리

### ariadne

- **현재 버전:** 0.6.0 (2025-10-28 업데이트)
- **라이선스:** MIT
- **저장소:** https://github.com/zesterer/ariadne (Codeberg 이전: https://codeberg.org/zesterer/ariadne)
- **총 다운로드:** 7,928,325회
- **MSRV:** 1.85.0 (검색 결과에서 확인)

**특징:** 팬시한 터미널 진단 출력에 특화. 인라인·멀티라인 레이블, 색상 자동 생성, 소스 범위 시각화. chumsky와 같은 저자(zesterer)라 두 크레이트의 오류 타입 연동이 자연스럽다.

### codespan-reporting

- **현재 버전:** 0.13.1 (2025-10-22 업데이트)
- **라이선스:** Apache-2.0
- **저장소:** https://github.com/brendanzab/codespan
- **총 다운로드:** 109,053,827회

**특징:** 안정적이고 널리 쓰이는(1억+ 다운로드) 진단 크레이트. 여러 개의 note를 하나의 진단에 첨부 가능. ariadne보다 표현 자유도가 낮으나 성숙도와 커뮤니티 채택률이 높다.

**vitamin 방침:** `diag` 크레이트에서 두 후보를 모두 검토. chumsky를 파서로 채택할 경우 같은 저자의 ariadne 연동이 더 매끄럽다. 최종 선택은 `diag` 크레이트 설계 단계에서 결정한다.

---

## Rust로 작성된 RTL 시뮬레이터 선례

**직접 선례(event-driven simulator in Rust):** 2025년 9월 업데이트된 소형 SystemVerilog 시뮬레이션 툴이 GitHub에 존재하나 검색 결과에서 구체 저장소 URL을 특정하기 어려웠다(stars/author 미확인). 즉, "완전한 Rust RTL 시뮬레이터"는 아직 성숙한 선례가 없다.

**간접 선례(Rust 기반 HDL 프레임워크):**
- **rust-hdl** (samitbasu): 486 stars. Rust DSL로 FPGA 펌웨어를 기술하면 Verilog로 변환하고 내장 시뮬레이터를 제공. 주 저자가 `rhdl`(새 버전)으로 이전 중, 이 저장소는 eventually archive 예정.
- **rhdl** (samitbasu): rust-hdl의 완전 재작성. "rust-hdl 대비 1~2 오더 빠른 시뮬레이션 성능"을 목표로 함.
- **kaze**: Rust 임베디드 HDL + Rust 시뮬레이터 코드 생성기.

**결론:** "순수 Rust 이벤트 구동 SV 시뮬레이터"는 공백 지대다. vitamin이 개척하는 영역이다. rust-hdl/rhdl의 내장 시뮬레이터 방식과 veryl/spade의 프론트엔드 구조를 모두 참고하되, vitamin은 표준 시뮬레이션(VCD 덤프·timescale 정밀도)에 집중하는 점에서 차별된다.

---

## Sources

- crate: sv-parser — https://crates.io/crates/sv-parser (v0.13.5, 2026-03-30)
- repo: sv-parser — https://github.com/dalance/sv-parser (472 stars)
- crate: logos — https://crates.io/crates/logos (v0.16.1, 2026-01-30)
- repo: logos — https://github.com/maciejhirsz/logos
- crate: chumsky — https://crates.io/crates/chumsky (v0.13.0, 2026-05-06)
- repo: chumsky — https://codeberg.org/zesterer/chumsky (~4,500 stars)
- crate: lalrpop — https://crates.io/crates/lalrpop (v0.23.1, 2026-03-11)
- repo: lalrpop — https://github.com/lalrpop/lalrpop (~3,500 stars)
- crate: ariadne — https://crates.io/crates/ariadne (v0.6.0, 2025-10-28)
- repo: ariadne — https://github.com/zesterer/ariadne
- crate: codespan-reporting — https://crates.io/crates/codespan-reporting (v0.13.1, 2025-10-22)
- repo: codespan — https://github.com/brendanzab/codespan
- crate: vcd — https://crates.io/crates/vcd (v0.7.0, 2023-07-15)
- repo: rust-vcd — https://github.com/kevinmehall/rust-vcd
- crate: veryl — https://crates.io/crates/veryl (v0.20.0, 2026-05-01)
- repo: veryl — https://github.com/veryl-lang/veryl (942 stars)
- repo: spade — https://gitlab.com/spade-lang/spade (GitHub mirror 65 stars)
- site: spade — https://spade-lang.org/
- repo: rust-hdl — https://github.com/samitbasu/rust-hdl (486 stars)
- repo: rhdl — https://github.com/samitbasu/rhdl
- search: WebSearch round 1–2 queries (see frontmatter)
