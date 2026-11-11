use annotated::Validate;

#[derive(Validate)]
struct Post {
    #[validate(length(max = 3, unit = "chars"))]
    tags: Vec<String>,
}

fn main() {}
