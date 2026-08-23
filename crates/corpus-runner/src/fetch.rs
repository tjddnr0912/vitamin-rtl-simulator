//! Getting the RTL onto this machine.
//!
//! The corpus is *described* in-repo and *stored* nowhere: `bench/*` is gitignored
//! precisely so that third-party RTL under eight different licences is never
//! redistributed by this project. What ships is the pinned SHA, so the design a
//! number was measured against can always be reconstructed exactly.

use crate::{Origin, Workload, CORPUS};
use std::path::Path;

/// One clone command, ready to print or run.
pub struct FetchStep {
    pub name: &'static str,
    pub repo: &'static str,
    pub sha: &'static str,
    pub license: &'static str,
    pub dest: String,
    pub present: bool,
    /// A first-party script to run after the clone, if the workload has one.
    ///
    /// One workload needs it today: `biriscv` runs on an image extracted from
    /// upstream's own `test.elf`, which makes the image third-party content. Rather
    /// than commit it, `prepare.sh` regenerates it from the pinned checkout.
    pub prepare: Option<String>,
}

/// What would have to be cloned for the corpus to be complete here.
///
/// Returned rather than executed: cloning eight repositories is a side effect on the
/// user's disk and their network, so the default is to show the plan and let
/// `fetch --run` carry it out.
pub fn plan_fetch(root: &Path) -> Vec<FetchStep> {
    CORPUS
        .iter()
        .filter_map(|w: &Workload| match w.origin {
            Origin::FirstParty => None,
            Origin::Upstream { repo, sha, license } => {
                let dest = format!("bench/{}/src", w.root);
                let prep = format!("bench/{}/prepare.sh", w.root);
                Some(FetchStep {
                    name: w.name,
                    repo,
                    sha,
                    license,
                    present: root.join(&dest).is_dir(),
                    prepare: root.join(&prep).is_file().then_some(prep),
                    dest,
                })
            }
        })
        .collect()
}

impl FetchStep {
    /// A shell transcript rather than a spawned process: the reader can see what is
    /// about to touch their disk, and can run it by hand on a machine where this
    /// tool is not built.
    pub fn script(&self) -> String {
        let mut s = format!(
            "git clone --filter=blob:none --no-checkout {repo} {dest}\n\
             git -C {dest} fetch --depth 1 origin {sha}\n\
             git -C {dest} checkout --detach {sha}",
            repo = self.repo,
            dest = self.dest,
            sha = self.sha,
        );
        if let Some(prep) = &self.prepare {
            s.push_str(&format!("\nsh {prep}"));
        }
        s
    }
}
