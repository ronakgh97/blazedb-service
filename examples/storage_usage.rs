// Example of how to use the DataStore storage engine with User schema

use anyhow::Result;
use blaze_service::server::schema::{Plans, User};
use blaze_service::server::storage::DataStore;
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> Result<()> {
    println!("Hashmap Storage Engine Example\n");

    // Create a user store (email -> User mapping)
    let user_store: DataStore<String, User> = DataStore::new(PathBuf::from("data/users.json"))?;

    println!("Created user store");

    // Create a sample user
    let user = User {
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        api_key: None,
        is_verified: false,
        plans: Plans::free_plan(),
        instance_url: "https://alice.blaze.io".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    // Insert the user
    user_store.insert(user.email.clone(), user.clone())?;
    println!("Inserted user: {}", user.email);

    // Get the user
    if let Some(retrieved_user) = user_store.get(&user.email)? {
        println!(" Retrieved user: {}", retrieved_user.username);
        println!("  Plan: {}", retrieved_user.plans.name);
        println!("  Verified: {}", retrieved_user.is_verified);
    }

    // Get all users
    let all_users = user_store.values()?;
    println!("\nTotal users in store: {}", all_users.len());

    // Create an OTP store (email -> OTP mapping)
    let otp_store: DataStore<String, String> = DataStore::new(PathBuf::from("data/otps.json"))?;

    println!("\nCreated OTP store");

    // Store an OTP
    otp_store.insert("alice@example.com".to_string(), "123456".to_string())?;
    println!(" Stored OTP for alice@example.com");

    // Verify OTP
    if let Some(otp) = otp_store.get(&"alice@example.com".to_string())? {
        println!(" Retrieved OTP: {}", otp);
    }

    // Demonstrate thread safety
    println!("\nTesting concurrent access...");

    let store_arc = Arc::new(user_store);
    let mut handles = vec![];

    for i in 0..5 {
        let store_clone = Arc::clone(&store_arc);
        let handle = std::thread::spawn(move || {
            let email = format!("user{}@example.com", i);
            let user = User {
                username: format!("user{}", i),
                email: email.clone(),
                api_key: None,
                is_verified: false,
                plans: Plans::free_plan(),
                instance_url: format!("https://user{}.blaze.io", i),
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            store_clone.insert(email, user).unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!(" Successfully inserted 5 users concurrently");
    println!(" Total users now: {}", store_arc.len()?);

    // Batch operations
    println!("\nTesting batch operations...");
    let api_key_store: DataStore<String, String> =
        DataStore::new(PathBuf::from("data/api_keys.json"))?;

    let batch_keys = vec![
        ("key1".to_string(), "abc123".to_string()),
        ("key2".to_string(), "def456".to_string()),
        ("key3".to_string(), "ghi789".to_string()),
    ];
    api_key_store.batch_insert(batch_keys)?;
    println!(" Batch inserted 3 API keys");

    Ok(())
}
