use annotated::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Shape {
    Rectangle(u32, u32),
}

fn main() {}
