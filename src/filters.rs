use std::fmt::{Display, Formatter, Write};

use askama::Html;
use askama_escape::Escaper;

const SECTION: char = '§';

#[derive(Clone, Debug, Default)]
pub struct Span<'a> {
    class: &'a [String],
    content: &'a str,
    color: Option<&'a str>,
}

impl<'a> Display for Span<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("<span class=\"")?;
        if let Some(color) = &self.color {
            f.write_str(color)?;
            f.write_char(' ')?;
        }
        for class in self.class {
            f.write_str(class)?;
            f.write_char(' ')?;
        }
        f.write_char('"')?;
        f.write_char('>')?;
        Html.write_escaped(&mut *f, self.content)?;
        f.write_str("</span>")?;
        Ok(())
    }
}

impl<'a> Span<'a> {
    pub const fn new(class: &'a [String], color: Option<&'a str>, content: &'a str) -> Self {
        Self {
            class,
            content,
            color,
        }
    }
}

pub fn mojang_colorize<T: Display>(s: T) -> askama::Result<String> {
    let s = s.to_string();
    let mut output = String::new();
    let mut last_was_section = false;
    let mut class = vec![];
    let mut color: Option<String> = None;

    let mut start = 0;
    let mut idx = 0;

    for char in s.chars() {
        let char = char.to_ascii_lowercase();
        if char == SECTION {
            let next = Span::new(&class, color.as_deref(), &s[start..idx]);
            write!(output, "{next}")?;
            last_was_section = true;
            idx += char.len_utf8();
            continue;
        }
        if char == '\n' {
            let next = Span::new(&class, color.as_deref(), &s[start..idx]);
            write!(output, "{next}")?;
            output.push_str("<br />");
            start = idx;
        } else if last_was_section {
            match char {
                'a'..='f' | '0'..='9' => {
                    color = Some(format!("motd-style-{char}"));
                }
                'k'..='o' => {
                    class.push(format!("motd-style-{char}"));
                }
                'r' => {
                    color = None;
                    class = Vec::new();
                }
                _ => {}
            }
        }
        idx += char.len_utf8();
        if last_was_section {
            start = idx;
        }
        last_was_section = false;
    }
    let next = Span::new(&class, color.as_deref(), &s[start..idx]);
    write!(output, "{next}")?;
    Ok(output)
}

#[allow(clippy::unnecessary_wraps)]
pub fn api_color<T: Display>(s: T) -> askama::Result<&'static str> {
    Ok(match s.to_string().as_str() {
        "Operational" => "green",
        "PossibleProblems" => "yellow",
        "DefiniteProblems" => "red",
        _ => "blue",
    })
}

#[allow(clippy::unnecessary_wraps)]
pub fn api_words<T: Display>(s: T) -> askama::Result<&'static str> {
    Ok(match s.to_string().as_str() {
        "Operational" => "OK",
        "PossibleProblems" => "Flaky",
        "DefiniteProblems" => "Down",
        _ => "Unknown",
    })
}
