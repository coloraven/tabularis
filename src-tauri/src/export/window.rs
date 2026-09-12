//! Stream window: skip `offset` rows, then write at most `max_rows`.

/// Sentinel returned from row callbacks when the export window is full.
/// Call sites map this to a successful early stop (not a user-facing error).
pub const EXPORT_LIMIT_REACHED: &str = "__export_row_limit__";

#[derive(Debug, Clone, Copy)]
pub struct ExportWindow {
    pub offset: u64,
    pub max_rows: Option<u64>,
}

impl ExportWindow {
    pub fn new(offset: Option<u64>, max_rows: Option<u64>) -> Self {
        Self {
            offset: offset.unwrap_or(0),
            max_rows,
        }
    }

    pub fn unlimited() -> Self {
        Self {
            offset: 0,
            max_rows: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowAction {
    Skip,
    Write,
    Stop,
}

#[derive(Debug)]
pub struct ExportWindowCounter {
    window: ExportWindow,
    skipped: u64,
    written: u64,
}

impl ExportWindowCounter {
    pub fn new(window: ExportWindow) -> Self {
        Self {
            window,
            skipped: 0,
            written: 0,
        }
    }

    pub fn next(&mut self) -> RowAction {
        if self.skipped < self.window.offset {
            self.skipped += 1;
            return RowAction::Skip;
        }
        if let Some(max) = self.window.max_rows {
            if self.written >= max {
                return RowAction::Stop;
            }
        }
        self.written += 1;
        RowAction::Write
    }

    pub fn written(&self) -> u64 {
        self.written
    }
}

pub fn is_limit_reached(err: &str) -> bool {
    err == EXPORT_LIMIT_REACHED
}

#[cfg(test)]
#[path = "window_tests.rs"]
mod tests;
