//! An arithmetic coder specifically for the ZP codec.

use crate::arithtable::State;
use std::io::Write;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ArithmeticError {
    #[error("I/O error: {0}")]
    Io(String),
}

impl From<std::io::Error> for ArithmeticError {
    fn from(err: std::io::Error) -> Self {
        ArithmeticError::Io(err.to_string())
    }
}

pub struct ZpArithmeticEncoder<W: Write> {
    writer: W,
    table: &'static [State],
    a: u32, // Interval size
    c: u32, // Code buffer
    b: u8,  // Current byte being built
    ct: u8, // Countdown to next byte
    finished: bool,
}

impl<W: Write> ZpArithmeticEncoder<W> {
    pub fn new(writer: W, table: &'static [State]) -> Self {
        Self {
            writer,
            table,
            a: 0x8000,
            c: 0,
            b: 0,
            ct: 12,
            finished: false,
        }
    }

    pub fn encode_bit(&mut self, ctx: usize, mps_val: bool) -> Result<(), ArithmeticError> {
        let state = &self.table[ctx];
        let qe = state.qe;
        // The `lps` parameter for the internal logic is the inverse of `mps_val`.
        self.encode_qe(qe, !mps_val)
    }

    fn encode_qe(&mut self, q: u16, lps: bool) -> Result<(), ArithmeticError> {
        self.a -= q as u32;
        if !lps { // MPS
            if self.a < 0x8000 {
                if self.a < q as u32 {
                    self.c += self.a;
                }
                self.a = q as u32;
                while self.a < 0x8000 {
                    self.a <<= 1;
                    self.renorm_step()?;
                }
            }
        } else { // LPS
            let q_u32 = q as u32;
            if self.a < q_u32 {
                self.c += self.a;
            }
            self.a = q_u32;
            while self.a < 0x8000 {
                self.a <<= 1;
                self.renorm_step()?;
            }
        }
        Ok(())
    }

    fn renorm_step(&mut self) -> Result<(), ArithmeticError> {
        self.ct -= 1;
        self.c <<= 1;
        if self.ct == 0 {
            let mut temp = self.c >> 19;
            self.writer.write_all(&[self.b + (temp as u8)])?;
            self.b = (self.c >> 11) as u8;
            if self.b == 0xFF {
                self.ct = 7;
            } else {
                self.ct = 8;
            }
            self.c &= 0x7FFFF;
        }
        Ok(())
    }

    pub fn flush(&mut self, _end: bool) -> Result<(), ArithmeticError> {
        // This is called on drop. The main finalization is in `finish`.
        Ok(())
    }

    pub fn finish(mut self) -> Result<W, ArithmeticError> {
        for _ in 0..18 {
            self.renorm_step()?;
        }
        self.writer.write_all(&[self.b])?;
        self.finished = true;
        Ok(self.writer)
    }
}
