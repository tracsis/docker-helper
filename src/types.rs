use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize)]
pub struct PortBinding {
    #[serde(rename = "HostPort")]
    pub host_port: String,
}

#[derive(Serialize)]
pub struct CreateContainer {
    #[serde(rename = "Image")]
    pub image: String,
    #[serde(rename = "NetworkMode")]
    pub network_mode: String,
}

#[derive(Deserialize, Debug)]
pub struct CreateContainerResult {
    #[serde(rename = "Id")]
    pub id: String,
}

#[derive(Deserialize, Debug)]
pub struct ImageDescriptor {
    #[serde(rename = "Id")]
    pub id: String,
}

#[derive(Serialize)]
pub struct ImageFilter {
    pub reference: Vec<String>,
}

#[derive(Serialize)]
pub struct ContainerFilter {
    pub id: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct ContainerDescriptor {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "NetworkSettings")]
    pub network_settings: NetworkSettings,
}

#[derive(Deserialize, Debug)]
pub struct NetworkSettings {
    #[serde(rename = "Networks")]
    pub networks: HashMap<String, Network>,
}

#[derive(Deserialize, Debug)]
pub struct Network {
    #[serde(rename = "IPAddress")]
    pub ip_address: String,
}
