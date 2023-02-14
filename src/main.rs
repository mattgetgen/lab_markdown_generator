use inquire::{Confirm, Select};
use reqwest::{ClientBuilder, Client};
use serde::Deserialize;
use std::{fmt, fs, process};
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::time::Duration;

mod requester;
use requester::{Course, Assignment, AssignmentData};
mod parser;

enum MenuOption {
    NewLabNote,
    SubmitLabNote,
}

impl fmt::Display for MenuOption {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MenuOption::NewLabNote => write!(f, "New Lab Note"),
            MenuOption::SubmitLabNote => write!(f, "Submit Lab Note"),
        }
    }
}

impl MenuOption {
    fn main_menu() -> MenuOption {
        println!("--- MAIN MENU ---");

        let main_options: Vec<MenuOption> = vec!{
            MenuOption::NewLabNote,
            MenuOption::SubmitLabNote,
        };

        let answer: MenuOption = Select::new("What would you like to do?", main_options)
            .prompt()
            .unwrap_or_else(|_| {
            println!("Exiting.");
            process::exit(1);
        });

        answer
    }
}

#[derive(Deserialize)]
struct LocalData {
    name: String,
    token: String,
    base_dir: String,
}

impl LocalData {
    fn get_local_data() -> LocalData {

        let local_file: String = String::from("local.json");

        // open the JSON file and read it into a string.
        let local_data: String = fs::read_to_string(&local_file)
            .unwrap_or_else(|error| {
            if error.kind() == ErrorKind::NotFound {
                println!("File `{local_file}` not found!: {error}");
                println!("\nPlease create a `{local_file}` file with this format:
                \n{{
                    \n\t\"name\": \"<your name here>\",
                    \n\t\"token\": \"<token from canvas here>\",
                    \n\t\"base_dir\": \"<full directory path for storing lab file here>\"
                \n}}");
                process::exit(1);
            } else {
                println!("Problem opening the file: {error:?}");
                process::exit(1);
            }
        });

        // make sure the JSON is the correct format.
        serde_json::from_str(&local_data).unwrap_or_else(|error| {
            println!("Invalid JSON format in `{local_file}`: {error}.");
            println!("\nPlease make sure the `{local_file}` file is formatted like this: 
            \n{{
                \n\t\"name\": \"<your name here>\",
                \n\t\"token\": \"<token from canvas here>\",
                \n\t\"base_dir\": \"<full directory path for storing lab file here>\"
                \n}}");
            process::exit(1);
        })
    }
}

fn should_create_dir(path: &str) -> bool {
    let ans = Confirm::new(&format!("{path} directory doesn't exist, do you want to create it?"))
        .with_default(true)
        .prompt();
    matches!(ans, Ok(true))
}

fn create_dir(dir: &Path, content: &str) {
    let path_name = dir.to_str().unwrap_or("");

    match should_create_dir(path_name) {
        // try and create it that bad boy.
        true => {
            fs::create_dir_all(dir).unwrap_or_else(|e| {
                // error creating the dir, just print the content.
                println!("Error creating directory: {e}");
                println!("lab note content:\n{content}");
                process::exit(1);
            });
            println!("Successfully created directory!");
        },
        // don't try to create it, just print the content.
        false => {
            println!("lab note content:\n{content}");
            process::exit(0);
        },
    }
}

fn should_overwrite_file(file_path: &str) -> bool {
    let ans = Confirm::new(&format!("{file_path} already exists, do you want to overwrite it?"))
        .with_default(false)
        .prompt();
    matches!(ans, Ok(true))
}

fn create_file(file_path: String, content: &str) -> Option<String> {

    let path: &Path = Path::new(&file_path);
    // file already exists and the user doesn't want to overwrite it, do nothing.
    if path.is_file() && !should_overwrite_file(&file_path) {
        return Some(file_path);
    }

    let mut file: fs::File = fs::File::create(path).unwrap_or_else(|e| {
        // error creating the file, just print the content.
        println!("Error creating file: {e}");
        println!("lab note content:\n{content}");
        process::exit(1);
    });

    if let Err(e) = file.write_all(content.as_bytes()) {
        // error writing to the file, just print the content.
        println!("Error writing to the file: {e}");
        println!("lab note content:\n{content}");
        process::exit(1);
    };

    Some(file_path)
}

async fn handle_new_lab_note(client: &Client, local_data: &LocalData) -> Option<String> {
    let course: Course = Course::get_course(client, &local_data.token).await;
    let assignment: Assignment = Assignment::get_assignment(client, &local_data.token, course.id).await;
    let assignment_data: AssignmentData = AssignmentData::get_assignment_data(client, &local_data.token, course.id, assignment.id).await;
    
    let content: String = parser::create_markdown(&assignment_data.description, &local_data.name, assignment.name.clone());
    let course_dir: String = local_data.base_dir.to_owned() + &course.name[..7].trim().to_lowercase().replace(' ', "") + "/lab/";
    let file_name: String = assignment.name.trim().to_lowercase().replace(' ', "_") + ".md";

    let dir: &Path = Path::new(&course_dir);
    // dir doesn't exist, create it.
    if !dir.is_dir() {
        create_dir(dir, &content);
    }

    let file_path: String = format!("{course_dir}/{file_name}");

    create_file(file_path, &content)
}

#[tokio::main]
async fn main() {
    let local_data: LocalData = LocalData::get_local_data();
    if local_data.base_dir.contains('~') {
        println!("Please use the full path to the lab note directory");
        process::exit(1);
    }
    // create the client
    let timeout: Duration = Duration::new(5, 0);
    let client: Client = ClientBuilder::new()
        .timeout(timeout)
        .build()
        .unwrap_or_else(|error| {
        println!("Error builing the client: {error:?}");
        process::exit(1);
    });

    let option: MenuOption = MenuOption::main_menu();
    match option {
        MenuOption::NewLabNote => {
            if let Some(file_path) = handle_new_lab_note(&client, &local_data).await {
                std::process::Command::new("/usr/bin/sh")
                    .arg("-c")
                    .arg(format!("vim {file_path}"))
                    .spawn()
                    .expect("Error: Failed to run editor")
                    .wait()
                    .expect("Error: Editor returned a non-zero status");
            }
        },
        _ => {
            println!("Sorry! Working on the implementation for this...");
        },
    }   
    
}

