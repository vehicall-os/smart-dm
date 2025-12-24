//! Data Validator for Range Checking

use crate::error::ValidationError;
use serde::{Deserialize, Serialize};

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// RPM valid range
    pub rpm_range: (f64, f64),
    /// Coolant temp valid range (°C)
    pub coolant_range: (f64, f64),
    /// Speed valid range (km/h)
    pub speed_range: (f64, f64),
    /// Engine load valid range (%)
    pub load_range: (f64, f64),
    /// MAF valid range (g/s)
    pub maf_range: (f64, f64),
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            rpm_range: (0.0, 8000.0),
            coolant_range: (-40.0, 215.0),
            speed_range: (0.0, 300.0),
            load_range: (0.0, 100.0),
            maf_range: (0.0, 655.35),
        }
    }
}

/// Result of validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether all values are valid
    pub valid: bool,
    /// List of validation errors
    pub errors: Vec<ValidationError>,
    /// Number of fields validated
    pub fields_checked: usize,
}

impl ValidationResult {
    /// Create a valid result
    pub fn valid(fields_checked: usize) -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            fields_checked,
        }
    }

    /// Create an invalid result with errors
    pub fn invalid(errors: Vec<ValidationError>) -> Self {
        let fields_checked = errors.len();
        Self {
            valid: false,
            errors,
            fields_checked,
        }
    }
}

/// Data validator for OBD-II sensor frames
pub struct Validator {
    config: ValidationConfig,
}

impl Validator {
    /// Create a new validator with given config
    pub fn new(config: ValidationConfig) -> Self {
        Self { config }
    }

    /// Validate a single value against a range
    pub fn validate_range(
        &self,
        field: &'static str,
        value: f64,
        range: (f64, f64),
    ) -> Result<(), ValidationError> {
        if value < range.0 || value > range.1 {
            Err(ValidationError::OutOfRange {
                field,
                value,
                min: range.0,
                max: range.1,
            })
        } else {
            Ok(())
        }
    }

    /// Validate RPM
    pub fn validate_rpm(&self, rpm: f64) -> Result<(), ValidationError> {
        self.validate_range("rpm", rpm, self.config.rpm_range)
    }

    /// Validate coolant temperature
    pub fn validate_coolant_temp(&self, temp: f64) -> Result<(), ValidationError> {
        self.validate_range("coolant_temp", temp, self.config.coolant_range)
    }

    /// Validate speed
    pub fn validate_speed(&self, speed: f64) -> Result<(), ValidationError> {
        self.validate_range("speed", speed, self.config.speed_range)
    }

    /// Validate engine load
    pub fn validate_engine_load(&self, load: f64) -> Result<(), ValidationError> {
        self.validate_range("engine_load", load, self.config.load_range)
    }

    /// Validate MAF
    pub fn validate_maf(&self, maf: f64) -> Result<(), ValidationError> {
        self.validate_range("maf", maf, self.config.maf_range)
    }

    /// Validate OBD-II checksum
    pub fn validate_checksum(&self, data: &[u8], expected: u8) -> Result<(), ValidationError> {
        let calculated: u8 = data.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
        if calculated != expected {
            Err(ValidationError::ChecksumMismatch {
                expected,
                actual: calculated,
            })
        } else {
            Ok(())
        }
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new(ValidationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_rpm() {
        let validator = Validator::default();
        assert!(validator.validate_rpm(3000.0).is_ok());
        assert!(validator.validate_rpm(0.0).is_ok());
        assert!(validator.validate_rpm(8000.0).is_ok());
    }

    #[test]
    fn test_invalid_rpm() {
        let validator = Validator::default();
        assert!(validator.validate_rpm(-100.0).is_err());
        assert!(validator.validate_rpm(10000.0).is_err());
    }

    #[test]
    fn test_coolant_temp_range() {
        let validator = Validator::default();
        assert!(validator.validate_coolant_temp(-40.0).is_ok());
        assert!(validator.validate_coolant_temp(90.0).is_ok());
        assert!(validator.validate_coolant_temp(215.0).is_ok());
        assert!(validator.validate_coolant_temp(-50.0).is_err());
        assert!(validator.validate_coolant_temp(250.0).is_err());
    }

    #[test]
    fn test_checksum() {
        let validator = Validator::default();
        let data = [0x41, 0x0C, 0x1A, 0x2B];
        let checksum = data.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
        assert!(validator.validate_checksum(&data, checksum).is_ok());
        assert!(validator.validate_checksum(&data, checksum.wrapping_add(1)).is_err());
    }
}
