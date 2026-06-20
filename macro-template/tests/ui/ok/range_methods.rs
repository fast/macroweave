use macro_template::template;

macro_rules! genstruct {
    ($($name:ident),* $(,)?) => {
        $(struct $name;)*
    };
}

template! {
    for P in (008..=010).strip_prefix() {
        paste::paste! {
            genstruct!(#([<Dec P>]),*);
        }
    }
}

template! {
    for P in (0b001..=0b011).strip_prefix() {
        paste::paste! {
            genstruct!(#([<Bin P>]),*);
        }
    }
}

template! {
    for P in (0o06..=0o10).strip_prefix() {
        paste::paste! {
            genstruct!(#([<Oct P>]),*);
        }
    }
}

template! {
    for P in (0x9f8..=0xa0a).strip_prefix() {
        paste::paste! {
            genstruct!(#([<Lower P>]),*);
        }
    }
}

template! {
    for P in (0x9F8..=0xA0A).strip_prefix() {
        paste::paste! {
            genstruct!(#([<Upper P>]),*);
        }
    }
}

template! {
    for P in (0x9f8..=0xA0A).strip_prefix() {
        paste::paste! {
            genstruct!(#([<Mixed P>]),*);
        }
    }
}

template! {
    for P in (0..=2).rev() {
        paste::paste! {
            genstruct!(#([<Rev P>]),*);
        }
    }
}

fn main() {
    let _ = (Dec008, Dec009, Dec010);
    let _ = (Bin001, Bin010, Bin011);
    let _ = (Oct06, Oct07, Oct10);
    let _ = (Lower9f8, Lower9fa, Lower9ff, Lowera00, Lowera0a);
    let _ = (Upper9F8, Upper9FA, Upper9FF, UpperA00, UpperA0A);
    let _ = (Mixed9F8, Mixed9FA, Mixed9FF, MixedA00, MixedA0A);
    let _ = (Rev2, Rev1, Rev0);
}
