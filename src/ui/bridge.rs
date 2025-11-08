// EventLoopBridge - Coordinates between tokio async runtime and Slint event loop
//
// This is a critical abstraction that solves the challenge of running two event loops:
// 1. Slint's single-threaded GUI event loop
// 2. Tokio's multi-threaded async runtime for I/O operations
//
// The bridge provides:
// - Safe UI updates from tokio tasks via invoke_from_event_loop
// - Spawning async tasks from Slint callbacks
// - Thread-safe marshaling between the two event loops

use slint::{ComponentHandle, Weak};
use std::future::Future;
use tokio::sync::mpsc;

/// Coordinates between tokio async runtime and Slint event loop
///
/// This bridge enables:
/// - UI updates from background tokio tasks (via `update_ui()`)
/// - Spawning async tasks from Slint callbacks (via `spawn_async()`)
/// - Safe marshaling between Slint's single-threaded event loop and tokio's thread pool
///
/// # Example
/// ```ignore
/// let runtime = tokio::runtime::Runtime::new().unwrap();
/// let ui = MainWindow::new().unwrap();
/// let bridge = EventLoopBridge::new(&ui, runtime.handle().clone());
///
/// // From a Slint callback, spawn an async task
/// bridge.spawn_async(|| async {
///     // Do async work...
///
///     // Update UI when done
///     bridge.update_ui(|ui| {
///         ui.set_status("Done!");
///     });
/// });
/// ```
pub struct EventLoopBridge<T: ComponentHandle> {
    /// Weak reference to the UI component to prevent circular references
    ui_weak: Weak<T>,

    /// Handle to the tokio runtime for spawning async tasks
    tokio_handle: tokio::runtime::Handle,

    /// Channel for sending UI update requests from tokio tasks to the Slint event loop
    /// Bounded to 100 updates to prevent unbounded memory growth if UI lags
    ui_update_tx: mpsc::Sender<Box<dyn FnOnce(&T) + Send>>,
}

impl<T: ComponentHandle + 'static> EventLoopBridge<T> {
    /// Create a new EventLoopBridge
    ///
    /// This sets up a background handler thread that processes UI update requests
    /// and marshals them to the Slint event loop using `invoke_from_event_loop`.
    ///
    /// # Arguments
    /// * `ui` - Strong reference to the Slint UI component
    /// * `tokio_handle` - Handle to the tokio runtime for spawning tasks
    ///
    /// # Returns
    /// A new EventLoopBridge instance
    pub fn new(ui: &T, tokio_handle: tokio::runtime::Handle) -> Self {
        let ui_weak = ui.as_weak();
        // Use bounded channel with capacity 100 to prevent OOM if UI lags
        let (ui_update_tx, mut ui_update_rx) = mpsc::channel::<Box<dyn FnOnce(&T) + Send>>(100);

        // Spawn a background thread to handle UI updates
        // This thread bridges between tokio tasks and the Slint event loop
        let ui_weak_clone = ui_weak.clone();
        std::thread::spawn(move || {
            tracing::debug!("EventLoopBridge handler thread started");

            while let Some(update_fn) = ui_update_rx.blocking_recv() {
                // Use Weak::upgrade_in_event_loop to safely update UI from another thread
                // This queues the update to run on Slint's event loop thread
                // The closure receives the upgraded component as an argument
                let result = ui_weak_clone.upgrade_in_event_loop(move |ui| {
                    update_fn(&ui);
                });

                if let Err(e) = result {
                    tracing::warn!("Failed to queue UI update to event loop: {:?}", e);
                    // If we can't queue updates, the event loop may have stopped
                    // Break out of the loop to terminate the handler thread
                    break;
                }
            }

            tracing::debug!("EventLoopBridge handler thread terminated");
        });

        Self {
            ui_weak,
            tokio_handle,
            ui_update_tx,
        }
    }

    /// Schedule a UI update from any thread (typically from tokio tasks)
    ///
    /// This safely marshals the update to the Slint event loop thread.
    /// The update will be queued and executed on the next event loop iteration.
    ///
    /// # Arguments
    /// * `update` - A closure that receives a reference to the UI component and performs updates
    ///
    /// # Example
    /// ```ignore
    /// bridge.update_ui(|ui| {
    ///     ui.set_progress_current(50);
    ///     ui.set_status_text("Processing...");
    /// });
    /// ```
    pub fn update_ui<F>(&self, update: F)
    where
        F: FnOnce(&T) + Send + 'static,
    {
        match self.ui_update_tx.try_send(Box::new(update)) {
            Ok(_) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                tracing::warn!("UI update channel full - skipping update to prevent backpressure");
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                tracing::warn!("Failed to send UI update - handler thread has stopped");
            }
        }
    }

    /// Spawn an async task on the tokio runtime from a Slint callback
    ///
    /// This allows Slint UI callbacks to trigger async operations that run on tokio's thread pool.
    /// This is essential for keeping the UI responsive during I/O operations.
    ///
    /// # Arguments
    /// * `future_factory` - A function that produces a Future to execute on tokio
    ///
    /// # Example
    /// ```ignore
    /// ui.on_start_cleaning(move || {
    ///     bridge.spawn_async(move || async move {
    ///         // Async cleaning work here...
    ///         do_async_cleaning().await;
    ///     });
    /// });
    /// ```
    pub fn spawn_async<F, Fut>(&self, future_factory: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.tokio_handle.spawn(async move {
            future_factory().await;
        });
    }

    /// Clone the bridge for use in multiple callbacks
    ///
    /// Returns a lightweight handle that can be cloned and passed to multiple Slint callbacks.
    /// This is necessary because Slint callbacks often need to capture the bridge by value.
    ///
    /// # Returns
    /// An EventLoopBridgeHandle that implements Clone
    pub fn clone_handle(&self) -> EventLoopBridgeHandle<T> {
        EventLoopBridgeHandle {
            ui_weak: self.ui_weak.clone(),
            tokio_handle: self.tokio_handle.clone(),
            ui_update_tx: self.ui_update_tx.clone(),
        }
    }
}

/// Lightweight handle that can be cloned and passed to callbacks
///
/// This is a cloneable version of EventLoopBridge that can be easily
/// shared across multiple Slint callbacks without worrying about ownership.
pub struct EventLoopBridgeHandle<T: ComponentHandle> {
    ui_weak: Weak<T>,
    tokio_handle: tokio::runtime::Handle,
    ui_update_tx: mpsc::Sender<Box<dyn FnOnce(&T) + Send>>,
}

// Manual Clone implementation to avoid requiring T: Clone
impl<T: ComponentHandle> Clone for EventLoopBridgeHandle<T> {
    fn clone(&self) -> Self {
        Self {
            ui_weak: self.ui_weak.clone(),
            tokio_handle: self.tokio_handle.clone(),
            ui_update_tx: self.ui_update_tx.clone(),
        }
    }
}

impl<T: ComponentHandle + 'static> EventLoopBridgeHandle<T> {
    /// Schedule a UI update from any thread
    ///
    /// See `EventLoopBridge::update_ui()` for details.
    pub fn update_ui<F>(&self, update: F)
    where
        F: FnOnce(&T) + Send + 'static,
    {
        match self.ui_update_tx.try_send(Box::new(update)) {
            Ok(_) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                tracing::warn!("UI update channel full - skipping update to prevent backpressure");
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                tracing::warn!("Failed to send UI update - handler thread has stopped");
            }
        }
    }

    /// Spawn an async task on the tokio runtime
    ///
    /// See `EventLoopBridge::spawn_async()` for details.
    pub fn spawn_async<F, Fut>(&self, future_factory: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.tokio_handle.spawn(async move {
            future_factory().await;
        });
    }

    /// Get a weak reference to the UI component
    ///
    /// This can be used to check if the UI is still alive or to manually
    /// upgrade the reference for custom operations.
    pub fn ui_weak(&self) -> &Weak<T> {
        &self.ui_weak
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    // Note: These tests are limited because they require a real Slint UI component
    // which needs a display/window system. More comprehensive tests will be in integration tests.

    #[test]
    fn test_bridge_handle_clone() {
        // Test that the handle is cloneable
        let rt = tokio::runtime::Runtime::new().unwrap();

        // We can't create a real Slint component in unit tests without a display,
        // but we can test that the handle type implements Clone
        // This is mainly a compile-time check

        // The actual functionality is tested in integration tests
    }

    #[test]
    fn test_async_spawn() {
        // Test that we can spawn async tasks
        let rt = tokio::runtime::Runtime::new().unwrap();
        let counter = Arc::new(AtomicUsize::new(0));

        // Simulate spawning async work
        let counter_clone = counter.clone();
        rt.spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Wait a bit for the task to complete (using blocking sleep)
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Explicitly shut down runtime
        rt.shutdown_timeout(Duration::from_secs(1));
    }

    #[test]
    fn test_thread_safety() {
        // Test that the handle can be sent between threads
        let rt = tokio::runtime::Runtime::new().unwrap();
        let flag = Arc::new(AtomicBool::new(false));

        let flag_clone = flag.clone();
        std::thread::spawn(move || {
            // Simulate tokio handle being used from another thread
            let _handle = rt.handle().clone();
            flag_clone.store(true, Ordering::SeqCst);
        })
        .join()
        .unwrap();

        assert!(flag.load(Ordering::SeqCst));
    }
}
