use macro_template::template;

fn main() {
    template! {
        for N in 0u8..=2u16 {
            let _ = N;
        }
    }
}
