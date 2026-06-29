# 02 · 구현 언어 결정 (Rust)

> 설계 명세 §4 기반. 크레이트 생태계 정보는 `research-log/rust-hdl-ecosystem-2026-05-28.md` 참조.

---

## 결정

**Rust 채택.** 후보는 Rust / C++ / Go 세 가지였다.

---

## 후보 비교

| 후보 | 장점 | 단점 | 판정 |
|---|---|---|---|
| **Rust** | 메모리 안전 + C급 성능; `enum`/패턴매칭이 lexer·parser·elaborator·AST/IR 구현에 이상적; `cargo`로 3-OS 재현 빌드; GC 없어 이산사건 시뮬레이터의 결정론적 타이밍 정밀도 유지 가능 | 러닝커브; 컴파일 시간이 C++ 대비 길 수 있음 | **채택** |
| C / C++ | 검증된 EDA 경로 (Verilator=C++, Icarus=C/C++); 최고 이식성·생태계; 기존 EDA 인력 접근 용이 | 대규모 파서·시뮬레이터를 메모리 안전하게 유지·디버깅하는 부담; 크로스 플랫폼 빌드 마찰(헤더·링커 의존) | 차점 |
| Go | 단순한 언어 모델; 빠른 빌드; 쉬운 크로스컴파일 | GC 지연이 결정론적 타임스케일 정밀도·처리량에 불리; `sum type` 부재로 AST/패턴매칭 표현이 어색 | 코어 부적합 |

---

## 채택 근거

spec §4에서 합의된 근거 다섯 가지:

1. **결정론적 정밀도:** 시뮬레이터 코어는 타이트한 이벤트 루프다. GC 중단(pause)은 결정론적 타임스케일 정밀도와 처리량 모두에 부담이 된다.
2. **패턴매칭 적합성:** 3개 HDL 프론트엔드(lexer/parser/elaborator)는 대용량 BNF 문법을 다루는 영역이다. Rust의 `enum` + 패턴매칭이 AST 노드 정의와 순회에 가장 자연스럽게 맞는다.
3. **버그 = 조용한 오답:** 시뮬레이터 버그는 컴파일 에러가 아니라 잘못된 파형으로 나타난다. 메모리 안전성이 보장된 언어에서 버그 수색 범위가 크게 줄어든다.
4. **3-OS 소스 빌드:** `cargo`는 3-OS 소스 빌드를 1급으로 지원한다. C 라이브러리 의존 없이 순수 Rust 크레이트로 구성하면 크로스 플랫폼 빌드 마찰이 거의 없다.
5. **선례:** `veryl`, `spade`, `sv-parser` 등 Rust로 작성된 HDL/EDA 툴이 이미 운용 중이다.

---

## Rust 생태계 검토 (research 결과 반영)

아래 버전·URL은 2026-05-28 crates.io API 직접 조회 결과다.

### 렉서: `logos`

- **버전:** 0.16.1 (2026-01-30)
- **저장소:** https://github.com/maciejhirsz/logos
- **특징:** `#[derive(Logos)]` 매크로로 토큰 정의를 작성하면 단일 DFA로 컴파일해 수작업 렉서보다 빠른 처리량을 제공한다. 다운로드 5,389만 회 이상으로 Rust 컴파일러 툴 분야의 사실상 표준 렉서 크레이트다.
- **vitamin 적용:** `hdl-lexer` 크레이트에서 언어별(SV/Verilog/VHDL) 토큰 집합을 `logos` 기반으로 구현한다.

### 파서: `winnow` 부트스트랩 → `수작업 RD` 이관 (결정)

> **결정(2026-06-01):** **`winnow` 1.0.3으로 부트스트랩**한 뒤, hot·복구-임계 규칙을 점진적으로
> **수작업 재귀하강(hand-RD)**으로 이관한다. 진단 렌더러는 `miette`(아래 절). 이 결정은 02의 과거
> "chumsky 권장"을 폐기한다.

| 후보 | 방식 | 오류 복구 | SV 대형 문법 적합성 | 판정 |
|---|---|---|---|---|
| ~~chumsky~~ (0.13.0) | 파서 콤비네이터 | 내장 — 여러 오류 동시 보고 | API 진화 중 | **배제** — GitHub **archived 2026-04-02**, 1.0 미출시(1.0.0-alpha.8에서 동결). 최장수·최대 컴포넌트를 upstream-없는 크레이트에 하드의존 불가 |
| **winnow** (1.0.3, 2026-05-14) | 파서 콤비네이터 | `unstable-recover` 피처 게이트 | 1.0 안정 API(nom 후속); 규칙 점진 조합 | **부트스트랩 채택** (복구가 unstable 피처임을 인지) |
| **수작업 RD + Pratt** | recursive descent | per-rule 패닉모드·복구힌트 1급 (직접 구현) | 최고 유연성; SV context-sensitivity 직접 처리; 유지보수 비용 높음 | **점진 이관 목표** (slang·Verible·veryl 선례) |
| lalrpop (0.23.1) | LR(1) 생성기 | coarse — `!`-토큰 메커니즘(hand-RD보다 약함) | SV context-sensitivity 처리 불가; 1,800규칙 codegen bloat | 부적합 |

**근거.** SV 문법은 Annex A 기준 약 1,800개 이상의 규칙을 갖는 대형 BNF이며, type-vs-identifier·net-vs-variable 같은 **context-sensitive 모호성**에 lexer/symbol-table 피드백이 필요해 LR/PEG 생성기로 깨끗이 잡히지 않는다 — **모든 프로덕션 SV 프론트엔드(slang, Verible)가 수작업 재귀하강을 채택**한 이유다. 따라서 최종 지향은 hand-RD다. 다만 1,800규칙 전부를 처음부터 손으로 쓰는 부담을 줄이기 위해, 안정 1.0 콤비네이터인 **winnow로 골격을 부트스트랩**하고 복구 품질이 중요한 규칙(`;`/`end`/`endmodule` 동기화셋 기반 패닉모드)부터 hand-RD로 옮긴다. **chumsky 부활은 금지**(archived). winnow의 오류 복구는 `unstable-recover` 피처 게이트이므로(파싱 API만 1.0 안정), 복구-임계 경로를 hand-RD로 이관하는 것이 이 게이트 의존을 제거하는 경로이기도 하다. 13(진단)의 `--error-limit`(기본 50)·09 corpus의 "한 런에서 다수 MsgCode assert"는 파서가 첫 오류에서 중단하지 않고 복구·계속해야만 성립한다.

### 진단: `miette` (결정) — `codespan-reporting` fallback

- **miette** 7.6.0 (2025-04-27, Apache-2.0) — **채택.** `code()`/`url()`/`related()`를 네이티브로 제공해 에러 카탈로그(15: 본문 full-entry **55개**(`MsgCode` enum 등재, 2026-06-12 v7 기준) + 부록 A 예약 107)·multi-span Frame(13)에 1:1 매핑된다. 13의 진단 데이터 모델이 이미 miette 어휘(`code()`/`url()`/`related()`)로 설계돼 있다. **MSRV 1.82**(manifest 실측 `rust-version = "1.82.0"`; crates.io에 노출된 1.70은 stale). leaf `diag` 크레이트는 IO/터미널 순수성(04: "IO 없음 → leaf")을 위해 **`default-features = false` 필수** — 기본 `fancy` 피처가 `owo-colors`/`supports-color`/`terminal_size`를 끌어들이기 때문이다. 터미널 렌더링(`fancy`)은 `vita-log` 크레이트에서만 활성화한다.
- **codespan-reporting** 0.13.1 (2025-10-22, Apache-2.0) — **fallback.** 성숙도 높고(1억+ 다운로드, MSRV 1.67) multi-span label+note 지원. miette의 dep-tree/바이너리 footprint가 installed-binary 크기 기준으로 블로킹이면 `code()`/`url()`/`explain` glue만 재구현해 스왑한다. `MsgCode`/`Diagnostic`/`Frame` 모델은 owner 소유 leaf `diag`에 있어 렌더 백엔드는 교체 가능하다.
- **ariadne** 0.6.0 (2025-10-28, MIT) — **미채택.** 유일 강점이던 "chumsky 동저자 시너지"가 chumsky archived로 소멸했고, `related()` 부재로 multi-span 카탈로그에 덜 맞는다. (참고: 02 이전 판의 "ariadne MSRV 1.85"는 manifest에 `rust-version` 필드 미선언 — "검증됨" 표기를 철회한다.)

### SystemVerilog 파싱 선례: `sv-parser`

- **버전:** 0.13.5 (2026-03-30, MIT OR Apache-2.0)
- **저장소:** https://github.com/dalance/sv-parser (472 stars)
- **특징:** IEEE 1800-2017 완전 준수 표방. Annex A 문법 규칙 이름과 1:1 대응하는 CST(구체 구문 트리) 반환. `svlint` 등 다운스트림 도구에서 사용 중.
- **활용:** vitamin은 sv-parser를 직접 의존성으로 쓰지 않는다(자체 파이프라인 구축). 그러나 SV 문법 규칙 명세 대조 시 이 크레이트의 규칙 이름 매핑을 참고 자료로 활용한다.

### 신규 HDL 선례: `veryl`, `spade`

- **veryl** 0.20.0 (2026-05-01, MIT OR Apache-2.0): 942 stars. SV를 transpile 대상으로 삼는 현대적 HDL. Rust로 전체 파이프라인(lexer/parser/elaborator/codegen) 구현. 프론트엔드 아키텍처 참고.
- **spade**: Rust + Clash 영감의 HDL. GitLab 주 저장소(GitHub mirror 65 stars). Rust 타입 시스템 기반 HDL 컴파일러 구현 사례. Surfer 파형 뷰어(Rust)도 동반 개발 중.

### VCD 참고: `vcd` 크레이트

- **버전:** 0.7.0 (MIT, 마지막 업데이트 2023-07-15)
- **저장소:** https://github.com/kevinmehall/rust-vcd
- **특징:** IEEE 1364 VCD 읽기/쓰기 API 제공. `Writer` 타입으로 헤더·`$var`·값 변화(`0/1/X/Z`) 직렬화를 지원한다.
- **vitamin 방침:** 직접 채택하지 않는다. 2023년 이후 업데이트가 없어 유지보수 정체 상태이므로, vitamin은 `vcd-writer` 크레이트를 자체 구현한다. API 설계 선례로 참고.

---

## MSRV / Toolchain

- **계획 MSRV:** Rust **1.82** (채택한 miette 7.6.0의 manifest MSRV가 `1.82.0`이라 이 상한으로 결정). Rust 1.80이 안정화한 `std::sync::LazyLock`을 포함한다(참고: `OnceLock`은 1.70, `let-else`는 1.65). 실제 기능 사용 패턴에 따라 추가 상향 가능하며, `rust-toolchain.toml`에 명시한다.
- **`rust-toolchain.toml` 사용:** 저장소 루트에 고정해 `cargo build`/`cargo test` 실행 시 자동으로 toolchain을 맞춘다. CI 및 로컬 개발 환경이 동일한 Rust 버전을 사용하도록 보장한다.
- **MSRV 상한 근거:** 채택 크레이트 MSRV 집합의 상한 = miette 7.6.0(1.82). winnow 1.0.3·logos 0.16.1(1.80)·codespan 0.13.1(1.67)은 모두 1.82 이하라 제약이 되지 않는다. 렌더 백엔드를 codespan으로 스왑하면 1.67까지 내려갈 수 있다. (이전 판의 "ariadne 1.85"는 manifest 미선언이며 ariadne 미채택으로 무관.)

---

## Sources

- 본 spec §4 — `/docs/superpowers/specs/2026-05-26-vitamin-rtl-simulator-design.md`
- research-log: `research-log/rust-hdl-ecosystem-2026-05-28.md`
- crates.io API: https://crates.io/api/v1/crates/{sv-parser,logos,winnow,miette,codespan-reporting,lalrpop,vcd,veryl} (2026-06-01 재검증)
- GitHub: https://github.com/dalance/sv-parser, https://github.com/maciejhirsz/logos
- GitHub: https://github.com/winnow-rs/winnow, https://github.com/zkat/miette
- GitHub: https://github.com/brendanzab/codespan, https://github.com/kevinmehall/rust-vcd, https://github.com/lalrpop/lalrpop
- 배제 참고: chumsky(https://github.com/zesterer/chumsky — GitHub archived 2026-04-02), ariadne(https://github.com/zesterer/ariadne — 미채택)
- GitHub: https://github.com/veryl-lang/veryl, https://gitlab.com/spade-lang/spade
