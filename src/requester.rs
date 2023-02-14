use reqwest::{Client, Response};
use serde_json::Value;


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

async fn response_to_json(response: Response) -> Value {
    response
        .json()
        .await
        .unwrap_or_else(|error| {
        panic!("JSON parsing error: {error:?}")
    })
}

pub async fn get_response(client: &Client, token: &str, endpoint: ApiEndpoint) -> Value {
    // create the API url based on the API Endpoint type.
    let url: String = ApiEndpoint::get_url(endpoint);

    // get the response from the server.
    let response: Response = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap_or_else(|error| {
        panic!("Response from server failed: {error:?}");
    });

    assert!(response.status().is_success());

    response_to_json(response).await
}

