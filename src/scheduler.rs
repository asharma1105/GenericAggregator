use chrono::{DateTime, Utc, NaiveDateTime};
use log::{info, error};
use std::collections::HashMap;
use std::fs::OpenOptions;
use tokio::time::{interval, Duration as TokioDuration};
use std::time::{Duration, Instant};

use crate::{AggregatorConfig};

pub async fn start_schedulers(aggregator_config: Vec<AggregatorConfig>) {
    let one_min_interval = interval(TokioDuration::from_secs(60));
    let five_min_interval = interval(TokioDuration::from_secs(300));

    let aggregator_config_clone = aggregator_config.clone();
    tokio::spawn(async move {
        info!("Starting the one minute scheduler...");
        let mut one_min_interval = one_min_interval;
        loop {
            one_min_interval.tick().await;
            let start = Instant::now();
            process_aggregations(aggregator_config_clone.clone(), 1).await;
            let duration = start.elapsed();
            info!("1minSchedulder::Completed Execution in {:?}", duration);
        }
    });

    tokio::spawn(async move {
        info!("Starting the five minute scheduler...");
        let mut five_min_interval = five_min_interval;
        loop {
            five_min_interval.tick().await;
            let start = Instant::now();
            process_aggregations(aggregator_config.clone(), 5).await;
            let duration = start.elapsed();
            info!("5minSchedulder::Completed Execution in {:?}", duration);
        }
    });
}

async fn process_aggregations(aggregator_config: Vec<AggregatorConfig>, interval: usize) {
    
    let current_time: DateTime<Utc> = Utc::now();
    let cutoff_duration = Duration::from_secs((interval * 60) as u64);
    let cutoff_time = current_time - chrono::Duration::from_std(cutoff_duration).unwrap();

    for config in aggregator_config.iter() {
        for aggregation in &config.aggregations {
            if aggregation.interval_in_mins == interval {
                let mut data_to_dump = HashMap::new();
                
                let mut aggregation_results = aggregation.aggregation_results.lock().unwrap();
                aggregation_results.retain(|&timestamp, results| {
                    let record_time = DateTime::<Utc>::from_utc(
                        NaiveDateTime::from_timestamp(timestamp as i64, 0),
                        Utc,
                    );
                    if record_time >= cutoff_time {
                        for (key, count) in results.iter() {
                            *data_to_dump.entry(key.clone()).or_insert(0) += count;
                        }
                        true
                    } else {
                        false
                    }
                });

                dump_to_file(&aggregation.output_key, &data_to_dump);
            }
        }
    }
}

fn dump_to_file(output_key: &str, data: &HashMap<String, usize>) {
    let filename = format!("{}_{}.csv", output_key, Utc::now().format("%Y%m%d%H%M%S"));
    let file_path = format!("src/outputs/{}", filename);

    if data.is_empty() {
        error!("No data to write inside filename:{}", filename);
        return
    }
    
    match OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(&file_path)
    {
        Ok(file) => {
            
            info!("Writing data inside csv filename:{}", filename);
            let mut writer = csv::Writer::from_writer(file);

            for(key, value) in data.iter() {
                if let Err(err) = writer.write_record(&[key, &value.to_string()]) {
                    error!("Error writing record to CSV: {}", err);
                }
            }

            writer.flush().unwrap();

        }
        Err(e) => {
            error!("Couldn't open file: {}", e);
        }
    }
}

