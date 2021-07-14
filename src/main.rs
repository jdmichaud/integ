extern crate serde;
use anyhow::{Context, Result};
use io::prelude::*;
use pathdiff;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::format;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short, long, parse(try_from_str = file_exists))]
    config: PathBuf,

    #[structopt(short, long, parse(try_from_str = file_exists))]
    output_path: PathBuf,
}

#[derive(Deserialize, Debug)]
struct Repo {
    url: String,
    branch: String,
    build: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct Folder {
    path: String,
    build: Vec<String>,
}

fn default_workers() -> usize {
    1
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Entry {
    Repo(Repo),
    Folder(Folder),
}

#[derive(Deserialize, Debug)]
struct Config {
    repositories: Vec<Entry>,
    #[serde(default = "default_workers")]
    workers: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct Package {
    name: String,
    #[serde(default)]
    dependencies: HashMap<String, String>,
    #[serde(default, rename = "devDependencies")]
    dev_dependencies: HashMap<String, String>,
}

fn file_exists(path: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path_buf = PathBuf::from(path);
    if path_buf.exists() {
        Ok(path_buf)
    } else {
        Err(format!("{} does not exists", path).into())
    }
}

fn load_config(config_yaml: &str) -> serde_yaml::Result<Config> {
    serde_yaml::from_str(config_yaml)
}

fn get_folder_names(opt: &Opt, config: &Config) -> Vec<String> {
    config
        .repositories
        .iter()
        .map(|repository| match repository {
            Entry::Repo(repo) => repo
                .url
                .split('/')
                .last()
                .unwrap()
                .split('.')
                .nth(0)
                .unwrap(),
            Entry::Folder(folder) => folder.path.split('/').last().unwrap(),
        })
        .map(|f| {
            format!(
                "{}",
                PathBuf::from(opt.output_path.clone())
                    .join(f)
                    .to_string_lossy()
            )
        })
        .collect::<_>()
}

fn is_rsync_present() -> bool {
    return match Command::new("rsync")
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_) => true,
        Err(_) => false,
    };
}

fn clone_repository(repository: &Repo, folder: &str) -> Result<()> {
    println!("cloning {}", repository.url);
    let output = Command::new("git")
        .arg("clone")
        .arg("--branch")
        .arg(repository.branch.to_owned())
        .arg("--depth")
        .arg("1")
        .arg(repository.url.to_owned())
        .arg(&folder)
        .output()
        .expect(&format!("fail to clone {}", repository.url));

    if !output.status.success() {
        // If clone failed, print the command's standard output
        let error_message = String::from_utf8(output.stderr).unwrap();
        return Err(io::Error::new(ErrorKind::Other, error_message.to_owned()))
            .with_context(|| format!("Trying to clone {}", repository.url))?;
    }
    Ok(())
}

fn copy_folder(repository: &Folder, folder: &str) -> Result<()> {
    let output = if is_rsync_present() {
        println!("syncing {} to {}", repository.path, folder);
        Command::new("rsync")
            .arg("-av")
            .arg(format!("{}/", &repository.path))
            .arg(&folder)
            .arg("--exclude=node_modules")
            .output()
            .expect(&format!("fail to sync {} to {}", repository.path, folder))
    } else {
        println!("copying {} to {}", repository.path, folder);
        Command::new("cp")
            .arg("-r")
            .arg(&repository.path)
            .arg(&folder)
            .output()
            .expect(&format!("fail to copy {} to {}", repository.path, folder))
    };
    if !output.status.success() {
        // If clone failed, print the command's standard output
        let error_message = String::from_utf8(output.stderr).unwrap();
        return Err(io::Error::new(ErrorKind::Other, error_message.to_owned()))
            .with_context(|| format!("Trying to sync/copy {}", repository.path))?;
    }
    Ok(())
}

fn retrieve_repositories(repositories: &Vec<Entry>, folders: &Vec<String>) -> Result<Vec<String>> {
    let mut paths = vec![];
    assert_eq!(repositories.len(), folders.len());
    for (index, repository) in repositories.iter().enumerate() {
        if PathBuf::from(folders[index].clone()).exists() {
            println!("{} already exists, skipping", folders[index]);
            continue;
        }
        match repository {
            Entry::Repo(repo) => clone_repository(repo, &folders[index]),
            Entry::Folder(folder) => copy_folder(folder, &folders[index]),
        }?;
        paths.push(String::from(&folders[index]));
    }

    Ok(paths)
}

fn parse_package(folders: &Vec<String>) -> Result<Vec<Package>> {
    let mut packages = vec![];
    for folder in folders {
        let package_json_path = PathBuf::from(folder).join("package.json");
        let package_json = fs::read_to_string(package_json_path.clone())
            .with_context(|| format!("reading {}", package_json_path.to_string_lossy()))?;
        let package = serde_json::from_str(&package_json)
            .with_context(|| format!("Trying to parse {}", package_json_path.to_string_lossy()))?;
        packages.push(package);
    }
    Ok(packages)
}

type Graph = HashMap<String, Vec<String>>;

fn build_dependency_graph(packages: &Vec<Package>) -> Result<Graph> {
    let mut graph = Graph::new();
    let names: Vec<String> = packages
        .iter()
        .map(|package| package.name.clone())
        .collect::<_>();
    for package in packages {
        let dependency_names = package
            .dependencies
            .iter()
            .chain(package.dev_dependencies.iter())
            .map(|(k, _)| k.clone())
            .filter(|n| names.contains(n))
            .collect::<Vec<String>>();
        graph.insert(package.name.clone(), dependency_names);
    }

    Ok(graph)
}

fn topo_sort(graph: &Graph) -> Vec<String> {
    fn topo_sort_rec(graph: &Graph, package: &str, result: &mut Vec<String>) {
        let dependencies = graph.get(package).unwrap();
        let unresolved_dependencies = dependencies
            .iter()
            .filter(|p| !result.contains(p))
            .map(|s| s.clone())
            .collect::<Vec<String>>();
        for ud in unresolved_dependencies {
            topo_sort_rec(graph, &ud, result);
        }
        if !result.contains(&package.to_string()) {
            result.push(package.to_string());
        }
    }

    let mut result: Vec<String> = vec![];
    for (package, _) in graph.into_iter() {
        topo_sort_rec(graph, package, &mut result);
    }

    return result;
}

fn patch_dependencies(folder: &str, dependencies: &Vec<(String, String)>) -> Result<()> {
    for (dependency_name, package_path) in dependencies {
        // sed -e 's#"@ifabric/common-logger": "[^"]*"#"@ifabric/common-logger": "mypackage"#' package.json
        let relative_package_path = pathdiff::diff_paths(package_path, folder).unwrap();
        let s_expression = format!(
            r#"s%"{}": "[^"]*"%"{}": "{}"%"#,
            dependency_name,
            dependency_name,
            relative_package_path.to_string_lossy(),
        );
        let dependency_output = Command::new("sed")
            .current_dir(folder)
            .arg("-i")
            .arg("-e")
            .arg(s_expression)
            .arg("package.json")
            .output()
            .expect(&format!(
                "failed to patch {} in {}",
                dependency_name, folder
            ));

        if !dependency_output.status.success() {
            let std_output = String::from_utf8(dependency_output.stdout).unwrap();
            eprintln!("{}", std_output);
            let error_message = String::from_utf8(dependency_output.stderr).unwrap();
            return Err(io::Error::new(ErrorKind::Other, error_message.to_owned()))
                .with_context(|| format!("Trying to patch {} for {}", dependency_name, folder))?;
        }
    }
    Ok(())
}

fn build_and_package(
    repository: &Entry,
    folder: &str,
    dependencies: &Vec<(String, String)>,
) -> Result<String> {
    // Patch dependencies
    patch_dependencies(folder, dependencies)?;
    // Clean up the folder
    std::fs::remove_file(String::from(
        PathBuf::from(folder)
            .join("package-lock.json")
            .to_string_lossy(),
    ))
    .ok();
    let node_module_path =
        String::from(PathBuf::from(folder).join("node_modules").to_string_lossy());
    std::fs::create_dir(&node_module_path).unwrap_or(());
    std::fs::remove_dir_all(&node_module_path)?;
    // Install dependencies
    println!("Installing dependencies for {}", folder);
    let dependency_output = Command::new("npm")
        .current_dir(folder)
        .arg("install")
        .output()
        .expect(&format!("failed to install dependencies in {}", folder));

    if !dependency_output.status.success() {
        let error_message = String::from_utf8(dependency_output.stderr).unwrap();
        return Err(io::Error::new(ErrorKind::Other, error_message.to_owned()))
            .with_context(|| format!("Trying to install dependencies in {}", folder))?;
    }
    // Run the build
    println!("Building {}", folder);
    let build_commands = match repository {
        Entry::Repo(r) => &r.build,
        Entry::Folder(f) => &f.build,
    };
    for command in build_commands.iter() {
        let build_output = Command::new("bash")
            .current_dir(folder)
            .arg("-c")
            .arg(command)
            .output()
            .expect(&format!("{}: failed build command {}", folder, command));

        if !build_output.status.success() {
            let std_output = String::from_utf8(build_output.stdout).unwrap();
            eprintln!("{}", std_output);
            let error_message = String::from_utf8(build_output.stderr).unwrap();
            return Err(io::Error::new(ErrorKind::Other, error_message.to_owned()))
                .with_context(|| format!("Trying to build {} with {}", folder, command))?;
        }
    }
    // Create the package
    println!("Packaging {}", folder);
    let package_output = Command::new("npm")
        .current_dir(folder)
        .arg("pack")
        .output()
        .expect(&format!("failed to package {}", folder));

    if !package_output.status.success() {
        let std_output = String::from_utf8(package_output.stdout).unwrap();
        eprintln!("{}", std_output);
        let error_message = String::from_utf8(package_output.stderr).unwrap();
        return Err(io::Error::new(ErrorKind::Other, error_message.to_owned()))
            .with_context(|| format!("Trying to pack {}", folder))?;
    }
    // Why do I need to create this temporary variable????
    let x = String::from_utf8(package_output.stdout).unwrap();
    let package_file = x.split('\n').filter(|s| s.len() > 0).last().unwrap();
    println!("{} generated", package_file);
    Ok(String::from(
        PathBuf::from(folder).join(package_file).to_string_lossy(),
    ))
}

#[derive(Debug)]
struct Project<'a> {
    name: String,
    package: &'a Package,
    repo: &'a Entry,
    folder: &'a String,
}

fn coalesce_projects<'a>(
    entries: &'a Vec<Entry>,
    folders: &'a Vec<String>,
    packages: &'a Vec<Package>,
) -> HashMap<String, Project<'a>> {
    assert_eq!(entries.len(), folders.len());
    assert_eq!(entries.len(), packages.len());
    entries
        .iter()
        .zip(folders)
        .zip(packages)
        .map(|((repo, folder), package)| Project {
            name: package.name.clone(),
            package,
            repo,
            folder,
        })
        .fold(HashMap::new(), |mut acc, project| {
            acc.insert(project.name.clone(), project);
            return acc;
        })
}

fn build_all(
    opt: &Opt,
    projects: &HashMap<String, Project>,
    graph: &Graph,
    order: &Vec<String>,
) -> Result<()> {
    let progress_path = PathBuf::from(opt.output_path.clone()).join("integ.progress");
    let progress_file = progress_path.to_string_lossy();
    let mut package_paths = load_package_paths(&progress_file).unwrap_or(HashMap::new());
    for project_name in order {
        let dependency_packages = graph
            .get(project_name)
            .unwrap()
            .iter()
            .map(|d| (d.clone(), package_paths.get(d).unwrap().clone()))
            .collect::<Vec<(String, String)>>();

        if let Some(project_package_file) = package_paths.get(project_name) {
            // Here we are going to check if the project has a package file that is
            // more recent that the oldest package files of its dependencies.
            // If this is the case, it means it should be rebuilt.
            let mut sorted_dependency_time = dependency_packages
                .iter()
                // No error handling on the metadata retrieval functions here to makes things simple.
                // We assume the package have been created correctly.
                .map(|(_, filepath)| {
                    fs::metadata(filepath)
                        .unwrap()
                        .modified()
                        .unwrap_or(UNIX_EPOCH)
                })
                .collect::<Vec<SystemTime>>();
            // We get an array of dependency packages creation/modification time
            sorted_dependency_time.sort_by(|a, b| b.partial_cmp(a).unwrap()); // oldest first
            if sorted_dependency_time.len() == 0
                || sorted_dependency_time[0]
                    < fs::metadata(project_package_file)
                        .unwrap()
                        .modified()
                        .unwrap()
            {
                continue;
            }
        }
        let project = projects.get(project_name).unwrap();
        let package_path = build_and_package(project.repo, project.folder, &dependency_packages)?;
        package_paths.insert(project_name.clone(), package_path.clone());
        dump_package_paths(&progress_file, &package_paths)
            .with_context(|| format!("Fail while trying to save progress in {}", progress_file))?;
    }
    Ok(())
}

fn dump_package_paths(
    progress_filename: &str,
    package_paths: &HashMap<String, String>,
) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(progress_filename)?;
    for (project_name, package_path) in package_paths {
        writeln!(file, "{} {}", project_name, package_path)?;
    }
    Ok(())
}

fn load_package_paths(progress_filename: &str) -> Result<HashMap<String, String>> {
    let result = std::fs::read_to_string(progress_filename)
        .with_context(|| format!(""))?
        .split('\n')
        .into_iter()
        .map(|line| line.split(' ').collect::<Vec<&str>>())
        .filter(|line| line.len() == 2)
        // ignore entry for which package file do not exists
        .filter(|line| PathBuf::from(line[1].clone()).exists())
        .map(|line| (String::from(line[0]), String::from(line[1])))
        .collect();

    Ok(result)
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let config_file = std::fs::read_to_string(&opt.config)
        .with_context(|| format!("could not read file `{}`", opt.config.to_string_lossy()))?;
    let config = load_config(&config_file)
        .with_context(|| format!("could not read file `{}`", opt.config.to_string_lossy()))?;
    let folders = get_folder_names(&opt, &config);
    retrieve_repositories(&config.repositories, &folders)
        .with_context(|| format!("could not clone repositories"))?;
    let packages = parse_package(&folders).with_context(|| format!("fail to parse package"))?;
    let projects = coalesce_projects(&config.repositories, &folders, &packages);
    let graph = build_dependency_graph(&packages)
        .with_context(|| format!("fail to build dependency graph"))?;
    let topological_order = topo_sort(&graph);

    build_all(&opt, &projects, &graph, &topological_order)
        .with_context(|| format!("Build failed"))?;

    println!("All builds successful!");
    Ok(())
}
