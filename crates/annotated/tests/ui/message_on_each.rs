use annotated::Validate;

#[derive(Validate)]
struct Post {
    #[validate(each(length(max = 10), message = "keep tags short"))]
    tags: Vec<String>,
}

fn main() {}
