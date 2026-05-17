use std::path::PathBuf;

use crate::error::SkillError;
use crate::types::skill::SkillPermission;

/// Validates skill permissions against the allowed context.
#[derive(Debug)]
pub struct PermissionValidator {
    allowed_network: bool,
    allowed_paths: Vec<PathBuf>,
}

impl PermissionValidator {
    pub fn new(allowed_network: bool, allowed_paths: Vec<PathBuf>) -> Self {
        Self {
            allowed_network,
            allowed_paths,
        }
    }

    /// Check a single permission. Returns Err(PermissionDenied) if not granted.
    pub fn check(&self, perm: &SkillPermission) -> Result<(), SkillError> {
        match perm {
            SkillPermission::Network => {
                if !self.allowed_network {
                    return Err(SkillError::PermissionDenied {
                        required: vec![SkillPermission::Network],
                    });
                }
            }
            SkillPermission::FileRead | SkillPermission::FileWrite => {
                if self.allowed_paths.is_empty() {
                    return Err(SkillError::PermissionDenied {
                        required: vec![perm.clone()],
                    });
                }
            }
            SkillPermission::Execute => {
                // Execute permission always requires explicit allow
                return Err(SkillError::PermissionDenied {
                    required: vec![SkillPermission::Execute],
                });
            }
            SkillPermission::Custom(tag) => {
                return Err(SkillError::PermissionDenied {
                    required: vec![SkillPermission::Custom(tag.clone())],
                });
            }
        }
        Ok(())
    }

    /// Check all required permissions.
    pub fn check_all(&self, perms: &[SkillPermission]) -> Result<(), SkillError> {
        let denied: Vec<SkillPermission> = perms
            .iter()
            .filter(|p| self.check(p).is_err())
            .cloned()
            .collect();

        if !denied.is_empty() {
            return Err(SkillError::PermissionDenied { required: denied });
        }
        Ok(())
    }
}
