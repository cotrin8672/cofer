use anyhow::{Context, Result};
use git2::{FetchOptions, PushOptions, Repository, RepositoryInitOptions, WorktreeAddOptions};
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub async fn init_remote_repository(repository_path: &Path) -> Result<()> {
    let repo = Repository::open(repository_path)
        .with_context(|| format!("not a git repository: {}", repository_path.display()))?;

    let cofer_repo_path = repository_path.join(".cofer");
    if !cofer_repo_path.exists() {
        let mut opts = RepositoryInitOptions::new();
        opts.bare(true);
        Repository::init_opts(&cofer_repo_path, &opts).with_context(|| {
            format!("failed to initialize repository: {}", cofer_repo_path.display())
        })?;
    } else {
        let _ = Repository::open_bare(&cofer_repo_path)
            .with_context(|| format!("failed to open repository: {}", cofer_repo_path.display()))?;
    }

    if repo.find_remote("cofer").is_err() {
        repo.remote("cofer", cofer_repo_path.to_string_lossy().as_ref())
            .with_context(|| format!("failed to add remote: {}", cofer_repo_path.display()))?;
    }

    Ok(())
}

pub fn fetch_from_cofer(project_root: &Path, refspecs: &[&str]) -> Result<()> {
    let repo = Repository::open(project_root)
        .with_context(|| format!("not a git repo: {}", project_root.display()))?;

    let mut remote =
        repo.find_remote("cofer").with_context(|| "remote 'cofer' not found (run init first)")?;

    let mut fo = FetchOptions::new();
    remote.fetch(refspecs, Some(&mut fo), None).with_context(|| "git fetch from 'cofer' failed")?;

    Ok(())
}

pub fn create_branch(project_root: &Path, base_ref: &str, branch_name: &str) -> Result<()> {
    let repo = Repository::open(project_root)
        .with_context(|| format!("not a git repo: {}", project_root.display()))?;

    let obj = repo
        .revparse_single(base_ref)
        .with_context(|| format!("cannot resolve base ref: {}", base_ref))?;
    let base_oid = obj.peel_to_commit()?.id();

    let mut remote = repo
        .find_remote("cofer")
        .with_context(|| "remote 'cofer' not found (run init first)".to_string())?;

    let refspec = format!("+{}:refs/heads/cofer/{}", base_oid, branch_name);
    let mut po = PushOptions::new();
    remote
        .push(&[refspec.as_str()], Some(&mut po))
        .with_context(|| "git push to 'cofer' failed")?;

    Ok(())
}

pub fn create_worktree_from_cofer(project_root: &Path, branch_name: &str) -> Result<PathBuf> {
    let cofer_repo_path = project_root.join(".cofer/remote.git");
    let cofer = Repository::open_bare(&cofer_repo_path)
        .with_context(|| format!("not a bare repo: {}", cofer_repo_path.display()))?;

    let refname = format!("refs/heads/cofer/{}", branch_name);
    let reference = cofer
        .find_reference(&refname)
        .with_context(|| format!("reference {} not found", refname))?;

    let wt_path = project_root.join(".cofer/worktrees").join(branch_name);

    if wt_path.exists() {
        return Ok(wt_path);
    }

    let mut opts = WorktreeAddOptions::new();
    opts.reference(Some(&reference));

    cofer
        .worktree(branch_name, &wt_path, Some(&mut opts))
        .with_context(|| format!("failed to add worktree at {}", wt_path.display()))?;

    Ok(wt_path)
}

pub fn ensure_gitignore_has_cofer(project_root: &Path) -> Result<()> {
    let gitignore_path = project_root.join(".gitignore");

    // ファイルを読み込んで ".cofer" を含む行があるか確認
    let mut already = false;
    if gitignore_path.exists() {
        let file = fs::File::open(&gitignore_path)
            .with_context(|| format!("failed to open {}", gitignore_path.display()))?;
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            let trimmed = line.trim_end_matches('/');
            if trimmed == ".cofer" {
                already = true;
                break;
            }
        }
    }

    // なければ追記
    if !already {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitignore_path)
            .with_context(|| format!("failed to open {}", gitignore_path.display()))?;
        writeln!(f, ".cofer/")?;
    }

    Ok(())
}
