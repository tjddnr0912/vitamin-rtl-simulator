# 06 · Conversion Functions

## 개요

정수와 실수, 부호와 무부호 간 변환을 담당하는 시스템 함수 카테고리다.
`$signed`/`$unsigned`는 HDL 산술의 부호 해석을 제어하고,
`$rtoi`/`$itor`는 정수-실수 값 변환을,
`$realtobits`/`$bitstoreal`는 IEEE 754 비트 패턴의 포트 통과에 쓰인다.

## 지원 Phase (vitamin 구현 상태)

- **✅ 구현됨 (Phase-2/v7)**: `$signed`, `$unsigned`, `$rtoi`, `$itor`, `$realtobits`, `$bitstoreal`
  (전부 `map_sysfunc` 등록).
- **미구현 (silent-degrade)**: `$shortrealtobits`, `$bitstoshortreal`(shortreal↔32-bit) — 미매핑
  → WARN + skip. 아래 본문 언급은 IEEE 표준 레퍼런스다.

---

## 항목 상세

### `$signed`

- **시그니처**: `$signed(expression)`
- **표준**: IEEE 1800-2017 §20.9 / IEEE 1364-2005 §17.10 (Verilog 2005 이후 포함)
- **반환 타입**: 입력과 **동일한 비트 폭**의 signed 값 — 비트 패턴 불변
- **의미**: 표현식의 비트 폭을 바꾸지 않고 해석만 signed(2의 보수)로 전환한다.
  `$signed` 자체가 비트를 추가하거나 제거하지 않는다.

부호 확장(sign extension)은 함수 자체가 아니라 **더 넓은 대입 타겟**과의 만남에서 발생한다.
`$signed(a)`를 더 넓은 와이어에 연결할 때, SystemVerilog 대입 규칙이 MSB를 복제하여 sign extension을 적용한다.

MSB가 x 또는 z인 경우, 더 넓은 타겟에 대입하면 x 또는 z로 sign extension된다.

```sv
logic [7:0] u = 8'hFF;         // unsigned → 255
logic signed [7:0] s;
logic signed [15:0] wide;

s    = $signed(u);              // -1 (8비트 2의 보수 해석)
wide = $signed(u);              // -1 (sign extension: 16'hFFFF)
// wide의 상위 8비트는 u의 MSB(1)으로 채워짐

// x 전파 예시
logic [3:0] x_val = 4'b1xxx;
logic signed [7:0] ext;
ext = $signed(x_val);           // ext = 8'b1xxx_xxxx (x로 sign extension)
```

**흔한 실수**: 같은 폭의 대입에서 `$signed`를 사용하면 비트 패턴이 동일하므로 아무 효과 없다.
산술 연산(`+`, `-`, `*`, `>>`) 문맥에서 비교 대상 표현식이 unsigned일 때 캐스트 목적으로 사용한다.

---

### `$unsigned`

- **시그니처**: `$unsigned(expression)`
- **표준**: IEEE 1800-2017 §20.9
- **반환 타입**: 입력과 **동일한 비트 폭**의 unsigned 값
- **의미**: 비트 패턴을 유지하면서 unsigned로 재해석한다.
  더 넓은 대입 타겟에 연결하면 **zero extension**이 적용된다.

```sv
logic signed [7:0] s = -1;      // 8'hFF, MSB=1
logic [15:0] u;

u = $unsigned(s);               // 16'h00FF (zero extension, MSB 1이 부호로 보이지 않음)
// 비교: $signed 대입이면 u = 16'hFFFF

// 비교 맥락 활용
if ($unsigned(s) > 8'd200)  // s를 255로 해석, 200 초과 → true
    $display("unsigned 비교");
```

---

### `$rtoi`

- **시그니처**: `$rtoi(real_val)` → `integer`
- **표준**: IEEE 1800-2017 §20.9.1 / IEEE 1364-2005 §17.10
- **반환 타입**: `integer` (32-bit signed)
- **의미**: 실수를 정수로 변환한다. **향0 방향 절사(truncation toward zero)**를 사용하며 반올림하지 않는다.

| 입력 `real_val` | `$rtoi` 결과 | 설명 |
|----------------|-------------|------|
| 192.15 | 192 | 소수 버림 |
| 7.9 | 7 | 반올림 아님 |
| -3.9 | -3 | 향0 방향: -4가 아니라 -3 |
| -0.1 | 0 | |

```sv
real r = 7.8;
integer i;
i = $rtoi(r);   // i = 7 (7.8의 소수점 이하 버림)

r = -3.9;
i = $rtoi(r);   // i = -3 (절사 방향: 0을 향해, -4가 아님)
```

**주의**: C의 `(int)` 캐스트와 동일한 truncation 방향이다.
반올림이 필요하면 `$rtoi(r + 0.5)` 또는 `$rtoi($round(r))` (단, `$round`는 SV 표준 함수가 아님)
패턴을 직접 구현해야 한다.

---

### `$itor`

- **시그니처**: `$itor(integer_val)` → `real`
- **표준**: IEEE 1800-2017 §20.9.1 / IEEE 1364-2005 §17.10
- **반환 타입**: `real` (IEEE 754 double precision)
- **의미**: 정수를 실수로 변환한다. IEEE 754 double의 유효 자릿수는 약 15~16자리이므로 32-bit integer는 완전히 표현 가능하다 — 정밀도 손실 없음.

```sv
integer i = 14;
real r;
r = $itor(i);   // r = 14.0

// 나눗셈 정밀도 향상
real ratio;
ratio = $itor(7) / $itor(3);   // 2.333...  (정수 나눗셈의 2가 아닌 실수 결과)
```

---

### `$realtobits`

- **시그니처**: `$realtobits(real_val)` → `[63:0]` (64-bit logic vector)
- **표준**: IEEE 1800-2017 §20.9.2 / IEEE 1364-2005 §17.10
- **반환 타입**: 64-bit logic vector `[63:0]`
- **의미**: IEEE 754 double precision 인코딩의 **비트 패턴을 그대로** 64-bit vector로 추출한다.
  숫자 값을 변환하는 것이 아니라 비트 레벨 표현을 추출하는 캐스트다.

주요 용도: Verilog/SV 모듈 포트는 real 타입을 직접 전달하지 못한다.
`$realtobits`로 64-bit vector로 변환한 뒤 포트를 통과시키고, 받는 쪽에서 `$bitstoreal`로 복원한다.

```sv
// 모듈 포트를 통한 real 값 전달 패턴
module sender(output logic [63:0] data_bits);
    real data = 3.14159;
    assign data_bits = $realtobits(data);
endmodule

module receiver(input logic [63:0] data_bits);
    real data;
    always_comb data = $bitstoreal(data_bits);
endmodule

// 64-bit IEEE 754 구조 확인 (부호 1 + 지수 11 + 가수 52)
real pi = 3.14159265358979;
logic [63:0] bits = $realtobits(pi);
// bits[63]    = 0 (양수)
// bits[62:52] = 11비트 지수 (바이어스 1023 포함)
// bits[51:0]  = 52비트 가수
```

---

### `$bitstoreal`

- **시그니처**: `$bitstoreal(64bit_vec)` → `real`
- **표준**: IEEE 1800-2017 §20.9.2 / IEEE 1364-2005 §17.10
- **반환 타입**: `real` (IEEE 754 double precision)
- **의미**: 64-bit vector를 IEEE 754 double로 역변환한다. `$realtobits`의 역.
  입력이 정확히 64비트가 아니면 동작 미정의다.

```sv
logic [63:0] encoded;
real decoded;

// roundtrip 검증
encoded = $realtobits(2.718281828);
decoded = $bitstoreal(encoded);
// decoded == 2.718281828  (bit-exact roundtrip)

// NaN/Inf 패턴도 그대로 복원
encoded = 64'h7FF8000000000000;  // IEEE 754 qNaN
decoded = $bitstoreal(encoded);   // real NaN
```

**$shortrealtobits / $bitstoshortreal**: `shortreal`(IEEE 754 single precision) ↔ 32-bit vector 변환 쌍.
동일한 패턴, 폭만 32비트.

---

## $signed/$unsigned vs 자동 부호 전환 비교

| 방법 | 비트 폭 변경 | 확장 방식 | 용도 |
|------|------------|---------|------|
| `$signed(a)` 대입 → wider | 없음 (함수), 확장은 대입 | sign extension | unsigned를 signed 산술에 사용 |
| `$unsigned(a)` 대입 → wider | 없음 (함수), 확장은 대입 | zero extension | signed를 unsigned 산술에 사용 |
| 자동 암묵 변환 | 컨텍스트 의존 | 타입 규칙 의존 | 의도 불명확, 피할 것 |

---

## Icarus / Verilator 지원

| 함수 | Icarus | Verilator | vitamin |
|------|--------|-----------|---------|
| `$signed` | 완전 지원 | Generally supported | ✅ |
| `$unsigned` | 완전 지원 | Generally supported | ✅ |
| `$rtoi` | 완전 지원 | Generally supported | ✅ |
| `$itor` | 완전 지원 | Generally supported | ✅ |
| `$realtobits` | 완전 지원 | Generally supported | ✅ |
| `$bitstoreal` | 완전 지원 | Generally supported | ✅ |

---

## 합성 가능성

| 함수 | 합성 |
|------|------|
| `$signed`, `$unsigned` | ✅ — 합성 가능 (해석 변경만, 논리 추가 없음) |
| `$rtoi`, `$itor` | ❌ — 비합성 (실수 연산) |
| `$realtobits`, `$bitstoreal` | ❌ — 비합성 (real 타입 연산) |

---

## 본 프로젝트 구현 메모

- `hdl-builtins` 크레이트 `conversion` 카테고리 담당
- `$signed`/`$unsigned`: 값 변경 없이 signedness 메타데이터만 토글. Rust enum variant로 표현.
- `$rtoi`: Rust `f64 as i32` (toward-zero truncation). 오버플로우 시 동작은 IEEE 1800 미정의 — Rust saturating_cast 또는 시뮬 경고 발행 검토.
- `$itor`: Rust `i32 as f64` — 완전 정밀도 보장.
- `$realtobits`: Rust `f64::to_bits()` → u64 → SV 64-bit vector.
- `$bitstoreal`: Rust `f64::from_bits(u64)` — NaN 패턴 포함 IEEE 754 bit-exact 보장.

## Sources

- IEEE 1800-2017 §20.9 (conversion system functions)
- IEEE 1364-2005 §17.10 (Verilog 2005, 1800에 흡수)
- research-log: [system-tasks-conversion-math-2026-05-28.md](../../research-log/system-tasks-conversion-math-2026-05-28.md)
- [circuitcove.com — Conversion Functions](https://circuitcove.com/system-tasks-conversion/) (WebFetch ✓)
- [chipverify.com — Verilog Conversion Functions](https://chipverify.com/verilog/verilog-conversion-functions) (WebFetch ✓)
- [hdlworks.com — System Real Conversion Functions](https://www.hdlworks.com/hdl_corner/verilog_ref/items/SystemRealConversionFuncs.htm) (WebFetch ✓)
- [01signal.com — Signed Arithmetic](https://www.01signal.com/verilog-design/arithmetic/signed-wire-reg/) (WebFetch ✓)
- [verilator.org — Input Languages](https://verilator.org/guide/latest/languages.html)
