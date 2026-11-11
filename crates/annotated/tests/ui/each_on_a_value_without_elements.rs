use annotated::Validate;

#[derive(Validate)]
struct Post {
    #[validate(each(length(max = 10)))]
    title: u8,
}

fn main() {}
