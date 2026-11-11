use annotated::Validate;

#[derive(Validate)]
struct Tag {
    #[validate(length(max = 3, unit = "words"))]
    label: String,
}

#[derive(Validate)]
struct Note {
    #[validate(length(max = 3, unit = bytes))]
    text: String,
}

fn main() {}
