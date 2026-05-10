    #[test]
    fn stair_delta_uses_request_direction_for_public_stair_family() {
        assert_eq!(stair_delta(80, ClimbIntent::Up), Some(1));
        assert_eq!(stair_delta(80, ClimbIntent::Down), Some(-1));
        assert_eq!(stair_delta(81, ClimbIntent::Down), Some(-1));
        assert_eq!(stair_delta(81, ClimbIntent::Up), Some(1));
        assert_eq!(stair_delta(16, ClimbIntent::Up), None);
    }
