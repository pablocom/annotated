use annotated::Validate;

#[derive(Validate)]
enum Payment {
    #[validate(custom)]
    Card { valid_from: u16, expires_in: u16 },
}

#[derive(Validate)]
enum Stay {
    #[validate(custom(field = last_night))]
    Hotel { first_night: u8, last_night: u8 },
}

#[derive(Validate)]
enum Filter {
    #[validate(custom(bounds_are_ordered, at = 1))]
    Between(u32, u32),
}

fn main() {}
