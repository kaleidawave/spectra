use super::utilities::TextWithSource;
use super::{Input, Options, Test, options::TransformOutput};
use simple_markdown_parser::{CodeBlock, MarkdownElement, QuoteBlock, parse};
use simple_yaml_parser::{RootYAMLValue, YAMLKey, parse as parse_yaml};

pub(crate) fn parse_yaml_config(content: &str) -> Options {
    let mut options = Options::default();
    parse_yaml(&content, |keys, value| {
        parse_yaml_keys(keys, value, &mut options);
    })
    .expect("invalid file");
    options
}

fn parse_yaml_keys<'a>(keys: &[YAMLKey<'a>], value: RootYAMLValue<'a>, options: &mut Options) {
    use YAMLKey::Slice;

    match keys {
        // [Slice("expected_runner")] => {
        // 	if let simple_yaml_parser::RootYAMLValue::String(value) = value {
        // 		// TODO this will be different in future right?
        // 		expected_runner = Some(value.to_owned());
        // 	} else {
        // 		panic!("expected runner to be string")
        // 	}
        // }
        [Slice("include-language-as-option")] => {
            // prepends *language*\n---\n
            todo!();
        }
        [Slice("transform")] => {
            // removes lines that end in some substring
            options.transform = match value {
                // TODO RootYAMLValue::Null => TransformOutput::None,
                RootYAMLValue::String("basic") => TransformOutput::Basic,
                RootYAMLValue::String("falsy") => TransformOutput::Falsy,
                value => {
                    panic!("unknown {value:?}. expected 'null', 'basic' or 'falsy'");
                }
            };
        }
        [Slice("merge-stderr")] => {
            // TODO
            // removes lines that end in some substring
            options.merge_stderr = true;
        }
        [Slice("wildcard-lines")] => {
            // TODO
            // removes lines that end in some substring
            options.wildcard_lines = true;
        }
        [Slice("lists-as-code-blocks")] => {
            // TODO
            options.lists_as_code_blocks = true;
            // if let RootYAMLValue::Boolean(value) = value {
            // } else {
            //     eprintln!("expected boolean");
            // }
        }
        keys => {
            eprintln!("unknown {keys:?} {value:?}");
        }
    }
}

#[must_use]
pub fn extract_tests(content: &str, parent_options: &Options) -> Input {
    let mut tests: Vec<Test> = Vec::new();

    // TODO
    let expected_runner = None;

    let mut last_was_with = false;

    let mut in_block = 0;

    let mut file_options = Options::default();
    let mut test_options = Options::default();

    let mut section = String::new();
    let mut test_name = String::new();

    let mut test_case: String = String::new();
    let mut test_arguments: String = String::new();
    let mut test_expected: TextWithSource = TextWithSource::new();

    // TODO
    // let mut only_language: Option<String> = None;

    let result = parse::<()>(content, |element| {
        let mut is_with = false;

        // TODO
        // let _empty_blocks = false;

        let add_new = if let MarkdownElement::Heading { level, .. } = element {
            if test_expected.is_empty() {
                test_case.clear();
                false
            } else {
                level >= 3
            }
        } else if let MarkdownElement::CodeBlock(_) = element {
            !test_expected.is_empty()
        } else {
            false
        };

        if add_new {
            let skip = test_name.ends_with("(skip)");

            let name: String = if let MarkdownElement::Heading { .. } = element {
                if in_block > 0 {
                    in_block += 1;
                    format!("{test_name} ({in_block})")
                } else {
                    in_block = 0;
                    std::mem::take(&mut test_name)
                }
            } else if let MarkdownElement::CodeBlock(_) = element {
                in_block += 1;
                format!("{test_name} ({in_block})")
            } else {
                std::mem::take(&mut test_name)
            };

            let wildcard_lines = parent_options.wildcard_lines
                || file_options.wildcard_lines
                || std::mem::take(&mut test_options.wildcard_lines);

            let merge_stderr = parent_options.merge_stderr
                || file_options.merge_stderr
                || std::mem::take(&mut test_options.merge_stderr);

            // TODO test options
            let transform = parent_options
                .transform
                .clone()
                .as_option()
                .unwrap_or(file_options.transform.clone());

            let test = Test {
                name,
                skip,
                section: section.clone(),
                arguments: std::mem::take(&mut test_arguments),
                case: std::mem::take(&mut test_case),
                expected: std::mem::take(&mut test_expected),
                wildcard_lines,
                merge_stderr,
                transform,
            };

            tests.push(test);
        }

        match element {
            MarkdownElement::Frontmatter(frontmatter) => {
                let result = frontmatter
                    .parse_yaml(|keys, value| parse_yaml_keys(keys, value, &mut file_options));
                if let Err(err) = result {
                    eprintln!("error parsing yaml {err:?}");
                }
            }
            MarkdownElement::Heading { level, content } => {
                // TODO
                if level >= 3 {
                    test_name = content.0.to_owned(); //.no_decoration();
                } else {
                    section = content.0.to_owned(); // .no_decoration();
                }
            }
            MarkdownElement::Paragraph(inner) => {
                if let Some(left) = inner.0.strip_prefix("With `")
                    && let Some(options) = left.strip_suffix('`')
                {
                    options.clone_into(&mut test_arguments);
                } else {
                    is_with = inner.0 == "With";
                }
            }
            MarkdownElement::List(list) => {
                let lists_as_code_blocks =
                    parent_options.lists_as_code_blocks || file_options.lists_as_code_blocks;
                let should_add =
                    lists_as_code_blocks && (!test_case.is_empty() && test_expected.is_empty());
                if should_add {
                    let inner = &list.0.0;
                    let start = inner.as_ptr() as usize - content.as_ptr() as usize;
                    let position = start..(start + inner.len());
                    // TODO more efficient
                    let inner = inner.replace("\\<", "<").replace("\\\"", "\"");
                    test_expected = TextWithSource(inner, position);
                }
            }
            MarkdownElement::CodeBlock(CodeBlock { raw_code, .. }) => {
                if !test_name.is_empty() {
                    if test_case.is_empty() {
                        raw_code.clone_into(&mut test_case);
                    } else if test_expected.is_empty() {
                        let inner = raw_code;
                        let start = inner.as_ptr() as usize - content.as_ptr() as usize;
                        test_expected =
                            TextWithSource(inner.to_owned(), start..(start + inner.len()));
                    }
                }
            }
            MarkdownElement::Quote(QuoteBlock { inner, .. }) => {
                if inner.0.trim() == "> Merge `stderr` here" {
                    test_options.merge_stderr = true;
                }
            }
            _ => {}
        }

        last_was_with = is_with;
        Ok(())
    });

    assert!(result.is_ok(), "{result:?}");
    if !test_expected.is_empty() {
        let skip = test_name.ends_with("(skip)");

        let name = if in_block > 0 {
            in_block += 1;
            format!("{test_name} ({in_block})")
        } else {
            test_name
        };

        let wildcard_lines = parent_options.wildcard_lines
            || file_options.wildcard_lines
            || test_options.wildcard_lines;

        let merge_stderr =
            parent_options.merge_stderr || file_options.merge_stderr || test_options.merge_stderr;

        // TODO test options
        let transform = parent_options
            .transform
            .clone()
            .as_option()
            .unwrap_or(file_options.transform.clone());

        // TODO as macro
        let test = Test {
            name,
            skip,
            section: section.clone(),
            arguments: test_arguments,
            case: test_case,
            expected: test_expected,
            wildcard_lines,
            merge_stderr,
            transform,
        };

        tests.push(test);
    }

    Input {
        tests,
        expected_runner,
    }
}

#[cfg(test)]
mod tests {
    use super::extract_tests;

    #[test]
    fn names() {
        let specification_uppercase = extract_tests(
            include_str!("../include/specification.uppercase.md"),
            &Default::default(),
        );
        assert_eq!(specification_uppercase.tests.len(), 3);

        let specification_uppercase = extract_tests(
            include_str!("../include/specification.lists.md"),
            &Default::default(),
        );
        assert_eq!(specification_uppercase.tests.len(), 3);

        {
            static SPECIFICATION_OPTIONS: &str =
                include_str!("../include/specification.options.md");
            let items = extract_tests(SPECIFICATION_OPTIONS, &Default::default());
            assert_eq!(items.tests.len(), 2);
            assert!(items.tests[0].merge_stderr);
            assert!(!items.tests[1].merge_stderr);
        }
    }

    #[test]
    fn errors() {
        let source = "
## A
### B
```rust
one
```
### C
```rust
two
```
        "
        .trim();

        let items = extract_tests(source, &Default::default());
        assert_eq!(items.tests.len(), 0, "{tests:?}", tests = items.tests);
    }

    #[test]
    fn multiple_tests() {
        let source = "
## A
### B
```rust
one
```
```rust
ONE
```
```rust
two
```
```rust
TWO
```
        "
        .trim();

        let items = extract_tests(source, &Default::default());
        assert_eq!(items.tests.len(), 2);

        assert_eq!(items.tests[0].name, "B (1)");
        assert_eq!(items.tests[0].case, "one");
        assert_eq!(&items.tests[0].expected, "ONE");

        assert_eq!(items.tests[1].name, "B (2)");
        assert_eq!(items.tests[1].case, "two");
        assert_eq!(&items.tests[1].expected, "TWO");
    }
}
