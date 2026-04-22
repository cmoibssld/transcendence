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
        ("aguillou", "aguillou@emiel.com", "wiwiaguillou", 1200_i64),
        ("jlepany", "jlepany@example.com", "wiwijlepany", 950_i64),
        ("zizi", "zizi@example.com", "wiwizizi", 300_i64),
    ];

    for (username, email, password_hash, money) in seeds {
        match create_user(&mut conn, username, email, password_hash, money) {
            Ok(user) => println!(
                "Upserted user #{}, {} <{}>, money={} ",
                user.id, user.username, user.email, user.money
            ),
            Err(err) => {
                eprintln!("Failed to insert {username}: {err}");
                std::process::exit(1);
            }
        }
    }
}
