use annotated::Deserialize;

#[derive(Deserialize)]
enum Payment {
    Card { number: String },
    #[serde(bound = "")]
    Cash,
}

fn main() {}
