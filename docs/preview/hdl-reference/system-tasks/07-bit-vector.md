# 07 · Bit-Vector Query Functions

## 개요

비트 벡터의 크기, 비트 카운트, one-hot 패턴, 알 수 없는 비트 존재 여부를
조회하는 시스템 함수 카테고리다.
`$bits`와 `$clog2`는 elaboration-time constant로 평가되어 파라미터 선언에 사용 가능하다.
`$countbits`/`$countones`/`$onehot`/`$onehot0`/`$isunknown`은 4-state 값에 대한 런타임 조회 함수다.

## 지원 Phase

- ✅ **구현됨 (2026-06-12, format_version 7)**: `$bits`, `$clog2`, `$countones`, `$onehot`, `$onehot0`, `$isunknown`.
  `$bits`/`$clog2`는 elaboration-time const-fold(`$bits`=array view/ir_bits_of/decl-order prescan 3-레인),
  `$countones`/`$onehot`/`$onehot0`/`$isunknown`은 word-parallel 런타임 조회(`SysFuncId::{CountOnes,OneHot,OneHot0,IsUnknown}`).
- ⏳ **미구현 (loud-reject E3009)**: `$countbits` — `SysFuncId`에 미배선이라 elaborate가 "unsupported system function"으로 거부한다. 등가식 `$countbits(v,1'b1)`은 구현된 `$countones`로, `$countbits(v,1'bx,1'bz)!=0`은 `$isunknown`으로 우회 가능.

---

## 항목 상세

### `$bits`

- **시그니처**: `$bits(type_or_expression)` → `integer`
- **표준**: IEEE 1800-2017 §20.6
- **반환 타입**: `integer` (elaboration-time constant)
- **의미**: 표현식 또는 데이터 타입의 **비트 폭**을 반환한다.
  C의 `sizeof()` 바이트 단위 버전이 아닌, 비트 단위 크기다.

인자로 **type name을 직접** 받을 수 있다는 점이 특징이다.
expression(신호, 변수)도 받는다. 두 경우 모두 elaboration-time constant로 평가된다.

```sv
// type name을 직접 인자로 사용
$bits(logic)              // 1
$bits(logic [7:0])        // 8
$bits(integer)            // 32

typedef struct packed {
    logic [7:0] addr;
    logic [15:0] data;
    logic        valid;
} bus_t;
$bits(bus_t)              // 25 (8+16+1)

// expression으로 사용
logic [3:0] nibble;
$bits(nibble)             // 4

// 파라미터 선언에 활용
parameter int DATA_W = 32;
logic [$bits(logic [DATA_W-1:0])-1:0] buffer;  // DATA_W 비트 폭 buffer
```

**배열**: packed 배열은 전체 비트 수를 반환한다. unpacked 배열의 동작은 시뮬레이터 정의.

---

### `$clog2`

- **시그니처**: `$clog2(unsigned_val)` → `integer`
- **표준**: IEEE 1800-2017 §20.8.1 (integer math functions 섹션에 위치)
- **반환 타입**: `integer` (signed — IEEE Verilog 2005 §17.11.1 명시)
- **의미**: `ceiling(log₂(N))`. N개 항목을 binary index로 표현할 때 필요한 **최소 비트 수**다.
  인자는 unsigned 값으로 처리된다.

| 입력 N | `$clog2(N)` | 비고 |
|--------|-------------|------|
| 0 | **0** | IEEE 1800-2017 §20.8.1 명시: "argument value of 0 shall produce a result of 0" |
| 1 | **0** | log₂(1) = 0, ceiling(0) = 0 |
| 2 | 1 | |
| 3 | 2 | log₂(3) ≈ 1.585 → ceiling = 2 |
| 4 | 2 | log₂(4) = 2 (정확한 2의 거듭제곱) |
| 5~8 | 3 | |
| 100 | 7 | 2⁷ = 128 > 100 |
| 1024 | 10 | 정확한 2의 거듭제곱 → log = ceiling |

```sv
// 메모리 주소 버스 폭 계산
parameter int MEM_DEPTH = 256;
logic [$clog2(MEM_DEPTH)-1:0] addr;   // [7:0] — 8비트

// FIFO depth → pointer 폭
parameter int FIFO_DEPTH = 100;
logic [$clog2(FIFO_DEPTH):0] wr_ptr;  // +1 for full/empty distinguish

// $clog2(1) = 0 edge case: 1-entry 메모리는 주소 0비트 필요
parameter int SINGLE = 1;
// $clog2(SINGLE) = 0 → [0-1:0] = [-1:0] — 시뮬레이터마다 0비트 wire로 처리되거나 오류
// 방어 패턴: $clog2(DEPTH > 1 ? DEPTH : 2)
```

**반환 타입 논쟁**: IEEE 표준은 `integer`(signed)를 명시하지만 Yosys 등 일부 도구는 unsigned를 반환한다.
`$clog2(N) - 1` 같은 빼기 연산을 signed 컨텍스트 없이 사용하면 N=1일 때 underflow가 발생할 수 있다.
안전한 패턴: `int'($clog2(N)) - 1` 또는 `$clog2(N) > 0 ? $clog2(N) - 1 : 0`.

---

### `$countones`

- **시그니처**: `$countones(expression)` → `int`
- **표준**: IEEE 1800-2017 §20.9
- **반환 타입**: `int` (32-bit signed integer)
- **의미**: 표현식 내 **1-valued 비트의 개수**를 반환한다.
  `$countbits(expression, 1'b1)`의 shorthand.
  x 또는 z 비트는 카운트에 **포함되지 않는다** (1이 아니므로).

```sv
logic [7:0] v = 8'b1010_1100;
$countones(v)            // 4 (비트 7,5,3,2가 1)

logic [3:0] x_val = 4'b1x01;
$countones(x_val)        // 2 (비트 3, 0이 1; 비트 2는 x → 카운트 제외)

// Hamming weight 활용
logic [15:0] data;
int weight;
always_comb weight = $countones(data);
```

---

### `$countbits` ⏳ 미구현 (loud-reject)

> **vitamin 구현 상태**: `$countbits`는 아직 `SysFuncId`에 배선되지 않아 elaborate가
> `E3009 "unsupported system function"`으로 거부한다. 아래는 IEEE 의미론 참조 문서다.
> 우회: `$countbits(v,1'b1)`=`$countones(v)`(구현됨), `$countbits(v,1'bx,1'bz)!=0`=`$isunknown(v)`(구현됨).

- **시그니처**: `$countbits(expression, control_bit [, control_bit ...])` → `int`
- **표준**: IEEE 1800-2017 §20.9
- **반환 타입**: `int`
- **의미**: 표현식 내에서 지정한 비트 값(0, 1, x, z)과 **일치하는 비트의 개수**를 반환한다.
  control_bit는 1-bit logic 값이며 여러 개를 나열하면 합산한다.
  4-state 벡터의 x/z 상태를 완전히 다룰 수 있다.

```sv
logic [7:0] v = 8'b1010_xxzz;

$countbits(v, 1'b1)      // 2 (비트 7, 5)
$countbits(v, 1'b0)      // 2 (비트 4, 3 — x,z는 해당 없음)
$countbits(v, 1'bx)      // 2 (비트 2, 1)
$countbits(v, 1'bz)      // 2 (비트 1, 0 — 참고: 위 예시 z는 비트 1,0)

// 여러 control_bit 합산
$countbits(v, 1'bx, 1'bz)  // 4 (x+z 비트 수 합산)

// $isunknown과 등가
$countbits(v, 1'bx, 1'bz) != 0   // $isunknown(v)와 동일한 결과

// 4-state 진단: 전체 비트 수 검증
logic [3:0] w = 4'b10xz;
// $countbits(w, 0) + $countbits(w, 1) + $countbits(w, 'x) + $countbits(w, 'z) == 4
```

---

### `$onehot`

- **시그니처**: `$onehot(expression)` → `bit`
- **표준**: IEEE 1800-2017 §20.9
- **반환 타입**: `bit` (0 또는 1)
- **의미**: 벡터 내에서 **정확히 하나**의 비트만 1이면 1을 반환한다.
  FSM one-hot 인코딩 검증, 어서션에 주로 사용된다.

x/z 비트 처리: x 또는 z가 포함된 경우 명확하게 one-hot인지 판단할 수 없어 대부분 시뮬레이터는 0을 반환한다.
IEEE 1800-2017은 이 경우를 명시하지 않아 구현 정의(implementation defined) 영역이다.

```sv
logic [3:0] fsm_state;

// FSM one-hot 어서션 패턴
always_ff @(posedge clk) begin
    assert ($onehot(fsm_state)) else
        $error("FSM not one-hot: %b", fsm_state);
end

$onehot(4'b0001)    // 1 (비트 0만 1)
$onehot(4'b0100)    // 1 (비트 2만 1)
$onehot(4'b0000)    // 0 (1인 비트 없음)
$onehot(4'b0011)    // 0 (두 비트가 1)
$onehot(4'b01x1)    // 0 또는 x (구현 정의)
```

---

### `$onehot0`

- **시그니처**: `$onehot0(expression)` → `bit`
- **표준**: IEEE 1800-2017 §20.9
- **반환 타입**: `bit`
- **의미**: **최대 하나**의 비트만 1이면 1을 반환한다.
  모두 0인 경우도 1을 반환한다는 점에서 `$onehot`과 다르다.
  "one-hot or all-zero" 패턴 검증.

```sv
$onehot0(4'b0001)   // 1 (1인 비트 하나)
$onehot0(4'b0000)   // 1 (1인 비트 없음 — 허용)
$onehot0(4'b0011)   // 0 (두 비트가 1 — 위반)
```

| 패턴 | `$onehot` | `$onehot0` |
|------|-----------|------------|
| all-zero (0000) | 0 | **1** |
| one-hot (0010) | 1 | 1 |
| two-hot (0110) | 0 | 0 |

---

### `$isunknown`

- **시그니처**: `$isunknown(expression)` → `bit`
- **표준**: IEEE 1800-2017 §20.9
- **반환 타입**: `bit`
- **의미**: 표현식의 비트 중 **x 또는 z가 하나라도 있으면** 1을 반환한다.
  등가식: `$countbits(e, 1'bx, 1'bz) != 0`.
  4-state 신호의 유효성 확인, 어서션 선행 조건으로 자주 쓰인다.

```sv
logic [3:0] sig;

// 어서션 전에 유효성 확인
always @(posedge clk) begin
    if (!$isunknown(sig))
        assert (sig != 4'd0) else $error("zero value");
end

$isunknown(4'b0000)    // 0 (모두 known)
$isunknown(4'b1010)    // 0
$isunknown(4'b10x0)    // 1 (x 포함)
$isunknown(4'b10z0)    // 1 (z 포함)
$isunknown(4'bxxxx)    // 1
```

---

## $countbits vs $countones vs $isunknown 관계

```
$countones(v)            ≡ $countbits(v, 1'b1)
$isunknown(v)            ≡ $countbits(v, 1'bx, 1'bz) != 0
$onehot(v)               ≡ $countbits(v, 1'b1) == 1
$onehot0(v)              ≡ $countbits(v, 1'b1) <= 1
```

---

## Icarus / Verilator 지원

| 함수 | Icarus | Verilator |
|------|--------|-----------|
| `$bits` | 완전 지원 | Generally supported |
| `$clog2` | 완전 지원 | Generally supported |
| `$countones` | 완전 지원 | Generally supported |
| `$countbits` | 완전 지원 | Generally supported |
| `$onehot` | 완전 지원 | Generally supported |
| `$onehot0` | 완전 지원 | Generally supported |
| `$isunknown` | 완전 지원 | Generally supported |

---

## 합성 가능성

| 함수 | 합성 |
|------|------|
| `$bits`, `$clog2` | ✅ — elaboration-time constant, 합성 가능 |
| `$countones` | ✅ — 대부분 합성 도구 지원 |
| `$countbits`, `$onehot`, `$onehot0` | ⚠️ — 도구/버전 의존 (FPGA 도구는 보통 지원) |
| `$isunknown` | ❌ — x/z는 시뮬레이션 전용 상태 (합성 시 제거) |

---

## 본 프로젝트 구현 메모

- `hdl-builtins` 크레이트 `bit_vector` 카테고리 담당
- `$bits`: elaboration-time type/expr 조회. 타입 시스템에서 직접 width 추출.
- `$clog2`: Rust `usize::BITS - N.leading_zeros()` 또는 별도 ceiling_log2 구현.
  반환 타입: signed integer (IEEE 표준 준수). edge case: `$clog2(0) = 0`, `$clog2(1) = 0`.
- `$countones`: Rust `u64::count_ones()` (2-state 환경) / 4-state는 1-bit 슬라이스 순회.
- `$countbits`: 4-state 비트별 순회, control_bit 집합과 일치 여부 카운트.
- `$onehot`: word-parallel — 1-bit가 정확히 1개(`SysFuncId::OneHot`).
- `$onehot0`: word-parallel — 1-bit가 최대 1개(`SysFuncId::OneHot0`).
- `$isunknown`: 4-state 벡터에서 x 또는 z 비트 존재 여부 확인(`SysFuncId::IsUnknown`).
- `$countbits`: ⏳ 미구현(loud-reject E3009) — `SysFuncId` 미배선. 향후 4-state 비트별 순회로 추가 예정.

## Sources

- IEEE 1800-2017 §20.6 ($bits), §20.8.1 ($clog2), §20.9 (count/onehot/isunknown)
- research-log: [system-tasks-conversion-math-2026-05-28.md](../../research-log/system-tasks-conversion-math-2026-05-28.md)
- [circuitcove.com — Bit Vector Functions](https://circuitcove.com/system-tasks-vector/) (WebFetch ✓)
- [circuitcove.com — $clog2](https://circuitcove.com/system-tasks-clog2/) (WebFetch ✓)
- [systemverilog.io — Ten Utilities](https://www.systemverilog.io/verification/ten-utilities/) (WebFetch ✓)
- [YosysHQ/yosys issue #708 — $clog2 return type](https://github.com/YosysHQ/yosys/issues/708) (WebFetch ✓)
- [chipverify.com — Verilog Math Functions](https://chipverify.com/verilog/verilog-math-functions)
- [verilator.org — Input Languages](https://verilator.org/guide/latest/languages.html)
