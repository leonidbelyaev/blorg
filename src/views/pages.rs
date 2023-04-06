use diesel::sql_types::{Text, Integer};
use rocket::fs::NamedFile;
use rocket::http::CookieJar;
use slugify::slugify;
extern crate diesel;
extern crate rocket;
use diesel::sqlite::SqliteConnection;
use diesel::{prelude::*, sql_query};
use dotenvy::dotenv;
use pandoc::{PandocOutput, PandocOption};
use rocket::response::{status::Created, Debug};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{get, post, put};
use crate::models;
use crate::schema;
use rocket_dyn_templates::{context, Template};
use std::collections::HashMap;
use std::path::{PathBuf, Path};
use diesel::sql_types::{Nullable};
use diesel::{prelude::*};
use slab_tree::*;
use models::Page;
use crate::views::establish_connection;

type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

#[derive(Serialize, Deserialize)]
pub struct NewPage {
    parent_id: Option<i32>,
    title: String,
    org_content: String,
}

#[derive(Serialize, Deserialize)]
pub struct UpdatePage {
    id: Option<i32>,
    title: String,
    org_content: String,
}

fn org2html(org: String) -> String {
    let mut pandoc = pandoc::new();
    pandoc.set_input(pandoc::InputKind::Pipe(org));
    pandoc.set_output(pandoc::OutputKind::Pipe);
    pandoc.set_input_format(pandoc::InputFormat::Org, Vec::new());
    pandoc.set_output_format(pandoc::OutputFormat::Html5, Vec::new());
    pandoc.add_option(PandocOption::HighlightStyle(String::from("zenburn")));
    pandoc.add_option(PandocOption::TableOfContents);
    let new_html_content = pandoc.execute().expect("Error converting org to html");
    match new_html_content {
        PandocOutput::ToFile(pathbuf) => {panic!()},
        PandocOutput::ToBuffer(string) => {string},
        PandocOutput::ToBufferRaw(vec) => {panic!()}
    }
}

#[post("/pages", format = "json", data = "<page>")]
pub fn create_page_id(page: Json<NewPage>) -> Result<Created<Json<Page>>> {
    use self::schema::pages::dsl::*;
    use models::Page;
    let connection = &mut establish_connection();

    let new_page = Page {
        id: None,
        parent_id: page.parent_id,
        title: page.title.to_string(),
        slug: slugify!(&page.title.to_string()),
        html_content: org2html(page.org_content.to_string()),
    };
diesel::insert_into(self::schema::pages::dsl::pages) .values(&new_page) .execute(connection)
        .expect("Error saving new page");

    Ok(Created::new("/").body(Json(new_page)))
}

#[post("/pages/<path..>", format="json", data = "<page>")]
pub fn create_page(page: Json<NewPage>, path: PathBuf) -> Result<Created<Json<Page>>> {
    use models::Page;
    let connection = &mut establish_connection();

    let parent = path2page(&path);

    let new_page = Page {
        id: None,
        parent_id: parent.id,
        title: page.title.to_string(),
        slug: slugify!(&page.title.to_string()),
        html_content: org2html(page.org_content.to_string()),
    };

    diesel::insert_into(self::schema::pages::dsl::pages) .values(&new_page) .execute(connection)
        .expect("Error saving new page");

    Ok(Created::new("/").body(Json(new_page)))
}

#[put("/pages", format="json", data="<page>")]
pub fn put_page_id(page: Json<UpdatePage>) -> Result<Created<Json<Page>>> {
    use self::schema::pages::dsl::*;

    let connection = &mut establish_connection();

    let binding = pages.filter(id.eq(page.id)).load::<Page>(connection).unwrap();

    let old_page = binding.first().unwrap();

    let new_page = Page {
        id: page.id,
        parent_id: old_page.parent_id,
        title: page.title.to_string(),
        slug: slugify!(&page.title.to_string()),
        html_content: org2html(page.org_content.to_string()),
    }; // TODO explore changeset updatepage

    diesel::update(pages).filter(id.eq(new_page.id)).set(&new_page).execute(connection).expect("Failed to meow meow meow");

    Ok(Created::new("/").body(Json(new_page)))
}

#[put("/pages/<path..>", format="json", data="<new_page>")]
pub fn put_page_path(new_page: Json<NewPage>, path: PathBuf) -> Result<Created<Json<Page>>> {
    let connection = &mut establish_connection();
    use self::models::Page;

    use self::schema::pages::dsl::*;

    let child = path2page(&path);

    let put_page = Page {
        id: child.id,
        parent_id: child.parent_id,
        title: new_page.title.to_string(),
        slug: slugify!(&new_page.title.to_string()),
        html_content: org2html(new_page.org_content.to_string())
    };

    diesel::update(pages).filter(id.eq(child.id)).set(&put_page).execute(connection).expect("Failed to update page from path");

    Ok(Created::new("/").body(Json(put_page)))
}

fn path2page(path: &PathBuf) -> Page {
    let connection = &mut establish_connection();
    use self::models::Page;

    use self::schema::pages::dsl::*;

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
    child.clone()
}

#[get("/create/pages/<path..>")]
pub fn edit_page_form(path: PathBuf, jar: &CookieJar<'_>) -> Template {
    if !is_logged_in(jar) {
       panic!("Not logged in.");
    }

    let page = path2page(&path);

    // TODO get Nav element for a particular page function

    todo!();

    // Template::render("edit_page", context! {page: &child, nav: &nav_element, is_user: is_user, path: path, pageroot: pageroot}) // todo When logged in, expose buttons
}

fn is_logged_in(jar: &CookieJar<'_>) -> bool {
    match jar.get_private("user_id") {
        Some(_) => true,
        None => false
    }
}

fn generate_nav() {
// TODO store the tree as a state, update whenever we add or delete
 todo!()
}

#[get("/pages/<path..>")]
pub fn get_page(path: PathBuf, jar: &CookieJar<'_>) -> Template {
    let connection = &mut establish_connection();
    use self::models::Page;

    use self::schema::pages::dsl::*;

    let child = path2page(&path);

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
    let binding = path.to_str().unwrap().to_string();
    let mut segments: Vec<&str> = binding.split('/').collect();
    segments.insert(0, ""); // root is on every path

    nav_element.push_str("<ul>");
    process_node(&tree, root_id, root_id, &mut nav_element, acc_path, &mut segments);
    nav_element.push_str("</ul>");


    fn process_node(tree: &Tree<String>, current_node_id: NodeId, root_id: NodeId, nav_element: &mut String, acc_path: String, segments: &mut Vec<&str>) {
        nav_element.push_str("<li>");
        let current_node = tree.get(current_node_id).unwrap();
        let children: Vec<NodeRef::<String>> = current_node.children().collect();
        let new_seg = if children.len() != 0 {
            format!("{}/", current_node.data())
        } else {
            format!("{}", current_node.data())
        };
        let new_path = format!("{}{}", acc_path, new_seg);
        nav_element.push_str(format!("<a href=\"http://localhost:8000/pages{}\">{}</a>", new_path, new_seg).as_str());

        if children.len() != 0 && segments.len() != 0 && current_node.data() == segments[0] {
            segments.remove(0);
            nav_element.push_str("<ul>");
            for child in current_node.children() {
                process_node(tree, child.node_id(), root_id, nav_element, new_path.clone(), segments);
            }
            nav_element.push_str("</ul>");
        }
        nav_element.push_str("</li>");

    }

    let is_user = match jar.get_private("user_id") {
        Some(other_id) => true,
        None => false
    };

    println!("{}", nav_element);

    let pageroot = "http://localhost:8000";

    Template::render("page", context! {page: &child, nav: &nav_element, is_user: is_user, path: path, pageroot: pageroot}) // todo When logged in, expose buttons
}

