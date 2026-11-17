use annotated::Validate;

#[derive(Validate)]
struct NewMember {
    #[validate(length(min = 10, max = 5))]
    nickname: String,
}

#[derive(Validate)]
struct Adult {
    #[validate(range(min = 120, max = 18))]
    age: u8,
}

#[derive(Validate)]
struct Account {
    #[validate(range(min = -1.5, max = -2.5))]
    balance: f64,
}

#[derive(Validate)]
struct Post {
    #[validate(each(length(max = 1, min = 2)))]
    tags: Vec<String>,
}

fn main() {}
