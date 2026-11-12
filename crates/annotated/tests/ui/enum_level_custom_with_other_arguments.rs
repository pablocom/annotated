use annotated::Validate;

#[derive(Validate)]
#[validate(custom(within_the_limit, is_refundable))]
enum Refund {
    Partial { cents: u32 },
    Full,
}

#[derive(Validate)]
#[validate(custom(within_the_limit, at = cents))]
enum Discount {
    Partial { cents: u32 },
    Full,
}

#[derive(Validate)]
#[validate(custom)]
enum Charge {
    Partial { cents: u32 },
    Full,
}

fn main() {}
