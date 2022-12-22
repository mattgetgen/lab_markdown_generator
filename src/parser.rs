use html_parser::Dom;
use serde::Deserialize;
use serde_json::Value;

const H1_KEY: &str = "h1";
const OL_KEY: &str = "ol";
const UL_KEY: &str = "ul";
const LI_KEY: &str = "li";
const P_KEY: &str = "p";
const EM_KEY: &str = "em";
const CODE_KEY: &str = "code";

#[derive(Deserialize)]
struct BaseHtml {
    children: Vec<Value>,
}

#[derive(Deserialize)]
struct ChildHtml {
    children: Vec<Value>,
    name: String,
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
                markdown_str.push_str(&format!("{}{}. ", num_tabs, n));
            },
            ListType::Unordered {indent: i } => {
                let num_tabs: String = String::from("    ").repeat(i);
                markdown_str.push_str(&format!("\n{}- ", num_tabs));
            }
        }
    }

    fn get_indent(list_type: ListType) -> usize {
        match list_type {
            ListType::Ordered {indent: i, num: _} => i,
            ListType::Unordered { indent: i } => i,
        }
    }

    fn increment_list_num(list_type: ListType) -> ListType {
        match list_type {
            ListType::Ordered {indent: i, num: n} => ListType::Ordered {indent: i, num: n+1},
            ListType::Unordered { indent: _ } => list_type,
        }
    }
}


fn create_markdown_header(assignment_name: &str, user_name: &str) -> String {
    format!("# {}\n#### _By {}_\n\n", assignment_name, user_name)
}

fn is_question_header(child_dom: &ChildHtml) -> bool {
    for child in child_dom.children.iter() {
        if let Some(val) = child.as_str() {
            if val == "Turn In" || val == "Questions" {
                return true;
            }
        }
    }
    false
}

// Questions are always Ordered lists in my experience. Otherwise, I'll have to refactor...
fn parse_questions(markdown_str: &mut String, question_head: ChildHtml) {
    markdown_str.push_str("\n## Questions\n");
    
    println!("{}", question_head.name);
    let mut ordered_list: ListType = ListType::Ordered { indent: 0, num: 1 };
    for child in question_head.children {
        if let Ok(child_dom) = serde_json::from_value::<ChildHtml>(child) {
            if child_dom.name == LI_KEY {
                parse_list_line(markdown_str, child_dom, ordered_list, true);
                markdown_str.push_str("\n\n");
                ordered_list = ListType::increment_list_num(ordered_list);
            }
        }
    }
}

fn parse_list_line(markdown_str: &mut String, line_head: ChildHtml, list_type: ListType, add_newline: bool) {
    if add_newline && line_head.name != UL_KEY && line_head.name != OL_KEY {
        ListType::append_list_string(markdown_str, list_type);
    }
    
    println!("line: {}\t{}", line_head.name, ListType::get_indent(list_type));

    for child in line_head.children {
        if child.is_string() {
            if let Some(question) = child.as_str() {
                markdown_str.push_str(question);
            }
        } else if child.is_object() {
            if let Ok(child_dom) = serde_json::from_value::<ChildHtml>(child) {
                
                if child_dom.name == P_KEY {
                    // should just be appended as the question, resend it.
                    parse_list_line(markdown_str, child_dom, list_type, false);

                } else if child_dom.name == LI_KEY {
                    // a list object, probably for a new list. This depends on it's parent list type.
                    let new_list_type: ListType = ListType::increment_list_num(list_type);
                    parse_list_line(markdown_str, child_dom, new_list_type, true);

                } else if child_dom.name == CODE_KEY {
                    // surround a code section with `` (code block in markdown).
                    markdown_str.push_str(" `");
                    parse_list_line(markdown_str, child_dom, list_type, false);
                    markdown_str.push_str("` ");

                } else if child_dom.name == EM_KEY {
                    // surround a code section with __ (italics in markdown).
                    markdown_str.push_str(" _");
                    parse_list_line(markdown_str, child_dom, list_type, false);
                    markdown_str.push_str("_ ");

                } else if child_dom.name == UL_KEY {
                    // indent and append the new list
                    let i: usize = ListType::get_indent(list_type);
                    parse_list_line(markdown_str, child_dom, ListType::Unordered{indent: i+1}, true);

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

    // convert html DOM string into a JSON string.
    let json_string: String = Dom::parse(doc).unwrap_or_else(|error| {
        panic!("{}", error);
    }).to_json_pretty().unwrap_or_else(|error| {
        panic!("{}", error);
    });
    
    let base_dom: BaseHtml = serde_json::from_str(&json_string).unwrap_or_else(|error| {
        panic!("Invalid Base DOM: {}", error);
    });

    let mut parse_next_list: bool = false;
    for child in base_dom.children {
        if let Ok(child_dom) = serde_json::from_value::<ChildHtml>(child) {
            if child_dom.name == H1_KEY && is_question_header(&child_dom) {
                println!("{}", child_dom.name);
                parse_next_list = true;
            }
            if parse_next_list && child_dom.name == OL_KEY {
                parse_questions(&mut markdown_str, child_dom);
            }
        }
    }
    markdown_str

}
