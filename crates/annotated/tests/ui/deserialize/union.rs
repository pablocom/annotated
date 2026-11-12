use annotated::Deserialize;

#[derive(Deserialize)]
union Bits {
    whole: u32,
    halves: [u16; 2],
}

fn main() {}
