use annotated::Deserialize;

#[derive(Deserialize)]
enum Payment {
    #[annotated(collect_type_errors)]
    Card { number: String },
    Cash,
}

#[derive(Deserialize)]
enum Refund {
    Partial {
        #[annotated(collect_type_errors)]
        cents: u32,
    },
    Full,
}

fn main() {}
