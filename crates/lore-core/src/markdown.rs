//! Markdown, resolved to a tree the window can render directly.
//!
//! Parsing happens here rather than in the window for two reasons.
//!
//! The first is safety. Rendering markdown in a browser usually means producing
//! HTML and injecting it, which in this app would mean injecting archived text
//! into a webview that holds `window.__TAURI_INTERNALS__` — and archived text is
//! full of `<script>` from other people's code samples. A tree of typed nodes has
//! no such edge: the window renders each node as a Svelte component, and Svelte
//! escapes text by construction.
//!
//! The second is cost. A long session is eight hundred turns, and the window has
//! one thread for everything. This runs inside a query that already takes tens of
//! milliseconds.
//!
//! The subset is chosen from what actually appears in this machine's archive:
//! inline code in 60% of assistant messages, bold in 51%, lists in 37%, fences in
//! 19%, headings in 18%, tables in 5% — and links in none, though they cost
//! almost nothing to carry so they are carried.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::Serialize;

/// A block-level node. Field names are short because every one of these is
/// serialized across the IPC boundary for every turn of a session.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "b", rename_all = "lowercase")]
pub enum Block {
    #[serde(rename = "p")]
    Paragraph { spans: Vec<Span> },
    #[serde(rename = "h")]
    Heading { level: u8, spans: Vec<Span> },
    /// A fenced or indented block. `lang` is whatever the fence declared.
    #[serde(rename = "code")]
    Code {
        #[serde(skip_serializing_if = "Option::is_none")]
        lang: Option<String>,
        text: String,
    },
    #[serde(rename = "list")]
    List {
        ordered: bool,
        /// Where an ordered list begins. Carried because a list written from 3
        /// renumbered itself to 1, which changes what the message said.
        #[serde(skip_serializing_if = "Option::is_none")]
        start: Option<u64>,
        items: Vec<Vec<Block>>,
    },
    #[serde(rename = "quote")]
    Quote { blocks: Vec<Block> },
    #[serde(rename = "table")]
    Table {
        head: Vec<Vec<Span>>,
        rows: Vec<Vec<Vec<Span>>>,
    },
    #[serde(rename = "rule")]
    Rule,
}

/// An inline node.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "s", rename_all = "lowercase")]
pub enum Span {
    #[serde(rename = "t")]
    Text { text: String },
    #[serde(rename = "c")]
    Code { text: String },
    #[serde(rename = "b")]
    Strong { spans: Vec<Span> },
    #[serde(rename = "i")]
    Em { spans: Vec<Span> },
    #[serde(rename = "a")]
    Link { href: String, spans: Vec<Span> },
}

/// Parses a turn's text.
///
/// Archived text is often a head of a longer original, so a fenced block can be
/// left open by the cut. `pulldown-cmark` closes it at the end of input, which is
/// the behaviour that keeps the rest of the turn readable rather than swallowing
/// it into a code block that never ends.
pub fn parse(text: &str) -> Vec<Block> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let mut blocks = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut span_stack: Vec<(SpanKind, Vec<Span>)> = Vec::new();
    let mut code: Option<(Option<String>, String)> = None;
    // A table's cells arrive as a flat run of events; these hold the row being
    // built and whether it is still the header.
    let mut table: Option<(Vec<Vec<Span>>, Vec<Vec<Vec<Span>>>, bool)> = None;
    let mut row: Vec<Vec<Span>> = Vec::new();

    for event in Parser::new_ext(text, options) {
        match event {
            // Both are pushed so the matching end pops this frame rather than a
            // list item or quote that encloses it.
            Event::Start(Tag::Paragraph) => {
                spans.clear();
                stack.push(Frame::Paragraph);
            }
            Event::Start(Tag::Heading { .. }) => {
                spans.clear();
                stack.push(Frame::Heading);
            }
            Event::End(TagEnd::Paragraph) => {
                let taken = std::mem::take(&mut spans);
                stack.pop();
                if !taken.is_empty() {
                    push(&mut blocks, &mut stack, Block::Paragraph { spans: taken });
                }
            }
            Event::End(TagEnd::Heading(level)) => {
                let taken = std::mem::take(&mut spans);
                stack.pop();
                push(
                    &mut blocks,
                    &mut stack,
                    Block::Heading {
                        level: heading_level(level),
                        spans: taken,
                    },
                );
            }

            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => {
                        let info = info.split_whitespace().next().unwrap_or("").to_string();
                        (!info.is_empty()).then_some(info)
                    }
                    CodeBlockKind::Indented => None,
                };
                code = Some((lang, String::new()));
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((lang, text)) = code.take() {
                    push(
                        &mut blocks,
                        &mut stack,
                        Block::Code {
                            lang,
                            text: text.trim_end_matches('\n').to_string(),
                        },
                    );
                }
            }

            Event::Start(Tag::List(first)) => stack.push(Frame::List {
                ordered: first.is_some(),
                start: first.filter(|n| *n != 1),
                items: Vec::new(),
            }),
            Event::End(TagEnd::List(_)) => {
                if let Some(Frame::List { ordered, start, items }) = stack.pop() {
                    push(
                        &mut blocks,
                        &mut stack,
                        Block::List { ordered, start, items },
                    );
                }
            }
            Event::Start(Tag::Item) => stack.push(Frame::Item(Vec::new())),
            Event::End(TagEnd::Item) => {
                if let Some(Frame::Item(mut inner)) = stack.pop() {
                    // A tight list puts its text directly in the item, with no
                    // paragraph event to close; anything pending belongs here.
                    let loose = std::mem::take(&mut spans);
                    if !loose.is_empty() {
                        inner.push(Block::Paragraph { spans: loose });
                    }
                    if let Some(Frame::List { items, .. }) = stack.last_mut() {
                        items.push(inner);
                    }
                }
            }

            Event::Start(Tag::BlockQuote(_)) => stack.push(Frame::Quote(Vec::new())),
            Event::End(TagEnd::BlockQuote(_)) => {
                if let Some(Frame::Quote(inner)) = stack.pop() {
                    push(&mut blocks, &mut stack, Block::Quote { blocks: inner });
                }
            }

            Event::Start(Tag::Table(_)) => table = Some((Vec::new(), Vec::new(), true)),
            Event::End(TagEnd::Table) => {
                if let Some((head, rows, _)) = table.take() {
                    push(&mut blocks, &mut stack, Block::Table { head, rows });
                }
            }
            Event::End(TagEnd::TableHead) => {
                if let Some((head, _, heading)) = table.as_mut() {
                    *head = std::mem::take(&mut row);
                    *heading = false;
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Some((_, rows, _)) = table.as_mut() {
                    rows.push(std::mem::take(&mut row));
                }
            }
            Event::End(TagEnd::TableCell) => row.push(std::mem::take(&mut spans)),

            Event::Start(Tag::Strong) => span_stack.push((SpanKind::Strong, std::mem::take(&mut spans))),
            Event::End(TagEnd::Strong) => close_span(&mut spans, &mut span_stack, SpanKind::Strong),
            Event::Start(Tag::Emphasis) => span_stack.push((SpanKind::Em, std::mem::take(&mut spans))),
            Event::End(TagEnd::Emphasis) => close_span(&mut spans, &mut span_stack, SpanKind::Em),
            Event::Start(Tag::Link { dest_url, .. }) => {
                span_stack.push((SpanKind::Link(dest_url.to_string()), std::mem::take(&mut spans)))
            }
            Event::End(TagEnd::Link) => {
                let kind = span_stack
                    .last()
                    .map(|(k, _)| k.clone())
                    .unwrap_or(SpanKind::Strong);
                close_span(&mut spans, &mut span_stack, kind);
            }

            Event::Text(t) => match code.as_mut() {
                Some((_, buffer)) => buffer.push_str(&t),
                None => spans.push(Span::Text { text: t.to_string() }),
            },
            Event::Code(t) => spans.push(Span::Code { text: t.to_string() }),
            // Kept as a newline rather than collapsed to a space. Markdown would
            // fold it, but these turns are typed messages, not authored documents:
            // 180 prompts in this archive put their lines apart deliberately, and
            // running them together changes what they say.
            Event::SoftBreak => spans.push(Span::Text { text: "\n".into() }),
            Event::HardBreak => spans.push(Span::Text { text: "\n".into() }),
            Event::Rule => push(&mut blocks, &mut stack, Block::Rule),

            // Raw HTML is archived text, not markup to honour. It is shown as the
            // characters it is, which is also what makes injection impossible.
            Event::Html(t) | Event::InlineHtml(t) => {
                spans.push(Span::Text { text: t.to_string() })
            }
            _ => {}
        }
    }

    // Anything still open was cut off mid-structure by truncation.
    if let Some((lang, text)) = code.take() {
        blocks.push(Block::Code {
            lang,
            text: text.trim_end_matches('\n').to_string(),
        });
    }
    if !spans.is_empty() {
        blocks.push(Block::Paragraph { spans });
    }
    blocks
}

enum Frame {
    Paragraph,
    Heading,
    List {
        ordered: bool,
        start: Option<u64>,
        items: Vec<Vec<Block>>,
    },
    Item(Vec<Block>),
    Quote(Vec<Block>),
}

#[derive(Clone, PartialEq)]
enum SpanKind {
    Strong,
    Em,
    Link(String),
}

/// Places a finished block inside whatever is open around it.
fn push(blocks: &mut Vec<Block>, stack: &mut [Frame], block: Block) {
    for frame in stack.iter_mut().rev() {
        match frame {
            Frame::Item(inner) | Frame::Quote(inner) => {
                inner.push(block);
                return;
            }
            _ => continue,
        }
    }
    blocks.push(block);
}

fn close_span(spans: &mut Vec<Span>, stack: &mut Vec<(SpanKind, Vec<Span>)>, kind: SpanKind) {
    let Some((_, mut outer)) = stack.pop() else {
        return;
    };
    let inner = std::mem::take(spans);
    outer.push(match kind {
        SpanKind::Strong => Span::Strong { spans: inner },
        SpanKind::Em => Span::Em { spans: inner },
        SpanKind::Link(href) => Span::Link { href, spans: inner },
    });
    *spans = outer;
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Span {
        Span::Text { text: s.into() }
    }

    #[test]
    fn the_common_shapes_parse() {
        let blocks = parse("## Findings\n\nSome **bold** and `code` here.\n\n- first\n- second\n");
        assert_eq!(
            blocks,
            vec![
                Block::Heading { level: 2, spans: vec![text("Findings")] },
                Block::Paragraph {
                    spans: vec![
                        text("Some "),
                        Span::Strong { spans: vec![text("bold")] },
                        text(" and "),
                        Span::Code { text: "code".into() },
                        text(" here."),
                    ]
                },
                Block::List {
                    ordered: false,
                    start: None,
                    items: vec![
                        vec![Block::Paragraph { spans: vec![text("first")] }],
                        vec![Block::Paragraph { spans: vec![text("second")] }],
                    ]
                },
            ]
        );
    }

    #[test]
    fn a_fence_keeps_its_language_and_body() {
        let blocks = parse("```rust\nfn main() {}\n```\n");
        assert_eq!(
            blocks,
            vec![Block::Code {
                lang: Some("rust".into()),
                text: "fn main() {}".into()
            }]
        );
    }

    /// Archived text is a head of the original, so a fence can be cut mid-block.
    /// The rest of the turn must not vanish into a code block that never closes.
    #[test]
    fn a_fence_cut_by_truncation_still_closes() {
        let blocks = parse("before\n\n```sh\nlore scan\nlore rebui");
        assert_eq!(
            blocks,
            vec![
                Block::Paragraph { spans: vec![text("before")] },
                Block::Code {
                    lang: Some("sh".into()),
                    text: "lore scan\nlore rebui".into()
                },
            ]
        );
    }

    /// The reason this runs in Rust: archived text carries other people's code
    /// samples, and none of it may become markup in a webview holding the app's
    /// command bridge.
    #[test]
    fn html_in_archived_text_stays_text() {
        let blocks = parse("<script>alert(1)</script>");
        let rendered = format!("{blocks:?}");
        assert!(rendered.contains("script"), "the characters are kept");
        assert!(
            !rendered.contains("Html"),
            "but never as markup: {rendered}"
        );
    }

    #[test]
    fn tables_keep_their_header_and_rows() {
        let blocks = parse("| kind | n |\n|---|---|\n| scan | 3 |\n");
        assert_eq!(
            blocks,
            vec![Block::Table {
                head: vec![vec![text("kind")], vec![text("n")]],
                rows: vec![vec![vec![text("scan")], vec![text("3")]]],
            }]
        );
    }

    #[test]
    fn nested_lists_keep_their_nesting() {
        let blocks = parse("- outer\n  - inner\n");
        let Block::List { items, .. } = &blocks[0] else {
            panic!("expected a list, got {blocks:?}");
        };
        assert_eq!(items.len(), 1, "one outer item");
        assert!(
            items[0].iter().any(|b| matches!(b, Block::List { .. })),
            "which contains the inner list: {:?}",
            items[0]
        );
    }

    /// A typed message is not an authored document. Markdown folds a single
    /// newline into a space, which turns a prompt written across lines into one
    /// run-on — and 180 prompts in this archive are written that way.
    #[test]
    fn a_typed_newline_survives() {
        assert_eq!(
            parse("what about the agent?\nand the timer?"),
            vec![Block::Paragraph {
                spans: vec![
                    text("what about the agent?"),
                    text("\n"),
                    text("and the timer?"),
                ]
            }]
        );
    }

    /// Found by the test above: a list written from 3 was renumbering itself to 1.
    #[test]
    fn an_ordered_list_keeps_where_it_started() {
        let Block::List { ordered, start, items } = &parse("3. third\n4. fourth")[0] else {
            panic!("expected a list");
        };
        assert!(ordered);
        assert_eq!(*start, Some(3), "the numbers the message used");
        assert_eq!(items.len(), 2);

        let Block::List { start, .. } = &parse("1. first\n2. second")[0] else {
            panic!("expected a list");
        };
        assert_eq!(*start, None, "a list from 1 needs no attribute");
    }

    #[test]
    fn plain_prose_is_one_paragraph() {
        assert_eq!(
            parse("just a sentence"),
            vec![Block::Paragraph { spans: vec![text("just a sentence")] }]
        );
    }
}
