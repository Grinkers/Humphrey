//! Provides the core server functionality and manages the underlying Humphrey app.

use humphrey::http::{Request, Response};
use humphrey::stream::Stream;
use humphrey::{App, SubApp};

#[cfg(feature = "plugins")]
use crate::plugins::manager::PluginManager;
#[cfg(feature = "plugins")]
use crate::plugins::plugin::PluginLoadResult;
#[cfg(feature = "plugins")]
use std::process::exit;

use crate::cache::Cache;
use crate::config::{BlacklistMode, Config, HostConfig, RouteType};
use crate::logger::Logger;
use crate::proxy::proxy_handler;
use crate::r#static::{directory_handler, file_handler, redirect_handler};

use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, RwLock};

/// Represents the application state.
/// Includes the target directory, cache state, and the logger.
pub struct AppState {
    /// The app's configuration.
    pub config: Config,
    /// The app's cache.
    pub cache: RwLock<Cache>,
    /// The app's logger.
    pub logger: Logger,
    /// The app's plugin manager.
    #[cfg(feature = "plugins")]
    pub plugin_manager: RwLock<PluginManager>,
}

impl From<Config> for AppState {
    fn from(config: Config) -> Self {
        let cache = RwLock::new(Cache::from(&config));
        let logger = Logger::from(&config);
        Self {
            config,
            cache,
            logger,
            #[cfg(feature = "plugins")]
            plugin_manager: RwLock::new(PluginManager::default()),
        }
    }
}

/// Main function for the static server.
pub fn main(config: Config) {
    let connection_timeout = config.connection_timeout;

    let mut app: App<AppState> = App::new_with_config(config.threads, AppState::from(config))
        .with_connection_condition(verify_connection)
        .with_connection_timeout(connection_timeout);

    let state = app.get_state();

    for route in init_app_routes(&state.config.default_host, 0).routes {
        app = app.with_route(&route.route, route.handler);
    }

    for (host_index, host) in state.config.hosts.iter().enumerate() {
        app = app.with_host(&host.matches, init_app_routes(host, host_index + 1));
    }

    if let Some(proxy) = state.config.default_websocket_proxy.as_ref() {
        app = app.with_websocket_route("/*", websocket_handler(proxy.to_string()));
    }

    #[cfg(feature = "tls")]
    if let Some(tls_config) = &state.config.tls_config {
        app = app
            .with_cert(&tls_config.cert_file, &tls_config.key_file)
            .with_forced_https(tls_config.force);
    }

    let addr = format!("{}:{}", state.config.address, state.config.port);
    let logger = &state.logger;
    logger.info("Starting server");

    #[cfg(feature = "plugins")]
    if let Ok(plugins_count) = load_plugins(&state.config, state.clone()) {
        logger.info(&format!("Loaded {} plugins", plugins_count))
    } else {
        exit(1);
    };

    logger.info(&format!("Running at {}", addr));
    logger.debug(&format!("Configuration: {:?}", state.config));

    #[cfg(feature = "tls")]
    if state.config.tls_config.is_some() {
        app.run_tls(addr).unwrap();
    } else {
        app.run(addr).unwrap();
    }

    #[cfg(not(feature = "tls"))]
    app.run(addr).unwrap();
}

fn init_app_routes(host: &HostConfig, host_index: usize) -> SubApp<AppState> {
    let mut subapp: SubApp<AppState> = SubApp::new();

    for (route_index, route) in host.routes.iter().enumerate() {
        subapp = subapp.with_route(&route.matches, move |request, state| {
            request_handler(request, state, host_index, route_index)
        });

        if let Some(proxy) = route.websocket_proxy.as_ref() {
            subapp =
                subapp.with_websocket_route(&route.matches, websocket_handler(proxy.to_string()));
        }
    }

    subapp
}

/// Verifies that the client is allowed to connect by checking with the blacklist config.
fn verify_connection(stream: &mut TcpStream, state: Arc<AppState>) -> bool {
    if let Ok(address) = stream.peer_addr() {
        if state.config.blacklist.mode == BlacklistMode::Block
            && state.config.blacklist.list.contains(&address.ip())
        {
            state.logger.warn(&format!(
                "{}: Blacklisted IP attempted to connect",
                &address.ip()
            ));
            return false;
        }
    } else {
        state.logger.warn("Corrupted stream attempted to connect");
        return false;
    }

    true
}

#[cfg(feature = "plugins")]
fn request_handler(
    mut request: Request,
    state: Arc<AppState>,
    host: usize,
    route: usize,
) -> Response {
    let plugins = state.plugin_manager.read().unwrap();

    let route_config = state.config.get_route(host, route);
    let mut response = plugins
        .on_request(&mut request, state.clone(), route_config) // If the plugin overrides the response, return it
        .unwrap_or_else(|| inner_request_handler(request, state.clone(), host, route)); // If no plugin overrides the response, generate it in the normal way

    // Pass the response to plugins before it is sent to the client
    plugins.on_response(&mut response, state.clone());

    response
}

#[cfg(not(feature = "plugins"))]
fn request_handler(request: Request, state: Arc<AppState>, host: usize, route: usize) -> Response {
    inner_request_handler(request, state, host, route)
}

fn inner_request_handler(
    request: Request,
    state: Arc<AppState>,
    host: usize,
    route: usize,
) -> Response {
    let route = state.config.get_route(host, route);

    match route.route_type {
        RouteType::File => file_handler(request, state.clone(), route.path.as_ref().unwrap(), host),
        RouteType::Directory => directory_handler(
            request,
            state.clone(),
            route.path.as_ref().unwrap(),
            &route.matches,
            host,
        ),
        RouteType::Proxy => proxy_handler(
            request,
            state.clone(),
            route.load_balancer.as_ref().unwrap(),
            &route.matches,
        ),
        RouteType::Redirect => {
            redirect_handler(request, state.clone(), route.path.as_ref().unwrap())
        }
    }
}

fn websocket_handler(target: String) -> impl Fn(Request, Stream, Arc<AppState>) {
    move |request: Request, source: Stream, state: Arc<AppState>| {
        proxy_websocket(request, source, &target, state).ok();
    }
}

fn proxy_websocket(
    request: Request,
    mut source: Stream,
    target: &str,
    state: Arc<AppState>,
) -> Result<(), Box<dyn Error>> {
    let source_addr = request.address.origin_addr.to_string();
    let bytes: Vec<u8> = request.into();

    if let Ok(destination) = TcpStream::connect(target) {
        // The target was successfully connected to

        let mut destination = Stream::Tcp(destination);

        destination.write_all(&bytes)?;

        state.logger.info(&format!(
            "{}: WebSocket connected, proxying data",
            source_addr
        ));

        source.set_nonblocking()?;
        destination.set_nonblocking()?;

        let mut source_buffer: [u8; 1024] = [0; 1024];
        let mut destination_buffer: [u8; 1024] = [0; 1024];

        loop {
            let source_read = match source.read(&mut source_buffer) {
                Ok(read) => read,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => 0,
                _ => break,
            };

            let destination_read = match destination.read(&mut destination_buffer) {
                Ok(read) => read,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => 0,
                _ => break,
            };

            if source_read != 0 {
                destination.write_all(&source_buffer[0..source_read])?;
            }

            if destination_read != 0 {
                source.write_all(&destination_buffer[0..destination_read])?;
            }

            std::thread::park_timeout(std::time::Duration::from_millis(10));
        }
    } else {
        state
            .logger
            .error(&format!("{}: Could not connect to WebSocket", source_addr));
    }

    Ok(())
}

#[cfg(feature = "plugins")]
fn load_plugins(config: &Config, state: Arc<AppState>) -> Result<usize, ()> {
    let mut manager = state.plugin_manager.write().unwrap();

    for plugin in &config.plugins {
        unsafe {
            let app_state = state.clone();
            match manager.load_plugin(&plugin.library, &plugin.config, app_state) {
                PluginLoadResult::Ok(name) => {
                    state.logger.info(&format!("Initialised plugin {}", name));
                }
                PluginLoadResult::NonFatal(e) => {
                    state
                        .logger
                        .warn(&format!("Non-fatal plugin error in {}", plugin.name));
                    state.logger.warn(&format!("Error message: {}", e));
                    state.logger.warn("Ignoring this plugin");
                }
                PluginLoadResult::Fatal(e) => {
                    state
                        .logger
                        .error(&format!("Could not initialise plugin {}", plugin.name));
                    state.logger.error(&format!("Error message: {}", e));

                    return Err(());
                }
            }
        }
    }

    Ok(manager.plugin_count())
}
