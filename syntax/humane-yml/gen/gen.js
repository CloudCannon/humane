const fs = require("fs");
const path = require("path");

const getJson = () => {
  return require("./humane-yaml-base.js");
};

const writeJson = (baseJson) => {
  const outputFilePath = path.join(
    __dirname,
    "../syntaxes/humane-yaml.tmLanguage.json"
  );
  const formattedJson = JSON.stringify(baseJson, null, 2);
  fs.writeFileSync(outputFilePath, formattedJson);
};

const baseYamlPatterns = [
  {
    include: "#comment",
  },
  {
    include: "#property",
  },
  {
    include: "#directive",
  },
  {
    match: "^---",
    name: "entity.other.document.begin.yaml",
  },
  {
    match: "^\\.{3}",
    name: "entity.other.document.end.yaml",
  },
];

const finalYamlPattern = { include: "#node" };

// Adds language highlighting for multiline strings who keys are, or end with, the language name.
const humaneLanguagePattern = (lang, assignment) => {
  if (!assignment) {
    assignment = `source.${lang}`;
  }
  return {
    begin: `(^\\s*)((?:\\w+_)?${lang}):[\\s]*(>)(-)[\\s]*\\n`,
    beginCaptures: {
      2: {
        name: "entity.name.tag.yaml",
      },
      3: {
        name: "keyword.control.flow.block-scalar.folded.yaml",
      },
      4: {
        name: "storage.modifier.chomping-indicator.yaml",
      },
    },
    end: "^(?!(\\1\\s{2}|\\n))",
    contentName: assignment,
    patterns: [
      {
        include: assignment,
      },
    ],
  };
};

const humaneStepPatterns = [
  {
    // Highlights reference keys to common steps
    match: "^(  - )(ref:) (.+)",
    captures: {
      1: {
        name: "punctuation.definition.list.item.yaml",
      },
      2: {
        name: "entity.name.tag.yaml",
      },
      3: {
        name: "support.class",
      },
    },
  },
  {
    // Errors out the first key in an object in arrays at the test indentation level,
    // if that key doesn't match an expected humane test key
    match: "^(  - )((?!(?:step|snapshot)\\b)[^:]*):(.+)",
    captures: {
      1: {
        name: "punctuation.definition.list.item.yaml",
      },
      2: {
        name: "invalid",
      },
      3: {
        name: "string.quoted.double",
      },
    },
  },
  {
    // Highlights strings or the first key of an object as a human function,
    // and highlights the parameters within it as strings
    begin: "^(  - )(?:(\\w+)(:))?\\s*(?:\"|')?",
    beginCaptures: {
      1: {
        name: "punctuation.definition.list.item.yaml",
      },
      2: {
        name: "entity.name.tag.yaml",
      },
      3: {
        name: "punctuation",
      },
    },
    end: "(?:\"|')?\\s*\\n",
    patterns: [
      {
        match: '("[^"]*")',
        captures: {
          1: {
            name: "string.quoted.double",
          },
        },
      },
      {
        match: "('[^']*')",
        captures: {
          1: {
            name: "string.quoted.single",
          },
        },
      },
      {
        match: "(\\{\\{[^\\]]*\\}\\})",
        captures: {
          1: {
            name: "string.quoted.double",
          },
        },
      },
      {
        match: "(.+?)",
        captures: {
          1: {
            name: "support.function",
          },
        },
      },
    ],
  },
];

const baseJson = getJson();

baseJson.patterns = [
  ...baseYamlPatterns,
  humaneLanguagePattern("html", "text.html.basic"),
  humaneLanguagePattern("md", "text.html.markdown"),
  humaneLanguagePattern("shell"),
  humaneLanguagePattern("js"),
  humaneLanguagePattern("ts"),
  humaneLanguagePattern("yml"),
  humaneLanguagePattern("yaml"),
  humaneLanguagePattern("toml"),
  humaneLanguagePattern("json"),
  humaneLanguagePattern("css"),
  ...humaneStepPatterns,
  finalYamlPattern,
];

writeJson(baseJson);

console.log("✨ Done ✨");
