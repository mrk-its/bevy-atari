use bevy::utils::Instant;
use std::time::Duration;

use crate::platform::FileSystem;

#[derive(Debug)]
pub struct ATR {
    path: String,
    updated_at: Option<Instant>,
    sector_size: usize,
    data: Vec<u8>,
}

impl ATR {
    pub fn new(path: &str, data: &[u8]) -> Self {
        assert!(data[0] == 0x96);
        assert!(data[1] == 0x02);
        let sector_size = data[4] as usize + 256 * data[5] as usize;
        assert!(sector_size == 128 || sector_size == 256);
        Self {
            path: path.to_owned(),
            data: data.to_owned(),
            sector_size,
            updated_at: None,
        }
    }
    pub fn sector_size(&self) -> usize {
        self.sector_size
    }

    pub fn get_status(&self, data: &mut [u8]) -> u8 {
        data[0] = if self.sector_size == 256 { 0x20 } else { 0 };
        data[1] = 0;
        data[2] = 0;
        data[3] = 0;
        0x01
    }

    pub fn get_sector(&self, n: usize, data: &mut [u8]) -> u8 {
        let range = self.get_range(n);
        if data.len() == range.len() && range.end <= self.data.len() {
            data.copy_from_slice(&self.data[range]);
            0x01
        } else {
            0xff
        }
    }

    pub fn put_sector(&mut self, n: usize, data: &[u8]) -> u8 {
        let range = self.get_range(n);
        if data.len() == range.len() {
            self.data[range].copy_from_slice(data);
            self.updated_at = Some(Instant::now());
            0x01
        } else {
            0xff
        }
    }

    fn get_range(&self, sector: usize) -> std::ops::Range<usize> {
        assert!(sector > 0);
        let sector_size = if sector <= 3 { 128 } else { self.sector_size };
        let start = if sector <= 3 {
            16 + (sector - 1) * sector_size
        } else {
            16 + 3 * 128 + (sector - 4) * sector_size
        };
        start..start + sector_size
    }

    pub fn store(&mut self, fs: &FileSystem) {
        if let Some(t) = self.updated_at {
            let now = Instant::now();
            if (now - t) >= Duration::from_millis(500) {
                bevy::log::info!("storing data in {}", self.path);
                fs.write(&self.path, &self.data);
                self.updated_at = None
            }
        }
    }
}
