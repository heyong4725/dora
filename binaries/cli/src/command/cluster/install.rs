use std::path::PathBuf;

use clap::Args;

use crate::command::{Executable, default_tracing};

use super::config::{ClusterConfig, MachineConfig};
use super::{
    format_daemon_port_arg, format_labels_arg, format_zenoh_peer_arg, print_summary,
    record_ssh_result, run_ssh, ssh_target,
};

/// Install dora-daemon as a systemd service on each machine.
///
/// SSH-es into each machine, writes a systemd unit file, and enables the service.
///
/// Examples:
///
///   dora cluster install cluster.yml
#[derive(Debug, Args)]
#[clap(verbatim_doc_comment)]
pub struct Install {
    /// Path to the cluster configuration file
    #[clap(value_name = "PATH", value_hint = clap::ValueHint::FilePath)]
    config: PathBuf,
}

fn systemd_unit(config: &ClusterConfig, machine: &MachineConfig) -> String {
    let labels_arg = format_labels_arg(&machine.labels);
    let daemon_port_arg = format_daemon_port_arg(machine.daemon_port);
    let zenoh_peer_arg = format_zenoh_peer_arg(config.zenoh_peer.as_deref());

    format!(
        r#"[Unit]
Description=Dora Daemon ({id})
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=dora daemon --machine-id {id} --coordinator-addr {addr} --coordinator-port {port}{daemon_port_arg}{zenoh_peer_arg}{labels} --quiet
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
"#,
        id = machine.id,
        addr = config.coordinator.addr,
        port = config.coordinator.port,
        labels = labels_arg,
    )
}

fn install_command(unit: &str, service_name: &str) -> String {
    let escaped_unit = unit.replace('\'', "'\\''");
    format!(
        "echo '{escaped_unit}' | sudo tee /etc/systemd/system/{service_name}.service > /dev/null && sudo systemctl daemon-reload && sudo systemctl enable --now {service_name}"
    )
}

impl Executable for Install {
    fn execute(self) -> eyre::Result<()> {
        default_tracing()?;
        let config = ClusterConfig::load(&self.config)?;

        let mut failures = Vec::new();

        for machine in &config.machines {
            let target = ssh_target(machine);
            let service_name = format!("dora-daemon-{}", machine.id);
            let unit = systemd_unit(&config, machine);
            let cmd = install_command(&unit, &service_name);

            println!("Installing {service_name} on {} ({target})", machine.id);
            let result = run_ssh(&target, machine.port, &cmd);
            record_ssh_result(
                &mut failures,
                &machine.id,
                result,
                &format!("{service_name} installed and started"),
            );
        }

        print_summary(
            "daemon(s) installed as systemd services",
            config.machines.len(),
            &failures,
        );

        if failures.is_empty() {
            Ok(())
        } else {
            eyre::bail!(
                "install failed on {}/{} machine(s)",
                failures.len(),
                config.machines.len()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::config::{CoordinatorConfig, MachineConfig};
    use super::*;
    use std::collections::BTreeMap;

    fn machine_with_runtime_options() -> MachineConfig {
        let mut labels = BTreeMap::new();
        labels.insert("arch".to_string(), "arm64".to_string());
        labels.insert("gpu".to_string(), "true".to_string());

        MachineConfig {
            id: "gpu-a".to_string(),
            host: "10.0.0.2".to_string(),
            user: Some("robot".to_string()),
            port: Some(2222),
            daemon_port: Some(53292),
            labels,
        }
    }

    fn cluster_config() -> ClusterConfig {
        ClusterConfig {
            coordinator: CoordinatorConfig {
                addr: "10.0.0.1".parse().unwrap(),
                port: 7777,
            },
            zenoh_peer: Some("tcp/10.0.0.1:5456".to_string()),
            machines: Vec::new(),
        }
    }

    #[test]
    fn systemd_unit_includes_cluster_runtime_options() {
        let unit = systemd_unit(&cluster_config(), &machine_with_runtime_options());

        assert!(unit.contains("Description=Dora Daemon (gpu-a)"));
        assert!(unit.contains(
            "ExecStart=dora daemon --machine-id gpu-a --coordinator-addr 10.0.0.1 --coordinator-port 7777 --local-listen-port 53292 --zenoh-peer tcp/10.0.0.1:5456 --labels arch=arm64,gpu=true --quiet"
        ));
        assert!(unit.contains("Restart=on-failure"));
        assert!(unit.contains("WantedBy=multi-user.target"));
    }

    #[test]
    fn install_command_escapes_systemd_unit_single_quotes() {
        let cmd = install_command("ExecStart=echo 'ready'\n", "dora-daemon-gpu-a");

        assert!(cmd.contains("'\\''ready'\\''"));
        assert!(cmd.contains("sudo tee /etc/systemd/system/dora-daemon-gpu-a.service"));
        assert!(cmd.contains("sudo systemctl enable --now dora-daemon-gpu-a"));
    }
}
