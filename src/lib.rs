use diesel::pg::PgConnection;
use diesel::prelude::*;

pub mod schema;

#[derive(Insertable)] // This struct is used for inserting new users into the database
#[diesel(table_name = crate::schema::users)]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password_hash: &'a str,
}

pub fn establish_connection() -> Result<PgConnection, ConnectionError> {
    dotenvy::dotenv().ok(); // Load .env if available

    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        ConnectionError::BadConnection("DATABASE_URL must be set in environment or .env".into())
    })?;

    PgConnection::establish(&database_url)
}

pub fn create_user(
    conn: &mut PgConnection,
    username: &str,
    email: &str,
    password_hash: &str,
) -> QueryResult<usize> {
    use crate::schema::users::dsl;
    use diesel::upsert::excluded;

    let new_user = NewUser {
        username,
        email,
        password_hash,
    };

    diesel::insert_into(crate::schema::users::table)
        .values(&new_user)
        .on_conflict(dsl::email)
        .do_update()
        .set((
            dsl::username.eq(excluded(dsl::username)),
            dsl::password_hash.eq(excluded(dsl::password_hash)),
        ))
        .execute(conn)
}
