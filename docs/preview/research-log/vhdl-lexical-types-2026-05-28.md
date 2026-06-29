# VHDL (IEEE 1076-2008) Lexical · Types · Objects Research Log

**조사일**: 2026-05-28
**대상**: IEEE 1076-2008 §5/§6/§9 — 렉시컬, 타입, 오브젝트
**목적**: 01-lexical.md / 02-types.md / 03-objects.md 작성 근거 확보

---

## 조사 방법

- Round 1: WebSearch 2쿼리 (렉시컬, 타입·오브젝트)
- Phase 1.5: WebFetch 원문 검증 (hdlfactory.com std_logic_1164 ✓, hdlworks.com std_logic_1164 ✓, vhdlwhiz.com delta cycles ✓, piembsystech.com ports ✓)
- Round 2: WebSearch 2쿼리 (키워드 목록, 포트모드·오브젝트), WebFetch UMBC reserved.html ✓
- Round 3: VHDL-2008 신규 키워드 WebSearch + Doulos 페이지 + SourceForge keywords.txt WebFetch ✓

---

## A. 렉시컬 요소

### 식별자

- **기본 식별자(basic)**: 대소문자 무구분. `Signal == SIGNAL == signal`.
- **확장 식별자(extended)**: `\...\` 역슬래시 구분자. 대소문자 구분. 예약어도 사용 가능.
  - 예: `\MySignal\ /= \mysignal\`, `\end\`는 유효한 확장 식별자

출처: UMBC VHDL LRM §13 portal.cs.umbc.edu ✓

### 주석

- `-- ...` : 단일행 (VHDL-87 이후)
- `/* ... */` : 블록 주석, VHDL-2008 추가. 중첩 불가.

출처: Doulos VHDL-2008 docs

### 예약어

VHDL-93 기준 92개 (UMBC §13 직접 열거 검증):

```
abs access after alias all and architecture array assert attribute
begin block body buffer bus
case component configuration constant
disconnect downto
else elsif end entity exit
file for function
generate generic group guarded
if impure in inertial inout is
label library linkage literal loop
map mod
nand new next nor not null
of on open or others out
package port postponed procedure process pure
range record register reject rem report return rol ror
select severity signal shared sla sll sra srl subtype
then to transport type
unaffected units until use
variable
wait when while with
xnor xor
```

VHDL-2008 추가 키워드 (PSL 통합 + 신규 기능, SourceForge vhdl_2008_keywords.txt ✓):

```
assume assume_guarantee context cover default
force parameter property release restrict restrict_guarantee
sequence vmode vprop vunit
```

총합 ~107개 (일부 PSL 키워드는 툴 지원 여부에 따라 다름).

### 리터럴

**정수 / 실수**:
- `42`, `1_000_000`, `3.14`, `1.0e-5`

**기반 리터럴(based literal)**:
- 형식: `기수#값#` 또는 `기수#값#지수`
- `16#FF#`, `2#1010_1100#`, `8#377#`, `16#E#E1`
- 기수 범위: 2~16

**문자 / 문자열**:
- `'A'`, `'"'` (큰따옴표 자체는 `""`)
- `"Hello World"`, `"say ""hi""`

**비트 문자열 리터럴(bit_string_literal)**:
| 접두사 | 기수 | 비트 폭/자리 | 예시 |
|--------|------|-------------|------|
| `B`/`b` | 2진 | 1 | `B"1010"` = 4비트 |
| `O`/`o` | 8진 | 3 | `O"7"` = `B"111"` |
| `X`/`x` | 16진 | 4 | `X"FF"` = `B"11111111"` |
| `D`/`d` | 10진 (2008+) | 자동 | `D"255"` = `X"FF"` |

너비 지정자 (VHDL-2008+): `8X"FF"`, `12D"255"`.

출처: Doulos VHDL-2008 easier-to-use, SynthWorks VHDL-2008 paper

---

## B. 타입 시스템

### 스칼라 타입 (표준 패키지 `std`)

| 타입 | 분류 | 범위/열거 |
|------|------|----------|
| `integer` | 정수 | -(2³¹-1) ~ 2³¹-1 (최소 보장) |
| `natural` | integer 서브타입 | 0 이상 |
| `positive` | integer 서브타입 | 1 이상 |
| `real` | 부동소수 | 구현 의존, 합성 불가 |
| `time` | 물리 | fs, ps, ns, us, ms, sec, min, hr |
| `boolean` | 열거 | `FALSE`, `TRUE` |
| `bit` | 열거 | `'0'`, `'1'` |
| `character` | 열거 | ISO-8859-1 256자 |
| `severity_level` | 열거 | `NOTE`, `WARNING`, `ERROR`, `FAILURE` |
| `file_open_kind` | 열거 | `READ_MODE`, `WRITE_MODE`, `APPEND_MODE` |
| `file_open_status` | 열거 | `OPEN_OK`, `STATUS_ERROR`, `NAME_ERROR`, `MODE_ERROR` |

### 복합 타입

**배열(array)**:
- 제약(constrained): 선언 시 범위 고정. `type byte_t is array(7 downto 0) of bit`
- 비제약(unconstrained): `type bit_vector is array(natural range <>) of bit`

**레코드(record)**:
```vhdl
type point_t is record
  x : integer;
  y : integer;
end record;
```

### 액세스 타입

포인터. `type link_ptr is access node_t`. 시뮬레이션 전용, 합성 불가.

### 파일 타입

`type text is file of string`. `file_open`, `file_close`, `read`, `write` 사용.

---

## C. std_logic_1164 패키지

출처: hdlworks.com ✓, hdlfactory.com ✓

### std_ulogic 9값

```vhdl
type std_ulogic is ('U','X','0','1','Z','W','L','H','-');
```

| 값 | 의미 |
|----|------|
| `'U'` | Uninitialized — 시뮬레이션 초기값 |
| `'X'` | Forcing Unknown — 강한 충돌 |
| `'0'` | Forcing 0 |
| `'1'` | Forcing 1 |
| `'Z'` | High Impedance — 트라이스테이트 |
| `'W'` | Weak Unknown |
| `'L'` | Weak 0 (풀다운) |
| `'H'` | Weak 1 (풀업) |
| `'-'` | Don't Care |

### std_logic (해소 서브타입)

```vhdl
subtype std_logic is resolved std_ulogic;
```

다중 드라이버 허용. `resolved` 함수가 충돌 값 배열을 받아 단일 값 반환.
예: `'0'` + `'1'` → `'X'`, `'0'` + `'Z'` → `'0'`

### 배열 타입

```vhdl
type std_ulogic_vector is array (natural range <>) of std_ulogic;
-- VHDL-2008: std_logic_vector는 std_ulogic_vector의 서브타입
subtype std_logic_vector is (resolved) std_ulogic_vector;
```

### VHDL-2008 개선 사항

- `std_logic_vector`가 `std_ulogic_vector` 서브타입으로 재정의
- 리덕션 연산자: `and_reduce`, `or_reduce`, `nand_reduce`, `nor_reduce`, `xor_reduce`, `xnor_reduce`
- 매칭 비교: `?=`, `?/=`, `?<`, `?<=`, `?>`, `?>=`
- 변환 함수: `to_string`, `to_hstring`, `to_ostring`, `to_bstring`

---

## D. numeric_std 패키지

출처: hdlfactory.com ✓

```vhdl
type unsigned is array (natural range <>) of std_logic;
type signed   is array (natural range <>) of std_logic;
```

| 타입 | 해석 | 범위 |
|------|------|------|
| `unsigned` | 양의 정수 | 0 ~ 2ⁿ−1 |
| `signed` | 2의 보수 | −2ⁿ⁻¹ ~ 2ⁿ⁻¹−1 |

주요 연산: `+`, `-`, `*`, `/`, `mod`, `rem`, `abs`, `**`
비교: `<`, `<=`, `>`, `>=`, `=`, `/=`
변환: `to_integer(u)`, `to_unsigned(n, size)`, `to_signed(n, size)`, `resize(u, size)`

---

## E. 오브젝트와 포트 모드

### 오브젝트 종류

| 오브젝트 | 선언 키워드 | 스코프 | 갱신 타이밍 |
|----------|------------|--------|------------|
| signal | `signal` | 아키텍처/패키지/포트 | delta cycle 이후 |
| variable | `variable` | process/subprogram 내부 | 즉시 |
| constant | `constant` | 모든 선언 영역 | 불변 |
| generic | `generic` | entity/component 파라미터 | 불변 (인스턴스화 시 결정) |
| file | `file` | 시뮬레이션 전용 | N/A |

**signal vs variable 핵심 차이**:
- `s <= expr` : delta cycle 후 반영. 같은 프로세스 내 이후 문장에서는 여전히 이전 값.
- `v := expr` : 즉시 반영. 다음 문장에서 새 값 사용 가능.

출처: vhdlwhiz.com delta-cycles-explained ✓, emlogic.no using-variables-as-registers ✓

### 포트 모드

| 모드 | 내부 읽기 | 내부 쓰기 | 다중 드라이버 | 비고 |
|------|-----------|-----------|--------------|------|
| `in` | ✅ | ❌ | N/A | 입력 전용 |
| `out` | ❌ (93) / ✅ (2008+) | ✅ | ❌ | VHDL-2008에서 읽기 가능 |
| `inout` | ✅ | ✅ | ✅ | 양방향 버스 |
| `buffer` | ✅ | ✅ | ❌ (단일만) | 피드백 출력, 2008 이후 용도 축소 |
| `linkage` | 제한 | 제한 | — | linkage 포트끼리만 연결 |

출처: piembsystech.com ports ✓, hdlworks.com port ✓

---

## Sources

| URL | 검증 방법 | 신뢰도 |
|-----|----------|--------|
| portal.cs.umbc.edu/help/VHDL/reserved.html | WebFetch 직접 열거 ✓ | 높음 (LRM §13 기반) |
| hdlworks.com/hdl_corner/vhdl_ref/VHDLContents/StdLogic1164.htm | WebFetch ✓ | 높음 |
| hdlfactory.com/post/2025/06/29/... | WebFetch ✓ | 높음 |
| vhdlwhiz.com/delta-cycles-explained/ | WebFetch ✓ | 높음 |
| piembsystech.com/ports-and-port-modes-in-vhdl | WebFetch ✓ | 중상 |
| sourceforge.net vhdl_2008_keywords.txt | WebFetch ✓ | 높음 (Scintilla 에디터 VHDL-2008 목록) |

---

## 불확실성 / 추가 확인 필요

1. VHDL-2008 정확한 총 예약어 수: "96+" 또는 "107+" 다양하게 언급. 일부 PSL 키워드는 context에 따라 예약어 취급이 다를 수 있음. 공식 LRM Annex E 확인 필요.
2. 해소 테이블(resolution table) 전체: 9×9 매트릭스 원문은 LRM §14에 수록. 본 조사에서는 대표 케이스만 검증.
3. `d`/`D` 비트 문자열 리터럴: 여러 소스에서 VHDL-2008 추가로 언급되나 PDF 원문 직접 확인은 불가 (바이너리 PDF).
