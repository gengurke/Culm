//! HTML (provider descriptions) to plain text: paragraphs, headings and list items become lines,
//! everything else (tags, images, scripts, styles) is dropped. Never interpreted, only stripped.

/// Longest provider description shown (characters of plain text).
pub const DESCRIPTION_MAX: usize = 8000;

/// Plain text of an HTML fragment, at most `max` characters (cut at a line end, then "…").
pub fn to_text(html: &str, max: usize) -> String {
    let mut out = String::with_capacity(html.len().min(max + 64));
    let mut rest = html;
    while let Some(lt) = rest.find('<') {
        push_text(&mut out, &rest[..lt]);
        let after = &rest[lt + 1..];
        // A stray `<` that starts no tag ("a < b") is text.
        if !after.starts_with(|c: char| c.is_ascii_alphabetic() || c == '/' || c == '!') {
            push_text(&mut out, "<");
            rest = after;
            continue;
        }
        let Some(gt) = after.find('>') else {
            rest = "";
            break;
        };
        let tag = &after[..gt];
        rest = &after[gt + 1..];
        let name: String = tag
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        let closing = tag.starts_with('/');
        match name.as_str() {
            // Their content is not text: skip to the closing tag.
            "script" | "style" | "head" | "title" if !closing => {
                let end = format!("</{name}");
                rest = find_ci(rest, &end)
                    .map(|i| &rest[i..])
                    .and_then(|r| r.find('>').map(|j| &r[j + 1..]))
                    .unwrap_or("");
            }
            "br" => out.push('\n'),
            "li" if !closing => {
                line_break(&mut out);
                out.push_str("• ");
            }
            "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "table"
            | "tr" | "blockquote" | "pre" | "hr" | "section" | "article" => paragraph(&mut out),
            "li" | "td" | "th" => line_break(&mut out),
            _ => {}
        }
        if out.len() > max * 2 {
            break;
        }
    }
    push_text(&mut out, rest);
    tidy(&out, max)
}

/// Plain text of a Markdown README: headings, paragraphs and list items as lines; images, badges,
/// HTML and link targets dropped (link text kept); at most `max` characters.
pub fn markdown_to_text(md: &str, max: usize) -> String {
    let mut out = String::new();
    let mut fenced = false;
    for raw in md.lines() {
        let line = raw.trim();
        if line.starts_with("```") || line.starts_with("~~~") {
            fenced = !fenced;
            paragraph(&mut out);
            continue;
        }
        if fenced {
            line_break(&mut out);
            out.push_str(raw.trim_end());
            out.push('\n');
            continue;
        }
        let heading = line.starts_with('#');
        let (bullet, body) = match line.split_once(' ') {
            Some(("-" | "*" | "+", rest)) => (true, rest),
            _ if heading => (false, line.trim_start_matches('#').trim()),
            _ => (false, line),
        };
        let text = inline_markdown(body);
        if text.is_empty() {
            // A blank line, or one that was only badges / images / HTML.
            if line.is_empty() {
                paragraph(&mut out);
            }
            continue;
        }
        if heading {
            paragraph(&mut out);
        } else if bullet {
            line_break(&mut out);
            out.push_str("• ");
        } else if !out.is_empty() && !out.ends_with('\n') && !out.ends_with(' ') {
            out.push(' ');
        }
        out.push_str(&text);
        if heading {
            paragraph(&mut out);
        }
    }
    tidy(&out, max)
}

/// One Markdown line's text: images and HTML dropped, `[text](url)` → text, emphasis marks off.
fn inline_markdown(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix("![") {
            // An image: skip `alt](target)`.
            rest = r
                .find("](")
                .and_then(|i| r[i..].find(')').map(|j| &r[i + j + 1..]))
                .unwrap_or("");
        } else if let Some(r) = rest.strip_prefix('[')
            && let Some(close) = closing_bracket(r)
            && r[close..].starts_with("](")
            && let Some(end) = r[close..].find(')')
        {
            out.push_str(&inline_markdown(&r[..close]));
            rest = &r[close + end + 1..];
        } else if rest.starts_with('<')
            && rest[1..].starts_with(|c: char| c.is_ascii_alphabetic() || c == '/' || c == '!')
        {
            rest = rest.find('>').map_or("", |i| &rest[i + 1..]);
        } else {
            let c = rest.chars().next().unwrap_or(' ');
            if !matches!(c, '*' | '`') && !rest.starts_with("__") {
                out.push(c);
            }
            rest = &rest[c.len_utf8()..];
        }
    }
    decode_entities(out.trim()).replace('\u{a0}', " ")
}

/// Index of the `]` closing an already opened `[` (nested brackets skipped).
fn closing_bracket(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '[' => depth += 1,
            ']' if depth == 0 => return Some(i),
            ']' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// Appends text with entities decoded and runs of whitespace as one space.
fn push_text(out: &mut String, raw: &str) {
    let decoded = decode_entities(raw);
    for c in decoded.chars() {
        if c.is_whitespace() && c != '\u{a0}' {
            if !out.ends_with([' ', '\n']) && !out.is_empty() {
                out.push(' ');
            }
        } else if c == '\u{a0}' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
}

fn line_break(out: &mut String) {
    trim_end_spaces(out);
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
}

fn paragraph(out: &mut String) {
    trim_end_spaces(out);
    if out.is_empty() || out.ends_with("\n\n") {
        return;
    }
    out.push_str(if out.ends_with('\n') { "\n" } else { "\n\n" });
}

fn trim_end_spaces(out: &mut String) {
    while out.ends_with(' ') {
        out.pop();
    }
}

/// Trims every line, drops blank-line runs and lines that are only bullets, caps the length.
fn tidy(s: &str, max: usize) -> String {
    let mut lines: Vec<&str> = Vec::new();
    for line in s.lines().map(str::trim) {
        if line == "•" {
            continue;
        }
        if line.is_empty() && lines.last().is_none_or(|l| l.is_empty()) {
            continue;
        }
        lines.push(line);
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    let text = lines.join("\n");
    if text.chars().count() <= max {
        return text;
    }
    let cut: String = text.chars().take(max).collect();
    let cut = cut.rfind('\n').map_or(cut.as_str(), |i| &cut[..i]);
    format!("{}\n…", cut.trim_end())
}

fn find_ci(hay: &str, needle: &str) -> Option<usize> {
    hay.to_ascii_lowercase().find(needle)
}

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest[1..].find(';').filter(|&i| i <= 10).and_then(|i| {
            let ent = &rest[1..1 + i];
            let c = match ent {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{a0}'),
                "ndash" => Some('–'),
                "mdash" => Some('—'),
                "hellip" => Some('…'),
                "rsquo" | "lsquo" => Some('\''),
                "rdquo" | "ldquo" => Some('"'),
                _ => ent
                    .strip_prefix("#x")
                    .or_else(|| ent.strip_prefix("#X"))
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .or_else(|| ent.strip_prefix('#').and_then(|d| d.parse().ok()))
                    .and_then(char::from_u32)
                    .filter(|c| !c.is_control() || *c == '\n'),
            };
            c.map(|c| (c, i + 2))
        });
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::to_text;

    #[test]
    fn strips_tags_and_keeps_structure() {
        let html = "<h2>&nbsp;</h2>\n<p><img src=\"x.png\"> <em>Most  powerful</em> meter &amp; more.</p>\
            <script>alert(1)</script><ul><li>One</li><li>Two&#33;</li></ul><p>a < b<br>next</p>";
        assert_eq!(
            to_text(html, 1000),
            "Most powerful meter & more.\n\n• One\n• Two!\n\na < b\nnext"
        );
    }

    #[test]
    fn markdown_keeps_text_and_drops_badges() {
        let md = "# Questie\n[![Build](https://x/b.svg)](https://x) <img src=\"a.png\">\n\nThe **best** quest\nhelper, see [the wiki](https://w).\n\n## Features\n- Shows `quests`\n* Tracks &amp; more\n";
        assert_eq!(
            super::markdown_to_text(md, 1000),
            "Questie\n\nThe best quest helper, see the wiki.\n\nFeatures\n\n• Shows quests\n• Tracks & more"
        );
    }

    #[test]
    fn caps_length_at_a_line_end() {
        let html = "<p>first line</p><p>second line that is long</p>";
        assert_eq!(to_text(html, 20), "first line\n…");
    }
}
