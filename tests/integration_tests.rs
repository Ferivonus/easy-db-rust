use easy_db::{EasyClient, EasyDB};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

/// Helper: Starts a test server in the background for integration testing.
async fn start_test_server(port: u16, db_name: &str) {
    let mut db = EasyDB::init(db_name).expect("Failed to init DB");

    // Create test tables
    db.create_table(
        "students",
        "id INTEGER PRIMARY KEY, name TEXT, age INTEGER, gpa REAL",
    )
    .expect("Failed to create students table");

    db.create_table("logs", "id INTEGER PRIMARY KEY, message TEXT")
        .expect("Failed to create logs table");

    tokio::spawn(async move {
        let _ = db.run_server(port).await;
    });

    // Wait for the server to be ready
    sleep(Duration::from_millis(300)).await;
}

#[tokio::test]
async fn test_professional_crud_flow() {
    let port = 9600;
    let db_name = "pro_test_db";
    start_test_server(port, db_name).await;

    let client = EasyClient::new("localhost", port);

    // --- 1. CREATE TEST (POST) ---
    // Testing data insertion
    let student = json!({"name": "John Doe", "age": 20, "gpa": 3.5});
    let res = client.post("students", student).await.expect("POST failed");
    assert_eq!(res["status"], "success");

    // --- 2. READ & FILTER TEST (GET) ---
    // Fetch and filter the added data by name to retrieve its ID
    let mut params = HashMap::new();
    params.insert("name", "John Doe");
    let results = client
        .get("students", Some(params))
        .await
        .expect("GET failed");

    let list = results.as_array().expect("Result is not an array");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["name"], "John Doe");

    // Retrieve John's ID (needed for subsequent Update and Delete tests)
    let john_id = list[0]["id"].as_i64().expect("ID not found");

    // --- 3. UPDATE TEST (PUT) ---
    // Utilizing the newly implemented EasyClient.put method
    let updated_data = json!({"age": 21, "gpa": 3.8});
    let update_res = client
        .put("students", john_id, updated_data)
        .await
        .expect("PUT failed");

    assert_eq!(update_res["status"], "success");

    // --- 4. ADVANCED QUERY TEST (SORTING) ---
    // Insert additional data to verify sorting logic
    client
        .post("students", json!({"name": "Alice", "age": 22, "gpa": 3.9}))
        .await
        .expect("Post Alice failed");
    client
        .post("students", json!({"name": "Bob", "age": 19, "gpa": 3.2}))
        .await
        .expect("Post Bob failed");

    let mut sort_params = HashMap::new();
    sort_params.insert("_sort", "age");
    sort_params.insert("_order", "desc"); // Sort by age descending

    let sorted_results = client.get("students", Some(sort_params)).await.unwrap();
    let sorted_list = sorted_results.as_array().unwrap();

    // Alice (22) should be at the top of the list
    assert_eq!(sorted_list[0]["name"], "Alice");
    // Bob (19) should be at the bottom of the list
    assert_eq!(sorted_list.last().unwrap()["name"], "Bob");

    // --- 5. DELETE TEST (DELETE) ---
    // Utilizing the newly implemented EasyClient.delete method
    let del_res = client
        .delete("students", john_id)
        .await
        .expect("DELETE failed");
    assert_eq!(del_res["status"], "success");

    // --- 6. ERROR HANDLING (404 Not Found) ---
    // Verifying that deleting a non-existent ID returns an error message
    let fake_del_res = client
        .delete("students", 9999)
        .await
        .expect("Fake DELETE failed");

    // According to lib.rs, a 404 returns {"error": "Record not found"}
    assert_eq!(fake_del_res["error"], "Record not found");

    println!("🚀 All professional test scenarios (CRUD + Sort + Error) passed successfully!");
}
