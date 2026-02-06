use crate::info;
use crate::server::ports::calculate_container_port;
use anyhow::Result;
use bollard::Docker;
use bollard::config::VolumeCreateRequest;
use bollard::models::{
    ContainerCreateBody, HealthStatusEnum, HostConfig, Mount, MountTypeEnum, PortBinding,
    RestartPolicy, RestartPolicyNameEnum,
};
#[allow(unused)]
use bollard::query_parameters::{
    CreateContainerOptions, CreateImageOptions, ListContainersOptions, ListVolumesOptions,
    RemoveContainerOptions, RemoveVolumeOptions, StartContainerOptions,
};
use hex::encode;
use pbkdf2::pbkdf2_hmac;
use sha2::Sha512;
use std::collections::HashMap;

/// Connects to Docker daemon (cross-platform: Windows named pipe or Linux socket)
fn connect_docker() -> Result<Docker> {
    #[cfg(windows)]
    {
        // Windows: Use named pipe
        Docker::connect_with_named_pipe_defaults()
            .map_err(|e| anyhow::anyhow!("Failed to connect to Docker on Windows: {}", e))
    }

    #[cfg(not(windows))]
    {
        // Linux/Mac: Use socket
        Docker::connect_with_local_defaults()
            .map_err(|e| anyhow::anyhow!("Failed to connect to Docker socket: {}", e))
    }
}

#[inline]
pub fn get_unique_instance_id(email: String) -> String {
    let mut instance_id = [0u8; 16];

    dotenv::dotenv().ok();

    let super_secret =
        std::env::var("BLAZE_INSTANCE_SECRET").expect("BLAZE_INSTANCE_SECRET must be set in env");

    let super_secret = super_secret.as_bytes();

    let email = email.trim().to_lowercase();

    pbkdf2_hmac::<Sha512>(email.as_bytes(), super_secret, 100_000, &mut instance_id);
    encode(instance_id)
}

// TODO: Need to implement retry logic for Docker operations, maybe not but on service module
/// Spawns a new BlazeDB container for a user
pub async fn spawn_blazedb_container(instance_id: &str) -> Result<()> {
    let docker = connect_docker()?;

    let container_name = format!("blazedb-{}", instance_id);

    // Create TWO volumes per user (matching BlazeDB's expected paths)
    let config_volume = format!("blazedb_config_{}", instance_id);
    let sources_volume = format!("blazedb_sources_{}", instance_id);

    create_volume_if_not_exists(&docker, &config_volume).await?;
    create_volume_if_not_exists(&docker, &sources_volume).await?;

    // Pull latest image if not exists
    pull_blazedb_image(&docker).await?;

    // Check if container already exists
    if container_exists(&docker, &container_name).await? {
        // Container exists, just start it
        docker
            .start_container(&container_name, None::<StartContainerOptions>)
            .await?;
        info!("Started existing container: {}", container_name);
        return Ok(());
    }

    dotenv::dotenv().ok();

    // Determine network mode based on environment
    // When running in development, we want to use "bridge" mode with port mapping to access the container directly.
    // In production, we can use an internal Docker network and have the proxy route traffic without exposing ports.
    let network_mode = std::env::var("BLAZEDB_NETWORK").unwrap_or_else(|_| "bridge".to_string());

    // Add port mapping when running in external mode
    let port_bindings = if network_mode == "bridge" {
        let host_port = calculate_container_port(instance_id);

        let mut bindings = HashMap::new();
        bindings.insert(
            format!("{}/tcp", "8080"), // Container internal port
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".to_string()),
                host_port: Some(host_port.to_string()),
            }]),
        );

        info!(
            "Mapping container port {} to host port {}",
            "8080", host_port
        );
        Some(bindings)
    } else {
        None // Internal Docker network - no port mapping needed
    };

    // Create new container with both config and sources volumes
    let config = ContainerCreateBody {
        image: Some("ronakgh97/blazedb:latest".to_string()),
        //TODO: Fix these env vars, broooo!!
        env: Some(vec![
            "RUST_LOG=info".to_string(),
            "PORT=8080".to_string(),
            "EMBEDDING_MODEL=text-embedding-qwen3-embedding-0.6b".to_string(),
            "EMBEDDING_API_URL=http://host.docker.internal:1234/v1/embeddings".to_string(),
            "EMBEDDING_API_KEY=local_dev_key".to_string(),
        ]),
        host_config: Some(HostConfig {
            mounts: Some(vec![
                // Config volume: settings, metadata, cache
                Mount {
                    target: Some("/home/blazedb/.config/blaze".to_string()),
                    source: Some(config_volume),
                    typ: Some(MountTypeEnum::VOLUME),
                    ..Default::default()
                },
                // Sources volume: actual data sources
                Mount {
                    target: Some("/home/blazedb/blaze".to_string()),
                    source: Some(sources_volume),
                    typ: Some(MountTypeEnum::VOLUME),
                    ..Default::default()
                },
            ]),
            network_mode: Some(network_mode),
            port_bindings, // Add port bindings here
            restart_policy: Some(RestartPolicy {
                name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let options = CreateContainerOptions {
        name: Some(container_name.clone()),
        ..Default::default()
    };

    docker.create_container(Some(options), config).await?;
    docker
        .start_container(&container_name, None::<StartContainerOptions>)
        .await?;

    info!("Spawned new container: {}", container_name);

    Ok(())
}

/// Destroys a user's BlazeDB container (data persists in volume)
pub async fn destroy_blazedb_container(instance_id: &str) -> Result<()> {
    let docker = connect_docker()?;
    let container_name = format!("blazedb-{}", instance_id);

    if !container_exists(&docker, &container_name).await? {
        return Ok(()); // Container doesn't exist, nothing to do
    }

    let options = RemoveContainerOptions {
        force: true,
        ..Default::default()
    };

    docker
        .remove_container(&container_name, Some(options))
        .await?;

    // TODO: I need backup/restore system first
    // Remove docker volumes as well
    // let config_volume = format!("blazedb_config_{}", instance_id);
    // let sources_volume = format!("blazedb_sources_{}", instance_id);
    //
    // let options = RemoveVolumeOptions {
    //     force: true,
    //     ..Default::default()
    // };
    //
    // docker
    //     .remove_volume(&config_volume, Some(options.clone()))
    //     .await?;
    // docker
    //     .remove_volume(&sources_volume, Some(options.clone()))
    //     .await?;

    info!("️ Destroyed container: {}", container_name);

    Ok(())
}

/// Checks if a container exists
async fn container_exists(docker: &Docker, name: &str) -> Result<bool> {
    let mut filters = HashMap::new();
    filters.insert("name".to_string(), vec![name.to_string()]);

    let options = ListContainersOptions {
        all: true,
        filters: Some(filters),
        ..Default::default()
    };

    let containers = docker.list_containers(Some(options)).await?;
    Ok(!containers.is_empty())
}

// TODO: Gotta use this, or find a different robust method to get port mapping
#[allow(unused)]
/// Get the host port mapping for a container (for external mode)
/// Returns the port number if container has port mapping, None otherwise
pub async fn get_container_port_mapping(instance_id: &str) -> Result<Option<u16>> {
    let docker = connect_docker()?;
    let container_name = format!("blazedb-{}", instance_id);

    // Inspect container to get port mapping
    let container_info = docker.inspect_container(&container_name, None).await?;

    // Check NetworkSettings -> Ports -> "8080/tcp" -> HostPort
    if let Some(network_settings) = container_info.network_settings {
        if let Some(ports) = network_settings.ports {
            if let Some(port_bindings) = ports.get("8080/tcp") {
                if let Some(bindings) = port_bindings {
                    if let Some(first_binding) = bindings.first() {
                        if let Some(host_port_str) = &first_binding.host_port {
                            if let Ok(port) = host_port_str.parse::<u16>() {
                                return Ok(Some(port));
                            }
                        }
                    }
                }
            }
        }
    }

    // No port mapping found (internal network mode)
    Ok(None)
}

/// Creates a Docker volume if it doesn't exist
async fn create_volume_if_not_exists(docker: &Docker, volume_name: &str) -> Result<()> {
    let mut filters = HashMap::new();
    filters.insert("name".to_string(), vec![volume_name.to_string()]);

    let options = ListVolumesOptions {
        filters: Some(filters),
    };

    let volumes = docker.list_volumes(Some(options)).await?;

    if volumes.volumes.is_none() || volumes.volumes.as_ref().unwrap().is_empty() {
        // Volume doesn't exist, create it

        let config = VolumeCreateRequest {
            name: Some(volume_name.to_string()),
            ..Default::default()
        };

        docker.create_volume(config).await?;
        info!("Created Docker volume: {}", volume_name);
    }

    Ok(())
}

#[allow(unused)]
/// Checks the health status of a container
pub async fn check_container_health(container_name: &str) -> Result<bool> {
    let docker = connect_docker()?;
    let container_info = docker.inspect_container(container_name, None).await?;

    if let Some(state) = container_info.state {
        if let Some(health) = state.health {
            return Ok(health.status == Some(HealthStatusEnum::HEALTHY));
        }
    }

    Ok(false)
}

// This function returns a tuple of (is_healthy, started_at, last_error_at, error_state) for the container
pub async fn get_container_status(container_name: &str) -> Result<(bool, String, String, String)> {
    let docker = connect_docker()?;

    let container_info = docker.inspect_container(container_name, None).await?;

    let result = (false, String::new(), String::new(), String::new());

    if let Some(state) = container_info.state {
        let is_healthy = if let Some(health) = state.health {
            health.status == Some(HealthStatusEnum::HEALTHY)
        } else {
            false
        };
        let started_at = state.started_at.unwrap_or(String::new());
        let last_error_at = state.finished_at.unwrap_or(String::new());
        let error_state = state.error.unwrap_or(String::from(""));
        return Ok((is_healthy, started_at, last_error_at, error_state));
    }

    Ok(result)
}

/// Pulls the BlazeDB image from Docker Hub
async fn pull_blazedb_image(docker: &Docker) -> Result<()> {
    use futures_util::stream::StreamExt;

    let options = CreateImageOptions {
        from_image: Some("ronakgh97/blazedb".to_string()),
        tag: Some("latest".to_string()),
        ..Default::default()
    };

    let mut stream = docker.create_image(Some(options), None, None);

    while let Some(_result) = stream.next().await {
        // Silently pull
    }

    Ok(())
}
