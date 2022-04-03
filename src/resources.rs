pub struct UIConfig {
    pub auto_hide_cnt: usize,
    pub memory: [MemoryConfig; 4],
    pub small_screen: bool,
    pub cpu: bool,
    pub antic: bool,
    pub gtia: bool,
    pub disasm: bool,
    pub fps: bool,
    pub debugger: bool,
    pub basic: bool,
}

impl UIConfig {
    pub const AUTO_HIDE: usize = 100;
    pub fn auto_hide_tick(&mut self) -> bool {
        if self.auto_hide_cnt > 0 {
            self.auto_hide_cnt -= 1;
        }
        self.auto_hide_cnt > 0
    }
    pub fn reset_auto_hide(&mut self) -> bool {
        self.auto_hide_cnt = UIConfig::AUTO_HIDE;
        true
    }
    pub fn all_unchecked(&self) -> bool {
        return !(self.disasm
            || self.gtia
            || self.antic
            || self.cpu
            || self.debugger
            || self.small_screen
            || self.memory.iter().any(|v| v.enabled));
    }
}

pub struct MemoryConfig {
    pub enabled: bool,
    pub address: String,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            address: "0000".to_string(),
        }
    }
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            memory: [
                MemoryConfig::default(),
                MemoryConfig::default(),
                MemoryConfig::default(),
                MemoryConfig::default(),
            ],
            auto_hide_cnt: UIConfig::AUTO_HIDE,
            small_screen: false,
            cpu: false,
            debugger: false,
            antic: false,
            gtia: false,
            disasm: false,
            fps: true,
            basic: false,
        }
    }
}
