use askama::Template;

#[derive(Template)]
#[template(source = "Hello [% name %]! This should work: ${{ github.repository_owner }}", ext = "txt", syntax = "custom")]
struct TestTemplate {
    name: String,
}

fn main() {
    let tmpl = TestTemplate {
        name: "World".to_string(),
    };
    println!("{}", tmpl.render().unwrap());
}