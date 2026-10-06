#![cfg(feature = "csv")]

use datafmts::{csv, DataStruct};

#[derive(DataStruct, Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(DataStruct, Debug, PartialEq)]
struct User {
    name: String,
    age: u32,
    admin: bool,
    nick: Option<String>,
}

#[derive(DataStruct, Debug, PartialEq)]
struct WithDefault {
    a: i32,
    #[datafmt(default)]
    b: i32,
}

#[test]
fn point_csv_roundtrip() {
    let p = Point { x: 1, y: -2 };
    let bytes = csv::write(&p).unwrap();
    assert_eq!(bytes, b"x,y\n1,-2\n");
    assert_eq!(csv::read::<Point>(&bytes).unwrap(), p);
}

#[test]
fn string_quoting_option_roundtrip() {
    let u = User {
        name: String::from("Doe, \"JD\""),
        age: 30,
        admin: true,
        nick: None,
    };
    let bytes = csv::write(&u).unwrap();
    assert_eq!(bytes, b"name,age,admin,nick\n\"Doe, \"\"JD\"\"\",30,true,\n");
    assert_eq!(csv::read::<User>(&bytes).unwrap(), u);

    let v = User {
        name: String::from("Ann"),
        age: 1,
        admin: false,
        nick: Some(String::from("x")),
    };
    let bytes = csv::write(&v).unwrap();
    assert_eq!(csv::read::<User>(&bytes).unwrap(), v);
}

#[test]
fn header_mismatch_rejected() {
    assert_eq!(
        csv::read::<Point>(b"y,x\n1,2\n"),
        Err(datafmts::Error::InvalidValue)
    );
}

#[test]
fn trailing_default_filled() {
    let w = csv::read::<WithDefault>(b"a,b\n5\n").unwrap();
    assert_eq!(w, WithDefault { a: 5, b: 0 });
}

#[test]
fn unit_csv_roundtrip() {
    #[derive(DataStruct, Debug, PartialEq)]
    struct Unit;
    let bytes = csv::write(&Unit).unwrap();
    assert_eq!(csv::read::<Unit>(&bytes).unwrap(), Unit);
}
