use indicatif::{ProgressBar, ProgressFinish, ProgressStyle};

pub fn create_progress_bar() -> ProgressBar {
    let template = "{msg:<60} {bytes:>12} {binary_bytes_per_sec:>12} {eta:>3} [{wide_bar:.cyan/blue}] {percent:>3}%";

    ProgressBar::new(0).with_finish(ProgressFinish::Abandon).with_style(
        ProgressStyle::with_template(template).expect("invalid template").progress_chars("##-"),
    )
}
