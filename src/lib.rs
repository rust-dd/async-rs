use futures::{
    future::FutureExt,
    task::{waker_ref, ArcWake},
};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc, Mutex,
    },
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};

/// A future that completes after a specified duration.
/// It can be used to simulate asynchronous time-based events.
pub struct Timer {
    /// Shared state between the future and the thread sleeping for the duration.
    shared_state: Arc<Mutex<SharedState>>,
}

/// Shared state between the `Timer` and the sleeping thread.
struct SharedState {
    /// Indicates whether the timer has completed.
    completed: bool,

    /// The waker to wake up the task when the timer completes.
    waker: Option<Waker>,
}

impl Future for Timer {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Acquire the lock to check the shared state.
        let mut shared_state = self.shared_state.lock().unwrap();
        if shared_state.completed {
            // If the timer has completed, return `Poll::Ready`.
            Poll::Ready(())
        } else {
            // Otherwise, store the waker to be called when the timer completes.
            shared_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl Timer {
    /// Creates a new `Timer` that will complete after the specified duration.
    ///
    /// # Arguments
    ///
    /// * `duration` - The amount of time after which the future will complete.
    pub fn new(duration: Duration) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            completed: false,
            waker: None,
        }));

        // Clone the shared state to move into the thread.
        let thread_shared_state = shared_state.clone();
        thread::spawn(move || {
            // Sleep for the specified duration.
            thread::sleep(duration);
            // After sleeping, acquire the lock and update the shared state.
            let mut shared_state = thread_shared_state.lock().unwrap();
            shared_state.completed = true;
            // Wake the task if a waker is available.
            if let Some(waker) = shared_state.waker.take() {
                waker.wake()
            }
        });

        Timer { shared_state }
    }
}

/// A structure that represents a task, encapsulating its future and the task sender.
struct Task {
    /// The future associated with the task.
    future: Mutex<Option<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>>,
    /// Sender to place the task back onto the task queue.
    task_sender: SyncSender<Arc<Task>>,
}

impl Task {
    /// Creates a new task with the given future and task sender.
    ///
    /// # Arguments
    ///
    /// * `future` - The future to be executed.
    /// * `task_sender` - The sender to send the task back to the executor.
    fn new(
        future: impl Future<Output = ()> + 'static + Send,
        task_sender: SyncSender<Arc<Task>>,
    ) -> Arc<Self> {
        Arc::new(Task {
            future: Mutex::new(Some(future.boxed())),
            task_sender,
        })
    }

    /// Polls the task's future.
    fn poll(self: Arc<Self>) {
        // Create a waker from the task itself.
        let waker = waker_ref(&self);
        let mut cx = Context::from_waker(&*waker);
        let mut future_slot = self.future.lock().unwrap();
        if let Some(mut future) = future_slot.take() {
            // Poll the future.
            if future.as_mut().poll(&mut cx).is_pending() {
                // If the future is still pending, put it back.
                *future_slot = Some(future);
            }
        }
    }
}

impl ArcWake for Task {
    /// Schedules the task to be polled again by sending it back to the executor.
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.task_sender.send(arc_self.clone()).unwrap();
    }
}

/// Executor that runs tasks and receives them from a channel.
pub struct Executor {
    /// Receiver side of the task queue channel.
    ready_queue: Receiver<Arc<Task>>,
}

/// Spawner that spawns new futures onto the task queue.
#[derive(Clone)]
pub struct Spawner {
    /// Sender side of the task queue channel.
    task_sender: SyncSender<Arc<Task>>,
}

/// Creates a new executor and spawner pair.
///
/// # Returns
///
/// A tuple containing the executor and spawner.
pub fn async_handler() -> (Executor, Spawner) {
    // Maximum number of tasks allowed in the queue.
    let (task_sender, ready_queue) = sync_channel(100);
    (Executor { ready_queue }, Spawner { task_sender })
}

impl Spawner {
    /// Spawns a new future onto the task queue.
    ///
    /// # Arguments
    ///
    /// * `future` - The future to be executed.
    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let task = Task::new(future, self.task_sender.clone());
        self.task_sender.send(task).unwrap();
    }
}

impl Executor {
    /// Runs the executor until all tasks are completed.
    fn run(&self) {
        // Continuously receive tasks from the ready queue.
        while let Ok(task) = self.ready_queue.recv() {
            task.poll();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Tests spawning two tasks that perform addition.
    #[test]
    fn run_addition_tasks() {
        let (executor, spawner) = async_handler();
        let sum = Arc::new(Mutex::new(0));

        // Spawn a task that adds two numbers after a delay.
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
        assert_eq!(*sum.lock().unwrap(), 25);
    }
}
