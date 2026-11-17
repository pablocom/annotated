use annotated::Validate;

#[derive(Validate)]
#[annotated(collect_everything)]
struct NewMember {
    #[validate(required)]
    full_name: String,
}

#[derive(Validate)]
struct NewTeam {
    #[annotated(crate = "annotated")]
    #[validate(required)]
    name: String,
}

#[derive(Validate)]
#[annotated(crate)]
struct NewGuest {
    #[validate(required)]
    full_name: String,
}

#[derive(Validate)]
#[annotated(crate = annotated)]
struct NewHost {
    #[validate(required)]
    full_name: String,
}

#[derive(Validate)]
#[annotated(crate = "two words")]
struct NewOwner {
    #[validate(required)]
    full_name: String,
}

#[derive(Validate)]
#[annotated(crate = "annotated", crate = "annotated")]
struct NewAdmin {
    #[validate(required)]
    full_name: String,
}

#[derive(Validate)]
#[annotated(crate = "elsewhere::annotated")]
struct NewViewer {
    #[validate(required)]
    full_name: String,
}

fn main() {}
