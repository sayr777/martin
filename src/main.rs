extern crate iron;
extern crate regex;
extern crate iron_test;
extern crate persistent;
extern crate tilejson;
extern crate rererouter;
extern crate r2d2;
extern crate r2d2_postgres;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

use std::env;
use persistent::Read;
use rererouter::RouterBuilder;
use iron::prelude::{Iron, Chain};

mod db;
mod cors;
mod routes;
mod tileset;

pub fn app(conn_string: String) -> iron::Chain {
    let mut router_builder = RouterBuilder::new();
    router_builder.get(r"/index.json", routes::index);
    router_builder.get(r"/(?P<tileset>[\w|\.]*)\.json", routes::tileset);
    router_builder.get(r"/(?P<tileset>[\w|\.]*)/(?P<z>\d*)/(?P<x>\d*)/(?P<y>\d*).pbf", routes::tile);
    let router = router_builder.finalize();

    let mut chain = Chain::new(router);

    println!("Connecting to postgres: {}", conn_string);
    match db::setup_connection_pool(&conn_string, 10) {
        Ok(pool) => {
            let conn = pool.get().unwrap();
            let tilesets = tileset::get_tilesets(conn).unwrap();
            chain.link(Read::<tileset::Tilesets>::both(tilesets));

            chain.link(Read::<db::DB>::both(pool));
        },
        Err(error) => {
            eprintln!("Error connectiong to postgres: {}", error);
            std::process::exit(-1);
        }
    };

    chain.link_after(cors::Middleware);

    chain
}

fn main() {
    let conn_string: String = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let chain = app(conn_string);

    let port = 3000;
    let bind_addr = format!("0.0.0.0:{}", port);
    println!("Server has been started on {}.", bind_addr);
    Iron::new(chain).http(bind_addr.as_str()).unwrap();
}

#[cfg(test)]
mod tests {
    use std::env;
    use iron::Headers;
    use iron_test::{request, response};
    
    use super::app;

    #[test]
    fn test_index() {
        let conn_string: String = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");

        let chain = app(conn_string);

        let headers = Headers::new();
        let response = request::get("http://localhost:3000/index.json", headers, &chain).unwrap();

        let result_body = response::extract_body_to_bytes(response);
        assert_eq!(result_body, b"{}");
    }
}