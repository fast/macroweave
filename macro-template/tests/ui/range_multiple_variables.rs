use macro_template::template;

fn main() {
    template! {
        for (Ty, Max) in 0..=2 {
            let _: Ty = Max;
        }
    }
}
