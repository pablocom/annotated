use annotated::Deserialize;

#[derive(Deserialize)]
enum Payment {
    Card,
    Transfer,
}

fn main() {}
