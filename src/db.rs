use sqlx::SqlitePool;

pub async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS feeds (
            id TEXT PRIMARY KEY,
            url TEXT NOT NULL,
            manage_token TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            feed_id TEXT NOT NULL,
            summary TEXT,
            description TEXT,
            start_time TEXT,
            end_time TEXT,
            FOREIGN KEY (feed_id) REFERENCES feeds (id)
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}

// TODO: Implement functions for CRUD operations on feeds and events
