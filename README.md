# Async Executor in Rust

## Overview

This project provides a minimal async runtime implemented in Rust. It includes:

- **Executor**: Manages a queue of tasks (futures) and runs them until completion.
- **Task**: Encapsulates a future and allows it to be scheduled and polled by the executor.
- **ArcWake Implementation**: Allows tasks to be woken and rescheduled when they're ready to make progress.

## Features

- **Lightweight**: Minimal dependencies and simple codebase.
- **Educational**: Great for learning how async runtimes work under the hood.
- **Customizable**: Extendable to support more complex async patterns.

## Usage

Here's how you can use the executor to run an asynchronous task:

```rust
use std::sync::Arc;

// Import the executor module.
// Adjust the path according to your project structure.
use async_rs::new_executor_and_spawner;

fn main() {
    let (executor, spawner) = new_executor_and_spawner();
    let sum = Arc::new(Mutex::new(0));

        
    spawner.spawn({
        let sum = sum.clone();
        async move {
            let a = 5;
            let b = 10;
            // Simulate some asynchronous computation or delay.
            Timer::new(Duration::from_secs(1)).await;
            *sum.lock().unwrap() += a + b;
        }
    });

    // Spawn another task that adds two different numbers after a delay.
    spawner.spawn({
        let sum = sum.clone();
        async move {
            let x = 3;
            let y = 7;
            // Simulate some asynchronous computation or delay.
            Timer::new(Duration::from_secs(2)).await;
            *sum.lock().unwrap() += x + y;
        }
    });

    // Dropping the spawner to indicate no more tasks will be spawned.
    drop(spawner);

    executor.run();
}
```

## Limitations

- **No Asynchronous I/O**: This executor does not handle asynchronous I/O operations.
- **Single-threaded**: Tasks are executed on a single thread.
- **Educational Purposes**: Meant for learning and understanding the basics of async runtimes.

## Contributing

Contributions are welcome! Please submit a pull request or open an issue to discuss improvements.

## License

This project is open-source and available under the [MIT License](LICENSE).
