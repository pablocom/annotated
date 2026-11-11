use annotated::Deserialize;

#[derive(Deserialize)]
#[annotated(collect_everything)]
struct NewMember {
    #[validate(required)]
    full_name: String,
}

#[derive(Deserialize)]
#[annotated(collect_type_errors = true)]
struct NewGuest {
    #[validate(required)]
    full_name: String,
}

#[derive(Deserialize)]
struct NewTeam {
    #[annotated(collect_type_errors)]
    name: String,
}

fn main() {}
