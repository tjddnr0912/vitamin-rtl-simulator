# 01 · VHDL 렉시컬 요소

IEEE 1076-2008 §13 기준.

---

## 식별자

**기본 식별자(basic identifier)**: 알파벳·숫자·밑줄로 구성. 시작은 알파벳.
연속 밑줄(`__`) 및 끝 밑줄 금지. **대소문자 무구분** — `Signal`, `SIGNAL`, `signal`은 동일.

**확장 식별자(extended identifier)**: 역슬래시로 감싼다.

```vhdl
\MySignal\      -- 유효
\end\           -- 예약어도 식별자로 사용 가능
\My Signal\     -- 공백 포함 가능
```

확장 식별자는 **대소문자 구분**: `\MySignal\ /= \mysignal\`.
역슬래시 자체는 `\\`로 이스케이프.

---

## 주석

```vhdl
-- 단일행 주석 (VHDL-87 이후)

/* 블록 주석
   여러 줄 가능
   VHDL-2008 추가 */
```

블록 주석 `/* ... */`은 중첩 불가.

---

## 예약어 (Reserved Words)

### VHDL-1993 기준 (92개)

| 그룹 | 키워드 |
|------|--------|
| 설계 단위 | `entity` `architecture` `package` `configuration` `library` `use` `context` |
| 타입·오브젝트 | `type` `subtype` `constant` `signal` `variable` `file` `shared` `alias` `attribute` `generic` `port` |
| 순차문 | `process` `begin` `end` `if` `then` `elsif` `else` `case` `when` `loop` `for` `while` `next` `exit` `return` `wait` `null` |
| 동시문 | `block` `generate` `component` `map` `open` `others` |
| 연산자 | `and` `or` `nand` `nor` `xor` `xnor` `not` `mod` `rem` `abs` `rol` `ror` `sla` `sll` `sra` `srl` |
| 지연·구동 | `after` `transport` `inertial` `reject` `guarded` `disconnect` `unaffected` |
| 타입 키워드 | `array` `record` `access` `range` `downto` `to` `of` `units` `group` `label` `literal` `in` `out` `inout` `buffer` `linkage` |
| 절차·함수 | `function` `procedure` `pure` `impure` `return` `body` |
| 보고·assert | `assert` `report` `severity` |
| 기타 | `all` `new` `on` `select` `with` `is` `register` `postponed` |
| 조건 | `bus` |

전체 알파벳순:
`abs` `access` `after` `alias` `all` `and` `architecture` `array` `assert` `attribute`
`begin` `block` `body` `buffer` `bus`
`case` `component` `configuration` `constant`
`disconnect` `downto`
`else` `elsif` `end` `entity` `exit`
`file` `for` `function`
`generate` `generic` `group` `guarded`
`if` `impure` `in` `inertial` `inout` `is`
`label` `library` `linkage` `literal` `loop`
`map` `mod`
`nand` `new` `next` `nor` `not` `null`
`of` `on` `open` `or` `others` `out`
`package` `port` `postponed` `procedure` `process` `pure`
`range` `record` `register` `reject` `rem` `report` `return` `rol` `ror`
`select` `severity` `signal` `shared` `sla` `sll` `sra` `srl` `subtype`
`then` `to` `transport` `type`
`unaffected` `units` `until` `use`
`variable`
`wait` `when` `while` `with`
`xnor` `xor`

### VHDL-2008 추가 (PSL 통합 + 신규 기능, ~15개)

`assume` `assume_guarantee` `context` `cover` `default`
`force` `parameter` `property` `release` `restrict`
`restrict_guarantee` `sequence` `vmode` `vprop` `vunit`

> `context`는 컨텍스트 선언(context declaration)에, `force`/`release`는 신호 강제 할당에, PSL 키워드들(`property`, `sequence`, `cover`, `assume` 등)은 검증 단위(verification unit) 작성에 사용.

---

## 리터럴

### 정수·실수 리터럴

```vhdl
42          -- 정수
1_000_000   -- 밑줄 구분자 허용 (가독성용, 무의미)
3.14        -- 실수 (소수점 필수)
1.0e-5      -- 실수 지수 표기
```

### 기반 리터럴 (Based Literal)

형식: `기수#값#` 또는 `기수#값#E지수`

```vhdl
16#FF#        -- 255 (16진)
16#ff#        -- 255 (대소문자 무관)
2#1010_1100#  -- 172 (2진)
8#377#        -- 255 (8진)
16#E#E1       -- 16#E# × 16¹ = 224
```

기수 범위: 2~16. 자릿값 문자도 대소문자 무관.

### 문자·문자열 리터럴

```vhdl
'A'           -- 문자 리터럴
' '           -- 공백
'"'           -- 큰따옴표 문자 (작은따옴표로 감쌈)

"Hello"       -- 문자열
"say ""hi"""  -- 내부 큰따옴표는 "" 이스케이프
""            -- 빈 문자열
```

### 비트 문자열 리터럴 (Bit String Literal)

```vhdl
B"1010_1100"    -- 2진, 8비트
b"1010"         -- 소문자도 허용
O"377"          -- 8진 → 9비트 (각 자리 3비트)
X"FF"           -- 16진 → 8비트 (각 자리 4비트)
x"deadbeef"     -- 32비트
D"255"          -- 10진 → 자동 폭 (VHDL-2008+)
```

| 접두사 | 기수 | 자리당 비트 수 | 도입 버전 |
|--------|------|--------------|----------|
| `B`/`b` | 2진 | 1 | 1993 |
| `O`/`o` | 8진 | 3 | 1993 |
| `X`/`x` | 16진 | 4 | 1993 |
| `D`/`d` | 10진 | 자동 | 2008 |

**VHDL-2008 너비 지정자**: 접두사 앞에 비트 폭을 지정할 수 있다.

```vhdl
8X"FF"    -- 16진 FF를 정확히 8비트로 표현
12X"FF"   -- 12비트로 제로 확장: "000011111111"
4X"FF"    -- 4비트로 잘림: "1111"
8D"255"   -- 255를 8비트 2진으로: "11111111"
```

---

## 구분자 (Delimiters)

```
:=   <=   =>   <>   --   /*   */
+  -  *  /  =  /=  <  <=  >  >=
&  |  .  ,  ;  :  (  )  '
```

`**` (거듭제곱), `??` (조건 변환, 2008+).

---

## Sources

- IEEE 1076-2008 §13 (Lexical elements) — UMBC portal.cs.umbc.edu ✓
- Doulos VHDL-2008 Easier to Use
- SourceForge Scintilla vhdl_2008_keywords.txt ✓
