use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

pub mod schema;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub money: i64,
    pub avatar: String,
}

#[derive(Insertable)] // This struct is used for inserting new users into the database
#[diesel(table_name = crate::schema::users)]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password_hash: &'a str,
    pub money: i64,
    pub avatar: &'a str,
}

#[derive(AsChangeset, Default)]
#[diesel(table_name = crate::schema::users)]
pub struct UpdateUser<'a> {
    pub username: Option<&'a str>,
    pub email: Option<&'a str>,
    pub password_hash: Option<&'a str>,
    pub money: Option<i64>,
    pub avatar: Option<&'a str>,
}

pub fn establish_connection() -> Result<PgConnection, ConnectionError> {
    dotenvy::dotenv().ok(); // Load .env if available

    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        ConnectionError::BadConnection("DATABASE_URL must be set in environment or .env".into())
    })?;

    PgConnection::establish(&database_url)
}

pub fn establish_pool() -> Result<DbPool, Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")?;
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder().build(manager)?;
    Ok(pool)
}

pub fn create_user(
    conn: &mut PgConnection,
    username: &str,
    email: &str,
    password_hash: &str,
    money: i64,
    avatar: &str,
) -> QueryResult<User> {
    use crate::schema::users::dsl;
    use diesel::upsert::excluded;

    let new_user = NewUser {
        username,
        email,
        password_hash,
        money,
        avatar,
    };

    diesel::insert_into(crate::schema::users::table)
        .values(&new_user)
        .on_conflict(dsl::email)
        .do_update()
        .set((
            dsl::username.eq(excluded(dsl::username)),
            dsl::password_hash.eq(excluded(dsl::password_hash)),
            dsl::money.eq(excluded(dsl::money)),
            dsl::avatar.eq(excluded(dsl::avatar)),
        ))
        .returning(User::as_returning())
        .get_result(conn)
}

pub fn list_users(conn: &mut PgConnection) -> QueryResult<Vec<User>> {
    use crate::schema::users::dsl;

    dsl::users
        .order(dsl::id.asc())
        .select(User::as_select())
        .load(conn)
}

pub fn get_user_by_id(conn: &mut PgConnection, user_id: i32) -> QueryResult<User> {
    use crate::schema::users::dsl;

    dsl::users
        .filter(dsl::id.eq(user_id))
        .select(User::as_select())
        .first(conn)
}

pub fn update_user(
    conn: &mut PgConnection,
    user_id: i32,
    changes: UpdateUser<'_>,
) -> QueryResult<User> {
    use crate::schema::users::dsl;

    diesel::update(dsl::users.filter(dsl::id.eq(user_id)))
        .set(changes)
        .returning(User::as_returning())
        .get_result(conn)
}

pub fn delete_user(conn: &mut PgConnection, user_id: i32) -> QueryResult<usize> {
    use crate::schema::users::dsl;

    diesel::delete(dsl::users.filter(dsl::id.eq(user_id))).execute(conn)
}
