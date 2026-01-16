use anyhow::{anyhow, Result};
use git2::{DiffFormat, DiffOptions, Repository};

pub struct GitHandler;

impl GitHandler {
    pub fn get_staged_diff() -> Result<String> {
        let repo = Repository::open(".")?;

        // 尝试获取 HEAD 树，如果不存在（如新仓库），则使用空树
        let head_tree = match repo.head().and_then(|h| h.peel_to_tree()) {
            Ok(tree) => Some(tree),
            Err(_) => None,
        };

        let mut opts = DiffOptions::new();
        let diff = repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))?;

        let mut diff_text = Vec::new();
        diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            diff_text.extend_from_slice(line.content());
            true
        })?;

        if diff_text.is_empty() {
            return Err(anyhow!("没有发现已暂存的变更。"));
        }

        Ok(String::from_utf8_lossy(&diff_text).to_string())
    }

    pub fn commit(message: &str) -> Result<String> {
        let repo = Repository::open(".")?;
        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        let sig = repo.signature()?;

        // 尝试获取父提交
        let parent_commits = match repo.head().and_then(|h| h.peel_to_commit()) {
            Ok(parent) => vec![parent],
            Err(_) => vec![], // 没有父提交（初始提交）
        };

        let parents_refs: Vec<&git2::Commit> = parent_commits.iter().collect();

        let commit_id = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents_refs)?;

        Ok(format!("Commit successful: {}", commit_id))
    }

    pub fn get_unstaged_files() -> Result<Vec<String>> {
        let repo = Repository::open(".")?;
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);
        let statuses = repo.statuses(Some(&mut opts))?;

        let mut files = Vec::new();
        for entry in statuses.iter() {
            let status = entry.status();
            if status.is_wt_new()
                || status.is_wt_modified()
                || status.is_wt_deleted()
                || status.is_wt_renamed()
            {
                if let Some(path) = entry.path() {
                    files.push(path.to_string());
                }
            }
        }
        Ok(files)
    }

    pub fn stage_files(paths: Vec<String>) -> Result<String> {
        let repo = Repository::open(".")?;
        let mut index = repo.index()?;

        for path in paths {
            index.add_path(std::path::Path::new(&path))?;
        }

        index.write()?;
        Ok("Files staged successfully".to_string())
    }

}
