use backon::ExponentialBuilder;
use tokio::{sync::OnceCell, time::Duration};

static BACKOFF_EXPONENTIAL: OnceCell<ExponentialBuilder> = OnceCell::const_new();

pub async fn get_exponential() -> &'static ExponentialBuilder {
    BACKOFF_EXPONENTIAL
        .get_or_init(|| async {
            ExponentialBuilder::default()
                .with_max_times(30)
                .with_max_delay(Duration::from_secs(60))
        })
        .await
}
