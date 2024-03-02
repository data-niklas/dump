use std::path::PathBuf;

use crate::{
    models::{File, Url},
    util::create_connection,
};

pub async fn clean(data_directory: PathBuf) {
    let files_directory = data_directory.join("files");
    std::fs::create_dir_all(&files_directory).expect("Could not create files directory");
    let connection = create_connection(&data_directory).expect("Could not create connection");
    println!(
        "Found {} expired urls",
        Url::count_expired(&connection).expect("Could not count expired urls")
    );
    Url::delete_expired(&connection).expect("Could not delete expired urls");
    let unlinked_files =
        File::search_unlinked(&connection).expect("Could not search unlinked files");
    println!("Found {} unlinked files", unlinked_files.len());
    for file in unlinked_files {
        std::fs::remove_file(files_directory.join(&file.hash)).expect("Could not delete file");
    }
    File::delete_unlinked(&connection).expect("Could not delete unlinked files");
}
