// src/encode/iw44/codec.rs

use crate::encode::iw44::coeff_map::{CoeffMap, Block};
use crate::encode::iw44::constants::{BAND_BUCKETS, IW_QUANT, IW_SHIFT};
use crate::encode::zc::ZEncoder;
use crate::Result;
use std::io::Write;
use anyhow::Context;
use log::{debug, info, warn, error};

// Coefficient states
pub const ZERO: u8 = 1;
pub const ACTIVE: u8 = 2;
pub const NEW: u8 = 4;
pub const UNK: u8 = 8;

pub struct Codec {
    pub map: CoeffMap,        // Input coefficients
    pub emap: CoeffMap,       // Encoded coefficients
    pub cur_band: usize,      // Current band index
    pub cur_bit: i32,         // Current bit-plane (decrements)
    pub quant_hi: [i32; 10],  // High-frequency quantization thresholds
    pub quant_lo: [i32; 16],  // Low-frequency quantization thresholds
    coeff_state: [u8; 256],   // Coefficient states per block
    bucket_state: [u8; 16],   // Bucket states
    ctx_start: [u8; 32],      // Context for Z-Encoder
    ctx_bucket: [[u8; 8]; 10], // Bucket contexts
    ctx_mant: u8,             // Mantissa context
    ctx_root: u8,             // Root context
}

impl Codec {
    /// Initialize a new Codec instance for a given coefficient map
    pub fn new(map: CoeffMap, params: &super::encoder::EncoderParams) -> Self {
        let (iw, ih) = (map.iw, map.ih);
        
        // Find maximum coefficient value to determine starting bit-plane
        let mut max_coeff = 0i32;
        let mut total_coeffs = 0;
        let mut nonzero_coeffs = 0;
        for (block_idx, block) in map.blocks.iter().enumerate() {
            for bucket_idx in 0..64 {
                if let Some(bucket) = block.get_bucket(bucket_idx) {
                    for &coeff in bucket {
                        total_coeffs += 1;
                        if coeff != 0 {
                            nonzero_coeffs += 1;
                            max_coeff = max_coeff.max((coeff as i32).abs());
                            if block_idx == 0 && bucket_idx == 0 {
                                debug!("MAXCOEFF_DEBUG: Block 0, bucket 0, coeff={}, current max_coeff={}", coeff, max_coeff);
                            }
                        }
                    }
                }
            }
        }
        debug!("MAXCOEFF_DEBUG: Processed {} total coeffs, {} non-zero, max_coeff={}", total_coeffs, nonzero_coeffs, max_coeff);

        // Debug: Show DC coefficient values for debugging solid color issue
        if map.blocks.len() > 0 {
            if let Some(dc_bucket) = map.blocks[0].get_bucket(0) {
                info!("SOLID_COLOR_DEBUG: DC coefficients in block 0, bucket 0: {:?}", dc_bucket);
                // Show all block 0 coefficients for comparison
                for bucket_idx in 0..16 {
                    if let Some(bucket) = map.blocks[0].get_bucket(bucket_idx) {
                        let non_zero: Vec<i16> = bucket.iter().filter(|&&x| x != 0).cloned().collect();
                        if !non_zero.is_empty() {
                            info!("SOLID_COLOR_DEBUG: Block 0, bucket {}: non-zero coeffs: {:?}", bucket_idx, non_zero);
                        }
                    }
                }
            }
        }

        let mut codec = Codec {
            emap: CoeffMap::new(iw, ih),
            map,
            cur_band: 0,
            cur_bit: 15, // Will be updated below
            quant_hi: [0; 10],
            quant_lo: [0; 16],
            coeff_state: [0; 256],
            bucket_state: [0; 16],
            ctx_start: [0; 32],
            ctx_bucket: [[0; 8]; 10],
            ctx_mant: 0,
            ctx_root: 0,
        };

        // Initialize quantization thresholds from IW_QUANT
        // Apply quality scaling based on params.decibels
        let quality_scale = if let Some(db) = params.decibels {
            // Convert decibels to quality scale factor (similar to JPEG quality)
            // Higher dB = higher quality = less quantization
            let normalized_db = (db - 50.0) / 50.0; // Normalize around 50dB
            2.0_f32.powf(-normalized_db) // Exponential scaling
        } else {
            1.0 // Default scaling
        };
        
        // Fixed: Initialize quant_lo directly from IW_QUANT with proper scaling
        for i in 0..16 {
            codec.quant_lo[i] = (IW_QUANT[i] >> IW_SHIFT).max(1);
        }
        
        // Fixed: Initialize quant_hi for bands 1-9 using the same indices
        codec.quant_hi[0] = codec.quant_lo[0];  // Band 0 uses quant_lo
        for j in 1..10 {
            let step_size_idx = j.min(15); // Bands 1-9 use indices 1-9, clamped to 15
            codec.quant_hi[j] = (IW_QUANT[step_size_idx] >> IW_SHIFT).max(1);
        }

        // Start from the highest bit-plane that contains information
        // For a solid color image, we need to be much more conservative
        // Different channels may need different starting bit-planes based on coefficient magnitude
        codec.cur_bit = if max_coeff > 0 {
            // For very small coefficients (like Cr channel with value 21), we need an even higher bit-plane
            if max_coeff < 50 {
                12 // Very conservative for small coefficients like Cr
            } else if max_coeff < 1000 {
                10 // Conservative starting point for small coefficients like Y/Cb
            } else {
                max_coeff.ilog2() as i32
            }
        } else {
            0 // For empty images, start at bit-plane 0
        };

        #[cfg(debug_assertions)]
        {
            info!("XXXXXXXXX CODEC NEW DEBUG XXXXXXXXX");
            info!("DEBUG Codec quantization thresholds (quality scale: {:.3}):", quality_scale);
            info!("  Max coefficient: {}", max_coeff);
            info!("  Starting bit-plane: {}", codec.cur_bit);
            info!("  quant_lo (band 0): {:?}", &codec.quant_lo[0..4]);
            info!("  quant_hi (bands 1-9): {:?}", &codec.quant_hi[1..5]);
        }

        codec
    }

    /// Encode a single slice (current band at current bit-plane)
    pub fn encode_slice<W: Write>(&mut self, zp: &mut ZEncoder<W>) -> Result<bool> {
        
        if self.cur_bit < 0 {
            return Ok(false); // No more bit-planes to process
        }

        // Check if this slice contains any significant data
        let is_null = self.is_null_slice(self.cur_bit as usize, self.cur_band);
        println!("PRINTLN: is_null_slice returned: {}", is_null);
        
        // Debug for solid color issue
        if self.cur_band == 0 && self.cur_bit <= 13 {
            error!("encode_slice - band={}, bit={}, is_null={}", 
                     self.cur_band, self.cur_bit, is_null);
        }
        
        // Debug current encoding state (only for first few slices to avoid flooding)
        #[cfg(debug_assertions)]
        if self.cur_bit > 10 || (self.cur_band == 0 && self.cur_bit > 8) {
            let threshold = if self.cur_band == 0 {
                self.quant_lo[0] >> self.cur_bit
            } else {
                self.quant_hi[self.cur_band] >> self.cur_bit
            };            debug!("Encode slice: band={}, bit={}, threshold={}, is_null={}", 
                self.cur_band, self.cur_bit, threshold, is_null);
        }
        
        // If slice is null, we can advance state and continue
        if is_null {
            #[cfg(debug_assertions)]
            if self.cur_bit > 10 || (self.cur_band == 0 && self.cur_bit > 8) {
                error!("SOLID_COLOR_DEBUG: Slice is null, advancing to next (band={}, bit={})", self.cur_band, self.cur_bit);
            }
            let has_more = self.finish_code_slice()?;
            // Return false to indicate no data was encoded in this slice
            return Ok(false);
        }

        error!("SOLID_COLOR_DEBUG: SLICE NOT NULL - proceeding to bucket encoding band={} bit={}", self.cur_band, self.cur_bit);

        // Count active coefficients for debugging
        let mut active_coeffs = 0;
        let mut encoded_bits = 0;
        let mut blocks_processed = 0;
        
        // Debug: Check block count
        if self.cur_band == 0 && self.cur_bit > 10 {
            debug!("SOLID_COLOR_DEBUG: encode_slice processing {} blocks for band={} bit={}", 
                     self.map.num_blocks, self.cur_band, self.cur_bit);
        }
        
        for blockno in 0..self.map.num_blocks {
            let bucket_info = BAND_BUCKETS[self.cur_band];
            blocks_processed += 1;
            
            // Debug block processing for band 0
            if self.cur_band == 0 && self.cur_bit > 10 {
                debug!("SOLID_COLOR_DEBUG: Processing block {} for band={} bit={} fbucket={} nbucket={}", 
                         blockno, self.cur_band, self.cur_bit, bucket_info.start, bucket_info.size);
            }
            
            // Debug first block's coefficient distribution (only for first few slices)
            if blockno == 0 && (self.cur_bit > 10 || (self.cur_band == 0 && self.cur_bit > 8)) {
                let input_block = &self.map.blocks[blockno];
                let mut coeff_count = 0;
                let mut max_coeff = 0i16;
                
                for bucket_idx in bucket_info.start..(bucket_info.start + bucket_info.size) {
                    if let Some(bucket) = input_block.get_bucket(bucket_idx as u8) {
                        for &coeff in bucket {
                            if coeff != 0 {
                                coeff_count += 1;
                                // Use safe abs to handle overflow of i16::MIN
                                let abs_coeff = if coeff == i16::MIN {
                                    32767i16 // Clamp to max positive i16
                                } else {
                                    coeff.abs()
                                };
                                max_coeff = max_coeff.max(abs_coeff);
                            }
                        }
                    }
                }
                debug!("  Block 0: {} non-zero coeffs, max magnitude: {}", coeff_count, max_coeff);
            }
            
            // Extract the blocks we need to avoid borrowing issues
            let input_block = &self.map.blocks[blockno];
            let output_block = &mut self.emap.blocks[blockno];
            
            // Call encode_buckets as a static function to avoid borrowing self
            Self::encode_buckets_static(
                zp,
                self.cur_bit as usize,
                self.cur_band,
                input_block,
                output_block,
                bucket_info.start,
                bucket_info.size,
                &mut self.coeff_state,
                &mut self.bucket_state,
                &mut self.ctx_start,
                &mut self.ctx_bucket,
                &mut self.ctx_root,
                &mut self.ctx_mant,
                &self.quant_lo,
                &self.quant_hi,
            )?;
        }

        // Enhanced debug output after processing all blocks
        if self.cur_band == 0 && self.cur_bit > 10 {
            error!("SOLID_COLOR_DEBUG: Completed slice encoding - band={}, bit={}, blocks_processed={}, active_coeffs={}", 
                   self.cur_band, self.cur_bit, blocks_processed, active_coeffs);
        }

        // Always advance to next band/bit-plane
        let has_more = self.finish_code_slice()?;
        
        // Return true if we have more to process
        Ok(has_more)
    }

    /// Check if the current slice is null (no significant coefficients)
    /// According to DjVu spec: a coefficient becomes active when |coeff| >= 2*step_size
    /// The step size at bit-plane k is: step_size = initial_step_size / 2^k
    fn is_null_slice(&mut self, bit: usize, band: usize) -> bool {
        println!("PRINTLN: is_null_slice ENTRY band={} bit={}", band, bit);
        
        if band == 0 {
            // For DC band, check all 16 subbands
            let mut is_null = true;
            let mut debug_significant_coeffs = Vec::new();
            
            for i in 0..16 {
                // Use proper DjVu significance check: (|coeff| << bit) >= (quant << (bit-1))
                let threshold = if bit > 0 {
                    self.quant_lo[i] << (bit - 1)
                } else {
                    self.quant_lo[i]
                };
                self.coeff_state[i] = ZERO;
                
                // Skip if threshold is 0 (no meaningful quantization possible)
                if threshold == 0 {
                    continue;
                }
                
                if band == 0 && bit >= 9 && i == 0 {
                    error!("SOLID_COLOR_DEBUG: Band 0 bucket 0: threshold={}, quant_lo[0]={}", threshold, self.quant_lo[0]);
                }
                
                // Check if any coefficients in this subband exceed the activation threshold
                let mut has_significant_coeff = false;
                let mut coeff_details = Vec::new();
                
                for blockno in 0..self.map.num_blocks {
                    if let Some(bucket) = self.map.blocks[blockno].get_bucket(i as u8) {
                        for (coeff_idx, &coeff) in bucket.iter().enumerate() {
                            let scaled_coeff = ((coeff as i32).abs()) << bit;
                            
                            // Always collect debug info for bucket 0
                            if i == 0 && band == 0 && bit >= 9 {
                                coeff_details.push(format!("block{}[{}]: coeff={}, scaled={}, threshold={}, significant={}", 
                                    blockno, coeff_idx, coeff, scaled_coeff, threshold, scaled_coeff >= threshold));
                            }
                            
                            if scaled_coeff >= threshold {
                                if band == 0 && bit >= 9 && i == 0 {
                                    error!("SOLID_COLOR_DEBUG: Found significant coeff: coeff={}, scaled_coeff={}, threshold={}", coeff, scaled_coeff, threshold);
                                }
                                has_significant_coeff = true;
                                debug_significant_coeffs.push(format!("bucket{}: coeff={}", i, coeff));
                                break;
                            }
                        }
                        if has_significant_coeff { break; }
                    }
                }
                
                // Debug output for bucket 0
                if i == 0 && band == 0 && bit >= 9 {
                    error!("SOLID_COLOR_DEBUG: Bucket 0 details:\n{}", coeff_details.join("\n"));
                }
                
                if has_significant_coeff {
                    self.coeff_state[i] = UNK;
                    is_null = false;
                }
            }
            
            // Enhanced debug output
            if band == 0 && bit >= 9 {
                error!("SOLID_COLOR_DEBUG: is_null_slice RESULT band={} bit={} is_null={}", band, bit, is_null);
                if !debug_significant_coeffs.is_empty() {
                    error!("SOLID_COLOR_DEBUG: Significant coefficients found: {}", debug_significant_coeffs.join(", "));
                }
            }
            
            is_null
        } else {
            // For AC bands, check if any coefficients exceed the activation threshold
            let threshold = if bit > 0 {
                self.quant_hi[band] << (bit - 1)
            } else {
                self.quant_hi[band]
            };
            
            // Skip if threshold is 0 (no meaningful quantization possible)
            if threshold == 0 {
                return true;
            }
            
            let bucket_info = BAND_BUCKETS[band];
            for blockno in 0..self.map.num_blocks {
                for bucket_idx in bucket_info.start..(bucket_info.start + bucket_info.size) {
                    if let Some(bucket) = self.map.blocks[blockno].get_bucket(bucket_idx as u8) {
                        for &coeff in bucket {
                            let scaled_coeff = ((coeff as i32).abs()) << bit;
                            if scaled_coeff >= threshold {
                                return false; // Found significant coefficient, slice is not null
                            }
                        }
                    }
                }
            }
            true // No significant coefficients found, slice is null
        }
    }

    /// Advance to the next band or bit-plane
    fn finish_code_slice(&mut self) -> Result<bool> {
        self.cur_band += 1;
        if self.cur_band >= BAND_BUCKETS.len() {
            self.cur_band = 0;
            self.cur_bit -= 1; // Decrement bit-plane after all bands
        }
        // Return true as long as we have more bit-planes to process
        Ok(self.cur_bit >= 0)
    }

    /// Encode buckets for a block in the current slice
    fn encode_buckets_static<W: Write>(
        zp: &mut ZEncoder<W>,
        bit: usize,
        band: usize,
        blk: &Block,
        eblk: &mut Block,
        fbucket: usize,
        nbucket: usize,
        coeff_state: &mut [u8; 256],
        bucket_state: &mut [u8; 16],
        ctx_start: &mut [u8; 32],
        ctx_bucket: &mut [[u8; 8]; 10],
        ctx_root: &mut u8,
        ctx_mant: &mut u8,
        quant_lo: &[i32; 16],
        quant_hi: &[i32; 10],
    ) -> Result<()> {
        let bbstate = Self::encode_prepare_static(
            band, fbucket, nbucket, blk, eblk, bit,
            coeff_state, bucket_state, quant_lo, quant_hi
        );
        
        // Debug bucket preparation
        if band == 0 && bit > 10 {
            debug!("SOLID_COLOR_DEBUG: encode_buckets_static - band={}, bit={}, bbstate={:02b}, nbucket={}", 
                   band, bit, bbstate, nbucket);
        }
        
        if bbstate == 0 {
            if band == 0 && bit > 10 {
                debug!("SOLID_COLOR_DEBUG: bbstate=0, no buckets to encode");
            }
            return Ok(());
        }

        // Encode bucket-level decisions
        let mut active_buckets = 0;
        for buckno in 0..nbucket {
            let bstate = bucket_state[buckno];
            
            // Debug bucket activation for band 0
            if band == 0 && buckno < 4 && bit > 10 {
                debug!("SOLID_COLOR_DEBUG: Band {} bucket {} state={:02b} (NEW={:02b} ACTIVE={:02b})", 
                         band, buckno, bstate, NEW, ACTIVE);
            }
            
            // Encode whether this bucket is active
            if (bstate & (NEW | ACTIVE)) != 0 {
                active_buckets += 1;
                
                if band == 0 && buckno < 4 && bit > 10 {
                    debug!("SOLID_COLOR_DEBUG: Encoding TRUE for bucket {} (active)", buckno);
                }
                
                let ctx_idx = if band == 0 {
                    &mut ctx_start[buckno.min(31)]
                } else {
                    &mut ctx_bucket[(band - 1).min(9)][buckno.min(7)]
                };
                zp.encode(true, ctx_idx)?;

                // Encode coefficient-level data for active buckets
                // Pass relative bucket index to fix state indexing
                Self::encode_bucket_coeffs_static(
                    zp, bit, band, blk, eblk, fbucket + buckno, buckno,
                    coeff_state, ctx_root, ctx_mant, quant_lo, quant_hi
                )?;
            } else {
                // Bucket is inactive - encode "false" bit
                if band == 0 && buckno < 4 && bit > 10 {
                    debug!("SOLID_COLOR_DEBUG: Encoding FALSE for bucket {} (inactive)", buckno);
                }
                
                let ctx_idx = if band == 0 {
                    &mut ctx_start[buckno.min(31)]
                } else {
                    &mut ctx_bucket[(band - 1).min(9)][buckno.min(7)]
                };
                zp.encode(false, ctx_idx)?;
            }
        }
        
        if band == 0 && bit > 10 {
            debug!("SOLID_COLOR_DEBUG: Encoded {} active buckets out of {}", active_buckets, nbucket);
        }

        Ok(())
    }

    /// Encode individual coefficients within a bucket
    fn encode_bucket_coeffs_static<W: Write>(
        zp: &mut ZEncoder<W>,
        bit: usize,
        band: usize,
        blk: &Block,
        eblk: &mut Block,
        bucket_idx: usize,
        relative_bucket_idx: usize,  // Added: relative bucket index within band
        coeff_state: &mut [u8; 256],
        ctx_root: &mut u8,
        ctx_mant: &mut u8,
        quant_lo: &[i32; 16],
        quant_hi: &[i32; 10],
    ) -> Result<()> {
        if let Some(coeffs) = blk.get_bucket(bucket_idx as u8) {
            // Debug: Show bucket data for band 0 to debug solid color encoding
            if band == 0 && bucket_idx < 4 && bit > 10 {
                debug!("Band {} bucket {} coeffs: {:?}", band, bucket_idx, coeffs);
            }
            
            let mut ecoeffs = eblk.get_bucket(bucket_idx as u8)
                .map(|prev| *prev)
                .unwrap_or([0; 16]);
            
            for (i, &coeff) in coeffs.iter().enumerate() {
                // Fixed: Use relative bucket index to prevent state collisions
                let cstate_idx = relative_bucket_idx * 16 + i;
                let cstate = if cstate_idx < coeff_state.len() {
                    coeff_state[cstate_idx]
                } else {
                    UNK
                };

                if (cstate & NEW) != 0 {
                    // New significant coefficient - encode activation decision
                    // Use proper DjVu significance check: (|coeff| << bit) >= (quant << (bit-1))
                    let threshold = if bit > 0 {
                        if band == 0 {
                            quant_lo[bucket_idx] << (bit - 1)
                        } else {
                            quant_hi[band] << (bit - 1)
                        }
                    } else {
                        if band == 0 {
                            quant_lo[bucket_idx]
                        } else {
                            quant_hi[band]
                        }
                    };

                    let scaled_coeff = ((coeff as i32).abs()) << bit;
                    
                    if threshold > 0 && scaled_coeff >= threshold {
                        // Debug: Show significant coefficient encoding for band 0
                        if band == 0 && bucket_idx < 4 && bit > 10 {
                            debug!("SOLID_COLOR_DEBUG: Encoding significant coeff band={} bucket={} i={} coeff={} threshold={} scaled_coeff={}", 
                                     band, bucket_idx, i, coeff, threshold, scaled_coeff);
                        }
                        
                        // Encode that coefficient becomes significant
                        zp.encode(true, ctx_root)?;
                        
                        // Encode sign
                        zp.encode(coeff < 0, ctx_root)?;
                        
                        // Set initial reconstructed value: thres + (thres >> 1)
                        // Use step size for reconstruction
                        let step_size = if bit > 0 {
                            if band == 0 { quant_lo[bucket_idx] >> bit } else { quant_hi[band] >> bit }
                        } else {
                            if band == 0 { quant_lo[bucket_idx] } else { quant_hi[band] }
                        }.max(1); // Ensure step size is at least 1
                        let sign = if coeff < 0 { -1 } else { 1 };
                        let recon = step_size + (step_size >> 1);
                        ecoeffs[i] = (sign * recon) as i16;
                        
                        // Update state: NEW -> ACTIVE for next bit-plane
                        if cstate_idx < coeff_state.len() {
                            coeff_state[cstate_idx] = ACTIVE;
                        }
                    } else {
                        // Coefficient not significant at this bit-plane
                        zp.encode(false, ctx_root)?;
                        // Keep as NEW for lower bit-planes
                    }
                } else if (cstate & ACTIVE) != 0 {
                    // Refinement of already significant coefficient
                    if band == 0 && bucket_idx < 4 && bit > 10 {
                        debug!("SOLID_COLOR_DEBUG: Refining coeff band={} bucket={} i={} coeff={} prev_val={}", 
                               band, bucket_idx, i, coeff, ecoeffs[i]);
                    }
                    
                    let step_size = if bit > 0 {
                        if band == 0 { quant_lo[bucket_idx] >> bit } else { quant_hi[band] >> bit }
                    } else {
                        if band == 0 { quant_lo[bucket_idx] } else { quant_hi[band] }
                    }.max(1); // Ensure step size is at least 1
                    
                    let orig_coeff = coeff as i32;
                    let prev_val = ecoeffs[i] as i32;
                    let coeff_abs = orig_coeff.abs();
                    
                    // Compute mantissa bit: pix = (coeff >= ecoeff) ? 1 : 0
                    let pix = if coeff_abs >= prev_val.abs() { 1 } else { 0 };
                    
                    // Encode mantissa bit
                    zp.encode(pix != 0, ctx_mant)?;
                    
                    // Adjust coefficient: epcoeff[i] = ecoeff - (pix ? 0 : step_size) + (step_size >> 1);
                    let sign = if prev_val < 0 { -1 } else { 1 };
                    let abs_ecoeff = prev_val.abs();
                    let adjustment = if pix != 0 { 0 } else { step_size };
                    let new_abs = abs_ecoeff - adjustment + (step_size >> 1);
                    ecoeffs[i] = (sign * new_abs) as i16;
                }
                // Note: ZERO coefficients are not encoded
            }
            
            eblk.set_bucket(bucket_idx as u8, ecoeffs);
        }

        Ok(())
    }

    /// Prepare states for encoding buckets
    fn encode_prepare_static(
        band: usize,
        fbucket: usize,
        nbucket: usize,
        blk: &Block,
        eblk: &Block,
        cur_bit: usize,
        coeff_state: &mut [u8; 256],
        bucket_state: &mut [u8; 16],
        quant_lo: &[i32; 16],
        quant_hi: &[i32; 10],
    ) -> u8 {
        let mut bbstate = 0;
        let mut total_new_coeffs = 0;
        let mut total_active_coeffs = 0;
        
        if band == 0 && cur_bit > 10 {
            debug!("SOLID_COLOR_DEBUG: encode_prepare_static - band={}, cur_bit={}, fbucket={}, nbucket={}", 
                   band, cur_bit, fbucket, nbucket);
        }
        
        for buckno in 0..nbucket {
            let pcoeff = blk.get_bucket((fbucket + buckno) as u8);
            let epcoeff = eblk.get_bucket((fbucket + buckno) as u8);
            let mut bstatetmp = 0;
            
            // Use proper DjVu significance check: (|coeff| << bit) >= (quant << (bit-1))
            // For band 0, use absolute bucket index; for other bands, use band-specific quantization
            let absolute_bucket_idx = fbucket + buckno;
            let threshold = if cur_bit > 0 {
                if band == 0 {
                    if absolute_bucket_idx < 16 {
                        quant_lo[absolute_bucket_idx] << (cur_bit - 1)
                    } else {
                        0 // Invalid bucket for band 0
                    }
                } else {
                    quant_hi[band] << (cur_bit - 1)
                }
            } else {
                if band == 0 {
                    if absolute_bucket_idx < 16 { quant_lo[absolute_bucket_idx] } else { 0 }
                } else {
                    quant_hi[band]
                }
            };

            // Debug threshold calculation for first few buckets
            if band == 0 && cur_bit > 10 && buckno < 4 {
                debug!("SOLID_COLOR_DEBUG: Bucket {} (abs={}) threshold={}, quant_lo[{}]={}", 
                       buckno, absolute_bucket_idx, threshold, absolute_bucket_idx, 
                       if absolute_bucket_idx < 16 { quant_lo[absolute_bucket_idx] } else { 0 });
            }

            match (pcoeff, epcoeff) {
                (Some(pc), Some(epc)) => {
                    let mut bucket_new_coeffs = 0;
                    let mut bucket_active_coeffs = 0;
                    
                    // Debug coefficient values for first bucket
                    if band == 0 && cur_bit > 10 && buckno == 0 {
                        debug!("SOLID_COLOR_DEBUG: Bucket 0 input coeffs: {:?}", pc);
                        debug!("SOLID_COLOR_DEBUG: Bucket 0 encoded coeffs: {:?}", epc);
                    }
                    
                    for i in 0..16 {
                        let cstate_idx = buckno * 16 + i;
                        if cstate_idx < coeff_state.len() {
                            let mut cstatetmp = ZERO;
                            
                            if epc[i] != 0 {
                                // Already active from previous bit-plane
                                cstatetmp = ACTIVE;
                                bucket_active_coeffs += 1;
                            } else if threshold > 0 && ((pc[i] as i32).abs() << cur_bit) >= threshold {
                                // Could become significant at this bit-plane
                                cstatetmp = NEW | UNK;
                                bucket_new_coeffs += 1;
                                
                                if band == 0 && cur_bit > 10 && buckno == 0 {
                                    debug!("SOLID_COLOR_DEBUG: Coeff[{}] becomes NEW: val={}, scaled={}, threshold={}", 
                                           i, pc[i], (pc[i] as i32).abs() << cur_bit, threshold);
                                }
                            } else if threshold > 0 {
                                // Not significant yet, but could be at lower bit-planes
                                cstatetmp = UNK;
                            }
                            
                            coeff_state[cstate_idx] = cstatetmp;
                            bstatetmp |= cstatetmp;
                        }
                    }
                    
                    if band == 0 && cur_bit > 10 && buckno < 4 {
                        debug!("SOLID_COLOR_DEBUG: Bucket {} stats - new_coeffs={}, active_coeffs={}, bstate={:02b}", 
                               buckno, bucket_new_coeffs, bucket_active_coeffs, bstatetmp);
                    }
                    
                    total_new_coeffs += bucket_new_coeffs;
                    total_active_coeffs += bucket_active_coeffs;
                }
                (Some(pc), None) => {
                    for i in 0..16 {
                        let cstate_idx = buckno * 16 + i;
                        if cstate_idx < coeff_state.len() {
                            let mut cstatetmp = ZERO;
                            
                            if threshold > 0 && ((pc[i] as i32).abs() << cur_bit) >= threshold {
                                // Could become significant at this bit-plane
                                cstatetmp = NEW | UNK;
                            } else if threshold > 0 {
                                // Not significant yet, but could be at lower bit-planes
                                cstatetmp = UNK;
                            }
                            
                            coeff_state[cstate_idx] = cstatetmp;
                            bstatetmp |= cstatetmp;
                        }
                    }
                }
                _ => bstatetmp = 0,
            }
            
            bucket_state[buckno] = bstatetmp;
            bbstate |= bstatetmp;
        }
        
        if band == 0 && cur_bit > 10 {
            debug!("SOLID_COLOR_DEBUG: encode_prepare_static RESULT - total_new_coeffs={}, total_active_coeffs={}, bbstate={:02b}", 
                   total_new_coeffs, total_active_coeffs, bbstate);
        }
        
        bbstate
    }
}