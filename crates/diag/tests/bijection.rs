//! CI sync gate: the MsgCode enum and docs/preview/15 §0–9 body codes are 1:1.
//! Appendix A reserved codes are excluded (they are not yet enum variants).
use diag::MsgCode;

const DOC15: &str = include_str!("../../../docs/preview/15-error-code-reference.md");

/// One parsed doc header: `### VITA-E0001 · `E-CLI-BAD-FLAG` (Error)`.
struct DocEntry {
    number: String,   // "VITA-E0001"
    mnemonic: String, // "E-CLI-BAD-FLAG"
    severity: String, // "Error" (verbatim doc token)
}

/// Parse every body `### ...` code header before "## 부록 A".
fn doc_entries() -> Vec<DocEntry> {
    let body = DOC15.split("## 부록 A").next().unwrap();
    let mut out = Vec::new();
    for line in body.lines() {
        let l = line.trim_start();
        let Some(rest) = l.strip_prefix("### ") else {
            continue;
        };
        // mnemonic between the first pair of backticks (skips non-code headers like `### 번호대 예약`).
        let Some(bt0) = rest.find('`') else { continue };
        let Some(rel) = rest[bt0 + 1..].find('`') else {
            continue;
        };
        let mnemonic = rest[bt0 + 1..bt0 + 1 + rel].to_string();
        // number = first whitespace-delimited token (before " · ").
        let number = rest
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string();
        // severity = the parenthesized token after the closing backtick.
        let after = &rest[bt0 + 1 + rel + 1..];
        let severity = after
            .rsplit_once('(')
            .and_then(|(_, s)| s.split_once(')').map(|(sev, _)| sev.trim().to_string()))
            .unwrap_or_default();
        out.push(DocEntry {
            number,
            mnemonic,
            severity,
        });
    }
    out
}

/// Body mnemonics only (back-compat helper).
fn doc_mnemonics() -> Vec<String> {
    doc_entries().into_iter().map(|e| e.mnemonic).collect()
}

#[test]
fn msgcode_matches_doc15_body_one_to_one() {
    let mut doc: Vec<String> = doc_mnemonics();
    let mut enum_codes: Vec<String> = MsgCode::ALL
        .iter()
        .map(|c| c.mnemonic().to_string())
        .collect();
    doc.sort();
    doc.dedup();
    enum_codes.sort();

    assert_eq!(
        enum_codes.len(),
        57,
        "MsgCode must have exactly 57 body variants"
    );
    assert_eq!(
        enum_codes, doc,
        "MsgCode enum and docs/preview/15 §0–9 diverged.\n\
         Adding a code requires a doc entry (and vice-versa)."
    );
}

#[test]
fn doc15_severity_and_number_match_enum() {
    // Beyond the mnemonic set: each doc entry's VITA-#### number and (Severity) tag
    // must agree with the enum's code_num()/default_severity(). Closes the gap where
    // a doc severity typo or wrong number would pass the mnemonic-only bijection.
    let by_mnemonic: std::collections::BTreeMap<&str, MsgCode> =
        MsgCode::ALL.iter().map(|c| (c.mnemonic(), *c)).collect();
    for e in doc_entries() {
        let code = by_mnemonic
            .get(e.mnemonic.as_str())
            .unwrap_or_else(|| panic!("doc code {} not in MsgCode enum", e.mnemonic));
        assert_eq!(
            code.code_num(),
            e.number,
            "VITA number mismatch for {}: enum={} doc={}",
            e.mnemonic,
            code.code_num(),
            e.number
        );
        assert_eq!(
            code.default_severity().token(),
            e.severity.to_ascii_lowercase(),
            "severity mismatch for {}: enum={} doc=({})",
            e.mnemonic,
            code.default_severity().token(),
            e.severity
        );
    }
}

#[test]
fn mnemonics_and_numbers_are_unique() {
    let mut m: Vec<&str> = MsgCode::ALL.iter().map(|c| c.mnemonic()).collect();
    let mut n: Vec<&str> = MsgCode::ALL.iter().map(|c| c.code_num()).collect();
    let (lm, ln) = (m.len(), n.len());
    m.sort();
    m.dedup();
    n.sort();
    n.dedup();
    assert_eq!(m.len(), lm, "duplicate mnemonic");
    assert_eq!(n.len(), ln, "duplicate VITA-#### number");
}
