use annotated::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct NewMember {
    #[validate(required)]
    full_name: String,
}

#[derive(Deserialize)]
struct NewTeam {
    #[serde(alias = "title", skip)]
    name: String,
}

fn main() {}
