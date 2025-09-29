use anyhow::Result;
use clap::{Parser, Subcommand};
use git2::{Repository, WorktreeAddOptions};

#[derive(Parser)]
#[command(name = "cofer")]
#[command(about = "Container environment manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new container environment
    Create {
        /// Environment name
        name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Create { name } => {
            create_environment(&name).await?;
        }
    }

    Ok(())
}

async fn create_environment(name: &str) -> Result<()> {
    println!("Creating environment: {}", name);

    let repo = Repository::open(".")?;
    println!("Repository opened: {:?}", repo.path());

    let branch_name = format!("cofer/{}", name);

    let short_branch = format!("cofer/{}", name);
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;

    match repo.branch(&branch_name, &commit, false) {
        Ok(_) => println!("Branch created: {}", branch_name),
        Err(e) if e.code() == git2::ErrorCode::Exists => {
            println!("Branch already exists: {}", branch_name);
        }
        Err(e) => return Err(e.into()),
    }

    let repo_workdir =
        repo.workdir().ok_or_else(|| anyhow::anyhow!("This appears to be a bare repository"))?;
    let worktrees_root = repo_workdir.join(".cofer/worktrees");
    let worktree_path = worktrees_root.join(name);

    if worktree_path.exists() {
        return Err(anyhow::anyhow!(
            "Worktree '{}' already exists at {:?}. Remove it first with 'git worktree remove {}'",
            name,
            worktree_path,
            name
        ));
    }

    if repo.find_worktree(name).is_ok() {
        return Err(anyhow::anyhow!(
            "Worktree '{}' already registered in git. Remove it first with 'git worktree remove {}'",
            name,
            name
        ));
    }

    std::fs::create_dir_all(worktree_path.parent().unwrap())?;

    println!("Creating worktree at: {:?}", worktree_path);

    let branch_ref = repo.find_reference(&format!("refs/heads/{}", short_branch))?;
    let mut options = WorktreeAddOptions::new();
    options.reference(Some(&branch_ref));

    let worktree = repo.worktree(name, &worktree_path, Some(&options))?;

    println!("✓ Worktree created at: {:?}", worktree.path());

    Ok(())
}
