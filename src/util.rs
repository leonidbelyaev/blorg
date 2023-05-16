use self::models::PageRevision;
use diesel::sql_types::{BigInt, Integer, Text};

use rocket::{http::CookieJar, response::Redirect};

extern crate diesel;
extern crate rocket;
use crate::{
    models::{self, AuthenticatedAdmin},
    schema, ManagedState, MemoryDatabase, PersistDatabase,
};

use diesel::{prelude::*, row::Row, sql_query, sql_types::Nullable};

use models::Page;
use pandoc::{PandocOption, PandocOutput};

use rocket::{
    form::Form,
    get, post,
    response::Debug,
    serde::{Deserialize, Serialize},
    uri, FromForm, State,
};
use rocket_dyn_templates::{context, Template};
use slab_tree::*;
use std::{collections::HashMap, path::PathBuf};

use pulldown_cmark::{html, Options, Parser};

fn _org2html(org: String) -> String {
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

pub fn md2html(md: String, options: Options) -> String {
    let parser = Parser::new_ext(&md, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

pub fn page2raw(
    title: &String,
    markdown_content: &String,
    sidebar_markdown_content: &String,
) -> String {
    let mut to_return = "# ".to_string();
    to_return.push_str(title);
    to_return.push_str("\n");
    to_return.push_str("-".repeat(80).as_ref());
    to_return.push_str("\n");
    to_return.push_str("\n");
    to_return.push_str(markdown_content);
    to_return.push_str("\n");
    to_return.push_str("\n");
    to_return.push_str("-".repeat(80).as_ref());
    to_return.push_str("\n");
    to_return.push_str("\n");
    if sidebar_markdown_content != "" {
        to_return.push_str(sidebar_markdown_content);
    } else {
        to_return.push_str("This page without sidenotes.");
    }

    to_return
}
