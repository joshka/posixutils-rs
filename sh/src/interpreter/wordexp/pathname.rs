use crate::interpreter::wordexp::pattern::FilenamePattern;
use std::ffi::{CString, OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
enum DirEntry {
    /// filename
    File(OsString),
    /// dir name
    Dir(OsString),
}

type DirContent = Vec<DirEntry>;

trait FileSystem {
    fn read_dir(&self, path: &Path) -> DirContent;
}

struct DefaultFileSystem;

impl FileSystem for DefaultFileSystem {
    fn read_dir(&self, path: &Path) -> DirContent {
        let mut result = DirContent::default();
        let dir_iter = if let Ok(iter) = std::fs::read_dir(path) {
            iter
        } else {
            return result;
        };
        for entry in dir_iter {
            let entry = if let Ok(item) = entry { item } else { continue };
            let file_type = if let Ok(file_type) = entry.file_type() {
                file_type
            } else {
                continue;
            };
            // TODO: symlinks
            if file_type.is_file() {
                result.push(DirEntry::File(entry.file_name()))
            } else if file_type.is_dir() {
                result.push(DirEntry::Dir(entry.file_name()))
            }
        }
        result
    }
}

fn list_files_rec(
    filesystem: &dyn FileSystem,
    pattern: &FilenamePattern,
    component_index: usize,
    current_directory: &mut PathBuf,
    prefix: &mut PathBuf,
    result: &mut Vec<OsString>,
) {
    let add_to_result = component_index == pattern.component_count();
    for entry in filesystem.read_dir(Path::new(current_directory)) {
        match entry {
            DirEntry::File(file_name) if add_to_result => {
                let file_name_cstring =
                    CString::new(file_name.clone().into_encoded_bytes()).unwrap();
                if pattern.matches_all(component_index, &file_name_cstring) {
                    let mut path = prefix.clone();
                    path.push(file_name);
                    result.push(path.into_os_string())
                }
            }
            DirEntry::Dir(dir_name) => {
                prefix.push(&dir_name);
                current_directory.push(&dir_name);
                let dir_name_cstring = CString::new(dir_name.into_encoded_bytes()).unwrap();
                if pattern.matches_all(component_index, &dir_name_cstring) {
                    if add_to_result {
                        result.push(prefix.clone().into_os_string())
                    } else {
                        list_files_rec(
                            filesystem,
                            pattern,
                            component_index + 1,
                            current_directory,
                            prefix,
                            result,
                        );
                    }
                }
                prefix.pop();
                current_directory.pop();
            }
            _ => {}
        }
    }
}

fn list_files(
    filesystem: &dyn FileSystem,
    pattern: &FilenamePattern,
    current_directory: &OsStr,
) -> Vec<OsString> {
    let mut result = Vec::new();

    if pattern.component_count() == 0 {
        // no patterns to match
        return Vec::new();
    }

    if pattern.is_absolute() {
        list_files_rec(
            filesystem,
            pattern,
            1,
            &mut PathBuf::from("/"),
            &mut PathBuf::from("/"),
            &mut result,
        );
    } else {
        list_files_rec(
            filesystem,
            pattern,
            1,
            &mut PathBuf::from(current_directory),
            &mut PathBuf::new(),
            &mut result,
        );
    }
    // TODO: sort properly taking into account LC_COLLATE
    result.sort();
    result
}

/// # Panics
/// panics if `starting_directory` is not an absolute path
pub fn glob(pattern: &FilenamePattern, starting_directory: &Path) -> Vec<OsString> {
    assert!(starting_directory.is_absolute());
    list_files(
        &DefaultFileSystem {},
        pattern,
        starting_directory.as_os_str(),
    )
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::interpreter::wordexp::pattern::tests::filename_pattern_from_str;
    use std::collections::hash_map::Entry;
    use std::collections::HashMap;

    type Directory = HashMap<String, FileSystemNode>;

    enum FileSystemNode {
        File,
        Directory(Directory),
    }

    impl FileSystemNode {
        fn unwrap_dir_mut(&mut self) -> &mut Directory {
            if let FileSystemNode::Directory(dir) = self {
                dir
            } else {
                unreachable!()
            }
        }
    }

    #[derive(Default)]
    pub struct TestFileSystem {
        root: Directory,
    }

    impl TestFileSystem {
        pub fn get_dir(&self, path: &Path) -> Option<&Directory> {
            let mut current_dir = &self.root;
            for entry in path.into_iter() {
                let next_dir_name = entry.to_str().unwrap();
                match current_dir.get(next_dir_name)? {
                    FileSystemNode::File => return None,
                    FileSystemNode::Directory(dir) => {
                        current_dir = dir;
                    }
                }
            }
            Some(current_dir)
        }

        pub fn add_file(mut self, path: &str) -> Self {
            let path = PathBuf::from(path);
            let mut current_dir = &mut self.root;
            if let Some(file_path) = path.parent() {
                for part in file_path.iter() {
                    match current_dir.entry(part.to_str().unwrap().to_string()) {
                        Entry::Vacant(e) => {
                            current_dir = e
                                .insert(FileSystemNode::Directory(Directory::new()))
                                .unwrap_dir_mut();
                        }
                        Entry::Occupied(e) => current_dir = e.into_mut().unwrap_dir_mut(),
                    }
                }
            }
            current_dir.insert(
                path.file_name().unwrap().to_str().unwrap().to_string(),
                FileSystemNode::File,
            );
            self
        }
    }

    impl FileSystem for TestFileSystem {
        fn read_dir(&self, path: &Path) -> DirContent {
            let mut items = Vec::new();
            if let Some(dir) = self.get_dir(path) {
                for (name, item) in dir {
                    let name = OsString::from(name);
                    match item {
                        FileSystemNode::File => {
                            items.push(DirEntry::File(name));
                        }
                        FileSystemNode::Directory(_) => {
                            items.push(DirEntry::Dir(name));
                        }
                    }
                }
            }
            items
        }
    }

    #[test]
    fn list_root() {
        let filesystem = TestFileSystem::default()
            .add_file("/file1")
            .add_file("/file2")
            .add_file("/file3")
            .add_file("/dir1/file4")
            .add_file("/dir2/file5");
        let pattern = filename_pattern_from_str("/*");
        assert_eq!(
            list_files(&filesystem, &pattern, OsStr::new("/")),
            vec![
                OsString::from("/dir1"),
                "/dir2".into(),
                "/file1".into(),
                "/file2".into(),
                "/file3".into(),
            ]
        )
    }

    #[test]
    fn list_current_dir() {
        let filesystem = TestFileSystem::default()
            .add_file("/dir/file1")
            .add_file("/dir/file2")
            .add_file("/dir/file3")
            .add_file("/dir/dir2/file4")
            .add_file("/dir/dir3/file5");
        let pattern = filename_pattern_from_str("*");
        assert_eq!(
            list_files(&filesystem, &pattern, OsStr::new("/dir/")),
            vec![
                OsString::from("dir2"),
                "dir3".into(),
                "file1".into(),
                "file2".into(),
                "file3".into(),
            ]
        );
    }

    #[test]
    fn nothing_is_listed_if_pattern_matches_nothing() {
        let filesystem = TestFileSystem::default()
            .add_file("/file1")
            .add_file("/file2")
            .add_file("/file3");
        let pattern = filename_pattern_from_str("file");
        assert!(list_files(&filesystem, &pattern, OsStr::new("/")).is_empty());
    }
}
