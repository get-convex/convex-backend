use common::interval::Interval;
use interval_map::IntervalMap;
use tikv_jemallocator::Jemalloc;

#[global_allocator]
static ALLOC: Jemalloc = Jemalloc;

fn main() {
    divan::main();
}

#[divan::bench(args = [10, 100, 1000, 10000, 100000])]
fn equal_keys(n: usize) -> IntervalMap {
    let mut map = IntervalMap::new();
    map.insert(
        1,
        std::iter::repeat_n(Interval::prefix(vec![1; 64].into()), n),
    )
    .unwrap();
    map
}
