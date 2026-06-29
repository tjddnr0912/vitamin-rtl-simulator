# 12 · Introspection Functions

## 개요

SystemVerilog의 introspection(내성) 함수는 실행 중인 시뮬레이션 안에서
타입·배열·파라미터 정보를 조회하는 수단을 제공한다.
`$typename`은 타입 이름 문자열을, `$cast`는 런타임 동적 캐스트를,
`$isunbounded`는 파라미터 unbounded 여부를 확인한다.
`$size` / `$left` / `$right` / `$low` / `$high` / `$increment` /
`$dimensions` / `$unpacked_dimensions`는 배열의 차원 정보를 런타임에 쿼리한다.

이 함수들은 파라미터화 모듈, 제네릭 테스트벤치, 동적 OOP 코드에서
타입에 독립적인 범용 코드를 작성할 때 특히 유용하다.

## 구현 상태

- ✅ **구현됨**: `$typename`, `$isunbounded`, `$size`, `$left`, `$right`, `$low`, `$high`,
  `$increment`, `$dimensions`, `$unpacked_dimensions` — 전부 elaborate-time const-fold
  (`try_introspect_fold`; net 인자 해소 → 차원 디스크립터/타입 spelling 조회. type-literal·indexed·
  expression 인자는 여전히 loud). `$cast` — task 형태(`SysTaskId::Cast`)·function 형태(`SysFuncId::Cast`)
  모두 배선됨(enum/value 캐스트; class 다운캐스트는 OOP(N7) 트랙과 연동).
  hand-IEEE 핀(iverilog 13.0이 이들을 거부 → 차분 오라클 부재).

---

## 타입 쿼리

### `$typename(expr_or_type)` — 타입 이름 문자열 반환

- **표준**: IEEE 1800-2017 §20.6
- **반환**: string — 인자의 "해결된 타입 이름"
- 표현식(expression)과 타입(data type) 양쪽을 인자로 받는다.

typedef, enum, parameterized type 모두 원래 이름을 보존해 반환한다:

```sv
typedef logic [7:0] byte_t;
typedef enum logic [1:0] { IDLE, RUN, DONE } state_e;

byte_t      b;
state_e     s;
logic [3:0] nibble;
int         n;

$display("%s", $typename(b));       // "byte_t"
$display("%s", $typename(s));       // "state_e"
$display("%s", $typename(nibble));  // "logic [3:0]"
$display("%s", $typename(n));       // "int"
$display("%s", $typename(byte_t));  // "byte_t"  (타입 직접 전달도 가능)
```

파라미터화 모듈에서 어떤 타입이 바인딩되었는지 로그로 확인할 때 유용하다:

```sv
module checker #(type T = logic [7:0]) (input T data);
  initial $display("data type: %s", $typename(data));
endmodule
```

**주의**: 반환 문자열의 정확한 포맷은 구현 정의(implementation defined)다.
문자열 내용을 파싱해 분기 로직을 만드는 것은 이식성을 깨뜨린다.
디버그·로깅 목적으로만 사용할 것.

**Icarus**: 기본 타입은 동작, typedef 이름 보존은 부분적.
**Verilator**: 지원 (`--sv` 모드).

---

### `$cast(dest, src)` — 런타임 동적 캐스트

- **표준**: IEEE 1800-2017 §20.5
- 소스 값(src)이 대상 타입(dest)에 런타임에 적합한지 확인하고 대입한다.
- 정적 캐스트(`type'(expr)`)는 컴파일타임 체크인 데 반해,
  `$cast`는 **런타임에 값(value)을 직접 검사**한다 — 변수 선언 타입이 아니라 실제 값을 본다.

두 가지 호출 형태가 있다:

#### 태스크(task) 형태

캐스트에 실패하면 런타임 에러를 발생시키고 dest는 변경되지 않는다.

```sv
$cast(dest_handle, src_handle);
```

#### 함수(function) 형태

성공이면 1, 실패이면 0을 반환한다. 실패해도 런타임 에러가 발생하지 않으며
dest는 변경되지 않는다. 직접 오류 메시지를 커스터마이즈할 때 사용한다.

```sv
if (!$cast(dest_handle, src_handle))
  $error("cast failed: src type mismatch at %0t", $time);
```

**사용 예 1 — enum 범위 확인**:

```sv
typedef enum logic [1:0] { S0, S1, S2 } fsm_t;
logic [1:0] raw_val;
fsm_t       state;

// raw_val = 2'b11 이면 cast 실패 (열거값에 없음)
if (!$cast(state, raw_val))
  $error("invalid state encoding: %0b", raw_val);
else
  $display("state = %s", state.name());
```

**사용 예 2 — 클래스 계층 다운캐스트**:

```sv
class Packet;
  int id;
endclass

class EthPacket extends Packet;
  int vlan;
endclass

Packet    base_pkt;
EthPacket eth_pkt;

base_pkt = new EthPacket();   // 부모 핸들로 자식 객체 참조 (업캐스트)

// 다운캐스트 — 부모 핸들이 실제로 EthPacket을 가리킬 때만 성공
if ($cast(eth_pkt, base_pkt))
  $display("vlan=%0d", eth_pkt.vlan);
else
  $error("not an EthPacket");
```

**핵심**: `$cast`는 **타입이 아닌 값**을 보기 때문에 같은 코드라도 런타임 값에 따라 결과가 달라진다. 항상 함수 형태로 호출해 반환값을 확인하는 것이 권장 패턴이다.

**Icarus**: 클래스 OOP 지원이 제한적 — class hierarchy cast 제한됨; enum cast 부분 지원.
**Verilator**: class OOP 포함 완전 지원.

---

### `$isunbounded(expr)` — unbounded 파라미터 확인

- **표준**: IEEE 1800-2017 §20.6
- 인자가 unbounded 값(`$`)이면 1'b1(true), 아니면 1'b0(false)을 반환한다.

SystemVerilog에서 파라미터는 `$`(unbounded)를 값으로 가질 수 있다.
`$isunbounded`는 이 값을 감지해 분기 처리할 때 사용한다.

```sv
module memctrl #(
  parameter int DEPTH = $,    // 기본값: unbounded
  parameter int WIDTH = 8
) (...);

  // 파라미터 유효성 체크
  initial begin
    if (!$isunbounded(DEPTH) && DEPTH < 4)
      $fatal(1, "DEPTH must be >= 4 or $ (unbounded), got %0d", DEPTH);
    if (!$isunbounded(DEPTH))
      $display("finite DEPTH=%0d", DEPTH);
    else
      $display("unbounded DEPTH — dynamic sizing");
  end
endmodule
```

SVA의 unbounded 반복(`##[n:$]`)과 조합해 사용하는 경우도 있다.

**Icarus / Verilator**: 지원.

---

## 배열 차원 쿼리

### 차원 번호 체계 (dim argument)

배열 쿼리 함수들은 모두 선택적 `dim` 인자를 받는다. 번호 규칙:

- `dim = 1`: 가장 왼쪽 **unpacked** 차원
- 이후 오른쪽 방향으로 unpacked 차원 번호 증가
- unpacked 차원 소진 후, 왼쪽에서 오른쪽 방향으로 **packed** 차원 번호 계속 증가
- **dim 생략**: 기본값 1 (첫 번째 unpacked 차원)

예시로 아래 선언을 기준으로 설명한다:

```sv
logic [7:0][15:0] arr [3:0][0:7];
//  unpacked: dim=1 → [3:0],  dim=2 → [0:7]
//  packed:   dim=3 → [7:0],  dim=4 → [15:0]
```

---

### `$size(arr [, dim])` — 원소 수

- **표준**: IEEE 1800-2017 §20.7
- 지정 차원의 원소 수를 반환한다. `$high(arr,dim) - $low(arr,dim) + 1`과 동등.
- dim 생략 시 dim=1.

```sv
logic [7:0] byte_arr [0:3];

$size(byte_arr)      // = 4  (dim=1 unpacked)
$size(byte_arr, 1)   // = 4
$size(byte_arr, 2)   // = 8  (dim=2 packed [7:0])
```

동적 배열은 현재 할당 크기를, 큐는 현재 원소 수를 반환한다.
연관 배열에는 `$size`를 쓸 수 없다 — `.num()` 메서드로 대체한다.

---

### `$left(arr [, dim])` / `$right(arr [, dim])` — 선언 경계

- **표준**: IEEE 1800-2017 §20.7
- `$left`: 선언에서 **왼쪽**에 적힌 경계값을 반환한다.
- `$right`: 선언에서 **오른쪽**에 적힌 경계값을 반환한다.
- 방향(up-counting / down-counting)을 그대로 반영한다.

```sv
logic [7:0] a_down [3:0];   // left=3, right=0 (down-counting)
logic [7:0] a_up   [0:3];   // left=0, right=3 (up-counting)

$left(a_down)   // = 3
$right(a_down)  // = 0
$left(a_up)     // = 0
$right(a_up)    // = 3
```

---

### `$low(arr [, dim])` / `$high(arr [, dim])` — 절대 최솟값·최댓값

- **표준**: IEEE 1800-2017 §20.7
- 선언 방향에 무관하게 항상 **작은 값**이 `$low`, **큰 값**이 `$high`.

```sv
logic [7:0] a_down [3:0];
logic [7:0] a_up   [0:3];

$low(a_down)    // = 0   (3과 0 중 작은 쪽)
$high(a_down)   // = 3
$low(a_up)      // = 0
$high(a_up)     // = 3
```

배열 순회에서 방향에 독립적인 코드를 쓰고 싶을 때 `$low`/`$high`를 쓴다.
선언된 방향 자체를 확인해야 할 때는 `$left`/`$right`를 쓴다.

---

### `$increment(arr [, dim])` — 인덱스 방향

- **표준**: IEEE 1800-2017 §20.7
- `$left >= $right`이면 **1**, `$left < $right`이면 **-1**을 반환한다.
- 인덱스 순회 방향을 런타임에 감지할 때 사용한다.

```sv
logic [7:0] a_down [7:0];   // $left=7 >= $right=0 → +1
logic [7:0] a_up   [0:7];   // $left=0 < $right=7  → -1

// 방향 독립적 순회
for (int i = $low(arr); i <= $high(arr); i++)
  process(arr[i]);

// increment를 이용한 정방향/역방향 선택
initial begin
  automatic int step = $increment(arr);
  automatic int idx  = $left(arr);
  repeat ($size(arr)) begin
    process(arr[idx]);
    idx -= step;  // down-count면 -(+1)=-1 → 감소, up-count면 -(-1)=+1 → 증가
  end
end
```

---

### `$dimensions(arr)` / `$unpacked_dimensions(arr)` — 차원 수

- **표준**: IEEE 1800-2017 §20.7
- `$dimensions`: packed + unpacked 전체 차원 수.
  1-D scalar 비트 벡터나 문자열은 1 반환. 비-배열 타입은 0.
- `$unpacked_dimensions`: unpacked 차원 수만. packed-only 배열은 0.

```sv
logic [7:0][15:0] arr [3:0][0:7];

$dimensions(arr)             // = 4 (unpacked 2 + packed 2)
$unpacked_dimensions(arr)    // = 2

logic [7:0] packed_only;
$dimensions(packed_only)     // = 1 (packed 1차원)
$unpacked_dimensions(packed_only) // = 0
```

**제네릭 테스트벤치 패턴** — 차원 수를 런타임에 확인해 로직 분기:

```sv
module auto_checker #(type T = logic [7:0]) (input T dut_out, T ref_out);
  initial begin
    if ($dimensions(dut_out) > 1)
      $display("multi-dim array: %0d dims", $dimensions(dut_out));
    // 차원별 루프는 generate나 recursive task로 구현
  end
endmodule
```

---

## 함수 정리 비교

| 함수 | 반환 | 주요 용도 |
|------|------|---------|
| `$typename(e)` | string | 디버그 타입 이름 출력 |
| `$cast(dst, src)` | 1/0 (function form) | 런타임 동적 타입 캐스트 |
| `$isunbounded(e)` | bit | 파라미터 unbounded 확인 |
| `$size(arr [,dim])` | int | 차원 원소 수 |
| `$left(arr [,dim])` | int | 선언 왼쪽 경계 |
| `$right(arr [,dim])` | int | 선언 오른쪽 경계 |
| `$low(arr [,dim])` | int | 절대 최솟값 경계 |
| `$high(arr [,dim])` | int | 절대 최댓값 경계 |
| `$increment(arr [,dim])` | 1 or -1 | 인덱스 방향 |
| `$dimensions(arr)` | int | 전체 차원 수 |
| `$unpacked_dimensions(arr)` | int | unpacked 차원 수 |

---

## Icarus / Verilator 지원

| 함수 | Icarus Verilog | Verilator |
|------|---------------|-----------|
| `$typename` | 기본 타입 지원, typedef 부분적 | 지원 (`--sv`) |
| `$cast` (enum) | 부분 지원 | 지원 |
| `$cast` (class) | 제한적 (OOP 미완성) | 지원 |
| `$isunbounded` | 지원 | 지원 |
| `$size` | 지원 (단순 1-dim) | 지원 |
| `$left/$right/$low/$high` | 부분 지원 | 지원 |
| `$increment` | 제한적 | 지원 |
| `$dimensions/$unpacked_dimensions` | 제한적 | 지원 |

Icarus는 SV 배열 쿼리의 서브셋만 구현한다. 멀티-dim 배열에서 dim 인자를 지정한 쿼리는
동작하지 않거나 잘못된 값을 반환할 수 있다.
Verilator는 SystemVerilog 서브셋에서 이 함수들을 안정적으로 지원한다.

---

## 합성 가능성

❌ 전 함수 비합성 — 시뮬레이션 및 검증 전용.
`$typename`/`$dimensions`/`$size` 등은 elaborate-time 상수로 계산될 수 있으나
합성 도구가 이를 인식하지 않는 경우가 대부분이므로 RTL에서 사용하지 않는다.

---

## 본 프로젝트 구현 메모

> ✅ **구현됨** — `$typename`/`$isunbounded`/`$size`/`$left`/`$right`/`$low`/`$high`/`$increment`/
> `$dimensions`/`$unpacked_dimensions`는 elaborate-time const-fold(`try_introspect_fold`),
> `$cast`는 task/function 양형 배선. 아래는 구현 배경·설계 메모다.

- **$typename**: 타입 메타데이터 조회 API — 타입 시스템 인프라에서 타입 ID → 이름 맵 유지.
  typedef alias가 있으면 alias 이름을 우선 반환.
- **$cast (function form)**: 런타임 타입 ID 비교 + 다운캐스트 허용 여부 체크.
  실패 시 0 반환, dest_var 불변 보장.
- **$cast (task form)**: 내부적으로 function form 호출 후 실패 시 `SimError` raise.
- **$isunbounded**: `$`는 내부 특수 상수값(예: `Value::Unbounded`)으로 표현.
  `$isunbounded(v)` = `v == Value::Unbounded`.
- **$size / $left / $right / $low / $high / $increment**: 배열 디스크립터 구조체에
  차원별 `(left, right)` 쌍을 저장. 쿼리 시 dim 인자로 인덱싱.
  동적 배열은 디스크립터를 런타임에 갱신.
- **$dimensions / $unpacked_dimensions**: 배열 디스크립터의 총 차원 수 / unpacked 차원 수 필드.

## Sources

- IEEE 1800-2017 §20.5 ($cast), §20.6 ($typename, $isunbounded), §20.7 (array dimension query)
- research-log: [system-tasks-introspection-misc-2026-05-28.md](../../research-log/system-tasks-introspection-misc-2026-05-28.md)
- [circuitcove.com — Data and Array Query Functions](https://circuitcove.com/system-tasks-query/) (WebFetch ✓)
- [vlsiverify.com — SystemVerilog Casting](https://vlsiverify.com/system-verilog/systemverilog-casting/) (WebFetch ✓)
- [siemens verificationhorizons — $cast() runtime checks](https://blogs.sw.siemens.com/verificationhorizons/2021/06/28/runtime-checks-with-the-cast-method/) (WebFetch ✓)
- [vlsi.pro — Array Querying System Functions](https://vlsi.pro/system-verilog/array-querying-system-functions/) (WebFetch ✓)
