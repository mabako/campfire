use chrono::{Date, Utc};
use pulldown_cmark::{html, Event, Options, Parser, Tag};
use regex::Regex;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

lazy_static! {
    // https://github.com/getzola/zola/blob/1ef8c85f53b4988fdafc0e6271cce590515d55aa/components/front_matter/src/lib.rs#L17
    static ref YAML_RE: Regex =
        Regex::new(r"^[[:space:]]*---(\r?\n(?s).*?(?-s))---\r?\n?((?s).*(?-s))$").unwrap();

    static ref INLINE_FOOTNOTE: Regex = Regex::new("\\^\\[(.*)\\]").unwrap();
    static ref NORMAL_FOOTNOTE: Regex = Regex::new("\\[\\^(.*)\\]:(.*)$").unwrap();
}

pub fn read_markdown_file(path: PathBuf) -> Option<MarkdownFile> {
    let content = fs::read_to_string(&path).unwrap();
    let cap = YAML_RE.captures(&content)?;

    let frontmatter: &str = cap.get(1).map_or("", |m| m.as_str());
    let markdown = cap.get(2).map_or("", |m| m.as_str());
    return Some(MarkdownFile {
        path: path.into(),
        frontmatter: serde_yaml::from_str(frontmatter.into()).unwrap(),
        markdown: markdown.into(),
    });
}

#[derive(Debug, Deserialize)]
pub struct Frontmatter {
    pub title: Option<String>,

    #[serde(with = "utc_date")]
    pub date: Option<Date<Utc>>,
    pub tags: Vec<String>,
}

#[derive(Debug)]
pub struct MarkdownFile {
    pub path: PathBuf,
    pub frontmatter: Frontmatter,
    markdown: String,
}

impl MarkdownFile {
    pub fn title(&self) -> String {
        match &self.frontmatter.title {
            Some(t) => t.into(),
            None => self
                .path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .replace(".md", ""),
        }
    }
    pub fn slug(&self, base_directory: &Path) -> String {
        let relative_parent_path = self
            .path
            .strip_prefix(base_directory)
            .unwrap()
            .parent()
            .unwrap();
        relative_parent_path
            .join(self.title())
            .to_str()
            .unwrap()
            .to_lowercase()
            .replace(" ", "-")
    }

    pub fn render_to_html(&self) -> String {
        let (content, footnotes) = self.split_content_and_footnotes();

        let mut dest = String::with_capacity(content.len() * 2);
        MarkdownFile::render_content_to_html(&mut dest, content.join("\n"));
        MarkdownFile::render_footnotes_to_html(&mut dest, footnotes);
        return dest;
    }

    /// Returns the content
    fn split_content_and_footnotes(&self) -> (Vec<String>, Vec<String>) {
        let mut footnotes: Vec<String> = Vec::new();
        let mut content: Vec<String> = Vec::new();
        for line in self.markdown.lines() {
            match line {
                line if line.starts_with("[^") => footnotes.push(line.into()),
                _ => content.push(MarkdownFile::separate_inline_footnote(
                    line.into(),
                    &mut footnotes,
                )),
            }
        }
        return (content, footnotes);
    }

    /// Separates Obsidian's inline footnotes from the text, since cmark can't handle them.
    fn separate_inline_footnote(mut line: String, footnotes: &mut Vec<String>) -> String {
        for cap in INLINE_FOOTNOTE.captures_iter(&line.clone()) {
            let label = format!("[^fn-{}]", footnotes.len());
            line = line.replace(&cap[0], label.as_str());

            let footnoted = format!("{}: {}", label, &cap[1]);
            footnotes.push(footnoted);
        }
        return line;
    }

    /// Writes the page contents to HTML
    fn render_content_to_html(mut dest: &mut String, content: String) {
        let mut footnote_no = 0;
        let parser = Parser::new_ext(&content, MarkdownFile::parser_options(true));
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

    /// Writes the footnotes to HTML
    fn render_footnotes_to_html(mut dest: &mut String, footnotes: Vec<String>) {
        let mut formatted_footnotes = String::new();
        if !footnotes.is_empty() {
            formatted_footnotes = format!(
                "---\n{}",
                footnotes
                    .iter()
                    .map(|f| MarkdownFile::format_footnote_li(f))
                    .collect::<Vec<String>>()
                    .join("\n")
            );
        }
        let mut footnote_no = 0;
        let parser = Parser::new_ext(&formatted_footnotes, MarkdownFile::parser_options(false));
        let events = parser.map(|event| match event {
            Event::Start(Tag::Item) => {
                let cap = NORMAL_FOOTNOTE.captures(&footnotes[footnote_no]).unwrap();
                footnote_no += 1;
                Event::Html(format!("<li id=\"{}\">", &cap[1]).into())
            }
            _ => event,
        });
        html::push_html(&mut dest, events);
    }

    /// Formats footnotes as lists and to include a back-link.
    fn format_footnote_li(line: &String) -> String {
        let cap = NORMAL_FOOTNOTE.captures(line).unwrap();
        return format!(
            "1. {} <a class=\"fn-back\" href=\"#{}-back\">↩</a>",
            &cap[2], &cap[1]
        );
    }

    /// Returns the default parser options, optionally including footnotes.
    fn parser_options(enable_footnotes: bool) -> Options {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);

        if enable_footnotes {
            options.insert(Options::ENABLE_FOOTNOTES);
        }

        return options;
    }
}

mod utc_date {
    use chrono::{Date, NaiveDate, Utc};
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Date<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parsed = s.parse::<NaiveDate>().map(|s| Date::from_utc(s, Utc));
        match parsed {
            Ok(p) => Ok(Some(p)),
            Err(_) => Ok(None),
        }
    }
}
