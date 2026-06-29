use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use clap::Args;

use crate::{
    command::{Executable, default_tracing, up::dora_executable_path},
    common::connect_to_coordinator,
};

use super::config::ClusterConfig;
use super::{query_connected_daemons, run_ssh, ssh_target};

/// Rolling upgrade: SCP the local dora binary to each machine and restart daemons.
///
/// For each machine sequentially:
///   1. SCP the local dora binary to `/usr/local/bin/dora`
///   2. Restart the systemd service
///   3. Wait for the daemon to reconnect
///
/// Examples:
///
///   dora cluster upgrade cluster.yml
#[derive(Debug, Args)]
#[clap(verbatim_doc_comment)]
pub struct Upgrade {
    /// Path to the cluster configuration file
    #[clap(value_name = "PATH", value_hint = clap::ValueHint::FilePath)]
    config: PathBuf,
}

fn scp_args(local_binary: &str, target: &str, port: Option<u16>) -> Vec<String> {
    let mut args = vec![
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
    ];
    if let Some(p) = port {
        // scp uses `-P` (capital) for the port; `-p` means "preserve mtimes"
        args.push("-P".to_string());
        args.push(p.to_string());
    }
    args.push(local_binary.to_string());
    args.push(format!("{target}:/usr/local/bin/dora"));
    args
}

fn restart_service_command(service_name: &str) -> String {
    format!("sudo systemctl restart {service_name}")
}

impl Executable for Upgrade {
    fn execute(self) -> eyre::Result<()> {
        default_tracing()?;
        let config = ClusterConfig::load(&self.config)?;
        let local_binary = dora_executable_path()?;
        let coordinator_addr =
            std::net::SocketAddr::from((config.coordinator.addr, config.coordinator.port));
        let session = connect_to_coordinator(coordinator_addr)?;

        let mut failures = Vec::new();

        for machine in &config.machines {
            let target = ssh_target(machine);
            let service_name = format!("dora-daemon-{}", machine.id);

            println!("Upgrading {} ({target})...", machine.id);

            // 1. SCP binary
            let local_binary_str = local_binary
                .to_str()
                .ok_or_else(|| eyre::eyre!("local binary path is not valid UTF-8"))?;
            let mut scp = std::process::Command::new("scp");
            scp.args(scp_args(local_binary_str, &target, machine.port));
            let scp_status = scp.status();

            match scp_status {
                Ok(s) if s.success() => {}
                Ok(s) => {
                    let msg = format!("scp failed with {s}");
                    eprintln!("  FAILED: {msg}");
                    failures.push((machine.id.clone(), msg));
                    continue;
                }
                Err(e) => {
                    let msg = format!("scp error: {e}");
                    eprintln!("  FAILED: {msg}");
                    failures.push((machine.id.clone(), msg));
                    continue;
                }
            }

            // 2. Restart systemd service
            let restart_cmd = restart_service_command(&service_name);
            match run_ssh(&target, machine.port, &restart_cmd) {
                Ok(true) => {}
                _ => {
                    let msg = "systemctl restart failed".to_string();
                    eprintln!("  FAILED: {msg}");
                    failures.push((machine.id.clone(), msg));
                    continue;
                }
            }

            // 3. Wait for daemon to reconnect (30s timeout)
            let deadline = Instant::now() + Duration::from_secs(30);
            let mut reconnected = false;
            while Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(500));
                if let Ok(connected) = query_connected_daemons(&session)
                    && connected
                        .iter()
                        .any(|d| d.daemon_id.matches_machine_id(&machine.id))
                {
                    reconnected = true;
                    break;
                }
            }

            if reconnected {
                println!("  OK: {} upgraded and reconnected", machine.id);
            } else {
                let msg = "daemon did not reconnect within 30s".to_string();
                eprintln!("  WARNING: {msg}");
                failures.push((machine.id.clone(), msg));
            }
        }

        if failures.is_empty() {
            println!("All {} machine(s) upgraded", config.machines.len());
            Ok(())
        } else {
            println!(
                "Upgraded {}/{} machine(s)",
                config.machines.len() - failures.len(),
                config.machines.len()
            );
            for (id, reason) in &failures {
                eprintln!("  {id}: {reason}");
            }
            eyre::bail!(
                "upgrade failed on {}/{} machine(s)",
                failures.len(),
                config.machines.len()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scp_args_use_capital_port_option_for_custom_ssh_port() {
        let args = scp_args("/usr/bin/dora", "robot@10.0.0.2", Some(2222));

        assert_eq!(
            args,
            vec![
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=10",
                "-P",
                "2222",
                "/usr/bin/dora",
                "robot@10.0.0.2:/usr/local/bin/dora",
            ]
        );
    }

    #[test]
    fn scp_args_omit_port_when_using_ssh_default() {
        let args = scp_args("/usr/bin/dora", "10.0.0.2", None);

        assert_eq!(
            args,
            vec![
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=10",
                "/usr/bin/dora",
                "10.0.0.2:/usr/local/bin/dora",
            ]
        );
    }

    #[test]
    fn restart_service_command_targets_machine_service() {
        assert_eq!(
            restart_service_command("dora-daemon-arm"),
            "sudo systemctl restart dora-daemon-arm"
        );
    }
}
