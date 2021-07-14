fn all<T, E>(results: Vec<Result<T, E>>) -> Result<Vec<T>, E> {
    let mut values = vec![];
    for r in results {
        values.push(r?);
    }
    return Ok(values);
}

fn main() {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_all() {
        assert_eq!(
            all::<u16, ()>(vec![Ok(42), Ok(12), Ok(1024)]).unwrap(),
            vec![42, 12, 1024]
        );
        match all::<u16, &str>(vec![Ok(42), Err("666"), Err("667")]) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "666"),
        };
    }
}
