use annotated::Validate;

#[derive(Validate)]
#[validate(report_everything)]
struct NewMember {
    #[validate(required)]
    name: String,
}

#[derive(Validate)]
#[validate(required)]
struct NewTeam {
    name: String,
}

fn main() {}
