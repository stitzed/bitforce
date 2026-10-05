use std::io;

mod uci;

fn main() {
    uci::UciProtocol::new(io::stdout()).run();
}
