use inquire::{Confirm, Select};
use std::io::Write;
pub use std::process;
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;
use reqwest::{ClientBuilder, Client};

mod requester;
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

#[derive(Deserialize)]
struct Course {
    id: i32,
    // is_public_to_auth_users: bool,
    name: String,
}

impl fmt::Display for Course {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Course {
    fn get_all_courses(courses_json: Value) -> Vec<Course> {
        let mut courses: Vec<Course> = Vec::new();
        if let Value::Array(items) = courses_json {
            // parse all courses.
            for item in items.into_iter() {
                if let Ok(course) = serde_json::from_value::<Course>(item) {
                    // use this when I actually have public courses.
                    // if course.is_public_to_auth_users {
                    //     courses.push(course);
                    // }
                    // for now, just push all :(
                    courses.push(course);
                }
            }
        }
        courses
    }

    fn choose_course(courses: Vec<Course>) -> Course {
        Select::new("Which course would you like to select?", courses)
            .prompt()
            .unwrap_or_else(|_| {
            println!("Exiting.");
            process::exit(1);
        })
    }
}

#[derive(Deserialize)]
struct AssignmentGroup {
    id: i32,
    name: String,
}

impl fmt::Display for AssignmentGroup {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl AssignmentGroup {
    fn get_lab_group(assignment_group_json: Value) -> Option<i32> {
        let mut assignment_groups: Vec<AssignmentGroup> = Vec::new();
        if let Value::Array(items) = assignment_group_json {
            for item in items.into_iter() {
                if let Ok(group) = serde_json::from_value::<AssignmentGroup>(item) {
                    assignment_groups.push(group);
                }
            }
            if let Some(group) = assignment_groups.into_iter().find(|x| x.name == "Labs & Homework") {
                return Some(group.id);
            }
        }
        None
    }
}

#[derive(Deserialize)]
struct Assignment {
    id: i32,
    // has_submitted_submissions: bool,
    name: String,
}

impl fmt::Display for Assignment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Assignment {
    fn get_all_assignments(assignments_json: Value) -> Vec<Assignment> {
        let mut assignments: Vec<Assignment> = Vec::new();
        if let Value::Array(items) = assignments_json {
            // parse all assignments.
            for item in items.into_iter() {
                if let Ok(assignment) = serde_json::from_value::<Assignment>(item) {
                    /*
                    For now, append all assignments.
                    Later, use has_submitted_submissions
                    to check if there have been any submissions.
                    There may be another value to track instead.
                    For now, add all assignments.
                     */
                    assignments.push(assignment);
                }
            }
        }
        assignments
    }

    fn choose_assignment(assignments: Vec<Assignment>) -> Assignment {
        let answer: Assignment = Select::new("Which assignment would you like to make a lab note for?", assignments)
            .prompt()
            .unwrap_or_else(|_| {
            println!("Exiting.");
            process::exit(1);
        });
        answer
    }
}


#[derive(Deserialize)]
struct AssignmentData {
     description: String,
}

impl AssignmentData {
    fn get_assignment_data(assignment_json: Value) -> AssignmentData {
        serde_json::from_value::<AssignmentData>(assignment_json.clone()).unwrap_or_else(|error| {
            println!("Invalid Response Body from canvas!: {error}\n with JSON: {assignment_json}");
            process::exit(1);
        })
    }
}

async fn get_course(client: &Client, token: &str) -> Course {
    let courses_json: Value = requester::get_response(client, token, requester::ApiEndpoint::CourseList).await;
    let courses: Vec<Course> = Course::get_all_courses(courses_json);
    Course::choose_course(courses)
}

async fn get_assignment(client: &Client, token: &str, course_id: i32) -> Assignment {
    let assignment_groups_json: Value = requester::get_response(client, token, requester::ApiEndpoint::AssignmentGroupList(course_id)).await;
    if let Some(group_id) = AssignmentGroup::get_lab_group(assignment_groups_json) {

        let assignments_json: Value = requester::get_response(client, token, requester::ApiEndpoint::AssignmentList(course_id, group_id)).await;
        let assignments: Vec<Assignment> = Assignment::get_all_assignments(assignments_json);
        Assignment::choose_assignment(assignments)
    } else {
        println!("Error: No lab group for this class");
        process::exit(1);
    }
}

async fn get_assignment_data(client: &Client, token: &str, course_id: i32, assignment_id: i32) -> AssignmentData {
    let assignment_json: Value = requester::get_response(client, token, requester::ApiEndpoint::Assignment(course_id, assignment_id)).await;
    AssignmentData::get_assignment_data(assignment_json)
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
    let course: Course = get_course(client, &local_data.token).await;
    let assignment: Assignment = get_assignment(client, &local_data.token, course.id).await;
    let assignment_data: AssignmentData = get_assignment_data(client, &local_data.token, course.id, assignment.id).await;
    
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

