(§5.2 Q-FLISTUTF8 · rank ② · source Q-STRBYTES design review z17 (N2)) (a `-f` file list re-encodes a UTF-8 path as Latin-1):
* symptom: a `-f` file list re-encodes a UTF-8 path as Latin-1: `cli/src/filelist.rs:128` `out.push(b[i] as char)` (`expand_env`, which every token passes through) turns `dé.sv` (`64 c3 a9`) into `d\xc3\x83\xc2\xa9.sv`, so `vita -f files.f` exits 3 with `error[VITA-E8005]: cannot read '…/d\xc3\x83\xc2\xa9.sv': No such file or directory (os error 2)` (PRE 9d60771d and Q-STRBYTES §4.5.629 alike) while the same file on the command line runs (`FL ok`, exit 0)
* fix: copy the token's text between `$` introducers as `&str` slices (the token is already UTF-8)
* fix: census the other filelist readers
