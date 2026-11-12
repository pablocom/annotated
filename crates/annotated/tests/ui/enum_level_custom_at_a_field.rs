use annotated::Validate;

#[derive(Validate)]
#[validate(custom(within_the_limit, field = cents))]
enum Refund {
    Partial { cents: u32 },
    Full,
}

fn main() {}
