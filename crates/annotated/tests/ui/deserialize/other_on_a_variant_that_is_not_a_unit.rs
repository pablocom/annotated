use annotated::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Attachment {
    Image { url: String },
    #[serde(other)]
    Unsupported { kind: String },
}

fn main() {}
