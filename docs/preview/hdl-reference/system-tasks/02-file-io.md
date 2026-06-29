# 02 · File I/O System Tasks

## 개요

시뮬레이션 중 파일을 읽고 쓰는 태스크/함수 카테고리다.
테스트벤치에서 자극 데이터를 파일에서 로드하거나, 결과를 파일에 저장해
포스트 프로세싱하는 용도로 사용한다.
합성 불가능(simulation-only)이며 `hdl-builtins` file-io 카테고리가 구현한다.

## 지원 Phase (vitamin 구현 상태)

- **✅ 구현됨 (WRITE family + string format, Phase-2)**: `$fopen`(mcd/fd 모드 분기, **$fopen은
  대입 RHS 특수형으로만 지원** — direct rhs 외엔 loud E3009), `$fclose`, `$fwrite`/`$fdisplay`
  (+b/o/h 변형, MCD bit0=stdout 브로드캐스트, closed-fd=W4022), `$sformat`, `$sformatf`
  ($sformatf도 대입 RHS 특수형 — string-literal 포맷 필수).
- **✅ 구현됨 (READ family, v9)**: `$fread`/`$fscanf`/`$fgets`/`$sscanf`/`$feof`/`$fgetc` — 모두 blocking
  assign의 직접 rhs 형태(`n = $fscanf(...)`)로만 지원(statement-level intercept; ref-VAR/메모리 쓰기).
- **미구현 (silent-degrade — 미인식 $task은 WARN + skip, IR 미생성)**: `$fmonitor`, `$fstrobe`.
  본 페이지의 IEEE 표준 레퍼런스 항목은 vitamin 지원 표기가 아니다.

---

## mcd vs fd — 반드시 알아야 할 구분

Verilog 파일 I/O에는 두 가지 파일 핸들 방식이 공존한다.
혼동하기 쉬운 핵심 미묘점이다.

### mcd (multi-channel descriptor) — Verilog-2001 이전 레거시

`$fopen("filename")` — 모드 인자 없이 파일명만 주면 mcd를 반환한다.

- 32비트 비트필드 (`reg [31:0]`)로, 하나의 bit만 set
- bit 0 = stdout (항상 열려있음, 닫을 수 없음)
- bit 31 = 예약 (사용 불가)
- 최대 30개 파일 동시 open
- OR 연산으로 여러 파일에 동시 출력 가능

```sv
// mcd 방식 — 두 파일에 동시 출력
reg [31:0] fd_log, fd_csv;
fd_log = $fopen("sim.log");   // 모드 없음 → mcd 반환
fd_csv = $fopen("data.csv");
$fdisplay(fd_log | fd_csv, "time=%0t val=%h", $time, val);
// stdout에도 함께 출력: fd_log | fd_csv | 32'h1
```

### fd (file descriptor) — IEEE 1364-2001 이후 현대 방식

`$fopen("filename", "mode")` — 모드 인자를 주면 fd를 반환한다.

- 양의 정수 핸들 (C의 FILE* 스타일)
- 0이면 open 실패
- 읽기 모드, seek, binary I/O 등 고급 기능 사용 가능
- mcd와 달리 OR 조합 불가

```sv
// fd 방식 — 읽기/쓰기 포함 고급 I/O
integer fd;
fd = $fopen("output.txt", "w");   // 모드 있음 → fd 반환
if (fd == 0) $display("open failed");
$fdisplay(fd, "result=%d", result);
$fclose(fd);
```

**요약**: 쓰기 전용 단순 로깅은 mcd, 읽기·seek·binary 등 고급 I/O는 fd 방식.

---

## 항목 상세

### `$fopen`

- **시그니처**:
  ```sv
  // mcd 방식 (Verilog-2001 이전)
  reg [31:0] mcd;
  mcd = $fopen("filename");

  // fd 방식 (IEEE 1364-2001+, SV)
  integer fd;
  fd = $fopen("filename", "mode");
  ```
- **표준**: IEEE 1800-2017 §21.3 / IEEE 1364-2005 §17.4.1
- **모드 문자열**:

| 모드 | 동작 |
|------|------|
| `"r"` / `"rb"` | 읽기 전용 (파일 없으면 실패→0 반환) |
| `"w"` / `"wb"` | 쓰기 (파일 생성 또는 기존 내용 삭제) |
| `"a"` / `"ab"` | 추가 쓰기 (파일 없으면 생성) |
| `"r+"` / `"rb+"` | 읽기+쓰기 (기존 파일 필요) |
| `"w+"` / `"wb+"` | 읽기+쓰기 (생성 또는 덮어쓰기) |
| `"a+"` / `"ab+"` | 읽기+추가 |

`b` suffix = binary mode (Unix에서는 텍스트/바이너리 구분 없음, Windows에서 줄 끝 처리 차이).

- **반환**: mcd 방식은 32비트 mcd (실패 시 0), fd 방식은 정수 fd (실패 시 0)

---

### `$fclose`

- **시그니처**: `$fclose(fd_or_mcd)`
- **표준**: IEEE 1800-2017 §21.3 / IEEE 1364-2005 §17.4.2
- **의미**: 파일을 닫는다.
  해당 파일에 active한 `$fmonitor`와 `$fstrobe`가 자동으로 취소된다.
- **반환**: void
- **예시**:

```sv
integer fd;
fd = $fopen("result.txt", "w");
$fdisplay(fd, "done at time %0t", $time);
$fclose(fd);
```

---

### `$fwrite` / `$fdisplay` / `$fmonitor` / `$fstrobe`

콘솔 출력 태스크($write/$display/$monitor/$strobe)와 동일하지만
첫 인자로 파일 핸들(fd 또는 mcd)을 받는다.

- **시그니처**:
  ```sv
  $fdisplay(fd_or_mcd [, "format" [, args...]]);
  $fwrite(fd_or_mcd [, "format" [, args...]]);
  $fstrobe(fd_or_mcd [, "format" [, args...]]);
  $fmonitor(fd_or_mcd [, "format" [, args...]]);
  ```
- **표준**: IEEE 1800-2017 §21.2 / IEEE 1364-2005 §17.3
- **실행 시점 및 개행**:

| 태스크 | 실행 시점 | 자동 개행 |
|--------|----------|---------|
| `$fdisplay` | Active/Inactive 영역 (즉시) | ✅ |
| `$fwrite` | Active/Inactive 영역 (즉시) | ❌ |
| `$fstrobe` | Postponed 영역 (NBA 반영 후) | ✅ |
| `$fmonitor` | Postponed 영역 (인자 변화 시 자동) | ✅ |

b/o/h 변형 존재: `$fdisplayh`, `$fwriteb`, `$fstrobeo`, `$fmonitorh` 등.

- **반환**: void
- **예시**:

```sv
integer log_fd;
initial begin
  log_fd = $fopen("sim.log", "w");

  // 콘솔과 파일에 동시 출력 (mcd OR 방식)
  reg [31:0] both;
  both = log_fd | 32'h1;   // bit0 = stdout
  $fdisplay(both, "starting simulation");
end

always @(posedge clk) begin
  $fwrite(log_fd, "t=%0t d=%b q=%b  ", $time, d, q);
  $fstrobe(log_fd, "q_final=%b", q);   // NBA 후 최종값
end
```

---

### `$fread`

- **시그니처**:
  ```sv
  integer n;
  n = $fread(reg_or_mem_target, fd);
  n = $fread(reg_or_mem_target, fd, start_addr);
  n = $fread(reg_or_mem_target, fd, start_addr, count);
  ```
- **표준**: IEEE 1800-2017 §21.4 / IEEE 1364-2005 §17.4.4
- **의미**: 바이너리 데이터를 파일에서 읽는다.
  `target`이 단일 reg이면 해당 폭(비트 수/8 바이트)만큼 읽는다.
  `target`이 memory 배열이면 `start_addr`부터 순서대로 원소를 채운다.
  `count`를 지정하면 해당 원소 수만큼만 읽는다.
- **반환**: 실제 읽은 바이트 수 (EOF 또는 오류 시 0 이하)
- **예시**:

```sv
reg [7:0] mem [0:255];
integer fd, n;
initial begin
  fd = $fopen("data.bin", "rb");
  n = $fread(mem, fd, 0, 256);   // 주소 0부터 256 원소 로드
  $display("read %0d bytes", n);
  $fclose(fd);
end
```

---

### `$fscanf`

- **시그니처**: `integer n = $fscanf(fd, "format_string", var1, var2, ...)`
- **표준**: IEEE 1800-2017 §21.3 / IEEE 1364-2005 §17.4.3
- **의미**: 파일에서 한 줄(또는 필드)을 읽어 포맷 문자열에 따라 파싱한다.
  C `fscanf()`와 동일한 포맷 specifier 사용.
- **반환**: 성공적으로 매칭한 항목 수 (EOF에 도달하면 음수, 오류 시 0)
- **예시**:

```sv
integer fd, addr, val, n;
initial begin
  fd = $fopen("vectors.txt", "r");
  while (!$feof(fd)) begin
    n = $fscanf(fd, "%h %h\n", addr, val);
    if (n == 2) begin
      mem[addr] = val;
    end
  end
  $fclose(fd);
end
```

---

### `$fgets`

- **시그니처**: `integer n = $fgets(str_var, fd)`
- **표준**: IEEE 1800-2017 §21.3 / IEEE 1364-2005 §17.4.3
- **의미**: 파일에서 한 줄을 읽어 `str_var`에 저장한다.
  newline 또는 EOF까지 읽는다. `str_var`의 크기가 한계를 정한다.
- **반환**: 읽은 문자 수 (오류 또는 EOF 즉시 도달 시 0)
- **예시**:

```sv
reg [255*8-1:0] line;
integer fd, n;
initial begin
  fd = $fopen("input.txt", "r");
  n = $fgets(line, fd);
  while (n > 0) begin
    $display("line: %s", line);
    n = $fgets(line, fd);
  end
  $fclose(fd);
end
```

---

### `$sscanf`

- **시그니처**: `integer n = $sscanf(source_string, "format_string", var1, var2, ...)`
- **표준**: IEEE 1800-2017 §21.3
- **의미**: 파일이 아닌 **문자열**에서 파싱한다. `$fscanf`의 문자열 버전.
- **반환**: 성공적으로 매칭한 항목 수
- **예시**:

```sv
string line = "addr=FF data=AB";
integer addr_v, data_v, n;
n = $sscanf(line, "addr=%h data=%h", addr_v, data_v);
// n=2, addr_v=8'hFF, data_v=8'hAB
```

---

### `$sformat`

- **시그니처**: `$sformat(output_reg, "format_string" [, arg1, arg2, ...])`
- **표준**: IEEE 1800-2017 §20.9 / IEEE 1364-2005 §17.1.3
- **의미**: 포맷 문자열과 인자를 조합해 `output_reg`에 문자열로 저장한다.
  **task** — void, 반환값 없음.
  첫 인자 `output_reg`는 결과를 받는 `reg` 또는 `string` 변수.
- **반환**: void (task)
- **예시**:

```sv
reg [255*8-1:0] msg;
$sformat(msg, "error: addr=%h expected=%h got=%h", addr, exp, got);
$display("%s", msg);
```

---

### `$sformatf`

- **시그니처**: `string s = $sformatf("format_string" [, arg1, arg2, ...])`
- **표준**: IEEE 1800-2017 §20.9.1 (SV 전용)
- **의미**: `$sformat`과 동일하나 **function** — string을 직접 반환한다.
  `$sformat`과의 차이: `$sformat`은 task(첫 인자에 결과 저장),
  `$sformatf`는 function(반환값이 string). SV에서는 `$sformatf`가 선호된다.
- **반환**: `string` (function)
- **예시**:

```sv
// $sformatf는 expression 위치에 직접 사용 가능
$display($sformatf("val=0x%08X tick=%0d", val, $time));

// 문자열 조합에 편리
string prefix = "ERROR";
string msg = $sformatf("[%s] mismatch at addr=%h", prefix, addr);
```

---

## $sformat vs $sformatf 비교

| 항목 | `$sformat` | `$sformatf` |
|------|-----------|------------|
| 종류 | task (void) | function (반환값 있음) |
| 결과 받는 방법 | 첫 인자 reg에 저장 | 반환값으로 직접 사용 |
| 가용 표준 | IEEE 1364-2001+ | IEEE 1800-2017 (SV 전용) |
| 권장 환경 | Verilog 호환 필요 시 | SystemVerilog (현대 TB) |

---

## Icarus / Verilator 동작 차이 + vitamin 구현 상태

| 태스크 | Icarus Verilog | Verilator | vitamin |
|--------|---------------|-----------|---------|
| `$fopen` (mcd/fd 양방식) | 완전 지원 | Generally supported | ✅ (대입 RHS 특수형) |
| `$fclose` | 완전 지원 | Generally supported | ✅ |
| `$fdisplay` / `$fwrite` | 완전 지원 | Generally supported | ✅ (+b/o/h, MCD) |
| `$fstrobe` / `$fmonitor` | 완전 지원 | Generally supported | ❌ 미구현 (silent-degrade) |
| `$fread` | 완전 지원 | 미명시 (확인 불가) | ✅ (v9 — blocking-assign rhs) |
| `$fscanf` | 완전 지원 | Generally supported | ✅ (v9 — blocking-assign rhs) |
| `$fgets` / `$fgetc` | 완전 지원 | Generally supported | ✅ (v9 — blocking-assign rhs) |
| `$sscanf` | 완전 지원 | Generally supported | ✅ (v9 — blocking-assign rhs) |
| `$sformat` | 완전 지원 | 미명시 | ✅ |
| `$sformatf` | 완전 지원 | 미명시 | ✅ (대입 RHS 특수형) |

---

## 합성 가능성

❌ 비합성 — 전 태스크/함수가 시뮬레이션 전용.

---

## 본 프로젝트 구현 메모

- WRITE family는 `sim-engine` `builtins.rs`가 실행(`hdl-builtins`는 stub; 기능은 sim-engine 인라인).
- **✅ mcd/fd 분기 구현(Phase-2/v7)**: `$fopen`을 대입 RHS 특수형(`fopen_special`, `elaborate`)으로
  처리 — 모드 유무로 mcd(`reg [31:0]`)/fd 분기, 인자=string literal 필수, intra-assignment delay 불가.
  fd 핸들 0x8000_0003…, MCD bit1…(bit0=stdout 브로드캐스트), closed-fd=W4022 warn-once.
- **✅ `$sformatf` 구현(Phase-2/v7, string 타입과 함께)**: 대입 RHS 특수형(`sformatf_special`),
  string-literal 포맷 필수. `$sformat`은 일반 task(`Sformat`)로 매핑.
- **✅ READ family 구현(v9)**: `$fread`(바이너리 stream)·`$fscanf`·`$fgets`·`$sscanf`·`$feof`·`$fgetc`는
  blocking assign rhs 전용 statement-level intercept(`k_fscanf`/`k_fgets`/`k_fread` 등, `sim-engine`)로 구현됨.
  `$fmonitor`/`$fstrobe`만 미매핑(silent-degrade).

## Sources

- IEEE 1800-2017 §21 (File I/O system functions/tasks)
- IEEE 1364-2005 §17.4 (Verilog file I/O)
- research-log: [system-tasks-io-memory-2026-05-28.md](../../research-log/system-tasks-io-memory-2026-05-28.md)
- [chipverify.com — Verilog File IO Operations](https://chipverify.com/verilog/verilog-file-io-operations)
- [circuitcove.com — File I/O Tasks](https://circuitcove.com/system-tasks-file-io/)
- [hdlworks.com — System File I/O Tasks](https://www.hdlworks.com/hdl_corner/verilog_ref/items/SystemFileTasks.htm)
- [verilator.org — Input Languages](https://verilator.org/guide/latest/languages.html)
