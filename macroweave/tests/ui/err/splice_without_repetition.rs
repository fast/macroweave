use macroweave::splice;

fn main() {
    splice!(T in [u8] {
        let _ = 0;
    });
}
