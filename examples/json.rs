extern crate serde;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
struct Package<'a> {
    name: &'a str,
    dependencies: HashMap<&'a str, &'a str>,
}

fn load_package(pjson: &str) -> Package {
    serde_json::from_str(pjson).expect("package.json not properly formated")
}

fn main() {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_json() {
        let package_json = r#"{
      "name": "A",
      "dependencies": {
        "B": "0.1.0",
        "C": "2.0.0"
      }
    }"#;

        let package = load_package(package_json);
        assert_eq!(package.name, "A");
        let dependencies = package
            .dependencies
            .into_iter()
            .map(|(k, _)| k)
            .collect::<Vec<&str>>();
        assert!(dependencies.contains(&"B"));
        assert!(dependencies.contains(&"C"));
    }
}
