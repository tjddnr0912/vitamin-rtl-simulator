# 00 · SystemVerilog (IEEE 1800) Reference

본 폴더는 SystemVerilog (IEEE 1800-2017) — Verilog 흡수 + 확장 — 의 신규/확장 기능 참조.

## 파일

| # | 파일 | 주제 |
|---|---|---|
| 01 | data-types | logic/bit/int, enum/struct/union/typedef, 2-state vs 4-state |
| 02 | arrays | packed/unpacked/dynamic/associative/queue + 배열 메소드 |
| 03 | procedural | always_comb/_ff/_latch, unique/priority, foreach |
| 04 | interfaces | interface/modport/clocking block |
| 05 | packages | package/import/export/$unit |
| 06 | classes-oop | class/상속 (검증용, 비합성) |
| 07 | assertions-sva | immediate/concurrent assertion, property, sequence |
| 08 | functions-tasks | SV 추가 시스템 태스크 cross-link |
| 09 | synthesizability | SV 합성 가능/조건부/비합성 매핑 |

## Verilog와의 관계

SV는 Verilog (IEEE 1364) 슈퍼셋. Verilog 문서(`../verilog/`)는 부분집합 정리,
본 폴더는 SV 추가/변경 사항.

## Sources

- 본 spec §10
- IEEE 1800-2017
