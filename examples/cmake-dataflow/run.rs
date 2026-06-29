use dora_cli::{Executable, RunCommand};
use eyre::{Context, bail};
use std::{path::Path, time::Duration};

fn main() -> eyre::Result<()> {
    if cfg!(windows) {
        tracing::error!(
            "The c++ example does not work on Windows currently because of a linker error"
        );
        return Ok(());
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::env::set_current_dir(root.join(file!()).parent().unwrap())
        .wrap_err("failed to set working dir")?;

    std::fs::create_dir_all("build")?;
    let mut cmd = std::process::Command::new("cmake");
    cmd.arg(format!("-DDORA_ROOT_DIR={}", root.display()));
    cmd.arg("-B").arg("build");
    cmd.arg(".");
    if !cmd
        .status()
        .wrap_err("failed to run `cmake`; install CMake and ensure it is on PATH")?
        .success()
    {
        bail!("failed to generate CMake build files");
    }

    let mut cmd = std::process::Command::new("cmake");
    cmd.arg("--build").arg("build");
    if !cmd
        .status()
        .wrap_err("failed to run `cmake --build`; install CMake and ensure it is on PATH")?
        .success()
    {
        bail!("failed to build the CMake-generated project binary tree");
    }

    let mut cmd = std::process::Command::new("cmake");
    cmd.arg("--install").arg("build");
    if !cmd
        .status()
        .wrap_err("failed to run `cmake --install`; install CMake and ensure it is on PATH")?
        .success()
    {
        bail!("failed to install the CMake-generated project binary tree");
    }

    build_package("dora-runtime")?;

    prepend_target_debug_to_path(root).wrap_err("failed to prepare dora runtime PATH")?;

    // Bound the run so a wedged node fails fast via the daemon's stop
    // escalation instead of hanging until the CI step timeout (#2152).
    // A healthy run self-terminates quickly.
    let mut run = RunCommand::new("dataflow.yml".to_string());
    run.stop_after = Some(Duration::from_secs(120));
    run.execute()?;

    Ok(())
}

fn prepend_target_debug_to_path(root: &Path) -> eyre::Result<()> {
    let mut paths = vec![root.join("target").join("debug")];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    let path = std::env::join_paths(paths).wrap_err("failed to compose PATH")?;
    // SAFETY: this example runner is single-threaded here and updates PATH
    // before Dora starts runtime child processes.
    unsafe {
        std::env::set_var("PATH", path);
    }
    Ok(())
}

fn build_package(package: &str) -> eyre::Result<()> {
    let cargo = std::env::var("CARGO").unwrap();
    let mut cmd = std::process::Command::new(&cargo);
    cmd.arg("build");
    cmd.arg("--package").arg(package);
    if !cmd.status()?.success() {
        bail!("failed to build {package}");
    }
    Ok(())
}
