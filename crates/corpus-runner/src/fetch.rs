//! Getting the RTL onto this machine.
//!
//! The corpus is *described* in-repo and *stored* nowhere: `bench/*` is gitignored
//! precisely so that third-party RTL, under each upstream's own licence, is never
//! redistributed by this project. What ships is the pinned SHA, so the design a
//! number was measured against can always be reconstructed exactly.

use crate::{Origin, Workload, CORPUS};
use std::path::{Component, Path, PathBuf};

/// What is on disk at a row's clone path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloneState {
    /// Nothing there: `fetch --run` clones it.
    Missing,
    /// A checkout whose `HEAD` resolves to the pinned SHA and which holds every file the
    /// row lists inside it.
    Present,
    /// Something is there, but not that checkout: a clone whose fetch, cone or checkout
    /// failed half-way, another commit, or a cone that misses a listed file. Treating
    /// it as present would print "already present" on every later `fetch --run` while
    /// `run` grades the row `absent`. `fetch --run` removes it and clones again, unless
    /// it holds local changes ([`clear_stale`]); the string says why.
    Stale(String),
}

/// One clone command, ready to print or run.
pub struct FetchStep {
    pub name: &'static str,
    pub repo: &'static str,
    pub sha: &'static str,
    pub license: &'static str,
    pub dest: String,
    pub state: CloneState,
    /// Directories to check out instead of the whole tree; empty for all of it.
    pub sparse: &'static [&'static str],
    /// A first-party script to run after the clone, if the workload has one.
    ///
    /// One workload needs it today: `biriscv` runs on an image extracted from
    /// upstream's own `test.elf`, which makes the image third-party content. Rather
    /// than commit it, `prepare.sh` regenerates it from the pinned checkout.
    pub prepare: Option<String>,
}

/// What would have to be cloned for the corpus to be complete here.
///
/// Returned rather than executed: cloning the upstream repositories is a side effect on
/// the user's disk and their network, so the default is to show the plan and let
/// `fetch --run` carry it out.
pub fn plan_fetch(root: &Path) -> Vec<FetchStep> {
    CORPUS
        .iter()
        .filter_map(|w: &Workload| match w.origin {
            Origin::FirstParty => None,
            Origin::Upstream {
                repo,
                sha,
                license,
                sparse,
            } => {
                let dest = format!("bench/{}/src", w.root);
                let prep = format!("bench/{}/prepare.sh", w.root);
                Some(FetchStep {
                    name: w.name,
                    repo,
                    sha,
                    license,
                    state: clone_state(root, w, sha),
                    sparse,
                    prepare: root.join(&prep).is_file().then_some(prep),
                    dest,
                })
            }
        })
        .collect()
}

/// Whether `bench/<root>/src` holds the pinned checkout of `w`: its `HEAD` resolves to
/// `sha` ([`resolve_head`]), and every file of `w.files` and `w.data` that lies inside the
/// clone exists. Files outside the clone (the testbench, a `prepare.sh` product) are not
/// this check's business. The listed-file check is what still marks a clone whose
/// checkout never ran stale when the remote's default branch happens to be the pin.
pub fn clone_state(root: &Path, w: &Workload, sha: &str) -> CloneState {
    let dest = root.join("bench").join(w.root).join("src");
    if !dest.exists() {
        return CloneState::Missing;
    }
    let head = match resolve_head(&dest.join(".git")) {
        Ok(h) => h,
        Err(why) => return CloneState::Stale(why),
    };
    if head != sha {
        return CloneState::Stale(format!("HEAD resolves to {head}, not the pinned {sha}"));
    }
    let clone = Path::new(w.root).join("src");
    for f in w.files.iter().chain(w.data) {
        // `bench/<dir>/<f>` with `..` resolved: darkriscv runs inside its clone.
        let mut p = PathBuf::new();
        for c in Path::new(w.dir).join(f).components() {
            match c {
                Component::ParentDir => {
                    p.pop();
                }
                Component::CurDir => {}
                c => p.push(c),
            }
        }
        if p.starts_with(&clone) && !root.join("bench").join(&p).is_file() {
            return CloneState::Stale(format!("listed file {f} is missing"));
        }
    }
    CloneState::Present
}

/// The commit a clone's `HEAD` names, read from the files git keeps, without running git:
/// the SHA itself for a detached `HEAD` (what `fetch --run` leaves), otherwise the target
/// of `ref: <r>` — the loose ref file `.git/<r>`, else the `<sha> <r>` line of
/// `.git/packed-refs`. A clone made by hand (`git clone` then `git checkout <sha>`, or a
/// branch that was reset to the pin) sits on a branch at the pin and must read present.
/// A `ref:` that resolves nowhere — a clone whose checkout never happened names a branch
/// it never created — is an error, and the clone is stale.
pub fn resolve_head(git: &Path) -> Result<String, String> {
    let head = std::fs::read_to_string(git.join("HEAD"))
        .map_err(|e| format!("no readable .git/HEAD ({e})"))?;
    let head = head.trim();
    let Some(r) = head.strip_prefix("ref:").map(str::trim) else {
        return Ok(head.to_string());
    };
    if let Ok(target) = std::fs::read_to_string(git.join(r)) {
        return Ok(target.trim().to_string());
    }
    if let Ok(packed) = std::fs::read_to_string(git.join("packed-refs")) {
        for line in packed.lines() {
            if line.starts_with('#') || line.starts_with('^') {
                continue;
            }
            if let Some((target, name)) = line.split_once(' ') {
                if name.trim() == r {
                    return Ok(target.to_string());
                }
            }
        }
    }
    Err(format!("HEAD names {r}, which does not resolve"))
}

/// Whether a manifest root is one plain path component: no separator, no `.` or `..`,
/// not absolute, no whitespace. Only then is `bench/<root>/src` a directory of its own
/// that [`clear_stale`] may remove.
pub fn root_is_plain(r: &str) -> bool {
    let mut c = Path::new(r).components();
    matches!((c.next(), c.next()), (Some(Component::Normal(_)), None))
        && !r.contains(['/', '\\'])
        && !r.contains(char::is_whitespace)
}

/// Remove a stale clone of `w` so that the plan's `git clone` can run. Returns whether
/// anything was removed: a missing or present clone is left alone.
///
/// The path is rebuilt here from the row's root, which must be one plain component, and
/// the state is measured again right before anything is removed, so a plan computed
/// earlier cannot delete a clone that has since become the pinned one. A clone that
/// holds anything besides `.git` is removed only when `git status --porcelain
/// --ignored=no` runs and prints nothing: local edits or untracked files (or a status that
/// cannot be read) make this an error and nothing is removed — the clone is someone's
/// work, and `fetch --run` fails for that row instead. A clone holding only `.git` (its
/// checkout never ran) is removed without asking git.
pub fn clear_stale(root: &Path, w: &Workload, sha: &str) -> Result<bool, String> {
    if !root_is_plain(w.root) {
        return Err(format!(
            "root {:?} is not one plain path component; nothing removed",
            w.root
        ));
    }
    let dest = root.join("bench").join(w.root).join("src");
    if !matches!(clone_state(root, w, sha), CloneState::Stale(_)) {
        return Ok(false);
    }
    let entries = std::fs::read_dir(&dest)
        .map_err(|e| format!("cannot read {}: {e}", dest.display()))?
        .map(|e| e.map(|e| e.file_name()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("cannot read {}: {e}", dest.display()))?;
    if entries.iter().any(|n| n != ".git") {
        // The clone's own repository and nothing else: a broken `.git` must fail here,
        // not resolve to an enclosing checkout (the clone sits inside this repository).
        let status = std::process::Command::new("git")
            .arg("--git-dir")
            .arg(dest.join(".git"))
            .arg("--work-tree")
            .arg(&dest)
            .args(["status", "--porcelain", "--ignored=no"])
            .output();
        match status {
            Ok(o) if o.status.success() && o.stdout.is_empty() => {}
            Ok(o) if o.status.success() => {
                let changes = String::from_utf8_lossy(&o.stdout);
                let first: Vec<&str> = changes.lines().take(5).collect();
                return Err(format!(
                    "{} is stale but has local changes ({}); nothing removed — move it aside \
                     or remove it by hand",
                    dest.display(),
                    first.join(", ")
                ));
            }
            Ok(o) => {
                return Err(format!(
                    "{} is stale and `git status` failed ({}): {}; nothing removed",
                    dest.display(),
                    o.status,
                    String::from_utf8_lossy(&o.stderr).trim()
                ))
            }
            Err(e) => {
                return Err(format!(
                    "{} is stale and `git status` could not run ({e}); nothing removed",
                    dest.display()
                ))
            }
        }
    }
    std::fs::remove_dir_all(&dest).map_err(|e| format!("cannot remove {}: {e}", dest.display()))?;
    Ok(true)
}

impl FetchStep {
    /// A shell transcript rather than a spawned process: the reader can see what is
    /// about to touch their disk, and can run it by hand on a machine where this
    /// tool is not built.
    ///
    /// A sparse row narrows the checkout before anything is checked out, so the blobs
    /// outside its directories are never downloaded (the clone is `blob:none`).
    pub fn script(&self) -> String {
        let mut s = format!(
            "git clone --filter=blob:none --no-checkout {repo} {dest}\n",
            repo = self.repo,
            dest = self.dest,
        );
        if !self.sparse.is_empty() {
            s.push_str(&format!(
                "git -C {dest} sparse-checkout set --cone {dirs}\n",
                dest = self.dest,
                dirs = self.sparse.join(" "),
            ));
        }
        s.push_str(&format!(
            "git -C {dest} fetch --depth 1 origin {sha}\n\
             git -C {dest} checkout --detach {sha}",
            dest = self.dest,
            sha = self.sha,
        ));
        if let Some(prep) = &self.prepare {
            s.push_str(&format!("\nsh {prep}"));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(sparse: &'static [&'static str]) -> FetchStep {
        FetchStep {
            name: "w",
            repo: "https://example.invalid/r",
            sha: "0123456789abcdef0123456789abcdef01234567",
            license: "MIT",
            dest: "bench/w/src".into(),
            state: CloneState::Missing,
            sparse,
            prepare: None,
        }
    }

    /// The whole-tree recipe is the three lines every row used before sparse rows.
    #[test]
    fn a_whole_tree_row_clones_fetches_and_checks_out() {
        let lines: Vec<String> = step(&[]).script().lines().map(String::from).collect();
        assert_eq!(
            lines,
            [
                "git clone --filter=blob:none --no-checkout https://example.invalid/r bench/w/src",
                "git -C bench/w/src fetch --depth 1 origin 0123456789abcdef0123456789abcdef01234567",
                "git -C bench/w/src checkout --detach 0123456789abcdef0123456789abcdef01234567",
            ]
        );
    }

    /// A sparse row sets its cone after the blob-less clone and before the checkout,
    /// so only the blobs inside the cone are ever fetched.
    #[test]
    fn a_sparse_row_sets_its_cone_before_the_checkout() {
        let s = step(&["hw/a", "hw/b"]).script();
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(
            lines[1],
            "git -C bench/w/src sparse-checkout set --cone hw/a hw/b"
        );
        assert!(lines[0].starts_with("git clone --filter=blob:none --no-checkout "));
        assert!(lines[3].starts_with("git -C bench/w/src checkout --detach "));
    }

    /// Every manifest row's plan carries its own cone, and only a row that names one
    /// gets a sparse line.
    #[test]
    fn the_plan_carries_each_rows_cone() {
        for s in plan_fetch(Path::new("/nonexistent")) {
            let w = CORPUS.iter().find(|w| w.name == s.name).expect("row");
            let Origin::Upstream { sparse, .. } = w.origin else {
                panic!("{}: a first-party row in the fetch plan", w.name)
            };
            assert_eq!(s.sparse, sparse, "{}", w.name);
            assert_eq!(
                s.script().contains("sparse-checkout"),
                !sparse.is_empty(),
                "{}",
                w.name
            );
        }
    }

    const PIN: &str = "0123456789abcdef0123456789abcdef01234567";

    /// A throwaway repository root under the system temp directory, removed on drop.
    struct TempRoot(PathBuf);
    impl TempRoot {
        fn new(tag: &str) -> Self {
            let p = std::env::temp_dir()
                .join(format!("corpus-runner-fetch-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).expect("temp root");
            TempRoot(p)
        }
        fn write(&self, rel: &str, text: &str) {
            let p = self.0.join(rel);
            std::fs::create_dir_all(p.parent().expect("parent")).expect("dirs");
            std::fs::write(p, text).expect("write");
        }
    }
    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A row like `darkriscv`: it runs inside its clone and names one clone file through
    /// `..`, one directly, and a data file outside the clone that no check may demand.
    fn row() -> Workload {
        use crate::{Expect, Shape};
        Workload {
            name: "w",
            origin: Origin::Upstream {
                repo: "https://example.invalid/r",
                sha: PIN,
                license: "MIT",
                sparse: &[],
            },
            shape: Shape::Cpu,
            root: "w",
            dir: "w/src/sim",
            vita_args: &[],
            iverilog_args: &[],
            files: &["../../tb.v", "../rtl/a.v", "b.v"],
            data: &["../../prog.hex"],
            plusargs: &[],
            digest: "DIGEST=0",
            expect: Expect::Runs { exit: 0 },
            oracle: "iverilog 13.0",
            note: "",
        }
    }

    /// A negative control: its name, how it breaks a good checkout, and the reason the
    /// stale verdict must give.
    type Breakage = (&'static str, fn(&TempRoot), &'static str);

    fn good(t: &TempRoot) {
        t.write("bench/w/src/.git/HEAD", &format!("{PIN}\n"));
        t.write("bench/w/src/rtl/a.v", "");
        t.write("bench/w/src/sim/b.v", "");
    }

    #[test]
    fn a_checkout_at_the_pin_with_every_listed_file_is_present() {
        let t = TempRoot::new("present");
        assert_eq!(clone_state(&t.0, &row(), PIN), CloneState::Missing);
        good(&t);
        // tb.v and prog.hex sit outside the clone and are absent on purpose.
        assert_eq!(clone_state(&t.0, &row(), PIN), CloneState::Present);
    }

    /// A clone made by hand sits on a branch at the pin: `HEAD` is `ref: refs/heads/…`
    /// and the SHA is in the loose ref file or in `packed-refs`. Both read present.
    #[test]
    fn a_branch_head_that_resolves_to_the_pin_is_present() {
        let t = TempRoot::new("branch-loose");
        good(&t);
        t.write("bench/w/src/.git/HEAD", "ref: refs/heads/master\n");
        t.write("bench/w/src/.git/refs/heads/master", &format!("{PIN}\n"));
        assert_eq!(clone_state(&t.0, &row(), PIN), CloneState::Present);

        let t = TempRoot::new("branch-packed");
        good(&t);
        t.write("bench/w/src/.git/HEAD", "ref: refs/heads/main\n");
        t.write(
            "bench/w/src/.git/packed-refs",
            &format!(
                "# pack-refs with: peeled fully-peeled sorted\n\
                 ffffffffffffffffffffffffffffffffffffffff refs/heads/other\n\
                 {PIN} refs/heads/main\n\
                 ^eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee\n"
            ),
        );
        assert_eq!(clone_state(&t.0, &row(), PIN), CloneState::Present);
    }

    /// Negative controls: each one breaks a single property of a good checkout, and each
    /// must read stale, so `fetch --run` re-clones instead of printing "already present".
    #[test]
    fn a_wrong_head_a_missing_file_or_no_checkout_is_stale() {
        let cases: [Breakage; 6] = [
            (
                "wrong-sha",
                |t| {
                    t.write(
                        "bench/w/src/.git/HEAD",
                        "ffffffffffffffffffffffffffffffffffffffff\n",
                    )
                },
                "resolves to ffffffffffffffffffffffffffffffffffffffff, not the pinned",
            ),
            (
                "branch-at-another-sha",
                |t| {
                    t.write("bench/w/src/.git/HEAD", "ref: refs/heads/master\n");
                    t.write(
                        "bench/w/src/.git/refs/heads/master",
                        "ffffffffffffffffffffffffffffffffffffffff\n",
                    );
                },
                "not the pinned",
            ),
            (
                // A clone whose checkout never ran: HEAD names a branch with no ref file
                // and no packed-refs line.
                "never-checked-out",
                |t| t.write("bench/w/src/.git/HEAD", "ref: refs/heads/master\n"),
                "HEAD names refs/heads/master, which does not resolve",
            ),
            (
                "packed-refs-without-the-branch",
                |t| {
                    t.write("bench/w/src/.git/HEAD", "ref: refs/heads/master\n");
                    t.write(
                        "bench/w/src/.git/packed-refs",
                        &format!("{PIN} refs/remotes/origin/master\n"),
                    );
                },
                "does not resolve",
            ),
            (
                "missing-file",
                |t| std::fs::remove_file(t.0.join("bench/w/src/rtl/a.v")).expect("rm"),
                "../rtl/a.v is missing",
            ),
            (
                "no-git",
                |t| std::fs::remove_dir_all(t.0.join("bench/w/src/.git")).expect("rm"),
                "no readable .git/HEAD",
            ),
        ];
        for (tag, breaks, why) in cases {
            let t = TempRoot::new(tag);
            good(&t);
            assert_eq!(clone_state(&t.0, &row(), PIN), CloneState::Present, "{tag}");
            breaks(&t);
            match clone_state(&t.0, &row(), PIN) {
                CloneState::Stale(s) => assert!(s.contains(why), "{tag}: {s}"),
                other => panic!("{tag}: {other:?}"),
            }
        }
    }

    #[test]
    fn a_root_must_be_one_plain_component() {
        for ok in ["w", "verilog-axis", "a.b", "opentitan-prims"] {
            assert!(root_is_plain(ok), "{ok:?} should pass");
        }
        for bad in [
            "", ".", "..", "a/b", "/abs", "a/", "a b", " a", "a\\b", "a\tb",
        ] {
            assert!(!root_is_plain(bad), "{bad:?} should be refused");
        }
    }

    /// `clear_stale` leaves a present clone alone, removes one whose checkout never ran
    /// (only `.git`) without asking git, refuses one whose status cannot be read, and
    /// refuses a row whose root is not one plain component.
    #[test]
    fn only_a_stale_clone_without_work_in_it_is_cleared() {
        let t = TempRoot::new("clear");
        good(&t);
        assert_eq!(clear_stale(&t.0, &row(), PIN), Ok(false));
        assert!(t.0.join("bench/w/src/rtl/a.v").is_file());

        // Only `.git`, naming a branch it never created: removed.
        let t = TempRoot::new("clear-unfinished");
        t.write("bench/w/src/.git/HEAD", "ref: refs/heads/main\n");
        assert!(matches!(
            clone_state(&t.0, &row(), PIN),
            CloneState::Stale(_)
        ));
        assert_eq!(clear_stale(&t.0, &row(), PIN), Ok(true));
        assert_eq!(clone_state(&t.0, &row(), PIN), CloneState::Missing);

        // Files beside a `.git` git cannot read: the status check fails, nothing goes.
        let t = TempRoot::new("clear-unreadable");
        good(&t);
        t.write(
            "bench/w/src/.git/HEAD",
            "ffffffffffffffffffffffffffffffffffffffff\n",
        );
        let e = clear_stale(&t.0, &row(), PIN).expect_err("must refuse");
        assert!(e.contains("nothing removed"), "{e}");
        assert!(t.0.join("bench/w/src/rtl/a.v").is_file());

        // A root that is not one plain component is refused before anything is read.
        let mut w = row();
        w.root = "../w";
        let e = clear_stale(&t.0, &w, PIN).expect_err("must refuse");
        assert!(e.contains("not one plain path component"), "{e}");
        assert!(t.0.join("bench/w/src/rtl/a.v").is_file());
    }

    /// The same checks against a real repository made by git: a branch `HEAD` at the pin
    /// through a loose ref and, after `git pack-refs`, through `packed-refs`; then, under
    /// another pin, a stale clone with an untracked file is refused and the same clone
    /// without it is removed. Skips where git cannot run (the RHEL test job has none).
    #[test]
    fn a_real_clone_on_a_branch_and_its_local_changes() {
        let t = TempRoot::new("real");
        t.write("bench/w/src/rtl/a.v", "module a; endmodule\n");
        t.write("bench/w/src/sim/b.v", "module b; endmodule\n");
        let dest = t.0.join("bench/w/src");
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .arg("-C")
                .arg(&dest)
                .args([
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                    "-c",
                    "core.hooksPath=/dev/null",
                ])
                .args(args)
                .output()
        };
        match git(&["init", "-q", "-b", "main"]) {
            Ok(o) if o.status.success() => {}
            _ => {
                eprintln!("SKIP a_real_clone_on_a_branch_and_its_local_changes: git unavailable");
                return;
            }
        }
        for args in [&["add", "-A"][..], &["commit", "-q", "-m", "x"][..]] {
            let o = git(args).expect("git");
            assert!(o.status.success(), "git {args:?}: {o:?}");
        }
        let o = git(&["rev-parse", "HEAD"]).expect("git");
        let real = String::from_utf8(o.stdout)
            .expect("utf-8")
            .trim()
            .to_string();
        let head = std::fs::read_to_string(dest.join(".git/HEAD")).expect("HEAD");
        assert!(head.starts_with("ref: "), "{head}");
        assert_eq!(clone_state(&t.0, &row(), &real), CloneState::Present);
        let o = git(&["pack-refs", "--all"]).expect("git");
        assert!(o.status.success());
        assert!(!dest.join(".git/refs/heads/main").exists());
        assert_eq!(clone_state(&t.0, &row(), &real), CloneState::Present);

        // Under another pin the clone is stale; an untracked file is local work.
        t.write("bench/w/src/notes.txt", "mine\n");
        let e = clear_stale(&t.0, &row(), PIN).expect_err("must refuse");
        assert!(
            e.contains("local changes") && e.contains("notes.txt"),
            "{e}"
        );
        assert!(dest.join("notes.txt").is_file());
        std::fs::remove_file(dest.join("notes.txt")).expect("rm");
        assert_eq!(clear_stale(&t.0, &row(), PIN), Ok(true));
        assert!(!dest.exists());
    }
}
