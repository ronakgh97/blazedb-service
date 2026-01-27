// Simple benchmark for the storage engine

use blaze_service::server::storage::DataStore;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    println!("HashMap Storage Engine - Performance Benchmark\n");

    let _ = std::fs::remove_file("data/bench_insert.json");
    let _ = std::fs::remove_file("data/bench_read.json");
    let _ = std::fs::remove_file("data/bench_concurrent.json");

    println!("Benchmark 1: Sequential Inserts");
    let store: DataStore<u64, String> = DataStore::new(PathBuf::from("data/bench_insert.json"))?;

    let start = Instant::now();
    let count = 10000;
    for i in 0..count {
        store.insert(i, format!("value_{}", i))?;
    }
    let duration = start.elapsed();

    println!("   Inserted {} items in {:?}", count, duration);
    println!("   Average: {:?} per insert", duration / count as u32);
    println!(
        "   Rate: {:.2} inserts/sec\n",
        count as f64 / duration.as_secs_f64()
    );

    // println!("Benchmark 2: Parallel Inserts");
    // let parallel_store: DataStore<u64, String> =
    //     DataStore::new(PathBuf::from("data/bench_read.json"))?;
    //
    // let start = Instant::now();
    // let threads: Vec<_> = (0..12)
    //     .map(|t| {
    //         let store_clone = parallel_store.clone();
    //         thread::spawn(move || {
    //             for i in 0..(count / 12) {
    //                 let key = t * (count / 12) + i;
    //                 store_clone.insert(key, format!("value_{}", key)).unwrap();
    //             }
    //         })
    //     })
    //     .collect();
    //
    // for handle in threads {
    //     handle.join().unwrap();
    // }
    // let duration = start.elapsed();
    //
    // println!("   Parallel inserted {} items in {:?}", count, duration);
    // println!("   Average: {:?} per insert", duration / count as u32);
    // println!(
    //     "   Rate: {:.2} inserts/sec\n",
    //     count as f64 / duration.as_secs_f64()
    // );
    //
    // println!("Benchmark 2: Sequential Reads");
    // let start = Instant::now();
    // for i in 0..count {
    //     let _ = store.get(&i)?;
    // }
    // let duration = start.elapsed();
    //
    // println!("   Read {} items in {:?}", count, duration);
    // println!("   Average: {:?} per read", duration / count as u32);
    // println!(
    //     "   Rate: {:.2} reads/sec\n",
    //     count as f64 / duration.as_secs_f64()
    // );

    println!("Benchmark 3: Batch Insert");
    let batch_store: DataStore<u64, String> =
        DataStore::new(PathBuf::from("data/bench_batch.json"))?;

    let batch: Vec<_> = (0..count).map(|i| (i, format!("value_{}", i))).collect();

    let start = Instant::now();
    batch_store.batch_insert(batch)?;
    let duration = start.elapsed();

    println!("   Batch inserted {} items in {:?}", count, duration);
    println!("   Average: {:?} per insert", duration / count as u32);
    println!(
        "   Rate: {:.2} inserts/sec\n",
        count as f64 / duration.as_secs_f64()
    );

    println!("Benchmark 4: Concurrent Writes");
    let concurrent_store = Arc::new(DataStore::new(PathBuf::from("data/bench_concurrent.json"))?);

    let num_threads = 12;
    let items_per_thread = 1000;

    let start = Instant::now();
    let mut handles = vec![];

    for t in 0..num_threads {
        let store_clone = Arc::clone(&concurrent_store);
        let handle = thread::spawn(move || {
            for i in 0..items_per_thread {
                let key = t * items_per_thread + i;
                let _ = store_clone.insert(key, format!("thread_{}_value_{}", t, i));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
    let duration = start.elapsed();
    let total_items = num_threads * items_per_thread;

    println!(
        "   {} threads wrote {} items in {:?}",
        num_threads, total_items, duration
    );
    println!("   Average: {:?} per insert", duration / total_items);
    println!(
        "   Rate: {:.2} inserts/sec\n",
        total_items as f64 / duration.as_secs_f64()
    );

    println!("Benchmark 5: Load from Disk");
    drop(store); // Drop the store to close it

    let start = Instant::now();
    let reloaded_store: DataStore<u64, String> =
        DataStore::new(PathBuf::from("data/bench_insert.json"))?;
    let duration = start.elapsed();
    let loaded_count = reloaded_store.len()?;

    println!("   Loaded {} items in {:?}", loaded_count, duration);
    println!(
        "   Rate: {:.2} items/sec\n",
        loaded_count as f64 / duration.as_secs_f64()
    );

    println!("Benchmark 6: Storage Size");
    let metadata = std::fs::metadata("data/bench_insert.json")?;
    let size_kb = metadata.len() as f64 / 1024.0;
    println!("   File size: {:.2} KB for {} items", size_kb, count);
    println!(
        "   Average: {:.2} bytes per item\n",
        (size_kb * 1024.0) / count as f64
    );

    let _ = std::fs::remove_file("data/bench_insert.json");
    let _ = std::fs::remove_file("data/bench_batch.json");
    let _ = std::fs::remove_file("data/bench_concurrent.json");

    println!("Benchmark complete!");

    Ok(())
}
