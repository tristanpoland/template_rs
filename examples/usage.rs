use template_rs::Template;

fn main() {
    // Usage example:
    let mut template = Template::from_file("./examples/hello_world.tmrs").expect("Failed to load template");
    template.set("greeting", "Hello").expect("Failed to set greeting placeholder");
    template.set("name", "World").expect("Failed to set name placeholder");
}