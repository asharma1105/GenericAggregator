use log::{info, error, debug};
use std::env;
use axum::{
    routing::post,
    Router,
};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use crossbeam::channel::{bounded, Receiver, Sender};
use std::collections::{HashMap};
use serde_json::{Value};

mod data_reader;
mod aggregation;
mod scheduler;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ServerConfig {
    server_config: Vec<EndpointConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct EndpointConfig {
    endpoint: String,
    module: String,
    channel_size: usize,
    receiver_threads_count: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AggregatorConfig {
    module: String,
    schema: Value,
    aggregations: Vec<Aggregation>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Aggregation {
    aggregation_keys: Vec<AggregationKey>,
    output_key: String,
    operation: String,
    interval_in_mins: usize,

    #[serde(skip)]
    aggregation_results: Arc<Mutex<HashMap<u64, HashMap<String, usize>>>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AggregationKey {
    key: String,
    level: usize,
}

fn main() {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    info!("Logger Initialised. Starting the Generic Aggregator ... ");

    let file = File::open("src/resources/server_config.json").expect("ERROR::Unable to open server config file");
    let reader = BufReader::new(file);
    let server_config: ServerConfig = serde_json::from_reader(reader).expect("ERROR::Error parsing server config file");
    debug!("Loaded Server config:{:?}", server_config);

    let file = File::open("src/resources/aggregator_config.json").expect("ERROR::Unable to open aggregator config file");
    let reader = BufReader::new(file);
    let mut aggregator_config: Vec<AggregatorConfig> = serde_json::from_reader(reader).expect("ERROR::Error parsing aggregator config file");
    for config in &mut aggregator_config {
        for aggregation in &mut config.aggregations {
            aggregation.aggregation_results = Arc::new(Mutex::new(HashMap::new()))
            ;
        }
    }

    debug!("Loaded Aggregtor config:{:?}", aggregator_config);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(40)
        .enable_all()
        .build()
        .unwrap();
    
    let channel_map = Arc::new(Mutex::new(HashMap::new()));

    runtime.block_on(async {
        let mut app = Router::new();
        
        for endpoint_config in server_config.server_config {
            let endpoint = endpoint_config.endpoint;
            let module = endpoint_config.module.clone();
            let receiver_threads = endpoint_config.receiver_threads_count;
            let channel_size = endpoint_config.channel_size;

            match aggregator_config.iter().find(|&config| config.module == module).cloned() {
                Some(aggregator_config_for_module) => {
                    let (sender, receiver): (Sender<Vec<String>>, Receiver<Vec<String>>) = bounded(channel_size);
            
                    channel_map.lock().unwrap().insert(endpoint.clone(), sender.clone());
                    
                    app = app.route(
                        &endpoint,
                        post({
                            let module = module.clone();
                            let sender = sender.clone();
                            let config = aggregator_config_for_module.clone();
                            move |body| {
                                data_reader::handler_dispatcher(module.clone(), sender.clone(), body, config.clone())
                            }
                        }),
                    );

                    for _ in 0..receiver_threads {
                        let receiver_module = module.clone();
                        let receiver = receiver.clone();
                        let config = aggregator_config_for_module.clone();
                        data_reader::start_receiver_thread(receiver_module, receiver, config);
                    }
                }
                None => {
                    error!("ERROR::{}::No matching aggregator config found for module", module);
                    continue; // Continue to the next iteration of the loop
                }
            }
        }

        scheduler::start_schedulers(aggregator_config).await;

        let address = SocketAddr::from(([127, 0, 0, 1], 3000));
        
        info!("Server is starting and listening on {}", address);
        axum::Server::bind(&address)
                .serve(app.into_make_service())
                .await
                .unwrap();
    })
}