use crate::context::GeneratorContext;
use crate::deserialize::{deserialize_tags, utc_date};
use chrono::{Date, Utc};
use log::warn;
use pulldown_cmark::{html, CowStr, Event, LinkType, Options, Parser, Tag};
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

#[derive(Debug, Deserialize, Clone)]
pub struct Frontmatter {
    pub title: Option<String>,

    #[serde(with = "utc_date", default)]
    pub date: Option<Date<Utc>>,
    #[serde(deserialize_with = "deserialize_tags")]
    pub tags: Vec<String>,
    pub author: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MarkdownFile {
    pub path: PathBuf,
    pub frontmatter: Frontmatter,
    markdown: String,
}

pub struct Asset {
    pub source: PathBuf,
    pub target: PathBuf,
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
        let mut path = self
            .path
            .strip_prefix(base_directory)
            .unwrap()
            .parent()
            .unwrap();
        let mut path_parts = Vec::new();
        while path.to_str().unwrap().ne("") {
            path_parts.insert(
                0,
                MarkdownFile::slugify(path.file_name().unwrap().to_str().unwrap()),
            );
            path = path.parent().unwrap();
        }
        path_parts.push(MarkdownFile::slugify(&self.title()));
        return path_parts.join("/");
    }

    fn slugify(path: &str) -> String {
        return slug::slugify(path.replace("'", ""));
    }

    pub fn render_to_html(&self, ctx: &GeneratorContext) -> (String, Vec<Asset>) {
        let (content, footnotes) = self.split_content_and_footnotes();

        let mut dest = String::with_capacity(content.len() * 2);
        let mut assets = Vec::new();
        MarkdownFile::render_content_to_html(&mut dest, &mut assets, &ctx, content.join("\n"));
        MarkdownFile::render_footnotes_to_html(&mut dest, &ctx, footnotes);
        return (dest, assets);
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
    fn render_content_to_html(
        mut dest: &mut String,
        assets: &mut Vec<Asset>,
        ctx: &GeneratorContext,
        content: String,
    ) {
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
            Event::Start(Tag::Heading(level)) => Event::Html(format!("<h{}>", level + 1).into()),
            Event::End(Tag::Heading(level)) => Event::Html(format!("</h{}>", level + 1).into()),
            Event::Start(Tag::Link(link_type, dest, title)) => {
                rewrite_relative_url(&ctx, link_type, dest, title)
            }
            Event::Start(Tag::Image(link_type, dest, title)) => {
                if is_relative_url(dest.to_string()) {
                    let source = PathBuf::from(dest.into_string());
                    let relative_target_path =
                        format!("static/{}", source.file_name().unwrap().to_str().unwrap());
                    let absolute_url =
                        format!("{}/{}", &ctx.config.base_url, &relative_target_path);
                    assets.push(Asset {
                        source: source.into(),
                        target: PathBuf::from(relative_target_path.clone()),
                    });
                    Event::Start(Tag::Image(link_type, absolute_url.into(), title))
                } else {
                    Event::Start(Tag::Image(link_type, dest, title))
                }
            }
            _ => event,
        });
        html::push_html(&mut dest, events);
    }

    /// Writes the footnotes to HTML
    fn render_footnotes_to_html(
        mut dest: &mut String,
        ctx: &GeneratorContext,
        footnotes: Vec<String>,
    ) {
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
            Event::Start(Tag::Link(link_type, dest, title)) => {
                rewrite_relative_url(&ctx, link_type, dest, title)
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
        options.insert(Options::ENABLE_SMART_PUNCTUATION);

        if enable_footnotes {
            options.insert(Options::ENABLE_FOOTNOTES);
        }

        return options;
    }
}

fn is_relative_url(dest: String) -> bool {
    return !dest.contains("://");
}

fn rewrite_relative_url<'a>(
    ctx: &'a GeneratorContext,
    link_type: LinkType,
    dest: CowStr<'a>,
    title: CowStr<'a>,
) -> Event<'a> {
    let mut target = dest.clone();
    if is_relative_url(dest.to_string()) {
        // Obsidian-ish quirk: whitespace is replaced by %20
        let replaced = dest.replace("%20", " ").to_string();

        let post = &ctx
            .posts
            .iter()
            .find(|(_, post)| replaced == post.original_file_name.replace("\\", "/"));
        if let Some((_, post)) = post {
            target = CowStr::from(format!("{}/{}", &ctx.config.base_url, &post.relative_url));
        } else {
            warn!("Unable to resolve relative link: {}", dest.to_string());
        }
    };

    Event::Start(Tag::Link(link_type, target, title))
}
