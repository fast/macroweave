use macrotable::repeat;

fn main() {
    repeat!(T in [] {
        let _ = stringify!(T);
    });
}
