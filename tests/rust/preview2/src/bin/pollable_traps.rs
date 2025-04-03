fn main() {
    // Polling an empty list should trap:
    wasi::io::poll::poll(&[]);
}
