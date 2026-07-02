# 05 · Time Functions

## 개요

시뮬레이션 현재 시각을 조회하는 시스템 함수 카테고리다.
`$time`과 `$realtime`은 `$display` 포맷 specifier `%t`와 함께 가장 빈번하게 사용된다.
내부 시간 표현에 대한 배경은 [08-timescale-and-timing.md](../../08-timescale-and-timing.md) 참조.

## 지원 Phase (vitamin 구현 상태)

- **구현됨**: `$time`, `$realtime`, `$display` `%t` specifier(§21.3.2 전체 의미론), `$timeformat`.
- **✅ `$stime` 구현(v7, 2026-06-12)**: `map_sysfunc`에 `SysFuncId::Stime` 등록, `$time`의 하위
  32비트 unsigned를 반환(`eval.rs` `(now/M) & 0xffff_ffff`, self-width=unsigned 32, 1364 §17.7.2).
- **✅ `%t`/`$timeformat` 완전 구현(2026-07-02, ROADMAP §4.5.60)**: 옛 plain-decimal caveat 해소.
  `%t`는 모듈-unit 시간값을 `$timeformat` 단위(기본=global precision)로 rescale하고
  기본 최소 필드폭 20으로 우측정렬한다(`%Nt`/`%0t`/`%0Nt`가 오버라이드, iverilog 핀).
  `$timeformat(units, prec, suffix, minw)`=런타임 문(0 args=defaults 리셋·1~3 args=loud).
  정수 경로=자릿수-문자열 정밀연산+**truncation**(9.995@prec1→"9.9")·real 경로=%f 라운딩.

---

## 항목 상세

### `$time`

- **시그니처**: `$time` (인자 없음)
- **표준**: IEEE 1800-2017 §20.3 / IEEE 1364-2005 §17.7.1
- **반환 타입**: 64-bit unsigned integer
- **의미**: 호출 모듈의 `timescale` unit 단위로 현재 시뮬 시각을 정수 반환한다.
  정밀도(precision)에 따라 반올림된 후 unit으로 변환한 값이다.

```sv
`timescale 1ns / 100ps
initial begin
  #5;                                  // 5ns 지연
  $display("time = %0d ns", $time);   // time = 5 ns
  #0.3;                                // 300ps 지연 → precision 반올림 → 300ps
  $display("time = %0d ns", $time);   // time = 5 ns  (300ps < 1ns → 정수 변환 시 0 추가)
  // 주의: $time은 ns 단위 정수 → 5.3ns는 5로 표현
end
```

**정수 변환 규칙**: precision 경계로 반올림된 시각을 unit으로 나눈 후 정수 변환(truncation이 아닌 반올림).
예: `1ns/100ps` timescale에서 5.25ns → precision 반올림 → 5.3ns → `$time` = 5 (ns 단위, fractional 버림).
`$realtime`과 달리 fractional 부분을 볼 수 없다.

---

### `$stime`

> **vitamin: ✅ 구현됨(v7).** `$time`의 하위 32비트를 unsigned 32-bit으로 반환한다
> (`(now/M) & 0xffff_ffff`, 1364 §17.7.2).

- **시그니처**: `$stime` (인자 없음)
- **표준**: IEEE 1800-2017 §20.3 / IEEE 1364-2005 §17.7.2
- **반환 타입**: 32-bit unsigned integer
- **의미**: `$time`의 **하위 32비트**를 반환한다.
  값이 2³² − 1 (~4.3 billion)을 초과하면 wrap-around가 발생한다.
  현대적인 용도에서는 거의 사용하지 않는 legacy 함수.

```sv
// wrap-around 위험 예시 (1ns 단위, 4.3초 이상 시뮬)
$display("stime=%0d", $stime);   // 하위 32비트만 반환
$display("time=%0d", $time);     // 64비트 전체 반환 (권장)
```

**권장**: `$stime` 대신 `$time`을 사용한다.
`$stime`은 하드웨어 레지스터처럼 32비트 정수를 기대하는 레거시 코드와의 호환 목적으로만 남아 있다.

---

### `$realtime`

- **시그니처**: `$realtime` (인자 없음)
- **표준**: IEEE 1800-2017 §20.3 / IEEE 1364-2005 §17.7.3
- **반환 타입**: `real` (IEEE 754 double-precision floating-point)
- **의미**: 현재 시뮬 시각을 **소수**로 반환한다.
  timescale precision이 반영되어 fractional 시각을 정확히 표현한다.

```sv
`timescale 1ns / 100ps
initial begin
  #2.5;                                       // 2.5ns
  $display("time=%0d realtime=%g", $time, $realtime);
  // time=2  realtime=2.5
  // ($time은 ns 단위 정수 → 2, $realtime은 소수 → 2.5)
end
```

**정밀도 한계**: `real`은 IEEE 754 double (약 15~17 유효 자릿수)이므로
극히 긴 시뮬레이션에서는 정밀도 손실이 생길 수 있다.
일반적인 RTL 시뮬레이션 범위에서는 문제없다.

---

## $time vs $realtime 비교

| 항목 | `$time` | `$realtime` |
|------|---------|-------------|
| 반환 타입 | 64-bit unsigned int | real (double) |
| 소수 표현 | ❌ (정수만) | ✅ (precision 반영) |
| %t specifier | 사용 권장 | 사용 가능 |
| 오버플로우 | 64비트 한계 (~1.8×10¹⁹) | double 정밀도 한계 |
| 사용 빈도 | 높음 (기본) | fractional 확인 시 |

---

## 변환 예시 (timescale 상호작용)

```sv
`timescale 10ns / 1ns   // unit=10ns, precision=1ns

initial begin
  #1.5;     // 1.5 × 10ns = 15ns → precision 반올림 → 15ns → $time = 1 (unit=10ns 단위)
  $display("time=%0d (×10ns)  realtime=%g (×10ns)", $time, $realtime);
  // time=1  realtime=1.5    (unit=10ns 기준 소수값)

  #3;       // 3 × 10ns = 30ns 추가
  $display("time=%0d  realtime=%g", $time, $realtime);
  // time=4  realtime=4.5
end
```

timescale 반올림·변환 규칙의 전체 정의는
[08-timescale-and-timing.md](../../08-timescale-and-timing.md) 참조.

---

## Icarus / Verilator 지원

| 함수 | Icarus | Verilator | vitamin |
|------|--------|-----------|---------|
| `$time` | 완전 지원 | Generally supported | ✅ |
| `$stime` | 완전 지원 | Generally supported | ✅ (v7) |
| `$realtime` | 완전 지원 | Generally supported | ✅ |

Verilator 공식 문서는 세 함수 모두 "Generally supported"로 분류한다.
동작 차이는 timescale 처리 정밀도에서 나타날 수 있으며,
차등검증 시 `$realtime` 소수 비교는 epsilon 허용 범위를 적용하는 것을 권장한다.

---

## 합성 가능성

❌ 비합성 — 시뮬레이션 전용 함수.
합성 도구는 `$time`/`$realtime` 호출을 무시하거나 오류를 발생시킨다.

---

## 본 프로젝트 구현 메모

- `$time`/`$realtime`은 `sim-engine` `eval_sysfunc`(eval.rs)가 평가, 매핑은 `elaborate::map_sysfunc`.
  내부 시간 모델: 64비트 정수 ticks (§08 timescale 참조)
- `$time`: `now / M`(호출 모듈의 multiplier `M`) — 정수, fractional 버림
- `$realtime`: `now / M`를 f64로 — fractional 유지
- **모듈별 `M`은 호출 시점에 결정**: postponed `$strobe`/`$monitor`는 등록 시점의 `M`을
  `FmtCapture.time_mult`에 스냅샷해 flush 때 그 값으로 렌더(혼합 timescale에서
  마지막 실행 프로세스의 `M`을 쓰지 않도록). 회귀: `cli/tests/timescale_postponed.rs`.
- `$stime`: **✅ 구현됨(v7)** — `map_sysfunc`에 `SysFuncId::Stime` 등록, `eval.rs`가
  `(now/M) & 0xffff_ffff`(unsigned 32)로 평가. 위 항목 참조.
- `%t`: **full §21.3.2 렌더**(`builtins.rs` `fmt_time_spec`) — 값 × M(호출 모듈 multiplier)을
  `$timeformat` 단위로 decimal-shift, 기본 minw 20 우측정렬. `$timeformat` 상태는 렌더 시점
  라이브(strobe/monitor flush=flush-time 상태·M=등록 시점 스냅샷, iverilog 핀). 회귀:
  `cli/tests/timeformat.rs` (21 tests).

## Sources

- IEEE 1800-2017 §20.3
- IEEE 1364-2005 §17.7
- [08-timescale-and-timing.md](../../08-timescale-and-timing.md) (내부 문서, timescale 변환 규칙)
- research-log: [system-tasks-display-time-2026-05-28.md](../../research-log/system-tasks-display-time-2026-05-28.md)
- [circuitcove.com Time Functions](https://circuitcove.com/system-tasks-time/)
- [chipverify.com Verilog Timescale](https://www.chipverify.com/verilog/verilog-timescale)
- [verilator.org Input Languages](https://verilator.org/guide/latest/languages.html)
