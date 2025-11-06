use anyhow::Result;
use git2::{Repository, BranchType, Status, StatusOptions};
use std::path::Path;
use std::string::String;
use crate::progress::ProgressHelper;

pub struct GitOperations {
    repo: Option<Repository>,
    progress: ProgressHelper,
}

impl std::fmt::Debug for GitOperations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitOperations")
            .field("repo", &self.repo.is_some())
            .field("progress", &"ProgressHelper")
            .finish()
    }
}

impl GitOperations {
    pub fn new() -> Self {
        Self {
            repo: None,
            progress: ProgressHelper::new(),
        }
    }

    pub fn open_repository<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.repo = Some(Repository::open(path)?);
        Ok(())
    }

    pub fn initialize_repository<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.repo = Some(Repository::init(path)?);
        Ok(())
    }

    pub fn current_branch(&self) -> Result<String> {
        match &self.repo {
            Some(repo) => {
                let head = repo.head()?;
                let reference = head.resolve()?;
                let branch_name = reference.shorthand().unwrap_or("HEAD");
                Ok(branch_name.to_string())
            }
            None => Err(anyhow::anyhow!("No repository opened"))
        }
    }

    pub fn list_branches(&self) -> Result<Vec<String>> {
        match &self.repo {
            Some(repo) => {
                let mut branches = Vec::new();

                // Local branches
                for branch_result in repo.branches(Some(BranchType::Local))? {
                    let (branch, _type) = branch_result?;
                    if let Some(name) = branch.name()? {
                        branches.push(format!("  {}", name));
                    }
                }

                // Remote branches
                for branch_result in repo.branches(Some(BranchType::Remote))? {
                    let (branch, _type) = branch_result?;
                    if let Some(name) = branch.name()? {
                        branches.push(format!("  remotes/{}", name));
                    }
                }

                Ok(branches)
            }
            None => Err(anyhow::anyhow!("No repository opened"))
        }
    }

    pub fn create_branch(&self, branch_name: &str) -> Result<()> {
        match &self.repo {
            Some(repo) => {
                let commit = repo.head()?.peel_to_commit()?;
                repo.branch(branch_name, &commit, false)?;
                println!("✅ Created branch: {}", branch_name);
                Ok(())
            }
            None => Err(anyhow::anyhow!("No repository opened"))
        }
    }

    pub fn checkout_branch(&self, branch_name: &str) -> Result<()> {
        match &self.repo {
            Some(repo) => {
                // Try to find the branch
                let _branch = repo.find_branch(branch_name, BranchType::Local)?;
                repo.set_head(&format!("refs/heads/{}", branch_name))?;
                println!("✅ Switched to branch: {}", branch_name);
                Ok(())
            }
            None => Err(anyhow::anyhow!("No repository opened"))
        }
    }

    pub fn delete_branch(&self, branch_name: &str) -> Result<()> {
        match &self.repo {
            Some(repo) => {
                let mut branch = repo.find_branch(branch_name, BranchType::Local)?;

                // Check if it's the current branch
                let current_branch = self.current_branch()?;
                if current_branch == branch_name {
                    return Err(anyhow::anyhow!("Cannot delete current branch. Switch to another branch first."));
                }

                // Delete the branch
                branch.delete()?;
                println!("✅ Deleted branch: {}", branch_name);
                Ok(())
            }
            None => Err(anyhow::anyhow!("No repository opened"))
        }
    }

    pub fn status(&self) -> Result<Vec<String>> {
        match &self.repo {
            Some(repo) => {
                let mut status_output = Vec::new();

                let mut opts = StatusOptions::default();
                opts.include_untracked(true);
                opts.include_ignored(false);

                let statuses = repo.statuses(Some(&mut opts))?;

                if statuses.is_empty() {
                    status_output.push("✅ Working directory clean".to_string());
                } else {
                    for status in &statuses {
                        if let Some(path) = status.path() {
                            let status_flags = status.status();
                            if status_flags.contains(Status::INDEX_NEW) {
                                status_output.push(format!("  + {}", path));
                            } else if status_flags.contains(Status::INDEX_MODIFIED) {
                                status_output.push(format!("  M {}", path));
                            } else if status_flags.contains(Status::INDEX_DELETED) {
                                status_output.push(format!("  D {}", path));
                            } else if status_flags.contains(Status::WT_NEW) {
                                status_output.push(format!("  ?? {}", path));
                            } else if status_flags.contains(Status::WT_MODIFIED) {
                                status_output.push(format!("  M {}", path));
                            } else if status_flags.contains(Status::WT_DELETED) {
                                status_output.push(format!("  D {}", path));
                            } else if status_flags.contains(Status::IGNORED) {
                                // Skip ignored files
                            } else {
                                status_output.push(format!("  ? {}", path));
                            }
                        }
                    }
                }

                Ok(status_output)
            }
            None => Err(anyhow::anyhow!("No repository opened"))
        }
    }

    pub fn add_all(&self) -> Result<()> {
        match &self.repo {
            Some(repo) => {
                let mut index = repo.index()?;
                let mut added = Vec::new();

                // Add all untracked files
                let mut opts = StatusOptions::default();
                opts.include_untracked(true);
                for status in &repo.statuses(Some(&mut opts))? {
                    if status.status().contains(Status::WT_NEW) {
                        if let Some(path_str) = status.path() {
                            let path = Path::new(path_str);
                            index.add_path(path)?;
                            added.push(path_str.to_string());
                        }
                    }
                }

                index.write()?;

                if added.is_empty() {
                    println!("ℹ️  No new files to add");
                } else {
                    println!("✅ Added files:");
                    for file in added {
                        println!("  {}", file);
                    }
                }
                Ok(())
            }
            None => Err(anyhow::anyhow!("No repository opened"))
        }
    }

    pub fn commit(&self, message: &str) -> Result<()> {
        match &self.repo {
            Some(repo) => {
                let signature = repo.signature()?;
                let mut index = repo.index()?;

                // Write the index
                index.write()?;

                // Create tree
                let tree_id = index.write_tree()?;
                let tree = repo.find_tree(tree_id)?;

                // Get parent commit
                let parent_commit = repo.head()
                    .ok()
                    .and_then(|head| head.peel_to_commit().ok());

                // Create commit
                let commit_id = if let Some(parent) = parent_commit {
                    repo.commit(
                        Some("HEAD"),
                        &signature,
                        &signature,
                        message,
                        &tree,
                        &[&parent],
                    )?
                } else {
                    repo.commit(
                        Some("HEAD"),
                        &signature,
                        &signature,
                        message,
                        &tree,
                        &[],
                    )?
                };

                println!("✅ Created commit: {}", commit_id);
                Ok(())
            }
            None => Err(anyhow::anyhow!("No repository opened"))
        }
    }

    }

impl Drop for GitOperations {
    fn drop(&mut self) {
        self.progress.finish();
    }
}