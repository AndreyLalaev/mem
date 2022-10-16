use clap::Parser;
use clap_num::maybe_hex;


#[derive(Parser)]
struct Args {
    #[clap(short, long, parse(try_from_str=maybe_hex))]
    address: usize,
    #[clap(short, long, parse(try_from_str=maybe_hex))]
    value: Option<u32>,
}

fn main() {
    let args = Args::parse();

    if let Some(val) = args.value {
        if let Err(err) = mem::write(mem::DEV_MEM, args.address, val) {
            eprintln!("Error ({}) write {:#X} to {:#X}", err, val, args.address);
        }
    } else {
        match mem::read(mem::DEV_MEM, args.address) {
            Ok(value) => {
                println!("{:#X}", value);
            }
            Err(err) => {
                eprintln!("Error ({}) read {:#X} ", err, args.address);
            }
        }
    }
}
