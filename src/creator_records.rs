#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordCounts {
    pub digital_asset_deliveries: usize,
    pub subscriber_updates: usize,
    pub content_processing_jobs: usize,
}

impl RecordCounts {
    pub fn total(&self) -> usize {
        self.digital_asset_deliveries + self.subscriber_updates + self.content_processing_jobs
    }
}

#[derive(Debug)]
pub struct CreatorRecords {
    counts: RecordCounts,
}

impl CreatorRecords {
    pub fn new(counts: RecordCounts) -> Self {
        Self { counts }
    }

    pub fn erase_all(&mut self) -> RecordCounts {
        std::mem::take(&mut self.counts)
    }

    pub fn is_empty(&self) -> bool {
        self.counts.total() == 0
    }
}
