use clap::Parser;
use std::io::{BufRead, BufReader, stdout};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::{env, thread};
use std::fs::OpenOptions;
use std::io::prelude::*;
use encoding_rs::*;
use encoding_rs::Encoding;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[clap(trailing_var_arg = true)]
struct RunConfigArgs {
    command: Vec<String>,
    #[arg(short, long)]
    shell: Option<String>,
    #[arg(short, long)]
    path: Option<String>,
    #[arg(short, long)]
    log_file_name: Option<String>,
    #[arg(short, long)]
    encoding: Option<String>,
}


#[derive(Debug)]
struct RunConfig {
    command: String,
    shell: String,
    path: String,
    log_file_name: String,
    encoding: &'static Encoding,
}

fn main() {
    println!("{}", env::consts::OS);
    let args = RunConfigArgs::parse();
    println!("{:?}", args);
    println!("{:?}", get_config(&args));
    let run_config = get_config(&args);
    loop {
        let mut result_status = run_command(&run_config);
        println!("result_status: {:?}", result_status);
        match result_status.try_wait() {
            Ok(Some(status)) => {
                println!("exited with: {status}");
            }
            Ok(None) => {
                // println!("status not ready yet, let's really wait");
                let res = result_status.wait();
                println!("result: {res:?}");
                if res.is_ok_and(|r| r.code() == Option::from(0)) {
                    println!("is_ok : ok");
                    break;
                }
            }
            Err(e) => {
                println!("error attempting to wait: {e}");
            }
        }
        println!("restart");
    }
    println!("end");
}

fn get_config(args: &RunConfigArgs) -> RunConfig {
    let default_shell = match env::consts::OS {
        "windows" => String::from("cmd"),
        _ => String::from("sh")
    };
    let current_path = env::current_dir().unwrap();
    let final_config = RunConfig {
        shell: match &args.shell {
            None => { default_shell }
            Some(command_line_shell) => { String::from(command_line_shell) }
        },
        path: match &args.path {
            None => { current_path.display().to_string() }
            Some(path) => { String::from(path) }
        },
        command: String::from(&args.command.join(" ")),
        log_file_name: match &args.log_file_name {
            None => { "stillrun.log".to_string() }
            Some(log_file_name) => { String::from(log_file_name) }
        },
        encoding: match &args.encoding {
            None => {
                match env::consts::OS {
                    "windows" => GBK,
                    _ => UTF_8
                }
            }
            Some(arg_encoding) => {
                match Encoding::for_label(arg_encoding.as_ref()) {
                    None => {
                        match env::consts::OS {
                            "windows" => GBK,
                            _ => UTF_8
                        }
                    }
                    Some(encoding_method) => encoding_method
                }
            }
        },
    };
    final_config
}


fn run_command(run_config: &RunConfig) -> Child {
    let command_line_shell = &run_config.shell;
    let current_path = &run_config.path;
    let config_encoding = run_config.encoding;
    println!("The current directory is {}", current_path);

    let argument = match env::consts::OS {
        "windows" => ["/C", &run_config.command],
        _ => ["-c", &run_config.command]
    };

    let log_file_name = &run_config.log_file_name;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(log_file_name)
        .unwrap();

    let mut command_child = Command::new(command_line_shell)
        .args(argument)
        .current_dir(current_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn().unwrap();
    println!("child.id: {}", command_child.id());
    let stdout = command_child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);
    let mut lines = reader.split(b'\n').map(|l| l.unwrap());
    lines.for_each(|line| {
        let (decoded_string, _, _) = config_encoding.decode(&*line);
        println!("{}", decoded_string);
        if let Err(e) = writeln!(file, "{}", decoded_string) {
            eprintln!("Couldn't write to file: {}", e);
        }
    });

    let stderr = command_child.stderr.take().unwrap();
    let reader_stderr = BufReader::new(stderr);
    let mut lines_stderr = reader_stderr.split(b'\n').map(|l| l.unwrap());
    lines_stderr.for_each(|line| {
        let (decoded_string, _, _) = config_encoding.decode(&*line);
        println!("{}", decoded_string);
        if let Err(e) = writeln!(file, "{}", decoded_string) {
            eprintln!("Couldn't write to file: {}", e);
        }
    });
    command_child
}
