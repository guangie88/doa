use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;
use which::which;

mod fmt;

#[derive(Debug, Serialize, Deserialize)]
pub struct DockerRun {
    pub image: String,
    pub help: Option<String>,

    pub interactive: Option<bool>,
    pub tty: Option<bool>,

    pub command: Option<Vec<String>>,
    pub entrypoint: Option<String>,
    pub envs: Option<HashMap<String, String>>,
    pub env_file: Option<PathBuf>,
    pub network: Option<String>,
    pub ports: Option<Vec<String>>,
    pub volumes: Option<Vec<String>>,
    pub user: Option<String>,
    pub extra_flags: Option<Vec<String>>,
}

pub fn shell_interpolate(raw: &str) -> Result<String, Box<dyn Error>> {
    fmt::shell_interpolate(raw, &|cmd| {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(&["/C", cmd]).output()?
        } else {
            Command::new("sh").args(&["-c", cmd]).output()?
        };

        Ok(str::from_utf8(&output.stdout)?.trim_end().to_string())
    })
}

impl DockerRun {
    pub fn run(
        &self,
        docker_cmd: &Path,
        _kv: &HashMap<String, String>,
    ) -> Result<(), Box<dyn Error>> {
        // Convert all options into flags
        let command_flags = self.command.as_ref().map_or(vec![], |cmds| {
            cmds.iter()
                .map(|cmd| {
                    shell_interpolate(cmd).expect("Invalid env for cmds")
                })
                .collect()
        });

        let entrypoint_flag =
            self.entrypoint.as_ref().map_or(vec![], |entrypoint| {
                vec![
                    "--entrypoint".to_string(),
                    shell_interpolate(entrypoint)
                        .expect("Invalid env for entrypoint"),
                ]
            });

        let envs_flags = self.envs.as_ref().map_or(vec![], |envs| {
            envs.iter()
                .flat_map(|(k, v)| {
                    vec![
                        "-e".to_string(),
                        shell_interpolate(&format!("{}={}", k, v))
                            .expect("Invalid env for envs"),
                    ]
                })
                .collect()
        });

        let env_file_flags =
            self.env_file.as_ref().map_or(vec![], |env_file| {
                vec![
                    "--env-file".to_string(),
                    shell_interpolate(&format!("{}", env_file.display()))
                        .expect("Invalid env for env-file"),
                ]
            });

        let network_flags = self.network.as_ref().map_or(vec![], |network| {
            vec![shell_interpolate(&format!("--network={}", network))
                .expect("Invalid env for env-file")]
        });

        let ports_flags = self.ports.as_ref().map_or(vec![], |ports| {
            ports
                .iter()
                .flat_map(|port| {
                    vec![
                        "-p".to_string(),
                        shell_interpolate(port).expect("Invalid env for ports"),
                    ]
                })
                .collect()
        });

        let volumes_flags = self.volumes.as_ref().map_or(vec![], |volumes| {
            volumes
                .iter()
                .flat_map(|volume| {
                    vec![
                        "-v".to_string(),
                        shell_interpolate(volume)
                            .expect("Invalid env for volumes"),
                    ]
                })
                .collect()
        });

        let user_flags = self.user.as_ref().map_or(vec![], |user| {
            vec![
                "-u".to_string(),
                shell_interpolate(user).expect("Invalid env for user"),
            ]
        });

        let extra_flags =
            self.extra_flags.as_ref().map_or(vec![], |extra_flags| {
                extra_flags
                    .iter()
                    .map(|extra_flag| {
                        shell_interpolate(extra_flag)
                            .expect("Invalid env for extra flags")
                    })
                    .collect()
            });

        let image = shell_interpolate(&self.image)?;

        let args = [
            // Command with default flags
            &["run".to_string()],
            &["--rm".to_string()],
            // Optional flags
            &entrypoint_flag[..],
            &envs_flags[..],
            &env_file_flags[..],
            &network_flags[..],
            &ports_flags[..],
            &volumes_flags[..],
            &user_flags[..],
            &extra_flags[..],
            // Mandatory fields
            &[image],
            &command_flags[..],
        ]
        .concat();

        let output = Command::new(docker_cmd).args(args).output()?;

        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;
        Ok(())
    }
}

pub fn get_cli_path() -> Result<PathBuf, which::Error> {
    which("docker")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dockerrun(image: &str) -> DockerRun {
        DockerRun {
            image: image.to_string(),
            help: None,
            interactive: None,
            tty: None,
            command: None,
            entrypoint: None,
            envs: None,
            env_file: None,
            volumes: None,
            user: None,
            extra_flags: None,
        }
    }

    #[test]
    fn test_run() {
        let mut dr = make_dockerrun("clux/muslrust:stable");
        dr.command = Some(vec!["cargo".to_string(), "--version".to_string()]);

        let docker_cmd = get_cli_path().unwrap();
        dr.run(&docker_cmd, &HashMap::new()).unwrap();
    }
}
