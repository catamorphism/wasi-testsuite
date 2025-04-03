use wasi::cli::stdin;
use wasi::io::poll::poll;

fn main() {
    let stdin = stdin::get_stdin();
    let p1 = stdin.subscribe();
    let p2 = stdin.subscribe();

    // Should work:
    // - Exactly the same pollable passed in multiple times.
    // - Distinct pollables for the same backing resource (stdin in this case).
    poll(&[&p1, &p2, &p1, &p2]);
}
