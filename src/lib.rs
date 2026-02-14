#[macro_use]
extern crate rust_i18n;

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

i18n!("locales");
