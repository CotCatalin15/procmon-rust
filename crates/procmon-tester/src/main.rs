use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    process,
    time::SystemTime,
};

fn main() {
    println!("ProcMon Tester Application");
    println!("==========================");

    loop {
        println!("\nSelect a test scenario:");
        println!("1. Basic file creation");
        println!("2. File modification (append)");
        println!("3. File deletion");
        println!("4. Directory creation and operations");
        println!("5. Multiple rapid file operations");
        println!("6. Complex scenario (create, modify, delete)");
        println!("7. Exit");

        let mut choice = String::new();
        io::stdin()
            .read_line(&mut choice)
            .expect("Failed to read input");

        match choice.trim() {
            "1" => test_file_creation(),
            "2" => test_file_modification(),
            "3" => test_file_deletion(),
            "4" => test_directory_operations(),
            "5" => test_rapid_operations(),
            "6" => test_complex_scenario(),
            "7" => {
                println!("Exiting tester...");
                process::exit(0);
            }
            _ => println!("Invalid choice, please try again"),
        }
    }
}

fn test_file_creation() {
    println!("\n--- Testing File Creation ---");
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let filename = format!("testfile_{}.txt", timestamp);
    let path = PathBuf::from(&filename);

    println!("Creating file: {}", filename);
    match File::create(&path) {
        Ok(_) => println!("Successfully created file: {}", filename),
        Err(e) => println!("Error creating file: {}", e),
    }

    // Leave the file for inspection
    println!("File remains for inspection: {}", filename);
}

fn test_file_modification() {
    println!("\n--- Testing File Modification ---");
    let filename = "mod_testfile.txt";
    let path = PathBuf::from(filename);

    // Create the file first if it doesn't exist
    if !path.exists() {
        File::create(&path).expect("Failed to create test file");
    }

    println!("Opening file for appending: {}", filename);
    let mut file = OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("Failed to open file for appending");

    let content = format!(
        "Modified at timestamp: {}\n",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    println!("Appending content to file...");
    file.write_all(content.as_bytes())
        .expect("Failed to write to file");

    println!("Successfully modified file: {}", filename);
    println!("File remains for inspection: {}", filename);
}

fn test_file_deletion() {
    println!("\n--- Testing File Deletion ---");
    let filename = "delete_testfile.txt";
    let path = PathBuf::from(filename);

    // Create the file first if it doesn't exist
    if !path.exists() {
        File::create(&path).expect("Failed to create test file");
        println!("Created test file for deletion: {}", filename);
    }

    println!("Deleting file: {}", filename);
    match fs::remove_file(&path) {
        Ok(_) => println!("Successfully deleted file: {}", filename),
        Err(e) => println!("Error deleting file: {}", e),
    }
}

fn test_directory_operations() {
    println!("\n--- Testing Directory Operations ---");
    let dirname = "test_directory";
    let path = PathBuf::from(dirname);

    // Create directory
    println!("Creating directory: {}", dirname);
    fs::create_dir(&path).expect("Failed to create directory");

    // Create file in directory
    let filepath = path.join("file_in_dir.txt");
    println!("Creating file in directory: {}", filepath.display());
    File::create(&filepath).expect("Failed to create file in directory");

    // List directory contents
    println!("Listing directory contents:");
    for entry in fs::read_dir(&path).expect("Failed to read directory") {
        let entry = entry.expect("Failed to get entry");
        println!("  {}", entry.path().display());
    }

    // Delete file in directory
    println!("Deleting file in directory: {}", filepath.display());
    fs::remove_file(&filepath).expect("Failed to delete file in directory");

    // Delete directory
    println!("Deleting directory: {}", dirname);
    fs::remove_dir(&path).expect("Failed to delete directory");
}

fn test_rapid_operations() {
    println!("\n--- Testing Rapid File Operations ---");
    let base_filename = "rapid_testfile";
    let count = 5;

    println!(
        "Performing {} rapid create/modify/delete operations...",
        count
    );

    for i in 0..count {
        let filename = format!("{}_{}.txt", base_filename, i);
        let path = PathBuf::from(&filename);

        // Create
        File::create(&path).expect(&format!("Failed to create {}", filename));
        println!("Created {}", filename);

        // Modify
        let mut file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect(&format!("Failed to open {} for writing", filename));
        file.write_all(b"Some data\n")
            .expect(&format!("Failed to write to {}", filename));
        println!("Modified {}", filename);

        // Delete
        fs::remove_file(&path).expect(&format!("Failed to delete {}", filename));
        println!("Deleted {}", filename);
    }

    println!("Completed {} rapid operations", count);
}

fn test_complex_scenario() {
    println!("\n--- Testing Complex Scenario ---");
    let dirname = "complex_test_dir";
    let dir_path = PathBuf::from(dirname);

    // Create directory
    println!("1. Creating directory: {}", dirname);
    fs::create_dir(&dir_path).expect("Failed to create directory");

    // Create multiple files
    let filenames = ["file1.txt", "file2.log", "data.dat"];
    for filename in &filenames {
        let filepath = dir_path.join(filename);
        println!("2. Creating file: {}", filepath.display());
        File::create(&filepath).expect(&format!("Failed to create {}", filename));
    }

    // Modify files
    for filename in &filenames {
        let filepath = dir_path.join(filename);
        println!("3. Modifying file: {}", filepath.display());
        let mut file = OpenOptions::new()
            .append(true)
            .open(&filepath)
            .expect(&format!("Failed to open {} for writing", filename));
        file.write_all(b"Some modified data\n")
            .expect(&format!("Failed to write to {}", filename));
    }

    // Rename a file
    let old_path = dir_path.join("file1.txt");
    let new_path = dir_path.join("renamed_file.txt");
    println!(
        "4. Renaming {} to {}",
        old_path.display(),
        new_path.display()
    );
    fs::rename(&old_path, &new_path).expect("Failed to rename file");

    // Delete some files
    println!("5. Deleting file: {}", new_path.display());
    fs::remove_file(&new_path).expect("Failed to delete file");

    // List remaining files
    println!("6. Remaining files in directory:");
    for entry in fs::read_dir(&dir_path).expect("Failed to read directory") {
        let entry = entry.expect("Failed to get entry");
        println!("  {}", entry.path().display());
    }

    // Clean up - delete directory and contents
    println!("7. Cleaning up - deleting directory and contents");
    fs::remove_dir_all(&dir_path).expect("Failed to delete directory");
}
