use crate::{
    models::{File, Url},
    util::create_connection,
};
use humansize::{format_size, DECIMAL};
use std::path::PathBuf;

pub async fn stats(data_directory: PathBuf) {
    let files_directory = data_directory.join("files");
    std::fs::create_dir_all(&files_directory).expect("Could not create files directory");
    let connection = create_connection(&data_directory).expect("Could not create connection");
    let file_count = File::count(&connection).expect("Could not count files");
    let url_count = Url::count(&connection).expect("Could not count urls");
    let unlinked_url_count =
        Url::count_expired(&connection).expect("Could not search unlinked urls");
    let total_size = File::size_sum(&connection).expect("Could not get total size");
    let formatted_total_size = format_size(total_size as u64, DECIMAL);

    let mime_count = File::mime_count(&connection).expect("Could not count mimes");
    let group_count = File::group_count(&connection).expect("Could not count groups");

    println!("Files: {} ({})", file_count, formatted_total_size);
    println!("Urls: {} ({} expired)", url_count, unlinked_url_count);

    if unlinked_url_count > 0 {
        println!("Run `dump clean` to remove unlinked files");
    }

    println!("\nCount by mime group:");
    for (group, count) in group_count {
        println!("{}: {}", group, count);
    }

    println!("\nCount by mime type:");
    for (mime, count) in mime_count {
        println!("{}: {}", mime, count);
    }
}
