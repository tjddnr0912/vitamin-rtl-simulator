# 00 · VHDL (IEEE 1076) Reference

본 폴더는 VHDL (IEEE 1076-2008) 문법·패키지·합성가능성 참조.

## 파일

| # | 파일 | 주제 |
|---|---|---|
| 01 | [lexical](01-lexical.md) | 토큰·식별자·주석·리터럴 |
| 02 | [types](02-types.md) | scalar/composite, std_logic_1164, numeric_std |
| 03 | [objects](03-objects.md) | signal/variable/constant/generic + 포트 modes |
| 04 | design-units | entity/architecture/package/configuration/library |
| 05 | concurrent | process, concurrent assignment, component, generate |
| 06 | sequential | if/case/loop, wait, variable assignment |
| 07 | subprograms | function/procedure |
| 08 | packages-libraries | ieee, std_logic_1164, numeric_std, std |
| 09 | synthesizability | VHDL 합성 가능/조건부/비합성 매핑 |

## 본 프로젝트 위치

Phase 3 진입 시 VHDL 프론트엔드를 별도 추가 (공유 IR 위에). std_logic_1164 /
numeric_std는 hdl-builtins의 VHDL 패키지 모듈로 구현한다.

## Sources

- 본 spec §10
- IEEE 1076-2008
