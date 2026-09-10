#![allow(non_camel_case_types, non_snake_case, dead_code)]
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::Path;

pub type git_object_size_t = u64;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct git_oid {
    pub id: [u8; 20],
}

impl git_oid {
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(40);
        for b in &self.id {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum git_object_t {
    GIT_OBJECT_ANY = -2,
    GIT_OBJECT_INVALID = -1,
    GIT_OBJECT_COMMIT = 1,
    GIT_OBJECT_TREE = 2,
    GIT_OBJECT_BLOB = 3,
    GIT_OBJECT_TAG = 4,
}

#[repr(C)]
pub enum git_treewalk_mode {
    GIT_TREEWALK_PRE = 0,
    GIT_TREEWALK_POST = 1,
}

#[repr(C)]
pub struct git_error {
    pub message: *mut c_char,
    pub klass: c_int,
}

pub type git_repository = c_void;
pub type git_object = c_void;
pub type git_tree = c_void;
pub type git_tree_entry = c_void;
pub type git_blob = c_void;

pub type git_treewalk_cb =
    unsafe extern "C" fn(root: *const c_char, entry: *const git_tree_entry, payload: *mut c_void) -> c_int;

extern "C" {
    pub fn git_libgit2_init() -> c_int;
    pub fn git_libgit2_shutdown() -> c_int;

    pub fn git_repository_open(out: *mut *mut git_repository, path: *const c_char) -> c_int;
    pub fn git_repository_free(repo: *mut git_repository);

    pub fn git_revparse_single(out: *mut *mut git_object, repo: *mut git_repository, spec: *const c_char) -> c_int;
    pub fn git_object_free(obj: *mut git_object);
    pub fn git_object_peel(peeled: *mut *mut git_object, obj: *const git_object, target_type: git_object_t) -> c_int;

    pub fn git_tree_free(tree: *mut git_tree);
    pub fn git_tree_entry_bypath(
        out: *mut *mut git_tree_entry,
        root: *const git_tree,
        path: *const c_char,
    ) -> c_int;
    pub fn git_tree_entry_free(entry: *mut git_tree_entry);
    pub fn git_tree_entry_id(entry: *const git_tree_entry) -> *const git_oid;
    pub fn git_tree_entry_type(entry: *const git_tree_entry) -> git_object_t;
    pub fn git_tree_entry_name(entry: *const git_tree_entry) -> *const c_char;
    pub fn git_tree_entrycount(tree: *const git_tree) -> usize;
    pub fn git_tree_entry_byindex(tree: *const git_tree, idx: usize) -> *const git_tree_entry;

    pub fn git_tree_lookup(out: *mut *mut git_tree, repo: *mut git_repository, id: *const git_oid) -> c_int;
    pub fn git_tree_walk(
        tree: *const git_tree,
        mode: git_treewalk_mode,
        cb: git_treewalk_cb,
        payload: *mut c_void,
    ) -> c_int;

    pub fn git_blob_lookup(out: *mut *mut git_blob, repo: *mut git_repository, id: *const git_oid) -> c_int;
    pub fn git_blob_rawcontent(blob: *const git_blob) -> *const c_void;
    pub fn git_blob_rawsize(blob: *const git_blob) -> git_object_size_t;
    pub fn git_blob_free(blob: *mut git_blob);

    pub fn git_error_last() -> *const git_error;
}

fn last_git_error(default_msg: &str) -> String {
    unsafe {
        let err = git_error_last();
        if !err.is_null() && !(*err).message.is_null() {
            CStr::from_ptr((*err).message)
                .to_string_lossy()
                .into_owned()
        } else {
            default_msg.to_string()
        }
    }
}

pub struct Repo {
    raw: *mut git_repository,
}

unsafe impl Send for Repo {}
unsafe impl Sync for Repo {}

impl Repo {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let p = path.as_ref();
        let target_path = if p.join(".kart").exists() {
            p.join(".kart")
        } else if p.join(".git").is_file() {
            // Check if .git file points to .kart
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

        let c_path = CString::new(target_path.to_str().ok_or("Invalid repo path UTF-8")?)
            .map_err(|e| e.to_string())?;

        let mut raw: *mut git_repository = std::ptr::null_mut();
        let code = unsafe { git_repository_open(&mut raw, c_path.as_ptr()) };
        if code != 0 {
            return Err(format!(
                "Failed to open git repository at {:?}: {}",
                target_path,
                last_git_error("unknown error")
            ));
        }

        Ok(Self { raw })
    }

    pub fn get_head_tree(&self) -> Result<Tree, String> {
        let c_spec = CString::new("HEAD^{tree}").unwrap();
        let mut obj: *mut git_object = std::ptr::null_mut();
        let code = unsafe { git_revparse_single(&mut obj, self.raw, c_spec.as_ptr()) };
        if code != 0 {
            return Err(format!(
                "Failed to resolve HEAD tree: {}",
                last_git_error("unknown error")
            ));
        }

        let mut tree_obj: *mut git_object = std::ptr::null_mut();
        let peel_code =
            unsafe { git_object_peel(&mut tree_obj, obj, git_object_t::GIT_OBJECT_TREE) };
        unsafe { git_object_free(obj) };

        if peel_code != 0 {
            return Err(format!(
                "Failed to peel HEAD to tree: {}",
                last_git_error("unknown error")
            ));
        }

        Ok(Tree {
            raw: tree_obj as *mut git_tree,
        })
    }

    pub fn read_blob(&self, oid: &git_oid) -> Result<Vec<u8>, String> {
        self.with_blob(oid, |slice| slice.to_vec())
    }

    pub fn with_blob<F, R>(&self, oid: &git_oid, f: F) -> Result<R, String>
    where
        F: FnOnce(&[u8]) -> R,
    {
        let mut blob: *mut git_blob = std::ptr::null_mut();
        let code = unsafe { git_blob_lookup(&mut blob, self.raw, oid) };
        if code != 0 {
            return Err(format!(
                "Failed to lookup blob {}: {}",
                oid.to_hex(),
                last_git_error("unknown error")
            ));
        }

        let size = unsafe { git_blob_rawsize(blob) } as usize;
        let ptr = unsafe { git_blob_rawcontent(blob) } as *const u8;
        let res = if !ptr.is_null() && size > 0 {
            let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
            f(slice)
        } else {
            f(&[])
        };

        unsafe { git_blob_free(blob) };
        Ok(res)
    }

    pub fn lookup_tree(&self, oid: &git_oid) -> Result<Tree, String> {
        let mut tree: *mut git_tree = std::ptr::null_mut();
        let code = unsafe { git_tree_lookup(&mut tree, self.raw, oid) };
        if code != 0 {
            return Err(format!(
                "Failed to lookup tree {}: {}",
                oid.to_hex(),
                last_git_error("unknown error")
            ));
        }
        Ok(Tree { raw: tree })
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { git_repository_free(self.raw) };
        }
    }
}

pub struct Tree {
    raw: *mut git_tree,
}

impl Tree {
    pub fn get_entry_bypath(&self, path: &str) -> Result<(git_oid, git_object_t), String> {
        let c_path = CString::new(path).map_err(|e| e.to_string())?;
        let mut entry: *mut git_tree_entry = std::ptr::null_mut();
        let code = unsafe { git_tree_entry_bypath(&mut entry, self.raw, c_path.as_ptr()) };
        if code != 0 {
            return Err(format!("Path not found in tree '{}': {}", path, last_git_error("not found")));
        }

        let oid = unsafe { *git_tree_entry_id(entry) };
        let type_ = unsafe { git_tree_entry_type(entry) };
        unsafe { git_tree_entry_free(entry) };

        Ok((oid, type_))
    }

    pub fn entry_count(&self) -> usize {
        unsafe { git_tree_entrycount(self.raw) }
    }

    pub fn get_entry_by_index(&self, idx: usize) -> Option<(String, git_oid, git_object_t)> {
        let entry = unsafe { git_tree_entry_byindex(self.raw, idx) };
        if entry.is_null() {
            return None;
        }
        let name_ptr = unsafe { git_tree_entry_name(entry) };
        let name = unsafe { CStr::from_ptr(name_ptr) }
            .to_string_lossy()
            .into_owned();
        let oid = unsafe { *git_tree_entry_id(entry) };
        let type_ = unsafe { git_tree_entry_type(entry) };
        Some((name, oid, type_))
    }

    pub fn walk_blobs<F>(&self, callback: F) -> Result<(), String>
    where
        F: FnMut(&str, &git_oid) -> Result<(), String>,
    {
        struct Payload<'a> {
            cb: Box<dyn FnMut(&str, &git_oid) -> Result<(), String> + 'a>,
            err: Option<String>,
        }

        let mut payload = Payload {
            cb: Box::new(callback),
            err: None,
        };

        unsafe extern "C" fn walk_cb(
            _root: *const c_char,
            entry: *const git_tree_entry,
            payload: *mut c_void,
        ) -> c_int {
            let p = &mut *(payload as *mut Payload);
            let type_ = git_tree_entry_type(entry);
            if type_ == git_object_t::GIT_OBJECT_BLOB {
                let name_ptr = git_tree_entry_name(entry);
                let name_str = CStr::from_ptr(name_ptr).to_string_lossy();
                let oid = &*git_tree_entry_id(entry);
                if let Err(e) = (p.cb)(&name_str, oid) {
                    p.err = Some(e);
                    return -1;
                }
            }
            0
        }

        let code = unsafe {
            git_tree_walk(
                self.raw,
                git_treewalk_mode::GIT_TREEWALK_PRE,
                walk_cb,
                &mut payload as *mut _ as *mut c_void,
            )
        };

        if let Some(err) = payload.err {
            return Err(err);
        }

        if code != 0 {
            return Err(format!("Tree walk failed: {}", last_git_error("error")));
        }

        Ok(())
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { git_tree_free(self.raw) };
        }
    }
}
