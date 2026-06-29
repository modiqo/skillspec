use crate::command::{skillspec_command, LabEnvironment};
use crate::temp::TempDir;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct HarnessLab {
    temp: TempDir,
    home: PathBuf,
    project: PathBuf,
    skillspec_home: PathBuf,
    xdg_config_home: PathBuf,
    xdg_cache_home: PathBuf,
    xdg_data_home: PathBuf,
}

impl HarnessLab {
    pub fn new(name: &str) -> Self {
        let temp = TempDir::new(name);
        let home = temp.path().join("home");
        let project = home.join("project");
        let skillspec_home = home.join(".skillspec");
        let xdg_config_home = home.join(".config");
        let xdg_cache_home = home.join(".cache");
        let xdg_data_home = home.join(".local/share");

        for path in [
            home.join(".agents/skills"),
            home.join(".codex/skills"),
            project.join(".claude/skills"),
            skillspec_home.clone(),
            xdg_config_home.clone(),
            xdg_cache_home.clone(),
            xdg_data_home.clone(),
        ] {
            std::fs::create_dir_all(path).unwrap();
        }

        let lab = Self {
            temp,
            home,
            project,
            skillspec_home,
            xdg_config_home,
            xdg_cache_home,
            xdg_data_home,
        };
        lab.assert_no_real_home_writes();
        lab
    }

    pub fn root(&self) -> &Path {
        self.temp.path()
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn project(&self) -> &Path {
        &self.project
    }

    pub fn skillspec_home(&self) -> &Path {
        &self.skillspec_home
    }

    pub fn agents_root(&self) -> PathBuf {
        self.home.join(".agents/skills")
    }

    pub fn codex_root(&self) -> PathBuf {
        self.home.join(".codex/skills")
    }

    pub fn claude_root(&self) -> PathBuf {
        self.project.join(".claude/skills")
    }

    pub fn command(&self) -> Command {
        self.command_at(&self.home)
    }

    pub fn command_in_project(&self) -> Command {
        self.command_at(&self.project)
    }

    pub fn command_at(&self, current_dir: &Path) -> Command {
        assert!(
            current_dir.starts_with(self.root()),
            "harness lab command current_dir must stay under lab root: {}",
            current_dir.display()
        );
        skillspec_command(
            current_dir,
            LabEnvironment {
                home: &self.home,
                skillspec_home: &self.skillspec_home,
                xdg_config_home: &self.xdg_config_home,
                xdg_cache_home: &self.xdg_cache_home,
                xdg_data_home: &self.xdg_data_home,
            },
        )
    }

    pub fn write_skill(
        &self,
        root: &Path,
        folder_name: &str,
        skill_md: &str,
        spec_yml: Option<&str>,
    ) -> PathBuf {
        assert!(
            root.starts_with(self.root()),
            "harness lab fixtures must stay under lab root: {}",
            root.display()
        );
        let skill_dir = root.join(folder_name);
        write_file(&skill_dir.join("SKILL.md"), skill_md);
        if let Some(spec_yml) = spec_yml {
            write_file(&skill_dir.join("skill.spec.yml"), spec_yml);
        }
        skill_dir
    }

    pub fn assert_no_real_home_writes(&self) {
        let real_home = std::env::var_os("HOME").map(PathBuf::from);
        if let Some(real_home) = real_home {
            assert_ne!(
                self.home, real_home,
                "harness lab HOME must not be the developer's real HOME"
            );
        }

        for path in [
            self.home(),
            self.project(),
            self.skillspec_home(),
            self.agents_root().as_path(),
            self.codex_root().as_path(),
            self.claude_root().as_path(),
        ] {
            assert!(
                path.starts_with(self.root()),
                "harness lab path escaped root: {}",
                path.display()
            );
        }
    }
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}
