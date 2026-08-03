//! split part of `pp` (mechanical move).

use super::*;

// ─────────────────────────────────────────────────────────────────────────────
// Directive handlers (§2.5)
// ─────────────────────────────────────────────────────────────────────────────

impl Preprocessor<'_> {
    /// Dispatch a directive. Returns the new cursor (always past the directive line
    /// as consumed).
    pub(crate) fn handle_directive(
        &mut self,
        src: &str,
        file: FileId,
        name: &str,
        backtick: usize,
        name_end: usize,
    ) -> usize {
        match name {
            "define" => self.dir_define(src, file, backtick, name_end),
            "undef" => self.dir_undef(src, file, name_end),
            "include" => self.dir_include(src, file, backtick, name_end),
            "ifdef" | "ifndef" => self.dir_ifdef(src, file, name == "ifdef", name_end),
            "elsif" => self.dir_elsif(src, name_end),
            "else" => self.dir_else(src, name_end),
            "endif" => self.dir_endif(name_end),
            "timescale" => {
                let line = self.consume_logical_line(src, name_end);
                // The directive line is stripped from the output, so `self.out.len()`
                // is the expanded offset where this timescale takes effect for all
                // following modules (file-order inheritance).
                match parse_timescale(&line.text) {
                    Ok(ts) => self.timescales.push((self.out.len(), ts)),
                    Err(msg) => self.err(
                        MsgCode::PpBadDirective,
                        format!("malformed `timescale: {msg}"),
                        self.out.len(),
                    ),
                }
                self.note_dir_newline(file, &line);
                line.cursor
            }
            "line" => {
                let line = self.consume_logical_line(src, name_end);
                self.note_dir_newline(file, &line);
                line.cursor
            }
            // `pragma <expression…>` (IEEE 1800 §22.11): accept-ignore policy —
            // the whole logical line is consumed, nothing is emitted, no diag.
            "pragma" => {
                let line = self.consume_logical_line(src, name_end);
                self.note_dir_newline(file, &line);
                line.cursor
            }
            // `default_nettype <type>` (IEEE 1364-2005 §19.2 / 1800 §22.8) governs
            // whether an undeclared identifier in a §3.5 position becomes an implicit
            // net. The directive line is stripped, so — exactly like `timescale` above —
            // `self.out.len()` is the expanded offset from which it takes effect, and
            // later stages resolve it per module by offset (file-order inheritance,
            // "RULE S" sticky). Only `none` differs in behaviour; every other legal
            // net type (`wire`, `tri`, `wand`, …) means "implicit nets allowed", which
            // is also the state before any directive appears.
            "default_nettype" => {
                let line = self.consume_logical_line(src, name_end);
                let arg = line.text.trim();
                let is_none = arg.split_whitespace().next() == Some("none");
                self.nettype_none.push((self.out.len(), is_none));
                self.note_dir_newline(file, &line);
                line.cursor
            }
            // `begin_keywords "spec"` and `unconnected_drive pull1|pull0` carry an
            // argument on their line; consume the whole logical line (accept-ignore —
            // we keep the full keyword set / drive state). IEEE 1800 §22.14, §22.10.
            "begin_keywords" | "unconnected_drive" => self.consume_one_token(src, file, name_end),
            // `end_keywords` / `nounconnected_drive` take no argument — strip the
            // directive token only, like `celldefine`.
            "celldefine"
            | "endcelldefine"
            | "resetall"
            | "end_keywords"
            | "nounconnected_drive" => name_end,
            _ => name_end,
        }
    }

    /// Capture the rest of the logical line from `from` (continuation-joined). The
    /// terminating NEWLINE is consumed (the directive line is stripped). Returns the
    /// joined text, the cursor past the terminating newline, the byte index of that
    /// terminating newline (or EOF), and how many continuation joins were absorbed.
    pub(crate) fn consume_logical_line(&self, src: &str, from: usize) -> CapturedLine {
        let bytes = src.as_bytes();
        let mut i = from;
        let mut raw = String::new();
        let mut conts: u32 = 0;
        // Capture physical lines, honoring `\`+NL continuation and verbatim contexts.
        loop {
            // Find end of this physical line, respecting strings/comments. We only
            // break on a continuation `\`+NL or a bare top-level newline.
            let mut j = i;
            let mut continued = false;
            while j < bytes.len() {
                match bytes[j] {
                    b'"' => {
                        let (end, _ok) = scan_string(src, j);
                        j = end;
                    }
                    b'/' if j + 1 < bytes.len() && bytes[j + 1] == b'/' => {
                        j = scan_line_comment(src, j);
                    }
                    b'/' if j + 1 < bytes.len() && bytes[j + 1] == b'*' => {
                        j = scan_block_comment(src, j);
                    }
                    b'\\' => {
                        let nl = j + 1 < bytes.len() && bytes[j + 1] == b'\n';
                        let crlf =
                            j + 2 < bytes.len() && bytes[j + 1] == b'\r' && bytes[j + 2] == b'\n';
                        if nl || crlf {
                            continued = true;
                            break;
                        }
                        j += 1;
                    }
                    b'\n' => break,
                    _ => j += utf8_len(bytes[j]),
                }
            }
            raw.push_str(&src[i..j]);
            if continued {
                conts += 1;
                // Skip `\` + (CR)LF, continue capturing the next physical line.
                if j + 1 < bytes.len() && bytes[j + 1] == b'\n' {
                    i = j + 2;
                } else {
                    i = j + 3; // `\` CR LF
                }
                continue;
            }
            // Reached a bare newline or EOF. `j` is the terminating newline byte.
            let cursor = if j < bytes.len() { j + 1 } else { j };
            return CapturedLine {
                text: raw,
                cursor,
                nl_byte: j as u32,
                conts,
            };
        }
    }

    /// Record this directive's stripped line as a pending newline. A maximal run of
    /// consecutive directives collapses to ONE newline; each continuation join in the
    /// run removes one from the pending count (so a fully-continued line yields none).
    /// `flush_pending_nl` emits `max(0, 1 - cont)` newlines just before the next
    /// non-directive output. Only meaningful when `emitting()`.
    pub(crate) fn note_dir_newline(&mut self, file: FileId, line: &CapturedLine) {
        if !self.emitting() {
            return;
        }
        if self.pending_nl.is_none() {
            self.pending_nl = Some((file, line.nl_byte));
        }
        self.pending_cont += line.conts;
    }

    /// Emit the pending directive-line newline(s) verbatim (mapped to the run's first
    /// terminating newline), collapsing a consecutive directive run to one newline.
    pub(crate) fn flush_pending_nl(&mut self) {
        if let Some((file, nl_byte)) = self.pending_nl.take() {
            let cont = std::mem::take(&mut self.pending_cont);
            let n = 1u32.saturating_sub(cont);
            for _ in 0..n {
                self.emit_verbatim("\n", file, nl_byte);
            }
        }
    }

    pub(crate) fn consume_one_token(&mut self, src: &str, file: FileId, from: usize) -> usize {
        let line = self.consume_logical_line(src, from);
        self.note_dir_newline(file, &line);
        line.cursor
    }

    pub(crate) fn dir_define(
        &mut self,
        src: &str,
        file: FileId,
        backtick: usize,
        name_end: usize,
    ) -> usize {
        let captured = self.consume_logical_line(src, name_end);
        let cursor = captured.cursor;
        self.note_dir_newline(file, &captured);
        if !self.emitting() {
            return cursor;
        }
        let joined = join_continuations(&captured.text);
        // Parse NAME at the start of the joined line (after the ws that followed
        // `define`).
        let trimmed = joined.trim_start();
        let lead_ws = joined.len() - trimmed.len();
        let Some((nm, after_name)) = parse_ident(trimmed, 0) else {
            self.err(
                MsgCode::PpBadDirective,
                "`define requires a macro name",
                self.out.len(),
            );
            return cursor;
        };
        let nm = nm.to_string();
        if is_directive_kw(&nm) {
            self.err(
                MsgCode::PpBadDirective,
                "cannot define a directive keyword as a macro",
                self.out.len(),
            );
            return cursor;
        }
        let tail = &trimmed[after_name..];
        // Significant-space: function-like iff `(` IMMEDIATELY follows NAME.
        let (params, body_src): (Option<Vec<String>>, &str) = if tail.starts_with('(') {
            // Parse parameter list up to matching ')'.
            match parse_param_list(tail) {
                Ok((ps, rest)) => (Some(ps), rest),
                Err(msg) => {
                    self.err(MsgCode::PpBadDirective, msg, self.out.len());
                    return cursor;
                }
            }
        } else {
            (None, tail)
        };
        // Body: trim leading ws after NAME/param-list, drop trailing line comment.
        let body_no_comment = strip_trailing_line_comment(body_src);
        let body = body_no_comment.trim_start().to_string();
        // def_byte: start of the body in the ORIGINAL file. Approximate as the
        // backtick site for collapsed provenance; exact body offset isn't surfaced.
        let def_byte = (backtick + 1) as u32;
        let _ = lead_ws;
        let new_mac = Macro {
            params,
            body,
            def_file: file,
            def_byte,
        };
        if let Some(existing) = self.macros.get(&nm) {
            if *existing == new_mac
                || (existing.params == new_mac.params && existing.body == new_mac.body)
            {
                // Identical redefinition: silent.
            } else {
                self.warn(
                    MsgCode::PpMacroRedefined,
                    format!("macro `{nm} redefined with different text"),
                    self.out.len(),
                );
            }
        }
        self.macros.insert(nm, new_mac);
        cursor
    }

    pub(crate) fn dir_undef(&mut self, src: &str, file: FileId, name_end: usize) -> usize {
        let captured = self.consume_logical_line(src, name_end);
        let cursor = captured.cursor;
        self.note_dir_newline(file, &captured);
        if !self.emitting() {
            return cursor;
        }
        let trimmed = captured.text.trim();
        let Some((nm, _)) = parse_ident(trimmed, 0) else {
            self.err(
                MsgCode::PpBadDirective,
                "`undef requires a macro name",
                self.out.len(),
            );
            return cursor;
        };
        if is_directive_kw(nm) {
            self.err(
                MsgCode::PpBadDirective,
                "cannot `undef a directive keyword",
                self.out.len(),
            );
            return cursor;
        }
        if self.macros.remove(nm).is_none() {
            self.warn(
                MsgCode::PpUndefUndefined,
                format!("`undef of macro `{nm} that was never defined"),
                self.out.len(),
            );
        }
        cursor
    }

    pub(crate) fn dir_include(
        &mut self,
        src: &str,
        file: FileId,
        backtick: usize,
        name_end: usize,
    ) -> usize {
        let captured = self.consume_logical_line(src, name_end);
        let cursor = captured.cursor;
        self.note_dir_newline(file, &captured);
        if !self.emitting() {
            return cursor;
        }
        let joined = join_continuations(&captured.text);
        let no_comment = strip_trailing_line_comment(&joined);
        // Macro-expand the captured path text (fixpoint, bounded).
        let expanded = self.expand_text_to_string(no_comment, (file, backtick as u32), 1);
        // Parse: optional ws/comment, exactly one "..." token, optional ws/comment.
        let Some(request) = parse_single_quoted(&expanded) else {
            self.err(
                MsgCode::PpBadDirective,
                "`include requires a single quoted path",
                self.out.len(),
            );
            return cursor;
        };
        if self.inc_depth >= self.opts.max_include_depth {
            self.err(
                MsgCode::PpRecursiveInclude,
                "`include nesting exceeded maximum depth",
                self.out.len(),
            );
            return cursor;
        }
        let current_dir = self.files[file.0 as usize].dir.clone();
        let Ok((disp_name, canon, text)) =
            self.reader
                .resolve(&request, &current_dir, &self.opts.incdirs)
        else {
            self.err(
                MsgCode::PpIncludeNotFound,
                format!("`include \"{request}\" not found on search path"),
                self.out.len(),
            );
            return cursor;
        };
        if self.inc_stack.iter().any(|p| p == &canon) {
            self.err(
                MsgCode::PpRecursiveInclude,
                format!("cyclic `include of \"{request}\""),
                self.out.len(),
            );
            return cursor;
        }
        let dir = canon
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(current_dir);
        let new_id = FileId(self.files.len() as u32);
        self.files.push(SourceFileEntry {
            name: disp_name,
            text,
            canon: Some(canon.clone()),
            dir,
        });
        self.inc_stack.push(canon);
        self.inc_depth += 1;
        self.scan_file(new_id);
        self.inc_stack.pop();
        self.inc_depth = self.inc_depth.saturating_sub(1);
        cursor
    }

    pub(crate) fn dir_ifdef(
        &mut self,
        src: &str,
        file: FileId,
        is_ifdef: bool,
        name_end: usize,
    ) -> usize {
        let captured = self.consume_logical_line(src, name_end);
        let cursor = captured.cursor;
        let trimmed = captured.text.trim();
        let name = parse_ident(trimmed, 0).map(|(n, _)| n.to_string());
        let defined = name
            .as_deref()
            .map(|n| self.macros.contains_key(n))
            .unwrap_or(false);
        let active = defined == is_ifdef;
        let parent_emitting = self.emitting();
        // Flush any pending define-run newline, then anchor this `ifdef` with a
        // verbatim newline mapped to its own terminating-newline byte (emitted even
        // in a dead region) so `open_at` resolves to THIS directive's line, giving
        // distinct unterminated-conditional diagnostics per frame.
        self.flush_pending_nl();
        self.emit_verbatim("\n", file, captured.nl_byte);
        self.cond.push(CondFrame {
            active,
            taken: active,
            seen_else: false,
            parent_emitting,
            open_at: self.out.len().saturating_sub(1) as u32,
        });
        if name.is_none() {
            self.err(
                MsgCode::PpBadDirective,
                "`ifdef/`ifndef requires a macro name",
                self.out.len(),
            );
        }
        cursor
    }

    pub(crate) fn dir_elsif(&mut self, src: &str, name_end: usize) -> usize {
        let captured = self.consume_logical_line(src, name_end);
        let cursor = captured.cursor;
        let trimmed = captured.text.trim();
        let name = parse_ident(trimmed, 0).map(|(n, _)| n.to_string());
        let Some(frame) = self.cond.last_mut() else {
            self.err(
                MsgCode::PpBadDirective,
                "`elsif without matching `ifdef",
                self.out.len(),
            );
            return cursor;
        };
        if frame.seen_else {
            self.err(
                MsgCode::PpBadDirective,
                "`elsif after `else",
                self.out.len(),
            );
            return cursor;
        }
        let defined = match &name {
            Some(n) => self.macros.contains_key(n),
            None => false,
        };
        let frame = self.cond.last_mut().unwrap();
        if frame.taken {
            frame.active = false;
        } else if defined {
            frame.active = true;
            frame.taken = true;
        } else {
            frame.active = false;
        }
        cursor
    }

    pub(crate) fn dir_else(&mut self, src: &str, name_end: usize) -> usize {
        let cursor = self.consume_logical_line(src, name_end).cursor;
        let Some(frame) = self.cond.last_mut() else {
            self.err(
                MsgCode::PpBadDirective,
                "`else without matching `ifdef",
                self.out.len(),
            );
            return cursor;
        };
        if frame.seen_else {
            self.err(MsgCode::PpBadDirective, "duplicate `else", self.out.len());
            return cursor;
        }
        frame.seen_else = true;
        frame.active = !frame.taken;
        frame.taken = true;
        cursor
    }

    pub(crate) fn dir_endif(&mut self, name_end: usize) -> usize {
        if self.cond.pop().is_none() {
            self.err(
                MsgCode::PpBadDirective,
                "`endif without matching `ifdef",
                self.out.len(),
            );
        }
        name_end
    }
}
