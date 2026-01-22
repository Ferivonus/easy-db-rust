use easy_db::EasyDB;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔔 Starting School Server...");

    // 1. Initialize a database named 'school_db'
    let mut db = EasyDB::init("school_db").expect("Failed to create database");

    // 2. Create Students Table
    // Columns: id, name, school_number, class_grade, gpa
    db.create_table(
        "students",
        "id INTEGER PRIMARY KEY, name TEXT, school_number INTEGER, class_grade TEXT, gpa REAL",
    )?;

    // 3. Create Grades Table
    // Columns: id, school_number, lesson, score
    db.create_table(
        "grades",
        "id INTEGER PRIMARY KEY, school_number INTEGER, lesson TEXT, score INTEGER",
    )?;

    // 4. Create Teachers Table
    // Columns: id, name, branch
    db.create_table("teachers", "id INTEGER PRIMARY KEY, name TEXT, branch TEXT")?;

    // 5. Start the server on port 9000
    println!("✅ Tables are ready. API is listening on port 9000.");
    db.run_server(9000).await?;

    Ok(())
}
