use std::sync::{Arc};
use std::sync::atomic::{AtomicU8, AtomicU64};

pub struct DebugInfoCpu {
    pub expected_time_nanos: Arc<AtomicU64>,
    pub actual_time_nanos: Arc<AtomicU64>,
}

impl DebugInfoCpu {
    pub fn new() -> Self {
        Self {
            expected_time_nanos: Arc::new(AtomicU64::new(0)),
            actual_time_nanos: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl Clone for DebugInfoCpu {
    fn clone(&self) -> Self {
        Self {
            expected_time_nanos: self.expected_time_nanos.clone(),
            actual_time_nanos: self.actual_time_nanos.clone(),
        }
    }
}

pub struct DebugInfoPpu {
    pub expected_time_micros: Arc<AtomicU64>,
    pub actual_time_micros: Arc<AtomicU64>,
    pub oam_time_nanos: Arc<AtomicU64>,
}

impl DebugInfoPpu {
    pub fn new() -> Self {
        Self {
            expected_time_micros: Arc::new(AtomicU64::new(0)),
            actual_time_micros: Arc::new(AtomicU64::new(0)),
            oam_time_nanos: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl Clone for DebugInfoPpu {
    fn clone(&self) -> Self {
        Self {
            expected_time_micros: self.expected_time_micros.clone(),
            actual_time_micros: self.actual_time_micros.clone(),
            oam_time_nanos: self.oam_time_nanos.clone(),
        }
    }
}
