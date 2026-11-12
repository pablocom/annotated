use annotated::Validate;

#[derive(Validate)]
union Bits {
    whole: u32,
    halves: [u16; 2],
}

fn main() {}
