extern crate input_rs;
use input_rs::InputHandler;

fn main() {
    let mut args = std::env::args();
    if args.len() != 3 {
        println!("Usage: {} <vendor_id_in_hex> <product_id_in_hex>\\nExample {0} 00ee cafe", args.nth(0).unwrap());
        return;
    }
    let args: Vec<String> = args.collect();

    let vendor = u16::from_str_radix(args.get(1).unwrap(), 16).unwrap();
    let product = u16::from_str_radix(args.get(2).unwrap(), 16).unwrap();

    let mut input_handler = InputHandler::new(vendor, product, Box::new(|ev| {
        println!("Got an event! {:?}", ev);
    })).unwrap();
    /*input_handler.register_touch_handler(|ev| {
        println!("Got an event! {:?}", ev);
    });*/
    input_handler.start_polling_events();
    loop {}
}