//! split part of `pp` (mechanical move).

use super::*;

// ─────────────────────────────────────────────────────────────────────────────
// The scanner (§2.3)
// ─────────────────────────────────────────────────────────────────────────────

impl Preprocessor<'_> {
    /// Walk one file's original text, mapping output verbatim 1:1.
    pub(crate) fn scan_file(&mut self, file: FileId) {
        let src = self.files[file.0 as usize].text.clone();
        // A file is scanned VERBATIM even when the `include that brought it in sits
        // inside a macro body: its directives map to its own bytes and its `__LINE__
        // is its own line (review B B2: an `ifdef diagnostic inside such an include
        // pointed at the macro use). The expansion context is cleared for its
        // duration and restored after.
        let saved_site = self.cur_site.take();
        let saved_anchor = self.line_anchor.take();
        self.scan_impl(&src, file, None, 0);
        self.cur_site = saved_site;
        self.line_anchor = saved_anchor;
    }

    /// Walk synthetic macro-expansion text; every emit collapses to `site`.
    pub(crate) fn scan_text(&mut self, text: &str, site: (FileId, u32), depth: u32) {
        // PP-FANOUT-CAP: once the output budget is blown, abandon the expansion
        // immediately so a 2^N fan-out recursion unwinds in O(depth), not O(2^N).
        if self.budget_blown {
            return;
        }
        if depth > self.opts.max_macro_depth {
            self.err(
                MsgCode::PpRecursiveMacro,
                "macro expansion exceeded maximum depth",
                self.out.len(),
            );
            return;
        }
        // The `file` argument identifies the include-search context. Macro bodies
        // resolve includes relative to the use-site file (site.0).
        let saved_site = self.cur_site.replace(site);
        self.scan_impl(text, site.0, Some(site), depth);
        self.cur_site = saved_site;
    }

    /// Shared scanner core. `site_for_collapse = None` => verbatim emits mapped
    /// 1:1 against `file`. `Some((f, b))` => every emit collapses to `(f, b)`.
    pub(crate) fn scan_impl(
        &mut self,
        src: &str,
        file: FileId,
        site_for_collapse: Option<(FileId, u32)>,
        depth: u32,
    ) {
        let bytes = src.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i];
            match c {
                b'"' => {
                    let (end, ok) = scan_string(src, i);
                    if !ok && end < bytes.len() {
                        // Reached a newline (unterminated). Emit up to the newline,
                        // report, and continue scanning AT the newline.
                        self.emit_run(&src[i..end], file, i as u32, site_for_collapse);
                        self.err(
                            MsgCode::PpBadDirective,
                            "unterminated string literal",
                            self.out.len(),
                        );
                        i = end;
                        continue;
                    }
                    self.emit_run(&src[i..end], file, i as u32, site_for_collapse);
                    if !ok {
                        // Unterminated at EOF.
                        self.err(
                            MsgCode::PpBadDirective,
                            "unterminated string literal",
                            self.out.len(),
                        );
                    }
                    i = end;
                }
                b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                    let end = scan_line_comment(src, i);
                    self.emit_run(&src[i..end], file, i as u32, site_for_collapse);
                    i = end;
                }
                b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                    let end = scan_block_comment(src, i);
                    self.emit_run(&src[i..end], file, i as u32, site_for_collapse);
                    i = end;
                }
                b'`' => {
                    i = self.handle_backtick(src, file, site_for_collapse, depth, i);
                }
                _ => {
                    // Ordinary run: copy up to the next interesting byte.
                    let start = i;
                    let mut j = i + 1;
                    while j < bytes.len()
                        && bytes[j] != b'"'
                        && bytes[j] != b'`'
                        && bytes[j] != b'/'
                    {
                        j += 1;
                    }
                    self.emit_run(&src[start..j], file, start as u32, site_for_collapse);
                    i = j;
                }
            }
        }
    }

    /// Emit `s` verbatim (mapped to `file`@`orig`) or collapsed to the site, and
    /// only when emitting.
    pub(crate) fn emit_run(
        &mut self,
        s: &str,
        file: FileId,
        orig: u32,
        site_for_collapse: Option<(FileId, u32)>,
    ) {
        if !self.emitting() {
            return;
        }
        // Before any real (non-directive) output, flush the pending directive-line
        // newline so a maximal run of directives collapses to one newline.
        self.flush_pending_nl();
        match site_for_collapse {
            None => self.emit_verbatim(s, file, orig),
            Some((sf, sb)) => self.emit_collapsed(s, sf, sb),
        }
    }

    /// Handle a backtick at `i`. Returns the new cursor.
    pub(crate) fn handle_backtick(
        &mut self,
        src: &str,
        file: FileId,
        site_for_collapse: Option<(FileId, u32)>,
        depth: u32,
        i: usize,
    ) -> usize {
        let backtick = i;
        let Some((name, name_end)) = parse_ident(src, i + 1) else {
            // Stray backtick: no identifier follows.
            self.saw_directive = true;
            if self.emitting() {
                self.err(MsgCode::PpBadDirective, "stray backtick", self.out.len());
            }
            return i + 1;
        };
        self.saw_directive = true;
        let name = name.to_string();

        if is_directive_kw(&name) {
            return self.handle_directive(src, file, &name, backtick, name_end);
        }

        // IEEE 1800-2017 §22.13 `__FILE__ / `__LINE__: the file name (as a string
        // literal) and line number of the USE — inside a macro body that is the line
        // of the macro's use, in an included file the including use's own file and
        // line (both oracles). Nothing is consumed but the name.
        if name == "__FILE__" || name == "__LINE__" {
            if !self.emitting() {
                return name_end;
            }
            let (f, byte) = match (self.line_anchor, site_for_collapse) {
                (Some((af, ab)), Some(_)) => (af, ab as usize),
                (_, Some((sf, sb))) => (sf, sb as usize),
                (_, None) => (file, backtick),
            };
            let text = if name == "__FILE__" {
                let nm = self.files[f.0 as usize]
                    .name
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"");
                format!("\"{nm}\"")
            } else {
                let src_f = &self.files[f.0 as usize].text;
                let upto = byte.min(src_f.len());
                (src_f.as_bytes()[..upto]
                    .iter()
                    .filter(|&&c| c == b'\n')
                    .count()
                    + 1)
                .to_string()
            };
            self.emit_run(&text, file, backtick as u32, site_for_collapse);
            return name_end;
        }

        // Macro use.
        if !self.emitting() {
            // Dead region: skip the macro use, do not parse arguments.
            return name_end;
        }
        self.handle_macro_use(
            src,
            file,
            site_for_collapse,
            depth,
            &name,
            backtick,
            name_end,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_macro_use(
        &mut self,
        src: &str,
        file: FileId,
        site_for_collapse: Option<(FileId, u32)>,
        depth: u32,
        name: &str,
        backtick: usize,
        name_end: usize,
    ) -> usize {
        // The site to which expanded text collapses. For a top-level (verbatim)
        // file the site is (file, backtick). For a re-scan the site is inherited.
        let site = site_for_collapse.unwrap_or((file, backtick as u32));
        let literal = format!("`{name}");

        if self.active.contains(name) {
            self.err(
                MsgCode::PpRecursiveMacro,
                format!("recursive expansion of macro `{name}"),
                self.out.len(),
            );
            self.emit_run(&literal, file, backtick as u32, site_for_collapse);
            return name_end;
        }

        let Some(mac) = self.macros.get(name).cloned() else {
            self.err(
                MsgCode::PpBadDirective,
                format!("undefined macro use `{name}"),
                self.out.len(),
            );
            self.emit_run(&literal, file, backtick as u32, site_for_collapse);
            return name_end;
        };

        match &mac.params {
            None => {
                // Object-like: NEVER consumes a following `(`. Route through
                // `substitute` (empty params = identity except token-paste ` `` ` and
                // stringify ` `" `, which an object-like body may also use).
                let body = substitute(&mac.body, &[], &[]);
                self.active.insert(name.to_string());
                self.macro_depth += 1;
                let saved_anchor = self.line_anchor;
                if site_for_collapse.is_none() {
                    self.line_anchor = Some((file, name_end as u32));
                }
                self.scan_text(&body, site, depth + 1);
                self.line_anchor = saved_anchor;
                self.macro_depth = self.macro_depth.saturating_sub(1);
                self.active.remove(name);
                name_end
            }
            Some(params) => {
                let params = params.clone();
                // Skip whitespace/newlines after NAME looking for `(`.
                let bytes = src.as_bytes();
                let mut k = name_end;
                while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
                if k >= bytes.len() || bytes[k] != b'(' {
                    self.err(
                        MsgCode::PpMacroArity,
                        format!("function-like macro `{name} used without argument list"),
                        self.out.len(),
                    );
                    self.emit_run(&literal, file, backtick as u32, site_for_collapse);
                    return name_end;
                }
                let split = split_args(src, k);
                if !split.closed {
                    self.err(
                        MsgCode::PpMacroArity,
                        format!("unterminated macro argument list for `{name}"),
                        self.out.len(),
                    );
                    self.emit_run(&literal, file, backtick as u32, site_for_collapse);
                    return split.close;
                }
                // Empty-actual rule: a single whitespace-only actual maps to []
                // iff the macro declares zero params.
                let mut actuals = split.actuals;
                if params.is_empty() && actuals.len() == 1 && actuals[0].is_empty() {
                    actuals.clear();
                }
                if actuals.len() > params.len() {
                    self.err(
                        MsgCode::PpMacroArity,
                        format!(
                            "macro `{name} expects {} argument(s), got {}",
                            params.len(),
                            actuals.len()
                        ),
                        self.out.len(),
                    );
                    self.emit_run(&literal, file, backtick as u32, site_for_collapse);
                    return split.close + 1;
                }
                // §22.5.1 defaults: an OMITTED trailing actual and an EMPTY one both
                // take the formal's default; an omitted actual whose formal has none is
                // the arity error (both oracles reject `\`M(1)` for `M(a, b)`); an empty
                // actual whose formal has none substitutes empty text, as before.
                // §22.13 `__LINE__ inside the ACTUALS of a multi-line use is the line
                // where the use closes too (review B B1: both oracles), so the anchor
                // is set BEFORE the actuals are pre-expanded.
                let saved_anchor = self.line_anchor;
                if site_for_collapse.is_none() {
                    self.line_anchor = Some((file, split.close as u32));
                }
                for (i, p) in params.iter().enumerate() {
                    // BLANK = nothing but whitespace and COMMENTS — `M(1, /*clk*/, /*rst*/)`
                    // is how ibex leaves a defaulted actual empty (review A F1; both
                    // oracles take the default).
                    let blank = actuals
                        .get(i)
                        .is_none_or(|a| skip_ws_comments(a, 0) == a.len());
                    if !blank {
                        continue;
                    }
                    match (&mac.defaults.get(i).cloned().flatten(), actuals.get(i)) {
                        (Some(d), _) => {
                            // A default is macro TEXT: its `"…`" stringification and ``
                            // paste are resolved as an object-like body's would be
                            // (review A F2, both oracles) before it travels as an actual.
                            let d = substitute(d, &[], &[]);
                            if i < actuals.len() {
                                actuals[i] = d;
                            } else {
                                actuals.push(d);
                            }
                        }
                        (None, Some(_)) => {}
                        (None, None) => {
                            self.err(
                                MsgCode::PpMacroArity,
                                format!(
                                    "macro `{name} expects {} argument(s), got {} — formal `{p}` \
                                     has no default",
                                    params.len(),
                                    actuals.len()
                                ),
                                self.out.len(),
                            );
                            self.emit_run(&literal, file, backtick as u32, site_for_collapse);
                            self.line_anchor = saved_anchor;
                            return split.close + 1;
                        }
                    }
                }
                // Pre-expand each actual to completion WITHOUT `name` in active.
                let expanded_actuals: Vec<String> = actuals
                    .iter()
                    .map(|a| self.expand_text_to_string(a, site, depth + 1))
                    .collect();
                // Substitute into the body, then re-scan ONLY body-derived text with
                // `name` held active.
                let substituted = substitute(&mac.body, &params, &expanded_actuals);
                self.active.insert(name.to_string());
                self.macro_depth += 1;
                self.scan_text(&substituted, site, depth + 1);
                self.line_anchor = saved_anchor;
                self.macro_depth = self.macro_depth.saturating_sub(1);
                self.active.remove(name);
                split.close + 1
            }
        }
    }

    /// Expand `text` (an argument actual or an include line) to a finished string,
    /// re-scanning macro uses to a stable result, WITHOUT polluting `self.out`.
    /// Used for pre-expanded actuals (recursion-guard scoping) and include paths.
    pub(crate) fn expand_text_to_string(
        &mut self,
        text: &str,
        site: (FileId, u32),
        depth: u32,
    ) -> String {
        // Swap out the live output buffers AND the pending-newline state, scan into
        // fresh ones, restore. The temp expansion must not flush the parent's pending
        // directive newline into the throwaway buffer, nor leak its own.
        let saved_out = std::mem::take(&mut self.out);
        let saved_segments = std::mem::take(&mut self.segments);
        let saved_pending = self.pending_nl.take();
        let saved_cont = std::mem::take(&mut self.pending_cont);
        // The argument is expanded in the CURRENT emitting context (we only reach
        // here when emitting), and collapses provenance to the use site like any
        // expansion text.
        self.scan_text(text, site, depth);
        let result = std::mem::take(&mut self.out);
        self.out = saved_out;
        self.segments = saved_segments;
        self.pending_nl = saved_pending;
        self.pending_cont = saved_cont;
        result
    }
}
