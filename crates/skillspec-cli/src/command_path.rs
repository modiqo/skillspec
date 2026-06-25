use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub fn find_on_path(command: &str) -> Option<PathBuf> {
    if command.contains(std::path::MAIN_SEPARATOR) {
        let path = PathBuf::from(command);
        return is_executable_file(&path).then_some(path);
    }
    let path = env::var_os("PATH")?;
    let candidates = command_candidates(command);
    env::split_paths(&path).find_map(|directory| {
        candidates
            .iter()
            .map(|candidate| directory.join(candidate))
            .find(|candidate| is_executable_file(candidate))
    })
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

fn command_candidates(command: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        let mut candidates = vec![command.to_owned()];
        if Path::new(command).extension().is_none() {
            let path_ext = env::var_os("PATHEXT")
                .map(|value| {
                    value
                        .to_string_lossy()
                        .split(';')
                        .filter(|extension| !extension.is_empty())
                        .map(|extension| extension.to_owned())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| {
                    vec![
                        ".COM".to_owned(),
                        ".EXE".to_owned(),
                        ".BAT".to_owned(),
                        ".CMD".to_owned(),
                    ]
                });
            candidates.extend(
                path_ext
                    .into_iter()
                    .map(|extension| format!("{command}{extension}")),
            );
        }
        candidates
    }
    #[cfg(not(windows))]
    {
        vec![command.to_owned()]
    }
}
