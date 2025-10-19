use crate::OcrApp;
use egui::Context;
use std::sync::{Arc, LazyLock, Mutex};

type UpdateQueue = Arc<Mutex<Vec<Box<dyn FnOnce(&Context, &mut OcrApp) + Send>>>>;

static UPDATE_QUEUE: LazyLock<UpdateQueue> = LazyLock::new(UpdateQueue::default);

pub fn update_state(ctx: &Context, ocr_app: &mut OcrApp) {
    let updates = UPDATE_QUEUE.lock().unwrap().drain(..).collect::<Vec<_>>();

    // Execute all queued updates
    for update_fn in updates {
        update_fn(ctx, ocr_app);
    }
}

pub fn enqueue_update<F>(update: F)
where
    F: FnOnce(&Context, &mut OcrApp) + Send + 'static,
{
    let mut queue = UPDATE_QUEUE.lock().unwrap();
    queue.push(Box::new(update));
}
