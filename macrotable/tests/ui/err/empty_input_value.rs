use macrotable::repeat;

fn main() {
    repeat!(value in [one,, two] {
        let _ = value;
    });
}
