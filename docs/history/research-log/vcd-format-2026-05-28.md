---
topic: vcd-format
date: 2026-05-28
rounds: 4
primary_sources_fetched:
  - https://en.wikipedia.org/wiki/Value_change_dump
  - https://zipcpu.com/blog/2017/07/31/vcd.html
  - https://chipverify.com/verilog/verilog-vcd
  - https://chipverify.com/verilog/verilog-dump-vcd
  - https://wiki.tcl-lang.org/page/VCD
  - https://github.com/ben-marshall/verilog-vcd-parser/blob/master/src/VCDTypes.hpp
  - https://pyvcd.readthedocs.io/en/latest/_modules/vcd/common.html
  - https://docs.rs/vcd/latest/vcd/struct.Writer.html
  - https://docs.rs/vcd/0.6.1/vcd/struct.IdCode.html
  - https://gtkwave.github.io/gtkwave/internals/vcd-recoding.html
  - https://github.com/gtkwave/gtkwave/issues/336
  - https://peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00056.htm
queries:
  - "Round 1 영문: IEEE 1364 VCD format specification header tokens $dumpvars value change dump"
  - "Round 1 영문: VCD file format Verilog identifier code ASCII 33-126 encoding algorithm printable"
  - "Round 2 영문: VCD $dumpoff $dumpon $dumpall $dumpflush $dumplimit semantics IEEE 1364 Verilog system tasks"
  - "Round 2 영문: VCD var types complete list event integer parameter real reg supply0 supply1 tri wire IEEE 1364"
  - "Round 3 영문: GTKWave VCD parser compatibility issues pitfalls identifier code timescale parsing caveats"
  - "Round 3 영문: iverilog VCD generation $dumpvars x value $dumpoff x-placeholder behavior system task semantics"
  - "Round 3 영문: rust-vcd crate VcdWriter API struct methods identifier encoding Verilog VCD generation"
  - "Round 4 영문: GTKWave VCD loading issues no space scalar value change format whitespace b vector timescale compatibility"
  - "Round 4 영문: VCD $dumpvars initial dump format x z uninitialized value $end after initial values example"
---

# Research: VCD 포맷 · IEEE 1364 §18

4라운드 조사. Wikipedia VCD 항목, zipcpu 블로그(실무 생성기 구현자), chipverify.com, wiki.tcl-lang.org (인코딩 알고리즘), pyvcd 소스, ben-marshall VCD 파서 헤더, docs.rs/vcd (rust-vcd crate API), GTKWave 공식 문서를 직접 fetch해 교차검증함. IEEE 1364-2005 표준 PDF는 WebFetch 압축으로 텍스트 추출 실패 — 표준 내용은 다른 소스들로 삼각측량.

---

## VCD 파일 전체 구조 — 확인된 내용

Wikipedia VCD 항목의 예시 코드 (WebFetch 직접 확인, verbatim):

```vcd
$date
   Date text. For example: November 11, 2009.
$end
$version
   VCD generator tool version info text.
$end
$comment
   Any comment text.
$end
$timescale 1ps $end
$scope module logic $end
$var wire 8 # data $end
$var wire 1 $ data_valid $end
$var wire 1 % en $end
$var wire 1 & rx_en $end
$var wire 1 ' tx_en $end
$var wire 1 ( empty $end
$var wire 1 ) underrun $end
$upscope $end
$enddefinitions $end
$dumpvars
bxxxxxxxx #
x$
0%
x&
x'
1(
0)
$end
#0
b10000001 #
0$
1%
0&
1'
0(
0)
#2211
0'
#2296
b0 #
1$
#2302
0$
#2303
```

[출처: en.wikipedia.org/wiki/Value_change_dump ✓ WebFetch 직접 확인]

---

## 헤더 토큰 상세

모든 키워드는 `$keyword ... $end` 블록 구조. 각 토큰의 역할:

| 토큰 | 내용 |
|------|------|
| `$date` | 파일 생성 일시 (자유 형식 텍스트) |
| `$version` | VCD 생성기 이름·버전 (자유 형식 텍스트) |
| `$comment` | 임의 주석. 시뮬레이션 중 어디서든 삽입 가능 |
| `$timescale` | 시뮬레이션 시간 단위. `1ps`, `10ns` 등 (magnitude: 1/10/100, unit: s/ms/us/ns/ps/fs) |
| `$scope <type> <name>` | 계층 범위 열기 |
| `$upscope` | 상위 범위로 복귀 |
| `$var <type> <width> <id> <ref>` | 변수 선언 |
| `$enddefinitions` | 헤더 종료 마커 — 이 이후부터 simulation data |

`$timescale`은 숫자와 단위를 공백 없이(`1ps`) 또는 공백으로(`1 ps`) 구분 가능.

[출처: Wikipedia + chipverify.com + peterfab.com ✓ 다중 소스 교차 확인]

---

## 스코프 타입

VCDTypes.hpp (ben-marshall 파서) 및 pyvcd ScopeType enum에서 확인된 6가지:

- `module` — 모듈 인스턴스
- `task` — Verilog task
- `function` — Verilog function
- `begin` — named begin 블록
- `fork` — fork 블록
- `root` — 최상위 (파서 내부 편의용, VCD 파일에는 일반적으로 명시 안 함)

[출처: VCDTypes.hpp ✓ WebFetch + pyvcd ✓ WebFetch]

---

## 변수 타입 — 18종 (IEEE 1364 표준)

VCDTypes.hpp와 pyvcd VarType enum에서 동일하게 확인된 18종:

```
event, integer, parameter, real, realtime, reg,
supply0, supply1, time, tri, triand, trior,
trireg, tri0, tri1, wand, wire, wor
```

pyvcd는 여기에 `string`, `logic` 2종을 추가로 정의하지만, 이는 확장 VCD 지원용이며 IEEE 1364-2005 표준 18종이 아님.

[출처: VCDTypes.hpp ✓ WebFetch + pyvcd ✓ WebFetch, 교차검증 일치]

---

## 식별자 코드 인코딩 알고리즘

wiki.tcl-lang.org VCD 패키지 소스에서 구체적인 알고리즘 확인:

**카운터 기반 다문자 인코딩:**

```
초기화: c0=32, c1=32, c2=32, c3=32
변수 추가 시:
  c0 += 1
  if c0 >= 127: c1 += 1, c0 = 33
  if c1 >= 127: c2 += 1, c1 = 33
  if c2 >= 127: c3 += 1, c2 = 33
  if c3 >= 127: 오류 ("Out of id's")

ID 문자열 생성: c3>32이면 %c(c3), c2>32이면 %c(c2), c1>32이면 %c(c1), 항상 %c(c0)
```

- 첫 변수: `!` (ASCII 33)
- 두 번째: `"` (ASCII 34)
- ...
- 94번째: `~` (ASCII 126)
- 95번째: `"!` (c1=34, c0=33)
- 최대 용량: 94^4 = 약 78,074,896개 변수

핵심: 식별자는 1~4문자의 printable ASCII (`!`=33 ~ `~`=126) 조합. 공백(32) 포함 금지.

[출처: wiki.tcl-lang.org ✓ WebFetch — 단일 소스이나 Python/Perl VCD 생성기들과 일관성 있고 Wikipedia의 ASCII 33-126 정의와 부합. 정확한 구현 세부는 단일 소스 주의 필요.]

Wikipedia 추가 확인: "The identifier is composed of one or more printable ASCII characters from ! to ~ (decimal 33 to 126), and these are conventionally kept short (i.e. one or two characters)." — 범위는 표준 확인, 알고리즘 세부는 Tcl 구현 기반.

---

## $dumpvars 초기 덤프 의미론

위 Wikipedia 예시에서 관찰:
- `$dumpvars` 섹션은 `$enddefinitions $end` 직후에 옴
- 시뮬레이션 시작 시 **모든 선택 변수의 초기 값을 한 번 기록**
- 초기화 전 상태는 `x` (unknown). Wikipedia 예시에서 `bxxxxxxxx #`, `x$` 등이 이를 보여줌
- 알고 있는 초기값(HDL에서 초기화된 경우)은 실제 값(`0`, `1`)으로 기록
- 섹션은 `$end`로 종료

이후 `#0` 타임스탬프부터 실제 값 변화가 시작.

[출처: Wikipedia 예시 ✓ WebFetch]

---

## 값 표기 형식

Wikipedia + zipcpu 블로그 + chipverify에서 교차 확인:

**스칼라 (1비트):**
```
<value><id>
```
- `value`는 `0`, `1`, `x`, `z` 중 하나
- id와 value 사이에 **공백 없음** — 이 점이 GTKWave 호환성에 중요
- 예: `0$`, `1%`, `x&`

**벡터 (다비트):**
```
b<bits> <id>
```
또는 대문자 `B`도 허용
- `b`와 `<bits>` 사이에 공백 없음
- `<bits>`와 `<id>` 사이에 **공백 필수**
- 예: `b10000001 #`, `bxxxxxxxx #`, `b0 #`

**실수 (real):**
```
r<value> <id>
```
또는 대문자 `R`도 허용
- `printf %.16g` 형식으로 표현
- 예: `r3.14159265358979 #`

[출처: Wikipedia ✓ WebFetch, zipcpu 블로그 ✓ WebFetch, GTKWave 검색 결과 — 공백 규칙 3소스 일치]

---

## 타임스탬프

- `#<number>` 형식. 숫자는 non-negative 정수.
- 타임스탬프는 **단조 증가** (non-decreasing) — 동일 시각 반복은 허용, 역행은 금지.
- 타임스탬프와 값 변화는 같은 시각에 여러 신호 변화가 일어날 경우 `#T` 한 줄 후에 복수의 값 변화 레코드를 나열.

---

## dump 시스템 태스크 의미론

chipverify + peterfab.com + 검색 결과 교차 확인:

| 태스크 | 의미론 |
|--------|--------|
| `$dumpfile("name.vcd")` | VCD 출력 파일 지정. 호출 안 하면 보통 `"dump.vcd"` 기본값 (구현체 따라 다름) |
| `$dumpvars(level, scope...)` | 덤프 대상 변수 지정. level=0은 지정 scope 하위 전체; level=N은 N단계까지. 인수 없으면 전체 |
| `$dumpoff` | 덤프 일시 정지. 모든 선택 변수를 `x`(스칼라) 또는 `bx`(벡터) 값으로 한 번 기록 후 이후 변화는 기록 안 함 |
| `$dumpon` | 이전 `$dumpoff`로 정지된 덤프 재개. 재개 시 현재 값을 기록 |
| `$dumpall` | 현재 시각의 **모든 선택 변수 현재값을 강제 기록** (체크포인트) |
| `$dumpflush` | VCD 출력 버퍼를 파일로 강제 플러시 (충돌/종료 전 유용) |
| `$dumplimit(bytes)` | VCD 파일 크기 한도(바이트). 도달 시 덤프 중단 + `$comment` 삽입으로 표시 |

iverilog sys_vcd.c 검색 결과에서 `$dumpoff` 구현 세부 확인:
- 스칼라: `x<id>`
- 벡터: `bx <id>`
- real: `rNaN <id>` (일부 구현)
- named event: 변화 없음 (event는 x 값이 없음)

[출처: chipverify.com ✓ WebFetch, peterfab.com ✓ WebFetch, iverilog 검색결과 ✓ — NaN for real: 단일 소스(iverilog 검색), 주의 필요]

---

## GTKWave / Surfer 파서 호환성 주의점

1. **스칼라 공백 없음**: `1!` 이지 `1 !` 아님. 공백 있으면 파서가 인식 실패 가능.
2. **벡터 공백 필수**: `b1010 !` 이지 `b1010!` 아님.
3. **파일 끝 개행 필수**: GTKWave issue #336 — VCD 파일 마지막에 newline이 없으면 parser가 버퍼 오버런으로 "Unknown VCD identifier" 오류 발생. 반드시 `\n`으로 종료.
4. **모든 VCD 토큰은 whitespace 구분**: "All VCD tokens are delineated by whitespace. Data in the VCD file is case sensitive." (GTKWave 문서)
5. **타임스탬프 단조성**: 역행 타임스탬프는 도구에 따라 무시되거나 오류 발생.
6. **`$timescale` 형식**: `1ps` 또는 `1 ps` 둘 다 허용되나, 도구에 따라 다중 토큰 형식(`1 ps $end` 등)을 거부하는 케이스 보고됨.

[출처: GTKWave VCD recoding docs ✓ WebFetch, GTKWave issue #336 ✓ WebFetch, GTKWave 검색 결과]

---

## 확장 VCD ($dumpports*)

`$dumpports`, `$dumpportsoff`, `$dumpportson`, `$dumpportsall`, `$dumpportsflush`, `$dumpportslimit`는 IEEE 1364-2005가 아닌 일부 상용 툴(Synopsys VCS 등)의 확장. 포트 방향성(input/output/inout)과 강도(strength) 정보를 추가로 기록. vitamin 현 단계 비목표.

---

## rust-vcd crate Writer API — 확인된 메서드 시그니처

docs.rs/vcd/latest WebFetch에서 확인된 주요 메서드:

```rust
// 생성
Writer::new(writer: W) -> Writer<W>

// 헤더
fn comment(&mut self, v: &str) -> Result<()>
fn date(&mut self, v: &str) -> Result<()>
fn version(&mut self, v: &str) -> Result<()>
fn timescale(&mut self, ts: u32, unit: TimescaleUnit) -> Result<()>

// 스코프
fn scope_def(&mut self, t: ScopeType, i: &str) -> Result<()>
fn add_module(&mut self, identifier: &str) -> Result<()>
fn upscope(&mut self) -> Result<()>
fn enddefinitions(&mut self) -> Result<()>

// 변수
fn add_var(&mut self, var_type: VarType, width: u32, reference: &str, index: Option<ReferenceIndex>) -> Result<IdCode>
fn add_wire(&mut self, width: u32, reference: &str) -> Result<IdCode>
fn var_def(&mut self, ..., id: IdCode, ...) -> Result<()>

// 시뮬레이션 데이터
fn timestamp(&mut self, ts: u64) -> Result<()>
fn change_scalar<V: Into<Value>>(&mut self, id: IdCode, v: V) -> Result<()>
fn change_vector(&mut self, id: IdCode, v: impl IntoIterator<Item=Value>) -> Result<()>
fn change_real(&mut self, id: IdCode, v: f64) -> Result<()>
fn change_string(&mut self, id: IdCode, v: &str) -> Result<()>

// dump 커맨드 블록
fn begin(&mut self, c: SimulationCommand) -> Result<()>
fn end(&mut self) -> Result<()>
fn flush(&mut self) -> Result<()>
```

`IdCode`: `FIRST` 상수 + `next()` 메서드로 순차 생성. `From<u32>`, `From<u64>` 구현으로 숫자에서도 생성 가능.

[출처: docs.rs/vcd/latest/vcd/struct.Writer.html ✓ WebFetch, docs.rs/vcd/0.6.1/vcd/struct.IdCode.html ✓ WebFetch]

---

## Sources

- [Value change dump — Wikipedia](https://en.wikipedia.org/wiki/Value_change_dump) — VCD 예시, 형식 개요
- [Writing your own VCD File — zipcpu.com](https://zipcpu.com/blog/2017/07/31/vcd.html) — 실무 생성기 구현 관점
- [Verilog VCD — chipverify.com](https://chipverify.com/verilog/verilog-vcd) — 헤더 토큰 설명
- [Verilog VCD Dump — chipverify.com](https://chipverify.com/verilog/verilog-dump-vcd) — dump 태스크 상세
- [VCD — wiki.tcl-lang.org](https://wiki.tcl-lang.org/page/VCD) — 식별자 코드 인코딩 알고리즘
- [VCDTypes.hpp — ben-marshall/verilog-vcd-parser](https://github.com/ben-marshall/verilog-vcd-parser/blob/master/src/VCDTypes.hpp) — var types / scope types 열거
- [vcd.common — pyvcd](https://pyvcd.readthedocs.io/en/latest/_modules/vcd/common.html) — VarType / ScopeType enum
- [Writer — docs.rs/vcd](https://docs.rs/vcd/latest/vcd/struct.Writer.html) — rust-vcd Writer API
- [IdCode — docs.rs/vcd](https://docs.rs/vcd/0.6.1/vcd/struct.IdCode.html) — IdCode 인코딩
- [VCD Recoding — GTKWave](https://gtkwave.github.io/gtkwave/internals/vcd-recoding.html) — GTKWave 파서 내부
- [GTKWave issue #336](https://github.com/gtkwave/gtkwave/issues/336) — 파일 끝 newline 누락 파싱 오류
- [VCD File — peterfab.com](https://peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00056.htm) — dump 태스크 의미론
