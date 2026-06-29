---
title: "SystemVerilog/Verilog System Tasks 조사 — Display·Control·Time (Phase 1)"
date: 2026-05-28
author: research-skill (Claude Sonnet 4.6)
scope:
  - IEEE 1800-2017 §20 / IEEE 1364-2005 §17 display system tasks
  - $display/$write/$monitor/$strobe + b/o/h 변형, $monitoron/$monitoroff
  - $finish/$stop/$exit 시뮬레이션 제어
  - $time/$stime/$realtime 시간 함수
  - 포맷 specifier 전체 목록
  - Icarus Verilog (vvp) / Verilator 동작 차이
rounds: 2
status: complete
---

# SystemVerilog/Verilog System Tasks 조사 — Display·Control·Time

## 배경

Vitamin RTL 시뮬레이터 `hdl-builtins` 크레이트 구현에 앞서,
Phase 1 핵심 시스템 태스크의 표준 의미론과 주요 시뮬레이터(Icarus · Verilator)
동작 차이를 확인한다. 참조 표준: IEEE 1800-2017 §20 (SystemVerilog),
IEEE 1364-2005 §17 (Verilog — 현재 1800 내 흡수).

---

## Display 태스크 4종과 b/o/h 변형 16종

### 4종 핵심 태스크 비교

Verilog/SV의 콘솔 출력 태스크는 **실행 시점**과 **자동 개행** 여부로 구분된다.

| 태스크 | 실행 시점 | 자동 개행 | 특징 |
|--------|----------|----------|------|
| `$display` | Active/Inactive 영역 — 호출 즉시 | ✅ | 가장 기본. 호출 시점의 신호값 |
| `$write` | Active/Inactive 영역 — 호출 즉시 | ❌ | $display와 동일, 개행 없음 |
| `$strobe` | Postponed 영역 (현재 timestep 마지막) | ✅ | NBA 결과 반영 후 출력 |
| `$monitor` | Postponed 영역 (인자 변화 시 자동 트리거) | ✅ | 백그라운드 상시 감시 |

**$display vs $strobe 핵심 차이**: 같은 시뮬 시각에 NBA (`<=`)로 변수가 갱신되는 경우,
`$display`는 갱신 전 값을, `$strobe`는 최종 갱신 후 값을 출력한다.

**$monitor 제약**: 시뮬레이션 전체에서 하나의 `$monitor`만 활성 상태를 유지한다.
새 `$monitor` 호출 시 이전 것은 자동 비활성화된다.
`$monitoroff` / `$monitoron`으로 일시 중단·재개 가능.

[출처: peterfab.com Verilog Display Tasks Reference, WebFetch 검증 ✓;
chipverify.com/verilog/verilog-display-tasks, WebFetch 검증 ✓]

### b/o/h 변형 — 기본 기수(radix) 변경

4개 태스크 패밀리 × 4가지 기수 = **16종 변형**.

| 기수 | $display 계열 | $write 계열 | $strobe 계열 | $monitor 계열 |
|------|--------------|------------|-------------|--------------|
| 십진(기본) | `$display` | `$write` | `$strobe` | `$monitor` |
| 이진 | `$displayb` | `$writeb` | `$strobeb` | `$monitorb` |
| 8진 | `$displayo` | `$writeo` | `$strobeo` | `$monitoro` |
| 16진 | `$displayh` | `$writeh` | `$strobeh` | `$monitorh` |

**동작 원리**: 명시적 포맷 specifier(`%d`, `%h` 등)가 없는 인자에 대해 기본 기수를 바꾼다.
명시적 specifier는 항상 우선한다.

```sv
// 예: 8비트 값 255를 각 기수로 출력
$display("val=%0d", 8'hFF);   // val=255  (명시적 %0d 우선)
$displayh("val=", 8'hFF);     // val=ff   (기본 16진 적용)
$displayb("val=", 8'hFF);     // val=11111111
```

[출처: peterfab.com Verilog Reference (WebFetch 검증 ✓);
Round 1 WebSearch synthesis]

---

## 포맷 Specifier 전체 목록

IEEE 1800-2017 §20.10 / IEEE 1364-2005 §17.1 기준.

| Specifier | 의미 | 인자 타입 | 비고 |
|-----------|------|----------|------|
| `%d` / `%D` | 십진수 | integer | 기본 형식 |
| `%b` / `%B` | 이진수 | bit vector | |
| `%h` / `%H` | 16진수 | bit vector | `%x`/`%X` 동의어 |
| `%x` / `%X` | 16진수 | bit vector | `%h`와 동일 |
| `%o` / `%O` | 8진수 | bit vector | |
| `%c` / `%C` | ASCII 문자 | 8-bit | 하위 8비트를 문자로 |
| `%s` / `%S` | 문자열 | string / byte array | |
| `%t` / `%T` | 시간 | time | `$timeformat` 영향 받음 |
| `%v` / `%V` | net 신호 강도 | net (4-state) | strength + value |
| `%e` / `%E` | 실수 지수 표기 | real | 예: 1.23e+02 |
| `%f` / `%F` | 실수 소수 표기 | real | 예: 123.000000 |
| `%g` / `%G` | 실수 자동 선택 | real | e/f 중 짧은 쪽 |
| `%m` / `%M` | 계층 모듈 이름 | (삽입, 인자 불필요) | 디버그용 |
| `%p` / `%P` | assignment pattern | struct/enum/dynamic | SV §20.10.2 |
| `%u` | 비형식 2-value | bit vector | 이진 덤프 |
| `%z` | 비형식 4-value | bit vector | 4-state 덤프 |
| `%l` / `%L` | 라이브러리 바인딩 | (삽입) | |

**폭 수정자**:
- `%6d` — 필드 폭 6, 오른쪽 정렬 (leading space)
- `%06d` — 필드 폭 6, 0 패딩
- `%0d` / `%0h` 등 — 0 앞의 공백 없이 최소 폭 출력 (most compact)

[출처: circuitcove.com/system-tasks-format-spec (WebFetch 검증 ✓);
hdlworks.com/SystemDisplayTasks (WebFetch 검증 ✓)]

---

## 시뮬레이션 제어: $finish / $stop / $exit

### 시그니처와 severity

```sv
$finish;          // default severity=1
$finish(0);       // 출력 없음
$finish(1);       // 시뮬 시각 + 위치 출력 (기본)
$finish(2);       // 시뮬 시각 + 위치 + 메모리/CPU 통계

$stop;
$stop(0);  $stop(1);  $stop(2);   // 동일 severity 규칙

$exit;            // SV program 블록 전용
```

| 태스크 | 동작 | OS 반환 |
|--------|------|--------|
| `$finish` | 시뮬 완전 종료, 라이선스 해제 | 제어 반환 |
| `$stop` | 시뮬 일시 중단, 인터랙티브 모드 | 재개 가능 |
| `$exit` | 모든 program 블록 완료 후 $finish | SV 전용 |

### iverilog vvp 종료 코드

| vvp 플래그 | $finish 동작 | $stop 동작 | exit code |
|-----------|------------|-----------|----------|
| (기본) | 정상 종료 | 인터랙티브 진입 | 0 |
| `-n` | 정상 종료 | $finish와 동일 | 0 |
| `-N` | 정상 종료 | $finish + exit 1 | **1 if $stop called** |
| `-q` | $display 출력 억제 | — | — |

CI 파이프라인에서는 `vvp -N` 사용 시 `$stop`으로 테스트 실패를 exit code로 전달 가능.

[출처: steveicarus.github.io/iverilog/usage/vvp_flags.html (WebFetch 검증 ✓)]

### Verilator 동작

- `$finish`/`$stop`의 선택적 severity 파라미터는 **무시**된다.
- `$exit`는 `$finish`의 별칭으로 처리된다.
- 별도 exit code 조작 없이 정상 종료 시 0 반환.

[출처: verilator.org/guide/latest/languages.html (WebFetch 검증 ✓)]

---

## 시간 함수: $time / $stime / $realtime

### 반환 타입과 의미론

| 함수 | 반환 타입 | 단위 | 비고 |
|------|----------|------|------|
| `$time` | 64-bit unsigned integer | 호출 모듈의 timescale unit | 반올림 적용 |
| `$stime` | 32-bit unsigned integer | 동일 | $time 하위 32비트, legacy |
| `$realtime` | real | 호출 모듈의 timescale unit | precision 반영 소수 |

### timescale과의 관계

```sv
`timescale 1ns / 100ps   // unit=1ns, precision=100ps
initial begin
  #2.5;   // 2.5ns → precision 반올림 → 2.5ns (100ps 경계)
  $display("time=%0d  realtime=%g", $time, $realtime);
  // time=2  realtime=2.5   (※ $time은 unit 단위 정수 → 2)
  // 단, precision 반올림 후 unit으로 나누므로 2.5ns/1ns = 2.5 → 정수화 = 2 (truncate 아닌 round)
end
```

- `$realtime`은 precision까지 반영한 소수를 반환해 fractional time을 정확히 표현한다.
- `$stime`은 32비트 범위를 초과하는 시뮬에서 wrap-around가 발생할 수 있어 현대 코드에서는 거의 사용하지 않는다.

[출처: circuitcove.com/system-tasks-time (WebFetch 검증 ✓)]

---

## Icarus vs Verilator — Display 태스크 주요 차이

| 항목 | Icarus Verilog | Verilator |
|------|---------------|-----------|
| combo 블록 $display | 1회 실행 (이벤트 기반) | 복수 실행 가능 (재정렬) |
| $monitor 지원 | 완전 지원 | 지원 (일부 timing 차이 가능) |
| $strobe 지원 | 완전 지원 | 지원 |
| $finish severity 파라미터 | 지원 | **무시** |
| $exit | $finish 동작 | $finish 별칭 |
| $time/$realtime | 완전 지원 | Generally supported |

**Verilator 권고**: combo `always_comb` / `always @(*)` 블록 내 `$display`는 피할 것.
이벤트 정렬이 표준에서 미정의이므로 의도치 않은 중복 출력 발생 가능.
sequential 블록(`always_ff`, `initial`) 또는 `$strobe` 사용 권장.

[출처: verilator.org/guide/latest/languages.html (WebFetch 검증 ✓);
이전 조사 iverilog-verilator-behaviors-2026-05-28.md 교차 확인]

---

## Sources

1. **circuitcove.com — System Tasks Format Specifiers** — https://circuitcove.com/system-tasks-format-spec/ (WebFetch Round 1 ✓)
2. **circuitcove.com — Simulation Control Tasks** — https://circuitcove.com/system-tasks-simulation-control/ (WebFetch Round 1 ✓)
3. **circuitcove.com — Time Functions** — https://circuitcove.com/system-tasks-time/ (WebFetch Round 2 ✓)
4. **chipverify.com — Verilog Stop Finish** — https://chipverify.com/verilog/verilog-stop-finish (WebFetch Round 1 ✓)
5. **chipverify.com — Verilog Display Tasks** — https://chipverify.com/verilog/verilog-display-tasks (WebFetch Round 1 ✓)
6. **hdlworks.com — System Display Tasks** — https://www.hdlworks.com/hdl_corner/verilog_ref/items/SystemDisplayTasks.htm (WebFetch Round 1 ✓)
7. **peterfab.com — Verilog Display Tasks Reference** — https://peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00013.htm (WebFetch Round 2 ✓)
8. **steveicarus.github.io — VVP Flags** — https://steveicarus.github.io/iverilog/usage/vvp_flags.html (WebFetch Round 1 ✓)
9. **verilator.org — Input Languages** — https://verilator.org/guide/latest/languages.html (WebFetch Round 1 ✓)
10. **IEEE 1800-2017 §20** — Display tasks, simulation control, time functions (primary standard; 섹션 번호 참조, 직접 fetch 불가)
11. **IEEE 1364-2005 §17** — Verilog system tasks (superseded by 1800-2009; 내용 1800에 흡수)
12. **이전 조사** — iverilog-verilator-behaviors-2026-05-28.md (교차 확인)
