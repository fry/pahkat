use chrono::naive::NaiveDateTime;
use diesel::sql_types::{BigInt, Text};

use super::schema::{downloads, users};

#[derive(Queryable, Debug)]
pub struct Download {
    pub id: Vec<u8>,

    pub package_id: String,

    pub package_version: String,

    pub timestamp: NaiveDateTime,
}

#[derive(QueryableByName, Debug)]
pub struct PackageCount {
    #[sql_type = "Text"]
    pub package_id: String,
    #[sql_type = "BigInt"]
    pub count: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "downloads"]
pub struct NewDownload {
    pub package_id: String,

    pub package_version: String,

    pub timestamp: NaiveDateTime,
}

#[derive(Queryable, Debug)]
pub struct User {
    pub id: Vec<u8>,

    pub username: String,

    pub token: Vec<u8>,
}

#[derive(Insertable, Debug)]
#[table_name = "users"]
pub struct NewUser {
    pub username: String,

    pub token: Vec<u8>,
}