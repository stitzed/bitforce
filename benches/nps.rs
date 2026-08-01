use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};

use bitforce::board::ChessBoard;

fn bench_perft(c: &mut Criterion) {
    let mut board: ChessBoard = ChessBoard::new();

    let mut group = c.benchmark_group("movegen");

    group.sample_size(10);
    group.throughput(Throughput::Elements(4_865_609));
    group.bench_function("perft", |b| b.iter(|| black_box(board.perft(5))));

    group.finish();
}

criterion_group!(benches, bench_perft);
criterion_main!(benches);
