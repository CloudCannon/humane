const fs = require("fs");
const path = require("path");

const version = process.env.GIT_VERSION;
const version_re = /version = "0.0.0"/;

const err = (m) => {
    console.error(m);
    process.exit(1);
}

if (!version) err("Versioning expected a GIT_VERSION environment variable");

const file = path.join(__dirname, "../humane/Cargo.toml");
if (!fs.existsSync(file)) err(`Versioning expected a file at ${file}`);

let contents = fs.readFileSync(file, { encoding: "utf-8" });
if (!version_re.test(contents)) err(`Expected file to contain version = "0.0.0"`);

contents = contents.replace(version_re, `version = "${version}"`);
fs.writeFileSync(file, contents);
