# 03 · Memory Load / Store System Tasks

## 개요

메모리 배열을 텍스트 파일에서 초기화하거나 파일로 저장하는 태스크 카테고리다.
ROM 초기화, 테스트 벡터 로드, 시뮬레이션 메모리 덤프 등에 사용한다.
합성 가능성은 도구 의존적이며(초기값 ROM은 합성 도구가 지원하기도 함),
`hdl-builtins` memory-load 카테고리가 구현한다.

## 지원 Phase (vitamin 구현 상태)

- **✅ 구현됨 (Phase-2/v7)**: `$readmemh`, `$readmemb` — `@addr`(hex)·`//`/`/* */` 주석·`_`·`x`/`z`,
  1364-2005 lowest-ascending, 디렉티브-유무 shortfall 규칙, 범위 인자(start/finish), 초과 시 W4023.
  iverilog 차분 핀.
- **✅ 구현됨 (Phase-2/v9)**: `$writememh`, `$writememb` — `$readmemh/b`가 다시 읽을 수 있는 포맷으로
  메모리 배열을 파일에 덤프(주소별 `@<hex>` 헤더, x/z nibble 압축, 범위 인자 start/finish, OOB=W-warn).

---

## 항목 상세

### `$readmemh` / `$readmemb`

- **시그니처**:
  ```sv
  // 4가지 형태 — 인자 개수에 따라 범위가 다름
  $readmemh("filename", mem_array);
  $readmemh("filename", mem_array, start_addr);
  $readmemh("filename", mem_array, start_addr, end_addr);

  $readmemb("filename", mem_array);
  $readmemb("filename", mem_array, start_addr);
  $readmemb("filename", mem_array, start_addr, end_addr);
  ```

- **표준**: IEEE 1364-2005 §17.2.8 / IEEE 1800-2017 §21.4

- **의미**:
  - `$readmemh` — 16진수(hex) 데이터 파일에서 읽는다
  - `$readmemb` — 2진수(binary) 데이터 파일에서 읽는다
  - `start_addr` / `end_addr`: 메모리 배열의 로딩 시작·끝 주소를 제한한다.
    생략 시 배열의 선언 범위 전체를 사용한다.

- **반환**: void
- **예시**:

```sv
// 기본 — 전체 배열 초기화
reg [7:0] rom [0:255];
initial $readmemh("rom_contents.hex", rom);

// 주소 범위 제한 — [0x10, 0x1F] 구간만 로드
reg [15:0] sram [0:4095];
initial $readmemh("patch.hex", sram, 16'h10, 16'h1F);

// 이진수 파일 로드
reg [3:0] lut [0:15];
initial $readmemb("lut_init.bin", lut);
```

---

## 메모리 파일 포맷 규칙 (IEEE 1364-2005 §17.2.8)

메모리 파일(.hex, .mem 등)이 따라야 하는 포맷 규칙.

### 데이터 구분자

공백(스페이스), 탭, 개행 모두 구분자로 사용된다.
여러 공백/개행이 연속해도 무방하다.

```text
AA BB CC DD
EE FF
```

### 주석

Verilog 소스와 동일한 두 가지 주석 형식을 지원한다.

```text
// 라인 주석: 이 줄 끝까지 무시
AA BB CC   // 첫 세 바이트

/* 블록 주석:
   여러 줄에 걸쳐 무시됨 */
DD EE FF
```

### `@<hex>` 주소 지시자

파일 내에서 로딩 시작 주소를 변경할 때 사용한다.
`@` 뒤에 **16진수** 주소값을 쓴다 (항상 hex, `0x` prefix 없음).

```text
@00 AA BB CC DD      // 주소 0x00부터: AA, BB, CC, DD
// 주소 0x04~0x0F는 건너뜀 — 변경 없음
@10 00 11 22 33      // 주소 0x10부터: 00, 11, 22, 33
@FF EE               // 주소 0xFF에: EE
```

`@addr` 지시자가 없으면 기본으로 배열의 시작 주소(또는 `start_addr` 인자)에서부터 순서대로 채운다.

### 언더스코어(`_`) — 가독성용 구분자

숫자 값 내부에 `_`를 넣어 가독성을 높일 수 있다. 값에 영향을 주지 않는다.

```text
// $readmemh 파일에서: 32비트 값
DEAD_BEEF   // 0xDEADBEEF와 동일
1234_5678

// $readmemb 파일에서: 8비트 값
1111_0000   // 8'b11110000과 동일
```

### 4-state 값 (`x`, `z`)

`x`(unknown), `z`(high-Z) 값도 파일에 포함할 수 있다.

```text
// $readmemh
xX   // 불확정 (hex 자리에 x)
zZ   // 고임피던스

// $readmemb
xxxx_0000   // 상위 4비트 x, 하위 4비트 0
```

---

## 파일 길이와 배열 크기가 다른 경우

### 파일 < 배열 (파일이 더 짧음)

`start_addr`부터 파일에 있는 데이터만 로드된다.
배열의 나머지 원소는 변경되지 않는다 (초기화 전 상태 유지).
이 동작은 `@addr` 지시자로 비연속 구간을 채울 때도 동일하다.

```sv
// 256원소 배열에 파일에 64개 데이터만 있는 경우
// → [0]~[63]만 파일값으로 채워지고, [64]~[255]는 x (미초기화 상태)
reg [7:0] mem [0:255];
initial $readmemh("partial.hex", mem);
```

### 파일 > 배열 (파일이 더 긺)

배열 범위를 초과하는 데이터가 있으면 시뮬레이터는 경고 또는 오류를 발생시킨다.
동작은 시뮬레이터 구현에 의존한다 — Icarus는 경고, 일부 도구는 오류로 중단.

---

## `$writememh` / `$writememb`

- **시그니처**:
  ```sv
  $writememh("filename", mem_array);
  $writememh("filename", mem_array, start_addr);
  $writememh("filename", mem_array, start_addr, end_addr);

  $writememb("filename", mem_array);
  $writememb("filename", mem_array, start_addr);
  $writememb("filename", mem_array, start_addr, end_addr);
  ```

- **표준**: IEEE 1800-2017 §21.4 (SV에서 추가)
- **의미**: 메모리 배열의 내용을 `$readmemh`/`$readmemb`가 다시 읽을 수 있는
  포맷으로 파일에 출력한다.
  `start_addr`/`end_addr`를 지정하면 해당 범위만 출력한다.
- **출력 포맷**: 각 주소 앞에 `@<hex_addr>` 지시자를 삽입하며 데이터를 기록.
  이 파일은 `$readmemh`/`$readmemb`로 그대로 다시 로드 가능.
- **반환**: void
- **예시**:

```sv
// 시뮬 후 메모리 내용을 파일로 저장
reg [7:0] ram [0:255];
// ... 시뮬레이션 실행 후 ...
initial begin
  // 전체 저장
  $writememh("ram_dump.hex", ram);

  // 특정 구간만 저장
  $writememh("ram_patch.hex", ram, 8'h10, 8'h1F);
end
```

**`$writememh` 출력 예시** (`ram[0]=8'hAA`, `ram[1]=8'hBB`):

```text
@00
AA
@01
BB
```

---

## 완전한 사용 예시

```sv
module tb_rom;
  // 256 x 8-bit ROM
  reg [7:0] rom [0:255];
  reg [7:0] addr;
  wire [7:0] data_out;

  // ROM 초기화
  initial begin
    // hex 파일: @00 DE AD BE EF ... (주소+데이터)
    $readmemh("rom_init.hex", rom);
    $display("rom[0]=%h rom[1]=%h", rom[0], rom[1]);
  end

  // 시뮬 후 덤프
  final begin
    $writememh("rom_verify_dump.hex", rom);
  end

endmodule
```

**rom_init.hex 내용 예시**:

```text
// ROM 초기화 데이터 (IEEE 1364 §17.2.8 포맷)
@00
DE AD BE EF   // 주소 0~3
@10
00 11 22 33   // 주소 16~19 직접 지정
// 사이 주소 4~15는 미초기화 상태 유지
```

---

## Icarus / Verilator 동작 차이

| 태스크 | Icarus Verilog | Verilator |
|--------|---------------|-----------|
| `$readmemh` | 완전 지원 | 지원 (단일 차원만) |
| `$readmemb` | 완전 지원 | 지원 (단일 차원만) |
| `@addr` 지시자 | 지원 | 지원 |
| `//`, `/* */` 주석 | 지원 | 지원 |
| `_` 언더스코어 | 지원 | 지원 여부 불확실 |
| `$writememh` | 지원 | 미명시 (불확실) |  *(vitamin: ✅ 구현, v9)*
| `$writememb` | 지원 | 미명시 (불확실) |  *(vitamin: ✅ 구현, v9)*
| 다차원 배열 | 지원 | **미지원** |

**Verilator 제약**: `$readmemh`/`$readmemb`는 1차원 배열(`reg [N:0] mem [0:M]`)만 지원한다.
`reg mem [N][M]` 형태의 다차원 배열에는 사용할 수 없다.

---

## 합성 가능성

합성 가능성은 도구 의존적이다:
- 일부 FPGA 합성 도구(Xilinx Vivado, Intel Quartus)는 `initial` 블록 내
  `$readmemh`를 ROM 초기화로 인식해 합성 가능.
- 범용 ASIC 합성 도구는 대부분 비합성.
- `$writememh`/`$writememb`는 항상 비합성.

---

## 본 프로젝트 구현 메모

- `$readmemh`/`$readmemb`는 `sim-engine` `builtins.rs` `readmem()`이 실행(`hdl-builtins`는 stub).
- **✅ 파서 구현**: `@addr`(hex), `//`, `/* */`, `_`, `x`/`z` 모두 처리. 선언-인덱스 도메인 @addr,
  1364-2005 lowest-ascending, 디렉티브 유무에 따른 shortfall 규칙, 범위 초과 시 W4023.
- `$fread`(binary stream)는 별도 file-io 카테고리에서 구현(v9); 본 카테고리는 `$readmem*`(ASCII 텍스트).
- 주소 범위 인자(start/finish): 명시적 start/finish가 배열 선언 범위보다 우선, 로드 범위 외=stopped.
- `$writememh`/`$writememb`: **✅ 구현(v9)** — `sim-engine` `builtins.rs` `writemem()`; 주소별 `@<hex>` 헤더,
  x/z nibble 압축, 범위 인자(start/finish), OOB=비치명 warn.

## Sources

- IEEE 1364-2005 §17.2.8 (readmem tasks — primary standard)
- IEEE 1800-2017 §21.4 (writemem tasks, SV 확장)
- research-log: [system-tasks-io-memory-2026-05-28.md](../../research-log/system-tasks-io-memory-2026-05-28.md)
- [projectf.io — Initialize Memory in Verilog](https://projectf.io/posts/initialize-memory-in-verilog/)
- [peterfab.com — File I/O Functions](https://peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00016.htm)
- [ovisign.com — Verilog Write/Read File Operations](https://ovisign.com/verilog-verification/verilog-write-read-file-operations/)
- [Verilog::Readmem — metacpan.org](https://metacpan.org/pod/Verilog::Readmem)
- [verilator.org — Input Languages](https://verilator.org/guide/latest/languages.html)
