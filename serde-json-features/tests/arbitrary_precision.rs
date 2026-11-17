//! With `serde_json`'s `arbitrary_precision` feature, `serde_json` hands any type a number that isn't a 64-bit integer
//! as a map of its text. A struct collecting type errors buffers its values that way, and reads each number from that
//! text as `serde_json` reads it for the type that asks.

use std::collections::BTreeMap;

use annotated::Deserialize;

fn read<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, String> {
    annotated::json::from_str(json).map_err(|error| error.to_string())
}

#[derive(Deserialize, Debug, PartialEq)]
#[annotated(collect_type_errors)]
struct Measurement {
    ratio: f64,
    weight: Option<f32>,
}

#[test]
fn a_float_is_read_as_with_serde() {
    assert_eq!(
        read::<Measurement>(r#"{ "ratio": 0.5, "weight": 1.25 }"#),
        Ok(Measurement {
            ratio: 0.5,
            weight: Some(1.25)
        })
    );
}

#[test]
fn negative_zero_is_read_as_a_float_as_with_serde() {
    let measurement = read::<Measurement>(r#"{ "ratio": -0 }"#).unwrap();

    assert_eq!(measurement.ratio.to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn a_float_of_the_wrong_type_is_found_as_a_float() {
    assert_eq!(
        read::<Measurement>(r#"{ "ratio": "half", "weight": 1e400 }"#),
        Err("ratio: must be f64, found string\nweight: must be f32, found float".to_owned())
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[annotated(collect_type_errors)]
struct Ledger {
    total: u128,
    balance: i128,
    count: u64,
}

#[test]
fn an_integer_beyond_64_bits_is_read_whole_by_a_128_bit_type() {
    assert_eq!(
        read::<Ledger>(r#"{ "total": 1180591620717411303424, "balance": -1180591620717411303424, "count": 1 }"#),
        Ok(Ledger {
            total: 1_180_591_620_717_411_303_424,
            balance: -1_180_591_620_717_411_303_424,
            count: 1
        })
    );
}

#[test]
fn an_integer_beyond_64_bits_is_of_the_wrong_type_for_a_64_bit_one() {
    assert_eq!(
        read::<Ledger>(r#"{ "total": 1, "balance": 1, "count": 18446744073709551616 }"#),
        Err("count: must be u64, found integer".to_owned())
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[annotated(collect_type_errors)]
struct Readings {
    #[validate(nested)]
    measurements: Vec<Measurement>,
    labels: BTreeMap<String, f64>,
}

#[test]
fn a_float_inside_a_value_read_where_it_is_in_the_buffer_is_read_as_with_serde() {
    assert_eq!(
        read::<Readings>(r#"{ "measurements": [{ "ratio": 0.5 }], "labels": { "low": 0.1 } }"#),
        Ok(Readings {
            measurements: vec![Measurement {
                ratio: 0.5,
                weight: None
            }],
            labels: BTreeMap::from([("low".to_owned(), 0.1)])
        })
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[annotated(collect_type_errors)]
struct Price {
    amount: serde_json::Number,
}

#[test]
fn a_number_kept_as_text_is_read_as_its_text() {
    assert_eq!(
        read::<Price>(r#"{ "amount": 2.50 }"#).map(|price| price.amount.to_string()),
        Ok("2.50".to_owned())
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
#[annotated(collect_type_errors)]
enum Shape {
    Circle { radius: f64 },
}

#[derive(serde::Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
enum ShapeBySerde {
    Circle { radius: f64 },
}

#[test]
fn a_float_beside_an_internal_tag_is_refused_as_serde_s_buffer_refuses_it() {
    let body = r#"{ "type": "Circle", "radius": 0.5 }"#;

    assert_eq!(
        (
            serde_json::from_str::<ShapeBySerde>(body).map_err(|error| error.to_string()),
            read::<Shape>(body)
        ),
        (
            Err("invalid type: map, expected f64".to_owned()),
            Err("radius: must be f64, found float".to_owned())
        )
    );
}
