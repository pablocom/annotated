use annotated::Validate;

#[derive(Validate)]
struct Flag {
    #[validate(length(max = 1, unit = "graphemes"))]
    emoji: String,
}

fn main() {}
