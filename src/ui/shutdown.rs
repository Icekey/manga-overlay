use log::info;
use std::sync::LazyLock;
use tokio::spawn;
use tokio_util::task::TaskTracker;

pub static TASK_TRACKER: LazyLock<TaskTracker> = LazyLock::new(|| TaskTracker::new());

pub fn shutdown_tasks() {
    let tracker = TASK_TRACKER.clone();
    info!("start shutdown of {:?} tasks", tracker.len());
    tracker.close();

    spawn(async move {
        tracker.wait().await;
        info!("shutdown down");
    });
}
