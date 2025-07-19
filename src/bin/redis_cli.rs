use tracing::info;
use redis_rs::client::client::{Client, Tokens};
use redis_rs::client::config::Config;
use redis_rs::parser::cmd::command::{CommandStrategy, RedisCommand};
use redis_rs::parser::frame::Frame;
use redis_rs::Result;
use redis_rs::parser::cmd::conn::ConnCmd::{*};

pub async fn run_client() -> Result<()> {
    print_logo();
    tracing_subscriber::fmt::try_init().expect("config log fail");
    let config = Config::new(None);
    let host = config.get_value("server_ip").unwrap().trim_matches('"').to_string();
    let port = config.get_value("server_port").unwrap();

    let addr = format!("{}:{}", host, port);
    info!("<{}>", addr);
    let mut db_idx = 0;

    let mut client = Client::connect(addr.clone()).await.unwrap();
    let mut command = String::new();
    'clear: loop {
        command.clear();
        eprint!("tinyredis {}[{}]>: ", addr, db_idx);
        loop {
             std::io::stdin().read_line(&mut command).unwrap();
            if command.ends_with("\n") {
                command.remove(command.len() - 1);
            }
            if command.ends_with("\r") {
                command.remove(command.len() - 1);
            }
            // if !command.ends_with(";") {
            //     command.push_str("\n");
            //     continue 'cmd;
            // }
            let tokens = Tokens::from(&command);
            let cmd = match tokens.to_command() {
                Ok(cmd) => {
                    cmd
                }
                Err(e) => {
                    println!("{}, please input command again.", e);
                    continue 'clear;
                }
            };
            let tmp_idx = match &cmd {
                RedisCommand::Connection(Select { index}) => *index,
                _ => db_idx,
            };
            let frame = cmd.into_frame();
            let _ = client.conn.write_frame(&frame).await;
            let res = client.conn.read_frame().await;
            match res {
                Ok(res) => {
                    if let Some(res) = res {
                        db_idx = tmp_idx;
                        println!("{}", res);
                    } else {
                        println!("client quit");
                        return Ok(());
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
(    '      ,       .-`  | `,    )     Running in single mode 🚀
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
async fn main() -> Result<()> {
    run_client().await
}