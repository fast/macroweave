use macrotable::repeat;

fn main() {
    repeat!((#T, #value) in [(u8)] {
        let _ = stringify!(#T #value);
    });
}
