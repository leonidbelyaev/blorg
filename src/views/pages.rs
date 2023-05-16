use self::models::PageRevision;
use diesel::sql_types::{BigInt, Integer, Text};

use rocket::{
    http::{CookieJar},
    response::Redirect,
};

extern crate diesel;
extern crate rocket;
use crate::{
    models::{self, AuthenticatedAdmin},
    schema, ManagedState, MemoryDatabase, PersistDatabase,
};

use diesel::{
    prelude::*,
    row::Row,
    sql_query,
    sql_types::Nullable,
};


use models::Page;
use pandoc::{PandocOption, PandocOutput};

use rocket::{
    form::Form,
    get, post,
    response::{Debug},
    serde::{Deserialize, Serialize},
    uri, FromForm, State,
};
use rocket_dyn_templates::{context, Template};
use slab_tree::*;
use std::{
    collections::HashMap,
    path::{PathBuf},
};


type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

#[derive(QueryableByName, Debug, Serialize)]
struct SearchResult {
    #[diesel(sql_type = Nullable<Integer>)]
    id: Option<i32>,
    #[diesel(sql_type = Text)]
    path: String,
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Text)]
    markdown_content: String,
    #[diesel(sql_type = Text)]
    sidebar_markdown_content: String,
}

#[derive(QueryableByName, Debug, Serialize)]
struct QualifiedSearchResult {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Text)]
    markdown_content: String,
    #[diesel(sql_type = Text)]
    sidebar_markdown_content: String,
    #[diesel(sql_type = Text)]
    strpath: String,
}

enum Padding {
    Blank,
    Bar,
}

#[derive(QueryableByName, Debug, Serialize)]
struct IntegerContainer {
    #[diesel(sql_type = Nullable<Integer>)]
    int: Option<i32>,
}

#[derive(QueryableByName, Debug, Serialize)]
struct StringContainer {
    #[diesel(sql_type = Text)]
    string: String,
}

#[derive(Serialize, Deserialize, FromForm, Clone)]
pub struct PageInfo {
    pub title: String,
    pub slug: String,
    pub markdown_content: String,
    pub sidebar_markdown_content: String,
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
        PandocOutput::ToFile(_pathbuf) => {
            panic!()
        }
        PandocOutput::ToBuffer(string) => string,
        PandocOutput::ToBufferRaw(_vec) => {
            panic!()
        }
    }
}

#[post("/pages/<path..>", data = "<child_page>")]
pub async fn create_child_page(
    state: &State<ManagedState>,
    child_page: Form<PageInfo>,
    path: PathBuf,
    _admin: AuthenticatedAdmin,
    connection: PersistDatabase,
    memory_connection: MemoryDatabase,
) -> Redirect {
    
    use models::Page;

    let parent = path2page(&path, &connection).await;

    let child_page = child_page.into_inner();

    let mut child_path = path.clone();
    child_path.push(&child_page.slug);

    // let new_page = Page::new(parent.id, child_page, state.parser_options);

    // let to_insert = new_page.clone();

    let _cloned_path = child_path.clone();

    Page::create_child_and_insert(
        parent.id,
        path.clone(),
        child_page,
        state,
        &connection,
        &memory_connection,
    )
    .await;

    Redirect::to(uri!(get_page(child_path, None::<usize>)))
}

#[get("/create/pages/<path..>")]
pub fn create_child_page_form(path: PathBuf, _admin: AuthenticatedAdmin) -> Template {
    Template::render("create_child_page_form", context! {path: path})
}

#[get("/download/pages/<path..>?<revision>")]
pub async fn download_page_markdown(
    path: PathBuf,
    revision: Option<usize>,
    connection: PersistDatabase,
) -> String {
    let page = path2page(&path, &connection).await;

    use self::schema::page_revision::dsl::*;
    let all_revisions = connection
        .run(move |c| {
            page_revision
                .filter(self::schema::page_revision::dsl::page_id.eq(page.id))
                .order(unix_time)
                .load::<PageRevision>(c)
                .expect("Database error finding page revision")
        })
        .await;
    let to_view = match revision {
        Some(rev) => all_revisions
            .iter()
            .nth(rev)
            .expect("No such page revision found."),
        None => all_revisions.last().expect("No such page revision found."),
    };

    let mut to_return = "# ".to_string();
    to_return.push_str(&page.title.clone());
    to_return.push_str("\n");
    to_return.push_str("-".repeat(80).as_ref());
    to_return.push_str("\n");
    to_return.push_str("\n");
    to_return.push_str(&to_view.markdown_content.clone());
    to_return.push_str("\n");
    to_return.push_str("\n");
    to_return.push_str("-".repeat(80).as_ref());
    to_return.push_str("\n");
    to_return.push_str("\n");
    to_return.push_str(&to_view.sidebar_markdown_content.clone());

    return to_return;
}

#[get("/pages/<path..>?<revision>")]
pub async fn get_page(
    path: PathBuf,
    revision: Option<usize>,
    jar: &CookieJar<'_>,
    connection: PersistDatabase,
) -> Template {
    use self::models::{PageRevision};

    use self::schema::page::dsl::*;

    let child = path2page(&path, &connection).await;

    let tree_source = connection
        .run(move |c| {
            page.select((self::schema::page::dsl::id, parent_id, slug))
                .load::<(Option<i32>, Option<i32>, String)>(c)
                .expect("Database error")
        })
        .await;

    // build a tree structure

    let mut tree = TreeBuilder::new().with_root("".to_owned()).build();
    let root_id = tree.root_id().expect("root doesn't exist");

    let mut tree_map = HashMap::new();

    for (ret_id, ret_parent_id, ret_slug) in tree_source {
        if ret_slug == "" {
            tree_map.insert(ret_id, root_id);
        } else if ret_parent_id == None {
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
    process_node(
        &tree,
        root_id,
        root_id,
        &mut nav_element,
        acc_path,
        &mut segments,
        false,
        &mut Vec::new(),
    );
    nav_element.push_str("</ul>");

    fn process_node(
        tree: &Tree<String>,
        current_node_id: NodeId,
        root_id: NodeId,
        nav_element: &mut String,
        acc_path: String,
        segments: &mut Vec<&str>,
        is_last_child: bool,
        prev: &mut Vec<Padding>,
    ) {
        let current_node = tree.get(current_node_id).unwrap();
        nav_element.push_str("<li>");
        if !prev.is_empty() {
            for i in 0..(prev.len() - 1) {
                match prev[i] {
                    Padding::Blank => nav_element.push_str("&nbsp;&nbsp;&nbsp;"),
                    Padding::Bar => nav_element.push_str("|&nbsp;&nbsp;"),
                }
            }
            if is_last_child {
                nav_element.push_str("└──");
            } else {
                nav_element.push_str("├──");
            }
        }

        let children: Vec<NodeRef<String>> = current_node.children().collect();
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
                    process_node(
                        tree,
                        child.node_id(),
                        root_id,
                        nav_element,
                        new_path.clone(),
                        segments,
                        false,
                        prev,
                    );
                    prev.pop();
                } else {
                    prev.push(Padding::Blank);
                    process_node(
                        tree,
                        child.node_id(),
                        root_id,
                        nav_element,
                        new_path.clone(),
                        segments,
                        true,
                        prev,
                    );
                    prev.pop();
                }
            }
            nav_element.push_str("</ul>");
        }
        nav_element.push_str("</li>");
    }

    let is_user = match jar.get_private("user_id") {
        Some(_other_id) => true,
        None => false,
    };

    println!("{}", nav_element);

    use self::schema::page_revision::dsl::*;
    let all_revisions = connection
        .run(move |c| {
            page_revision
                .filter(self::schema::page_revision::dsl::page_id.eq(child.id))
                .order(unix_time)
                .load::<PageRevision>(c)
                .expect("Database error finding page revision")
        })
        .await;
    let mut is_latest = true;
    if revision.is_some() && revision.unwrap() != (all_revisions.len() - 1) {
        is_latest = false;
    }
    let to_view = match revision {
        Some(rev) => all_revisions
            .iter()
            .nth(rev)
            .expect("No such page revision found."),
        None => all_revisions.last().expect("No such page revision found."),
    };

    Template::render(
        "page",
        context! {page: &child, page_revision: to_view.clone(), all_revisions: all_revisions, nav: &nav_element, is_user: is_user, path: path, is_latest: is_latest, revision_number: revision},
    )
}

#[post("/edit/pages/<path..>", data = "<new_page>")]
pub async fn edit_page(
    state: &State<ManagedState>,
    new_page: Form<PageInfo>,
    path: PathBuf,
    _admin: AuthenticatedAdmin,
    connection: PersistDatabase,
    memory_connection: MemoryDatabase,
) -> Redirect {
    Page::edit_and_update(
        path.clone(),
        new_page.into_inner(),
        &connection,
        &memory_connection,
        state,
    )
    .await;

    // use self::models::Page;

    // use self::schema::page::dsl::*;

    // let child = path2page(&path, &connection).await;

    // let new_page = new_page.into_inner();

    // let mem_page = new_page.clone();

    // let mut redirect_path = path.clone();
    // redirect_path.pop();
    // redirect_path.push(&new_page.slug);

    // let mem_path = redirect_path.clone();

    // let put_page = Page::edit(child.clone(), new_page, state.parser_options);

    // connection
    //     .run(move |c| {
    //         diesel::update(pages)
    //             .filter(id.eq(child.id))
    //             .set(&put_page)
    //             .execute(c)
    //             .expect("Failed to update page from path")
    //     })
    //     .await;

    // memory_connection
    //     .run(move |c| {
    // 	    let query = sql_query("UPDATE search SET path=?, title=?, markdown_content=?, sidebar_markdown_content=? WHERE id = ?");
    // 	    query
    //     .bind::<Text, _>(mem_path.display().to_string())
    //     .bind::<Text, _>(mem_page.title.clone())
    //     .bind::<Text, _>(mem_page.markdown_content.clone())
    //     .bind::<Text, _>(mem_page.sidebar_markdown_content.clone())
    //     .bind::<Nullable<Integer>, _>(child.id)
    //     .execute(c).expect("Database error");
    // 	}).await;

    Redirect::to(uri!(get_page(path, None::<usize>)))
}

pub async fn get_latest_revision(_page_id: i32, connection: &PersistDatabase) -> PageRevision {
    use self::{models::PageRevision, schema::page_revision::dsl::*};
    let all_revisions = connection
        .run(move |c| {
            page_revision
                .filter(self::schema::page_revision::dsl::page_id.eq(page_id))
                .order(unix_time)
                .load::<PageRevision>(c)
                .expect("Database error finding page revision")
        })
        .await;
    all_revisions
        .last()
        .expect("No such page revision found")
        .clone()
}

#[get("/edit/pages/<path..>")]
pub async fn edit_page_form(
    _admin: AuthenticatedAdmin,
    path: PathBuf,
    connection: PersistDatabase,
) -> Template {
    let page = path2page(&path, &connection).await;
    let latest_revision = get_latest_revision(page.id.unwrap(), &connection).await;

    Template::render(
        "edit_page_form",
        context! {page: page, latest_revision: latest_revision, path: path},
    )
}

#[get("/delete/pages/<path..>")]
pub async fn delete_page(
    path: PathBuf,
    _admin: AuthenticatedAdmin,
    connection: PersistDatabase,
    memory_connection: MemoryDatabase,
) -> Redirect {
    

    use self::schema::page::dsl::*;

    let spath = format!("/{}", path.to_str().unwrap().to_string());
    if spath == "/" {
        panic!()
    }
    let to_delete = path2page(&path, &connection).await;

    connection
        .run(move |c| {
            diesel::delete(page)
                .filter(id.eq(to_delete.id))
                .execute(c)
                .expect("Failed to delete page.")
        })
        .await;

    // Cascade delete should take care of children

    memory_connection
        .run(move |c| {
            let query = sql_query("DELETE FROM search WHERE id = ?");
            query
                .bind::<Nullable<Integer>, _>(to_delete.id)
                .execute(c)
                .expect("Database error");
        })
        .await;

    let mut path = path.clone();
    path.pop();
    Redirect::to(uri!(get_page(path, None::<usize>)))
}

#[get("/search/pages?<query>")]
pub async fn search_pages(
    query: String,
    memory_connection: MemoryDatabase,
    _connection: PersistDatabase,
) -> Template {
    

    let search_results = sql_query(
        r#"SELECT id, path, snippet(search, 2, '<span class="highlight">', '</span>', '...', 64) AS "title", snippet(search, 3, '<span class="highlight">', '</span>', '...', 64) AS "markdown_content", snippet(search, 4, '<span class="highlight">', '</span>', '...', 64) AS "sidebar_markdown_content" FROM search WHERE search MATCH '{title markdown_content sidebar_markdown_content}: ' || ? "#,
    );

    let qclone = query.clone();

    let binding = memory_connection
        .run(move |c| {
            search_results
                .bind::<Text, _>(qclone)
                .load::<SearchResult>(c)
                .expect("Database error")
        })
        .await;

    Template::render(
        "search_results",
        context! {search_results: binding, search_term: query},
    )
}

async fn id2path(page_id: i64, connection: &PersistDatabase) -> String {
    

    let query = sql_query(
        r#"
             WITH RECURSIVE CTE AS (
             SELECT id, slug AS path
             FROM page
             WHERE parent_id IS NULL
             UNION ALL
             SELECT p.id, path || '/' || slug
             FROM page p
             JOIN CTE ON p.parent_id = CTE.id
           )
           SELECT path AS "string" FROM CTE WHERE id = ?
        "#,
    );

    let binding = connection
        .run(move |c| {
            query
                .bind::<BigInt, _>(page_id)
                .load::<StringContainer>(c)
                .expect("Database error")
        })
        .await;
    let strctr = binding.first().expect("Found no page with that id");

    strctr.string.clone()
}

async fn path2page(path: &PathBuf, connection: &PersistDatabase) -> Page {
    use self::{models::Page};

    let query = sql_query(
        r#"
             WITH RECURSIVE CTE AS (
             SELECT id, slug AS path
             FROM page
             WHERE parent_id IS NULL
             UNION ALL
             SELECT p.id, path || '/' || slug
             FROM page p
             JOIN CTE ON p.parent_id = CTE.id
           )
           SELECT * FROM page WHERE id = (
           SELECT id FROM CTE WHERE path = ?
           );
"#,
    );
    let path = path.to_str().unwrap().to_string();
    let path_spec = if path != "" {
        format!("/{}", path)
    } else {
        path
    };
    println!("path spec is {:?}", path_spec);
    let binding = connection
        .run(move |c| {
            query
                .bind::<Text, _>(path_spec)
                .load::<Page>(c)
                .expect("Database error finding page")
        })
        .await;
    let child = binding.first().expect("No such page found");
    println!("Child is: {:?}", child);
    child.clone()
}
