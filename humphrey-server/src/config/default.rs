use crate::config::{
    BlacklistConfig, BlacklistMode, Config, LoggingConfig, RouteConfig, RouteType,
};
use crate::server::logger::LogLevel;

impl Default for Config {
    fn default() -> Self {
        Self {
            address: "0.0.0.0".into(),
            port: 80,
            threads: 32,
            default_websocket_proxy: None,
            hosts: vec![Default::default()],
            default_host: Default::default(),
            #[cfg(feature = "plugins")]
            plugins: Vec::new(),
            logging: Default::default(),
            cache: Default::default(),
            blacklist: Default::default(),
        }
    }
}

impl Default for RouteConfig {
    fn default() -> Self {
        Self {
            route_type: RouteType::Directory,
            matches: "/*".into(),
            path: Some('.'.into()),
            load_balancer: None,
            websocket_proxy: None,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            console: true,
            file: None,
        }
    }
}

impl Default for BlacklistConfig {
    fn default() -> Self {
        Self {
            list: Default::default(),
            mode: BlacklistMode::Block,
        }
    }
}
