use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take, take_while1},
    character::is_digit,
    combinator::{map, not, opt},
    error::ErrorKind,
    multi::{many0, many1},
    sequence::{delimited, pair, preceded, terminated, tuple},
    Err::Error,
    IResult,
};

pub type MarkdownText = Vec<MarkdownInLine>;


// TODO: Right now lists can not be nested and will be parsed as plain text in quotes
//  Add something similar to Markdown::List(Vec<Markdown>) maybe? So maybe this can work:
//  Md::List(vec![
//      Md::Heading(1, vec![MIL::Plain("Food")])
//      Md::List(vec![
//          Md::Text(vec![MIL::Plain("text")],
//          Md::Text(vec![MIL::Bold("bold"), MIL::Plain("text")],
//      ]),
//      Md::Text(vec![Mil::Plain("hope this works")])
//  ])
//  for:
//  - Food
//      - text
//      - **bold** text
//  - hope this works

// TODO:  After that make quote nested?
#[derive(Clone, Debug, PartialEq)]
pub enum Markdown {
    // (num of #, text)
    Heading(usize, MarkdownText),
    OrderedList(Vec<MarkdownText>),
    UnorderedList(Vec<MarkdownText>),
    Quote(Vec<MarkdownText>),
    CodeBlock(String, Option<String>),
    Text(MarkdownText),
}

#[derive(Clone, Debug, PartialEq)]
pub enum MarkdownInLine {
    // (tag, url)
    Link(String, String),
    // (tag, url)
    Image(String, String),
    // (code, language)
    InlineCode(String, Option<String>),
    Bold(String),
    Italic(String),
    Plain(String),
}

// [text](url)
pub fn parse_link(i: &str) -> IResult<&str, (&str, &str)> {
    pair(
        delimited(tag("["), is_not("]"), tag("]")),
        delimited(tag("("), is_not(")"), tag(")")),
    )(i)
}

// ![text](url / path)
fn parse_image(i: &str) -> IResult<&str, (&str, &str)> {
    pair(
        delimited(tag("!["), is_not("]"), tag("]")),
        delimited(tag("("), is_not(")"), tag(")")),
    )(i)
}

// `code`language  (whitespace is the separator for the next)
pub fn parse_inline(i: &str) -> IResult<&str, (&str, Option<&str>)> {
    pair(
        delimited(tag("`"), is_not("`"), tag("`")), // code
        opt(delimited(tag(""), is_not(" \t\r\n"), not(is_not(" \t\r\n")))),                      // language
    )(i)
}

// **text**
pub fn parse_bold(i: &str) -> IResult<&str, &str> {
    delimited(tag("**"), is_not("**"), tag("**"))(i)
}

// *text*
pub fn parse_italic(i: &str) -> IResult<&str, &str> {
    delimited(tag("*"), is_not("*"), tag("*"))(i)
}

// match against all special tags and then join each array
pub fn parse_plain(i: &str) -> IResult<&str, String> {
    map(
        many1(preceded(
            not(alt((tag("*"), tag("`"), tag("["), tag("!["), tag("\n")))),
            take(1u8),
        )),
        |vec| vec.join(""),
    )(i)
}

pub fn parse_markdown_inline(i: &str) -> IResult<&str, MarkdownInLine> {
    alt((
        map(parse_plain, |s| MarkdownInLine::Plain(s.to_string())),
        map(parse_bold, |s| MarkdownInLine::Bold(s.to_string())),
        map(parse_italic, |s| MarkdownInLine::Italic(s.to_string())),
        map(parse_inline, |(code, language)| {
            MarkdownInLine::InlineCode(code.to_string(), language.map(String::from))
        }),
        map(parse_image, |(tag, url)| {
            MarkdownInLine::Image(tag.to_string(), url.to_string())
        }),
        map(parse_link, |(tag, url)| {
            MarkdownInLine::Link(tag.to_string(), url.to_string())
        }),
    ))(i)
}

pub fn parse_markdown_text(i: &str) -> IResult<&str, MarkdownText> {
    terminated(many0(parse_markdown_inline), tag("\n"))(i)
}

pub fn parse_header_tag(i: &str) -> IResult<&str, usize> {
    map(
        terminated(take_while1(|c| c == '#'), tag(" ")),
        |s: &str| s.len(),
    )(i)
}

pub fn parse_header(i: &str) -> IResult<&str, (usize, MarkdownText)> {
    tuple((parse_header_tag, parse_markdown_text))(i)
}

pub fn parse_unordered_list_tag(i: &str) -> IResult<&str, &str> {
    terminated(tag("-"), tag(" "))(i)
}

pub fn parse_unordered_list_element(i: &str) -> IResult<&str, MarkdownText> {
    preceded(parse_unordered_list_tag, parse_markdown_text)(i)
}

pub fn parse_unordered_list(i: &str) -> IResult<&str, Vec<MarkdownText>> {
    many1(parse_unordered_list_element)(i)
}

pub fn parse_ordered_list_tag(i: &str) -> IResult<&str, &str> {
    terminated(
        terminated(take_while1(|d| is_digit(d as u8)), tag(".")),
        tag(" "),
    )(i)
}

pub fn parse_ordered_list_element(i: &str) -> IResult<&str, MarkdownText> {
    preceded(parse_ordered_list_tag, parse_markdown_text)(i)
}

pub fn parse_ordered_list(i: &str) -> IResult<&str, Vec<MarkdownText>> {
    many1(parse_ordered_list_element)(i)
}

// > text
pub fn parse_quote_tag(i: &str) -> IResult<&str, &str> {
    terminated(tag(">"), tag(" "))(i)
}

pub fn parse_quote_line(i: &str) -> IResult<&str, MarkdownText> {
    preceded(parse_quote_tag, parse_markdown_text)(i)
}

// > #text
// > this is a quote
// > - list in quote
// > - list in quote
pub fn parse_quote(i: &str) -> IResult<&str, Vec<MarkdownText>> {
    many1(parse_quote_line)(i)
}

// ``` lang\n
//  text
// ```
//
pub fn parse_code_block(i: &str) -> IResult<&str, (&str, &str)> {
    tuple((
        delimited(tag("```"), is_not("\n"), tag("\n")),
        delimited(tag(""), is_not("```"), tag("```"))
    ))(i)
}

pub fn parse_markdown(i: &str) -> IResult<&str, Vec<Markdown>> {
    many1(alt((
        map(parse_header, |e| Markdown::Heading(e.0, e.1)),
        map(parse_ordered_list, |e| Markdown::OrderedList(e)),
        map(parse_unordered_list, |e| Markdown::UnorderedList(e)),
        map(parse_quote, |e| Markdown::Quote(e)),
        map(parse_code_block, |(language, code)| {
            let mut lang = None;
            let language = language.trim();
            if language != "" {
                lang = Some(String::from(language));
            }
            Markdown::CodeBlock(code.to_string(), lang)
        }),
        map(parse_markdown_text, |e| Markdown::Text(e)),
    )))(i)
}

// Credit:
// Most of tests are copied and edited from HGHimself/prose
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bold() {
        assert_eq!(parse_bold("**bold text**"), Ok(("", "bold text")));
        assert_eq!(parse_bold("**not bold"), Err(Error(("", ErrorKind::Tag))));
        assert_eq!(
            parse_bold("not bold**"),
            Err(Error(("not bold**", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_bold("another not bold"),
            Err(Error(("another not bold", ErrorKind::Tag)))
        );
        assert_eq!(parse_bold("****"), Err(Error(("**", ErrorKind::IsNot))));
        assert_eq!(parse_bold("**"), Err(Error(("", ErrorKind::IsNot))));
        assert_eq!(parse_bold("*"), Err(Error(("*", ErrorKind::Tag))));
        assert_eq!(parse_bold(""), Err(Error(("", ErrorKind::Tag))));
        assert_eq!(
            parse_bold("*this is italic*"),
            Err(Error(("*this is italic*", ErrorKind::Tag)))
        );
    }

    #[test]
    fn test_parse_italics() {
        assert_eq!(
            parse_italic("*italic text*"),
            Ok(("", "italic text"))
        );
        assert_eq!(
            parse_italic("*not italic"),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_italic("not italic*"),
            Err(Error(("not italic*", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_italic("another not italic"),
            Err(Error(("another not italic", ErrorKind::Tag)))
        );
        assert_eq!(parse_italic("*"), Err(Error(("", ErrorKind::IsNot))));
        assert_eq!(parse_italic("**"), Err(Error(("*", ErrorKind::IsNot))));
        assert_eq!(parse_italic(""), Err(Error(("", ErrorKind::Tag))));
        assert_eq!(
            parse_italic("**this is bold**"),
            Err(Error(("*this is bold**", ErrorKind::IsNot)))
        );
    }

    #[test]
    fn test_parse_inline_code() {
        assert_eq!(
            parse_inline("`inline text`"),
            Ok(("", ("inline text", None)))
        );
        assert_eq!(
            parse_inline("`inline text`rust"),
            Ok(("", ("inline text", Some("rust"))))
        );
        assert_eq!(
            parse_inline("`inline text`rust\n"),
            Ok(("\n", ("inline text", Some("rust"))))
        );
        assert_eq!(
            parse_inline("`inline text`rust "),
            Ok((" ", ("inline text", Some("rust"))))
        );
        assert_eq!(
            parse_inline("`not inline"),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_inline("not inline`"),
            Err(Error(("not inline`", ErrorKind::Tag)))
        );
        assert_eq!(parse_inline("``"), Err(Error(("`", ErrorKind::IsNot))));
        assert_eq!(parse_inline("`"), Err(Error(("", ErrorKind::IsNot))));
        assert_eq!(parse_inline(""), Err(Error(("", ErrorKind::Tag))));
    }

    #[test]
    fn test_parse_link() {
        assert_eq!(
            parse_link("[title](https://www.example.com)"),
            Ok(("", ("title", "https://www.example.com")))
        );
        assert_eq!(parse_inline(""), Err(Error(("", ErrorKind::Tag))));
    }

    #[test]
    fn test_parse_image() {
        assert_eq!(
            parse_image("![alt text](image.jpg)"),
            Ok(("", ("alt text", "image.jpg")))
        );
        assert_eq!(parse_inline(""), Err(Error(("", ErrorKind::Tag))));
    }

    #[test]
    fn test_parse_plaintext() {
        assert_eq!(
            parse_plain("1234567890"),
            Ok(("", String::from("1234567890")))
        );
        assert_eq!(
            parse_plain("plaintext"),
            Ok(("", String::from("plaintext")))
        );
        assert_eq!(
            parse_plain("plaintext!"),
            Ok(("", String::from("plaintext!")))
        );
        assert_eq!(
            parse_plain("plaintext!["),
            Ok(("![", String::from("plaintext")))
        );
        assert_eq!(
            parse_plain("plaintext!*"),
            Ok(("*", String::from("plaintext!")))
        );
        assert_eq!(
            parse_plain("plaintext![image"),
            Ok(("![image", String::from("plaintext")))
        );
        assert_eq!(
            parse_plain("plaintext\n"),
            Ok(("\n", String::from("plaintext")))
        );
        assert_eq!(
            parse_plain("*bold text*"),
            Err(Error(("*bold text*", ErrorKind::Not)))
        );
        assert_eq!(
            parse_plain("[example](https://example.com)"),
            Err(Error(("[example](https://example.com)", ErrorKind::Not)))
        );
        assert_eq!(
            parse_plain("`codeblock for bums`"),
            Err(Error(("`codeblock for bums`", ErrorKind::Not)))
        );
        assert_eq!(
            parse_plain("![ but wait theres more](jk)"),
            Err(Error(("![ but wait theres more](jk)", ErrorKind::Not)))
        );
        assert_eq!(
            parse_plain("*italic*"),
            Err(Error(("*italic*", ErrorKind::Not)))
        );
        assert_eq!(
            parse_plain("**bold**"),
            Err(Error(("**bold**", ErrorKind::Not)))
        );
        assert_eq!(
            parse_plain("`inline code`"),
            Err(Error(("`inline code`", ErrorKind::Not)))
        );
        assert_eq!(
            parse_plain("[title](https://example.com)"),
            Err(Error(("[title](https://example.com)", ErrorKind::Not)))
        );
        assert_eq!(
            parse_plain("![alt text](image.jpg)"),
            Err(Error(("![alt text](image.jpg)", ErrorKind::Not)))
        );
        assert_eq!(parse_plain(""), Err(Error(("", ErrorKind::Eof))));
    }

    #[test]
    fn test_parse_markdown_inline() {
        assert_eq!(
            parse_markdown_inline("*italic*"),
            Ok(("", MarkdownInLine::Italic(String::from("italic"))))
        );
        assert_eq!(
            parse_markdown_inline("**bold**"),
            Ok(("", MarkdownInLine::Bold(String::from("bold"))))
        );
        assert_eq!(
            parse_markdown_inline("`inline code`python"),
            Ok(("", MarkdownInLine::InlineCode(String::from("inline code"), Some(String::from("python")))))
        );
        assert_eq!(
            parse_markdown_inline("[title](https://www.example.com)"),
            Ok((
                "",
                (MarkdownInLine::Link(
                    String::from("title"),
                    String::from("https://www.example.com"),
                ))
            ))
        );
        assert_eq!(
            parse_markdown_inline("![text](image.png)"),
            Ok((
                "",
                (MarkdownInLine::Image(String::from("text"), String::from("image.png")))
            ))
        );
        assert_eq!(
            parse_markdown_inline("plaintext!"),
            Ok((
                "",
                MarkdownInLine::Plain(String::from("plaintext!"))
            ))
        );
        assert_eq!(
            parse_markdown_inline("here is some plaintext *but what if we italicize?"),
            Ok((
                "*but what if we italicize?",
                MarkdownInLine::Plain(String::from("here is some plaintext "))
            ))
        );
        assert_eq!(
            parse_markdown_inline("here is some plaintext \n*but what if we italicize?"),
            Ok((
                "\n*but what if we italicize?",
                MarkdownInLine::Plain(String::from("here is some plaintext "))
            ))
        );
        assert_eq!(
            parse_markdown_inline("\n"),
            Err(Error(("\n", ErrorKind::Tag)))
        );
        assert_eq!(parse_markdown_inline(""), Err(Error(("", ErrorKind::Tag))));
    }

    #[test]
    fn test_parse_markdown_text() {
        assert_eq!(parse_markdown_text("\n"), Ok(("", vec![])));
        assert_eq!(
            parse_markdown_text("here is some plaintext\n"),
            Ok((
                "",
                vec![MarkdownInLine::Plain(String::from(
                    "here is some plaintext"
                ))]
            ))
        );
        assert_eq!(
            parse_markdown_text("here is some plaintext *but what if we italicize?*\n"),
            Ok((
                "",
                vec![
                    MarkdownInLine::Plain(String::from("here is some plaintext ")),
                    MarkdownInLine::Italic(String::from("but what if we italicize?")),
                ]
            ))
        );
        assert_eq!(
            parse_markdown_text("here is some plaintext *but what if we italicize?* I guess it doesnt **matter** in my `code`\n"),
            Ok(("", vec![
                MarkdownInLine::Plain(String::from("here is some plaintext ")),
                MarkdownInLine::Italic(String::from("but what if we italicize?")),
                MarkdownInLine::Plain(String::from(" I guess it doesnt ")),
                MarkdownInLine::Bold(String::from("matter")),
                MarkdownInLine::Plain(String::from(" in my ")),
                MarkdownInLine::InlineCode(String::from("code"), None),
            ]))
        );
        assert_eq!(
            parse_markdown_text("here is some plaintext *but what if we italicize?*\n"),
            Ok((
                "",
                vec![
                    MarkdownInLine::Plain(String::from("here is some plaintext ")),
                    MarkdownInLine::Italic(String::from("but what if we italicize?")),
                ]
            ))
        );
        assert_eq!(
            parse_markdown_text("here is some plaintext *but what if we italicize?"),
            Err(Error(("*but what if we italicize?", ErrorKind::Tag))) // Ok(("*but what if we italicize?", vec![MarkdownInline::Plaintext(String::from("here is some plaintext "))]))
        );
    }

    #[test]
    fn test_parse_header_tag() {
        assert_eq!(parse_header_tag("# "), Ok(("", 1)));
        assert_eq!(parse_header_tag("### "), Ok(("", 3)));
        assert_eq!(parse_header_tag("# h1"), Ok(("h1", 1)));
        assert_eq!(parse_header_tag("# h1"), Ok(("h1", 1)));
        assert_eq!(
            parse_header_tag(" "),
            Err(Error((" ", ErrorKind::TakeWhile1)))
        );
        assert_eq!(parse_header_tag("#"), Err(Error(("", ErrorKind::Tag))));
    }

    #[test]
    fn test_parse_header() {
        assert_eq!(
            parse_header("# h1\n"),
            Ok(("", (1, vec![MarkdownInLine::Plain(String::from("h1"))])))
        );
        assert_eq!(
            parse_header("## h2\n"),
            Ok(("", (2, vec![MarkdownInLine::Plain(String::from("h2"))])))
        );
        assert_eq!(
            parse_header("###  h3\n"),
            Ok((
                "",
                (3, vec![MarkdownInLine::Plain(String::from(" h3"))])
            ))
        );
        assert_eq!(parse_header("###h3"), Err(Error(("h3", ErrorKind::Tag))));
        assert_eq!(parse_header("###"), Err(Error(("", ErrorKind::Tag))));
        assert_eq!(parse_header(""), Err(Error(("", ErrorKind::TakeWhile1))));
        assert_eq!(parse_header("#"), Err(Error(("", ErrorKind::Tag))));
        assert_eq!(parse_header("# \n"), Ok(("", (1, vec![]))));
        assert_eq!(parse_header("# test"), Err(Error(("", ErrorKind::Tag))));
    }

    #[test]
    fn test_parse_unordered_list_tag() {
        assert_eq!(parse_unordered_list_tag("- "), Ok(("", "-")));
        assert_eq!(
            parse_unordered_list_tag("- and some more"),
            Ok(("and some more", "-"))
        );
        assert_eq!(
            parse_unordered_list_tag("-"),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_unordered_list_tag("-and some more"),
            Err(Error(("and some more", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_unordered_list_tag("--"),
            Err(Error(("-", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_unordered_list_tag(""),
            Err(Error(("", ErrorKind::Tag)))
        );
    }

    #[test]
    fn test_parse_unordered_list_element() {
        assert_eq!(
            parse_unordered_list_element("- this is an element\n"),
            Ok((
                "",
                vec![MarkdownInLine::Plain(String::from(
                    "this is an element"
                ))]
            ))
        );
        assert_eq!(
            parse_unordered_list_element("- this is an element\n- this is another element\n"),
            Ok((
                "- this is another element\n",
                vec![MarkdownInLine::Plain(String::from(
                    "this is an element"
                ))]
            ))
        );
        assert_eq!(
            parse_unordered_list_element(""),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(parse_unordered_list_element("- \n"), Ok(("", vec![])));
        assert_eq!(
            parse_unordered_list_element("- "),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_unordered_list_element("- test"),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_unordered_list_element("-"),
            Err(Error(("", ErrorKind::Tag)))
        );
    }

    #[test]
    fn test_parse_unordered_list() {
        assert_eq!(
            parse_unordered_list("- this is an element"),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_unordered_list("- this is an element\n"),
            Ok((
                "",
                vec![vec![MarkdownInLine::Plain(String::from(
                    "this is an element"
                ))]]
            ))
        );
        assert_eq!(
            parse_unordered_list("- this is an element\n- here is another\n"),
            Ok((
                "",
                vec![
                    vec![MarkdownInLine::Plain(String::from(
                        "this is an element"
                    ))],
                    vec![MarkdownInLine::Plain(String::from("here is another"))]
                ]
            ))
        );
    }

    #[test]
    fn test_parse_ordered_list_tag() {
        assert_eq!(parse_ordered_list_tag("1. "), Ok(("", "1")));
        assert_eq!(parse_ordered_list_tag("1234567. "), Ok(("", "1234567")));
        assert_eq!(
            parse_ordered_list_tag("3. and some more"),
            Ok(("and some more", "3"))
        );
        assert_eq!(
            parse_ordered_list_tag("1"),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_ordered_list_tag("1.and some more"),
            Err(Error(("and some more", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_ordered_list_tag("1111."),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_ordered_list_tag(""),
            Err(Error(("", ErrorKind::TakeWhile1)))
        );
    }

    #[test]
    fn test_parse_ordered_list_element() {
        assert_eq!(
            parse_ordered_list_element("1. this is an element\n"),
            Ok((
                "",
                vec![MarkdownInLine::Plain(String::from(
                    "this is an element"
                ))]
            ))
        );
        assert_eq!(
            parse_ordered_list_element("1. this is an element\n1. here is another\n"),
            Ok((
                "1. here is another\n",
                vec![MarkdownInLine::Plain(String::from(
                    "this is an element"
                ))]
            ))
        );
        assert_eq!(
            parse_ordered_list_element(""),
            Err(Error(("", ErrorKind::TakeWhile1)))
        );
        assert_eq!(
            parse_ordered_list_element(""),
            Err(Error(("", ErrorKind::TakeWhile1)))
        );
        assert_eq!(parse_ordered_list_element("1. \n"), Ok(("", vec![])));
        assert_eq!(
            parse_ordered_list_element("1. test"),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_ordered_list_element("1. "),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_ordered_list_element("1."),
            Err(Error(("", ErrorKind::Tag)))
        );
    }

    #[test]
    fn test_parse_ordered_list() {
        assert_eq!(
            parse_ordered_list("1. this is an element\n"),
            Ok((
                "",
                vec![vec![MarkdownInLine::Plain(String::from(
                    "this is an element"
                ))]]
            ))
        );
        assert_eq!(
            parse_ordered_list("1. test"),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_ordered_list("1. this is an element\n2. here is another\n"),
            Ok((
                "",
                vec![
                    vec!(MarkdownInLine::Plain(String::from(
                        "this is an element"
                    ))),
                    vec![MarkdownInLine::Plain(String::from("here is another"))]
                ]
            ))
        );
    }

    #[test]
    fn test_parse_codeblock() {
        assert_eq!(
            parse_code_block("```bash\npip install foobar\n```"),
            Ok(("", ("bash", "pip install foobar\n")))
        );
        assert_eq!(
            parse_code_block("```python\nimport foobar\n\nfoobar.pluralize('word') # returns 'words'\nfoobar.pluralize('goose') # returns 'geese'\nfoobar.singularize('phenomena') # returns 'phenomenon'\n```"),
            Ok(("", ("python", "import foobar\n\nfoobar.pluralize('word') # returns 'words'\nfoobar.pluralize('goose') # returns 'geese'\nfoobar.singularize('phenomena') # returns 'phenomenon'\n")))
        );
    }

    #[test]
    fn test_parse_markdown() {
        assert_eq!(
            parse_markdown("# Foobar\n\nFoobar is a Python library for dealing with word pluralization.\n\n```bash\n#!/bin/bash\npip install foobar\n```\n## Installation\n\nUse the package manager [pip](https://pip.pypa.io/en/stable/) to install foobar.\n```python\nimport foobar\n\nfoobar.pluralize('word') # returns 'words'\nfoobar.pluralize('goose') # returns 'geese'\nfoobar.singularize('phenomena') # returns 'phenomenon'\n```"),
            Ok(("", vec![
                Markdown::Heading(1, vec![MarkdownInLine::Plain(String::from("Foobar"))]),
                Markdown::Text(vec![]),
                Markdown::Text(vec![MarkdownInLine::Plain(String::from("Foobar is a Python library for dealing with word pluralization."))]),
                Markdown::Text(vec![]),
                Markdown::CodeBlock(String::from("#!/bin/bash\npip install foobar\n"), Some(String::from("bash"))),
                Markdown::Text(vec![]),
                Markdown::Heading(2, vec![MarkdownInLine::Plain(String::from("Installation"))]),
                Markdown::Text(vec![]),
                Markdown::Text(vec![
                    MarkdownInLine::Plain(String::from("Use the package manager ")),
                    MarkdownInLine::Link(String::from("pip"), String::from("https://pip.pypa.io/en/stable/")),
                    MarkdownInLine::Plain(String::from(" to install foobar.")),
                ]),
                Markdown::CodeBlock(String::from("import foobar\n\nfoobar.pluralize('word') # returns 'words'\nfoobar.pluralize('goose') # returns 'geese'\nfoobar.singularize('phenomena') # returns 'phenomenon'\n"), Some(String::from("python"))),
            ]))
        )
    }

    #[test]
    fn test_parse_quote_tag() {
        assert_eq!(
            parse_quote_tag("> "),
            Ok(("", ">"))
        );
        assert_eq!(
            parse_quote_tag("> this is a quote\n"),
            Ok(("this is a quote\n", ">"))
        );
        assert_eq!(
            parse_quote_tag("> this is a quote\n> this is another quote\n"),
            Ok(("this is a quote\n> this is another quote\n", ">"))
        );
        assert_eq!(
            parse_quote_tag("> **this is a bold quote**\n"),
            Ok(("**this is a bold quote**\n", ">"))
        );
        assert_eq!(
            parse_quote_tag(""),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_quote_tag("not a quote"),
            Err(Error(("not a quote", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_quote_tag(">"),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_quote_tag(">not a quote"),
            Err(Error(("not a quote", ErrorKind::Tag)))
        );
    }

    #[test]
    fn test_parse_quote_text() {
        assert_eq!(
            parse_quote_line("> this is a quote\n"),
            Ok(("", vec![
                MarkdownInLine::Plain(String::from("this is a quote"))
            ]))
        );
        assert_eq!(
            parse_quote_line("> **this is a bold quote**\n> this is another quote\n"),
            Ok(("> this is another quote\n", vec![
                MarkdownInLine::Bold(String::from("this is a bold quote"))
            ]))
        );
        assert_eq!(
            parse_quote_line(""),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_quote_line(">"),
            Err(Error(("", ErrorKind::Tag)))
        );
        assert_eq!(
            parse_quote_line("not a quote"),
            Err(Error(("not a quote", ErrorKind::Tag)))
        );
    }

    #[test]
    fn test_parse_quote() {
        assert_eq!(
            parse_quote("> this is a quote\n"),
            Ok(("", vec![
                vec![MarkdownInLine::Plain(String::from("this is a quote"))],
            ]))
        );
        assert_eq!(
            parse_quote("> **this is a bold quote**\n> this is another quote\n"),
            Ok(("", vec![
                vec![MarkdownInLine::Bold(String::from("this is a bold quote"))],
                vec![MarkdownInLine::Plain(String::from("this is another quote"))]
            ]))
        );
        assert_eq!(
            parse_quote("> - this is a list inside a quote\n> - this the second list\n"),
            Ok(("", vec![
                vec![MarkdownInLine::Plain(String::from("- this is a list inside a quote"))],
                vec![MarkdownInLine::Plain(String::from("- this the second list"))]
            ]))
        );
    }
}
