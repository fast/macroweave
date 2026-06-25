use macroweave::repeat;

fn main() {
    repeat!((T, T) in [(u8, u16)] {
        let _ = stringify!(T);
    });
}
