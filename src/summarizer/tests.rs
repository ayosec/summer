use super::gitdiff::Change;
use std::fs::{self, File};
use std::process::{Command, Stdio};
use tempdir::TempDir;

const TEST_CONFIG: &str = r#"
colors:
  when: always
  use_lscolors: true
  styles:
  - indicator: "✎"
    matchers:
      - glob: "*.txt"
  - color: yellow blue
    matchers:
      - changes: 2 days

info:
  variables:
    caps: [ regex: '^[A-Z]' ]
    dirs: [ type: directory ]

grid:
  max_rows: 15
  column_padding: 4

columns:
- matchers: [ { type: directory } ]

- matchers: [ { changes: git } ]

- matchers:
  - mime: text
  - glob: "*.plaintext"
  - regex: "^[A-Z0-9]+$"

- matchers: [ any ]
  include_hidden: true

collector:
  disk_usage: true
  git_diff: true
  timeout: 10 seconds
"#;

#[test]
fn collect_dir_data() {
    let root = TempDir::new("summer").unwrap();
    let config_path = root.path().join("config.yaml");

    // Configuration.
    fs::write(&config_path, TEST_CONFIG.as_bytes()).unwrap();
    let config = crate::config::load(&config_path).unwrap();

    // Populate directory.
    macro_rules! add_file {
        ($name:expr, $len:expr) => {
            let path = root.path().join($name);

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create_dir_all");
            }

            let file = File::create(path).expect($name);

            if $len > 0 {
                file.set_len($len).expect("set_len");
            }

            drop(file);
        };
    }

    add_file!("CHANGES.txt", 0);
    add_file!(".X.txt", 0);
    add_file!("README", 0);
    add_file!("X.plaintext", 0);
    add_file!("other", 250);
    add_file!("src/a", 0);
    add_file!("src/b", 2000);
    add_file!("target/c", 100);
    add_file!("target/d", 50);

    fs::write(root.path().join("README"), "A\nB\nC\n").unwrap();

    // Initialize a Git repository.
    macro_rules! run_git {
        ($args:expr) => {
            let mut git = Command::new("git");
            git.current_dir(root.path())
                .stdout(Stdio::null())
                .args(["-c", "init.defaultBranch=x"])
                .args(["-c", "user.name=ab"])
                .args(["-c", "user.email=a@b"]);

            for arg in $args.split_whitespace() {
                git.arg(arg);
            }

            assert_eq!(git.status().unwrap().code(), Some(0));
        };
    }

    run_git!("init");
    run_git!("add .");
    run_git!("commit -m x");

    // Modify some files in the repository.
    fs::write(root.path().join("README"), "B\nB\n").unwrap();
    fs::write(root.path().join("NEW"), "\n\n").unwrap();
    fs::write(root.path().join("src").join("a"), ".\n.\n.\n").unwrap();
    fs::write(root.path().join("src").join("a2"), "\n\n").unwrap();
    run_git!("add .");

    // Analyze path.
    let analysis = super::analyzer::analyze_path(root.path(), &config).unwrap();
    let groups = analysis.groups;

    macro_rules! assert_file {
        ($group:expr, $file:expr,) => {};

        ($group:expr, $file:expr, name = $name:expr, $($t:tt)*) => {
            assert_eq!(
                groups[$group].files[$file].file_name.to_str(),
                Some($name)
            );

            assert_file!($group, $file, $($t)*);
        };

        ($group:expr, $file:expr, change = ($i:expr, $d:expr), $($t:tt)*) => {
            assert_eq!(
                groups[$group].files[$file].git_changes,
                Some(Change::new($i,$d))
            );

            assert_file!($group, $file, $($t)*);
        };

        ($group:expr, $file:expr, disk_usage = $disk:expr, $(,$t:tt)*) => {
            assert_eq!(
                groups[$group].files[$file].tree_info.as_ref().unwrap().get().map(|ti| ti.disk_usage),
                Some($disk)
            );

            assert_file!($group, $file, $($t)*);
        };
    }

    // Check group contents

    assert_file!(0, 0, name = "src", change = (5, 0), disk_usage = 2008,);
    assert_file!(0, 1, name = "target", disk_usage = 150,);
    assert_eq!(groups[0].files.len(), 2);

    assert_file!(1, 0, name = "NEW", change = (2, 0),);
    assert_file!(1, 1, name = "README", change = (1, 2),);
    assert_eq!(groups[1].files.len(), 2);

    assert_file!(2, 0, name = "CHANGES.txt",);
    assert_file!(2, 1, name = "X.plaintext",);
    assert_eq!(groups[2].files.len(), 2);

    assert_file!(3, 0, name = ".X.txt",);
    assert_file!(3, 1, name = ".git",);
    assert_file!(3, 2, name = "config.yaml",);
    assert_file!(3, 3, name = "other",);
    assert_eq!(groups[3].files.len(), 4);

    assert_eq!(groups.len(), 4);

    // Check variables.
    let variables = analysis.variables;
    assert_eq!(variables["caps"], 4);
    assert_eq!(variables["dirs"], 3);
}
