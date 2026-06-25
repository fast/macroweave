use macroweave::repeat;

fn main() {
    repeat!(value in [one,, two] {
        let _ = value;
    });
}
