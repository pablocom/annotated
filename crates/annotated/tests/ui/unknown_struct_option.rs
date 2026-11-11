use annotated::Validate;

#[derive(Validate)]
#[validate(report_everything)]
struct NewMember {
    #[validate(required)]
    name: String,
}

fn main() {}
