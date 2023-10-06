use crate::Span;
use alloc::{format, string::String};
use core::fmt;

#[derive(Debug)]
struct Pos {
    line: usize,
    col: usize,
}
#[derive(Debug)]
struct PosSpan {
    line: usize,
    col_start: usize,
    col_end: usize,
}

/// Formatter options for [Span](crate::Span).
pub struct FormatOption<SpanFormatter, MarkerFormatter, NumberFormatter> {
    pub span_formatter: SpanFormatter,
    pub marker_formatter: MarkerFormatter,
    pub number_formatter: NumberFormatter,
}

type FmtPtr<Writer> = fn(&str, &mut Writer) -> fmt::Result;
impl<Writer: fmt::Write> Default for FormatOption<FmtPtr<Writer>, FmtPtr<Writer>, FmtPtr<Writer>> {
    fn default() -> Self {
        Self {
            span_formatter: |s, f| write!(f, "{s}"),
            marker_formatter: |m, f| write!(f, "{m}"),
            number_formatter: |n, f| write!(f, "{n}"),
        }
    }
}

impl<SF, MF, NF> FormatOption<SF, MF, NF> {
    /// Create option with given functions.
    pub fn new<Writer>(span_formatter: SF, marker_formatter: MF, number_formatter: NF) -> Self
    where
        Writer: fmt::Write,
        SF: FnMut(&str, &mut Writer) -> fmt::Result,
        MF: FnMut(&str, &mut Writer) -> fmt::Result,
        NF: FnMut(&str, &mut Writer) -> fmt::Result,
    {
        Self {
            span_formatter,
            marker_formatter,
            number_formatter,
        }
    }
    fn visualize_white_space(line: &str) -> String {
        // \r ␍
        // \n ␊
        line.replace('\n', "␊").replace('\r', "␍")
    }
    fn display_snippet_single_line<Writer>(
        mut self,
        f: &mut Writer,
        index_digit: usize,
        line: (&str, PosSpan),
    ) -> fmt::Result
    where
        Writer: fmt::Write,
        SF: FnMut(&str, &mut Writer) -> fmt::Result,
        MF: FnMut(&str, &mut Writer) -> fmt::Result,
        NF: FnMut(&str, &mut Writer) -> fmt::Result,
    {
        let spacing = " ".repeat(index_digit);
        write!(f, "{} ", spacing)?;
        (self.number_formatter)("|", f)?;
        writeln!(f)?;

        let number = format!("{:w$}", line.1.line + 1, w = index_digit);
        (self.number_formatter)(&number, f)?;
        write!(f, " ")?;
        (self.number_formatter)("|", f)?;
        write!(f, " {}", &line.0[..line.1.col_start],)?;
        (self.span_formatter)(&line.0[line.1.col_start..line.1.col_end], f)?;
        write!(f, "{}", &line.0[line.1.col_end..])?;
        writeln!(f)?;

        write!(f, "{} ", spacing)?;
        (self.number_formatter)("|", f)?;
        write!(f, " {}", &line.0[..line.1.col_start])?;
        (self.marker_formatter)(&"^".repeat(line.1.col_end - line.1.col_start), f)?;
        writeln!(f)?;

        Ok(())
    }
    fn display_snippet_multi_line<Writer>(
        mut self,
        f: &mut Writer,
        index_digit: usize,
        start: (&str, Pos),
        end: (&str, Pos),
    ) -> fmt::Result
    where
        Writer: fmt::Write,
        SF: FnMut(&str, &mut Writer) -> fmt::Result,
        MF: FnMut(&str, &mut Writer) -> fmt::Result,
        NF: FnMut(&str, &mut Writer) -> fmt::Result,
    {
        let spacing = " ".repeat(index_digit);
        write!(f, "{} ", spacing)?;
        (self.number_formatter)("|", f)?;
        write!(f, " {}", &start.0[..start.1.col])?;
        (self.marker_formatter)("v", f)?;
        writeln!(f)?;

        let number = format!("{:w$}", start.1.line + 1, w = index_digit);
        (self.number_formatter)(&number, f)?;
        write!(f, " ")?;
        (self.number_formatter)("|", f)?;
        write!(f, " {}", &start.0[..start.1.col])?;
        (self.span_formatter)(&start.0[start.1.col..], f)?;
        writeln!(f)?;

        if start.1.line.abs_diff(end.1.line) > 1 {
            write!(f, "{} ", spacing)?;
            (self.number_formatter)("|", f)?;
            writeln!(f, " ...")?;
        }

        let number = format!("{:w$}", end.1.line + 1, w = index_digit);
        (self.number_formatter)(&number, f)?;
        write!(f, " ")?;
        (self.number_formatter)("|", f)?;
        write!(f, " ")?;
        (self.span_formatter)(&end.0[..end.1.col], f)?;
        writeln!(f, "{}", &end.0[end.1.col..])?;

        write!(f, "{} ", spacing)?;
        (self.number_formatter)("|", f)?;
        write!(f, " {}", &end.0[..end.1.col - 1])?;
        (self.marker_formatter)("^", f)?;
        writeln!(f)?;

        Ok(())
    }
    pub(crate) fn display_snippet<'i, Writer>(self, span: &Span<'i>, f: &mut Writer) -> fmt::Result
    where
        Writer: fmt::Write,
        SF: FnMut(&str, &mut Writer) -> fmt::Result,
        MF: FnMut(&str, &mut Writer) -> fmt::Result,
        NF: FnMut(&str, &mut Writer) -> fmt::Result,
    {
        let mut start = None;
        let mut end = None;
        let mut pos = 0usize;
        let input = Span::new(span.get_input(), 0, span.get_input().len()).unwrap();
        let mut iter = input.lines().enumerate().peekable();
        while let Some((index, line)) = iter.peek() {
            if pos + line.len() >= span.start() {
                start = Some(Pos {
                    line: index.clone(),
                    col: span.start() - pos,
                });
                break;
            }
            pos += line.len();
            iter.next();
        }
        for (index, line) in iter {
            if pos + line.len() >= span.end() {
                end = Some(Pos {
                    line: index,
                    col: span.end() - pos,
                });
                break;
            }
            pos += line.len();
        }
        let start = start.unwrap();
        let end = end.unwrap();
        let mut lines = input
            .lines()
            .skip(start.line)
            .take(end.line - start.line + 1)
            .peekable();
        let index_digit = {
            let mut digit = 1usize;
            let mut i = end.line + 1;
            while i >= 10 {
                digit += 1;
                i /= 10;
            }
            digit
        };
        if start.line == end.line {
            let cur_line = Self::visualize_white_space(lines.next().unwrap());
            let span = PosSpan {
                line: start.line,
                col_start: start.col,
                col_end: end.col,
            };
            let line = (cur_line.as_str(), span);
            self.display_snippet_single_line(f, index_digit, line)?;
        } else {
            let start_line = Self::visualize_white_space(lines.next().unwrap());
            let end_line = Self::visualize_white_space(lines.last().unwrap());
            let start = (start_line.as_str(), start);
            let end = (end_line.as_str(), end);
            self.display_snippet_multi_line(f, index_digit, start, end)?;
        }
        Ok(())
    }
}