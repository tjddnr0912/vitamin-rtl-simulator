# 12 · 조사 방법론 (research 스킬 기반)

## 원칙
**다라운드 + 다각도 + 1차 source 직접 확인.** 단일 소스 의존 없이 라운드마다 의도적으로 다른 각도(쿼리·언어·시각)에서 검색·검증해 hallucination을 거른다.

## 사용 도구
**`research` 스킬** = Anthropic WebSearch + WebFetch 다라운드.
- `gemini -p` 서브에이전트 방식은 **폐기** (이전 백엔드, 기록만 보존).

## 3 소스
1. **Claude 내부 지식** — 빠른 초안·골격(baseline). 단독 사용 금지(반드시 라이브 검증과 결합).
2. **WebSearch (다각도)** — 라운드마다 의도적으로 다른 쿼리/언어(영/한)/시각.
3. **WebFetch — 1차 source 직접 확인** — IEEE LRM 페이지, 표준 본문, 도구 공식 문서를 **직접 읽어** hallucination 차단 (Phase 1.5 검증).

## 워크플로우

```
다각도 WebSearch
   ↓
WebFetch로 핵심 source 검증
   ↓
Claude 5차원 체크리스트 gap 점검 (covered? sourced? precise? balanced? Korean clarity?)
   ↓
gap 있으면 라운드 추가 (다른 각도) — 최대 4
   ↓
한국어 narrative 수렴
```

## 충돌 처리
소스 간 정보 불일치 시:
- 양쪽 기록 + 차이 명시
- **IEEE LRM(1차 표준) 우선**
- 도구별(Icarus/Verilator) 동작 차이는 별도 표기

## 재현성
모든 조사는 `research-log/<topic-slug>-YYYY-MM-DD.md`에 기록:
- 머리말 YAML: topic / date / rounds / primary_sources_fetched / queries
- 본문: research 스킬의 narrative 출력
- 하단 Sources: 인용 URL 재정리

## 헤더 규약
research 스킬 규약 준수 — 본문 헤더에 한자/추상 명사(起承轉結·기승전결·"도입/본론/결론") 금지. 주제에서 따온 구체 헤더만.

## 호출 예시

```
research IEEE 1364 §18 VCD format: exact BNF, identifier code encoding, scalar/vector/real value notation, $dumpvars semantics. Primary: IEEE 1364-2005 §18. 한국어 narrative with BNF.
```

## Sources
- 본 spec §11.
- 본 plan 공통 규칙.
- research 스킬 정의 (Claude Code skills).
