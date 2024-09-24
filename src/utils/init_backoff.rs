use backon::ExponentialBuilder;
use lazy_static::lazy_static;
use tokio::time::Duration;

lazy_static! {
    pub static ref exponential: ExponentialBuilder = ExponentialBuilder::default()
        .with_max_times(30)
        .with_max_delay(Duration::from_secs(60));
}
