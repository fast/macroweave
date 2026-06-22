use macrotable::repeat;

fn main() {
    repeat!(#value in [1usize] {
        let _ = #value;
    });
}
