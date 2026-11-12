use annotated::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Payment {
    Card { number: String },
    Cash,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Recipient {
    Email { address: String },
    Id(u64),
}

fn main() {}
