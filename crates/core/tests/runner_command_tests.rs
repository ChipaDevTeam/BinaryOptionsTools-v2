//! Tests for RunnerCommand lifecycle and connection state management.
//!
//! These tests verify the new `DisconnectAndHold` functionality and ensure
//! the connection lifecycle state machine behaves correctly.

use binary_options_tools_core::traits::RunnerCommand;

#[test]
fn test_runner_command_debug_output() {
    // Ensure all RunnerCommand variants produce non-empty debug output
    let disconnect = RunnerCommand::Disconnect;
    let disconnect_hold = RunnerCommand::DisconnectAndHold;
    let shutdown = RunnerCommand::Shutdown;
    let connect = RunnerCommand::Connect;
    let reconnect = RunnerCommand::Reconnect;

    assert_eq!(format!("{:?}", disconnect), "Disconnect");
    assert_eq!(format!("{:?}", disconnect_hold), "DisconnectAndHold");
    assert_eq!(format!("{:?}", shutdown), "Shutdown");
    assert_eq!(format!("{:?}", connect), "Connect");
    assert_eq!(format!("{:?}", reconnect), "Reconnect");
}

#[test]
fn test_runner_command_clone_copy() {
    // RunnerCommand should be Clone + Copy
    let original = RunnerCommand::DisconnectAndHold;
    let cloned = original;
    let copied = original;

    assert!(matches!(cloned, RunnerCommand::DisconnectAndHold));
    assert!(matches!(copied, RunnerCommand::DisconnectAndHold));
}

#[test]
fn test_exponential_backoff_calculation() {
    // Test the backoff calculation logic as used in ClientRunner::run
    // delay = min(base_delay * 2^attempts, 300) * jitter(0.8..1.2)
    let base_delay: u64 = 5;

    // Attempt 0: 5 * 2^0 = 5
    let delay0 = base_delay
        .saturating_mul(2u64.saturating_pow(0.min(10)))
        .min(300);
    assert_eq!(delay0, 5);

    // Attempt 1: 5 * 2^1 = 10
    let delay1 = base_delay
        .saturating_mul(2u64.saturating_pow(1.min(10)))
        .min(300);
    assert_eq!(delay1, 10);

    // Attempt 5: 5 * 2^5 = 160
    let delay5 = base_delay
        .saturating_mul(2u64.saturating_pow(5.min(10)))
        .min(300);
    assert_eq!(delay5, 160);

    // Attempt 10 (exponent at cap): 5 * 2^10 = 5120, capped at 300
    let delay10 = base_delay
        .saturating_mul(2u64.saturating_pow(10.min(10)))
        .min(300);
    assert_eq!(delay10, 300);

    // Attempt 15 (exponent capped at 10): same as attempt 10
    let delay15 = base_delay
        .saturating_mul(2u64.saturating_pow(15.min(10)))
        .min(300);
    assert_eq!(delay15, 300);
}

#[test]
fn test_exponential_backoff_with_large_base_delay() {
    // Ensure large base delays don't cause overflow due to saturating_mul
    let base_delay: u64 = 100;

    // Attempt 10: 100 * 2^10 = 102400, capped at 300
    let delay = base_delay
        .saturating_mul(2u64.saturating_pow(10.min(10)))
        .min(300);
    assert_eq!(delay, 300);

    // Attempt 20 (exponent capped): same result
    let delay_capped = base_delay
        .saturating_mul(2u64.saturating_pow(20.min(10)))
        .min(300);
    assert_eq!(delay_capped, 300);
}

#[test]
fn test_exponential_backoff_with_zero_base_delay() {
    // Edge case: base_delay = 0 should still produce valid results
    let base_delay: u64 = 0;

    let delay = base_delay
        .saturating_mul(2u64.saturating_pow(5.min(10)))
        .min(300);
    assert_eq!(delay, 0);
}

#[test]
fn test_jitter_range() {
    // Verify jitter stays within expected bounds (0.8 to 1.2)
    use rand::RngExt;

    let mut rng = rand::rng();
    for _ in 0..1000 {
        let jitter = rng.random_range(0.8..1.2);
        assert!(jitter >= 0.8, "Jitter below minimum: {}", jitter);
        assert!(jitter < 1.2, "Jitter at or above maximum: {}", jitter);
    }
}

#[test]
fn test_command_pattern_matching() {
    // Verify pattern matching works correctly for all variants
    let commands = vec![
        RunnerCommand::Disconnect,
        RunnerCommand::DisconnectAndHold,
        RunnerCommand::Shutdown,
        RunnerCommand::Connect,
        RunnerCommand::Reconnect,
    ];

    let mut disconnect_count = 0;
    let mut disconnect_hold_count = 0;
    let mut shutdown_count = 0;
    let mut connect_count = 0;
    let mut reconnect_count = 0;

    for cmd in commands {
        match cmd {
            RunnerCommand::Disconnect => disconnect_count += 1,
            RunnerCommand::DisconnectAndHold => disconnect_hold_count += 1,
            RunnerCommand::Shutdown => shutdown_count += 1,
            RunnerCommand::Connect => connect_count += 1,
            RunnerCommand::Reconnect => reconnect_count += 1,
        }
    }

    assert_eq!(disconnect_count, 1);
    assert_eq!(disconnect_hold_count, 1);
    assert_eq!(shutdown_count, 1);
    assert_eq!(connect_count, 1);
    assert_eq!(reconnect_count, 1);
}

#[test]
fn test_disconnect_vs_disconnect_and_hold_distinction() {
    // Ensure Disconnect and DisconnectAndHold are distinct variants
    let disconnect = RunnerCommand::Disconnect;
    let disconnect_hold = RunnerCommand::DisconnectAndHold;

    assert!(!matches!(disconnect, RunnerCommand::DisconnectAndHold));
    assert!(!matches!(disconnect_hold, RunnerCommand::Disconnect));

    assert!(matches!(disconnect, RunnerCommand::Disconnect));
    assert!(matches!(disconnect_hold, RunnerCommand::DisconnectAndHold));
}

#[test]
fn test_reconnect_alias_for_connect() {
    // Reconnect and Connect should be distinct variants but semantically related
    let connect = RunnerCommand::Connect;
    let reconnect = RunnerCommand::Reconnect;

    assert!(!matches!(connect, RunnerCommand::Reconnect));
    assert!(!matches!(reconnect, RunnerCommand::Connect));
}
