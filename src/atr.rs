pub struct ATR {
    sector_size: usize,
    data: Vec<u8>,
}

impl ATR {
    pub fn new(data: &[u8]) -> Self {
        assert!(data[0] == 0x96);
        assert!(data[1] == 0x02);
        let sector_size = data[4] as usize + 256 * data[5] as usize;
        assert!(sector_size == 128 || sector_size == 256);
        Self { data: data[16..].to_owned(), sector_size }
    }
    pub fn sector_size(&self) -> usize {
        self.sector_size
    }
    pub fn get_sector(&self, n: usize) -> Option<&[u8]> {
        let start = (n - 1) * self.sector_size;
        let end = start + self.sector_size;
        if end <= self.data.len() {
            Some(&self.data[start..end])
        } else {
            None
        }
    }
}
