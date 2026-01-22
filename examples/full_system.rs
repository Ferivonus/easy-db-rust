use easy_db::{EasyClient, EasyDB};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 --- STARTING MONOLITHIC SYSTEM DEMO --- \n");

    // =====================================================
    // PART 1: SERVER SETUP (Background Task)
    // =====================================================
    let port = 9900; // Using a different port for this demo
    let db_name = "monolith_school_db";

    // 1. Initialize Database
    let mut db = EasyDB::init(db_name).expect("Failed to init DB");

    // 2. Create Tables
    db.create_table(
        "students",
        "id INTEGER PRIMARY KEY, name TEXT, age INTEGER, gpa REAL",
    )?;
    db.create_table("logs", "id INTEGER PRIMARY KEY, message TEXT, level TEXT")?;

    println!("✅ [SERVER] Tables created.");

    // 3. Spawn the Server in a Background Task
    // This ensures the server runs without blocking the main thread.
    tokio::spawn(async move {
        println!("✅ [SERVER] Listening on port {}...", port);
        if let Err(e) = db.run_server(port).await {
            eprintln!("❌ [SERVER] Error: {}", e);
        }
    });

    // Give the server a moment to start up
    sleep(Duration::from_millis(500)).await;

    // =====================================================
    // PART 2: CLIENT OPERATIONS (Main Thread)
    // =====================================================
    let client = EasyClient::new("localhost", port);
    println!("\n🔗 [CLIENT] Connected to localhost:{}\n", port);

    // --- SCENARIO 1: CREATE (POST) ---
    println!("📝 Action: Adding Students...");
    client
        .post(
            "students",
            json!({"name": "Alice Wonderland", "age": 20, "gpa": 3.8}),
        )
        .await?;
    client
        .post(
            "students",
            json!({"name": "Bob Builder", "age": 22, "gpa": 2.5}),
        )
        .await?;
    client
        .post(
            "students",
            json!({"name": "Charlie Chaplin", "age": 25, "gpa": 4.0}),
        )
        .await?;
    println!("✅ Students added.");

    // --- SCENARIO 2: READ & FILTER (GET) ---
    println!("🔍 Action: Finding 'Bob Builder'...");
    let mut params = HashMap::new();
    params.insert("name", "Bob Builder");

    let bob_res = client.get("students", Some(params)).await?;
    let bob_list = bob_res.as_array().expect("Expected array");
    let bob_id = bob_list[0]["id"].as_i64().unwrap();
    println!("✅ Found Bob. ID: {}", bob_id);

    // --- SCENARIO 3: UPDATE (PUT) ---
    println!("🔄 Action: Updating Bob's GPA...");
    client
        .put(
            "students",
            bob_id,
            json!({
                "gpa": 3.0,
                "age": 23
            }),
        )
        .await?;
    println!("✅ Bob's record updated.");

    // --- SCENARIO 4: SORTING (GET) ---
    println!("📊 Action: Fetching Top Students (Sorted by GPA DESC)...");
    let mut sort_params = HashMap::new();
    sort_params.insert("_sort", "gpa");
    sort_params.insert("_order", "desc");

    let sorted = client.get("students", Some(sort_params)).await?;
    if let Some(list) = sorted.as_array() {
        for s in list {
            println!("   - {} (GPA: {})", s["name"], s["gpa"]);
        }
    }

    // --- SCENARIO 5: DELETE (DELETE) ---
    println!("❌ Action: Deleting Charlie...");
    // Find Charlie first to get ID (Simulated) or assuming ID 3 since it's sequential
    client.delete("students", 3).await?;
    println!("✅ Charlie deleted.");

    println!("\n✨ --- DEMO COMPLETED SUCCESSFULLY ---");

    // Optional: Keep server running if you want to test manually via browser
    // std::future::pending::<()>().await;

    Ok(())
}
