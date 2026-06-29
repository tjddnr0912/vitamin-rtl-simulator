---
title: "SystemVerilog System Functions 조사 — Random · Assertion Sampling (Phase 2)"
date: 2026-05-28
author: research-skill (Claude Sonnet 4.6)
scope:
  - IEEE 1800-2017 §18 (random number generation)
  - IEEE 1800-2017 §16 (SVA sampled value functions)
  - $random / $urandom / $urandom_range / $dist_*
  - $past / $rose / $fell / $stable / $changed / $sampled
  - $assertoff / $asserton / $assertkill / $assertcontrol
  - Icarus Verilog / Verilator 동작 차이
rounds: 2
status: complete
---

# SystemVerilog System Functions 조사 — Random · Assertion Sampling

## 조사 배경

Vitamin RTL 시뮬레이터 `hdl-builtins` 크레이트의 Phase 2 구현을 위해
난수 생성 함수(§18)와 SVA assertion sampling 함수(§16) 두 카테고리의
표준 의미론과 주요 시뮬레이터 동작 차이를 확인한다.

---

## A. 난수 생성 함수

### $random — 전통 Verilog 전역 난수

`$random[(seed)]`는 32-bit **signed** integer를 반환한다.
Verilog-1995 PLI에서 유래한 함수로 IEEE 1800-2017이 아닌 IEEE 1364-2005 §17.9에
뿌리를 두고 있다. seed 인자는 선택적이며 reg/integer/time 타입이어야 한다.

결정적으로, `$random`은 **전역(global) RNG 상태**를 사용한다.
멀티-스레드 테스트벤치에서 어느 스레드가 `$random`을 호출하든 같은 공유 시퀀스에서 값을 꺼낸다.
스레드 생성 순서나 실행 순서가 바뀌면 전체 난수 시퀀스가 달라지므로
재현성(reproducibility)이 깨지기 쉽다.

[출처: circuitcove.com/system-tasks-random/ WebFetch ✓;
hdlworks.com SystemRandomFunctions WebFetch ✓;
systemverilog.io/verification/randomization/ WebFetch ✓]

### $urandom / $urandom_range — IEEE 1800 표준 thread-local 난수

`$urandom[(seed)]`는 32-bit **unsigned** integer를 반환한다.
$random과 외형은 비슷하지만 내부 동작 방식이 근본적으로 다르다.

핵심 차이는 **RNG 상태가 thread-local**이라는 점이다.
IEEE 1800-2017 §18은 각 프로세스(thread)가 독립적인 RNG 인스턴스를 보유하도록 요구한다.
새 dynamic thread가 생성될 때 그 thread의 RNG 초기 seed는 **부모 thread의 RNG next value**로
결정된다(hierarchical seeding). 이 방식 덕분에 특정 최상위 seed 값을 고정하면
멀티-스레드 테스트벤치 전체의 난수 시퀀스가 결정론적으로 고정된다.

`$urandom_range(maxval [, minval=0])`은 `[minval, maxval]` 범위의
unsigned 정수를 반환한다. 두 경계값 모두 포함(inclusive)이다.
minval의 기본값은 0이므로 `$urandom_range(15)`는 0~15 범위를 반환한다.

[출처: verificationguide.com random-system-methods WebFetch ✓;
verificationacademy.com 포럼 thread-RNG WebFetch ✓;
systemverilog.io/verification/random-stability/ WebFetch ✓]

### RNG 알고리즘 — Annex N과 cross-simulator 재현성

IEEE 1800-2017 Annex N에는 $dist_* 계열 함수의 C 소스 코드가 수록되어 있다.
이 알고리즘이 표준화되어 있기 때문에 $dist_* 계열은 도구(VCS, QuestaSim 등) 간에
동일한 시드에 대해 동일한 시퀀스를 생성하는 것이 보장된다.

반면 $urandom의 RNG 알고리즘(LCG인지 Mersenne Twister인지 등)은
IEEE 1800-2017이 명시적으로 구현에 위임(implementation defined)했다.
Verification Academy에서 활동하는 EDA 전문가에 따르면 "도구 간 $urandom 수열의
동일성은 표준이 보장하지 않으며 같은 도구의 버전이 바뀌어도 달라질 수 있다."
재현이 필요할 때는 동일 도구 + 동일 버전 + 동일 seed를 유지해야 한다.

[출처: systemverilog.io/verification/randomization/ — Annex N 언급 ✓;
verificationacademy.com 포럼 — "LRM specifically left that to the implementation" ✓]

### $dist_* 함수군 — 확률 분포 난수

모든 $dist_* 함수는 Verilog 1364에서 유래한 함수들로, seed를 **inout 파라미터**로 받는다.
호출마다 seed 값이 내부적으로 갱신되어 다음 호출에 전달된다.
seed 변수는 반드시 integer 타입이어야 하며, 첫 호출 전에 초기값을 설정해 두면
결정론적 시퀀스가 보장된다.

반환값은 모두 integer다.

| 함수 | 시그니처 | 분포 |
|------|---------|------|
| `$dist_uniform` | `(seed, start, end)` | 균등 분포 [start, end] |
| `$dist_normal` | `(seed, mean, std_deviation)` | 정규(가우시안) 분포 |
| `$dist_exponential` | `(seed, mean)` | 지수 분포 |
| `$dist_poisson` | `(seed, mean)` | 포아송 분포 |
| `$dist_chi_square` | `(seed, degree_of_freedom)` | 카이제곱 분포 |
| `$dist_t` | `(seed, degree_of_freedom)` | t 분포 |
| `$dist_erlang` | `(seed, k_stage, mean)` | 얼랑 분포 |

[출처: peterfab.com dist functions WebFetch ✓;
hdlworks.com WebFetch ✓;
circuitcove.com/system-tasks-random/ WebFetch ✓]

### Icarus / Verilator 지원

| 함수 | Icarus Verilog | Verilator |
|------|---------------|-----------|
| `$random` | 완전 지원 | 지원 |
| `$urandom`, `$urandom_range` | 완전 지원 | 지원 |
| `$dist_*` | 지원 | 지원 (일부 제한 가능) |

공개 자료에서 Verilator의 $dist_* 지원 여부에 대한 명시적 확인은 부족하다(공개 자료 한계).
Icarus는 PLI 유래 $dist_* 함수를 포함해 완전 지원으로 알려져 있다.

---

## B. Assertion Sampling 함수

### Observed 리전과 샘플링 의미론

SVA concurrent assertion의 sampling 타이밍은 시뮬레이션 스케줄러 리전 구조와 직결된다.
관련 상세는 [06-simulation-engine.md](../hdl-reference/system-tasks/06-simulation-engine.md)에서 다루고,
여기서는 핵심만 정리한다.

클록 에지가 발생하면 Active → Inactive → NBA 리전에서 설계 로직이 실행되어
신호 값이 확정된다. 그 후 **Observed 리전**에서 concurrent assertion이 평가된다.
assertion이 보는 값은 이 시점에 이미 안정된 "샘플링된 값"이다.
이 구분이 중요한 이유는 assertion 내부에서 신호를 직접 참조하든 `$past(sig)`로 참조하든
모두 이 Observed 리전 기준의 샘플값이기 때문이다.

[출처: vlsiverify.com sample-value-functions WebFetch ✓;
circuitcove.com/system-tasks-sampled/ WebFetch ✓]

### $past / $rose / $fell / $stable / $changed / $sampled 정의

**$past(expr [, n [, enable_expr [, @clocking_event]]])**
- n 클록 사이클 전의 샘플값을 반환. n 기본값 = 1.
- enable_expr: 이 조건이 참인 클록 에지만 "틱"으로 카운트한다.
- @clocking_event: 생략 시 assertion의 clocking event를 상속.

**$rose(expr)**
- 이전 클록 에지 샘플 → 현재 클록 에지 샘플에서
  LSB가 (0/x/z) → 1 로 변했으면 true. bit 타입 반환.
- 등가식: `$past(expr[0]) !== 1'b1 && expr[0] === 1'b1`

**$fell(expr)**
- LSB가 (1/x/z) → 0 으로 변했으면 true.
- 등가식: `$past(expr[0]) !== 1'b0 && expr[0] === 1'b0`

**$stable(expr)**
- 현재 샘플값 == 이전 샘플값이면 true.
- 등가식: `$past(expr) === expr`

**$changed(expr)**
- 값이 바뀌었으면 true. $stable의 반대.
- 등가식: `!$stable(expr)` 또는 `$past(expr) !== expr`

**$sampled(expr)**
- assertion이 평가되는 Observed 리전 시점의 샘플값을 반환.
- assertion property body 안에서는 신호가 이미 샘플링된 값으로 보이므로 $sampled는 중복이다.
- 필요한 경우: action block (pass/fail 블록) 안에서 메시지를 출력할 때.
  action block은 Reactive 리전에서 실행되므로, 이 시점에 신호가 Active 리전에서
  다시 변동될 수 있다. `$display("val=%0d", $sampled(sig))`처럼 쓰면
  Observed 리전 기준값을 안정적으로 출력한다.

[출처: vlsiverify.com WebFetch ✓;
circuitcove.com/system-tasks-sampled/ WebFetch ✓;
verificationguide.com SVA built-in methods WebFetch ✓]

### $assertoff / $asserton / $assertkill

세 태스크는 런타임에 assertion의 활성 상태를 제어한다.
reset 구간 또는 초기화 시퀀스처럼 assertion이 false를 밷을 것이 확실한 구간에서
노이즈 제거 목적으로 흔히 사용한다.

```sv
// 시그니처 (IEEE 1800-2017 §20.12)
$assertoff  [(levels [, list_of_scopes])]
$asserton   [(levels [, list_of_scopes])]
$assertkill [(levels [, list_of_scopes])]
```

- `$assertoff`: 현재 진행 중인 assertion은 완료하도록 두고 그 이후 assertion을 비활성화.
- `$asserton`: 비활성화된 assertion을 다시 활성화.
- `$assertkill`: 진행 중인 assertion 포함 즉시 종료(kill).
- `levels`: 0 = 모든 계층, 1 = 지정 모듈만, n = n 레벨 하위까지.
- scope 인자 없으면 전체 설계에 적용.

[출처: circuitcove.com/system-tasks-assertion/ WebFetch ✓;
semiwiki.com assertion control WebFetch partial ✓]

### $assertcontrol

$assertoff/$asserton/$assertkill보다 세밀한 제어가 필요할 때 사용한다.

```sv
$assertcontrol(control_type
               [, [assertion_type]
               [, [directive_type]
               [, [levels]
               [, list_of_modules_or_assertions]]]]])
```

**control_type** (integer):

| 값 | 이름 | 의미 |
|----|------|------|
| 1 | Lock | assertion 상태 잠금 |
| 2 | Unlock | 잠금 해제 |
| 3 | On | 활성화 (`$asserton` 동등) |
| 4 | Off | 비활성화 (`$assertoff` 동등) |
| 5 | Kill | 즉시 kill (`$assertkill` 동등) |
| 6 | PassOn | pass 액션 블록 실행 활성화 |
| 7 | PassOff | pass 액션 블록 실행 비활성화 |
| 8 | FailOn | fail 액션 블록 실행 활성화 |
| 9 | FailOff | fail 액션 블록 실행 비활성화 |
| 10 | NonvacuousOn | vacuous success 제외 pass만 활성화 |
| 11 | VacuousOff | vacuous pass 비활성화 |

**assertion_type** (bitmask, OR 조합 가능):
1=Concurrent, 2=SimpleImmediate, 4=ObservedDeferredImmediate,
8=FinalDeferredImmediate, 16=Expect, 32=Unique, 64=Unique0, 128=Priority

**directive_type** (bitmask): 1=assert, 2=cover, 4=assume

사용 예: `$assertcontrol(4, 2, 1)` — SimpleImmediate assert 디렉티브를 Off(4).

[출처: circuitcove.com/system-tasks-assertion/ WebFetch ✓;
accellera.org sv-bc $assertcontrol 논의 WebFetch ✓]

---

## Sources

1. **circuitcove.com — Random Functions** — https://circuitcove.com/system-tasks-random/ (WebFetch Round 1 ✓)
2. **circuitcove.com — Assertion Control** — https://circuitcove.com/system-tasks-assertion/ (WebFetch Round 2 ✓)
3. **circuitcove.com — Sampled Value Functions** — https://circuitcove.com/system-tasks-sampled/ (WebFetch Round 2 ✓)
4. **hdlworks.com — System Random Functions** — https://www.hdlworks.com/hdl_corner/verilog_ref/items/SystemRandomFunctions.htm (WebFetch Round 1 ✓)
5. **peterfab.com — Probabilistic Distribution Functions** — https://peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00035.htm (WebFetch Round 1 ✓)
6. **verificationguide.com — Random System Methods** — https://verificationguide.com/systemverilog/systemverilog-random-system-methods/ (WebFetch Round 1 ✓)
7. **verificationguide.com — SVA Built-in Methods** — https://verificationguide.com/systemverilog/systemverilog-sva-built-in-methods/ (WebFetch Round 2 ✓)
8. **vlsiverify.com — Sample Value Functions** — https://vlsiverify.com/system-verilog/assertions/sample-value-functions/ (WebFetch Round 1 ✓)
9. **systemverilog.io — Randomization** — https://www.systemverilog.io/verification/randomization/ (WebFetch Round 2 ✓)
10. **systemverilog.io — Random Stability** — https://www.systemverilog.io/verification/random-stability/ (WebFetch Round 2 ✓)
11. **verificationacademy.com — Thread RNG forum** — https://verificationacademy.com/forums/t/random-stability-question-how-is-the-thread-rng-used-for-running-urandom-urandom-range-array-shuffle-and-randcase/33904 (WebFetch Round 1 ✓)
12. **accellera.org sv-bc — $assertcontrol naming proposal** — https://www.accellera.org/images/eda/sv-bc/10917.html (WebFetch Round 2 ✓)
13. **IEEE 1800-2017 §18** (random), **§16** (SVA), **§20.12** (assertion control tasks) — 2차 소스 교차 확인 (직접 fetch 불가)
