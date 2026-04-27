use diesel_demo::{create_user, establish_pool};

fn main() {
    let pool = match establish_pool() {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("DB pool error: {err}");
            std::process::exit(1);
        }
    };

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("DB pooled connection error: {err}");
            std::process::exit(1);
        }
    };

    let seeds = [
        (
            "aguillou",
            "aguillou@emiel.com",
            "wiwiaguillou",
            1200_i64,
            "avatar_knight",
        ),
        (
            "jlepany",
            "jlepany@example.com",
            "wiwijlepany",
            950_i64,
            "avatar_wizard",
        ),
        (
            "zizi",
            "zizi@example.com",
            "wiwizizi",
            300_i64,
            "avatar_rogue",
        ),
    ];

    for (username, email, password_hash, money, avatar) in seeds {
        match create_user(&mut conn, username, email, password_hash, money, avatar) {
            Ok(user) => println!(
                "Upserted user #{}, {} <{}>, money={}, avatar={} ",
                user.id, user.username, user.email, user.money, user.avatar
            ),
            Err(err) => {
                eprintln!("Failed to insert {username}: {err}");
                std::process::exit(1);
            }
        }
    }
}
