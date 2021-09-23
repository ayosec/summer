//! Run tests in shell scripts, and compare the `stdout` with their
//! `.output` files.
//!
//! If `$SUMMER_TEST_UI` is `update`, the `.output` files are replaced
//! with the current output from the scripts.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};
use std::{env, fs};

/// Execute shell scripts in `src/tests/ui`.
///
/// This test is only available on Linux because many scripts relies on
/// Linux-only features.
#[test]
fn run_scripts() {
    let update_output = env::var("SUMMER_TEST_UI").as_deref() == Ok("update");

    let sources = Path::new(file!()).parent().unwrap();

    // Store files only in the current directory has a `target` entry.
    let copy_dir;
    if Path::new("target/debug").exists() {
        let path = Path::new("target/debug/tests/ui");
        fs::create_dir_all(path).unwrap();
        copy_dir = Some(path);
    } else {
        copy_dir = None;
    }

    // Update the executable binary.
    assert!(Command::new("cargo")
        .args(["build", "-q"])
        .status()
        .unwrap()
        .success());

    // Absolute path to the binary.
    //
    // TODO launch cargo-build with `--message-format json`, and extract the
    // path from the artifacts messages.
    let bin_path = Path::new("target/debug/summer").canonicalize().unwrap();

    let mut failed = 0;
    for source in fs::read_dir(sources).unwrap() {
        let source = source.unwrap().path();
        if source.extension() != Some(OsStr::new("sh")) {
            continue;
        }

        let expected_output = source.with_extension("output");

        // Execute the script in a temporary directory.
        let tempdir = tempdir::TempDir::new("summer-ui-tests").unwrap();
        let output = Command::new("bash")
            .arg(source.canonicalize().unwrap())
            .current_dir(tempdir.path())
            .env("SUMMER", &bin_path)
            .env("LS_COLORS", "")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap();

        if output.status.success() {
            if update_output {
                fs::write(expected_output, &output.stdout).unwrap();
            } else {
                let expected = fs::read(expected_output).unwrap();
                if output.stdout != expected {
                    failed += 1;
                    eprintln!("[FAILED] {}", source.display());
                    eprintln!(" EXPECTED {:?}", String::from_utf8_lossy(&expected));
                    eprintln!(" CURRENT  {:?}", String::from_utf8_lossy(&output.stdout));
                }
            }

            // Write output streams in `copy_dir`.
            if let Some(dir) = copy_dir {
                let stem = source.file_stem().unwrap();
                fs::write(dir.join(stem).with_extension("stdout"), &output.stdout).unwrap();
                fs::write(dir.join(stem).with_extension("stderr"), &output.stderr).unwrap();
            }
        } else {
            failed += 1;

            eprintln!("[FAILED] {}", source.display());
            eprintln!("= STDOUT =\n{}", String::from_utf8_lossy(&output.stdout));
            eprintln!("= STDERR =\n{}", String::from_utf8_lossy(&output.stderr));
        }
    }

    assert_eq!(failed, 0);
}
