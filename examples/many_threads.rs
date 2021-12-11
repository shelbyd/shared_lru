use rand::Rng;
use shared_lru::SharedLru;
use std::sync::Arc;

fn main() {
    let shared = SharedLru::with_byte_limit(1024 * 1024);

    let numbers = Arc::new(shared.make_cache::<usize, usize>());

    let handles = (0..4)
        .map(|thread| {
            let numbers = Arc::clone(&numbers);
            std::thread::spawn(move || {
                eprintln!("Thread {} started", thread);
                let mut rng = rand::thread_rng();
                for i in 0..1_000_000 {
                    if let None = numbers.get(&i) {
                        numbers.insert(i, rng.gen());
                    }
                    if i % 10_000 == 0 {
                        eprintln!("Thread {} finished {}", thread, i);
                    }
                }
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.join().unwrap();
    }

    eprintln!("Done");
}
