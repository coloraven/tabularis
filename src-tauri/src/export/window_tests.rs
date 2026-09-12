use super::*;

#[test]
fn all_rows_are_written_when_unlimited() {
    let mut c = ExportWindowCounter::new(ExportWindow::unlimited());
    assert_eq!(c.next(), RowAction::Write);
    assert_eq!(c.next(), RowAction::Write);
    assert_eq!(c.written(), 2);
}

#[test]
fn skips_offset_then_writes() {
    let mut c = ExportWindowCounter::new(ExportWindow::new(Some(2), Some(3)));
    assert_eq!(c.next(), RowAction::Skip);
    assert_eq!(c.next(), RowAction::Skip);
    assert_eq!(c.next(), RowAction::Write);
    assert_eq!(c.next(), RowAction::Write);
    assert_eq!(c.next(), RowAction::Write);
    assert_eq!(c.next(), RowAction::Stop);
    assert_eq!(c.written(), 3);
}

#[test]
fn max_rows_zero_means_immediate_stop() {
    let mut c = ExportWindowCounter::new(ExportWindow::new(None, Some(0)));
    assert_eq!(c.next(), RowAction::Stop);
}
