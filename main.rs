use postgres::Error as PostgresError;
use postgres::{Client, NoTls};
// use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[macro_use]
extern crate serde_derive;

#[derive(Debug, Deserialize, Serialize)]
struct User {
    id: Option<i32>,
    name: String,
    email: String,
}

// static DB_URL: &str = env::var("DATABASE_URL");
const DB_URL: &str = "postgresql://postgres:postgres@db:5432/postgres";

const OK: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const CREATED: &str = "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\n\r\n";
// const ACCEPTED: &str = "HTTP/1.1 202 Accepted\r\nContent-Type: application/json\r\n\r\n";
const NO_CONTENT: &str = "HTTP/1.1 204 No Content\r\n\r\n";
const BAD_REQUEST: &str = "HTTP/1.1 400 Bad Request\r\n\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 Not Found\r\n\r\n";
// const UNAUTHORIZED: &str = "HTTP/1.1 401 Unauthorized\r\n\r\n";
// const FORBIDDEN: &str = "HTTP/1.1 403 Forbidden\r\n\r\n";
// const UNPROCESSABLE_ENTITY: &str = "HTTP/1.1 422 Unprocessable Entity\r\n\r\n";
// const TOO_MANY_REQUESTS: &str = "HTTP/1.1 429 Too Many Requests\r\n\r\n";
const INTERNAL_SERVER_ERROR: &str = "HTTP/1.1 500 Internal Server Error\r\n\r\n";
// const NOT_IMPLEMENTED: &str = "HTTP/1.1 501 Not Implemented\r\n\r\n";
// const BAD_GATEWAY: &str = "HTTP/1.1 502 Bad Gateway\r\n\r\n";
// const SERVICE_UNAVAILABLE: &str = "HTTP/1.1 503 Service Unavailable\r\n\r\n";
// const GATEWAY_TIMEOUT: &str = "HTTP/1.1 504 Gateway Timeout\r\n\r\n";
// const HTTP_VERSION_NOT_SUPPORTED: &str = "HTTP/1.1 505 HTTP Version Not Supported\r\n\r\n";

fn main() {
    if let Err(e) = set_database() {
        println!("Failed to set database: {}", e);
        return;
    }

    let listener = TcpListener::bind("0.0.0.0:8080").unwrap();
    println!("Listening on http://localhost:8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}

// Routes and handlers
fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            let (status, content) = match &*request {
                // r if r.starts_with("GET /") => health_check(r),
                r if r.starts_with("GET /user/") => handle_get_request(r),
                r if r.starts_with("GET /users") => handle_get_all_request(r),
                r if r.starts_with("POST /user") => handle_post_request(r),
                r if r.starts_with("PUT /user/") => handle_update_request(r),
                r if r.starts_with("DELETE /user/") => handle_delete_request(r),
                _ => (NOT_FOUND.to_string(), "Not Found".to_string()),
            };
            stream
                .write_all(format!("{}{}", status, content).as_bytes())
                .unwrap();
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

// Controllers
// fn health_check(request: &str) -> (String, String) {
//     if request.starts_with("GET /") {
//         (OK.to_string(), serde_json::to_string("OK").unwrap())
//     } else {
//         (
//             INTERNAL_SERVER_ERROR.to_string(),
//             "Internal Server Error".to_string(),
//         )
//     }
// }

fn handle_post_request(request: &str) -> (String, String) {
    match (
        get_user_request_body(&request),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(user), Ok(mut client)) => {
            client
                .execute(
                    "INSERT INTO users (name, email) VALUES ($1, $2)",
                    &[&user.name, &user.email],
                )
                .unwrap();

            (OK.to_string(), "User created".to_string())
        }
        _ => (
            INTERNAL_SERVER_ERROR.to_string(),
            "Internal error".to_string(),
        ),
    }
}

fn handle_get_request(request: &str) -> (String, String) {
    match (
        get_id(&request).parse::<i32>(),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(id), Ok(mut client)) => {
            match client.query_one("SELECT * FROM users WHERE id = $1", &[&id]) {
                Ok(row) => {
                    let user = User {
                        id: row.get(0),
                        name: row.get(1),
                        email: row.get(2),
                    };

                    (OK.to_string(), serde_json::to_string(&user).unwrap())
                }
                _ => (NOT_FOUND.to_string(), "User not found".to_string()),
            }
        }

        _ => (
            INTERNAL_SERVER_ERROR.to_string(),
            "Internal error".to_string(),
        ),
    }
}

fn handle_get_all_request(_request: &str) -> (String, String) {
    match Client::connect(DB_URL, NoTls) {
        Ok(mut client) => {
            let mut users = Vec::new();

            for row in client
                .query("SELECT id, name, email FROM users", &[])
                .unwrap()
            {
                users.push(User {
                    id: row.get(0),
                    name: row.get(1),
                    email: row.get(2),
                });
            }

            (OK.to_string(), serde_json::to_string(&users).unwrap())
        }
        _ => (
            INTERNAL_SERVER_ERROR.to_string(),
            "Internal error".to_string(),
        ),
    }
}

fn handle_update_request(request: &str) -> (String, String) {
    match (
        get_id(&request).parse::<i32>(),
        get_user_request_body(&request),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(id), Ok(user), Ok(mut client)) => {
            client
                .execute(
                    "UPDATE users SET name = $1, email = $2 WHERE id = $3",
                    &[&user.name, &user.email, &id],
                )
                .unwrap();

            (OK.to_string(), "User updated".to_string())
        }
        _ => (
            INTERNAL_SERVER_ERROR.to_string(),
            "Internal error".to_string(),
        ),
    }
}

fn handle_delete_request(request: &str) -> (String, String) {
    match (
        get_id(&request).parse::<i32>(),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(id), Ok(mut client)) => {
            let rows_affected = client
                .execute("DELETE FROM users WHERE id = $1", &[&id])
                .unwrap();

            //if rows affected is 0, user not found
            if rows_affected == 0 {
                return (NOT_FOUND.to_string(), "User not found".to_string());
            }

            (OK.to_string(), "User deleted".to_string())
        }
        _ => (
            INTERNAL_SERVER_ERROR.to_string(),
            "Internal error".to_string(),
        ),
    }
}

fn set_database() -> Result<(), PostgresError> {
    let mut client = Client::connect(DB_URL, NoTls)?;
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            email VARCHAR(255) NOT NULL
        )",
    )?;
    Ok(())
}

fn get_id(request: &str) -> &str {
    request
        .split("/")
        .nth(2)
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or_default()
}

fn get_user_request_body(request: &str) -> Result<User, serde_json::Error> {
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}
