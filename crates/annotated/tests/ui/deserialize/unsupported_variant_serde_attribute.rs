use annotated::Deserialize;

#[derive(Deserialize)]
enum Payment {
    Card { number: String },
    #[serde(skip)]
    Cash,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum Quantity {
    Exact(u32),
    #[serde(untagged)]
    Described(String),
}

#[derive(Deserialize)]
enum Contact {
    #[serde(deserialize_with = "read_phone")]
    Phone(String),
}

fn main() {}
