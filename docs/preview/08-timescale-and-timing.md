# 08 · Timescale · 정밀 시간

## 왜 정밀도가 핵심인가

시뮬레이터 버그는 조용한 오답을 만든다. 신호 천이 시각이 1 precision unit만 어긋나도 setup/hold time 위반 검출이 빠지거나, 두 이벤트의 선후가 뒤바뀌어 설계 오류가 통과될 수 있다. 파형 덤프(VCD)에는 틀린 시각이 기록되고, 검증 엔지니어는 올바른 회로를 디버깅하느라 시간을 낭비한다.

따라서 시뮬레이터가 `timescale`을 정확히 구현했는지는 단순 기능 요구가 아니라 **시뮬레이터 신뢰성의 기준**이다.

---

## `` `timescale unit/precision ``

### 문법

```verilog
`timescale <time_unit>/<time_precision>
```

두 인수 모두 `{1|10|100}<단위>` 형식이어야 한다. 허용값:

| 필드 | 허용 숫자 | 허용 단위 |
|------|-----------|-----------|
| time_unit | 1, 10, 100 | s, ms, us, ns, ps, fs |
| time_precision | 1, 10, 100 | s, ms, us, ns, ps, fs |

제약:
- `time_precision ≤ time_unit` — precision이 unit보다 클 수 없다.
- `timescale`은 컴파일러 디렉티브이므로 모듈 정의 바깥, 파일 최상단에 위치한다.

예시:
```verilog
`timescale 1ns/1ps     // 1ns 단위, 1ps 정밀도 — 일반 RTL 검증에 흔한 조합
`timescale 10ns/100ps  // 10ns 단위, 100ps 정밀도
`timescale 1us/1ns     // 느린 아날로그 인터페이스 모델
```

### 모듈별 적용 vs 컴파일 단위

`timescale`은 **파일 내 이후에 오는 모든 모듈**에 적용된다. 다음 `timescale` 선언이 나올 때까지 유지된다. 이 파일-순서 의존성은 복잡한 설계에서 함정이 된다:

```
파일 A: `timescale 1ns/1ps
         module fast_logic ... endmodule

파일 B: module no_timescale_here ... endmodule
          ↑ 파일 B를 파일 A 뒤에 컴파일하면 1ns/1ps 상속
          ↑ 파일 B를 단독으로 컴파일하면 툴 default 받음
          → 컴파일 순서 바뀌면 결과가 달라지는 고전 버그
```

**vitamin 기저값 (no-timescale base — 잠금).** 어떤 모듈도 `` `timescale ``/`timeunit`을 선언하지 않고 `--timescale` 플래그도 없으면, 전역 시간 단위/정밀도의 기저값을 **`1ns/1ns`** 로 적용하도록 잠그고(동작 규칙 확정), 구현은 **`W-PP-TIMESCALE-DEFAULT`**(W1017, §15 부록 A 인벤토리 — 구현 시 본문+enum 승격) 경고를 발화한다. 이 기저값이 §14의 `global_time_precision = min(소비단위 유효 precision)` 계산에서 **공집합 min()을 정의**한다(빈 집합 → 기저 정밀도 `1ns`). 기저값은 OS/컴파일 순서와 무관한 상수이므로 "3-OS 동일 결과"·velab 합성 해시(RULE T)의 결정성을 깨지 않는다. 일부 모듈만 `` `timescale ``을 가진 **부분 지정**은 별개 조건으로, 기본 정책 lenient(`W-PARSE-TIMESCALE-PARTIAL`)·strict 선택(`E-PP-TIMESCALE-PARTIAL`)으로 처리한다(§15, `--timescale-policy`).

SystemVerilog는 이를 해결하는 **모듈 내 선언** 방식을 IEEE 1800에 추가했다:

```systemverilog
module precise_block;
  timeunit 1ns;
  timeprecision 1ps;
  // 이 모듈은 컴파일 순서와 무관하게 항상 1ns/1ps
  // ...
endmodule
```

규칙 (IEEE 1800-2012): 모듈, program, package, interface, $root 각각 최대 하나의 `timeunit`/`timeprecision`. 반드시 해당 스코프의 다른 아이템보다 먼저 선언.

---

## 64-bit 정수 시간 모델

### 전역 simulation tick

혼합 timescale 설계에서 시뮬레이터는 **전체 설계에서 가장 작은(finest) precision**을 1 simulation tick으로 사용한다.

```
설계 내 모든 `timescale` / timeunit 선언에서:
  global_precision = min(precision_1, precision_2, ..., precision_n)
```

모든 시간 값은 이 global_precision 단위의 64-bit 정수로 저장된다. 부동소수는 사용하지 않는다.

### 왜 정수인가

부동소수(float/double)를 누적하면 표현 오차가 쌓인다. 0.1 같은 소수는 IEEE 754 binary64에서 정확히 표현되지 않는다. 수백만 번의 시각 진전 후 마지막 유효 비트가 소실되고 시각이 조용히 어긋난다.

역사적 사례 — Patriot 방공 시스템(1991, 걸프전): 0.1초씩 float로 누적 → 100시간 후 0.34초 오차 → 표적 추적 실패 → 28명 사망. 시뮬레이터 버그가 무기 시스템만큼 치명적이지 않더라도, **동일한 수학적 원리로 조용한 오답**을 만든다.

64-bit 정수는 이 문제가 없다. 2^64 − 1 tick까지 정확히 표현하며, 10ps precision으로 잡으면 최대 표현 가능 시간 ≈ 184,467초 — 어떤 RTL 시뮬레이션에도 충분하다.

### 모듈별 unit/precision → 전역 tick 환산

각 모듈의 `#delay`는 다음 두 단계로 정수 tick으로 변환된다:

> **구현 위치(format_version 4, 2026-06-10):** 이 환산은 **엔진이 suspension-time에** 수행한다 —
> `Terminator::Delay.amount`는 모듈 단위의 RAW 값을 평가하는 ExprId이므로 `#5` 같은 상수와
> `#d`/`#(d*2)`/`#r` 같은 **런타임 식이 한 경로**를 공유한다(상수는 Const 노드로 폴딩될 뿐).
> X/Z delay는 0 tick(iverilog parity), 정수는 u64-saturating × M, real은 아래 round 규칙 그대로.

**1단계: 모듈 precision으로 반올림**
```
반올림 규칙: fractional_part >= 0.5 → 올림, < 0.5 → 내림
```

**2단계: global_precision으로 스케일**
```
ticks = rounded_value_in_module_unit × (module_unit / global_precision)
```

### 구체 계산 예시

```
// ModuleA: `timescale 1ns/100ps
//   time_unit = 1ns = 1000ps, time_precision = 100ps
// ModuleB: `timescale 1us/10ns
//   time_unit = 1us = 1,000,000ps, time_precision = 10ns = 10,000ps
// global_precision = min(100ps, 10,000ps) = 100ps

// ModuleA에서 #1.55 처리:
//   1단계: 1.55ns → 1.6ns (100ps precision으로 반올림: 1600ps → 가장 가까운 100ps = 1600ps)
//         정확히: 1.55 × 10 = 15.5 → 반올림 16 → 1600ps
//   2단계: 1600ps / 100ps = 16 ticks

// ModuleA에서 #0.49 처리:
//   1단계: 0.49ns → 0.5ns? 아니, 정밀도 기준:
//         0.49 × 10 = 4.9 → 반올림 5 → 500ps
//   2단계: 500ps / 100ps = 5 ticks

// ModuleA에서 #0.044 처리 (precision보다 작은 경우):
//   1단계: 0.044 × 10 = 0.44 → 반올림 0 → 0ps
//   결과: 0 ticks — 이 delay는 아무 시간도 소비하지 않음 (advance 없음)
//   → VCD에 기록되지 않고 동일 time-step에서 처리됨

// ModuleB에서 #3.4 처리:
//   1단계: 3.4us → 3.4 × 100 = 340 → 반올림 340 → 3,400,000ps
//   2단계: 3,400,000ps / 100ps = 34,000 ticks
```

반올림 규칙 요약: precision 단위의 배수 중 가장 가까운 값으로. 정확히 중간(0.5)이면 올림.

---

## $time vs $realtime

두 시스템 함수 모두 현재 simulation 시각을 반환하지만, 반환 타입과 정밀도 처리가 다르다.

| | `$time` | `$realtime` |
|---|---------|-------------|
| 반환 타입 | 64-bit 정수 | `real` (64-bit IEEE 754 double) |
| 단위 | 호출 모듈의 `time_unit` | 호출 모듈의 `time_unit` |
| Precision 반영 | 아니오 — `time_unit` 미만 절사 | 예 — 소수점으로 표시 |
| 용도 | 이벤트 시각 비교, 조건 분기 | 파형 출력, 사람이 읽는 로그 |

```systemverilog
// `timescale 1ns/100ps 모듈에서
// 현재 내부 시각 = 2500ps (global tick)

$display($time);      // 출력: 2  (2ns 정수, 500ps 절사)
$display($realtime);  // 출력: 2.5 (precision 반영한 소수)
```

`$stime`(IEEE: `$time`의 하위 32비트)도 표준에 있으나 **vitamin 미구현** — 호출 시 `VITA-E3009`.
하위 32비트가 필요하면 `$time & 32'hFFFF_FFFF`.

본 프로젝트 구현 방침:
- 내부 시간 레지스터는 `u64` (전역 tick)
- `$time` 구현: `current_tick / (time_unit / global_precision)` 의 정수 부분
- `$realtime` 구현: `current_tick as f64 * global_precision_in_ns / time_unit_in_ns`
- VCD 타임스탬프 출력: 전역 tick 그대로 (`$timescale` 헤더에 global_precision 기록)

---

## 정밀도 회귀 테스트

서로 다른 timescale 모듈이 혼재하는 설계에서 천이 시각이 1 precision unit까지 Icarus Verilog와 일치하는지 확인한다.

### 테스트 케이스 1 — 기본 반올림

```verilog
`timescale 1ns/100ps
module test_round;
  reg a;
  initial begin
    a = 0;
    #1.44 a = 1;  // 1.44ns → 1400ps → 1400ps  (round: 14.4→14)
    #0.05 a = 0;  // 0.05ns →   50ps →   100ps  (round: 0.5→1)
    #0.04 a = 1;  // 0.04ns →   40ps →     0ps  (round: 0.4→0, advance 없음)
    $finish;
  end
endmodule
// 기대 VCD: a=1 at 1400ps, a=0 at 1500ps, a=1 at 1500ps (동일 tick)
```

### 테스트 케이스 2 — 혼합 timescale

```verilog
`timescale 1ns/100ps
module fast_mod(output reg q);
  initial #2.5 q = 1;  // 2500ps
endmodule

`timescale 1us/10ns
module slow_mod(output reg r);
  initial #1 r = 1;    // 1us = 1,000,000ps → 1,000,000ps
endmodule

`timescale 1ns/100ps
module tb;
  wire q, r;
  fast_mod u_fast(.q(q));
  slow_mod u_slow(.r(r));
  initial begin
    $dumpfile("mixed.vcd");
    $dumpvars;
    #1001000 $finish;
  end
endmodule
// 기대: global precision = 100ps
// q: 1 at 2500ps (25 ticks × 100ps)
// r: 1 at 1,000,000ps (10,000 ticks × 100ps)
```

### 테스트 케이스 3 — $time vs $realtime 차이

```verilog
`timescale 1ns/100ps
module test_time;
  initial begin
    #2.3;
    $display("$time    = %0d",  $time);     // 기대: 2
    $display("$realtime = %0f", $realtime); // 기대: 2.3
  end
endmodule
```

### 테스트 케이스 4 — zero-time advance 감지

```verilog
`timescale 1ns/1ns
module test_zero;
  reg a;
  initial begin
    a = 0;
    #0.4 a = 1;   // 0.4ns → 반올림 0 → 동일 time-step
    $display("T=%0d a=%b", $time, a);
    // 기대: a=1이 즉시 보임 (advance 없으므로 T=0에서 a=1)
    $finish;
  end
endmodule
```

### 테스트 케이스 5 — precision 경계 정확성

```verilog
`timescale 10ns/1ns
module test_boundary;
  reg clk;
  initial clk = 0;
  always #5 clk = ~clk;  // 5ns 토글 → 1ns precision으로 5ns = 5 ticks

  integer count;
  initial begin
    count = 0;
    repeat(10) @(posedge clk) count++;
    // 10번 posedge: 0→5→10→15...→50ns
    $display("Final time = %0d ns (expect 50)", $time);
    $finish;
  end
endmodule
// 기대: $time = 50, 각 posedge 간격 정확히 5ns
```

### 검증 방법

1. 각 테스트를 Icarus Verilog로 실행해 VCD 생성
2. 동일 소스를 본 시뮬레이터로 실행해 VCD 생성
3. VCD 타임스탬프를 파싱해 1 precision unit 이내 일치 확인
4. 불일치 시 반올림 계산 로그를 덤프해 어느 단계에서 갈렸는지 추적

---

## Sources

- 본 spec §6.3 (Timescale and Timing)
- research-log: [`timescale-precision-2026-05-28.md`](research-log/timescale-precision-2026-05-28.md)
- IEEE 1800-2012 §3 (Scheduling), §20 ($time, $realtime)
- IEEE 1364-2005 §17 (Compiler directives), §6 (Expressions and operators)
- https://www.chipverify.com/verilog/verilog-timescale (syntax, rounding)
- https://www.chipverify.com/verilog/verilog-timescale-scope (scoping)
- https://systemverilog.dev/6.html (mixed timescale, 64-bit integer time)
- https://sagar5258.blogspot.com/2017/11/timeunit-and-timeprecision-in.html (timeunit/timeprecision SV declarations)
- https://circuitcove.com/system-tasks-time/ ($time vs $realtime semantics)
- https://verilator.org/guide/latest/warnings.html (TIMESCALEMOD, --timescale flag)
