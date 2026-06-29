# 09 · 테스트 · 검증 전략

Vitamin 시뮬레이터의 정확성을 보장하는 4계층 검증 전략을 기술한다.
각 계층은 독립적으로 실행 가능하며, CI 파이프라인에서 순차적으로 또는
병렬로 수행된다.

---

## 검증 계층

```
┌──────────────────────────────────────────────────────┐
│  4. 컴플라이언스 코퍼스  ← 언어 기능별 + system tasks    │
│  3. 차등검증            ← Icarus / Verilator 교차 비교  │
│  2. 통합 테스트          ← RTL → 시뮬 → VCD 엔드투엔드   │
│  1. 단위 테스트          ← 크레이트별 (lexer/parser/...) │
└──────────────────────────────────────────────────────┘
```

1. **단위 테스트** — 크레이트별 격리 테스트 (lexer / parser / elaborate /
   sim-ir / sim-engine / hdl-builtins / vcd-writer / diag)
2. **통합 테스트** — 작은 RTL 입력 → 시뮬레이션 → VCD 파일 생성 → 내용 비교
3. **차등검증** — 동일 입력을 Icarus / Verilator와 동시에 실행하여
   신호값 · 천이 시각을 비교
4. **컴플라이언스 코퍼스** — 언어 기능별 + system tasks 범주별 최소 재현 케이스

---

## 단위 테스트 정책

### TDD 지향

모든 크레이트는 기능 구현 전에 테스트를 먼저 작성하는 것을 원칙으로 한다.
버그 수정 시에는 재현 테스트를 먼저 추가한 뒤 수정한다.

### 크레이트별 `tests/` 디렉터리

```
crates/
├── hdl-lexer/tests/      # 토큰 분류, 키워드, 식별자, 숫자 리터럴
├── hdl-parser/tests/     # 문법 규칙, 오류 복구, AST 구조
├── elaborate/tests/      # 파라미터 해소, 포트 정합, 다중구동 검사
├── sim-ir/tests/         # IR 구조 불변식, 직렬화
├── sim-engine/tests/     # 이벤트 큐, delta cycle, NBA 순서
├── hdl-builtins/tests/   # $display, $monitor, $dumpfile 등
├── vcd-writer/tests/     # VCD 헤더, 식별자 코드, 천이 기록
└── diag/tests/           # 오류 메시지 포맷, 소스 위치 정확도
```

### 커버리지 목표

| 크레이트 | 라인 커버리지 목표 |
|----------|-----------------|
| hdl-lexer | 95% 이상 |
| hdl-parser | 95% 이상 |
| elaborate | 90% 이상 |
| sim-engine (핵심 경로) | 90% 이상 |
| hdl-builtins | 85% 이상 |
| vcd-writer | 90% 이상 |

커버리지 측정: `cargo llvm-cov` 또는 `cargo tarpaulin`.

---

## 차등검증 워크플로우

같은 RTL 입력과 테스트벤치를 세 도구에 동시에 투입하여 VCD 출력을 비교한다.
차이가 발견될 경우 표준 위반인지 도구 버그인지 판별하는 것이 핵심이다.

### 단계

```
1. 준비: RTL 소스 + 테스트벤치 작성 (tools/corpus/ 또는 tests/corpus/)

2. 세 도구 동시 실행
   vita top.sv tb.sv -o out/vita.vcd          # 원샷: compile→elab→sim 일괄
   iverilog -g2012 -o /tmp/sim.vvp top.sv tb.sv && vvp -N /tmp/sim.vvp
   verilator --binary --trace-vcd --build top.sv tb.sv -o /tmp/sim_vlt && /tmp/sim_vlt

3. VCD 추출
   출력 파일: out/vita.vcd, out/iverilog.vcd, out/verilator.vcd

4. 정규화 diff 도구로 비교
   cargo run --bin vcd-diff -- out/iverilog.vcd out/vita.vcd
   cargo run --bin vcd-diff -- out/verilator.vcd out/vita.vcd

5. 차이 판별
   - Icarus와 Vitamin이 일치 → Verilator quirk(2-state 차이) 여부 확인
   - Verilator와 Vitamin이 일치 → Icarus quirk 여부 확인
   - 셋 모두 일치 → PASS
   - Vitamin만 다름 → Vitamin 버그 수정 대상
```

### 알려진 동작 차이 (차등검증 판별 기준)

차등검증에서 아래 차이는 예상된 것이며, 표준 위반이 아니다.

**① x-전파 (X-propagation)**

Icarus는 초기화되지 않은 신호를 X로 처리하고 논리 전파한다. Verilator는
기본적으로 X를 0으로 처리한다. 이는 Verilator의 2-state 설계 한계이며
표준(IEEE 1800 §4)은 미초기화 값을 X로 정의한다.

```verilog
// 리셋 없는 플립플롭
reg [7:0] data;
initial $display("data = %h", data);
// Icarus:    data = xx   (X 전파)
// Verilator: data = 00   (0으로 묵시 처리)
// Vitamin:   data = xx   (Icarus 동작 기준 = 표준 준수)
```

Verilator에서 `--x-initial unique` 플래그를 사용하면 랜덤 초기화로
이 차이를 일부 노출할 수 있다.
[출처: verilator.org/guide/latest/languages.html — WebFetch 검증]

**② combo 블록 내 $display 복수 실행**

Verilator는 성능을 위해 조합 논리 블록을 복수 평가할 수 있으므로,
`always @(*)` 또는 `always_comb` 내의 `$display`가 동일 시각에
여러 번 출력될 수 있다. 표준은 이벤트 정렬 순서를 완전히 명시하지 않으므로
이는 규격 위반이 아니다.

```verilog
always @(*) begin
  y = a & b;
  $display("y=%b at %0t", y, $time);
  // Icarus:    1회 출력
  // Verilator: 복수 출력 가능
  // 판별 기준: 타임스텝당 마지막 출력값만 비교
end
```

[출처: verilator.org/guide/latest/languages.html — WebFetch 검증]

**③ tri-state / Z 처리**

Icarus는 Z(고임피던스) 상태를 VCD에 정확히 기록한다. Verilator는 Z를 0으로
변환하므로 VCD에 Z가 나타나지 않는다.

```verilog
wire bus;
assign bus = (en) ? data : 1'bz;
// Icarus:    en=0 → bus=z
// Verilator: en=0 → bus=0
// diff 정규화: Z→0 매핑 규칙 적용
```

표준 준수 기준은 Icarus 동작이다.

---

## VCD Golden Diff 도구

`crates/vcd-diff` — Vitamin 내부 VCD 비교 도구. `corpus-runner`(`crates/corpus-runner`)와 함께 **dev/test 전용 바이너리 크레이트**로 워크스페이스에 둔다(`publish = false`, 배포 multicall·설치 대상에 미포함 — 03 참조). 프로덕션 설치 바이너리는 `cli`(`vita`/`vcmp`/`velab`/`vrun`) 하나뿐이다.

### 정규화 규칙

1. **식별자 코드 매핑** — 도구별로 식별자 코드가 다르므로 신호명(계층 경로 + 이름)
   기반으로 재매핑한다.
2. **공백·주석 무시** — VCD 헤더의 `$comment` 블록, 공백 행, 날짜 타임스탬프 무시.
3. **동일 시각 · 동일 신호 값만 비교** — 이벤트 기록 순서가 아닌 `(시각, 신호경로, 값)` 트리플로 정규화.
4. **Z→0 정규화 옵션** — Verilator와 비교 시 Z값을 0으로 치환하는 모드 제공.
5. **scope 계층명 차이 흡수** — 최상위 모듈명이 다를 경우 깊이 기준 상대 경로로 매핑.

### diff 출력 포맷

```
DIFF: t=1050 signal=tb.dut.q
  expected (iverilog): 8'hFF
  actual   (vita):     8'h00
  → Vitamin 버그 의심: sim-engine NBA 처리 확인
```

```
QUIRK (expected): t=500 signal=tb.bus
  iverilog: z
  verilator: 0
  → Z/2-state 차이. 정규화 옵션 --normalize-z 권장
```

---

## 컴플라이언스 코퍼스 구조

```
tests/corpus/
├── verilog-2005/                   # IEEE 1364-2005 핵심 케이스
│   ├── blocking-assign/
│   ├── nba-swap/
│   ├── timescale-basic/
│   ├── always-posedge/
│   └── continuous-assign/
├── sv-extensions/                  # IEEE 1800 서브셋 (Phase 2~)
│   ├── always-comb/
│   ├── always-ff/
│   ├── logic-type/
│   └── package-basic/
└── system-tasks/                   # $-system task 범주별
    ├── display-format/             # %b, %h, %d, %s
    ├── monitor/
    ├── dumpfile-dumpvars/
    ├── finish-stop/
    └── time-realtime/
```

각 케이스 구성:

```
tests/corpus/<category>/<name>/
├── <name>.sv        # RTL + 테스트벤치 (self-checking preferred)
├── <name>.golden.vcd  # 골든 VCD (iverilog 기준으로 생성)
└── <name>.meta.toml   # 메타데이터 (기능 태그, 알려진 quirk, 제외 도구)
```

`<name>.meta.toml` 예시:

```toml
[test]
name = "nba-swap"
tags = ["nba", "posedge", "flip-flop"]
golden_tool = "iverilog"
known_quirks = ["verilator-x-propagation"]  # 이 테스트는 Verilator와 Z/X 차이 허용
expect_codes = ["E-ELAB-MULTIDRIVER"]        # (선택) 발화할 진단 메시지 코드 — 텍스트가 아닌 코드로 assert
```

### 메시지 코드 · exit 코드 기반 판정

깨지기 쉬운 메시지 *텍스트* 대신 **안정 메시지 코드**로 assert한다. corpus 케이스는
`expect_codes`로 특정 진단(예: `E-ELAB-MULTIDRIVER`)이 발화했는지 검증한다 — 메시지 문구가
바뀌어도 테스트가 깨지지 않는다(코드 체계는 [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md),
각 코드의 원인·예시·해결은 [15-error-code-reference.md](15-error-code-reference.md)).

corpus-runner는 **exit 코드로 결과를 분류**한다: `0`=PASS, `1`=RTL/설계 FAIL, `2`=staleness/
artifact 게이트(STALE — vcmp/velab 재실행 필요), `3`=CLI/usage, `101`=내부 panic(CRASH).
**`101`(vitamin 자체 crash)은 절대 RTL FAIL로 점수 매기지 않는다.** exit `1` 안에서 compile
실패 vs runtime 실패 구분이 필요하면 always-logged 요약의 메시지 코드를 본다(exit code 단독
grep 금지).

**`$error` 처리(도구 기본 = IEEE-strict, corpus = strict).** 도구 기본은 IEEE를 따라 `$error`만
으로는 종료·exit에 영향을 주지 않는다(iverilog/historical-VCS와 동일 → 차등검증 calibration
보존). 그러나 **corpus-runner는 `--error-exit`를 기본 ON**으로 실행해, self-checking이 `$error`만
쓰는 케이스도 "에러 1개 = FAIL"로 엄격히 게이트한다. 즉 도구 동작은 표준 준수, CI 정책만
strict — 두 요구를 분리해 만족한다(13-diagnostics-and-logging.md `--error-exit`).

---

## 추천 외부 테스트벤치

### 1. CHIPS Alliance sv-tests

- **저장소**: https://github.com/chipsalliance/sv-tests
- **규모**: 1,600+ 케이스, IEEE 1800 챕터별 최소 재현
- **활용**: 언어 기능별 Vitamin 통과율 추적. iverilog(72.2%) / Verilator(95.3%)
  대비 Vitamin 위치 파악.
- **대시보드**: https://chipsalliance.github.io/sv-tests-results/

### 2. steveicarus/iverilog 자체 testsuite

- **저장소**: https://github.com/steveicarus/iverilog/tree/master/testsuite
- **규모**: iverilog 회귀 테스트 전체. IEEE 1364-2005 핵심 케이스 다수.
- **활용**: Phase 1 MVP 골든 레퍼런스. iverilog와 동일 입력 → 동일 출력 검증.

### 3. Caliptra RISC-V VeeR (CHIPS Alliance)

- **저장소**: https://github.com/chipsalliance/caliptra-rtl
- **설명**: CI-driven 오픈소스 RISC-V 코어. 프로세서 수준의 실전 RTL.
- **활용**: 파이프라인 프로세서 수준의 대규모 차등검증 참조.

### 4. OpenTitan (lowRISC)

- **저장소**: https://github.com/lowrisc/opentitan
- **설명**: Verilator + FuseSoC + Bazel 기반 상용급 오픈소스 SoC.
- **활용**: SoC 수준 통합 테스트 참조 (Phase 3+ 목표).

---

## CI 통합

### 빌드 매트릭스

> **구현됨(2026-06-10):** 모노레포 루트 `.github/workflows/vitamin-ci.yml` —
> `016_claude_rtl/**` paths 필터, **ubuntu-latest / macos-14 / windows-latest**
> 3-OS 매트릭스(결정성 계약과 일치), toolchain은 `rust-toolchain.toml`(MSRV 1.82
> 핀)을 rustup이 자동 인식(stable/beta 매트릭스는 핀과 충돌이라 비채택), 게이트 =
> `fmt --check` → `clippy -D warnings` → `test --workspace --locked`. ubuntu에만
> iverilog를 설치해 차분 스위트가 라이브 오라클로 돌고, 부재 OS에선 설계대로
> graceful skip. **골든 해시 핀이 in-repo 테스트라 3-OS 바이트 동일성은 아티팩트
> 교환 없이 OS별 자체 검증으로 강제된다.**

```yaml
# 초기 스케치(역사 보존; 실물은 위 워크플로)
matrix:
  os: [ubuntu-22.04, ubuntu-24.04, macos-14]
  rust: [stable, beta]
```

### 단계별 게이트

```
cargo test --workspace          # 단위 + 통합 테스트
cargo run --bin corpus-runner   # corpus 전체 실행 → PASS/FAIL 집계
cargo run --bin vcd-diff -- ...  # 차등검증 diff
```

### VCD diff PR 게이트

- corpus runner가 `tests/corpus/` 전체를 실행하고 각 케이스별 PASS/FAIL을 집계한다.
- 실패 케이스가 있으면 PR merge를 블록한다.
- 알려진 quirk (`meta.toml`의 `known_quirks`)는 WARN으로 처리하며 블록하지 않는다.

```
[PASS] 42/42 verilog-2005 corpus
[PASS] 18/20 sv-extensions corpus (2 WARN: known quirks)
[FAIL] 3/15 system-tasks corpus → PR blocked
```

---

## Sources

- 본 spec §8 (시뮬레이션 엔진 설계)
- research-log: [`iverilog-verilator-behaviors-2026-05-28.md`](../research-log/iverilog-verilator-behaviors-2026-05-28.md)
- Icarus Verilog 공식 문서: https://steveicarus.github.io/iverilog/usage/index.html
- Verilator 공식 문서: https://verilator.org/guide/latest/
- CHIPS Alliance sv-tests: https://github.com/chipsalliance/sv-tests
- sv-tests 결과 대시보드: https://chipsalliance.github.io/sv-tests-results/
