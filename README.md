## integ

`integ` is a tool used to integrate a system of interdependent frontend projects.
The projets must define their dependencies in a `package.json` file. `integ`
will clone the projects, build them, generate packages and install the resulting
package as dependencies in the dependent projects and build them in turn.

Once done, you will be sure that all projects build with the particular versions
you need.

⚠️ `integ` works best with `npm` driven projects.

## Example

Let's say you have three project A, B and C with the following dependencies:
```
A -> B
A -> C
B -> C
```
So B depends on A and C depends on A and B. Moreover, you have pushed branches
to the git repository of those projects this way:
```
A (my-branch-with-breaking-change)
B (fix-issue)
C (integ-breaking-change)
```

In order to ensure all those branch builds together, create a config file as such:
```
repositories:
    - url: http://mygit.com/user/project-A
      branch: my-branch-with-breaking-change
      build:
        - npm run test
        - npm run build
        - npm run generate-docs
    - url: http://mygit.com/user/project-B
      branch: fix-issue
      build:
        - npm run build
    - path: /path/to/project-C
      build:
        - npm run all
```

`integ` will:
1. clone project A, B and C,
2. read the package.json to graph the dependencies,
3. build A, generate a package for A,
4. install this newly generate package in B,
5. build B, generate a package for B,
6. install the package for A and B in C and
7. build C

## Installation

`integ` has no crate publish for now. To install, clone this repository and
build it (necessitate cargo):

```
git clone https://github.com/jdmichaud/integ
cd integ
cargo run -- -c my-config.yaml -o output-folder
```

## FAQ

**Does this work with any type of projects?**

No, only javasript/typescript components with a package.json are handled by `integ`.

**How to force recompile of a particular component**

Remove the generated package file and `integ` will rebuild it from scratch.

