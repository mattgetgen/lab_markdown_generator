use inquire::Select;
pub use std::process;
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;
use reqwest::{ClientBuilder, Client};

mod requester;
mod parser;

enum MenuOption {
    NewLabNote,
    SubmitLabNote,
    SubmitAssignment,
}

impl fmt::Display for MenuOption {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MenuOption::NewLabNote => write!(f, "New Lab Note"),
            MenuOption::SubmitLabNote => write!(f, "Submit Lab Note"),
            MenuOption::SubmitAssignment => write!(f, "Submit Assignment"),
        }
    }
}

impl MenuOption {
    fn main_menu() -> MenuOption {
        println!("--- MAIN MENU ---");

        let main_options: Vec<MenuOption> = vec!{
            MenuOption::NewLabNote,
            MenuOption::SubmitLabNote,
            MenuOption::SubmitAssignment,
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
}

impl LocalData {
    fn get_local_data() -> LocalData {

        let local_file: String = String::from("local.json");

        // open the JSON file and read it into a string.
        let local_data: String = fs::read_to_string(&local_file)
            .unwrap_or_else(|error| {
            if error.kind() == ErrorKind::NotFound {
                println!("File `{}` not found!: {}", local_file, error);
                println!("\nPlease create a `{}` file with this format:\n{{\n\t\"name\": \"<your name here>\",\n\t\"token\": \"<token from canvas here>\"\n}}", local_file);
                process::exit(1);
            } else {
                println!("Problem opening the file: {:?}", error);
                process::exit(1);
            }
        });

        // make sure the JSON is the correct format.
        serde_json::from_str(&local_data).unwrap_or_else(|error| {
            println!("Invalid JSON format in `{}`: {}.", local_file, error);
            println!("\nPlease make sure the `{}` file is formatted like this: \n{{\n\t\"name\": \"<your name here>\",\n\t\"token\": \"<token from canvas here>\"\n}}", local_file);
            process::exit(1);
        })
    }
}

#[derive(Deserialize)]
struct Course {
    id: i32,
    is_public: bool,
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
                    /*if course.is_public {
                        courses.push(course);
                    }*/
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
struct Assignment {
    id: i32,
    has_submitted_submissions: bool,
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
            println!("Invalid Response Body from canvas!: {}\n with JSON: {}", error, assignment_json);
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
    let assignments_json: Value = requester::get_response(client, token, requester::ApiEndpoint::AssignmentList(course_id)).await;
    let assignments: Vec<Assignment> = Assignment::get_all_assignments(assignments_json);
    Assignment::choose_assignment(assignments)
}

async fn get_assignment_data(client: &Client, token: &str, course_id: i32, assignment_id: i32) -> AssignmentData {
    let assignment_json: Value = requester::get_response(client, token, requester::ApiEndpoint::Assignment(course_id, assignment_id)).await;
    AssignmentData::get_assignment_data(assignment_json)
}

#[tokio::main]
async fn main() {
    let local_data: LocalData = LocalData::get_local_data();
    // create the client
    let timeout: Duration = Duration::new(5, 0);
    let client: Client = ClientBuilder::new()
        .timeout(timeout)
        .build()
        .unwrap_or_else(|error| {
        println!("Error builing the client: {:?}", error);
        process::exit(1);
    });

    let option: MenuOption = MenuOption::main_menu();
    match option {
        MenuOption::NewLabNote => {
            let course: Course = get_course(&client, &local_data.token).await;
            let assignment: Assignment = get_assignment(&client, &local_data.token, course.id).await;
            let assignment_data: AssignmentData = get_assignment_data(&client, &local_data.token, course.id, assignment.id).await;
            println!("\n{}", parser::create_markdown(&assignment_data.description, &local_data.name, assignment.name));
        },
        _ => {
            println!("Sorry! Working on the implementation for this...");
        },
    }   
    
}

