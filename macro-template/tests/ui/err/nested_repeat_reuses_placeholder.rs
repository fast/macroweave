use macrotable::repeat;

fn main() {
    repeat!(#T in [u8] {
        repeat!(#T in [#T] {
            let _ = stringify!(#T);
        });
    });
}
