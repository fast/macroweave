use macro_template::template;

fn main() {
    template! {
        for () in [] {
            let _: Ty;
        }
    }
}
