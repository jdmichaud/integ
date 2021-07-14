use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Repo {
    url: String,
    branch: String,
    build: String,
}

#[derive(Deserialize, Debug)]
struct Config {
    repositories: Vec<Repo>,
}

fn load_config(config_yaml: &str) -> Config {
    serde_yaml::from_str(config_yaml).expect("could not load config")
}

fn main() {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_yaml() {
        let config_yaml = r#"
repositories:
  - url: git@gitlab.com:jdmichaud/observable.git
    branch: dependabot/npm_and_yarn/handlebars-4.5.3
    build: npm run build
  - url: https://gitlab.com/jdmichaud/terrain
    branch: master
    build: npm run all
"#;
        let config = load_config(config_yaml);
        assert_eq!(config.repositories.len(), 2);
        assert_eq!(
            config.repositories[0].url,
            "git@gitlab.com:jdmichaud/observable.git"
        );
    }
}
