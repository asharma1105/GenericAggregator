use axum::{
    body::Bytes,
    response::IntoResponse,
};
use flate2::read::ZlibDecoder;
use log::{info, error, debug};
use std::io::{Read};
use serde_json::{Value,Result};
use crossbeam::channel::{Receiver, Sender};
use tokio::task;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use jsonschema::{JSONSchema};

use crate::{AggregatorConfig, aggregation};

pub fn parse_json(payload: String) -> Result<serde_json::Value> {
    return serde_json::Deserializer::from_str(&payload).into_iter::<serde_json::Value>().collect::<Result<serde_json::Value>>();
}

fn validate_record(input_record: &Value, schema: Value, module: String) -> bool
{
    // Compile the schema
    let compiled_schema = JSONSchema::compile(&schema).expect("A valid schema");

    // Validate the input JSON against the schema
    let result = compiled_schema.validate(&input_record);

    // Check validation result
    if let Err(errors) = result {
        error!("ERROR::{}::Input JSON validation failed with the following errors:", module);
        for error in errors {
            error!("Error: {}", error);
        }
        return false;
    } 
    true
}

fn flatten_record(json: &Value, prefix: String, flattened: &mut HashMap<String, Value>) {
    match json {
        Value::Object(map) => {
            for (key, value) in map {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten_record(value, new_prefix, flattened);
            }
        }
        Value::Array(array) => {
            for (index, value) in array.iter().enumerate() {
                let new_prefix = format!("{}[{}]", prefix, index);
                flatten_record(value, new_prefix, flattened);
            }
        }
        _ => {
            flattened.insert(prefix, json.clone());
        }
    }
}

fn flatten_validate_json_records(json: &Value, config: &AggregatorConfig, module: String) -> Option<Vec<String>> {
    
    if let Some(json_records) = json.as_array(){
        let mut json_list = Vec::with_capacity(json_records.len());

        for record in json_records {
            if validate_record(record, config.schema.clone(), module.clone())
            {
                let mut flattened = HashMap::new();   
                flatten_record(record, String::new(), &mut flattened);

                let flattened_value = serde_json::to_value(&flattened).unwrap();
                let record_string = serde_json::to_string(&flattened_value).unwrap();

                json_list.push(record_string)
            }
        }

        Some(json_list)
    }
    else{

        None
    }
}


fn handler_json_records(json : &Value, sender: &Sender<Vec<String>>, module: String, config: &AggregatorConfig)  -> impl IntoResponse 
{
    if let Some(json_list) = flatten_validate_json_records(&json, &config, module.clone()) {
        if let Err(err) = sender.send(json_list) {
            error!("ERROR::{}::Failed to send data: {:?}", module, err);
            return format!("ERROR::{}::Failed to send data to channel", module).into_response();
        }
        
        format!("{}::Successfully flatten the records and send to channel", module).into_response()
    }
    else
    {
        error!("ERROR::{}::Invalid JSON Received, No Processing", module).into_response()
    }
}

pub async fn handler_dispatcher(module: String,
    sender: Sender<Vec<String>>,
    body: Bytes,
    aggregator_config: AggregatorConfig) -> impl IntoResponse 
{
    debug!("{}::Dispatching handler", module);

    debug!("{}::Using aggregator config : {:?}", module, aggregator_config);

    let start = Instant::now();

    let mut decoder = ZlibDecoder::new(&*body);
    
    let mut decompressed_data = String::new();
    let _ = decoder.read_to_string(&mut decompressed_data).map_err(|_|  format!("ERROR::{}::Received corrupted payload", module).into_response());

    if let Ok(payload_array)= parse_json(decompressed_data) {
        if let Some(payload) = payload_array.as_array(){
            for json in payload {
                handler_json_records(&json, &sender, module.clone(), &aggregator_config);
            }
        }
    } else {
        return format!("ERROR::{}::Error while parsing the JSON", module).into_response();
    }

    let duration = start.elapsed();
    format!("{}::Completed Execution Post Request in {:?}", module, duration).into_response()
}

pub fn start_receiver_thread(module: String, receiver: Receiver<Vec<String>>, aggregator_config: AggregatorConfig) {
    task::spawn(async move {
        while let Ok(response) = receiver.recv() {

            let start = Instant::now();
            for data in &response
            {
                let record: HashMap<String, Value> = serde_json::from_str(&data).expect("ERROR::{}::Failed to parse string in receiver");

                aggregation::aggregate_record(&record, module.clone(), &aggregator_config);

                // for aggregation in &aggregator_config.aggregations {
                //     debug!("{:?}", aggregation.aggregation_results);
                // }
            }
            let duration = start.elapsed();
            info!("{}::Completed Execution of Aggregation for data of size {} in {:?}", module, response.len(), duration)
        }
    });
}
