use std::{
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let url = args[1].clone();
    let concurrency: usize = args[2].parse().unwrap();
    let duration = Duration::from_secs_f64(args[3].parse().unwrap());
    let expected = std::fs::read(&args[4]).unwrap();
    let barrier = Arc::new(Barrier::new(concurrency + 1));
    let handles: Vec<_> = (0..concurrency)
        .map(|_| {
            let (url, expected, barrier) = (url.clone(), expected.clone(), barrier.clone());
            thread::spawn(move || {
                let client = reqwest::blocking::Client::builder()
                    .no_proxy()
                    .timeout(Duration::from_secs(5))
                    .build()
                    .unwrap();
                // Establish a separate persistent connection for each worker.
                let _ = client.get(&url).send().unwrap().bytes().unwrap();
                barrier.wait();
                let started = Instant::now();
                let mut latencies = Vec::new();
                let mut errors = 0u64;
                while started.elapsed() < duration {
                    let t = Instant::now();
                    match client.get(&url).send() {
                        Ok(response) if response.status().as_u16() == 200 => match response.bytes()
                        {
                            Ok(body) if body.as_ref() == expected => {
                                latencies.push(t.elapsed().as_secs_f64() * 1000.0)
                            }
                            _ => errors += 1,
                        },
                        _ => errors += 1,
                    }
                }
                (latencies, errors)
            })
        })
        .collect();
    barrier.wait();
    let started = Instant::now();
    let mut latencies = Vec::new();
    let mut errors = 0;
    for handle in handles {
        let (mut samples, failures) = handle.join().unwrap();
        latencies.append(&mut samples);
        errors += failures;
    }
    let elapsed = started.elapsed().as_secs_f64();
    latencies.sort_by(f64::total_cmp);
    let percentile = |p: f64| {
        if latencies.is_empty() {
            0.0
        } else {
            latencies[((latencies.len() - 1) as f64 * p).round() as usize]
        }
    };
    println!(
        "{}",
        serde_json::json!({"requests": latencies.len(), "errors": errors, "seconds": elapsed, "rps": latencies.len() as f64 / elapsed, "p50_ms": percentile(0.50), "p95_ms": percentile(0.95), "p99_ms": percentile(0.99)})
    );
}
