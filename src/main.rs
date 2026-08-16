use fba_algo::{Order, clear_batch};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bids = [Order::new(110, 65)];
    let asks = [
        Order::new(105, 40),
        Order::new(108, 20),
        Order::new(109, 30),
    ];

    match clear_batch(&bids, &asks)? {
        Some(outcome) => {
            println!("uniform price: {}", outcome.price.midpoint());
            println!("executed quantity: {}", outcome.executed_quantity);
            println!(
                "clearing interval: [{}, {}]",
                outcome.price.lower, outcome.price.upper
            );
        }
        None => println!("no trade"),
    }

    Ok(())
}
