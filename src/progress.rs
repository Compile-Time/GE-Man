use std::io::Read;

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

use ge_man_lib::download::ReadProgressWrapper;
use ge_man_lib::download::response::GeAsset;

fn style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("{msg} {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .progress_chars("=>-")
}

#[derive(Clone)]
pub struct DownloadProgressTracker {
    pb: ProgressBar,
}

impl DownloadProgressTracker {
    pub fn new(pb: ProgressBar) -> Self {
        DownloadProgressTracker { pb }
    }
}

impl Default for DownloadProgressTracker {
    fn default() -> Self {
        DownloadProgressTracker::new(ProgressBar::new(0))
    }
}

impl ReadProgressWrapper for DownloadProgressTracker {
    fn init(self: Box<Self>, len: u64) -> Box<dyn ReadProgressWrapper> {
        let pb = ProgressBar::with_draw_target(len, ProgressDrawTarget::stdout())
            .with_style(style())
            .with_message("Downloading archive:");

        Box::new(DownloadProgressTracker::new(pb))
    }

    fn wrap(&self, reader: Box<dyn Read>) -> Box<dyn Read> {
        Box::new(self.pb.wrap_read(reader))
    }

    fn finish(&self, asset: &GeAsset) {
        self.pb
            .finish_with_message(format!("Finished download of {}", asset.name))
    }
}

pub struct ExtractionProgressTracker {
    pb: ProgressBar,
}

impl ExtractionProgressTracker {
    pub fn new(len: u64) -> Self {
        let pb = ProgressBar::with_draw_target(len, ProgressDrawTarget::stdout())
            .with_message("Extracting archive:")
            .with_style(style());

        ExtractionProgressTracker { pb }
    }

    pub fn inner(&self) -> &ProgressBar {
        &self.pb
    }

    pub fn finish(&self) {
        self.pb.finish_with_message("Finished archive extraction");
    }
}
