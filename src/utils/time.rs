use chrono::{DateTime, Utc};

pub fn datetime_in(delay: chrono::Duration) -> Option<DateTime<Utc>> {
    let future_date = Utc::now().checked_add_signed(delay)?;
    Some(future_date)
}

pub async fn wake_up(at: Option<DateTime<Utc>>) -> Option<()> {
    let at = at?;

    let delay = at.signed_duration_since(Utc::now());
    let delay = chrono::Duration::to_std(&delay).unwrap();
    tokio::time::sleep(delay).await;

    Some(())
}
