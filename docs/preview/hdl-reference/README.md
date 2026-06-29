# HDL Reference

본 폴더는 Verilog · SystemVerilog · VHDL의 문법 · 패키지 · 합성 가능성을 항목별로 정리한 참조 문서다. 시뮬레이터 구현 시 표준 준수 확인용.

## 폴더 구성

| 폴더/파일 | 설명 |
|---|---|
| [00-standards-map.md](00-standards-map.md) | IEEE 1800/1364/1076/1164 버전 · 관계 매핑 |
| [01-synthesizability-legend.md](01-synthesizability-legend.md) | ✅/⚠️/❌ 합성 표기 범례 (전 참조문서 공통) |
| [system-tasks/](system-tasks/) | 표준 `$`-system tasks/functions 카테고리별 |
| [verilog/](verilog/) | Verilog (IEEE 1364) 문법 · 구조 |
| [systemverilog/](systemverilog/) | SystemVerilog (IEEE 1800) 확장 |
| [vhdl/](vhdl/) | VHDL (IEEE 1076) 문법 · 패키지 |

## 권장 읽기 순서

1. `00-standards-map` → `01-synthesizability-legend` (전체 규약 파악)
2. `system-tasks/00-index` (커버리지 매트릭스)
3. 관심 언어 폴더의 `00-index` → 항목별 문서

## Sources

- 본 spec §10 (구조)
