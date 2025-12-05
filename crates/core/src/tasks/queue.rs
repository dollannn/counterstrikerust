//! Main thread task queue
//!
//! Allows background threads to queue work to execute on the main game thread.
//! Tasks are processed each frame in GameFrame hook.

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use std::sync::LazyLock;

/// A task to execute on the main thread
pub type Task = Box<dyn FnOnce() + Send + 'static>;

/// Capacity of the task queue per frame
const QUEUE_CAPACITY: usize = 1024;

/// Task queue channels
struct TaskQueue {
    sender: Sender<Task>,
    receiver: Receiver<Task>,
}

static TASK_QUEUE: LazyLock<TaskQueue> = LazyLock::new(|| {
    let (sender, receiver) = bounded(QUEUE_CAPACITY);
    TaskQueue { sender, receiver }
});

/// Queue a task to execute on the next game frame
///
/// This is safe to call from any thread.
///
/// # Returns
/// - `Ok(())` if the task was queued
/// - `Err(())` if the queue is full (task is dropped)
#[tracing::instrument(skip(task))]
pub fn queue_task<F>(task: F) -> Result<(), ()>
where
    F: FnOnce() + Send + 'static,
{
    match TASK_QUEUE.sender.try_send(Box::new(task)) {
        Ok(()) => Ok(()),
        Err(TrySendError::Full(_)) => {
            tracing::warn!("Task queue full, dropping task");
            Err(())
        }
        Err(TrySendError::Disconnected(_)) => {
            tracing::error!("Task queue disconnected");
            Err(())
        }
    }
}

/// Queue a task, blocking if the queue is full
///
/// # Warning
/// Only call from background threads, never from the main thread
/// (would deadlock if queue is full and waiting for frame to process)
#[tracing::instrument(skip(task))]
pub fn queue_task_blocking<F>(task: F)
where
    F: FnOnce() + Send + 'static,
{
    if let Err(e) = TASK_QUEUE.sender.send(Box::new(task)) {
        tracing::error!("Failed to queue task (blocking): {}", e);
    }
}

/// Process all queued tasks
///
/// Called from GameFrame hook on the main thread.
/// Returns the number of tasks processed.
#[tracing::instrument]
pub fn process_queued_tasks() -> usize {
    let mut count = 0;

    // Process up to QUEUE_CAPACITY tasks per frame
    while let Ok(task) = TASK_QUEUE.receiver.try_recv() {
        task();
        count += 1;

        if count >= QUEUE_CAPACITY {
            break;
        }
    }

    count
}

/// Check how many tasks are currently queued
pub fn queued_task_count() -> usize {
    TASK_QUEUE.receiver.len()
}
