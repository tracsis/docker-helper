//! This crate contains a set of utilities that use [curl::easy::Easy] to interact with
//! Docker unix socket located at `/var/run/docker.sock` in order to perform certain Docker
//! operations. It can be useful in writing tests which have external service dependencies
//! that need to be orchestrated from within rust.

mod types;

pub use crate::types::*;
use anyhow::{anyhow, Context, Result};
use curl::easy::{Easy, List};
use serde_json::ser::to_string;
use std::collections::HashMap;
use std::io::Read;
use urlencoding::encode;

/// High level utility that pulls image, creates container with a given image,
/// maps container port to host one and automatically starts it.
///
/// # Arguments
/// * `container_name` - Unique container name
/// * `image` - Full name of Docker image in the form `image:version`
/// * `container_port` - internal container port to forward to host
/// * `host_port` - host port to forward internal container port to
///
/// # Examples
/// ```no_run
/// let result = docker_helper::start_container_with_port_binding("test", "ubuntu:20.04", 80, 81);
/// ```
pub fn start_container_with_port_binding(
    container_name: &str,
    image: &str,
    container_port: u16,
    host_port: u16,
) -> Result<String> {
    let existing_images = find_images(image)?;
    if existing_images.is_empty() {
        pull_image(image)?;
    }
    let id = create_container(
        container_name,
        CreateContainer {
            image: image.to_owned(),
            port_bindings: HashMap::from([(
                format!("{}/tcp", container_port),
                vec![PortBinding {
                    host_port: format!("{}", host_port),
                }],
            )]),
        },
    )?;
    start_container(&id)?;
    Ok(id)
}

/// Pulls Docker image
///
/// # Arguments
/// * `image_name` - Full name of Docker image in the form `image:version`
///
/// # Examples
/// ```no_run
/// let result = docker_helper::pull_image("ubuntu:20.04");
/// ```
pub fn pull_image(image_name: &str) -> Result<()> {
    let path = format!("/images/create?fromImage={}", image_name);
    let _ = send_request(&path, true, None)?;
    Ok(())
}

/// Stops container with a given `id` and subsequently calls [`prune_containers`]
///
/// # Arguments
/// * `id` - container id
///
/// # Examples
/// ```no_run
/// let id = docker_helper::start_container_with_port_binding("test", "ubuntu:20.04", 80, 81).unwrap();
/// let result = docker_helper::stop_and_cleanup_container(&id);
/// ```
pub fn stop_and_cleanup_container(id: &str) -> Result<()> {
    stop_container(id)?;
    prune_containers()
}

/// Starts container with a given `id`
///
/// # Arguments
/// * `id` - container id
///
/// # Examples
/// ```no_run
/// let result = docker_helper::start_container("6fe66725ed81");
/// ```
pub fn start_container(id: &str) -> Result<()> {
    let path = format!("/containers/{}/start", id);
    let _ = send_request(&path, true, None)?;
    Ok(())
}

/// Stops container with a given `id`
///
/// # Arguments
/// * `id` - container id
///
/// # Examples
/// ```no_run
/// let result = docker_helper::stop_container("6fe66725ed81");
/// ```
pub fn stop_container(id: &str) -> Result<()> {
    let path = format!("/containers/{}/stop", id);
    let _ = send_request(&path, true, None)?;
    Ok(())
}

/// Prunes all stopped container
///
/// # Examples
/// ```no_run
/// let result = docker_helper::prune_containers();
/// ```
pub fn prune_containers() -> Result<()> {
    let path = "/containers/prune".to_string();
    let _ = send_request(&path, true, None);
    Ok(())
}

/// Finds images for a given reference string (`image_name:version`)
///
/// # Examples
/// ```no_run
/// let result = docker_helper::find_images("ubuntu:20.04");
/// ```
pub fn find_images(reference: &str) -> Result<Vec<ImageDescriptor>> {
    let filter = to_string(&ImageFilter {
        reference: vec![reference.to_owned()],
    })?;
    let path = format!("/images/json?filters={}", encode(&filter));
    let resp = send_request(&path, false, None)?;
    let result: Vec<ImageDescriptor> = serde_json::from_str(&resp)
        .with_context(|| format!("Failed to parse find_images response json: {}", resp))?;

    Ok(result)
}

fn create_container(container_name: &str, request: CreateContainer) -> Result<String> {
    let path = format!("/containers/create?name={}", container_name);
    let json = serde_json::to_string(&request)?;
    let bytes = json.as_bytes();
    let resp = send_request(&path, true, Some(bytes))?;
    let result: CreateContainerResult = serde_json::from_str(&resp)
        .with_context(|| format!("Failed to parse create_container response json: {}", resp))?;
    Ok(result.id)
}

fn send_request(path: &str, post: bool, maybe_json_data: Option<&[u8]>) -> Result<String> {
    let mut easy = Easy::new();
    easy.unix_socket("/var/run/docker.sock")?;
    let url = format!("http://localhost{}", path);
    easy.url(&url)?;
    println!("url = {}", &url);

    if post {
        easy.post(true)?;
        easy.post_field_size(0)?;
    }

    let mut resp_data: Vec<u8> = Vec::new();
    let read_data = |buf: &[u8]| {
        resp_data.extend_from_slice(buf);
        Ok(buf.len())
    };

    match maybe_json_data {
        Some(mut req_data) => {
            let mut list = List::new();
            list.append("Content-Type: application/json")?;
            easy.http_headers(list)?;
            easy.post_field_size(req_data.len() as u64)?;
            let mut transfer = easy.transfer();
            transfer
                .read_function(|buf| Ok(req_data.read(buf).unwrap_or(0)))
                .unwrap();
            transfer.write_function(read_data)?;
            transfer.perform()?;
        }
        None => {
            let mut transfer = easy.transfer();
            transfer.write_function(read_data)?;
            transfer.perform()?;
        }
    }

    let data = std::str::from_utf8(&resp_data).unwrap();
    match easy.response_code()? {
        200..=204 => Ok(data.to_owned()),
        _ => Err(anyhow!("Docker API call failed: {}", data)),
    }
}
