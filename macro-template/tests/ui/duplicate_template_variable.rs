use macro_template::template;

fn main() {
    template! {
        for (Ty, Ty) in [
            (u8, u16),
        ] {
            let _: Ty;
        }
    }
}
