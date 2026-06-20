use macro_template::template;

fn main() {
    template! {
        for N in 'a'..=3 {
            let _ = N;
        }
    }
}
