use std::collections::HashMap;

type Graph<'a, 'b> = HashMap<&'a str, Vec<&'b str>>;

fn topo_sort<'a>(graph: &Graph<'a, 'a>) -> Vec<&'a str> {
    fn topo_sort_rec<'a>(graph: &Graph<'a, 'a>, package: &'a str, result: &mut Vec<&'a str>) {
        let dependencies = graph.get(package).unwrap();
        let unresolved_dependencies = dependencies
            .into_iter()
            .filter(|p| !result.contains(p))
            .collect::<Vec<&&str>>();
        for ud in unresolved_dependencies {
            topo_sort_rec(graph, ud, result);
        }
        if !result.contains(&package) {
            result.push(package);
        }
    }

    let mut result: Vec<&str> = vec![];
    for (&package, _) in graph.iter() {
        topo_sort_rec(graph, package, &mut result);
    }

    return result;
}

fn main() {
    let hm = HashMap::new();
    topo_sort(&hm);
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
      let mut map = ::std::collections::HashMap::new();
      $( map.insert($key, $val); )*
      map
    }}
  }

    #[test]
    fn test_topo() {
        let graph = hashmap![
      "a" => vec![],
      "b" => vec!["a"],
      "c" => vec!["a", "b"],
      "d" => vec!["a", "b", "c"]];
        assert_eq!(topo_sort(&graph), vec!["a", "b", "c", "d"]);
    }
}
