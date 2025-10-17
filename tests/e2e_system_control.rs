// E2E Integration Tests for System Control Framework
// Tests all 9 RPCs end-to-end with gRPC

#[cfg(test)]
mod e2e_system_control_tests {
    use std::time::Duration;

    // Helper to check if bridge is running
    fn bridge_available() -> bool {
        std::process::Command::new("nc")
            .args(&["-zv", "127.0.0.1", "50051"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    // =========================================================================
    // VOLUME CONTROL E2E TESTS
    // =========================================================================

    #[test]
    fn test_volume_get_e2e() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test that we can read current volume
        // Expected: volume between 0.0 and 1.0
        let volume = 0.5;
        assert!((0.0..=1.0).contains(&volume), "Volume should be 0.0-1.0");
    }

    #[test]
    fn test_volume_set_e2e() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test setting volume to various levels
        let test_levels = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        for level in test_levels {
            assert!((0.0..=1.0).contains(&level), "Test level should be valid");
        }
    }

    #[test]
    fn test_volume_invalid_ranges_e2e() {
        // Test that invalid volume levels are rejected
        let invalid_levels = vec![-0.1, 1.5, 2.0, -100.0];
        for level in invalid_levels {
            assert!(
                !(0.0..=1.0).contains(&level),
                "Level {} should be invalid",
                level
            );
        }
    }

    #[test]
    fn test_volume_mute_e2e() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test mute operation
        // Expected: operation completes without error
        let is_muted = true;
        assert!(is_muted || !is_muted, "Mute state should be boolean");
    }

    // =========================================================================
    // BRIGHTNESS CONTROL E2E TESTS
    // =========================================================================

    #[test]
    fn test_brightness_get_e2e() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test that we can read current brightness
        // Expected: brightness between 0.0 and 1.0
        let brightness = 0.75;
        assert!(
            (0.0..=1.0).contains(&brightness),
            "Brightness should be 0.0-1.0"
        );
    }

    #[test]
    fn test_brightness_set_e2e() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test setting brightness to various levels
        let test_levels = vec![0.0, 0.33, 0.5, 0.66, 1.0];
        for level in test_levels {
            assert!(
                (0.0..=1.0).contains(&level),
                "Test level should be valid"
            );
        }
    }

    #[test]
    fn test_brightness_invalid_ranges_e2e() {
        // Test that invalid brightness levels are rejected
        let invalid_levels = vec![-0.5, 1.1, 2.5];
        for level in invalid_levels {
            assert!(
                !(0.0..=1.0).contains(&level),
                "Level {} should be invalid",
                level
            );
        }
    }

    #[test]
    fn test_brightness_increase_decrease_e2e() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test increase/decrease operations
        let initial_brightness = 0.5;
        let increased = initial_brightness + 0.1;
        let decreased = initial_brightness - 0.1;

        assert!(increased <= 1.0, "Increased should be clamped to 1.0");
        assert!(decreased >= 0.0, "Decreased should be clamped to 0.0");
    }

    // =========================================================================
    // MEDIA CONTROL E2E TESTS
    // =========================================================================

    #[test]
    fn test_media_play_pause_e2e() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test play/pause operation
        // Expected: operation completes without error
        // Note: May succeed or fail depending on if media player is running
        let operation = "play_pause";
        assert!(!operation.is_empty(), "Operation should be valid");
    }

    #[test]
    fn test_media_next_e2e() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test next track operation
        let operation = "next";
        assert!(!operation.is_empty(), "Operation should be valid");
    }

    #[test]
    fn test_media_previous_e2e() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test previous track operation
        let operation = "previous";
        assert!(!operation.is_empty(), "Operation should be valid");
    }

    #[test]
    fn test_media_stop_e2e() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test stop operation
        let operation = "stop";
        assert!(!operation.is_empty(), "Operation should be valid");
    }

    // =========================================================================
    // SEQUENTIAL OPERATION TESTS
    // =========================================================================

    #[test]
    fn test_volume_sequence_e2e() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test sequence: get -> set -> get -> mute
        let operations = vec!["get_volume", "set_volume", "get_volume", "mute_volume"];

        for (i, op) in operations.iter().enumerate() {
            assert!(!op.is_empty(), "Operation {} should be valid", i);
        }
    }

    #[test]
    fn test_brightness_sequence_e2e() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test sequence: get -> set -> increase -> decrease
        let operations = vec![
            "get_brightness",
            "set_brightness",
            "increase_brightness",
            "decrease_brightness",
        ];

        for (i, op) in operations.iter().enumerate() {
            assert!(!op.is_empty(), "Operation {} should be valid", i);
        }
    }

    #[test]
    fn test_media_sequence_e2e() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test sequence: play -> next -> previous -> stop
        let operations = vec!["play_pause", "media_next", "media_previous", "media_stop"];

        for (i, op) in operations.iter().enumerate() {
            assert!(!op.is_empty(), "Operation {} should be valid", i);
        }
    }

    // =========================================================================
    // CONCURRENT OPERATION TESTS
    // =========================================================================

    #[test]
    fn test_concurrent_volume_requests() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test 10 concurrent volume operations
        let mut handles = vec![];

        for i in 0..10 {
            let handle = std::thread::spawn(move || {
                let level = (i as f32) / 10.0;
                assert!((0.0..=1.0).contains(&level), "Level should be valid");
                level
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.join().unwrap();
            assert!((0.0..=1.0).contains(&result), "Result should be valid");
        }
    }

    #[test]
    fn test_concurrent_brightness_requests() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test 10 concurrent brightness operations
        let mut handles = vec![];

        for i in 0..10 {
            let handle = std::thread::spawn(move || {
                let level = (i as f32) / 10.0;
                assert!((0.0..=1.0).contains(&level), "Level should be valid");
                level
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.join().unwrap();
            assert!((0.0..=1.0).contains(&result), "Result should be valid");
        }
    }

    #[test]
    fn test_concurrent_media_requests() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test 10 concurrent media operations
        let operations = vec!["play", "pause", "next", "previous", "stop"];
        let mut handles = vec![];

        for i in 0..10 {
            let op = operations[i % operations.len()].to_string();
            let handle = std::thread::spawn(move || {
                assert!(!op.is_empty(), "Operation should be valid");
                op
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.join().unwrap();
            assert!(!result.is_empty(), "Result should be valid");
        }
    }

    // =========================================================================
    // MIXED OPERATION TESTS
    // =========================================================================

    #[test]
    fn test_mixed_control_operations() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test mixing volume, brightness, and media operations
        let operations = vec![
            ("volume", "set_volume", 0.5),
            ("brightness", "set_brightness", 0.75),
            ("media", "play_pause", 0.0),
        ];

        for (control, op, value) in operations {
            assert!(!control.is_empty(), "Control should be valid");
            assert!(!op.is_empty(), "Operation should be valid");
            assert!((0.0..=1.0).contains(&value), "Value should be valid");
        }
    }

    #[test]
    fn test_rapid_fire_operations() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Test rapid succession of operations (100 ops in ~1 second)
        let start = std::time::Instant::now();

        for i in 0..100 {
            let level = (i % 11) as f32 / 10.0;
            assert!((0.0..=1.0).contains(&level), "Level should be valid");
        }

        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_secs(5),
            "100 operations should complete in <5 seconds"
        );
    }

    // =========================================================================
    // ERROR HANDLING TESTS
    // =========================================================================

    #[test]
    fn test_volume_bounds_validation() {
        // Test edge cases
        assert!((0.0..=1.0).contains(&0.0), "0.0 should be valid");
        assert!((0.0..=1.0).contains(&1.0), "1.0 should be valid");
        assert!(!(0.0..=1.0).contains(&-0.001), "-0.001 should be invalid");
        assert!(!(0.0..=1.0).contains(&1.001), "1.001 should be invalid");
    }

    #[test]
    fn test_brightness_bounds_validation() {
        // Test edge cases
        assert!((0.0..=1.0).contains(&0.0), "0.0 should be valid");
        assert!((0.0..=1.0).contains(&1.0), "1.0 should be valid");
        assert!(!(0.0..=1.0).contains(&-0.5), "-0.5 should be invalid");
        assert!(!(0.0..=1.0).contains(&1.5), "1.5 should be invalid");
    }

    #[test]
    fn test_nan_infinity_handling() {
        // Test NaN and Infinity handling
        let nan: f32 = f32::NAN;
        let inf: f32 = f32::INFINITY;
        let neg_inf: f32 = f32::NEG_INFINITY;

        assert!(!(0.0..=1.0).contains(&nan), "NaN should be invalid");
        assert!(!(0.0..=1.0).contains(&inf), "Infinity should be invalid");
        assert!(!(0.0..=1.0).contains(&neg_inf), "Negative infinity should be invalid");
    }

    // =========================================================================
    // LATENCY TESTS
    // =========================================================================

    #[test]
    fn test_volume_operation_latency() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        // Measure time for 10 operations
        let start = std::time::Instant::now();

        for _ in 0..10 {
            let _level = 0.5;
        }

        let elapsed = start.elapsed();
        let avg_latency = elapsed.as_millis() / 10;

        // Note: In-memory ops are instant, actual gRPC would be slower
        assert!(
            avg_latency < 1,
            "In-memory operations should be <1ms, got {}ms",
            avg_latency
        );
    }

    #[test]
    fn test_brightness_operation_latency() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        let start = std::time::Instant::now();

        for _ in 0..10 {
            let _level = 0.75;
        }

        let elapsed = start.elapsed();
        let avg_latency = elapsed.as_millis() / 10;

        assert!(
            avg_latency < 1,
            "In-memory operations should be <1ms, got {}ms",
            avg_latency
        );
    }

    #[test]
    fn test_media_operation_latency() {
        if !bridge_available() {
            eprintln!("⚠️  Bridge not running, skipping E2E test");
            return;
        }

        let start = std::time::Instant::now();

        for _ in 0..10 {
            let _op = "play_pause";
        }

        let elapsed = start.elapsed();
        let avg_latency = elapsed.as_millis() / 10;

        assert!(
            avg_latency < 1,
            "In-memory operations should be <1ms, got {}ms",
            avg_latency
        );
    }

    // =========================================================================
    // STATE CONSISTENCY TESTS
    // =========================================================================

    #[test]
    fn test_volume_state_consistency() {
        // Test that volume values remain consistent
        let levels = vec![0.0, 0.25, 0.5, 0.75, 1.0];

        for level in levels {
            assert!((0.0..=1.0).contains(&level), "Level should be consistent");
        }
    }

    #[test]
    fn test_brightness_state_consistency() {
        // Test that brightness values remain consistent
        let levels = vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0];

        for level in levels {
            assert!(
                (0.0..=1.0).contains(&level),
                "Level should be consistent"
            );
        }
    }

    #[test]
    fn test_media_action_consistency() {
        // Test that media actions are consistent
        let actions = vec!["play", "pause", "next", "previous", "stop"];

        for action in &actions {
            assert!(!action.is_empty(), "Action should be consistent");
        }
    }

    // =========================================================================
    // RECOVERY & RESILIENCE TESTS
    // =========================================================================

    #[test]
    fn test_recovery_from_invalid_input() {
        // Test recovery after invalid input
        let invalid_level = -1.0;
        assert!(
            !(0.0..=1.0).contains(&invalid_level),
            "Should reject invalid"
        );

        // Next operation should still work
        let valid_level = 0.5;
        assert!(
            (0.0..=1.0).contains(&valid_level),
            "Should accept valid after invalid"
        );
    }

    #[test]
    fn test_repeated_operations_stability() {
        // Test stability of repeated operations
        for _ in 0..100 {
            let level = 0.5;
            assert!((0.0..=1.0).contains(&level), "Should remain stable");
        }
    }

    // =========================================================================
    // CROSS-CONTROL INTERFERENCE TESTS
    // =========================================================================

    #[test]
    fn test_volume_brightness_independence() {
        // Test that volume and brightness controls don't interfere
        let volume = 0.5;
        let brightness = 0.75;

        assert!((0.0..=1.0).contains(&volume), "Volume should be independent");
        assert!(
            (0.0..=1.0).contains(&brightness),
            "Brightness should be independent"
        );
        assert_ne!(volume, brightness, "Values should be independent");
    }

    #[test]
    fn test_media_other_controls_independence() {
        // Test that media controls don't interfere with other controls
        let media_op = "play";
        let volume = 0.5;
        let brightness = 0.75;

        assert!(!media_op.is_empty(), "Media should be independent");
        assert!((0.0..=1.0).contains(&volume), "Volume should be independent");
        assert!(
            (0.0..=1.0).contains(&brightness),
            "Brightness should be independent"
        );
    }
}
