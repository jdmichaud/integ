use std::format;
use std::io::{self, Write};
use std::process::Command;
use std::thread;

fn synchronous() {
    let repo = "git@gitlab.com:jdmichaud/observable.git";
    let output = Command::new("git")
        .arg("clone")
        .arg(repo)
        .output()
        .expect(&format!("fail to clone {}", repo));

    if !output.status.success() {
        io::stderr().write_all(&output.stderr).unwrap();
        std::process::exit(1);
    }
    println!("{} cloned", repo);
}

fn asynchronous() {
    let repo = "git@gitlab.com:jdmichaud/observable.git";
    let jh = thread::spawn(move || {
        return Command::new("git")
            .arg("clone")
            .arg(repo)
            .output()
            .expect(&format!("fail to clone {}", repo));
    });
    println!("wait on thread");
    let output = jh.join().unwrap();
    if !output.status.success() {
        io::stderr().write_all(&output.stderr).unwrap();
        std::process::exit(1);
    }
    println!("{} cloned", repo);
}

fn main() {
    synchronous();
}
