use tracing::info;
use redis_rs::client::client::{Client, Tokens};
use redis_rs::client::config::Config;

pub async fn run_client() {
    print_logo();
    tracing_subscriber::fmt::try_init().expect("config log fail");
    let config = Config::new(None);
    let host = config.get_value("server_ip").unwrap().trim_matches('"').to_string();
    let port = config.get_value("server_port").unwrap();

    let addr = format!("{}:{}", host, port);
    info!("<{}>", addr);

    let mut client = Client::connect(addr.clone()).await.unwrap();
    let mut command = String::new();
    'clear: loop {
        command.clear();
        println!("Please input command: ");
        'cmd: loop {
             std::io::stdin().read_line(&mut command).unwrap();
            if command.ends_with("\n") {
                command.remove(command.len() - 1);
            }
            if command.ends_with("\r") {
                command.remove(command.len() - 1);
            }
            if !command.ends_with(";") {
                command.push_str("\n");
                continue 'cmd;
            }
            let tokens = Tokens::from(&command);
            let cmd = tokens.to_command().unwrap();
            let frame = cmd.into_frame();
            let _ = client.conn.write_frame(&frame).await;
            let res = client.conn.read_frame().await;
            match res {
                Ok(res) => {
                    if let Some(res) = res {
                        println!("{}", res);
                    } else {
                        println!("receive fail");
                    }
                },
                Err(e) => println!("error: {}", e),
            };
            continue 'clear;
        }
    }
}

fn print_logo() {
    let version = "0.1.0";
    let port = 8000;
    let pid = std::process::id();

    let logo = format!(
        r#"
               _._
          _.-``__ ''-._
     _.-``    `.  `_.  ''-._           tinyredis {} (custom) 🦀
 .-`` .-```.  ```\/    _.,_ ''-._
(    '      ,       .-`  | `,    )     Running in tiny mode 🚀
|`-._`-...-` __...-.``-._|'` _.-'|     Port: {}
|    `-._   `._    /     _.-'    |     PID: {}
 `-._    `-._  `-./  _.-'    _.-'
|`-._`-._    `-.__.-'    _.-'_.-'|
|    `-._`-._        _.-'_.-'    |  github.com/wangyi-kai/tinyredis
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

#[tokio::main]
async fn main() {
    run_client().await;
}