//! Integration tests for StateManager with state change events
//!
//! These tests verify that the StateManager correctly:
//! - Emits state change events on mutations
//! - Supports multiple subscribers
//! - Handles concurrent access from multiple threads
//! - Maintains consistency across state transitions

use autoqac::{StateChange, StateManager};
use std::sync::Arc;
use tokio::time::{Duration, timeout};

#[tokio::test]
async fn test_state_change_events_emitted() {
    let state = Arc::new(StateManager::new());
    let mut rx = state.subscribe();

    // Start cleaning
    state.start_cleaning(vec!["plugin1.esp".to_string(), "plugin2.esp".to_string()]);

    // Should receive CleaningStarted event
    let event = timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("Timeout waiting for event")
        .expect("Channel closed");

    assert!(
        matches!(event, StateChange::CleaningStarted { total_plugins: 2 }),
        "Expected CleaningStarted event, got: {:?}",
        event
    );
}

#[tokio::test]
async fn test_multiple_subscribers_receive_events() {
    let state = Arc::new(StateManager::new());
    let mut rx1 = state.subscribe();
    let mut rx2 = state.subscribe();
    let mut rx3 = state.subscribe();

    // Trigger state change
    state.update(|s| {
        s.is_cleaning = true;
        s.total_plugins = 5;
    });

    // All three subscribers should receive the CleaningStarted event
    let event1 = timeout(Duration::from_millis(100), rx1.recv())
        .await
        .expect("Timeout on rx1")
        .expect("rx1 closed");

    let event2 = timeout(Duration::from_millis(100), rx2.recv())
        .await
        .expect("Timeout on rx2")
        .expect("rx2 closed");

    let event3 = timeout(Duration::from_millis(100), rx3.recv())
        .await
        .expect("Timeout on rx3")
        .expect("rx3 closed");

    assert!(matches!(event1, StateChange::CleaningStarted { .. }));
    assert!(matches!(event2, StateChange::CleaningStarted { .. }));
    assert!(matches!(event3, StateChange::CleaningStarted { .. }));
}

#[tokio::test]
async fn test_configuration_change_detection() {
    let state = Arc::new(StateManager::new());
    let mut rx = state.subscribe();

    // Set load order path
    state.set_load_order_path(Some("/path/to/loadorder.txt".into()));

    // Should receive ConfigurationChanged event
    let event = timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("Timeout")
        .expect("Channel closed");

    match event {
        StateChange::ConfigurationChanged {
            is_fully_configured,
        } => {
            assert!(
                !is_fully_configured,
                "Should not be fully configured with only load order set"
            );
        }
        other => panic!("Expected ConfigurationChanged, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_progress_updates_emit_events() {
    let state = Arc::new(StateManager::new());
    let mut rx = state.subscribe();

    // Update progress
    state.update_progress("test.esp".to_string(), "Cleaning...".to_string());

    // Should receive ProgressUpdated and OperationChanged events
    let mut received_progress = false;
    let mut received_operation = false;

    for _ in 0..2 {
        let event = timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("Timeout")
            .expect("Channel closed");

        match event {
            StateChange::ProgressUpdated { .. } => received_progress = true,
            StateChange::OperationChanged { .. } => received_operation = true,
            other => panic!("Unexpected event: {:?}", other),
        }
    }

    assert!(received_progress, "Should receive ProgressUpdated event");
    assert!(received_operation, "Should receive OperationChanged event");
}

#[tokio::test]
async fn test_plugin_result_events() {
    let state = Arc::new(StateManager::new());
    let mut rx = state.subscribe();

    // Start cleaning to set up state
    state.start_cleaning(vec!["plugin1.esp".to_string()]);

    // Clear the start events (CleaningStarted and ProgressUpdated)
    let _ = rx.recv().await;
    let _ = rx.recv().await;

    // Add plugin result
    state.add_plugin_result(
        "plugin1.esp".to_string(),
        "cleaned",
        "Removed 5 ITMs".to_string(),
        None,
    );

    // add_plugin_result emits PluginProcessed event
    // It may also emit ProgressUpdated, so collect all events
    let mut found_plugin_processed = false;

    for _ in 0..3 {
        match timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(Ok(StateChange::PluginProcessed {
                plugin,
                status,
                message,
            })) => {
                assert_eq!(plugin, "plugin1.esp");
                assert_eq!(status, "cleaned");
                assert_eq!(message, "Removed 5 ITMs");
                found_plugin_processed = true;
            }
            Ok(Ok(_)) => continue, // Other events are fine
            Ok(Err(_)) => break,
            Err(_) => break, // Timeout is fine
        }
    }

    assert!(
        found_plugin_processed,
        "Should receive PluginProcessed event"
    );
}

#[tokio::test]
async fn test_cleaning_workflow_events() {
    let state = Arc::new(StateManager::new());
    let mut rx = state.subscribe();

    // Start cleaning
    state.start_cleaning(vec!["plugin1.esp".to_string()]);

    // Collect events (may be CleaningStarted and ProgressUpdated)
    let mut found_cleaning_started = false;
    for _ in 0..3 {
        match timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(Ok(StateChange::CleaningStarted { .. })) => {
                found_cleaning_started = true;
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    assert!(
        found_cleaning_started,
        "Should receive CleaningStarted event"
    );

    // Stop cleaning
    state.stop_cleaning();

    // Receive CleaningFinished (clear any other events first)
    let mut found_cleaning_finished = false;
    for _ in 0..3 {
        match timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(Ok(StateChange::CleaningFinished { .. })) => {
                found_cleaning_finished = true;
                break;
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    assert!(
        found_cleaning_finished,
        "Should receive CleaningFinished event"
    );
}

#[tokio::test]
async fn test_concurrent_state_access() {
    let state = Arc::new(StateManager::new());

    // Spawn multiple tasks that update state concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            state_clone.update(|s| {
                s.progress = i;
            });
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Final progress should be one of the values (last write wins)
    let final_progress = state.read(|s| s.progress);
    assert!(final_progress < 10, "Progress should be within range");
}

#[tokio::test]
async fn test_full_configuration_detection() {
    let state = Arc::new(StateManager::new());
    let mut rx = state.subscribe();

    // Set all required paths
    state.set_load_order_path(Some("/path/to/loadorder.txt".into()));
    let _ = rx.recv().await; // Clear event

    state.set_xedit_exe_path(Some("/path/to/xedit.exe".into()));
    let _ = rx.recv().await; // Clear event

    state.set_mo2_exe_path(Some("/path/to/mo2.exe".into()));

    // Last event should indicate full configuration
    let event = timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("Timeout")
        .expect("Channel closed");

    match event {
        StateChange::ConfigurationChanged {
            is_fully_configured,
        } => {
            assert!(
                is_fully_configured,
                "Should be fully configured with all paths set"
            );
        }
        other => panic!("Expected ConfigurationChanged, got: {:?}", other),
    }

    // Verify via snapshot
    let snapshot = state.snapshot();
    assert!(
        snapshot.is_fully_configured(),
        "Snapshot should show full configuration"
    );
}

#[tokio::test]
async fn test_reset_cleaning_state() {
    let state = Arc::new(StateManager::new());
    let mut rx = state.subscribe();

    // Set up some cleaning state
    state.start_cleaning(vec!["plugin1.esp".to_string()]);

    // Clear all start events
    for _ in 0..5 {
        match timeout(Duration::from_millis(50), rx.recv()).await {
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }

    state.add_plugin_result(
        "plugin1.esp".to_string(),
        "cleaned",
        "Done".to_string(),
        None,
    );

    // Clear all plugin result events
    for _ in 0..5 {
        match timeout(Duration::from_millis(50), rx.recv()).await {
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }

    // Reset state
    state.reset_cleaning_state();

    // Should receive StateReset event (may also receive other events)
    let mut found_state_reset = false;
    for _ in 0..5 {
        match timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(Ok(StateChange::StateReset)) => {
                found_state_reset = true;
                break;
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }

    assert!(found_state_reset, "Expected StateReset event");

    // Verify state is clean
    let snapshot = state.snapshot();
    assert!(!snapshot.is_cleaning);
    assert_eq!(snapshot.progress, 0);
    assert_eq!(snapshot.total_plugins, 0);
    assert!(snapshot.cleaned_plugins.is_empty());
}

#[tokio::test]
async fn test_statistics_aggregation() {
    let state = Arc::new(StateManager::new());

    // Start cleaning
    state.start_cleaning(vec!["plugin1.esp".to_string(), "plugin2.esp".to_string()]);

    // Add results with statistics
    use autoqac::services::CleaningStats;

    let stats1 = CleaningStats {
        undeleted: 3,
        removed: 5,
        skipped: 1,
        partial_forms: 0,
    };

    state.add_plugin_result(
        "plugin1.esp".to_string(),
        "cleaned",
        "Done".to_string(),
        Some(stats1),
    );

    let stats2 = CleaningStats {
        undeleted: 2,
        removed: 7,
        skipped: 0,
        partial_forms: 1,
    };

    state.add_plugin_result(
        "plugin2.esp".to_string(),
        "cleaned",
        "Done".to_string(),
        Some(stats2),
    );

    // Verify aggregated statistics
    let snapshot = state.snapshot();
    assert_eq!(snapshot.total_undeleted, 5, "Total UDRs should be 3 + 2");
    assert_eq!(snapshot.total_removed, 12, "Total ITMs should be 5 + 7");
    assert_eq!(snapshot.total_skipped, 1, "Total navmeshes should be 1 + 0");
    assert_eq!(
        snapshot.total_partial_forms, 1,
        "Total partial forms should be 0 + 1"
    );
    assert_eq!(
        snapshot.total_records_processed, 19,
        "Total records should be sum of all"
    );
}
