use annotated::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", transparent)]
struct NewMember {
    #[validate(required)]
    full_name: String,
}

#[derive(Deserialize)]
struct NewTeam {
    #[serde(alias = "title", bound = "")]
    name: String,
}

#[derive(Deserialize)]
#[serde(bound(serialize = "T: Clone", deserialize = "T: Clone"))]
struct Page<T> {
    items: Vec<T>,
}

#[derive(Deserialize)]
#[serde(bound = "T: Clone")]
struct Shelf<T> {
    items: Vec<T>,
}

#[derive(Deserialize)]
struct Invoice {
    #[serde(skip_serializing, bound(deserialize = ""))]
    total: u32,
}

#[derive(Deserialize)]
struct Receipt {
    #[serde(getter = "Receipt::total")]
    total: u32,
}

fn main() {}
