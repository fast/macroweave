use macro_template::template;

fn main() {
    template! {
        for Ty in [u8],
        for Ty in [u16] {
            let _ = stringify!(Ty);
        }
    }
}
