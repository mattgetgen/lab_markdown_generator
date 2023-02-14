use html_parser::Dom;
use serde::Deserialize;
use serde_json::Value;
use std::process;

const H1_KEY: &str = "h1";
const OL_KEY: &str = "ol";
const UL_KEY: &str = "ul";
const LI_KEY: &str = "li";
const P_KEY: &str = "p";
const EM_KEY: &str = "em";
const CODE_KEY: &str = "code";

#[derive(Deserialize)]
struct BaseHtml {
    // base html only has children.
    children: Vec<Value>,
}

impl BaseHtml {
    fn convert_doc_to_struct(doc: &str) -> BaseHtml {

        let json_string: String = Dom::parse(doc).unwrap_or_else(|error| {
            println!("Couldn't parse the HTML Document: {error}");
            process::exit(1);
        }).to_json().unwrap_or_else(|error| {
            println!("Couldn't parse the Dom Object into JSON: {error}");
            process::exit(1);
        });
    
        serde_json::from_str::<BaseHtml>(&json_string).unwrap_or_else(|error| {
            println!("Couldn't parse the JSON into a valid BaseHtml object: {error}");
            process::exit(1);
        })
    }
}

#[derive(Deserialize)]
struct ChildHtml {
    // child html have children and a name.
    children: Vec<Value>,
    name: String,
}

impl ChildHtml {
    fn is_question_header(&self) -> bool {
        //self.children.iter();
        for child in self.children.iter() {
            if let Some(val) = child.as_str() {
                if val == "Turn In" || val == "Questions" {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(PartialEq, Clone, Copy)]
enum ListType {
    Ordered { indent: usize, num: i32 },
    Unordered { indent: usize },
}

impl ListType {
    fn append_list_string(markdown_str: &mut String, list_type: ListType) {
        match list_type {
            ListType::Ordered {indent: i, num: n} => {
                if i == 0 {
                    markdown_str.push('\n');
                }
                let num_tabs: String = String::from("    ").repeat(i);
                markdown_str.push_str(&format!("{num_tabs}{n}. "));
            },
            ListType::Unordered {indent: i } => {
                let num_tabs: String = String::from("    ").repeat(i);
                markdown_str.push_str(&format!("\n{num_tabs}- "));
            }
        }
    }

    fn get_indent(&self) -> usize {
        match *self {
            ListType::Ordered {indent: i, num: _} => i,
            ListType::Unordered { indent: i } => i,
        }
    }

    fn increment_list_num(&mut self) {
        *self = match *self {
            ListType::Ordered {indent: i, num: n} => ListType::Ordered {indent: i, num: n+1},
            ListType::Unordered { indent: _ } => *self,
        };
    }
}


fn create_markdown_header(assignment_name: &str, user_name: &str) -> String {
    format!("# {assignment_name}\n#### _By {user_name}_\n\n")
}

// Questions are always Ordered lists in my experience. Otherwise, I'll have to make some changes...
fn parse_questions(markdown_str: &mut String, question_head: ChildHtml) {
    markdown_str.push_str("\n## Questions\n");
    
    let mut ordered_list: ListType = ListType::Ordered { indent: 0, num: 1 };
    for child in question_head.children {
        if let Ok(child_dom) = serde_json::from_value::<ChildHtml>(child) {
            if child_dom.name == LI_KEY {
                parse_list_line(markdown_str, child_dom, ordered_list);
                markdown_str.push_str("\n\n");
                ordered_list.increment_list_num();
            }
        }
    }
}

fn parse_list_line(markdown_str: &mut String, line_head: ChildHtml, mut list_type: ListType) {
    if line_head.name == LI_KEY {
        ListType::append_list_string(markdown_str, list_type);
    }
    
    for child in line_head.children {
        if child.is_string() {
            if let Some(question) = child.as_str() {
                markdown_str.push_str(question);
            }
        } else if child.is_object() {
            if let Ok(child_dom) = serde_json::from_value::<ChildHtml>(child) {
                
                if child_dom.name == P_KEY {
                    // should just be appended as the question, resend it.
                    parse_list_line(markdown_str, child_dom, list_type);

                } else if child_dom.name == LI_KEY {
                    // a list object, probably for a new list. This depends on it's parent list type.
                    parse_list_line(markdown_str, child_dom, list_type);
                    list_type.increment_list_num();

                } else if child_dom.name == CODE_KEY {
                    // surround a code section with `` (code block in markdown).
                    markdown_str.push_str(" `");
                    parse_list_line(markdown_str, child_dom, list_type);
                    markdown_str.push_str("` ");

                } else if child_dom.name == EM_KEY {
                    // surround a code section with __ (italics in markdown).
                    markdown_str.push_str(" _");
                    parse_list_line(markdown_str, child_dom, list_type);
                    markdown_str.push_str("_ ");

                } else if child_dom.name == UL_KEY {
                    // indent and append the new list.
                    let i: usize = list_type.get_indent();
                    parse_list_line(markdown_str, child_dom, ListType::Unordered{indent: i+1});

                } else if child_dom.name == OL_KEY {
                    // indent and search through it.
                    let i: usize = list_type.get_indent();
                    let new_list_type: ListType = ListType::Ordered {indent: i+1, num: 1};
                    parse_list_line(markdown_str, child_dom, new_list_type);

                } else {
                    println!("Unhandled html element: {}", child_dom.name);
                    println!("\tValue: {:?}", child_dom.children);
                }
            }
        }
    }
}

pub fn create_markdown(doc: &str, user_name: &str, assignment_name: String) -> String {
    let mut markdown_str: String = create_markdown_header(&assignment_name, user_name);

    let base_dom: BaseHtml = BaseHtml::convert_doc_to_struct(doc);

    let mut parse_next_list: bool = false;
    for child in base_dom.children {
        if let Ok(child_dom) = serde_json::from_value::<ChildHtml>(child) {
            if child_dom.name == H1_KEY && child_dom.is_question_header() {
                parse_next_list = true;
            }
            // if a question section ever has an unordered list start, edit it here.
            if parse_next_list && child_dom.name == OL_KEY {
                parse_questions(&mut markdown_str, child_dom);
            }
        }
    }
    markdown_str

}

