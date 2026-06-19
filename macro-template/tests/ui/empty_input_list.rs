use macro_template::template;

fn main() {
    template! {
        for Ty in [] {
            let _: Ty;
        }
    }
}
