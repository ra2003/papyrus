use crate::output::OutputChange;
use crossterm as xterm;
use mortal::Event;
use mortal::{Event::*, Key::*};
use std::io::{self, stdout, Stdout, Write};
use xterm::{Clear, ClearType, Goto, Output, QueueableCommand};

/// Terminal screen interface.
///
/// Its own struct as there is specific configuration and key handling for moving around the
/// interface.
pub struct Screen(mortal::Screen);

impl Screen {
    pub fn new() -> io::Result<Self> {
        let config = mortal::PrepareConfig {
            block_signals: true,
            ..Default::default()
        };

        Ok(Screen(mortal::Screen::new(config)?))
    }
}

pub struct InputBuffer {
    buf: Vec<char>,
    pos: usize,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            pos: 0,
        }
    }

    pub fn buffer(&self) -> String {
        self.buf.iter().collect()
    }

    /// Character index of cursor.
    pub fn ch_pos(&self) -> usize {
        self.pos
    }

    /// Number of characters.
    pub fn ch_len(&self) -> usize {
        self.buf.len()
    }

    pub fn clear(&mut self) {
        self.buf.clear();
        self.pos = 0;
    }

    pub fn insert(&mut self, ch: char) {
        self.buf.insert(self.pos, ch);
        self.pos += 1;
    }

    pub fn insert_str(&mut self, s: &str) {
        for c in s.chars() {
            self.insert(c);
        }
    }

    /// Removes from _start_ of position.
    pub fn backspace(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
            self.buf.remove(self.pos);
        }
    }

    /// Removes from _end_ of position.
    pub fn delete(&mut self) {
        if self.pos < self.buf.len() {
            self.buf.remove(self.pos);
        }
    }

    /// Return the number moved.
    pub fn move_pos_left(&mut self, n: usize) -> usize {
        let n = if self.pos < n { self.pos } else { n };
        self.pos -= n;
        n
    }

    /// Return the number moved.
    pub fn move_pos_right(&mut self, n: usize) -> usize {
        let max = self.buf.len() - self.pos;
        let n = if n > max { max } else { n };

        self.pos += n;
        n
    }

    pub fn truncate(&mut self, ch_pos: usize) {
        self.buf.truncate(ch_pos);
        if self.pos > self.buf.len() {
            self.pos = self.buf.len()
        }
    }
}

pub struct CItem {
    pub matchstr: String,
    pub input_chpos: usize,
}

#[derive(Default)]
pub struct CompletionWriter {
    input_line: String,
    completions: Vec<CItem>,
    completion_idx: usize,
    lines_to_clear: u16,
}

impl CompletionWriter {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn is_same_input(&self, line: &str) -> bool {
        self.input_line == line
    }

    pub fn next_completion(&mut self) {
        let idx = self.completion_idx + 1;
        let idx = if idx >= self.completions.len() {
            0
        } else {
            idx
        };
        self.completion_idx = idx;
    }

    pub fn new_completions<I: Iterator<Item = CItem>>(&mut self, completions: I) {
        self.completions.clear();
        for c in completions {
            self.completions.push(c)
        }
        self.completion_idx = 0;
    }

    pub fn overwrite_completion(
        &mut self,
        initial: (u16, u16),
        buf: &mut InputBuffer,
    ) -> io::Result<()> {
        let completion = self.completions.get(self.completion_idx);

        if let Some(CItem {
            matchstr,
            input_chpos,
        }) = completion
        {
            buf.truncate(*input_chpos);
            buf.insert_str(matchstr);
            let (_, y) = write_input_buffer(initial, self.lines_to_clear, &buf)?;
            self.lines_to_clear = y.saturating_sub(initial.1);
            self.input_line = buf.buffer();
        }

        Ok(())
    }
}

/// Waits for a terminal event to occur.
///
/// > Post-processes escaped input to work with WSL.
pub fn read_terminal_event(screen: &Screen) -> io::Result<Event> {
    use mortal::{Event::*, Key::*, Signal::*};

    const ESC_TIMEOUT: Option<std::time::Duration> = Some(std::time::Duration::from_millis(5));

    let screen = &screen.0;

    let ev = screen.read_event(None)?.unwrap_or(NoEvent);

    let res = match ev {
        Key(Escape) => {
            // The escape key can be prelude to arrow keys
            // To handle this we need to get the next _two_
            // events, but they should be fast in coming
            // so timeout and if they don't come, well just return
            // Escape
            let fst = screen.read_event(ESC_TIMEOUT)?;
            let snd = screen.read_event(ESC_TIMEOUT)?;

            let ev = match (fst, snd) {
                (Some(fst), Some(snd)) => pat_match_escaped_keys(fst, snd),
                _ => None,
            };

            ev.unwrap_or(Key(Escape))
        }
        Key(Ctrl('c')) => Signal(Interrupt),
        x => x,
    };

    Ok(res)
}

fn pat_match_escaped_keys(first: Event, second: Event) -> Option<Event> {
    use mortal::{Event::*, Key::*};

    match (first, second) {
        (Key(Char('[')), Key(Char('A'))) => Some(Key(Up)),
        (Key(Char('[')), Key(Char('B'))) => Some(Key(Down)),
        (Key(Char('[')), Key(Char('C'))) => Some(Key(Right)),
        (Key(Char('[')), Key(Char('D'))) => Some(Key(Left)),
        _ => None,
    }
}

pub struct TermEventIter<'a>(pub &'a mut Screen);

impl<'a> Iterator for TermEventIter<'a> {
    type Item = Event;
    fn next(&mut self) -> Option<Event> {
        read_terminal_event(self.0).ok()
    }
}

pub fn apply_event_to_buf(mut buf: InputBuffer, event: Event) -> (InputBuffer, bool) {
    let cmd = match event {
        Key(Left) => {
            buf.move_pos_left(1);
            false
        }
        Key(Right) => {
            buf.move_pos_right(1);
            false
        }
        Key(Backspace) => {
            buf.backspace();
            true
        }
        Key(Delete) => {
            buf.delete();
            true
        }
        Key(Char(c)) => {
            buf.insert(c);
            true
        }
        _ => false,
    };

    (buf, cmd)
}

/// Given an initial buffer starting point in the terminal, offset the cursor to the buffer's
/// character position. This method is indiscriminant
/// of what is on screen.
///
/// # Wrapping
/// Wrapping on a new line starts at column 0.
pub fn set_cursor_from_input(initial: (u16, u16), buf: &InputBuffer) -> io::Result<()> {
    let (initialx, initialy) = initial;
    let term = xterm::terminal();
    let width = term.terminal_size().0 as isize;

    let mut lines = 0;
    let mut chpos = (buf.ch_pos() as isize) - (width - initialx as isize);
    while chpos >= 0 {
        lines += 1;
        chpos -= width;
    }

    chpos = width + chpos;

    let x = chpos as u16;
    let y = initialy + lines;

    xterm::cursor().goto(x, y).map_err(|e| match e {
        xterm::ErrorKind::IoError(e) => e,
        _ => io::Error::new(io::ErrorKind::Other, "cursor setting failed"),
    })
}

/// Returns where the cursor ends up.
pub fn write_input_buffer(
    initial: (u16, u16),
    lines_to_clear: u16,
    buf: &InputBuffer,
) -> io::Result<(u16, u16)> {
    let (x, y) = initial;
    let mut stdout = stdout()
        .queue(Goto(x, y))
        .queue(Clear(ClearType::UntilNewLine));

    for i in 1..=lines_to_clear {
        stdout = stdout
            .queue(Goto(0, y + i))
            .queue(Clear(ClearType::UntilNewLine));
    }

    stdout
        .queue(Goto(x, y))
        .queue(Output(buf.buffer()))
        .flush()?;

    Ok(xterm::cursor().pos())
}

pub fn read_until(
    screen: &mut Screen,
    initial: (u16, u16),
    buf: InputBuffer,
    events: &[Event],
) -> (InputBuffer, Event) {
    let iter = TermEventIter(screen);

    let mut last = Event::NoEvent;

    let mut lines_to_clear = 0;

    let input = iter
        .inspect(|ev| last = *ev)
        .take_while(|ev| !events.contains(ev))
        .fold(buf, |buf, ev| {
            let (buf, chg) = apply_event_to_buf(buf, ev);
            if chg {
                let write_to = write_input_buffer(initial, lines_to_clear, &buf).unwrap_or(initial);
                lines_to_clear = write_to.1.saturating_sub(initial.1);
            }

            set_cursor_from_input(initial, &buf).ok();

            buf
        });

    (input, last)
}

pub fn write_output_chg(change: OutputChange) -> io::Result<()> {
    use OutputChange::*;
    let mut stdout = stdout();
    match change {
        CurrentLine(line) => erase_current_line(stdout).queue(Output(line)).flush(),
        NewLine => writeln!(&mut stdout, ""),
    }
}

/// Resets position to start of line.
/// **Does not flush, should be called afterwards.**
pub fn erase_current_line(stdout: Stdout) -> Stdout {
    let (_, y) = xterm::cursor().pos();
    stdout
        .queue(Clear(ClearType::CurrentLine))
        .queue(Goto(0, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_movement() {
        let mut input = InputBuffer::new();

        "Hello, world!".chars().for_each(|c| input.insert(c));
        assert_eq!(&input.buffer(), "Hello, world!");
        assert_eq!(input.pos, 13);

        // can't go past end of buffer
        input.move_pos_right(1);
        assert_eq!(input.pos, 13);

        input.move_pos_left(1);
        assert_eq!(input.pos, 12);

        input.insert('?');
        assert_eq!(&input.buffer(), "Hello, world?!");
        assert_eq!(input.pos, 13);

        // can't go past start of buffer
        input.move_pos_left(14);
        assert_eq!(input.pos, 0);
    }

    #[test]
    fn test_input_removing() {
        let mut input = InputBuffer::new();

        "Hello, world!".chars().for_each(|c| input.insert(c));

        input.delete();
        assert_eq!(&input.buffer(), "Hello, world!");
        assert_eq!(input.pos, 13);

        input.backspace();
        assert_eq!(&input.buffer(), "Hello, world");
        assert_eq!(input.pos, 12);

        input.move_pos_left(14);
        input.backspace();
        assert_eq!(&input.buffer(), "Hello, world");
        assert_eq!(input.pos, 0);

        input.delete();
        assert_eq!(&input.buffer(), "ello, world");
        assert_eq!(input.pos, 0);
    }
}