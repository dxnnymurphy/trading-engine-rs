use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use rand::{RngExt, SeedableRng};
use rand::rngs::StdRng;
use trading_engine_rs::models::order;
use trading_engine_rs::orderbook::OrderBook;
use trading_engine_rs::models::messages::{NewOrderCommand, ModifyOrderCommand, CancelOrderCommand};
use trading_engine_rs::models::types::Side;
use trading_engine_rs::orderbook::v1::book::Book;


/// Helper function to deterministicly generate a certain amount of NewOrderCommands 
/// to be used in benchmarks
fn create_new_order_commands(size: i32) -> Vec<NewOrderCommand> {
    let mut rng = StdRng::seed_from_u64(42);

    (0..size).map(|i| NewOrderCommand {
        client_order_id: i as u64,
        side: if rng.random_bool(0.5) { Side::Buy } else { Side::Sell },
        price: rng.random_range(95.0..105.0).into(),
        quantity: rng.random_range(0.0..1000.0)
    }).collect()
}

/// Throughput benchmark function for new_order. Steps:
/// 1. Create a certain amount of new_order_commands (not measured)
/// 2. Loop through adding them to the orderbook (measurable)
fn new_order(c: &mut Criterion) {
    let mut bench_group = c.benchmark_group("new_order");
    for &size in [1000,10000,100000].iter() {
        bench_group.throughput(Throughput::Elements(size as u64));
        bench_group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, size| {
            b.iter_batched(
                || (Book::new(), create_new_order_commands(*size)), // Setup
                |(mut orderbook, commands)| {
                    for command in commands {
                        let order_id = orderbook.new_order(command).unwrap();
                        black_box(order_id);
                    }
                }, //Routine 
                criterion::BatchSize::LargeInput);
        });
    }
    bench_group.finish();
}


criterion_group!(benches, new_order);
criterion_main!(benches);