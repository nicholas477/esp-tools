use std::{path::PathBuf, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};

use esp_tools_lib::commands::package::{get_esp_file_assets, scan_esp_file_graph};

async fn reference_counting_benchmark() -> Result<(), Box<dyn std::error::Error>> {
    let input_file =
        PathBuf::from("I:/SteamLibrary/steamapps/common/Morrowind/Data Files/Tribunal.esm");

    let _res = scan_esp_file_graph(&input_file).await?;

    Ok(())
}

fn async_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("io_bound_tasks");
    group.measurement_time(Duration::from_secs(35));

    // 1. Create a Tokio runtime instance
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    // 2. Setup the benchmark group
    group.bench_function("reference_counting_benchmark", |b| {
        // 3. Use to_async to inform Criterion to handle a Future
        b.to_async(&runtime).iter(|| async {
            reference_counting_benchmark().await.unwrap();
        });
    });

    group.finish();

    let graph = runtime.block_on(async move {
        let input_file =
            PathBuf::from("I:/SteamLibrary/steamapps/common/Morrowind/Data Files/Tribunal.esm");

        scan_esp_file_graph(&input_file).await
    });

    if graph.is_err() {
        eprintln!("Failed to scan ESP file graph: {:?}", graph.err());
        return;
    }
    let graph = graph.unwrap();

    // 2. Setup the benchmark group
    c.bench_function("get_esp_file_assets_benchmark", |b| {
        b.iter(|| {
            get_esp_file_assets(&graph.plugin);
        });
    });
}

criterion_group!(benches, async_benchmark);
criterion_main!(benches);
