use shared_lru::SharedLru;

fn main() {
    let shared = SharedLru::with_byte_limit(2 * 1024 + 512);

    let fruits = shared.make_cache();
    fruits.insert("apple", vec![0u8; 1024]);
    fruits.insert("banana", vec![0u8; 1024]);

    // This `get` touches "apple" so it is now the newest.
    assert!(fruits.get(&"apple").is_some());

    let veggies = shared.make_cache();
    veggies.insert("brocolli", vec![0u8; 1024]);

    assert!(fruits.get(&"apple").is_some());
    assert!(fruits.get(&"banana").is_none());
}
