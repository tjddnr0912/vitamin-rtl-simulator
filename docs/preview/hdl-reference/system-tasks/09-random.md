# 09 · Random Functions

## 개요

SystemVerilog는 두 세대의 난수 생성 인터페이스를 제공한다.
Verilog-1995 PLI에서 유래한 `$random`/`$dist_*` 계열과,
IEEE 1800-2017 §18에서 정의한 `$urandom`/`$urandom_range` 계열이다.
두 세대는 부호(signed vs unsigned), RNG 상태 범위(전역 vs thread-local),
seed 처리 방식에서 차이가 있어 혼용 시 주의가 필요하다.

## 지원 Phase

- ✅ **구현됨 (2026-06-12, format_version 7)**: `$random`, `$urandom`, `$urandom_range`.
  `$random`=IEEE 1364 Annex N(69069 LCG, signed 32-bit, 시드형은 ref-VAR 쓰기 → blocking-assign 직접 rhs statement-level 인터셉트)·`$urandom`/`$urandom_range`=splitmix64 자체 계약(unsigned, 3-OS 결정성, `$urandom_range`는 arg 자동 스왑).
- ⏳ **미구현 (loud-reject E3009)**: `$dist_uniform`, `$dist_normal`, `$dist_exponential`, `$dist_poisson`, `$dist_chi_square`, `$dist_t`, `$dist_erlang` — `SysFuncId`에 미배선이라 elaborate가 거부한다(Annex N inout-seed 포팅은 향후 과제).

---

## 항목 상세

### 전체 함수 목록

| 함수 | 시그니처 | 반환 타입 | 세대 |
|------|---------|---------|------|
| `$random` | `[$random[(seed)]]` | 32-bit signed int | Verilog-1995 |
| `$urandom` | `[$urandom[(seed)]]` | 32-bit unsigned int | IEEE 1800 |
| `$urandom_range` | `$urandom_range(max [, min])` | unsigned int | IEEE 1800 |
| `$dist_uniform` | `$dist_uniform(seed, start, end)` | integer | Verilog-1995 |
| `$dist_normal` | `$dist_normal(seed, mean, std_dev)` | integer | Verilog-1995 |
| `$dist_exponential` | `$dist_exponential(seed, mean)` | integer | Verilog-1995 |
| `$dist_poisson` | `$dist_poisson(seed, mean)` | integer | Verilog-1995 |
| `$dist_chi_square` | `$dist_chi_square(seed, degree)` | integer | Verilog-1995 |
| `$dist_t` | `$dist_t(seed, degree)` | integer | Verilog-1995 |
| `$dist_erlang` | `$dist_erlang(seed, k_stage, mean)` | integer | Verilog-1995 |

---

### `$random` — 전역 RNG, 32-bit signed

- **시그니처**: `$random [(seed)]`
- **표준**: IEEE 1364-2005 §17.9 (SV에서 상속)
- **반환**: 32-bit signed integer (음수 포함)
- **seed**: 선택적. reg / integer / time 변수여야 한다.

```sv
integer r;

r = $random;          // seed 없이 호출 — 시뮬레이터 전역 RNG 사용
r = $random(42);      // seed=42로 RNG 상태 초기화 후 첫 번째 값 반환

// 음수가 나올 수 있으므로 양수 범위가 필요하면 abs 처리 필요
r = $random % 100;    // -99 ~ 99 범위 (음수 포함!)
r = {$random} % 100;  // 32-bit unsigned 해석 후 mod → 0~99 (양수만)
```

**주의 — 전역 RNG 상태**: `$random`은 시뮬레이션 전체에 하나인 전역 RNG를 공유한다.
멀티-스레드 테스트벤치에서 어느 스레드에서 `$random`을 호출하느냐에 따라
다른 스레드가 받는 값도 바뀐다. 스레드 생성 순서나 스케줄 순서가 달라지면
전체 난수 시퀀스가 달라져 재현이 어렵다.

이 문제 때문에 IEEE 1800에서는 `$urandom`이 도입되었다.
새 코드에서는 가능하면 `$urandom` 계열을 사용하는 것이 권장된다.

---

### `$urandom` — thread-local RNG, 32-bit unsigned

- **시그니처**: `$urandom [(seed)]`
- **표준**: IEEE 1800-2017 §18
- **반환**: 32-bit unsigned integer (항상 0 이상)
- **seed**: 선택적. 지정하면 해당 thread의 RNG를 재초기화(re-seed)한다.

```sv
logic [31:0] u;

u = $urandom;         // 현재 thread의 RNG에서 값 추출
u = $urandom(99);     // 이 thread의 RNG를 seed=99로 재초기화
```

**thread-local RNG 모델**: IEEE 1800-2017은 각 프로세스(initial/always 블록,
fork 블록 내 각 스레드, task/function 콜 체인)가 **독립적인 RNG 인스턴스**를
보유하도록 규정한다.

새 dynamic thread(`fork ... join_none` 등)가 생성될 때 그 thread의 초기 seed는
부모 thread RNG의 다음 값(next generated value)으로 결정된다.
이를 **hierarchical seeding**이라 한다.
최상위 seed 하나를 고정하면 전체 계층 구조의 난수 시퀀스가 결정론적으로 고정된다.

```sv
// command-line로 전체 seed 고정 예시
// Synopsys VCS:       +ntb_random_seed=42
// Mentor QuestaSim:  -sv_seed 42
```

**RNG 알고리즘과 cross-simulator 주의**: IEEE 1800-2017은 `$urandom` RNG의 구체적
알고리즘을 implementation defined로 남겼다. 도구마다 알고리즘이 다를 수 있어
동일 seed라도 VCS와 QuestaSim이 다른 시퀀스를 생성할 수 있다.
시퀀스의 통계적 특성(편향 없는 uniform distribution)은 보장되지만,
특정 값의 일치는 보장되지 않는다.

---

### `$urandom_range` — 범위 지정 uniform 난수

- **시그니처**: `$urandom_range(max [, min = 0])`
- **표준**: IEEE 1800-2017 §18
- **반환**: `[min, max]` 범위의 unsigned integer (양 끝 포함)

```sv
int unsigned v;

v = $urandom_range(7);       // 0 ~ 7 (min 생략 → 0)
v = $urandom_range(9, 1);    // 1 ~ 9
v = $urandom_range(15, 8);   // 8 ~ 15

// max < min 이면 구현 정의 동작 — 인자 순서 주의
// ($urandom_range(3, 10) → 도구마다 다를 수 있음)
```

`$urandom_range`는 `$urandom`과 같은 thread-local RNG를 사용한다.

---

### `$dist_*` 계열 — 확률 분포 난수 ⏳ 미구현 (loud-reject)

> **vitamin 구현 상태**: `$dist_uniform`/`$dist_normal`/`$dist_exponential`/`$dist_poisson`/`$dist_chi_square`/`$dist_t`/`$dist_erlang`는
> 아직 `SysFuncId`에 배선되지 않아 elaborate가 `E3009 "unsupported system function"`으로 거부한다.
> 아래는 IEEE 의미론 참조 문서다. (`$random`/`$urandom`/`$urandom_range`는 구현 완료.)

$dist_* 함수들은 IEEE 1364-2005 §17.9 기반 PLI 함수다.
IEEE 1800-2017 Annex N에 알고리즘의 C 소스 코드가 수록되어 있어
표준을 준수하는 시뮬레이터라면 **동일한 seed에 대해 도구 간 동일한 시퀀스**를 생성한다.
($urandom과 달리 cross-simulator 재현성이 보장된다.)

**seed 파라미터 — inout 동작**: 모든 $dist_* 함수는 seed를 inout으로 처리한다.
호출마다 seed 값이 내부적으로 갱신되며, 갱신된 값이 다음 호출의 입력 seed가 된다.
seed 변수는 **integer** 타입이어야 한다.

```sv
integer seed = 12345;  // 초기 seed — integer 타입 필수
integer r;

r = $dist_uniform(seed, 0, 100);   // 0~100 균등 정수, seed가 갱신됨
r = $dist_uniform(seed, 0, 100);   // 다른 값 (seed가 이미 바뀜)
```

---

#### `$dist_uniform`

- **시그니처**: `$dist_uniform(seed, start, end)` → integer
- **의미**: `[start, end]` 범위의 균등 분포 정수. start ≤ 반환값 ≤ end.
- **참고**: `$random`은 내부적으로 `$dist_uniform`의 특수 케이스로 구현 가능.

```sv
integer seed = 1;
integer r;

r = $dist_uniform(seed, 1, 6);    // 주사위 시뮬레이션 (1~6)
r = $dist_uniform(seed, -10, 10); // 음수 범위도 가능
```

---

#### `$dist_normal`

- **시그니처**: `$dist_normal(seed, mean, std_deviation)` → integer
- **의미**: 평균 `mean`, 표준편차 `std_deviation`의 정규(가우시안) 분포 정수.
- **주의**: 반환값이 integer이므로 분포 꼬리 쪽 값이 절사된다.
  실수 정밀도가 필요한 경우 이 함수는 적합하지 않다.

```sv
integer seed = 777;
integer r;

r = $dist_normal(seed, 100, 15);  // 평균 100, 표준편차 15의 정수
```

---

#### `$dist_exponential`

- **시그니처**: `$dist_exponential(seed, mean)` → integer
- **의미**: 평균 `mean`의 지수 분포 정수.
  통신/큐잉 시뮬레이션에서 도착 간격(inter-arrival time) 모델링에 사용.

```sv
integer seed = 3;
integer r;

r = $dist_exponential(seed, 10);  // 평균 10의 지수 분포
```

---

#### `$dist_poisson`

- **시그니처**: `$dist_poisson(seed, mean)` → integer
- **의미**: 평균 `mean`의 포아송 분포 정수.
  단위 시간당 이벤트 발생 횟수 모델링에 사용.

```sv
integer seed = 55;
integer r;

r = $dist_poisson(seed, 5);  // 평균 5회 발생
```

---

#### `$dist_chi_square`

- **시그니처**: `$dist_chi_square(seed, degree_of_freedom)` → integer
- **의미**: 자유도(degree of freedom) `k`의 카이제곱(χ²) 분포.
  `k`개의 표준 정규 확률변수의 제곱합 분포.

---

#### `$dist_t`

- **시그니처**: `$dist_t(seed, degree_of_freedom)` → integer
- **의미**: 자유도 `k`의 t 분포(Student's t-distribution).
  소표본 통계 모델링에 사용.

---

#### `$dist_erlang`

- **시그니처**: `$dist_erlang(seed, k_stage, mean)` → integer
- **의미**: k-단계(k-stage) 얼랑 분포. 서비스 큐에서 k단계를 거치는
  대기 시간 모델링에 사용. `k=1`이면 지수 분포와 동일.

---

## $random vs $urandom 요약 비교

| 항목 | `$random` | `$urandom` |
|------|----------|-----------|
| 반환 타입 | 32-bit signed | 32-bit unsigned |
| RNG 상태 범위 | 전역(global) | thread-local |
| 표준 출처 | IEEE 1364-2005 §17.9 | IEEE 1800-2017 §18 |
| 멀티-스레드 재현성 | 취약 | 강건 (hierarchical seeding) |
| cross-simulator 재현 | 구현 정의 | 구현 정의 |
| seed 인자 | 선택적, reg/int/time | 선택적 |

---

## Icarus / Verilator 지원

| 함수 | Icarus Verilog | Verilator |
|------|---------------|-----------|
| `$random` | 완전 지원 | 지원 |
| `$urandom`, `$urandom_range` | 완전 지원 | 지원 |
| `$dist_*` | 완전 지원 | 지원 (일부 제한 가능, 공개 자료 확인 한계) |

---

## 합성 가능성

❌ 전 함수 비합성 — 시뮬레이션 전용.
RTL 내부에서 난수를 생성해야 하는 경우(LFSR 등)는 별도 회로로 구현한다.

---

## 본 프로젝트 구현 메모

- ✅ `$random` (구현됨): IEEE 1364 Annex N(69069 LCG) + float-mantissa, signed 32-bit. 시드형 `$random(seed)`은 ref-VAR을 쓰는 부작용이라 blocking-assign 직접 rhs일 때만 허용(statement-level 인터셉트), 그 외 위치는 `E3009`(`crates/sim-engine/src/rng.rs`).
- ✅ `$urandom` / `$urandom_range` (구현됨): splitmix64 자체 계약(implementation-defined를 vita가 핀, 3-OS 결정성). `$urandom_range`는 IEEE §18.13.3대로 `max<min` 시 arg 자동 스왑.
- ⏳ `$dist_*` (미구현, loud-reject E3009): Annex N inout-seed 알고리즘을 Rust로 포팅하는 향후 과제. seed inout = seed 변수 mutable reference 전달 설계.
- ℹ️ thread-local RNG / hierarchical seeding: 현재 단일 RNG 상태 기반. 동적 thread별 독립 RNG 인스턴스 + 부모 RNG 시드 파생은 향후 과제.

## Sources

- IEEE 1800-2017 §18 (random number generation), §20.15 ($dist_*)
- IEEE 1364-2005 §17.9 (PLI random functions)
- research-log: [system-tasks-random-assertion-2026-05-28.md](../../research-log/system-tasks-random-assertion-2026-05-28.md)
- [circuitcove.com — Random Functions](https://circuitcove.com/system-tasks-random/) (WebFetch ✓)
- [hdlworks.com — System Random Functions](https://www.hdlworks.com/hdl_corner/verilog_ref/items/SystemRandomFunctions.htm) (WebFetch ✓)
- [peterfab.com — Probabilistic Distribution Functions](https://peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00035.htm) (WebFetch ✓)
- [systemverilog.io — Randomization](https://www.systemverilog.io/verification/randomization/) (WebFetch ✓)
- [systemverilog.io — Random Stability](https://www.systemverilog.io/verification/random-stability/) (WebFetch ✓)
- [verificationacademy.com — Thread RNG forum](https://verificationacademy.com/forums/t/random-stability-question-how-is-the-thread-rng-used-for-running-urandom-urandom-range-array-shuffle-and-randcase/33904) (WebFetch ✓)
