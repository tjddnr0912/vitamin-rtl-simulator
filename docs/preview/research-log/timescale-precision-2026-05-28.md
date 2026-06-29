---
topic: timescale-precision
date: 2026-05-28
rounds: 3
primary_sources_fetched:
  - https://www.chipverify.com/verilog/verilog-timescale
  - https://www.chipverify.com/verilog/verilog-timescale-scope
  - https://circuitcove.com/system-tasks-time/
  - https://systemverilog.dev/6.html
  - https://sagar5258.blogspot.com/2017/11/timeunit-and-timeprecision-in.html
  - https://verilator.org/guide/latest/warnings.html
queries:
  - "Round 1 영문: Verilog SystemVerilog timescale directive syntax units ps fs rounding rules IEEE 1800"
  - "Round 1 영문: SystemVerilog $time vs $realtime integer real precision semantics IEEE 1800"
  - "Round 2 영문: Verilog timescale per-module scope mixed timescale design compilation unit IEEE behavior"
  - "Round 2 영문: Icarus Verilog Verilator mixed timescale simulation integer time 64-bit floating point accumulation error"
  - "Round 3 영문: Verilog simulator 64-bit integer time representation precision rounding delay calculation time wheel"
  - "Round 3 영문: SystemVerilog timeunit timeprecision declaration per-module scope IEEE 1800 compilation unit mixed precision global finest"
  - "Round 3 영문: Verilog simulator floating point time accumulation error why integer representation avoids drift"
---

# Research: `timescale` 정밀도와 시간 모델

## `timescale` 문법 — 확인된 규칙

### 기본 구문

```verilog
`timescale <time_unit>/<time_precision>
```

두 인수 모두 `{1|10|100}<단위>` 형식이어야 한다 — 허용값은 1, 10, 100 세 가지뿐이고, 단위는 `s / ms / us / ns / ps / fs` 중 하나. 따라서 `200ps` 나 `3ns` 는 문법 오류.

예시:
```verilog
`timescale 1ns/1ps     // unit=1ns, precision=1ps
`timescale 10ns/100ps  // unit=10ns, precision=100ps
`timescale 1us/10ns    // unit=1us, precision=10ns
```

제약: precision은 unit보다 작거나 같아야 한다(`precision ≤ unit`). `1ps/1ns` 는 precision > unit 이므로 에러.

[출처: chipverify.com/verilog/verilog-timescale ✓ WebFetch]

---

## 반올림 규칙 — IEEE 1800 정의

delay 값은 두 단계를 거쳐 정수 이벤트 카운트로 변환된다:

1. **모듈 precision으로 반올림**: 해당 모듈의 `time_precision` 단위로 반올림. 0.5 이상 → 올림, 0.5 미만 → 내림.
2. **전역 finest precision으로 스케일**: 설계 전체에서 가장 미세한 precision을 1 simulation tick으로 삼고, 반올림된 값을 그 배수로 환산.

구체 예시 (`chipverify.com` ✓):

```
`timescale 1ns/1ns 인 모듈에서:
  #0.49 → 0ns (반올림하면 0 — advance 없음)
  #0.50 → 1ns (반올림하면 1)
  #0.51 → 1ns (반올림하면 1)
```

`systemverilog.dev/6.html` ✓ 에서 혼합 timescale 예시:

```
Module A: `timescale 1ns/10ps  → #1.2 → 1.2ns = 120 × 10ps = 120 global ticks
Module B: `timescale 1us/10ns  → #3.4 → 3.4us = 340 × 10ns = 340,000 × 10ps ticks
(전역 finest precision = 10ps 가정 시)
```

---

## 스코프 — 파일 순서 종속성

`timescale`은 **컴파일러 디렉티브**다. 적용 범위 규칙:

- 디렉티브는 파일 내에서 그것 다음에 오는 모든 모듈에 적용된다.
- 다음 `timescale`이 나올 때까지 유지된다.
- `include`로 붙여넣기된 코드에도 그대로 영향을 준다.
- **timescale 없이 오는 모듈**은 직전에 선언된 디렉티브를 상속하거나, 아무것도 없으면 툴 default를 받는다. 컴파일 순서에 따라 결과가 달라진다 — 고전적인 file-order dependency 함정.

[출처: chipverify.com/verilog-timescale-scope ✓ WebFetch]

### SystemVerilog `timeunit` / `timeprecision` — 순서 독립

SV는 `timescale`의 파일-순서 의존성 문제를 해결하기 위해 모듈 내부 선언을 도입:

```systemverilog
module top;
  timeunit 1ns;
  timeprecision 1ps;
  // ...
endmodule
```

규칙 (IEEE 1800-2012):
- 모듈, program, package, interface, $root 각각 최대 1개.
- 반드시 해당 스코프의 다른 아이템보다 먼저 선언.
- 명시 없으면 상위 스코프(포함 모듈 → 컴파일 유닛 → 이전 `timescale` 디렉티브 순서) 에서 상속.

[출처: sagar5258.blogspot.com ✓ WebFetch]

---

## 혼합 timescale 설계의 전역 precision

핵심 규칙 (복수 출처 교차확인):

> **전역 simulation time precision = 설계 전체 모듈 중 가장 작은(finest) precision**

- 모든 `timescale` 디렉티브의 precision argument
- 모든 `timeprecision` 선언
- 모든 `timeunit` 선언의 precision argument

이 중 가장 작은 것이 시뮬레이터의 1 simulation tick이 된다.

결과: 한 모듈에서 `1ns/1ns`를 쓰더라도 다른 모듈이 `10ps` precision을 가지면 전체 설계가 `10ps` 단위로 동작한다 — `1ns/1ns` 모듈의 `#1` 은 `100 ticks`로 스케일.

[출처: systemverilog.dev/6.html ✓, sagar5258.blogspot.com ✓]

---

## `$time` vs `$realtime` — 정확한 의미론

| | `$time` | `$realtime` |
|---|---------|-------------|
| 반환 타입 | 64-bit 부호 없는 정수 | real (64-bit 부동소수) |
| 단위 | 호출 모듈의 `time_unit` | 호출 모듈의 `time_unit` (소수 포함) |
| precision 반영 | 정수 → precision 이하 절사 | precision까지 소수로 표시 |
| 용도 | 이벤트 시각 비교, 로직 | 파형 덤프, 사람이 읽는 출력 |

예시:
```systemverilog
// `timescale 1ns/100ps 모듈에서
// 현재 시각 = 2.5ns
$display($time);      // → 2 (정수, 소수 절사)
$display($realtime);  // → 2.5 (precision 포함)
```

`$stime`도 존재: 32-bit 정수 반환, 큰 시각에서 overflow 위험.

[출처: circuitcove.com/system-tasks-time/ ✓ WebFetch]

---

## Icarus Verilog vs Verilator — 혼합 timescale 동작 차이

**Icarus Verilog**:
- 일부 모듈에만 `timescale`이 있고 나머지가 없을 때 경고 발행: "timescale for xxx inherited from another file"
- 자동 감지 없음; 컴파일 순서 의존성 경고 수준에서 처리

**Verilator**:
- `TIMESCALEMOD` 경고: 일부 모듈만 `timescale`을 선언하고 나머지가 없을 때 트리거
- `--timescale` CLI 플래그로 전역 timescale 강제 설정 가능
- 또는 `timescale.vh` 헤더를 모든 파일에 `include` 하는 패턴 권장

공통점: 둘 다 내부적으로 64-bit 정수 기반 시간 표현 사용.

[출처: verilator.org/guide/latest/warnings.html ✓ WebFetch, sourceforge Icarus mailing list]

---

## 64-bit 정수 시간 — 왜 부동소수를 쓰지 않는가

IEEE 1800 / 실제 시뮬레이터가 부동소수 대신 64-bit 정수를 쓰는 이유:

1. **부동소수 누적 오차**: 0.1 같은 소수는 IEEE 754에서 정확히 표현되지 않는다. 수백만 번 더하면 마지막 유효 비트가 소실되고 시각이 조용히 어긋난다.

2. **덧셈 반복의 정밀도 열화**: 지수(exponent) 차이가 크면 덧셈 결과의 유효 비트가 줄어든다. 시뮬레이션 시간은 긴 수명 동안 작은 값을 누적하는 극단적 사례.

3. **정수는 정확하다**: 64-bit 정수는 2^64 − 1 까지 정확히 표현. 10ps precision으로 전체 설계를 잡으면 최대 표현 가능 시간 ≈ 184,467 초 — 어떤 RTL 시뮬레이션에도 충분.

4. **재현성**: 정수 연산은 플랫폼·컴파일러 최적화와 무관하게 동일한 결과를 보장.

역사적 교훈 — Patriot 미사일 사건: 0.1초 단위 float 누적 → 100시간 후 0.34초 오차 → 미사일 요격 실패 (1991, 걸프전). 시뮬레이터 버그는 무기 시스템만큼 치명적이지 않더라도 동일한 수학적 원리로 조용한 오답을 만든다.

[분석 출처: systemverilog.dev/6.html ✓, blog.demofox.org floating-point precision]

---

## Sources

- https://www.chipverify.com/verilog/verilog-timescale (WebFetch ✓)
- https://www.chipverify.com/verilog/verilog-timescale-scope (WebFetch ✓)
- https://circuitcove.com/system-tasks-time/ (WebFetch ✓)
- https://systemverilog.dev/6.html (WebFetch ✓)
- https://sagar5258.blogspot.com/2017/11/timeunit-and-timeprecision-in.html (WebFetch ✓)
- https://verilator.org/guide/latest/warnings.html (WebFetch ✓)
- https://verificationacademy.com/forums/systemverilog/timescale-and-timeunit
- https://sourceforge.net/p/iverilog/mailman/iverilog-devel/thread/1320251933.13733.YahooMailNeo@web25405.mail.ukl.yahoo.com/
- IEEE 1800-2012 (LRM) — §3 (Scheduling), §6.3 (timescale), §20 (system tasks $time/$realtime)
