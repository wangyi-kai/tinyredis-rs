use redis_rs::server::server::run_server;
use redis_rs::{DB_SIZE, DEFAULT_PORT};

use tokio::{net::TcpListener, signal};
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::try_init().expect("config log fail");
    let port = DEFAULT_PORT;

    // Bind a TCP listener
    let listener = TcpListener::bind(&format!("0.0.0.0:{}", port)).await.unwrap();
    print_logo();
    info!("Redis Server start");
    unsafe {
        run_server(listener, signal::ctrl_c(), DB_SIZE as u32).await;
    }
}

pub fn print_logo() {
    let version = "0.1.0";
    let port = 8000;
    let pid = std::process::id();

    let logo = format!(
        r#"
               _._
          _.-``__ ''-._
     _.-``    `.  `_.  ''-._           TinyRedis {} (custom)
 .-`` .-```.  ```\/    _.,_ ''-._
(    '      ,       .-`  | `,    )     Running in Standalone mode
|`-._`-...-` __...-.``-._|'` _.-'|     Port: {}
|    `-._   `._    /     _.-'    |     PID: {}
 `-._    `-._  `-./  _.-'    _.-'
|`-._`-._    `-.__.-'    _.-'_.-'|
|    `-._`-._        _.-'_.-'    |     github.com/wangyi-kai/tinyredis
 `-._    `-._`-.__.-'_.-'    _.-'
|`-._`-._    `-.__.-'    _.-'_.-'|
|    `-._`-._        _.-'_.-'    |
 `-._    `-._`-.__.-'_.-'    _.-'
     `-._    `-.__.-'    _.-'
         `-._        _.-'
             `-.__.-'
"#,
        version, port, pid
    );

    println!("{}", logo)
}