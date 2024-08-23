use maud::{html, DOCTYPE, PreEscaped};

pub async fn index() -> maud::Markup {
    html! {
        (DOCTYPE)
        head {
            meta charset="utf-8";
            title { "memcal" }

            meta name="viewport" content="width=device-width";
            meta name="description" content="An iCal compatible server with memory.";
            meta name="author" content="Shantanu Raj";
            link rel="author" href="https://sraj.me";

            style type="text/css" {
                ( PreEscaped(include_str!("./global.css")) )
            }
        }
        body {
            h1 { "memcal" }
            p { "Welcome to memcal, an iCal compatible server with memory." }
            form action="/feed" method="POST" {
                input placeholder="iCal feed URL" type="url" id="url" name="url" required;
                input type="submit" value="Add";
            }
        }
    }
}

// TODO: Implement additional pages for feed management
