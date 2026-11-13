use annotated::Validate;

#[derive(Validate)]
enum Payment {
    #[validate(custom(a_card_expires_after_it_starts, field = expires_on))]
    Card { valid_from: u16, expires_in: u16 },
    Transfer { expires_on: u16 },
}

#[derive(Validate)]
enum Filter {
    #[validate(custom(bounds_are_ordered, field = 2))]
    Between(u32, u32),
}

fn main() {}
