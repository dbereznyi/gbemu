use std::sync::{Arc};
use std::sync::atomic::{AtomicU64};

pub struct Debug {
    pub cpu_expected_time_micros: Arc<AtomicU64>,
    pub cpu_actual_time_micros: Arc<AtomicU64>,
    pub ppu_expected_time_micros: Arc<AtomicU64>,
    pub ppu_actual_time_micros: Arc<AtomicU64>,
}

impl Debug {
    pub fn new() -> Self {
        Self {
            cpu_expected_time_micros: Arc::new(AtomicU64::new(0)),
            cpu_actual_time_micros: Arc::new(AtomicU64::new(0)),
            ppu_expected_time_micros: Arc::new(AtomicU64::new(0)),
            ppu_actual_time_micros: Arc::new(AtomicU64::new(0)),
        }
    }
}
