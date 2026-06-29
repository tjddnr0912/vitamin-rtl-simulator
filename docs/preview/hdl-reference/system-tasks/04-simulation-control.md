# 04 · Simulation Control

## 개요

시뮬레이션을 종료하거나 일시 중단하는 태스크 카테고리다.
가장 단순한 테스트벤치에서도 `$finish` 없이는 시뮬이 끝나지 않으므로
Phase 1 MVP의 필수 항목이다.

## 지원 Phase (vitamin 구현 상태)

- **✅ 구현됨 (Phase 1)**: `$finish`, `$stop`. severity 파라미터(0|1|2)는 **수용-무시**
  (verbosity 미구현; exit code와 무관). `$finish` 시 현 타임스텝 postponed 영역($strobe/$monitor)
  드레인 후 종료(P1-6).
- **✅ 구현됨 (v9)**: `$exit` — `$finish` 별칭(`SysTaskId::Finish`)으로 배선. `program` 블록 자체는
  미지원이라 program-완료-대기 시맨틱은 단순화돼 즉시 `$finish` 동작이다(module-based TB에서 안전).

---

## 항목 상세

### `$finish[(severity)]`

- **시그니처**: `$finish;` 또는 `$finish(0|1|2);`
- **표준**: IEEE 1800-2017 §20.2 / IEEE 1364-2005 §17.8.1
- **의미**: 시뮬레이션을 **완전 종료**하고 OS로 제어를 반환한다.
  라이선스(상용 시뮬레이터의 경우)를 해제하며 재개 불가.
  `severity` 파라미터는 종료 시 출력할 진단 메시지 수준을 결정한다.
  기본값은 1.
- **반환**: void (반환하지 않음 — 시뮬 종료)

#### severity 레벨

| Level | 출력 내용 |
|-------|----------|
| `0` | 아무것도 출력하지 않음 |
| `1` | 시뮬 시각 + 소스 파일/라인 위치 (기본값) |
| `2` | 시뮬 시각 + 위치 + 메모리 사용량 + CPU 시간 통계 |

```sv
// 가장 흔한 패턴
initial begin
  // ... 자극 인가 ...
  #100;
  $finish;          // severity=1 (기본): 시각+위치 출력 후 종료
end

// 조용한 종료 (CI 로그 정리용)
$finish(0);

// 상세 프로파일링 정보 포함
$finish(2);
```

#### iverilog vvp 종료 코드

| vvp 실행 방식 | $finish 동작 | exit code |
|-------------|------------|----------|
| `vvp sim.vvp` (기본) | 정상 종료 | 0 |
| `vvp -n sim.vvp` | $stop → $finish와 동일 | 0 |
| `vvp -N sim.vvp` | $stop → exit 1 반환 | **1** (if $stop called) |
| `vvp -q sim.vvp` | $display 출력 억제 | 0 |

`-N` 플래그를 사용하면 `$stop`이 호출될 때 exit code 1을 반환한다.
CI 파이프라인에서 테스트 실패를 exit code로 감지하려면 이 패턴을 사용한다:

```bash
# CI에서 $stop을 에러 신호로 활용
vvp -N sim.vvp
if [ $? -ne 0 ]; then echo "SIMULATION FAILED"; fi
```

#### Verilator 동작

Verilator는 `$finish`/`$stop`의 severity 파라미터를 **무시**한다.
기본 exit code는 0. 별도 `VL_FINISH_CALLBACK` 매크로로 커스텀 종료 핸들러를
등록할 수 있으나 Phase 1 범위 밖이다.

---

### `$stop[(severity)]`

- **시그니처**: `$stop;` 또는 `$stop(0|1|2);`
- **표준**: IEEE 1800-2017 §20.2 / IEEE 1364-2005 §17.8.2
- **의미**: 시뮬레이션을 **일시 중단**하고 인터랙티브 디버그 모드로 진입한다.
  상용 시뮬레이터에서는 웨이브폼 뷰어나 콘솔 프롬프트가 뜬다.
  severity 레벨은 `$finish`와 동일한 규칙.
- **비대화형 환경**: CI 환경처럼 인터랙티브 모드가 없는 경우 동작이 시뮬레이터마다 다르다.
  - iverilog 기본: 인터랙티브 프롬프트 진입 (무한 대기 가능)
  - `vvp -n`: `$stop` → `$finish`로 자동 변환
  - `vvp -N`: `$stop` → `$finish` + exit code 1
  - Verilator: 즉시 종료 처리

```sv
// 오류 검출 패턴
always @(posedge clk) begin
  if (error_flag) begin
    $display("ERROR at time %0t: error_flag asserted", $time);
    $stop;          // 디버그용 중단
  end
end

// CI 친화적 패턴 ($stop을 명시적 실패로 사용)
// → vvp -N 실행 시 exit code 1 반환
```

---

### `$exit` (SV only — vitamin ✅ `$finish` 별칭)

> **vitamin: ✅ 구현(v9).** `$exit`는 `$finish` 별칭(`SysTaskId::Finish`)으로 배선됐다. program 블록
> 자체는 미지원이라 program-완료-대기 시맨틱은 단순화돼 즉시 `$finish` 동작이다. 아래는 IEEE 표준 레퍼런스다.

- **시그니처**: `$exit;`
- **표준**: IEEE 1800-2017 §20.2
- **의미**: `program` 블록 내에서만 의미 있다.
  모든 program 블록이 완료될 때까지 대기한 뒤 암묵적으로 `$finish`를 호출한다.
  `module` 블록에서 직접 사용할 경우 동작은 시뮬레이터 정의이며,
  대부분은 `$finish`의 별칭으로 처리한다.
- **Verilator**: `$exit`는 `$finish`의 별칭으로 구현되어 있다.
- **Phase 2 이유**: program 블록 기반 SV testbench는 Phase 1 MVP 범위 밖.

```sv
// SV program 블록 패턴 (Phase 2)
program automatic tb;
  initial begin
    // ... 테스트 로직 ...
    $exit;   // 모든 program 블록 완료 후 시뮬 종료
  end
endprogram
```

---

## 합성 가능성

❌ 비합성 — 전 태스크가 시뮬레이션 전용.
합성 도구는 `$finish`, `$stop`, `$exit` 호출을 무시한다.

---

## 본 프로젝트 구현 메모

- Phase 1: `$finish`, `$stop` 구현. severity 파라미터(0|1|2)는 **수용-무시**(verbosity 미구현; exit code와 무관).
- `$finish` 호출 시 현 타임스텝 postponed 영역($strobe/$monitor)을 드레인한 뒤 종료(IEEE §17, P1-6).
- exit code 전달 방식은 simulator runner(CLI) 레이어에서 결정 (`FinishReason`→exit class 매핑).
- `$exit`: **✅ 구현(v9)** — `$finish` 별칭(`"$exit" => SysTaskId::Finish`). program 블록 미지원이라 program-완료-대기 시맨틱은 즉시-`$finish`로 단순화.

## Sources

- IEEE 1800-2017 §20.2
- IEEE 1364-2005 §17.8
- research-log: [system-tasks-display-time-2026-05-28.md](../../research-log/system-tasks-display-time-2026-05-28.md)
- [chipverify.com $stop $finish](https://chipverify.com/verilog/verilog-stop-finish)
- [circuitcove.com Simulation Control Tasks](https://circuitcove.com/system-tasks-simulation-control/)
- [steveicarus.github.io VVP Flags](https://steveicarus.github.io/iverilog/usage/vvp_flags.html)
- [verilator.org Input Languages](https://verilator.org/guide/latest/languages.html)
