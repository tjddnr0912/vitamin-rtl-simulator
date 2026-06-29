---
title: "Icarus Verilog vs Verilator 동작 차이 조사"
date: 2026-05-28
author: research-skill (Claude Sonnet 4.6)
scope:
  - IEEE 1800 지원 범위 비교
  - NBA 타이밍 · $display 순서 · x-전파 · 다중 구동 처리 차이
  - 헤드리스 배치 실행 커맨드라인 인보케이션
  - VCD 출력 옵션 및 quirk
  - 외부 테스트벤치 · 컴플라이언스 코퍼스 추천
rounds: 2
status: complete
---

# Icarus Verilog vs Verilator 동작 차이 조사

## 배경

Vitamin RTL 시뮬레이터의 차등검증(differential verification) 기반 구축을 위해
Icarus Verilog(iverilog/vvp)와 Verilator 간의 동작 차이, 커맨드라인 인보케이션,
VCD 출력 방식, 외부 코퍼스를 조사한다.

---

## 두 도구의 근본적 차이

### 시뮬레이션 모델

| 항목 | Icarus Verilog | Verilator |
|------|---------------|-----------|
| 시뮬레이션 모델 | 이벤트 기반 인터프리터 | C++ 트랜스파일 (cycle-accurate) |
| 상태 표현 | 4-state (0/1/X/Z) | 2-state (0/1) 기본 |
| 표준 준수 목표 | IEEE 1364 + 일부 IEEE 1800 | IEEE 1800 합성 가능 서브셋 |
| 성능 | 기준 | ~100× 빠름 (컴파일드 실행) |
| SV 컴플라이언스 (sv-tests) | 72.2% (1,166/1,614) | 95.3% (1,539/1,614) |

Icarus는 표준 이벤트 큐 기반 시뮬레이션 모델을 구현하며 합성 의미론이 아닌
시뮬레이션 의미론을 따른다. Verilator는 성능 우선 설계로 합성 의미론에 더
가까운 동작을 보이며, 일부 기능은 표준 컴플라이언스보다 성능을 위해 재정렬된다.
[출처: arxiv.org/html/2502.19348v1, WebFetch 검증 ✓]

---

## 주요 동작 차이

### 1. 4-state vs 2-state: x-전파(X-propagation)

**차이**: Icarus는 초기화되지 않은 신호를 X로 처리하고 이를 후속 논리로 전파한다.
Verilator는 기본적으로 X를 0으로 처리하며 침묵하게 숨긴다.

**구체 예시**:

```verilog
// 리셋 없는 레지스터
module test;
  reg [7:0] data;
  initial begin
    if (data == 8'hFF) $display("FAIL: uninitialized"); // Icarus: X != 8'hFF → 분기 안 함
    else $display("data = %h", data);                  // Verilator: 0 == 0 → "data = 00"
  end
endmodule
```

- Icarus: `data` = X, 비교 결과 = X → 분기 미실행, $display 출력 없음 또는 X 출력
- Verilator: `data` = 0 (묵시적 2-state 초기화) → "data = 00" 출력

**Verilator 완화 옵션**:
- `--x-assign unique` — X 대입을 랜덤 0/1로
- `--x-initial unique` — 초기화 미정 변수를 랜덤 seed 기반으로 설정
[출처: verilator.org/guide/latest/languages.html, WebFetch 검증 ✓; chipverify.com/rtl-synthesis/verilator-simulation]

---

### 2. NBA(Non-Blocking Assignment) 타이밍

**공통점**: Verilator 5.x (2022년) 이후 IEEE 호환 스케줄러가 추가되어
Active → NBA 영역 순서를 IEEE 1800 §4에 따라 구현한다.

**차이점**: Verilator는 성능을 위해 이벤트 실행 순서를 재정렬(reorder)할 수 있다.
NBA 큐 처리 방식은 동일하지만, combo 블록의 다중 평가가 발생할 수 있다.

```verilog
// NBA 기본 패턴
always @(posedge clk) begin
  a <= b;  // NBA: clk 상승 후 NBA 영역에서 커밋
  b <= a;  // 교환: 동일 클록에서 a, b 값 교환
end
// → 두 도구 모두 정확히 동작 (Verilator 5+ 이후)
```

이전 Verilator (4.x)는 delay를 무시하고 timing 제약을 건너뛰었으나, 5.x에서 수정됨.
[출처: Verilator Wikipedia + WebSearch Round 1]

---

### 3. $display 순서(ordering)

**차이**: Verilator는 combo 블록(always_comb / always @(*))에서 $display가
여러 번 실행될 수 있다. 이벤트 정렬이 표준에서 명시되지 않았기 때문이며,
Verilator는 성능 최적화를 위해 재정렬한다.

```verilog
// 문제 패턴
always @(*) begin
  y = a & b;
  $display("y=%b at %0t", y, $time);  // Verilator: 복수 호출 가능
end
```

Verilator 공식 문서에 따르면 combo always 블록에 `$display` 문을 두는 것을 권장하지 않는다. 표준에서 이벤트 정렬 순서를 명시하지 않아 의도와 관계없이 같은 시간에 여러 번 출력될 수 있으며, 이는 규격 준수 시뮬레이터에서도 발생한다.
[출처: verilator.org/guide/latest/languages.html, WebFetch 검증 ✓]

---

### 4. 다중 구동(Multi-driver) 처리

**차이**: Icarus는 Z 상태를 포함한 4-state 와이어 해소를 수행한다. Verilator는
Z를 0으로 처리하며, tri-state 버스 중재가 포함된 설계에서는 잘못된 결과를 낼 수 있다.

```verilog
// tri-state 버스 패턴
wire bus;
assign bus = (en1) ? data1 : 1'bz;
assign bus = (en2) ? data2 : 1'bz;
// Icarus: en1=0, en2=0 → bus=Z
// Verilator: Z를 0 처리 → bus=0 (잘못된 결과)
```

[출처: chipverify.com/rtl-synthesis/verilator-simulation, WebSearch Round 2]

---

### 5. IEEE 1800 지원 범위 (CHIPS Alliance sv-tests 기준)

| 기능 영역 | Icarus | Verilator |
|-----------|--------|-----------|
| 전체 통과율 | 72.2% | 95.3% |
| Associative Arrays (§7.8-7.9) | 0/9 | 9/9 |
| Array Manipulation (§7.12) | 0/10 | 10/10 |
| Queues (§7.10) | 2/13 | 13/13 |
| Class (§8) | 0–부분 | 41/41 |
| always_comb / always_ff | 지원 | 지원 |
| Interface / modport | 미지원(-g2012에서도 제한) | 지원 (일부 제한) |
| Assertions (simple) | 지원 | 변환 지원 |
| SEREs (복잡 assertion) | 미지원 | 미지원 |

[출처: chipsalliance.github.io/sv-tests-results/, WebFetch 검증 ✓]

---

## 헤드리스 배치 실행 커맨드라인

### Icarus Verilog (2단계)

```bash
# Step 1: 컴파일
iverilog -g2012 -o sim.vvp top.sv tb.sv

# Step 2: 실행 (VCD 출력은 RTL 내 $dumpfile/$dumpvars로 제어)
vvp sim.vvp

# 헤드리스 CI 권장 플래그
vvp -N sim.vvp          # $stop → exit code 1 (테스트 실패 감지)
vvp -n sim.vvp          # $stop → $finish (정상 종료)
vvp -q sim.vvp          # stdout 억제 (로그만)
```

**언어 버전 플래그**:
- `-g2005` — IEEE 1364-2005 (v0.9+ 기본값)
- `-g2009` — IEEE 1800-2009 (SystemVerilog 진입)
- `-g2012` / `-g2017` / `-g2023` — 최신 SV 표준

**출력 포맷 선택** (IVERILOG_DUMPER 환경변수 또는 vvp 플래그):
- 기본: VCD
- `-fst` → FST 포맷 (GTKWave 권장)
- `-none` → 파형 비활성화 (긴 시뮬레이션 속도 향상)

[출처: steveicarus.github.io/iverilog/usage/command_line_flags.html + vvp_flags.html, WebFetch 검증 ✓]

---

### Verilator (컴파일 + 실행)

```bash
# VCD 트레이싱 포함 헤드리스 바이너리 생성
verilator --binary --trace-vcd --build top.sv tb.sv -o sim_bin

# 실행
./sim_bin

# FST 포맷 (더 작은 크기)
verilator --binary --trace-fst --build top.sv tb.sv -o sim_bin_fst
./sim_bin_fst

# 트레이싱 없이 (성능 우선)
verilator --binary --build top.sv tb.sv -o sim_bin_notrace
```

**핵심 플래그**:
- `--binary` — 독립 실행 가능 바이너리 생성
- `--build` — GNU Make 자동 호출 포함
- `--trace-vcd` — VCD 출력 (기본 trace 플래그는 FST)
- `--trace-fst` — FST 출력 (더 효율적)
- `--trace-depth N` — 트레이싱 계층 깊이 제한
- `--x-assign unique` — x 대입 랜덤화
- `--x-initial unique` — 초기화 랜덤화
- `--timing` — delay 처리 활성화

[출처: verilator.org/guide/latest/verilating.html, WebFetch 검증 ✓]

---

## VCD 출력 quirk

| 항목 | Icarus VCD | Verilator VCD |
|------|-----------|---------------|
| 기본 포맷 | VCD (IEEE 1364 §18) | `--trace`만으로는 FST; `--trace-vcd` 필요 |
| 트리거 방식 | RTL 내 $dumpfile/$dumpvars 명시 필요 | C++ harness 또는 --main 연동 필요 |
| 식별자 코드 | 짧은 ASCII 코드 (!, ", # ...) | 동일 VCD 표준; scope 계층명 차이 있음 |
| Z 표현 | 'z' 리터럴 | z → 0 변환 (2-state 한계) |

**차등검증 시 주의**: Icarus VCD는 Z 값을 정확히 덤프하지만 Verilator VCD는
Z가 없으므로 diff 시 Z↔0 차이가 발생한다. diff 정규화 단계에서 Z=0 매핑 규칙이 필요하다.

---

## 추천 외부 테스트벤치 / 컴플라이언스 코퍼스

### 1. CHIPS Alliance sv-tests
- **URL**: https://github.com/chipsalliance/sv-tests
- **설명**: IEEE 1800 챕터별 최소 테스트케이스. iverilog/Verilator 포함 14개 도구 동시 비교.
- **결과 대시보드**: https://chipsalliance.github.io/sv-tests-results/
- **활용**: 언어 기능별 통과율 추적. 새 기능 구현 후 Vitamin 추가 가능.

### 2. OpenTitan (lowRISC)
- **URL**: https://github.com/lowrisc/opentitan
- **설명**: 상용급 open-source silicon Root of Trust. Verilator + FuseSoC + Bazel 기반 CI.
- **활용**: 복잡한 SoC 수준 RTL의 통합 테스트 참조.

### 3. RISC-V VeeR Core (Caliptra / CHIPS Alliance)
- **URL**: https://github.com/chipsalliance/caliptra-rtl
- **설명**: CI-driven 오픈소스 RISC-V 코어. Verilator 기반 RTL CI 파이프라인 실제 사례.
- **활용**: 파이프라인 프로세서 수준의 차등검증 참조.

### 4. steveicarus/iverilog test suite (내장)
- **URL**: https://github.com/steveicarus/iverilog/tree/master/testsuite
- **설명**: iverilog 자체 회귀 테스트. 1364-2005 핵심 케이스 다수 포함.
- **활용**: Phase 1 MVP 차등검증 골든 레퍼런스.

---

## 차등검증 시 알려진 함정 요약

| 함정 | Icarus 결과 | Verilator 결과 | 대응 |
|------|------------|---------------|------|
| 초기화 미정 레지스터 | X 전파 | 0 취급 (묵시) | `--x-initial unique` 사용 또는 비교 제외 |
| Z tri-state 버스 | Z 정확 표현 | Z→0 변환 | VCD diff 시 Z=0 정규화 |
| combo 블록 $display | 1회 실행 | 복수 실행 가능 | combo에 $display 금지 또는 비교 제외 |
| SV classes/queues | 미지원 | 지원 | Phase 2 이후 대상 |
| Z 식별자 코드 | ASCII 문자 | ASCII 문자 (scope명 다름) | 식별자→신호명 정규화 |

---

## Sources

1. **Verilator Input Languages 공식 문서** — https://verilator.org/guide/latest/languages.html (WebFetch 검증 ✓)
2. **Verilator Verilating 공식 문서** — https://verilator.org/guide/latest/verilating.html (WebFetch 검증 ✓)
3. **Icarus Verilog Command Line Flags** — https://steveicarus.github.io/iverilog/usage/command_line_flags.html (WebFetch 검증 ✓)
4. **Icarus VVP Flags** — https://steveicarus.github.io/iverilog/usage/vvp_flags.html (WebFetch 검증 ✓)
5. **CHIPS Alliance sv-tests** — https://github.com/chipsalliance/sv-tests
6. **sv-tests 결과 대시보드** — https://chipsalliance.github.io/sv-tests-results/ (WebFetch 검증 ✓)
7. **"The Simulation Semantics of Synthesisable Verilog"** (arxiv 2502.19348) — https://arxiv.org/html/2502.19348v1 (WebFetch 검증 ✓)
8. **ChipVerify — Verilator Simulation** — https://chipverify.com/rtl-synthesis/verilator-simulation
9. **Verilator Wikipedia** — https://en.wikipedia.org/wiki/Verilator
10. **OpenTitan FPGA adaptation (Antmicro)** — https://antmicro.com/blog/2023/03/adapting-opentitan-for-fpga-prototyping-and-tooling-development
11. **CHIPS Alliance Caliptra VeeR CI** — https://www.chipsalliance.org/news/open-source-rtl-ci-testing-and-verification-for-caliptra-veer/
