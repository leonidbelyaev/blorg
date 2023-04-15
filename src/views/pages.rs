use diesel::sql_types::{Text, Integer};
use image::imageops::{BiLevel, ColorMap};
use image::io::Reader;
use rocket::fs::NamedFile;
use rocket::http::{CookieJar, Status};
use rocket::response::Redirect;
use slugify::slugify;
extern crate diesel;
extern crate rocket;
use diesel::sqlite::{SqliteConnection, Sqlite};
use diesel::{prelude::*, sql_query};
use dotenvy::dotenv;
use pandoc::{PandocOutput, PandocOption};
use rocket::response::{status::Created, Debug};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{get, post, put, FromForm, delete};
use crate::models::{self, AuthenticatedAdmin};
use crate::{schema, PersistDatabase, MemoryDatabase};
use rocket_dyn_templates::{context, Template};
use std::collections::HashMap;
use std::path::{PathBuf, Path};
use diesel::sql_types::{Nullable};
use diesel::{prelude::*};
use slab_tree::*;
use models::Page;
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
use chrono::prelude::*;
use diesel::row::Row;

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
    sidebar_markdown_content: String,
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
pub async fn create_child_page(state: &State<ManagedState>, child_page: Form<PageInfo>, path: PathBuf, admin: AuthenticatedAdmin, connection: PersistDatabase) -> Result<Created<Json<Page>>> {
    use models::Page;

    let parent = path2page(&path, &connection).await;

    let utc: DateTime<Utc> = Utc::now();

    let new_page = Page {
        id: None,
        parent_id: parent.id,
        title: child_page.title.to_string(),
        slug: slugify!(&child_page.title.to_string()),
        create_time: utc.format("%Y-%m-%d").to_string(),
        update_time: Some(utc.format("%Y-%m-%d").to_string()),
        sidebar_html_content: md2html(child_page.sidebar_markdown_content.clone(), state.parser_options),
        sidebar_markdown_content: child_page.sidebar_markdown_content.clone(),
        html_content: md2html(child_page.markdown_content.clone(), state.parser_options),
        markdown_content: child_page.markdown_content.clone()
    };

    let to_insert = new_page.clone();

    connection.run(
        move |c| {
            diesel::insert_into(self::schema::pages::dsl::pages).values(to_insert).execute(c).expect("Error saving new page")
        }).await;

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
pub async fn get_page(path: PathBuf, jar: &CookieJar<'_>, connection: PersistDatabase) -> Template {
    use self::models::Page;

    use self::schema::pages::dsl::*;

    let child = path2page(&path, &connection).await;

    let tree_source = connection.run(
        move |c| {
            pages.select((id, parent_id, slug)).load::<(Option<i32>, Option<i32>, String)>(c).expect("Database error")
        }
    ).await;

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
pub async fn edit_page(state: &State<ManagedState>, new_page: Form<PageInfo>, path: PathBuf, admin: AuthenticatedAdmin, connection: PersistDatabase) -> Redirect {
    use self::models::Page;

    use self::schema::pages::dsl::*;

    let child = path2page(&path, &connection).await;

    let utc: DateTime<Utc> = Utc::now();

    let put_page = Page {
        id: child.id,
        parent_id: child.parent_id,
        title: if child.title != "" { new_page.title.to_string()} else { "Root".to_string() },
        create_time: child.create_time,
        update_time: Some(utc.format("%Y-%m-%d").to_string()),
        slug: if child.title != "" { slugify!(&new_page.title.to_string())} else { "".to_string() },
        sidebar_html_content: md2html(new_page.sidebar_markdown_content.clone(), state.parser_options),
        sidebar_markdown_content: new_page.sidebar_markdown_content.clone(),
        html_content: md2html(new_page.markdown_content.clone(), state.parser_options),
        markdown_content: new_page.markdown_content.clone()
    };

    let to_update = put_page.clone();

    connection.run(
        move |c| {
            diesel::update(pages).filter(id.eq(child.id)).set(&put_page).execute(c).expect("Failed to update page from path")
        }
    ).await;


    Redirect::to(uri!(get_page(path)))
}


#[get("/edit/pages/<path..>")]
pub async fn edit_page_form(admin: AuthenticatedAdmin, path: PathBuf, connection: PersistDatabase) -> Template {
    let page = path2page(&path, &connection).await;

    Template::render("edit_page_form", context!{page: page, path: path})
}

#[get("/delete/pages/<path..>")]
pub async fn delete_page(path: PathBuf, admin: AuthenticatedAdmin, connection: PersistDatabase) -> Redirect {
    use self::models::Page;

    use self::schema::pages::dsl::*;

    let spath = format!("/{}", path.to_str().unwrap().to_string());
    if spath == "/" {
        panic!()
    }
    let page = path2page(&path, &connection).await;

    connection.run( move |c|
        {
            diesel::delete(pages).filter(id.eq(page.id)).execute(c).expect("Failed to delete page.")
        }
    ).await;

    let mut path = path.clone();
    path.pop();
    Redirect::to(uri!(get_page(path)))
}

#[derive(QueryableByName, Debug, Serialize)]
struct SearchResult {
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Text)]
    markdown_content: String,
    #[diesel(sql_type = Text)]
    sidebar_markdown_content: String
}

#[get("/search/pages?<query>")]
pub async fn search_pages(query: String, memory_connection: MemoryDatabase) -> Template {
    use self::models::Page;
    use self::schema::pages::dsl::*;

    let search_results = sql_query(
        r#"SELECT snippet(search, 1, '<span class="highlight">', '</span>', '...', 64) AS "title", snippet(search, 2, '<span class="highlight">', '</span>', '...', 64) AS "markdown_content", snippet(search, 3, '<span class="highlight">', '</span>', '...', 64) AS "sidebar_markdown_content" FROM search WHERE search MATCH '{title markdown_content sidebar_markdown_content}: ' || ? "#
    ); // recall: id, title, markdown_content, sidebar_markdown_content. This is markdown_content highlighting.

    let qclone = query.clone();

    let binding = memory_connection.run(
        move |c| {
            search_results.bind::<Text, _>(qclone).load::<SearchResult>(c).expect("Database error")
        }
    ).await;

    for child in &binding {
        println!("{:?}", child);
    }

    Template::render("search_results", context!{search_results: binding, search_term: query})
}

async fn path2page(path: &PathBuf, connection: &PersistDatabase) -> Page {
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
    let binding = connection.run(
        move |c| {
            query.bind::<Text, _>(path_spec).load::<Page>(c).expect("Database error finding page")
        }
    ).await;
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
