use macro_template::template;

fn main() {
    template! {
        for N in (0..=3).step_by() {
            let _ = N;
        }
    }
}
