use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use trading_engine_rs::orderbook::models::{Order, OrderModify, OrderType, Side};
use trading_engine_rs::orderbook::simple_order_book::SimpleOrderBook;
use trading_engine_rs::orderbook::OrderBook;

#[derive(Clone, Copy)]
enum Op {
    Add {
        id: i64,
        side: Side,
        price: i32,
        qty: i32,
    },
    Modify {
        id: i64,
        side: Side,
        price: Option<i32>,
        qty: Option<i32>,
    },
    Cancel {
        id: i64,
    },
}

fn apply_op<B: OrderBook>(book: &mut B, op: Op) {
    match op {
        Op::Add { id, side, price, qty } => {
            let order = Order::to_order_pointer(Order::new(id, side, OrderType::Limit, price, qty));
            let _ = book.add_order(order);
        }
        Op::Modify { id, side, price, qty } => {
            let modify = OrderModify::new(id, side, price, qty);
            let _ = book.modify_order(modify);
        }
        Op::Cancel { id } => {
            let _ = book.cancel_order(id);
        }
    }
}

fn run_ops<B: OrderBook>(book: &mut B, ops: &[Op]) {
    for &op in ops {
        apply_op(book, op);
    }
}

fn build_add_only(n: usize) -> Vec<Op> {
    (0..n)
        .map(|i| Op::Add {
            id: i as i64,
            side: if i % 2 == 0 { Side::Buy } else { Side::Sell },
            price: 10_000 + (i % 200) as i32,
            qty: 1 + (i % 10) as i32,
        })
        .collect()
}

fn build_cancel_heavy(n: usize) -> Vec<Op> {
    let mut ops = build_add_only(n);
    for i in 0..n {
        ops.push(Op::Cancel { id: i as i64 });
    }
    ops
}

fn build_modify_heavy(n: usize) -> Vec<Op> {
    let mut ops = build_add_only(n);
    for i in 0..n {
        let side = if i % 2 == 0 { Side::Buy } else { Side::Sell };
        ops.push(Op::Modify {
            id: i as i64,
            side,
            price: if i % 3 == 0 { Some(10_050 + (i % 100) as i32) } else { None },
            qty: if i % 3 != 0 { Some(1 + (i % 20) as i32) } else { None },
        });
    }
    ops
}

fn build_mixed(n: usize) -> Vec<Op> {
    let mut ops = build_add_only(n);
    for i in 0..n {
        let side = if i % 2 == 0 { Side::Buy } else { Side::Sell };
        ops.push(Op::Modify {
            id: i as i64,
            side,
            price: Some(9_900 + (i % 250) as i32),
            qty: Some(1 + (i % 15) as i32),
        });
        if i % 2 == 0 {
            ops.push(Op::Cancel { id: i as i64 });
        }
        ops.push(Op::Add {
            id: (n + i) as i64,
            side,
            price: 10_100 + (i % 300) as i32,
            qty: 1 + (i % 12) as i32,
        });
    }
    ops
}

fn bench_workload<B, F>(
    c: &mut Criterion,
    impl_name: &str,
    workload_name: &str,
    sizes: &[usize],
    make_book: F,
    workload_builder: fn(usize) -> Vec<Op>,
) where
    B: OrderBook,
    F: Fn() -> B + Copy,
{
    let mut group = c.benchmark_group(format!("{impl_name}/{workload_name}"));

    for &size in sizes {
        let ops = workload_builder(size);
        group.throughput(Throughput::Elements(ops.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter_batched(
                make_book,
                |mut book| {
                    run_ops(&mut book, black_box(&ops));
                    black_box(book);
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn orderbook_benches(c: &mut Criterion) {
    let sizes = [1_000usize, 5_000, 10_000];

    bench_workload::<SimpleOrderBook, _>(
        c,
        "simple_order_book",
        "add_only",
        &sizes,
        SimpleOrderBook::new,
        build_add_only,
    );

    bench_workload::<SimpleOrderBook, _>(
        c,
        "simple_order_book",
        "cancel_heavy",
        &sizes,
        SimpleOrderBook::new,
        build_cancel_heavy,
    );

    bench_workload::<SimpleOrderBook, _>(
        c,
        "simple_order_book",
        "modify_heavy",
        &sizes,
        SimpleOrderBook::new,
        build_modify_heavy,
    );

    bench_workload::<SimpleOrderBook, _>(
        c,
        "simple_order_book",
        "mixed",
        &sizes,
        SimpleOrderBook::new,
        build_mixed,
    );
}

criterion_group!(benches, orderbook_benches);
criterion_main!(benches);
