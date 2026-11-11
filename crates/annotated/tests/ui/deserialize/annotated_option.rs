use annotated::Deserialize;

#[derive(Deserialize)]
#[annotated(collect_type_errors)]
struct NewMember {
    #[validate(required)]
    full_name: String,
}

#[derive(Deserialize)]
struct NewTeam {
    #[annotated(collect_type_errors)]
    name: String,
}

fn main() {}
