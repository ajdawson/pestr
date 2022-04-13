use serde::Deserialize;

static DEFAULT_CPUS_PER_NODE: u32 = 128;
static DEFAULT_SEARCH_CONSERVE_NODES: bool = false;
static DEFAULT_SEARCH_PE_RADIUS: f32 = 0.25;
static DEFAULT_SEARCH_THREAD_RADIUS: f32 = 0.5;

pub struct Config {
    pub cpus_per_node: u32,
    pub search: SearchConfig,
}

pub struct SearchConfig {
    pub conserve_nodes: bool,
    pub pe_radius: f32,
    pub thread_radius: f32,
}

impl Config {
    pub fn new() -> Self {
        Self::create(FileConfig::empty())
    }

    pub fn from_file(config_file: &str) -> Self {
        let file_config = FileConfig::from_file(config_file);
        Self::create(file_config)
    }

    fn create(file_config: FileConfig) -> Self {
        let cpus_per_node = read_from_env("PESTR_CPUS_PER_NODE")
            .map(|s| s.parse().unwrap())
            .or(file_config.cpus_per_node)
            .unwrap_or(DEFAULT_CPUS_PER_NODE);

        let conserve_nodes = read_from_env("PESTR_SEARCH_CONSERVE_NODES")
            .map(|s| s.parse().unwrap())
            .or(file_config.search.conserve_nodes)
            .unwrap_or(DEFAULT_SEARCH_CONSERVE_NODES);

        let pe_radius = read_from_env("PESTR_SEARCH_PE_RADIUS")
            .map(|s| s.parse().unwrap())
            .or(file_config.search.pe_radius)
            .unwrap_or(DEFAULT_SEARCH_PE_RADIUS);

        let thread_radius = read_from_env("PESTR_SEARCH_THREAD_RADIUS")
            .map(|s| s.parse().unwrap())
            .or(file_config.search.thread_radius)
            .unwrap_or(DEFAULT_SEARCH_THREAD_RADIUS);

        Self {
            cpus_per_node,
            search: SearchConfig {
                conserve_nodes,
                pe_radius,
                thread_radius,
            },
        }
    }
}

#[derive(Deserialize)]
struct FileConfig {
    cpus_per_node: Option<u32>,
    search: FileSearchConfig,
}

#[derive(Deserialize)]
struct FileSearchConfig {
    conserve_nodes: Option<bool>,
    pe_radius: Option<f32>,
    thread_radius: Option<f32>,
}

impl FileConfig {
    pub fn from_file(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(file_contents) => {
                let config: Self =
                    toml::from_str(&file_contents).unwrap_or_else(|_| FileConfig::empty());
                config
            }
            Err(_) => FileConfig::empty(),
        }
    }

    fn empty() -> Self {
        Self {
            cpus_per_node: None,
            search: FileSearchConfig {
                conserve_nodes: None,
                pe_radius: None,
                thread_radius: None,
            },
        }
    }
}

fn read_from_env(env_name: &str) -> Option<String> {
    std::env::var(env_name).ok()
}
