use std::fmt::{Display, Formatter, Write};

#[allow(unused_imports)]
pub use bustdir::rinja::bust_dir;
use rinja::filters::{Escaper as _, Html};

const SECTION: char = '§';

#[derive(Clone, Debug, Default)]
pub struct Span<'a> {
    class: &'a [String],
    content: &'a str,
    color: Option<&'a str>,
}

impl Display for Span<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.content.is_empty() {
            return Ok(());
        }
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
        Html.write_escaped_str(&mut *f, self.content)?;
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

pub fn mojang_colorize<T: Display>(s: T) -> rinja::Result<String> {
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
pub fn api_color<T: Display>(s: T) -> rinja::Result<&'static str> {
    Ok(match s.to_string().as_str() {
        "Operational" => "green",
        "PossibleProblems" => "yellow",
        "DefiniteProblems" => "red",
        _ => "blue",
    })
}

#[allow(clippy::unnecessary_wraps)]
pub fn api_words<T: Display>(s: T) -> rinja::Result<&'static str> {
    Ok(match s.to_string().as_str() {
        "Operational" => "OK",
        "PossibleProblems" => "Flaky",
        "DefiniteProblems" => "Down",
        _ => "Unknown",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_api_words() {
        assert_eq!(api_words("Operational").unwrap(), "OK");
        assert_eq!(api_words("PossibleProblems").unwrap(), "Flaky");
        assert_eq!(api_words("DefiniteProblems").unwrap(), "Down");
        assert_eq!(api_words("operational").unwrap(), "Unknown");
    }
    #[test]
    fn test_api_colors() {
        assert_eq!(api_color("Operational").unwrap(), "green");
        assert_eq!(api_color("PossibleProblems").unwrap(), "yellow");
        assert_eq!(api_color("DefiniteProblems").unwrap(), "red");
        assert_eq!(api_color("operational").unwrap(), "blue");
    }
    #[test]
    fn test_span_no_color() {
        let class = ["test".to_string()];
        let span = Span::new(&class, None, "test content");
        assert_eq!(
            span.to_string(),
            "<span class=\"test \">test content</span>"
        );
    }
    #[test]
    fn test_span_color_5() {
        let class = ["test".to_string(), "test2".to_string()];
        let span = Span::new(&class, Some("motd-style-5"), "test content");
        assert_eq!(
            span.to_string(),
            "<span class=\"motd-style-5 test test2 \">test content</span>"
        );
    }
    #[test]
    fn test_span_empty_class() {
        let class = [];
        let span = Span::new(&class, Some("motd-style-5"), "test content");
        assert_eq!(
            span.to_string(),
            "<span class=\"motd-style-5 \">test content</span>"
        );
    }
    #[test]
    fn test_span_empty_both() {
        let class = [];
        let span = Span::new(&class, None, "test content");
        assert_eq!(span.to_string(), "<span class=\"\">test content</span>");
    }
    #[test]
    fn test_span_escaping() {
        let class = [];
        let span = Span::new(&class, None, "<script>alert(\"bad\");</script>");
        assert_eq!(
            span.to_string(),
            "<span class=\"\">&lt;script&gt;alert(&quot;bad&quot;);&lt;/script&gt;</span>"
        );
    }
    #[test]
    fn test_colorize_none() {
        let input = "No color codes";
        assert_eq!(
            mojang_colorize(input).unwrap(),
            "<span class=\"\">No color codes</span>"
        );
    }
    #[test]
    fn test_colorize_one_color() {
        let input = "§acolor a";
        assert_eq!(
            mojang_colorize(input).unwrap(),
            "<span class=\"motd-style-a \">color a</span>"
        );
    }
    #[test]
    fn test_colorize_color_immediate_change() {
        let input = "§a§bcolor b";
        assert_eq!(
            mojang_colorize(input).unwrap(),
            "<span class=\"motd-style-b \">color b</span>"
        );
    }
    #[test]
    fn test_colorize_color_reset() {
        let input = "§acolor a§rblank§bcolor b";
        assert_eq!(
            mojang_colorize(input).unwrap(),
            r#"<span class="motd-style-a ">color a</span><span class="">blank</span><span class="motd-style-b ">color b</span>"#
        );
    }
    #[test]
    fn test_colorize_additive() {
        let input = "§a§nunderlined";
        assert_eq!(
            mojang_colorize(input).unwrap(),
            r#"<span class="motd-style-a motd-style-n ">underlined</span>"#
        );
    }
}
