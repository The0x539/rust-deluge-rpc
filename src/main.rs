use serde_json;

mod rencode;

fn main() {
    let x = serde_json::json!([
        8i64,
        2,
        ["abc", 0xdef, true],
        {
            "use_foo": false,
            "amount": 3,
        }
    ]);
    let encoded = rencode::to_bytes(&x).unwrap();
    let decoded: serde_json::Value = rencode::from_bytes(&encoded).unwrap();
    println!("{}", serde_json::to_string_pretty(&decoded).unwrap());
}
