use macro_template::template;

fn main() {
    template! {
        for C in ('a'..='c').strip_prefix() {
            let _ = C;
        }
    }
}
