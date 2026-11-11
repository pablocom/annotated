use annotated::Validate;

#[derive(Validate)]
struct Post {
    #[validate(each)]
    tags: Vec<String>,
}

#[derive(Validate)]
struct Note {
    #[validate(each = length)]
    tags: Vec<String>,
}

fn main() {}
