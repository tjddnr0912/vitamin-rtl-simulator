# 00 · Verilog (IEEE 1364) Reference

본 폴더는 Verilog (IEEE 1364-2005) — SV에 흡수된 부분집합 — 의 문법/구조 참조.

## 파일

| # | 파일 | 주제 |
|---|---|---|
| 01 | lexical | 토큰·식별자·숫자 리터럴·주석 |
| 02 | data-types | net/wire/reg/integer/real/vector/parameter |
| 03 | expressions-operators | 연산자·우선순위·signed/x/z |
| 04 | modules-hierarchy | module/port/instantiation/parameter/generate |
| 05 | behavioral | initial/always/blocking(=)/non-blocking(<=) |
| 06 | procedural-statements | if/case/for/while/repeat/forever/fork-join |
| 07 | tasks-functions | task/function/automatic |
| 08 | gate-level | primitives/UDP/drive strength |
| 09 | compiler-directives | `` `timescale/`define/`ifdef/`include `` |
| 10 | system-tasks | 본 폴더 개요 + ../system-tasks/ cross-link |
| 11 | synthesizability | 합성 가능/조건부/비합성 매핑 |

## 버전 이력 요약

| 표준 | 주요 추가사항 |
|------|-------------|
| IEEE 1364-1995 (Verilog-1995) | 기본 Verilog 언어 |
| IEEE 1364-2001 (Verilog-2001) | signed 타입, ANSI 포트 스타일, generate, `**` 연산자 |
| IEEE 1364-2005 (Verilog-2005) | uwire, 마이너 수정, 최종 독립 표준 |
| IEEE 1800-2009~ (SystemVerilog) | 1364-2005 흡수 통합 — 이후 1364 별도 발행 없음 |

## 본 프로젝트 입장

SV 프론트엔드가 Verilog를 완전 흡수 — 별도 Verilog 프론트엔드 없음.
본 폴더는 RTL 사용자가 Verilog 서브셋만 쓸 때의 참조.

SV와 달리 Verilog에는 없는 것: `logic`, `always_ff/comb/latch`, `interface`,
`struct/union`, `package`, `clocking`, assertion 등.

## Sources

- 본 spec §10 (violet_sw/016_claude_rtl project spec)
- IEEE 1364-2005, IEEE 1800-2017
