pub fn load_order() -> String {
    let order = std::fs::read_to_string("order.txt").unwrap();
    println!("loaded");
    order
}
