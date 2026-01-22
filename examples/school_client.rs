use easy_db::EasyClient;
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect to the server (Localhost:9000)
    let client = EasyClient::new("localhost", 9000);
    println!("🔗 Connected to the School System.\n");

    // ==========================================
    // SCENARIO 1: NEW RECORDS (POST)
    // ==========================================
    println!("📝 --- STEP 1: Registering Students and Grades ---");

    // Class 10-A
    client
        .post(
            "students",
            json!({
                "name": "Ali Yilmaz", "school_number": 101, "class_grade": "10-A", "gpa": 85.5
            }),
        )
        .await?;

    client
        .post(
            "students",
            json!({
                "name": "Zeynep Kaya", "school_number": 102, "class_grade": "10-A", "gpa": 92.0
            }),
        )
        .await?;

    // Class 11-B (Mehmet Demir)
    client
        .post(
            "students",
            json!({
                "name": "Mehmet Demir", "school_number": 201, "class_grade": "11-B", "gpa": 76.5
            }),
        )
        .await?;

    // Grade Entry
    client
        .post(
            "grades",
            json!({"school_number": 101, "lesson": "Mathematics", "score": 90}),
        )
        .await?;
    client
        .post(
            "grades",
            json!({"school_number": 102, "lesson": "Mathematics", "score": 100}),
        )
        .await?;

    println!("✅ Registration complete.\n");

    // ==========================================
    // SCENARIO 2: DATA UPDATE (PUT)
    // ==========================================
    println!("🔄 --- STEP 2: Mehmet Demir Promoted (Update) ---");

    // 1. First, let's find Mehmet (ID is required)
    let mut search_params = HashMap::new();
    search_params.insert("school_number", "201");

    let search_res = client.get("students", Some(search_params)).await?;

    if let Some(list) = search_res.as_array() {
        if !list.is_empty() {
            let mehmet_id = list[0]["id"].as_i64().unwrap();

            // 2. Update Mehmet's class and GPA
            client
                .put(
                    "students",
                    mehmet_id,
                    json!({
                        "class_grade": "12-A", // Promoted to next grade
                        "gpa": 80.0            // GPA increased
                    }),
                )
                .await?;

            println!("✅ Mehmet (ID: {}) information updated.", mehmet_id);
        }
    }

    // ==========================================
    // SCENARIO 3: ADVANCED QUERY (GET + FILTER + SORT)
    // ==========================================
    println!("\n🔍 --- STEP 3: Class 10-A Report Card (Sorted) ---");

    let mut params = HashMap::new();
    params.insert("class_grade", "10-A"); // Filter: Only 10-A
    params.insert("_sort", "gpa"); // Sort: By GPA
    params.insert("_order", "desc"); // Direction: Descending (Highest first)

    let results = client.get("students", Some(params)).await?;

    if let Some(list) = results.as_array() {
        for student in list {
            println!(
                " - {} (No: {}) -> GPA: {}",
                student["name"], student["school_number"], student["gpa"]
            );
        }
    } else {
        println!("No data found.");
    }

    // ==========================================
    // SCENARIO 4: DATA DELETION (DELETE)
    // ==========================================
    println!("\n❌ --- STEP 4: Cleanup Graduates/Leavers (Delete) ---");

    // Suppose Ali (ID: 1) has left the school.
    let delete_res = client.delete("students", 1).await?;
    println!("Deletion Result: {}", delete_res);

    Ok(())
}
