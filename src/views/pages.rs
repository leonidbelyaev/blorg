use diesel::sql_types::{Text, Integer};
use image::imageops::{BiLevel, ColorMap};
use image::io::Reader;
use rocket::fs::NamedFile;
use rocket::http::{CookieJar, Status};
use rocket::response::Redirect;
use slugify::slugify;
extern crate diesel;
extern crate rocket;
use diesel::sqlite::SqliteConnection;
use diesel::{prelude::*, sql_query};
use dotenvy::dotenv;
use pandoc::{PandocOutput, PandocOption};
use rocket::response::{status::Created, Debug};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{get, post, put, FromForm, delete};
use crate::models::{self, AuthenticatedAdmin};
use crate::schema;
use rocket_dyn_templates::{context, Template};
use std::collections::HashMap;
use std::path::{PathBuf, Path};
use diesel::sql_types::{Nullable};
use diesel::{prelude::*};
use slab_tree::*;
use models::Page;
use crate::views::{establish_connection, is_logged_in};
use rocket::form::Form;
use rocket::uri;
use pulldown_cmark::{Parser, Options, html};
use rocket::State;
use crate::ManagedState;
use rocket::fs::TempFile;
use image::imageops::colorops::dither;
use image::RgbImage;
use image::{open, DynamicImage};
use std::io::Cursor;
use image::ImageFormat;
use tempdir::TempDir;
use std::fs::File;
use std::io::{self, Write};

type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

#[derive(Serialize, Deserialize, FromForm)]
pub struct NewPage {
    parent_id: Option<i32>,
    title: String,
    org_content: String,
}

#[derive(Serialize, Deserialize, FromForm)]
pub struct PageInfo {
    title: String,
    markdown_content: String,
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

fn md2html(md: String, options: Options) -> String {
    let parser = Parser::new_ext(&md, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

#[post("/pages/<path..>", data = "<child_page>")]
pub fn create_child_page(state: &State<ManagedState>, child_page: Form<PageInfo>, path: PathBuf, admin: AuthenticatedAdmin) -> Result<Created<Json<Page>>> {
    use models::Page;
    let connection = &mut establish_connection();

    let parent = path2page(&path);

    let new_page = Page {
        id: None,
        parent_id: parent.id,
        title: child_page.title.to_string(),
        slug: slugify!(&child_page.title.to_string()),
        html_content: md2html(child_page.markdown_content.clone(), state.parser_options),
        markdown_content: child_page.markdown_content.clone()
    };

    diesel::insert_into(self::schema::pages::dsl::pages) .values(&new_page) .execute(connection)
        .expect("Error saving new page");

    Ok(Created::new("/").body(Json(new_page))) // TODO change this
}

#[get("/create/pages/<path..>")]
pub fn create_child_page_form(path: PathBuf, admin: AuthenticatedAdmin) -> Template {
    Template::render("create_child_page_form", context!{path: path})
}

enum Padding {
    Blank,
    Bar
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
        if ret_slug == "" {
            tree_map.insert(ret_id, root_id);
        }
        else if ret_parent_id == None {
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

    println!("{:#?}", &tree);

    // traverse tree to emit nav element

    let mut nav_element = String::from("");
    let acc_path = String::from("");
    let binding = path.to_str().unwrap().to_string();
    let mut segments: Vec<&str> = binding.split('/').collect();
    segments.insert(0, ""); // root is on every path

    nav_element.push_str("<ul>");
    process_node(&tree, root_id, root_id, &mut nav_element, acc_path, &mut segments, false, &mut Vec::new());
    nav_element.push_str("</ul>");

    fn process_node(tree: &Tree<String>, current_node_id: NodeId, root_id: NodeId, nav_element: &mut String, acc_path: String, segments: &mut Vec<&str>, is_last_child: bool, prev: &mut Vec<Padding>) {
        let current_node = tree.get(current_node_id).unwrap();
        nav_element.push_str("<li>");
        if !prev.is_empty() {
            for i in 0..(prev.len()-1) {
                match prev[i] {
                    Padding::Blank => nav_element.push_str("&nbsp;&nbsp;&nbsp;"),
                    Padding::Bar => nav_element.push_str("|&nbsp;&nbsp;")
                }
            }
            if is_last_child {
                nav_element.push_str("└──");
            } else {
                nav_element.push_str("├──");
            }
        }

        let children: Vec<NodeRef::<String>> = current_node.children().collect();
        let new_seg = if children.len() != 0 {
            format!("{}/", current_node.data())
        } else {
            format!("{}", current_node.data())
        };
        let new_path = format!("{}{}", acc_path, new_seg);
        nav_element.push_str(format!("<a href=\"/pages{}\">{}</a>", new_path, new_seg).as_str());

        if children.len() != 0 && segments.len() != 0 && current_node.data() == segments[0] {
            segments.remove(0);
            nav_element.push_str("<ul>");

            let mut children = current_node.children().peekable();

            while let Some(child) = children.next() {
                if children.peek().is_some() {
                    prev.push(Padding::Bar);
                    process_node(tree, child.node_id(), root_id, nav_element, new_path.clone(), segments, false, prev);
                    prev.pop();
                } else {
                    prev.push(Padding::Blank);
                    process_node(tree, child.node_id(), root_id, nav_element, new_path.clone(), segments, true, prev);
                    prev.pop();
                }
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

    Template::render("page", context! {page: &child, nav: &nav_element, is_user: is_user, path: path})
}

#[post("/edit/pages/<path..>", data="<new_page>")]
pub fn edit_page(state: &State<ManagedState>, new_page: Form<PageInfo>, path: PathBuf, admin: AuthenticatedAdmin) -> Redirect {
    let connection = &mut establish_connection();
    use self::models::Page;

    use self::schema::pages::dsl::*;

    let child = path2page(&path);

    let put_page = Page {
        id: child.id,
        parent_id: child.parent_id,
        title: new_page.title.to_string(),
        slug: slugify!(&new_page.title.to_string()),
        html_content: md2html(new_page.markdown_content.clone(), state.parser_options),
        markdown_content: new_page.markdown_content.clone()
    };

    diesel::update(pages).filter(id.eq(child.id)).set(&put_page).execute(connection).expect("Failed to update page from path");

    Redirect::to(uri!(get_page(path)))
}


#[get("/edit/pages/<path..>")]
pub fn edit_page_form(admin: AuthenticatedAdmin, path: PathBuf) -> Template {
    let page = path2page(&path);

    Template::render("edit_page_form", context!{page: page, path: path})
}

#[get("/delete/pages/<path..>")]
pub fn delete_page(path: PathBuf, admin: AuthenticatedAdmin) -> Redirect {
    let connection = &mut establish_connection();
    use self::models::Page;

    use self::schema::pages::dsl::*;

    let spath = format!("/{}", path.to_str().unwrap().to_string());
    if spath == "/" {
        panic!()
    }
    let page = path2page(&path);

    diesel::delete(pages).filter(id.eq(page.id)).execute(connection).expect("Failed to delete page.");

    let mut path = path.clone();
    path.pop();
    Redirect::to(uri!(get_page(path)))
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
    let path = path.to_str().unwrap().to_string();
    let path_spec = if path != "" { format!("/{}", path) } else {path};
    println!("path spec is {:?}", path_spec);
    let binding = query.bind::<Text, _>(path_spec).load::<Page>(connection).expect("Database error finding page");
    let child = binding.first().expect("No such page found");
    println!("Child is: {:?}", child);
    child.clone()
}

#[derive(FromForm)]
pub struct Upload<'f> {
    filename: String,
    image: TempFile<'f>
}

#[post("/upload/image", data = "<form>")]
pub async fn upload_image(mut form: Form<Upload<'_>>) -> std::io::Result<()> {

    // hand is forced by https://github.com/SergioBenitez/Rocket/issues/2296 for now

    let tmp_dir = TempDir::new("blorg")?;
    let tmppath = tmp_dir.path().join(form.filename.clone());
    form.image.move_copy_to(&tmppath).await;

    let mut image = Reader::open(&tmppath)?.decode().unwrap().into_luma8();
    dither(&mut image, &BiLevel);

    let persist_path = format!("./static/img/runtime/{}", form.filename.clone());
    image.save(persist_path);

    Ok(())
}

#[get("/upload/image")]
pub fn upload_image_form(admin: AuthenticatedAdmin) -> Template {
    Template::render("upload_image_form", context!{})
}
