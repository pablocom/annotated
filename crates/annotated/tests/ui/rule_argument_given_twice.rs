use annotated::Validate;

#[derive(Validate)]
struct NewMember {
    #[validate(length(max = 5, max = 6))]
    nickname: String,
}

#[derive(Validate)]
struct Post {
    #[validate(each(length(min = 1, unit = "bytes", min = 2)))]
    tags: Vec<String>,
}

fn main() {}
