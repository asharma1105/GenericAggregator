use serde_json::{Value};
use std::collections::{HashMap};
use log::{error};
use chrono::{Utc, DateTime};

use crate::AggregatorConfig;

pub fn aggregate_record(record: &HashMap<String, Value>, module: String, aggregator_config: &AggregatorConfig)
{
    let mut not_present = false;
    for aggregation in &aggregator_config.aggregations {
        let mut group_key = Vec::new();
        
        for key in &aggregation.aggregation_keys {
            if let Some(value) = extract_value_for_key(record, &key.key) {
                group_key.push(value);
            } else {
                error!("{}::aggregation key={:?} not present in the incoming record {:?}, Moving to next Aggregation", module, key.key, record);
                not_present = true;
                break;
            }
        }

        if not_present {
            continue;
        }

        let group_key_str = serde_json::to_string(&group_key).unwrap();
        
        //putting time as the key and value as the aggregation key and its count
        let current_time: DateTime<Utc> = Utc::now();
        let timestamp = current_time.timestamp() as u64;
        
        let mut aggregation_results = aggregation.aggregation_results.lock().unwrap();
        let interval_map = aggregation_results.entry(timestamp).or_insert_with(HashMap::new);
        let counter = interval_map.entry(group_key_str).or_insert(0);
        *counter += 1;
    } 
}

fn extract_value_for_key(record: &HashMap<String, Value>, key: &str) -> Option<Value> {
    record.get(key).cloned() //array type object in json is not supported yet
}
