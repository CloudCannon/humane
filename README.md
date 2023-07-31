# Humane

Humane is a standalone binary used to run integration tests, generally aiming to support the testing of CLI tools.
This project primarily exists to support CloudCannon's open-source tooling(e.g. https://github.com/CloudCannon/pagefind/, https://github.com/CloudCannon/rosey/).

At present, Humane is a thin wrapper around a [Cucumber](https://cucumber.io/) testing framework, https://github.com/cucumber-rs/cucumber.Predefined steps are supplied, tailored to testing our applications. Temporary testing directories are created automatically, and various steps exist for running commands, interacting with the filesystem, and hosting files to test in the browser using [chromiumoxide](https://github.com/mattsse/chromiumoxide).

## Stability

This tool is primarily an internal dependency of CloudCannon's open-source work, and isn't (yet) targeting any wider usage. There are plans to change the internals of this repository in the future. Most likely it will move away from using Cucumber directly to being Cucumber-inspired, with a slightly different testing syntax. The step definitions will also change in future releases, to be more consistent in their formatting.

As such, feel free to use this tool, but instead of `npx -y humane@latest` it is highly recommended to pin a specific version, i.e. `npx -y humane@v0.9.0`. Humane _does_ keep a changelog and this is included in releases, so any notable breaking changes will be noted there (but will likely still live under a `0.x` version).

## Usage

Running Humane usually looks like the following:

```bash
TEST_BINARY=../target/release/pagefind npx -y humane@latest
```

Humane is distributed using an npm wrapper, which downloads the precompiled binary for your system automatically. Alternatively, you can download a binary directly from the releases page on GitHub, and run that binary in place of `npx -y humane@latest`. 

The `TEST_BINARY` environment variable is used for the step `When I run my program`.

`.feature` files will be auto-discovered in any directories beneath the directory you run the command in.

## Steps

The steps are not currently documented — skimming this source code will give insight, or the best resource is Pagefind's integration test directory, which contains extensive use of the steps. 
