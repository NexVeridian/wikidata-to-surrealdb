use indicatif::{ProgressBar, ProgressState, ProgressStyle};

pub async fn create_pb() -> ProgressBar {
    let total_size = 112_500_000;
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ETA:[{eta}]",
        )
        .unwrap()
        .with_key(
            "eta",
            |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                let sec = state.eta().as_secs();
                let min = (sec / 60) % 60;
                let hr = (sec / 60) / 60;
                write!(w, "{}:{:02}:{:02}", hr, min, sec % 60).unwrap()
            },
        ),
    );
    pb
}
