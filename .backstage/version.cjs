const fs = require("fs");
const path = require("path");

const version = process.env.GIT_VERSION;
const version_re = /version\s*[=:]\s*"0.0.0"/;

const err = (m) => {
    console.error(m);
    process.exit(1);
}

if (!version) err("Script expected a GIT_VERSION environment variable");

const file = path.join(__dirname, "../humane/Cargo.toml");
if (!fs.existsSync(file)) err(`Script expected a file at ${file}`);

let contents = fs.readFileSync(file, { encoding: "utf-8" });
if (!version_re.test(contents)) err(`Expected file to contain a version of "0.0.0"`);

contents = contents.replace(version_re, `version = "${version}"`);
fs.writeFileSync(file, contents);
