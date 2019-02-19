use super::*;
use std::io::{Result, Write};

impl<'a, T: Terminal> Writer<'a, T> {
    pub fn overwrite_current_console_line(&self, line: &str) -> Result<()> {
        let mut wtr = self.0.lock_write();
        wtr.move_to_first_column()?;
        wtr.clear_to_screen_end()?;
        wtr.write(line)
    }
}

impl<'a, T: Terminal> Write for Writer<'a, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut wtr = self.0.lock_write();
        wtr.write(&String::from_utf8_lossy(buf)).unwrap();
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}