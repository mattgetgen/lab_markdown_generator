use inquire::Select;
use reqwest::{Client, Response};
use serde::Deserialize;
use serde_json::Value;
use std::{fmt, process};


pub enum ApiEndpoint {
    CourseList,
    AssignmentGroupList(i32),
    AssignmentList(i32, i32),
    Assignment(i32, i32),
}

impl ApiEndpoint {
    fn get_url(endpoint: ApiEndpoint) -> String {
        let uri: String = String::from("https://canvas.cse.taylor.edu/api/v1");
        match endpoint {
            ApiEndpoint::CourseList                         => format!("{uri}/courses"),
            ApiEndpoint::AssignmentGroupList(id)       => format!("{uri}/courses/{id}/assignment_groups"),
            ApiEndpoint::AssignmentList(cid, gid) => format!("{uri}/courses/{cid}/assignment_groups/{gid}/assignments"),
            ApiEndpoint::Assignment(cid, aid)     => format!("{uri}/courses/{cid}/assignments/{aid}"),
        }
    }
}

async fn response_to_json(response: Response) -> Result<Value, reqwest::Error> {
    response.json().await
}

async fn get_response(client: &Client, token: &str, endpoint: ApiEndpoint) -> Result<Value, reqwest::Error> {
    // create the API url based on the API Endpoint type.
    let url: String = ApiEndpoint::get_url(endpoint);

    // get the response from the server.
    let response = client.get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await?;

    assert!(response.status().is_success());

    response_to_json(response).await
}


/*
 * COURSE
 */

#[derive(Deserialize)]
pub struct Course {
    pub id: i32,
    // is_public_to_auth_users: bool,
    pub name: String,
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

    pub async fn get_course(client: &Client, token: &str) -> Course {
        
        let courses_json: Value = get_response(client, token, ApiEndpoint::CourseList)
            .await.unwrap_or_else(|error| {
            println!("Course Endpoint failed: {error}");
            process::exit(1);
        });
        let courses: Vec<Course> = Course::get_all_courses(courses_json);
        Course::choose_course(courses)
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
pub struct Assignment {
    pub id: i32,
    // has_submitted_submissions: bool,
    pub name: String,
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
    pub async fn get_assignment(client: &Client, token: &str, course_id: i32) -> Assignment {
        let assignment_groups_json: Value = get_response(client, token, ApiEndpoint::AssignmentGroupList(course_id))
            .await.unwrap_or_else(|error| {
            println!("Assignment Group Endpoint failed: {error}");
            process::exit(1);
        });
        if let Some(group_id) = AssignmentGroup::get_lab_group(assignment_groups_json) {

            let assignments_json: Value = get_response(client, token, ApiEndpoint::AssignmentList(course_id, group_id))
                .await.unwrap_or_else(|error| {
                println!("Assignment Endpoint failed: {error}");
                process::exit(1);
            });
            let assignments: Vec<Assignment> = Assignment::get_all_assignments(assignments_json);
            Assignment::choose_assignment(assignments)
        } else {
            println!("Error: No lab group for this class");
            process::exit(1);
        }
    }
}


#[derive(Deserialize)]
pub struct AssignmentData {
     pub description: String,
}

impl AssignmentData {
    fn get_assignment_stuff(assignment_json: Value) -> AssignmentData {
        serde_json::from_value::<AssignmentData>(assignment_json).unwrap_or_else(|error| {
            println!("Assignment may not be accesible yet.\nError: {error}");
            process::exit(1);
        })
    }

    pub async fn get_assignment_data(client: &Client, token: &str, course_id: i32, assignment_id: i32) -> AssignmentData {
        let assignment_json: Value = get_response(client, token, ApiEndpoint::Assignment(course_id, assignment_id))
            .await.unwrap_or_else(|error| {
            println!("Assignment Data Endpoint failed: {error}");
            process::exit(1);
        });
        AssignmentData::get_assignment_stuff(assignment_json)
    }
}

