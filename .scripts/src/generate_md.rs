use std::fs::File;
use std::io::Write;

use tera::Tera;
use tera::Context;

fn main() {
    let mut tera = Tera::default();
    tera.add_template_file("src/toot.md", None).unwrap();

    let data_dir = std::fs::read_dir("../.data")
        .expect("Failed to open data directory");

    let mut count = 0;

    for entry in data_dir {

        let opath = entry.ok().map(|v| v.path());


        // Skip if we encountered an error
        if opath.is_none() {
            continue;
        }

        let path = opath.unwrap();

        // Skip if we're not looking at a JSON file
        if !path.is_file() || !path.extension().map(|x| x == "json").unwrap_or(false) {
            continue;
        }

        let contents = std::fs::read_to_string(&path)
            .expect(format!("Failed to read file {}", path.to_string_lossy()).as_ref());

        let mut json_response: serde_json::Value = serde_json::from_str(contents.as_ref())
            .expect("JSON parse error");

        let content: serde_json::Value = json_response
            .get("content")
            .expect("Failed to get content from JSON response")
            .to_owned();
        json_response.as_object_mut().unwrap().remove("content");

        let frontmatter = serde_json::to_string(&json_response)
            .expect("Failed to serialize to string");

        let mut context = Context::new();
        context.insert("frontmatter", &frontmatter);
        context.insert("body", content.as_str().unwrap());

        let output = tera.render("src/toot.md", &context)
            .expect("Failed to render template.");

        let new_path = format!("../{}.md", path.file_stem().unwrap().to_string_lossy());
        let mut file = File::create(new_path)
            .expect("Failed to create new markdown file");

        file.write_all(output.as_bytes())
            .expect("Failed to write to markdown file");

        count += 1;

    }

    println!("Successfully generated {} Markdown Files", count);

}
