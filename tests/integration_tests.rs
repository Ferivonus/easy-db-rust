use easy_db::{EasyClient, EasyDB};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

/// Helper: Test için bir sunucuyu arka planda başlatır.
async fn start_test_server(port: u16, db_name: &str) {
    let mut db = EasyDB::init(db_name).expect("Failed to init DB");

    // Test tablolarını oluştur
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

    // Sunucunun hazır olması için bekle
    sleep(Duration::from_millis(300)).await;
}

#[tokio::test]
async fn test_professional_crud_flow() {
    let port = 9600;
    let db_name = "pro_test_db";
    start_test_server(port, db_name).await;

    let client = EasyClient::new("localhost", port);

    // --- 1. CREATE TEST (POST) ---
    // Veri ekleme testi
    let student = json!({"name": "John Doe", "age": 20, "gpa": 3.5});
    let res = client.post("students", student).await.expect("POST failed");
    assert_eq!(res["status"], "success");

    // --- 2. READ & FILTER TEST (GET) ---
    // Eklenen veriyi isme göre filtreleyip çekme ve ID'sini bulma
    let mut params = HashMap::new();
    params.insert("name", "John Doe");
    let results = client
        .get("students", Some(params))
        .await
        .expect("GET failed");

    let list = results.as_array().expect("Result is not an array");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["name"], "John Doe");

    // John'un ID'sini alıyoruz (Update ve Delete için lazım)
    let john_id = list[0]["id"].as_i64().expect("ID not found");

    // --- 3. UPDATE TEST (PUT) ---
    // Artık EasyClient.put metodunu kullanıyoruz
    let updated_data = json!({"age": 21, "gpa": 3.8});
    let update_res = client
        .put("students", john_id, updated_data)
        .await
        .expect("PUT failed");

    assert_eq!(update_res["status"], "success");

    // --- 4. ADVANCED QUERY TEST (SORTING) ---
    // Sıralama testi için ek veriler girelim
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
    sort_params.insert("_order", "desc"); // Yaşa göre büyükten küçüğe

    let sorted_results = client.get("students", Some(sort_params)).await.unwrap();
    let sorted_list = sorted_results.as_array().unwrap();

    // Alice (22) en üstte olmalı
    assert_eq!(sorted_list[0]["name"], "Alice");
    // Bob (19) en altta olmalı
    assert_eq!(sorted_list.last().unwrap()["name"], "Bob");

    // --- 5. DELETE TEST (DELETE) ---
    // EasyClient.delete metodunu kullanıyoruz
    let del_res = client
        .delete("students", john_id)
        .await
        .expect("DELETE failed");
    assert_eq!(del_res["status"], "success");

    // --- 6. ERROR HANDLING (404 Not Found) ---
    // Olmayan bir ID silinmeye çalışıldığında hata mesajı dönmeli
    let fake_del_res = client
        .delete("students", 9999)
        .await
        .expect("Fake DELETE failed");

    // lib.rs içinde 404 durumunda {"error": "Record not found"} dönüyor
    assert_eq!(fake_del_res["error"], "Record not found");

    println!("🚀 Tüm profesyonel test senaryoları (CRUD + Sort + Error) başarıyla geçti!");
}
