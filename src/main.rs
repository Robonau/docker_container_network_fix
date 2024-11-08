use async_process::{Command, Stdio};
use bollard::{container::ListContainersOptions, secret::ContainerSummary, Docker};
use futures_lite::{io::BufReader, prelude::*};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use tokio::time::interval;

#[tokio::main]
async fn main() {
    let mut interval = interval(std::time::Duration::from_secs(10));
    loop {
        interval.tick().await;
        run().await;
    }
}

async fn run() {
    let all_containers: Vec<ContainerSummary> = Docker::connect_with_socket_defaults()
        .unwrap()
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            filters: HashMap::from([("status".to_string(), vec!["running".to_string()])]),
            ..Default::default()
        }))
        .await
        .unwrap();
    let accessible_containers: Vec<&ContainerSummary> = all_containers
        .iter()
        .filter(|container| {
            match get_from_labels(container, "com.docker.compose.project.config_files") {
                Some(compose_yml) => {
                    let compose_yml = Path::new(&compose_yml);
                    let compose_stacks_dir =
                        env::var("COMPOSE_STACKS_DIR").unwrap_or("/opt/stacks".to_string());
                    let compose_stacks_dir = Path::new(&compose_stacks_dir);
                    compose_yml.starts_with(compose_stacks_dir)
                }
                None => false,
            }
        })
        .collect();

    println!("checking {} containers", accessible_containers.len());

    let containers_to_fix: Vec<&ContainerSummary> = accessible_containers
        .into_iter()
        .filter(|container| {
            let Some(host_config) = &container.host_config else {
                return false;
            };
            let Some(network_mode) = &host_config.network_mode else {
                return false;
            };
            if network_mode.starts_with("container:") {
                let id = Some(network_mode["container:".len()..].to_string());
                all_containers.iter().find(|c| c.id == id).is_none()
            } else {
                false
            }
        })
        .collect();

    for container in containers_to_fix {
        let Some(compose_yml) =
            get_from_labels(&container, "com.docker.compose.project.config_files")
        else {
            println!(
                "failed to find compose dir for container: {}",
                serde_json::to_string_pretty(container).unwrap()
            );
            continue;
        };

        match &container {
            ContainerSummary {
                names: Some(names), ..
            } if names.len() > 1 => {
                println!("Fixing container: {}", names[0]);
            }
            ContainerSummary {
                image: Some(image), ..
            } => {
                println!(
                    "Fixing container: {}",
                    serde_json::to_string_pretty(image).unwrap()
                );
            }
            _ => {
                println!(
                    "Fixing container: {}",
                    serde_json::to_string_pretty(container).unwrap()
                );
            }
        }

        let Some(container_name) = get_from_labels(container, "com.docker.compose.service") else {
            continue;
        };

        let Ok(mut child) = Command::new("docker")
            .arg("--log-level")
            .arg("ERROR")
            .arg("compose")
            .arg("-f")
            .arg(compose_yml)
            .arg("up")
            .arg("-d")
            .arg("--remove-orphans")
            .arg("--force-recreate")
            .arg(&container_name)
            .stdout(Stdio::piped())
            .spawn()
        else {
            println!(
                "failed to recreate compose: {}",
                serde_json::to_string_pretty(container).unwrap()
            );
            continue;
        };

        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();

        while let Some(line) = lines.next().await {
            let Ok(line) = line else {
                continue;
            };
            println!("{}", line);
        }
    }
}

fn get_from_labels<'a>(container: &'a ContainerSummary, key: &str) -> Option<&'a String> {
    let Some(labels) = &container.labels else {
        return None;
    };
    labels.get(key)
}
