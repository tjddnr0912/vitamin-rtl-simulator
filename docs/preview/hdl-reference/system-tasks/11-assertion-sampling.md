# 11 · Assertion Sampling Functions

## 개요

SVA(SystemVerilog Assertions) concurrent assertion에서 사용되는
시간적 샘플링 함수 카테고리다.
`$past`/`$rose`/`$fell`/`$stable`/`$changed`/`$sampled`는 클록 기반 신호 이력을 조회하고,
`$assertoff`/`$asserton`/`$assertkill`/`$assertcontrol`은 런타임에 assertion의 활성 상태를 제어한다.

전부 합성 불가 검증 전용이며 IEEE 1800-2017 §16에 정의되어 있다.

## 구현 상태

- **구현됨 (2026-06-16, hand-IEEE)**: `$past`, `$rose`, `$fell`, `$stable`, `$changed`, `$sampled` —
  signal당 공유 prev-reg desugar(`rewrite_sampled`)로 합성되며, SVA antecedent 시퀀스 + pass/fail
  action block 내부에서 사용할 수 있다(`$changed`=`$past(e)!==e`, `$sampled`=action-block prev-reg 조회).
  ⚠️ iverilog 13.0이 이들을 거부('not defined')해 차분 오라클이 없어 hand-IEEE 핀.
- **구현됨 (✅ SVA-REST)**: `$assertoff`, `$asserton`, `$assertkill` — 전역 fire-gate 런타임 제어
  (`$assertoff`/`$assertkill`=후속 fire 억제, `$asserton`=재활성화; scoped `$assertoff(level, scope)` 폼은 loud).
- **미구현 (참조)**: `$assertcontrol`.

---

## 클록 기반 샘플링 의미론

이 함수들을 이해하려면 시뮬레이션 스케줄러의 리전 구조를 먼저 파악해야 한다.
전체 스케줄러 모델은 [06-simulation-engine.md](../../06-simulation-engine.md)에서 다루고,
여기서는 assertion에 직접 관련된 부분만 요약한다.

한 타임스텝(time slot) 안에서 리전은 다음 순서로 실행된다:

```
Active → Inactive → NBA → ... → Observed → Reactive → ...
```

- **Active 리전**: blocking 대입, 연속 대입, 게이트 로직 실행.
- **NBA 리전**: non-blocking 대입(`<=`) 반영 — 이 단계가 끝나야 신호가 최종값을 갖는다.
- **Observed 리전**: concurrent assertion 평가. NBA 완료 후 신호가 안정된 상태에서 실행된다.
- **Reactive 리전**: assertion의 pass/fail action block이 실행되는 곳.

Observed 리전이 핵심이다. assertion이 신호를 볼 때는 Active/Inactive/NBA에서의 변동이
모두 반영된 "이미 확정된 값"을 본다. 이 값이 "샘플링된 값(sampled value)"이다.

---

## 항목 상세

### `$past(expr [, n [, enable_expr [, @clocking_event]]])`

- **표준**: IEEE 1800-2017 §16.9.3
- **의미**: n 클록 사이클 전의 sampled value를 반환한다.
- `n`: 몇 사이클 전의 값인지. 기본값 = 1. 1 이상의 상수여야 한다.
- `enable_expr`: 이 조건이 참인 클록 에지만 "1 tick"으로 카운트한다.
  false인 에지에서는 카운터가 진행되지 않는다.
- `@clocking_event`: 명시하지 않으면 assertion의 clocking event를 상속한다.

```sv
// 기본 — 1 클록 전 값
assert property (@(posedge clk)
  req |-> ##1 ack == $past(req));

// 3 클록 전 값
assert property (@(posedge clk)
  out == $past(in, 3));

// enable_expr — valid가 1일 때만 카운트
assert property (@(posedge clk)
  valid |-> out == $past(data, 2, valid));

// 명시적 클록 지정
assert property (
  out == $past(in, 1, , @(posedge clk)));
```

**n=0은 허용하지 않는다**: IEEE 1800-2017은 `n ≥ 1`을 요구한다.
현재 사이클의 값을 얻으려면 `$sampled(expr)` 또는 expr을 직접 참조한다.

---

### `$rose(expr [, @clocking_event])` — LSB 상승 에지 감지

- **표준**: IEEE 1800-2017 §16.9.2
- **반환**: bit (true/false)
- **의미**: 이전 클록 에지의 LSB 샘플값이 (0/x/z)이고 현재 LSB 샘플값이 1이면 true.

형식적으로:

```
$rose(expr) ≡ ($past(expr[0]) !== 1'b1) && (expr[0] === 1'b1)
```

x나 z에서 1로 올라가는 경우도 true를 반환한다는 점에 주의하자.
1 → 1 유지(stable high)는 false다.

```sv
// ack가 posedge clk에서 0→1 전이하면 속성 검사
assert property (@(posedge clk)
  $rose(ack) |-> ##1 done);

// 멀티비트 신호에서 LSB만 본다
logic [3:0] bus;
$rose(bus)  // bus[0]의 (0/x/z)→1 전이만 감지, 상위 비트 무관
```

---

### `$fell(expr [, @clocking_event])` — LSB 하강 에지 감지

- **표준**: IEEE 1800-2017 §16.9.2
- **반환**: bit
- **의미**: 이전 LSB 샘플값이 (1/x/z)이고 현재 LSB 샘플값이 0이면 true.

형식적으로:

```
$fell(expr) ≡ ($past(expr[0]) !== 1'b0) && (expr[0] === 1'b0)
```

```sv
// 리셋 해제 감지 (active-low reset)
assert property (@(posedge clk)
  $fell(rst_n) |-> ##[1:5] fsm_idle);
```

---

### `$stable(expr [, @clocking_event])` — 값 유지 확인

- **표준**: IEEE 1800-2017 §16.9.2
- **반환**: bit
- **의미**: 이전 클록 에지 샘플값 == 현재 클록 에지 샘플값이면 true.
  4-state 비교이므로 x→x, z→z도 stable로 본다.

형식적으로:

```
$stable(expr) ≡ ($past(expr) === expr)
```

```sv
// 핸드셰이크: req가 assert된 동안 addr가 안정적으로 유지되어야 함
assert property (@(posedge clk)
  req && !$rose(req) |-> $stable(addr));
```

---

### `$changed(expr [, @clocking_event])` — 값 변화 감지

- **표준**: IEEE 1800-2017 §16.9.2
- **반환**: bit
- **의미**: 이전 클록 에지 샘플값 != 현재 클록 에지 샘플값이면 true.
  `$stable`의 논리적 반대.

형식적으로:

```
$changed(expr) ≡ !$stable(expr) ≡ ($past(expr) !== expr)
```

```sv
// 상태 머신 전이 감지
assert property (@(posedge clk)
  $changed(state) |-> valid_transition(state));
```

---

### `$sampled(expr)` — action block 내 안정 샘플값 조회

- **표준**: IEEE 1800-2017 §16.9.1
- **의미**: assertion이 평가된 Observed 리전 시점의 sampled value를 반환한다.

**사용 시점**: assertion의 property body 안에서는 신호가 이미 sampled value로 보이므로
`$sampled`를 쓸 필요가 없다. `$sampled`가 유용한 곳은 **action block**(pass/fail 블록)이다.

Action block은 Reactive 리전에서 실행된다. 이 시점에 Active 리전에서 신호가
다시 변동될 수 있으므로, Reactive에서 `$display("val=%0d", sig)`처럼 신호를 직접 참조하면
assertion이 평가된 시점의 값과 다른 값이 출력될 수 있다.

`$sampled(sig)`는 Observed 리전의 "확정된" 샘플값을 명시적으로 조회해
이 불일치를 방지한다.

```sv
// action block에서 정확한 신호값 출력
assert property (@(posedge clk) req |-> ack)
else $error("req=%0b ack=%0b at time %0t",
            $sampled(req), $sampled(ack), $time);

// property body 안에서는 $sampled 불필요 (중복)
assert property (@(posedge clk)
  $sampled(req) |-> $sampled(ack));  // 이렇게 쓸 필요 없음
```

---

## Assertion Runtime Control

### `$assertoff` / `$asserton` / `$assertkill`

- **표준**: IEEE 1800-2017 §20.12
- **용도**: reset 구간, 초기화 시퀀스처럼 assertion 위반이 예상되는 구간에서 노이즈를 억제한다.

```sv
// 시그니처
$assertoff  [(levels [, list_of_scopes])];
$asserton   [(levels [, list_of_scopes])];
$assertkill [(levels [, list_of_scopes])];
```

- `levels`: 0 = 모든 계층 영향. 1 = 지정 scope만. n = n 레벨 하위까지.
  생략 시 0(전체)으로 동작한다.
- scope 인자 없으면 전체 설계에 적용.

| 태스크 | 동작 |
|--------|------|
| `$assertoff` | 현재 진행 중인 assertion은 완료될 때까지 두고, 이후 새 assertion을 비활성화 |
| `$asserton` | 비활성화된 assertion을 재활성화 |
| `$assertkill` | 진행 중인 assertion 포함 즉시 종료 |

```sv
initial begin
  // 리셋 구간에서 assertion 억제
  $assertoff(0);          // 전체 비활성화

  rst_n = 0;
  #100;
  rst_n = 1;
  #20;

  $asserton(0);           // 전체 재활성화
end
```

---

### `$assertcontrol` — 세밀한 런타임 제어

`$assertoff`/`$asserton`/`$assertkill`로 커버되지 않는 세밀한 제어가 필요할 때 사용한다.
assertion 종류(concurrent, immediate 등)와 directive 종류(assert, cover, assume)별로
개별 제어할 수 있다.

```sv
// 시그니처 (IEEE 1800-2017 §20.12)
$assertcontrol(control_type
               [, [assertion_type]
               [, [directive_type]
               [, [levels]
               [, list_of_modules_or_assertions]]]]])
```

#### control_type 값

| 값 | 이름 | 의미 |
|----|------|------|
| 1 | Lock | assertion 상태를 잠근다 (이후 assertoff/on 무시) |
| 2 | Unlock | 잠금 해제 |
| 3 | On | 활성화 (`$asserton`과 동등) |
| 4 | Off | 비활성화 (`$assertoff`와 동등) |
| 5 | Kill | 즉시 kill (`$assertkill`과 동등) |
| 6 | PassOn | pass action block 실행 활성화 |
| 7 | PassOff | pass action block 실행 비활성화 |
| 8 | FailOn | fail action block 실행 활성화 |
| 9 | FailOff | fail action block 실행 비활성화 |
| 10 | NonvacuousOn | vacuous success를 제외한 pass만 활성화 |
| 11 | VacuousOff | vacuous pass 비활성화 |

#### assertion_type 비트마스크 (OR 조합 가능)

| 값 | 의미 |
|----|------|
| 1 | Concurrent assertions |
| 2 | Simple Immediate assertions |
| 4 | Observed Deferred Immediate assertions |
| 8 | Final Deferred Immediate assertions |
| 16 | `expect` 문 |
| 32 | `unique` 조건부 |
| 64 | `unique0` 조건부 |
| 128 | `priority` 조건부 |

#### directive_type 비트마스크 (OR 조합 가능)

| 값 | 의미 |
|----|------|
| 1 | `assert` 디렉티브 |
| 2 | `cover` 디렉티브 |
| 4 | `assume` 디렉티브 |

```sv
// 예: simple immediate assert 디렉티브만 Off
$assertcontrol(4, 2, 1);

// 예: concurrent assertion의 pass action block 비활성화
$assertcontrol(7, 1, 1);

// 예: 모든 assertion 타입의 cover 디렉티브 Off
$assertcontrol(4, 255, 2);
```

`$assertoff` / `$asserton` / `$assertkill`은 각각
`$assertcontrol(4, ...)` / `$assertcontrol(3, ...)` / `$assertcontrol(5, ...)`의 단순 래퍼로
볼 수 있다.

---

## 함수 정리 비교

| 함수 | 적용 영역 | 기반 개념 |
|------|----------|----------|
| `$rose(e)` | LSB 전이 감지 | `$past(e[0])` 비교 |
| `$fell(e)` | LSB 하강 감지 | `$past(e[0])` 비교 |
| `$stable(e)` | 값 유지 확인 | `$past(e) === e` |
| `$changed(e)` | 값 변화 감지 | `!$stable(e)` |
| `$past(e, n)` | n 클록 전 값 | sampled value 이력 |
| `$sampled(e)` | 현재 샘플값 | action block 안전 참조 |

---

## Icarus / Verilator 지원

| 함수 | Icarus Verilog | Verilator |
|------|---------------|-----------|
| `$past`, `$rose`, `$fell`, `$stable`, `$changed` | ❌ 거부 (13.0, 아래 ⚠️) | 지원 (`--assert` 활성화 필요) |
| `$sampled` | ❌ 거부 (13.0) | 지원 |
| `$assertoff`, `$asserton`, `$assertkill` | 부분 지원 | 지원 |
| `$assertcontrol` | 제한적 | 제한적 |

> ⚠️ **vitamin 라이브 검증 (iverilog 13.0):** iverilog 13.0은 concurrent assertion과
> `$past`/`$rose`/`$fell`/`$stable`을 **모두 거부**한다('not supported'/'not defined') —
> 차분 오라클이 없어 vitamin은 이 함수들을 **hand-IEEE**로 구현한다(검증=합성 prev-reg 등가).
> Verilator는 `--assert` 플래그 없이는 assertion 자체를 무시한다.

---

## 합성 가능성

❌ 전 함수 비합성 — 검증 전용.
`$past`를 참조하는 assertion은 formal verification 도구(SymbiYosys 등)로
등가 검증에도 활용 가능하지만, 합성 netlist에 포함되지 않는다.

---

## 본 프로젝트 구현 메모

- **구현 방식 (2026-06-16)**: `$past`/`$rose`/`$fell`/`$stable`은 Observed-region 훅·circular
  buffer가 아니라 **순수 IR-0 prev-reg desugar**로 구현됐다 — `rewrite_sampled`가 signal당 공유
  full-width `prev` reg를 합성하고 NBA `prev<=sig`로 1-사이클 지연시킨다(`$past(e,n)`=n-스테이지 시프트,
  `$rose`=`~prev[0]&e[0]`·`$fell`=`prev[0]&~e[0]`·`$stable`=`prev===e`·`$changed`=`prev!==e`). hand-IEEE 핀(iverilog 13.0 거부).
  `$sampled`(action-block prev-reg 조회)와 `$assertoff`/`$asserton`/`$assertkill`(전역 fire-gate)도 구현 완료(✅).
  **아래 항목은 미구현 함수(`$assertcontrol`)의 향후 설계 메모다.**
- **샘플링 타이밍**: assertion 평가 엔진은 Observed 리전에서 호출되어야 한다.
  NBA 완료 후 신호 스냅샷을 캡처하는 훅이 필요.
- **$past 이력 버퍼**: 각 concurrent assertion이 참조하는 신호에 대해
  `n` 사이클치 sampled value를 원형 버퍼(circular buffer)로 유지.
- **$rose/$fell/$stable/$changed**: 이전 샘플값 1 사이클분만 있으면 모두 파생 가능.
  `$past(expr, 1)`의 특수 케이스로 구현.
- **$sampled**: action block 실행 전에 캡처된 스냅샷에 대한 조회 — Observed 리전 값을
  Reactive 리전까지 보존하는 별도 snapshot 맵 필요.
- **$assertoff/$asserton/$assertkill**: assertion 인스턴스별 활성 플래그 + 계층 트리 순회.
- **$assertcontrol**: control_type/assertion_type/directive_type 비트마스크를 사용한
  assertion 인스턴스 필터링 후 상태 변경.

## Sources

- IEEE 1800-2017 §16.9 (sampled value functions), §16.12 (assertion control)
- IEEE 1800-2017 §20.12 ($assertoff/$asserton/$assertkill/$assertcontrol)
- research-log: [system-tasks-random-assertion-2026-05-28.md](../../research-log/system-tasks-random-assertion-2026-05-28.md)
- 시뮬레이션 스케줄러 리전 구조: [06-simulation-engine.md](../../06-simulation-engine.md)
- [circuitcove.com — Assertion Control](https://circuitcove.com/system-tasks-assertion/) (WebFetch ✓)
- [circuitcove.com — Sampled Value Functions](https://circuitcove.com/system-tasks-sampled/) (WebFetch ✓)
- [vlsiverify.com — Sample Value Functions](https://vlsiverify.com/system-verilog/assertions/sample-value-functions/) (WebFetch ✓)
- [verificationguide.com — SVA Built-in Methods](https://verificationguide.com/systemverilog/systemverilog-sva-built-in-methods/) (WebFetch ✓)
- [accellera.org sv-bc — $assertcontrol discussion](https://www.accellera.org/images/eda/sv-bc/10917.html) (WebFetch ✓)
