use macro_template::template;

fn main() {
    template! {
        for N in (0..=3).rev(true) {
            let _ = N;
        }
    }
}
