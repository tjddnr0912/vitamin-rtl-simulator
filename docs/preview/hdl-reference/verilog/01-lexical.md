# 01 · Verilog Lexical Conventions

IEEE 1364-2005 §3 기준. 어휘(lexical) 계층은 소스 텍스트를 토큰 스트림으로
변환하는 첫 번째 처리 단계이다.

---

## 토큰 분류

Verilog 소스는 다음 7종 토큰으로 구성된다.

| 토큰 | 예시 |
|------|------|
| whitespace | 공백, 탭, 개행(CR/LF) |
| comment | `// ...` , `/* ... */` |
| operator | `+`, `~`, `===` |
| number | `8'hAB`, `3.14` |
| string | `"hello"` |
| identifier | `clk`, `\sys.clk ` |
| keyword | `module`, `wire`, `always` |

Whitespace는 토큰 경계를 결정하지만, 문자열 리터럴 외부에서는 의미를 갖지 않는다.

---

## 주석 (Comments)

```verilog
// 한 줄 주석 — 줄 끝(\n)까지
/* 블록 주석
   여러 줄 가능
   중첩 불가: /* 안에 /* */ 넣으면 에러 */
```

블록 주석은 중첩할 수 없다. `// /* */` 처럼 줄 주석 안에 블록 구분자를 넣어도
줄 끝에서 종료된다.

---

## 식별자 (Identifiers)

### 단순 식별자 (Simple Identifier)

```
첫 문자: [A-Za-z_]
나머지:  [A-Za-z0-9_$]*
```

- 대소문자 구분 (`clk` ≠ `CLK`)
- `$`로 시작 불가 (시스템 태스크 전용)
- 숫자로 시작 불가

### 탈출 식별자 (Escaped Identifier)

```
\<임의 문자열><whitespace>
```

`\`로 시작하고 공백(스페이스·탭·개행) 하나로 종료. 그 사이의 모든 출력 가능
문자를 이름으로 사용할 수 있다. 탈출 시퀀스(`\n` 등)가 아니라 리터럴 문자다.

```verilog
\x+y            // 식별자: x+y
\sys.clk        // 식별자: sys.clk
\a[0]           // 식별자: a[0]  (배열 접근 아님)
wire \reset-n ; // 유효한 식별자 (하이픈 포함)
```

탈출 식별자의 역슬래시와 종료 whitespace는 이름의 일부가 아니다.
`\reset-n ` 과 같이 쓰면 시뮬레이터는 이를 `reset-n` 이라는 이름으로 인식한다.

---

## 숫자 리터럴 (Number Literals)

### 정수 리터럴 형식

```
[size] ' [s|S] base_specifier digits
```

| 필드 | 의미 | 기본값 |
|------|------|--------|
| `size` | 비트 폭 (양의 정수) | 32 (unsized) |
| `s` / `S` | signed 지정 (1364-2001+) | unsigned |
| base_specifier | `d` / `b` / `o` / `h` (대소문자 무관) | `d` |
| digits | 해당 기수 숫자 + x/X + z/Z + `_` | — |

### 예시

```verilog
// 기본 크기 지정 리터럴
8'hAB          // 8비트 hex 0xAB (십진 171)
4'b1010        // 4비트 binary 10
12'd255        // 12비트 decimal 255
6'o63          // 6비트 octal 51

// Signed 리터럴 (1364-2001+)
4'sd5          // 4비트 signed +5
8'sb1111_0000  // 8비트 signed binary −16 (2의 보수)
12'sh800       // 12비트 signed hex −2048
'sd9           // unsized signed decimal 9 (32비트)

// x / z 자릿수
4'bx           // 4비트 all-unknown (= 4'bxxxx)
8'hzz          // 8비트 all-high-Z (= 8'hzzzz)
8'b1010_xxxx   // 상위 4비트 known, 하위 4비트 unknown
4'bz0          // MSB=z, LSB=0

// 가독성용 언더스코어
32'hDEAD_BEEF  // 16진 구분
20'b0001_1010_0011_0100_0101

// Unsized (비트폭 미지정)
42             // 32비트 signed decimal 42
'b1101         // 32비트 binary
'hFF           // 32비트 hex
```

### 크기 규칙

- **Truncation**: `size` < 실제 값 비트수 → MSB 잘림 (경고 없을 수 있음)
- **Zero extension**: unsigned unsized literal을 더 넓은 컨텍스트에 대입 → 상위 0 채움
- **Sign extension**: signed literal을 더 넓은 signed 컨텍스트에 대입 → MSB 반복

### 실수 리터럴 (Real Number)

두 형식만 허용한다:

```
고정소수점:  <정수부>.<소수부>     예: 3.14, 0.5, 1.0
지수 표기:   <정수부>[.<소수부>]e[+|-]<지수>  예: 1.5e3, 2.5E-4, 64e0
```

소수점 앞뒤 모두 한 자리 이상 필수 (`1.` 또는 `.5` 단독은 문법 오류).

```verilog
real clk_freq = 1.0e9;  // 1 GHz
real tau      = 1.5e-9; // 1.5 ns
```

---

## 키워드 (Keywords)

Verilog-2005 예약 키워드 **140개** — 모두 소문자. 대문자로 쓰면 식별자로
인식된다 (`Module` ≠ `module`). 단, 관례적으로 키워드와 동일한 이름의
사용자 식별자는 피한다.

```
always        and           assign        automatic
begin         buf           bufif0        bufif1
case          casex         casez         cell
cmos          config        deassign      default
defparam      design        disable       edge
else          end           endcase       endconfig
endfunction   endgenerate   endmodule     endprimitive
endspecify    endtable      endtask       event
for           force         forever       fork
function      generate      genvar        highz0
highz1        if            ifnone        incdir
include       initial       inout         input
instance      integer       join          large
liblist       library       localparam    macromodule
medium        module        nand          negedge
nmos          nor           noshowcancelled  not
notif0        notif1        or            output
parameter     pmos          posedge       primitive
pull0         pull1         pulldown      pullup
pulsestyle_onevent  pulsestyle_ondetect   rcmos
real          realtime      reg           release
repeat        rnmos         rpmos         rtran
rtranif0      rtranif1      scalared      showcancelled
signed        small         specify       specparam
strong0       strong1       supply0       supply1
table         task          time          tran
tranif0       tranif1       tri           tri0
tri1          triand        trior         trireg
unsigned      use           uwire         vectored
wait          wand          weak0         weak1
while         wire          wor           xnor
xor
```

`automatic`, `generate`, `genvar`, `localparam`, `signed`, `unsigned`,
`uwire` 는 Verilog-2001/2005 에서 추가된 키워드다 (Verilog-1995에는 없음).

---

## 컴파일러 키워드 버전 지시자

```verilog
`begin_keywords "1364-2005"
  // 이 범위에서는 1364-2005 키워드만 예약어로 취급
`end_keywords
```

SV 파서가 Verilog 소스를 처리할 때 버전을 명시하면 SV 전용 키워드와
충돌하는 사용자 식별자 문제를 회피할 수 있다.

---

## Sources

- IEEE 1364-2005 §3 (Lexical conventions)
- IEEE 1800-2017 §5 (Lexical conventions — Verilog-compat)
- portal.cs.umbc.edu/help/VHDL/verilog/reserved.html (키워드 목록)
- vlsiverify.com/verilog/lexical-conventions/
- chipverify.com/verilog/verilog-syntax
- projectf.io/posts/numbers-in-verilog/
