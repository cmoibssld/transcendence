use diesel_demo::{create_user, establish_connection};

fn main() {
    let mut conn = match establish_connection() {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("DB connection error: {err}");
            std::process::exit(1);
        }
    };

    let seeds = [
        ("aguillou", "aguillou@emiel.com", "wiwiaguillou"),
        ("jlepany", "jlepany@example.com", "wiwijlepany"),
        ("zizi", "zizi@example.com", "wiwizizi"),
    ];

    for (username, email, password_hash) in seeds {
        match create_user(&mut conn, username, email, password_hash) {
            Ok(_) => println!("Upserted user: {username}"),
            Err(err) => {
                eprintln!("Failed to insert {username}: {err}");
                std::process::exit(1);
            }
        }
    }
}
