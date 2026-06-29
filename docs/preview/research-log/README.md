# Research Log

본 폴더는 `research` 스킬 호출 시점의 1차 자료를 라운드별로 보관해 재현성·투명성을 확보한다. spec §11 방법론의 산물.

## 파일 명명
`<topic-slug>-YYYY-MM-DD.md` — 예: `vcd-format-2026-05-29.md`, `sv-data-types-2026-06-02.md`.

## 파일 구조

각 로그는 다음 머리말로 시작한다:

```yaml
---
topic: <짧은 영문 슬러그>
date: YYYY-MM-DD
rounds: <실행 라운드 수, 1~4>
primary_sources_fetched:
  - https://...
queries:
  - "Round 1 영문 쿼리"
  - "Round 1 한국어 쿼리"
  - "Round 2 ..."
---
```

본문은 research 스킬의 narrative 출력을 그대로 첨부하고, 하단 `## Sources` 섹션에 인용 URL을 다시 정리한다.

## 사용 규칙

- 동일 topic 재조사 시 새 날짜 파일 추가, 이전 파일은 **보존**(라운드 이력).
- 학술/상용 paywall(IEEE 표준 등) 내용 **verbatim 복제 금지** — 요약·인용만.
- 라운드별 쿼리 다양성(영/한 언어 혼용, 다른 각도)을 의도적으로 확보.
