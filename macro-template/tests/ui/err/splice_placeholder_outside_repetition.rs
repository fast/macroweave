use macrotable::splice;

fn main() {
    splice!(#T in [u8] {
        let _ = stringify!(#T);
        #( let _ = stringify!(#T); )*
    });
}
