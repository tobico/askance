//! What the CLI derives from the working directory: the project, the branch,
//! and the Diff.
//!
//! Agents never supply these — determinism over trust (ADR-0001). Everything
//! here shells out to `git` rather than linking a git library: the answers are
//! one-liners, and shelling out means the CLI sees exactly what the agent sees
//! when it runs git itself.
//!
//! Nothing here fails: outside a repository, or with no git on the PATH, each
//! field is simply absent. A Question Set is worth putting to the human either
//! way.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The fields the CLI fills in on every Set. All three are absent outside a
/// git repository; the Diff is also absent when the tree is clean.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Enrichment {
    pub project: Option<String>,
    pub branch: Option<String>,
    pub diff: Option<String>,
}

/// Derive everything the CLI attaches to a Set, as seen from `dir`.
pub fn enrichment(dir: &Path) -> Enrichment {
    Enrichment {
        project: project(dir),
        branch: branch(dir),
        diff: diff(dir),
    }
}

/// The repository's name, worktree-smart: from a linked worktree this is the
/// root repo's name, because that is the project the human recognises.
///
/// `--git-common-dir` is the root repo's `.git` however deep, and however
/// linked, the working directory is — unlike `--git-dir`, which in a linked
/// worktree points inside `.git/worktrees/`.
pub fn project(dir: &Path) -> Option<String> {
    let common = git(dir, &["rev-parse", "--git-common-dir"])?;

    // The path comes back relative to `dir` in the main worktree and absolute
    // in a linked one; joining and canonicalising settles both, and resolves
    // the `..` in the `../.git` a subdirectory reports.
    let common = dir.join(common.trim()).canonicalize().ok()?;

    let name = match common.file_name().and_then(OsStr::to_str)? {
        // The ordinary layout: the repository is `.git`'s parent.
        ".git" => common.parent()?.file_name().and_then(OsStr::to_str)?,
        // A bare or separate-git-dir repository is named by the directory
        // itself, conventionally `<name>.git`.
        directory => directory.strip_suffix(".git").unwrap_or(directory),
    };

    Some(name.to_owned())
}

/// The branch checked out where the CLI is running. A detached HEAD has no
/// branch to report, so the commit stands in — the human still needs to know
/// what the agent was looking at.
pub fn branch(dir: &Path) -> Option<String> {
    let current = git(dir, &["branch", "--show-current"])?;
    if !current.trim().is_empty() {
        return Some(current.trim().to_owned());
    }

    let commit = git(dir, &["rev-parse", "--short", "HEAD"])?;
    Some(commit.trim().to_owned()).filter(|commit| !commit.is_empty())
}

/// The repository's uncommitted changes: everything not in the last commit,
/// staged or not, plus the contents of untracked files. Binary contents are
/// left out — git says the file differs and stops there.
///
/// `None` when the tree is clean or `dir` is not in a repository.
pub fn diff(dir: &Path) -> Option<String> {
    // The whole tree is the subject, not whichever directory the agent happened
    // to run in: `git ls-files` is relative to the working directory, so all of
    // this runs from the top of the worktree.
    let root = PathBuf::from(git(dir, &["rev-parse", "--show-toplevel"])?.trim());
    if root.as_os_str().is_empty() {
        return None;
    }

    // Tracked changes, staged and unstaged, in one patch. Before the first
    // commit there is no HEAD to compare against and the index stands in.
    let mut diff = git(&root, &["diff", "HEAD"])
        .or_else(|| git(&root, &["diff", "--cached"]))
        .unwrap_or_default();

    // An untracked file has nothing to diff against, so it is compared with the
    // empty file. `--no-index` exits 1 when the two differ, which for a
    // non-empty untracked file is the ordinary case.
    for path in untracked(&root) {
        if let Some(patch) = run(
            &root,
            &["diff", "--no-index", "--", "/dev/null", &path],
            &[0, 1],
        ) {
            diff.push_str(&patch);
        }
    }

    (!diff.is_empty()).then_some(diff)
}

/// The paths of files git knows nothing about, ignores aside, relative to
/// `root`. NUL-separated so a path with a newline in it survives the trip.
fn untracked(root: &Path) -> Vec<String> {
    git(root, &["ls-files", "-z", "--others", "--exclude-standard"])
        .unwrap_or_default()
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Run git in `dir` and take its stdout, or `None` if it failed.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    run(dir, args, &[0])
}

/// Run git in `dir`, accepting any of the `ok` exit codes.
fn run(dir: &Path, args: &[&str], ok: &[i32]) -> Option<String> {
    let output = Command::new("git")
        // Reading the working tree should never take a lock: the CLI is on its
        // way to submitting a Question Set, not driving git.
        .arg("--no-optional-locks")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !ok.contains(&output.status.code()?) {
        return None;
    }

    // Paths and patches are whatever bytes the filesystem holds; the Set is
    // UTF-8 either way, so anything else is replaced rather than refused.
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}
