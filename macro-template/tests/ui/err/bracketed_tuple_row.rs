use macrotable::repeat;

fn main() {
    repeat!((#T, #Kind) in [[u16, Small]] {
        let _ = stringify!(#T #Kind);
    });
}
