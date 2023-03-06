use super::traits::AppIndex;
use std::{fs, os::unix::prelude::PermissionsExt, path::Path};

/**
 * Creates application indexes from the PATH
 */
pub fn index() -> Vec<AppIndex> {
    let path = match std::env::var("PATH") {
        Ok(path) => path,
        Err(_) => return vec![],
    };

    path.split(':')
        .flat_map(|path| match fs::read_dir(path) {
            Ok(files) => files
                .filter_map(|file| file.ok())
                .filter(|file| {
                    let meta = match file.metadata() {
                        Ok(meta) => meta,
                        Err(_) => return false,
                    };

                    let is_file = meta.is_file();
                    let is_executable = meta.permissions().mode() & 0o111 != 0;

                    is_file && is_executable
                })
                .filter_map(|file| {
                    Some(AppIndex {
                        actions: vec![],
                        exec: Path::new(path).join(file.path()).to_str()?.to_owned(),
                        generic_name: None,
                        name: file.file_name().to_str()?.to_owned(),
                    })
                })
                .collect(),
            Err(_) => vec![],
        })
        .collect()
}
