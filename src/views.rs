use diesel::sql_types::{Text, Integer};
use rocket::fs::NamedFile;
use slugify::slugify;
extern crate diesel;
extern crate rocket;
use diesel::sqlite::SqliteConnection;
use diesel::{prelude::*, sql_query};
use dotenvy::dotenv;
use pandoc::PandocOutput;
use rocket::response::{status::Created, Debug};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{get, post };
use crate::models;
use crate::schema;
use rocket_dyn_templates::{context, Template};
use std::collections::HashMap;
use std::env;
use std::path::{PathBuf, Path};
use diesel::sql_types::{Nullable};
use diesel::{prelude::*};
use slab_tree::*;

type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

#[derive(Serialize, Deserialize)]
pub struct NewPage {
    parent_id: Option<i32>,
    title: String,
    org_content: String,
}

#[post("/pages", format = "json", data = "<page>")]
pub fn create_page(page: Json<NewPage>) -> Result<Created<Json<NewPage>>> {
    use self::schema::pages::dsl::*;
    use models::Page;
    let connection = &mut establish_connection();

    let mut pandoc = pandoc::new();
    pandoc.set_input(pandoc::InputKind::Pipe(page.org_content.to_string()));
    pandoc.set_output(pandoc::OutputKind::Pipe);
    pandoc.set_input_format(pandoc::InputFormat::Org, Vec::new());
    pandoc.set_output_format(pandoc::OutputFormat::Html5, Vec::new());
    let new_html_content = pandoc.execute().expect("Error converting org to html");
    let new_html_content = match new_html_content {
        PandocOutput::ToFile(pathbuf) => {panic!()},
        PandocOutput::ToBuffer(string) => {string},
        PandocOutput::ToBufferRaw(vec) => {panic!()}
    };

    let new_slug = slugify!(&page.title.to_string());

    let new_page = Page {
        id: None,
        parent_id: page.parent_id,
        title: page.title.to_string(),
        slug: new_slug,
        html_content: new_html_content,
    };

    diesel::insert_into(self::schema::pages::dsl::pages)
        .values(&new_page)
        .execute(connection)
        .expect("Error saving new page");

    Ok(Created::new("/").body(page))
}

#[get("/pages")]
pub fn list() -> Template {
    use self::models::Page;
    let connection = &mut establish_connection();
    let results = self::schema::pages::dsl::pages
        .load::<Page>(connection)
        .expect("Error loading pages");
    Template::render("pages", context! {pages: &results, count: results.len()})
}

#[get("/page/<path..>")]
pub fn get_page(path: PathBuf) -> Template {
    let connection = &mut establish_connection();
    use self::models::Page;

    use self::schema::pages::dsl::*;

    // RECURSIVE SQL QUERY

    // Get all paths: then, compare with the path provided
    let query = sql_query(
        r#"
             WITH RECURSIVE CTE AS (
             SELECT id, slug AS path
             FROM pages
             WHERE parent_id IS NULL
             UNION ALL
             SELECT p.id, path || '/' || slug
             FROM pages p
             JOIN CTE ON p.parent_id = CTE.id
           )
           SELECT * FROM pages WHERE id = (
           SELECT id FROM CTE WHERE path = ?
           );
"#
    );
    println!("path spec is {:?}", path.to_str().unwrap().to_string());
    let binding = query.bind::<Text, _>(path.to_str().unwrap().to_string()).load::<Page>(connection).expect("Database error finding page");
    let child = binding.first().expect("No such page found");
    println!("Child is: {:?}", child);

//     let all_paths = sql_query(
//      r#"
//              WITH RECURSIVE CTE AS (
//              SELECT id, parent_id, slug
//              FROM pages
//              WHERE parent_id IS NULL
//              UNION ALL
//              SELECT p.id, slug
//              FROM pages p
//              JOIN CTE ON p.parent_id = CTE.id
//            )
//            SELECT * FROM CTE
// "#
//     ).load::<(String)>(connection);

    let tree_source = pages.select((id, parent_id, slug)).load::<(Option<i32>, Option<i32>, String)>(connection).expect("Database error");

    // build a tree structure

    let mut tree = TreeBuilder::new().with_root("".to_owned()).build();
    let root_id = tree.root_id().expect("root doesn't exist");

    let mut tree_map = HashMap::new();

    for (ret_id, ret_parent_id, ret_slug) in tree_source {
        if ret_parent_id == None {
            let mut root = tree.get_mut(root_id).unwrap();
            let slug_node = root.append(ret_slug);

            let slug_node_id = slug_node.node_id();
            tree_map.insert(ret_id, slug_node_id);
        } else {
            let tree_parent_id = tree_map.get(&ret_parent_id).unwrap();
            let mut tree_parent = tree.get_mut(*tree_parent_id).unwrap();

            let slug_node = tree_parent.append(ret_slug);

            let slug_node_id = slug_node.node_id();
            tree_map.insert(ret_id, slug_node_id);
        }
    }

    println!("{:?}", &tree);

    // traverse tree to emit nav element

    let mut nav_element = String::from("");
    let acc_path = String::from("");

    nav_element.push_str("<ul>");
    process_node(&tree, root_id, root_id, &mut nav_element, acc_path);
    nav_element.push_str("</ul>");

    fn process_node(tree: &Tree<String>, current_node_id: NodeId, root_id: NodeId, nav_element: &mut String, acc_path: String) {
        nav_element.push_str("<li>");
        let current_node = tree.get(current_node_id).unwrap();
        let children: Vec<NodeRef::<String>> = current_node.children().collect();
        let new_path = if children.len() != 0 {
            format!("{}{}/", acc_path, current_node.data())
        } else {
            format!("{}{}", acc_path, current_node.data())
        };
        nav_element.push_str(format!("<a href=\"http://localhost:8000/page{}\"><div>{}</div></a>", new_path, current_node.data()).as_str());

        // TODO accumulate a slug-path, to be used as the href
        if children.len() != 0 {
            nav_element.push_str("<ul>");
            for child in current_node.children() {
                process_node(tree, child.node_id(), root_id, nav_element, new_path.clone());
            }
            nav_element.push_str("</ul>");
        }
        nav_element.push_str("</li>");

    }

    println!("{}", nav_element);

    Template::render("page", context! {page: &child, nav: &nav_element})
}

#[get("/<file..>")] // HACK
pub async fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).await.ok()
}
