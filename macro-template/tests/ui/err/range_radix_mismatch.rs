use macro_template::template;

fn main() {
    template! {
        for N in 0x0..=3 {
            let _ = N;
        }
    }
}
