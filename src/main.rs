mod methods;

use clap::{Parser, ValueEnum};

#[derive(ValueEnum, Clone, Debug)]
enum Action {
    INIT,
    APPLY,
    ADD,
    REMOVE,
}

#[derive(Parser, Debug)]
#[command(name = "dots", version)]
struct Args {
    #[arg(short, long)]
    action: Action,
    #[arg(short, long, required = false)]
    file: Option<String>,
}

fn main() {
    let args = Args::parse();

    match args.action {
        Action::INIT => methods::init(),
        Action::APPLY => methods::apply(),
        Action::ADD => methods::add(args.file),
        Action::REMOVE => println!("remove dotfiles"),
    }
}
