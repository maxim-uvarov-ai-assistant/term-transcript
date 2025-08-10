use std::io::Write;

use termcolor::{Ansi, Color, ColorSpec, WriteColor};

use super::*;

fn prepare_term_output() -> anyhow::Result<String> {
    let mut writer = Ansi::new(vec![]);
    writer.set_color(
        ColorSpec::new()
            .set_fg(Some(Color::Cyan))
            .set_underline(true),
    )?;
    write!(writer, "Hello")?;
    writer.reset()?;
    write!(writer, ", ")?;
    writer.set_color(
        ColorSpec::new()
            .set_fg(Some(Color::White))
            .set_bg(Some(Color::Green))
            .set_intense(true),
    )?;
    write!(writer, "world")?;
    writer.reset()?;
    write!(writer, "!")?;

    String::from_utf8(writer.into_inner()).map_err(From::from)
}

#[test]
fn converting_captured_output_to_text() -> anyhow::Result<()> {
    let output = Captured(prepare_term_output()?);
    assert_eq!(output.to_plaintext()?, "Hello, world!");
    Ok(())
}

#[test]
fn converting_captured_output_to_html() -> anyhow::Result<()> {
    const EXPECTED_HTML: &str = "<span class=\"underline fg6\">Hello</span>, \
        <span class=\"fg15 bg10\">world</span>!";

    let output = Captured(prepare_term_output()?);
    assert_eq!(output.to_html()?, EXPECTED_HTML);
    Ok(())
}

#[test]
fn html_closes_span_when_output_ends_with_color() -> anyhow::Result<()> {
    // This test reproduces the bug where terminal output ending with colored text
    // would not have its final </span> tag closed in HTML generation
    let mut writer = Ansi::new(vec![]);
    
    // Write some colored text that ends without a reset
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    write!(writer, "error")?;
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
    write!(writer, ": ")?;
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
    write!(writer, "file not found")?;
    // Note: No reset() call here - the output ends with color active
    
    let ansi_output = writer.into_inner();
    let output = Captured(String::from_utf8(ansi_output)?);
    let html = output.to_html()?;
    
    // Verify the HTML has properly balanced tags
    let open_spans = html.matches("<span").count();
    let close_spans = html.matches("</span>").count();
    assert_eq!(
        open_spans, close_spans,
        "HTML should have equal number of opening and closing span tags. \
         HTML output: {}", html
    );
    
    // The expected HTML should have all spans properly closed
    const EXPECTED_HTML: &str = "<span class=\"fg1\">error</span>\
        <span class=\"fg4\">: </span>\
        <span class=\"fg2\">file not found</span>";
    assert_eq!(html, EXPECTED_HTML);
    
    Ok(())
}

#[test]
#[cfg(feature = "svg")]
fn svg_generation_with_colored_output_at_end() -> anyhow::Result<()> {
    // This test verifies that SVG generation works correctly when output ends with color
    use crate::{svg::Template, Transcript, UserInput};
    
    let mut writer = Ansi::new(vec![]);
    
    // Create output that ends with colored text (no reset at the end)
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
    write!(writer, "Success")?;
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
    write!(writer, "!")?;
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    write!(writer, " Error")?;
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
    write!(writer, ": ")?;
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
    write!(writer, "Failed")?;
    // Intentionally no reset() here - ends with color
    
    let output = String::from_utf8(writer.into_inner())?;
    
    // Create a transcript with this output
    let mut transcript = Transcript::new();
    transcript.add_interaction(UserInput::command("test"), output);
    
    // Render to SVG
    let mut svg_output = Vec::new();
    Template::default().render(&transcript, &mut svg_output)?;
    let svg_string = String::from_utf8(svg_output)?;
    
    // Verify the SVG is well-formed XML (would fail with unclosed tags)
    assert!(svg_string.contains("<svg"), "Should contain SVG opening tag");
    assert!(svg_string.contains("</svg>"), "Should contain SVG closing tag");
    
    // Count that all HTML tags are properly balanced in the foreignObject section
    if let Some(foreign_start) = svg_string.find("<foreignObject") {
        if let Some(foreign_end) = svg_string.find("</foreignObject>") {
            let foreign_content = &svg_string[foreign_start..foreign_end];
            
            let pre_open = foreign_content.matches("<pre").count();
            let pre_close = foreign_content.matches("</pre>").count();
            assert_eq!(pre_open, pre_close, "Pre tags should be balanced");
            
            let div_open = foreign_content.matches("<div").count();
            let div_close = foreign_content.matches("</div>").count();
            assert_eq!(div_open, div_close, "Div tags should be balanced");
            
            let span_open = foreign_content.matches("<span").count();
            let span_close = foreign_content.matches("</span>").count();
            assert_eq!(
                span_open, span_close,
                "Span tags should be balanced. Open: {}, Close: {}", 
                span_open, span_close
            );
        }
    }
    
    Ok(())
}

fn assert_eq_term_output(actual: &[u8], expected: &[u8]) {
    assert_eq!(
        String::from_utf8_lossy(actual),
        String::from_utf8_lossy(expected)
    );
}

#[test]
fn term_roundtrip_simple() -> anyhow::Result<()> {
    let mut writer = Ansi::new(vec![]);
    write!(writer, "Hello, ")?;
    writer.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Green)))?;
    write!(writer, "world")?;
    writer.reset()?;
    write!(writer, "!")?;

    let term_output = writer.into_inner();

    let mut new_writer = Ansi::new(vec![]);
    TermOutputParser::new(&mut new_writer).parse(&term_output)?;
    let new_term_output = new_writer.into_inner();
    assert_eq_term_output(&new_term_output, &term_output);
    Ok(())
}

#[test]
fn term_roundtrip_with_multiple_colors() -> anyhow::Result<()> {
    let mut writer = Ansi::new(vec![]);
    write!(writer, "He")?;
    writer.set_color(
        ColorSpec::new()
            .set_bg(Some(Color::White))
            .set_fg(Some(Color::Black)),
    )?;
    write!(writer, "ll")?;
    writer.set_color(
        ColorSpec::new()
            .set_intense(true)
            .set_fg(Some(Color::Magenta)),
    )?;
    write!(writer, "o")?;
    writer.set_color(
        ColorSpec::new()
            .set_italic(true)
            .set_fg(Some(Color::Green))
            .set_bg(Some(Color::Yellow)),
    )?;
    write!(writer, "world")?;
    writer.set_color(
        ColorSpec::new()
            .set_underline(true)
            .set_dimmed(true)
            .set_bg(Some(Color::Cyan)),
    )?;
    write!(writer, "!")?;

    let term_output = writer.into_inner();

    let mut new_writer = Ansi::new(vec![]);
    TermOutputParser::new(&mut new_writer).parse(&term_output)?;
    let new_term_output = new_writer.into_inner();
    assert_eq_term_output(&new_term_output, &term_output);
    Ok(())
}

#[test]
fn roundtrip_with_indexed_colors() -> anyhow::Result<()> {
    let mut writer = Ansi::new(vec![]);
    write!(writer, "H")?;
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Ansi256(5))))?;
    write!(writer, "e")?;
    writer.set_color(ColorSpec::new().set_bg(Some(Color::Ansi256(11))))?;
    write!(writer, "l")?;
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Ansi256(33))))?;
    write!(writer, "l")?;
    writer.set_color(ColorSpec::new().set_bg(Some(Color::Ansi256(250))))?;
    write!(writer, "o")?;

    let term_output = writer.into_inner();

    let mut new_writer = Ansi::new(vec![]);
    TermOutputParser::new(&mut new_writer).parse(&term_output)?;
    let new_term_output = new_writer.into_inner();
    assert_eq_term_output(&new_term_output, &term_output);
    Ok(())
}

#[test]
fn roundtrip_with_rgb_colors() -> anyhow::Result<()> {
    let mut writer = Ansi::new(vec![]);
    write!(writer, "H")?;
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(16, 22, 35))))?;
    write!(writer, "e")?;
    writer.set_color(ColorSpec::new().set_bg(Some(Color::Rgb(255, 254, 253))))?;
    write!(writer, "l")?;
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(0, 0, 0))))?;
    write!(writer, "l")?;
    writer.set_color(ColorSpec::new().set_bg(Some(Color::Rgb(0, 160, 128))))?;
    write!(writer, "o")?;

    let term_output = writer.into_inner();

    let mut new_writer = Ansi::new(vec![]);
    TermOutputParser::new(&mut new_writer).parse(&term_output)?;
    let new_term_output = new_writer.into_inner();
    assert_eq_term_output(&new_term_output, &term_output);
    Ok(())
}

#[test]
fn skipping_ocs_sequence_with_bell_terminator() -> anyhow::Result<()> {
    let term_output = "\u{1b}]0;C:\\WINDOWS\\system32\\cmd.EXE\u{7}echo foo";

    let mut writer = Ansi::new(vec![]);
    TermOutputParser::new(&mut writer).parse(term_output.as_bytes())?;
    let rendered_output = writer.into_inner();

    assert_eq!(String::from_utf8(rendered_output)?, "echo foo");
    Ok(())
}

#[test]
fn skipping_ocs_sequence_with_st_terminator() -> anyhow::Result<()> {
    let term_output = "\u{1b}]0;C:\\WINDOWS\\system32\\cmd.EXE\u{1b}\\echo foo";

    let mut writer = Ansi::new(vec![]);
    TermOutputParser::new(&mut writer).parse(term_output.as_bytes())?;
    let rendered_output = writer.into_inner();

    assert_eq!(String::from_utf8(rendered_output)?, "echo foo");
    Ok(())
}

#[test]
fn skipping_non_color_csi_sequence() -> anyhow::Result<()> {
    let term_output = "\u{1b}[49Xecho foo";

    let mut writer = Ansi::new(vec![]);
    TermOutputParser::new(&mut writer).parse(term_output.as_bytes())?;
    let rendered_output = writer.into_inner();

    assert_eq!(String::from_utf8(rendered_output)?, "echo foo");
    Ok(())
}

#[test]
fn implicit_reset_sequence() -> anyhow::Result<()> {
    let term_output = "\u{1b}[34mblue\u{1b}[m";

    let mut writer = Ansi::new(vec![]);
    TermOutputParser::new(&mut writer).parse(term_output.as_bytes())?;
    let rendered_output = writer.into_inner();

    assert_eq!(
        String::from_utf8(rendered_output)?,
        "\u{1b}[0m\u{1b}[34mblue\u{1b}[0m"
    );
    Ok(())
}

#[test]
fn intense_color() -> anyhow::Result<()> {
    let term_output = "\u{1b}[94mblue\u{1b}[m";

    let mut writer = Ansi::new(vec![]);
    TermOutputParser::new(&mut writer).parse(term_output.as_bytes())?;
    let rendered_output = writer.into_inner();

    assert_eq!(
        String::from_utf8(rendered_output)?,
        "\u{1b}[0m\u{1b}[38;5;12mblue\u{1b}[0m"
    );
    Ok(())
}

#[test]
fn carriage_return_at_middle_of_line() -> anyhow::Result<()> {
    let term_output = "\u{1b}[32mgreen\u{1b}[m\r\u{1b}[34mblue\u{1b}[m";

    let mut writer = Ansi::new(vec![]);
    TermOutputParser::new(&mut writer).parse(term_output.as_bytes())?;
    let rendered_output = writer.into_inner();

    assert_eq!(
        String::from_utf8(rendered_output)?,
        "\u{1b}[0m\u{1b}[34mblue\u{1b}[0m"
    );
    Ok(())
}
