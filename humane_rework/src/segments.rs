use std::{collections::HashMap, hash::Hash};

use crate::{errors::HumaneInputError, options::HumaneContext};

use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum HumaneSegment {
    Literal(String),
    Value(serde_json::Value),
    Variable(String),
}

#[derive(Debug, Clone)]
pub struct HumaneSegments {
    pub segments: Vec<HumaneSegment>,
}

impl Hash for HumaneSegments {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        use HumaneSegment::*;

        for seg in &self.segments {
            match seg {
                Literal(lit) => lit.hash(state),
                Value(_) | Variable(_) => 0.hash(state),
            }
        }
    }
}

impl PartialEq for HumaneSegments {
    fn eq(&self, other: &Self) -> bool {
        use HumaneSegment::*;

        if self.segments.len() != other.segments.len() {
            return false;
        }

        self.segments
            .iter()
            .zip(other.segments.iter())
            .all(|(a, b)| match a {
                Literal(_) => a == b,
                Value(_) | Variable(_) => matches!(b, Variable(_)),
            })
    }
}

impl Eq for HumaneSegments {}

impl HumaneSegments {
    pub fn get_comparison_string(&self) -> String {
        use HumaneSegment::*;

        self.segments
            .iter()
            .map(|s| match s {
                Literal(l) => l,
                Value(_) | Variable(_) => "{___}",
            })
            .collect()
    }

    pub fn get_as_string(&self) -> String {
        use HumaneSegment::*;

        self.segments
            .iter()
            .map(|s| match s {
                Literal(l) => l.clone(),
                Value(val) => format!("\"{val}\""),
                Variable(var) => format!("{{{var}}}"),
            })
            .collect()
    }
}

pub struct SegmentArgs<'a> {
    args: HashMap<String, &'a serde_json::Value>,
    placeholder_delim: String,
    placeholders: HashMap<String, String>,
}

impl<'a> SegmentArgs<'a> {
    pub fn build(
        reference_instruction: &HumaneSegments,
        supplied_instruction: &'a HumaneSegments,
        supplied_args: &'a HashMap<String, serde_json::Value>,
        ctx: Option<&HumaneContext>,
    ) -> Result<SegmentArgs<'a>, HumaneInputError> {
        let mut args = HashMap::new();

        for (reference, supplied) in reference_instruction
            .segments
            .iter()
            .zip(supplied_instruction.segments.iter())
        {
            let HumaneSegment::Variable(inst_key) = reference else {
                continue;
            };

            match supplied {
                HumaneSegment::Value(val) => {
                    args.insert(inst_key.to_owned(), val);
                }
                HumaneSegment::Variable(var) => {
                    let Some(var_val) = supplied_args.get(var) else {
                        return Err(HumaneInputError::NonexistentArgument {
                            arg: var.to_string(),
                            has: supplied_args.keys().cloned().collect::<Vec<_>>().join(", "),
                        });
                    };
                    args.insert(inst_key.to_owned(), var_val);
                }
                HumaneSegment::Literal(l) => panic!("{l} should be unreachable"),
            }
        }

        Ok(Self {
            args,
            placeholders: ctx
                .map(|c| c.params.placeholders.clone())
                .unwrap_or_default(),
            placeholder_delim: ctx
                .map(|c| c.params.placeholder_delimiter.clone())
                .unwrap_or_default(),
        })
    }

    pub fn get_value(&self, k: impl AsRef<str>) -> Result<serde_json::Value, HumaneInputError> {
        let Some(value) = self.args.get(k.as_ref()) else {
            return Err(HumaneInputError::NonexistentArgument {
                arg: k.as_ref().to_string(),
                has: self.args.keys().cloned().collect::<Vec<_>>().join(", "),
            });
        };

        let mut value = (*value).clone();
        replace_inside_value(&mut value, &self.placeholder_delim, &self.placeholders);

        Ok(value)
    }

    pub fn get_string(&self, k: impl AsRef<str>) -> Result<String, HumaneInputError> {
        let Some(value) = self.args.get(k.as_ref()) else {
            return Err(HumaneInputError::NonexistentArgument {
                arg: k.as_ref().to_string(),
                has: self.args.keys().cloned().collect::<Vec<_>>().join(", "),
            });
        };

        let mut value = (*value).clone();
        replace_inside_value(&mut value, &self.placeholder_delim, &self.placeholders);

        let found = match value {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
            Value::String(st) => return Ok(st),
        };

        return Err(HumaneInputError::IncorrectArgumentType {
            arg: k.as_ref().to_string(),
            was: found.to_string(),
            expected: "string".to_string(),
        });
    }
}

fn replace_inside_value(value: &mut Value, delim: &str, placeholders: &HashMap<String, String>) {
    use Value::*;

    match value {
        Null | Bool(_) | Number(_) => {}
        Value::String(s) => {
            if s.contains(delim) {
                for (placeholder, value) in placeholders.iter() {
                    let matcher = format!("{delim}{placeholder}{delim}");

                    if s.contains(&matcher) {
                        *s = s.replace(&matcher, value);
                    }
                }
            }
        }
        Value::Array(vals) => {
            vals.iter_mut().for_each(|v| {
                replace_inside_value(v, delim, placeholders);
            });
        }
        Value::Object(o) => {
            o.values_mut().for_each(|v| {
                replace_inside_value(v, delim, placeholders);
            });
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        civilization::Civilization,
        definitions::{register_instructions, HumaneInstruction},
        errors::HumaneStepError,
        options::HumaneParams,
        parser::parse_segments,
    };

    use super::*;

    #[test]
    fn test_building_args() {
        let segments_def = parse_segments("I have a {name} file with the contents {var}")
            .expect("Valid instruction");

        let user_instruction =
            parse_segments("I have a \"index.html\" file with the contents ':)'")
                .expect("Valid instruction");

        let input = HashMap::new();

        let args = SegmentArgs::build(&segments_def, &user_instruction, &input, None)
            .expect("Args built successfully");

        let Ok(str) = args.get_string("name") else {
            panic!(
                "Argument was not a string, got {:?}",
                args.get_string("name")
            );
        };
        assert_eq!(str, "index.html");
    }

    #[test]
    fn test_arg_placeholders() {
        let instruction_def = parse_segments("I have a {name} file with the contents {var}")
            .expect("Valid instruction");

        let user_instruction =
            parse_segments("I have a \"index.%ext%\" file with the contents ':)'")
                .expect("Valid instruction");

        let input = HashMap::new();
        let mut params = HumaneParams::default();
        params.placeholders.insert("ext".into(), "pdf".into());
        let ctx = HumaneContext {
            version: "test",
            working_directory: std::env::current_dir().unwrap(),
            params,
        };

        let args = SegmentArgs::build(&instruction_def, &user_instruction, &input, Some(&ctx))
            .expect("Args built successfully");

        let Ok(str) = args.get_string("name") else {
            panic!(
                "Argument was not a string, got {:?}",
                args.get_string("name")
            );
        };
        assert_eq!(str, "index.pdf");
    }

    // Segments should alias to each other regardless of the contents of their
    // variables or values.
    #[test]
    fn test_segments_equality() {
        let segments_a = parse_segments("I have a 'index.html' file with the contents {var}")
            .expect("Valid segments");

        let segments_b = parse_segments("I have a {filename} file with the contents {var}")
            .expect("Valid segments");

        let segments_c = parse_segments("I have one {filename} file with the contents {var}")
            .expect("Valid segments");

        assert_eq!(segments_a, segments_b);

        let mut map = HashMap::new();
        map.insert(&segments_b, "b");

        assert_eq!(map.get(&&segments_a), Some(&"b"));

        assert_ne!(segments_b, segments_c);
        assert_eq!(map.get(&&segments_c), None);
    }

    #[test]
    fn test_complex_placeholders() {
        let placeholders = HashMap::from([
            ("cloud".to_string(), "cannon".to_string()),
            ("thekey".to_string(), "the value".to_string()),
        ]);

        let start_value: serde_json::Value = serde_json::from_str(
            r#"
            {
                "title": "Hello cloud%cloud%",
                "tags": [ "cannon", "%cloud%" ],
                "nested": {
                    "null": null,
                    "count": 3,
                    "replaced": "thekey is %thekey%"
                }
            }
        "#,
        )
        .unwrap();

        let mut end_value = start_value.clone();
        replace_inside_value(&mut end_value, "%", &placeholders);

        let expected_end_value: serde_json::Value = serde_json::from_str(
            r#"
            {
                "title": "Hello cloudcannon",
                "tags": [ "cannon", "cannon" ],
                "nested": {
                    "null": null,
                    "count": 3,
                    "replaced": "thekey is the value"
                }
            }
        "#,
        )
        .unwrap();

        assert_eq!(end_value, expected_end_value);
    }
}
