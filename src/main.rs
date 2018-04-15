extern crate binary_lst_parser;

use binary_lst_parser::analyze_lst;
use std::collections::HashMap;

fn main() {
    let mut starts_map = HashMap::new();
    starts_map.insert("trial1.lst", 2usize);
    starts_map.insert("4-byte006.lst", 1480usize);
    let fname = String::from("4-byte006.lst");
    let start_of_data = starts_map[fname.as_str()];
    let range = 512u64;
//    let timepatch = String::from("0");
    let channel_map = [0, 0, 0, 0, 0, 1];
    let timepatch = String::from("5");
    let res = analyze_lst(fname, start_of_data, range,
                                                                 timepatch, channel_map);

    println!("Done.")
}
