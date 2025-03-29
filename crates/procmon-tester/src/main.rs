use std::{
    fs::{File, OpenOptions},
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
        println!("3. Multiple rapid file operations");
        println!("4. Exit");

        let mut choice = String::new();
        io::stdin()
            .read_line(&mut choice)
            .expect("Failed to read input");

        match choice.trim() {
            "1" => test_file_creation(),
            "2" => test_file_modification(),
            "3" => test_rapid_operations(),
            "4" => {
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

    println!("File remains for inspection: {}", filename);
}

fn test_file_modification() {
    println!("\n--- Testing File Modification ---");
    let filename = "mod_testfile.txt";
    let path = PathBuf::from(filename);

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .expect("Failed to open or create file");

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

fn test_rapid_operations() {
    println!("\n--- Testing Rapid File Operations ---");
    let base_filename = "rapid_testfile";
    let count = 5;

    println!("Performing {} rapid create/modify operations...", count);

    for i in 0..count {
        let filename = format!("{}_{}.txt", base_filename, i);
        let path = PathBuf::from(&filename);

        // Create and write
        File::create(&path)
            .expect(&format!("Failed to create {}", filename))
            .write_all(b"Initial data\n")
            .expect(&format!("Failed to write to {}", filename));
        println!("Created and wrote to {}", filename);

        // Modify with append
        let mut file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect(&format!("Failed to open {}", filename));
        file.write_all(b"Additional data\n")
            .expect(&format!("Failed to append to {}", filename));
        println!("Appended to {}", filename);
    }

    println!("Completed {} rapid operations", count);
    println!("Note: Created files remain on disk");
}
