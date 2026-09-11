#![allow(non_camel_case_types, non_snake_case, dead_code)]
use std::path::Path;

pub type git_oid = gix::ObjectId;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum git_object_t {
    GIT_OBJECT_COMMIT,
    GIT_OBJECT_TREE,
    GIT_OBJECT_BLOB,
    GIT_OBJECT_OTHER,
}

#[derive(Clone)]
pub struct Repo {
    safe_repo: gix::ThreadSafeRepository,
}

pub fn git_libgit2_init() {
    // No-op for gix
}

impl Repo {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let p = path.as_ref();
        let target_path = if p.join(".kart").exists() {
            p.join(".kart")
        } else if p.join(".git").is_file() {
            if let Ok(content) = std::fs::read_to_string(p.join(".git")) {
                let trimmed = content.trim();
                if let Some(gitdir) = trimmed.strip_prefix("gitdir:") {
                    let rel = gitdir.trim();
                    p.join(rel)
                } else {
                    p.to_path_buf()
                }
            } else {
                p.to_path_buf()
            }
        } else {
            p.to_path_buf()
        };

        let repo = gix::open(&target_path)
            .map_err(|e| format!("Failed to open git repo at {:?}: {}", target_path, e))?;

        Ok(Self {
            safe_repo: repo.into_sync(),
        })
    }

    pub fn to_local(&self) -> gix::Repository {
        self.safe_repo.to_thread_local()
    }

    pub fn get_head_tree(&self) -> Result<Tree, String> {
        let repo = self.to_local();
        let head = repo.head_commit().map_err(|e| format!("Failed to resolve HEAD commit: {}", e))?;
        let tree_id = head.tree_id().map_err(|e| format!("Failed to get HEAD tree id: {}", e))?;
        Ok(Tree {
            repo: self.clone(),
            oid: tree_id.detach(),
        })
    }

    pub fn read_blob(&self, oid: &git_oid) -> Result<Vec<u8>, String> {
        self.with_blob(oid, |s| s.to_vec())
    }

    pub fn with_blob<F, R>(&self, oid: &git_oid, f: F) -> Result<R, String>
    where
        F: FnOnce(&[u8]) -> R,
    {
        let repo = self.to_local();
        let obj = repo.find_object(*oid).map_err(|e| format!("Failed to find blob {}: {}", oid, e))?;
        Ok(f(&obj.data))
    }

    pub fn lookup_tree(&self, oid: &git_oid) -> Result<Tree, String> {
        Ok(Tree {
            repo: self.clone(),
            oid: *oid,
        })
    }
}

pub struct Tree {
    repo: Repo,
    oid: git_oid,
}

impl Tree {
    pub fn id(&self) -> &git_oid {
        &self.oid
    }

    pub fn get_entry_bypath(&self, path: &str) -> Result<(git_oid, git_object_t), String> {
        let repo = self.repo.to_local();
        let tree_obj = repo.find_object(self.oid).map_err(|e| e.to_string())?.into_tree();
        
        let mut current_tree = tree_obj;
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Err("Empty path".to_string());
        }

        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            let mut found = None;
            for entry_res in current_tree.iter() {
                let entry = entry_res.map_err(|e| e.to_string())?;
                if entry.filename() == *part {
                    let oid = entry.oid().to_owned();
                    let mode = entry.mode();
                    let obj_type = if mode.is_tree() {
                        git_object_t::GIT_OBJECT_TREE
                    } else if mode.is_blob() {
                        git_object_t::GIT_OBJECT_BLOB
                    } else {
                        git_object_t::GIT_OBJECT_OTHER
                    };
                    found = Some((oid, obj_type));
                    break;
                }
            }

            match found {
                Some((oid, obj_type)) => {
                    if is_last {
                        return Ok((oid, obj_type));
                    } else if obj_type == git_object_t::GIT_OBJECT_TREE {
                        current_tree = repo.find_object(oid).map_err(|e| e.to_string())?.into_tree();
                    } else {
                        return Err(format!("Component '{}' in path '{}' is not a tree", part, path));
                    }
                }
                None => return Err(format!("Path '{}' not found in tree", path)),
            }
        }

        Err(format!("Path '{}' not found in tree", path))
    }

    pub fn entry_count(&self) -> usize {
        let repo = self.repo.to_local();
        let count = match repo.find_object(self.oid) {
            Ok(obj) => {
                let tree = obj.into_tree();
                tree.iter().count()
            }
            Err(_) => 0,
        };
        count
    }

    pub fn get_entry_by_index(&self, idx: usize) -> Option<(String, git_oid, git_object_t)> {
        let repo = self.repo.to_local();
        let tree_obj = repo.find_object(self.oid).ok()?.into_tree();
        for (i, entry_res) in tree_obj.iter().enumerate() {
            if i == idx {
                let entry = entry_res.ok()?;
                let name = entry.filename().to_string();
                let oid = entry.oid().to_owned();
                let mode = entry.mode();
                let obj_type = if mode.is_tree() {
                    git_object_t::GIT_OBJECT_TREE
                } else if mode.is_blob() {
                    git_object_t::GIT_OBJECT_BLOB
                } else {
                    git_object_t::GIT_OBJECT_OTHER
                };
                return Some((name, oid, obj_type));
            }
        }
        None
    }

    pub fn walk_blobs<F>(&self, mut callback: F) -> Result<(), String>
    where
        F: FnMut(&str, &git_oid) -> Result<(), String>,
    {
        let repo = self.repo.to_local();
        let mut stack = vec![self.oid];

        while let Some(tree_oid) = stack.pop() {
            let tree_obj = repo.find_object(tree_oid).map_err(|e| e.to_string())?.into_tree();
            for entry_res in tree_obj.iter() {
                let entry = entry_res.map_err(|e| e.to_string())?;
                let mode = entry.mode();
                if mode.is_tree() {
                    stack.push(entry.oid().to_owned());
                } else if mode.is_blob() {
                    let name = entry.filename().to_string();
                    let oid = entry.oid().to_owned();
                    callback(&name, &oid)?;
                }
            }
        }

        Ok(())
    }
}
