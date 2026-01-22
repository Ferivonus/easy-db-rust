use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use rusqlite::{types::ValueRef, Connection, ToSql};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;

// --- SECURITY CHECK ---
// SQL Injection protection: Ensures table and column names only contain safe characters.
fn is_valid_identifier(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// =========================================================
// 1. SERVER PART (EasyDB)
// =========================================================

/// Main library structure (Server Engine)
pub struct EasyDB {
    pub db_name: String,
    conn: Arc<Mutex<Connection>>,
    exposed_tables: Vec<String>,
}

impl EasyDB {
    /// Initializes the database connection.
    pub fn init(name: &str) -> anyhow::Result<Self> {
        let db_path = format!("{}.db", name);
        let conn = Connection::open(db_path)?;

        Ok(Self {
            db_name: name.to_string(),
            conn: Arc::new(Mutex::new(conn)),
            exposed_tables: Vec::new(),
        })
    }

    /// Creates a table and automatically exposes it to the API.
    pub fn create_table(&mut self, table_name: &str, columns: &str) -> anyhow::Result<()> {
        // Security check for table name
        if !is_valid_identifier(table_name) {
            return Err(anyhow::anyhow!("Invalid table name: {}", table_name));
        }

        let sql = format!("CREATE TABLE IF NOT EXISTS {} ({})", table_name, columns);

        let conn = self.conn.lock().unwrap();
        conn.execute(&sql, [])?;

        self.exposed_tables.push(table_name.to_string());
        println!("✅ Table '{}' created and exposed to API.", table_name);
        Ok(())
    }

    /// Starts the server and generates routes.
    pub async fn run_server(self, port: u16) -> anyhow::Result<()> {
        let mut app = Router::new();
        let shared_state = Arc::clone(&self.conn);

        // Dynamically add routes for each table
        for table in &self.exposed_tables {
            let t = table.clone();
            let state = Arc::clone(&shared_state);

            app = app
                .route(
                    &format!("/{}", t),
                    get({
                        let t = t.clone();
                        let s = Arc::clone(&state);
                        move |q| handle_get(State(s), t, q)
                    }),
                )
                .route(
                    &format!("/{}", t),
                    post({
                        let t = t.clone();
                        let s = Arc::clone(&state);
                        move |j| handle_post(State(s), t, j)
                    }),
                )
                // FIX: Changed from /:id to /{id} for Axum 0.7 compatibility
                // Note: We use double braces {{id}} to escape them in format! macro
                .route(
                    &format!("/{}/{{id}}", t),
                    put({
                        let t = t.clone();
                        let s = Arc::clone(&state);
                        move |p, j| handle_put(State(s), t, p, j)
                    }),
                )
                .route(
                    &format!("/{}/{{id}}", t),
                    delete({
                        let t = t.clone();
                        let s = Arc::clone(&state);
                        move |p| handle_delete(State(s), t, p)
                    }),
                );
        }

        // CORS: Allow requests from anywhere (Permissive)
        app = app.layer(CorsLayer::permissive());

        let addr = format!("0.0.0.0:{}", port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        println!("🚀 Easy-DB Server is running: http://{}", addr);

        axum::serve(listener, app).await?;
        Ok(())
    }
}

// =========================================================
// 2. CLIENT PART (EasyClient)
// =========================================================

/// Client Structure: Allows users to easily connect to the server
pub struct EasyClient {
    pub base_url: String,
}

impl EasyClient {
    /// Creates a new client (e.g., localhost, 9000)
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            base_url: format!("http://{}:{}", host, port),
        }
    }

    /// Sends a GET request (Supports Filtering and Sorting)
    pub async fn get(
        &self,
        table: &str,
        params: Option<HashMap<&str, &str>>,
    ) -> anyhow::Result<Value> {
        let mut url = format!("{}/{}", self.base_url, table);

        // If there are filter parameters, add them to the URL
        if let Some(p) = params {
            let query_str: Vec<String> = p.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            if !query_str.is_empty() {
                url.push_str(&format!("?{}", query_str.join("&")));
            }
        }

        let res = reqwest::get(url).await?.json::<Value>().await?;
        Ok(res)
    }

    /// Sends a POST request (Create Data)
    pub async fn post(&self, table: &str, data: Value) -> anyhow::Result<Value> {
        let client = reqwest::Client::new();
        let url = format!("{}/{}", self.base_url, table);

        let res = client
            .post(url)
            .json(&data)
            .send()
            .await?
            .json::<Value>()
            .await?;

        Ok(res)
    }

    /// Sends a PUT request (Update Data)
    pub async fn put(&self, table: &str, id: i64, data: Value) -> anyhow::Result<Value> {
        let client = reqwest::Client::new();
        let url = format!("{}/{}/{}", self.base_url, table, id);
        let res = client
            .put(url)
            .json(&data)
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(res)
    }

    /// Sends a DELETE request (Delete Data)
    pub async fn delete(&self, table: &str, id: i64) -> anyhow::Result<Value> {
        let client = reqwest::Client::new();
        let url = format!("{}/{}/{}", self.base_url, table, id);
        let res = client.delete(url).send().await?.json::<Value>().await?;
        Ok(res)
    }
}

// =========================================================
// 3. HANDLERS (API Logic)
// =========================================================

/// GET: List, filter, and sort data (SECURE VERSION)
async fn handle_get(
    State(db): State<Arc<Mutex<Connection>>>,
    table_name: String,
    Query(params): Query<HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let conn = db.lock().unwrap();
    let mut sql = format!("SELECT * FROM {}", table_name);
    let mut filters = Vec::new();
    let mut sql_params: Vec<Box<dyn ToSql>> = Vec::new();

    // 1. Secure Filtering (Parameterized Query)
    for (k, v) in &params {
        if !k.starts_with('_') {
            if !is_valid_identifier(k) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "Invalid column name"})),
                );
            }
            filters.push(format!("{} = ?", k));
            sql_params.push(Box::new(v.clone()));
        }
    }

    if !filters.is_empty() {
        sql.push_str(&format!(" WHERE {}", filters.join(" AND ")));
    }

    // 2. Sorting
    if let Some(sort_col) = params.get("_sort") {
        if !is_valid_identifier(sort_col) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid sort column"})),
            );
        }
        let order = params
            .get("_order")
            .map(|s| s.to_uppercase())
            .unwrap_or("ASC".to_string());
        let safe_order = if order == "DESC" { "DESC" } else { "ASC" };
        sql.push_str(&format!(" ORDER BY {} {}", sort_col, safe_order));
    }

    // 3. Execute Query
    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        }
    };

    let rows = stmt.query_map(
        rusqlite::params_from_iter(sql_params.iter().map(|p| p.as_ref())),
        |row| Ok(row_to_json(row)),
    );

    match rows {
        Ok(mapped) => {
            let results: Vec<Value> = mapped.filter_map(|r| r.ok()).collect();
            (StatusCode::OK, Json(Value::from(results)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

/// POST: Create new record (SECURE VERSION)
async fn handle_post(
    State(db): State<Arc<Mutex<Connection>>>,
    table_name: String,
    Json(payload): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let conn = db.lock().unwrap();

    if let Some(obj) = payload.as_object() {
        if obj.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Empty JSON body"})),
            );
        }

        let keys: Vec<String> = obj.keys().cloned().collect();
        for key in &keys {
            if !is_valid_identifier(key) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": format!("Invalid column: {}", key)})),
                );
            }
        }

        let placeholders: Vec<String> = keys.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            keys.join(", "),
            placeholders.join(", ")
        );

        let vals: Vec<String> = obj
            .values()
            .map(|v| v.as_str().unwrap_or(&v.to_string()).to_string())
            .collect();

        match conn.execute(&sql, rusqlite::params_from_iter(vals.iter())) {
            Ok(_) => (
                StatusCode::CREATED,
                Json(serde_json::json!({"status": "success", "message": "Record created"})),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            ),
        }
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid JSON format"})),
        )
    }
}

/// PUT: Update record (SECURE VERSION)
async fn handle_put(
    State(db): State<Arc<Mutex<Connection>>>,
    table_name: String,
    Path(id): Path<i32>,
    Json(payload): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let conn = db.lock().unwrap();

    if let Some(obj) = payload.as_object() {
        for key in obj.keys() {
            if !is_valid_identifier(key) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "Invalid column name"})),
                );
            }
        }

        let updates: Vec<String> = obj.keys().map(|k| format!("{} = ?", k)).collect();
        let sql = format!(
            "UPDATE {} SET {} WHERE id = ?",
            table_name,
            updates.join(", ")
        );

        let mut params: Vec<String> = obj
            .values()
            .map(|v| v.as_str().unwrap_or(&v.to_string()).to_string())
            .collect();
        params.push(id.to_string());

        match conn.execute(&sql, rusqlite::params_from_iter(params.iter())) {
            Ok(affected) => {
                if affected == 0 {
                    (
                        StatusCode::NOT_FOUND,
                        Json(serde_json::json!({"error": "Record not found"})),
                    )
                } else {
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({"status": "success", "message": "Record updated"})),
                    )
                }
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            ),
        }
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid JSON format"})),
        )
    }
}

/// DELETE: Delete record (SECURE VERSION)
async fn handle_delete(
    State(db): State<Arc<Mutex<Connection>>>,
    table_name: String,
    Path(id): Path<i32>,
) -> (StatusCode, Json<Value>) {
    let conn = db.lock().unwrap();
    let sql = format!("DELETE FROM {} WHERE id = ?", table_name);

    match conn.execute(&sql, [id]) {
        Ok(affected) => {
            if affected == 0 {
                (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "Record not found"})),
                )
            } else {
                (
                    StatusCode::OK,
                    Json(serde_json::json!({"status": "success", "message": "Record deleted"})),
                )
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

/// Helper: Converts SQLite row to JSON
fn row_to_json(row: &rusqlite::Row) -> Value {
    let mut map = Map::new();
    let column_names = row.as_ref().column_names();

    for (i, name) in column_names.iter().enumerate() {
        let value = match row.get_ref(i).unwrap() {
            ValueRef::Null => Value::Null,
            ValueRef::Integer(n) => Value::from(n),
            ValueRef::Real(f) => Value::from(f),
            ValueRef::Text(t) => Value::from(std::str::from_utf8(t).unwrap_or("")),
            ValueRef::Blob(b) => Value::from(format!("{:?}", b)),
        };
        map.insert(name.to_string(), value);
    }
    Value::Object(map)
}
