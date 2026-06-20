use macro_template::template;

fn main() {
    template! {
        for (Ty, Max) in [
            (u8),
        ] {
            let _: Ty = Max;
        }
    }
}
