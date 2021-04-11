#[macro_use]
extern crate lazy_static;
use pulldown_cmark::{html, Event, Options, Parser, Tag};
use regex::Regex;
use std::fs;

lazy_static! {
    static ref YAML_RE: Regex =
        Regex::new(r"^[[:space:]]*---(\r?\n(?s).*?(?-s))---\r?\n?((?s).*(?-s))$").unwrap();
    static ref INLINE_FOOTNOTE: Regex = Regex::new("\\^\\[(.*)\\]").unwrap();
    static ref NORMAL_FOOTNOTE: Regex = Regex::new("\\[\\^(.*)\\]:(.*)$").unwrap();
}

fn main() {
    let markdown_input = fs::read_to_string("E:\\kopp\\Devolution of BSB.md").expect("no file :(");
    let (_frontmatter, markdown) = split_frontmatter(&markdown_input);
    let (content, footnotes) = build_content(markdown);

    let mut dest = String::with_capacity(content.len() * 2);
    write_content(&mut dest, content);
    write_footnotes(&mut dest, footnotes);

    fs::write("output.html", &dest).expect("Failed to write output");
}

fn split_frontmatter(content: &str) -> (&str, &str) {
    let cap = YAML_RE.captures(content).unwrap();
    return (cap.get(1).unwrap().as_str(), cap.get(2).unwrap().as_str());
}

fn build_content(markdown: &str) -> (String, Vec<String>) {
    let mut footnotes: Vec<String> = Vec::new();
    let mut content: Vec<String> = Vec::new();
    for line in markdown.lines() {
        match line {
            line if line.starts_with("[^") => footnotes.push(line.into()),
            _ => content.push(replace_footnote_html(line.into(), &mut footnotes)),
        }
    }
    return (content.join("\n"), footnotes);
}

fn replace_footnote_html(mut line: String, footnotes: &mut Vec<String>) -> String {
    for cap in INLINE_FOOTNOTE.captures_iter(&line.clone()) {
        let label = format!("[^fn-{}]", footnotes.len());
        line = line.replace(&cap[0], label.as_str());

        let footnoted = format!("{}: {}", label, &cap[1]);
        footnotes.push(footnoted);
    }
    return line;
}

fn write_content<'a>(mut dest: &mut String, content: String) {
    let mut footnote_no = 0;
    let parser = Parser::new_ext(&content, parser_options());
    let events = parser.map(|event| match event {
        Event::FootnoteReference(name) => {
            footnote_no += 1;
            let formatted = format!(
                "<sup class=\"fn\"><a id=\"{}-back\" href=\"#{}\">[{}]</a></sup>",
                name, name, footnote_no
            );
            Event::Html(formatted.into())
        }
        _ => event,
    });
    html::push_html(&mut dest, events);
}

fn write_footnotes(mut dest: &mut String, footnotes: Vec<String>) {
    let mut formatted_footnotes = String::new();
    if !footnotes.is_empty() {
        formatted_footnotes = format!(
            "---\n# Footnotes\n{}",
            footnotes
                .iter()
                .map(|f| format_footnote_li(f))
                .collect::<Vec<String>>()
                .join("\n")
        );
    }
    let mut footnote_no = 0;
    let parser = Parser::new_ext(&formatted_footnotes, parser_options());
    let events = parser.map(|event| match event {
        Event::Start(Tag::Item) => {
            let cap = NORMAL_FOOTNOTE.captures(&footnotes[footnote_no]).unwrap();
            footnote_no += 1;
            Event::Html(format!("<li id=\"{}\">", &cap[1]).into())
        },
        _ => event
    });
    html::push_html(&mut dest, events);
}

fn format_footnote_li(line: &String) -> String {
    let cap = NORMAL_FOOTNOTE.captures(line).unwrap();
    return format!(
        "1. {} <a class=\"fn-back\" href=\"#{}-back\">↩</a>",
        &cap[2], &cap[1]
    );
}

fn parser_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TABLES);
    return options;
}
