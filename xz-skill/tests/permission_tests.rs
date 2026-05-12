use xz_skill::{
    PermissionValidator,
    SkillPermission, SkillError,
};

#[test]
fn test_permission_denied_network() {
    let validator = PermissionValidator::new(false, vec![]);
    let err = validator.check(&SkillPermission::Network).unwrap_err();
    assert!(matches!(err, SkillError::PermissionDenied { .. }));
}

#[test]
fn test_permission_allowed_network() {
    let validator = PermissionValidator::new(true, vec![]);
    assert!(validator.check(&SkillPermission::Network).is_ok());
}

#[test]
fn test_permission_denied_execute() {
    let validator = PermissionValidator::new(true, vec!["/tmp".into()]);
    let err = validator.check(&SkillPermission::Execute).unwrap_err();
    assert!(matches!(err, SkillError::PermissionDenied { .. }));
}

#[test]
fn test_permission_check_all() {
    let validator = PermissionValidator::new(false, vec![]);
    let err = validator
        .check_all(&[SkillPermission::Network, SkillPermission::FileRead])
        .unwrap_err();
    assert!(matches!(err, SkillError::PermissionDenied { .. }));
}
