mod table;

use std::io::Write;
use crate::arithtable::{ZP_STATE_TABLE, ZP_STATE_TABLE_PATCHED};
use crate::encode::zp::arithmetic_coder::{ArithmeticError, ZpArithmeticEncoder};
use table::DEFAULT_ZP_TABLE;
use thiserror::Error;

pub type BitContext = u8;

#[derive(Error, Debug)]
pub enum ZpCodecError {
    #[error("Arithmetic coding error: {0:?}")]
    Arithmetic(ArithmeticError),
    #[error("Encoder already finished")]
    Finished,
}

impl From<ArithmeticError> for ZpCodecError {
    fn from(err: ArithmeticError) -> Self {
        ZpCodecError::Arithmetic(err)
    }
}

struct ZpTables {
    p: [u16; 256],
    m: [u16; 256],
    up: [BitContext; 256],
    dn: [BitContext; 256],
}

impl ZpTables {
    fn new(djvu_compat: bool) -> Self {
        let mut p = [0; 256];
        let mut m = [0; 256];
        let mut up = [0; 256];
        let mut dn = [0; 256];

        for i in 0..256 {
            p[i] = DEFAULT_ZP_TABLE[i].p;
            m[i] = DEFAULT_ZP_TABLE[i].m;
            up[i] = DEFAULT_ZP_TABLE[i].up;
            dn[i] = DEFAULT_ZP_TABLE[i].dn;
        }

        if !djvu_compat {
            for j in 0..256 {
                let a = 0x10000u32 - p[j] as u32;
                let a_norm = if a >= 0x8000 { a << 1 } else { a };
                if m[j] > 0 && a + p[j] as u32 >= 0x8000 && a_norm >= m[j] as u32 {
                    let x = DEFAULT_ZP_TABLE[j].dn;
                    let y = DEFAULT_ZP_TABLE[x as usize].dn;
                    dn[j] = y;
                }
            }
        }

        Self { p, m, up, dn }
    }
}

pub struct ZpEncoder<W: Write> {
    ac: Option<ZpArithmeticEncoder<W>>,
    tables: ZpTables,
    a: u32,      // Probability interval base
    subend: u32, // Carry for interval arithmetic
    finished: bool,
}

impl<W: Write> ZpEncoder<W> {
    pub fn new(writer: W, djvu_compat: bool) -> Self {
        let tables = ZpTables::new(djvu_compat);
        let table_ref = if djvu_compat {
            &ZP_STATE_TABLE
        } else {
            &*ZP_STATE_TABLE_PATCHED
        };
        let ac = ZpArithmeticEncoder::new(writer, table_ref);
        Self {
            ac: Some(ac),
            tables,
            a: 0,
            subend: 0,
            finished: false,
        }
    }

    pub fn encode(&mut self, bit: bool, ctx: &mut BitContext) -> Result<(), ZpCodecError> {
        if self.finished {
            return Err(ZpCodecError::Finished);
        }
        let z = self.a + self.subend;
        let p = self.tables.p[*ctx as usize] as u32;
        let m = self.tables.m[*ctx as usize] as u32;
        let lps_range = (z * p) >> 16;
        let mps_val = z >= 0x8000;

        if bit == mps_val {
            self.encode_mps(ctx, z - lps_range)?;
            if z < m {
                *ctx = self.tables.dn[*ctx as usize];
            }
        } else {
            self.encode_lps(ctx, lps_range)?;
            if z >= m {
                *ctx = self.tables.up[*ctx as usize];
            }
        }
        Ok(())
    }

    pub fn iw_encoder(&mut self, bit: bool) -> Result<(), ZpCodecError> {
        if self.finished {
            return Err(ZpCodecError::Finished);
        }
        let z = self.a + self.subend;
        let p = 0x8000u32;
        let lps_range = (z * p) >> 16;
        let mps_val = z >= 0x8000;

        if bit == mps_val {
            self.encode_mps_simple(z - lps_range)?;
        } else {
            self.encode_lps_simple(lps_range)?;
        }
        Ok(())
    }

    pub fn encode_raw(&mut self, bit: bool) -> Result<(), ZpCodecError> {
        self.iw_encoder(bit)
    }

    pub fn finish(&mut self) -> Result<W, ZpCodecError> {
        if self.ac.is_none() {
            return Err(ZpCodecError::Finished);
        }
        let ac = self.ac.take().unwrap();
        let writer = ac.finish().map_err(|e| -> ZpCodecError { e.into() })?;
        self.finished = true;
        Ok(writer)
    }

    fn encode_mps(&mut self, ctx: &mut BitContext, z: u32) -> Result<(), ZpCodecError> {
        let d = 0x6000 + ((z + self.a) >> 2);
        let z_clipped = if z > d { d } else { z };

        if self.a >= self.tables.m[*ctx as usize] as u32 {
            *ctx = self.tables.up[*ctx as usize];
        }

        self.a = z_clipped;
        if self.a >= 0x8000 {
            self.ac.as_mut().unwrap().encode_bit(*ctx as usize, true)?;
            self.subend <<= 1;
            self.a <<= 1;
        }
        Ok(())
    }

    fn encode_lps(&mut self, ctx: &mut BitContext, z: u32) -> Result<(), ZpCodecError> {
        let d = 0x6000 + ((z + self.a) >> 2);
        let z_clipped = if z > d { d } else { z };

        *ctx = self.tables.dn[*ctx as usize];

        let z_inv = 0x10000 - z_clipped;
        self.subend += z_inv;
        self.a += z_inv;

        while self.a >= 0x8000 {
            self.ac.as_mut().unwrap().encode_bit(*ctx as usize, (self.subend >> 15) != 0)?;
            self.subend <<= 1;
            self.a <<= 1;
        }
        Ok(())
    }

    fn encode_mps_simple(&mut self, z: u32) -> Result<(), ZpCodecError> {
        self.a = z;
        if self.a >= 0x8000 {
            self.ac.as_mut().unwrap().encode_bit(0, true)?;
            self.subend <<= 1;
            self.a <<= 1;
        }
        Ok(())
    }

    fn encode_lps_simple(&mut self, z: u32) -> Result<(), ZpCodecError> {
        let z_inv = 0x10000 - z;
        self.subend += z_inv;
        self.a += z_inv;
        while self.a >= 0x8000 {
            self.ac.as_mut().unwrap().encode_bit(0, (self.subend >> 15) != 0)?;
            self.subend <<= 1;
            self.a <<= 1;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), ZpCodecError> {
        if self.finished {
            return Ok(());
        }
        self.subend = if self.subend > 0x8000 {
            0x10000
        } else if self.subend > 0 {
            0x8000
        } else {
            0
        };

        while self.subend != 0 {
            self.ac.as_mut().unwrap().encode_bit(0, (self.subend >> 15) == 0)?;
            self.subend <<= 1;
        }
        self.ac.as_mut().unwrap().flush(false)?;
        Ok(())
    }
}

impl<W: Write> Drop for ZpEncoder<W> {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.finish();
        }
    }
}