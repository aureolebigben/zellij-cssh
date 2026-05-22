use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ServerGroup {
    name: String,
    servers: Vec<ServerConfig>,
}

#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ServerConfig {
    name: String,
    host: String,
}

impl ServerGroup {
    pub fn new(name: String, servers: Vec<ServerConfig>) -> Self {
        ServerGroup { name, servers }
    }

    pub fn all_selected_in(&self, servers: &HashSet<ServerConfig>) -> bool {
        !self.servers.is_empty() && self.servers.iter().all(|server| servers.contains(server))
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_servers(&self) -> &Vec<ServerConfig> {
        &self.servers
    }
}

impl ServerConfig {
    pub fn new(name: String, host: String) -> Self {
        ServerConfig { name, host }
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_host(&self) -> &str {
        &self.host
    }
}

pub fn load_config_from_path(path: &PathBuf) -> Vec<ServerGroup> {
    let content = fs::read(path).expect("Unable to read config file");
    toml::from_slice::<Vec<ServerGroup>>(&content).expect("Unable to parse config file")
}
