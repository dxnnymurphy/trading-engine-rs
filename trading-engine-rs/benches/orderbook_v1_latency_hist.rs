use std::time::Instant;

use hdrhistogram::Histogram;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use trading_engine_rs::models::messages::NewOrderCommand;
use trading_engine_rs::models::types::Side;
use trading_engine_rs::orderbook::v1::book::Book;
use trading_engine_rs::orderbook::OrderBook;

fn create_new_order_commands(size: usize) -> Vec<NewOrderCommand> {
    let mut rng = StdRng::seed_from_u64(42);

    (0..size)
        .map(|index| NewOrderCommand {
            client_order_id: index as u64,
            side: if rng.random_bool(0.5) { Side::Buy } else { Side::Sell },
            price: rng.random_range(95.0..105.0).into(),
            quantity: rng.random_range(1.0..1000.0),
        })
        .collect()
}

fn run_latency_histogram(size: usize) {
    let mut book = Book::new();
    let commands = create_new_order_commands(size);

    let mut histogram = Histogram::<u64>::new_with_bounds(1, 10_000_000_000, 3)
        .expect("failed to create histogram");

    let start_total = Instant::now();

    for command in commands {
        let start = Instant::now();
        let order_id = book
            .new_order(command)
            .expect("failed to insert order in latency bench");
        std::hint::black_box(order_id);

        let elapsed_ns = start.elapsed().as_nanos() as u64;
        histogram
            .record(elapsed_ns)
            .expect("failed to record latency sample");
    }

    let elapsed_total = start_total.elapsed().as_secs_f64();
    let throughput = size as f64 / elapsed_total;

    println!("\n=== new_order latency histogram (n={size}) ===");
    println!("throughput: {:.2} ops/s", throughput);
    println!("p50   : {} ns", histogram.value_at_quantile(0.50));
    println!("p90   : {} ns", histogram.value_at_quantile(0.90));
    println!("p99   : {} ns", histogram.value_at_quantile(0.99));
    println!("p99.9 : {} ns", histogram.value_at_quantile(0.999));
    println!("max   : {} ns", histogram.max());
}

fn main() {
    for &size in &[1_000usize, 10_000, 100_000] {
        run_latency_histogram(size);
    }
}
