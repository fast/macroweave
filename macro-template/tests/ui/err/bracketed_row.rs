use macro_template::template;

fn main() {
    template! {
        for (Ty, Width) in [
            [u16, 2],
        ] {
            let _: [Ty; Width];
        }
    }
}
