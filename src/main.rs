use std::env;
use std::path::PathBuf;

use clap::{Arg, App, ArgAction, value_parser};

pub mod lib;

fn retrieve_env_var(key: &str) -> String{
    env::var(key).expect(&format!("The environment variable \"{}\" must be fixed and be in UTF-8.", key))
}

fn main() {

    let matches = App::new("XAnnuaire dump")
        .about("Retrieves data from the XAnnuaire into a csv file. The environment variables XANNUAIRE_USERNAME and\
        XANNUAIRE_PASSWORD must contain your login information")
        .arg(
            Arg::new("brief")
                .short('b')
                .long("brief")
                .action(ArgAction::SetTrue)
                .help("In brief mode, the program makes far fewer queries but only retrieves the following elements: uid, names, attachment structures and phone numbers (if available). The email addresses can be deduced from the uids.")
        )
        .arg(
            Arg::new("slow")
                .short('s')
                .long("slow")
                .action(ArgAction::SetTrue)
                .help("In slow mode, the program waits a few seconds between each of these requests. This allows not to overload the server and to steal data in a more discrete way.")
        )
        .arg(
            Arg::new("filename")
                .required(false)
                .value_parser(value_parser!(PathBuf))
                .default_value("xannuaire.csv")
                .help("Name of the csv file, can be a path.")

        )
        .get_matches();


    let brief_mode = matches.get_flag("brief") as bool;
    let slow_mode = matches.get_flag("slow") as bool;
    let filename = matches.get_one::<PathBuf>("filename").unwrap();

    let username = retrieve_env_var("XANNUAIRE_USERNAME");
    let password = retrieve_env_var("XANNUAIRE_PASSWORD");

    crate::lib::main(&username, &password, brief_mode, slow_mode, filename);
}
