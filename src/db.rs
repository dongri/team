use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::NoTls;

pub type PgPool = Pool;

pub fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let pg_config: tokio_postgres::Config = database_url.parse()?;
    let manager = Manager::from_config(
        pg_config,
        NoTls,
        ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        },
    );
    let pool = Pool::builder(manager).max_size(16).build()?;
    Ok(pool)
}
