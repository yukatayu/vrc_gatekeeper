
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    test_request()
}

fn test_request() -> Result<(), Box<dyn std::error::Error>> {
    let resp = reqwest::blocking::get("https://httpbin.org/ip")?
        .json::<HashMap<String, String>>()?;
    println!("{:#?}", resp);
    Ok(())
}
