use annotated::Validate;

mod house_rules {
    pub fn divisible_by(divisor: u32) -> u32 {
        divisor
    }
}

#[derive(Validate)]
struct Pack {
    #[validate(house_rules::divisible_by(5, 6))]
    size: u32,
}

fn main() {}
