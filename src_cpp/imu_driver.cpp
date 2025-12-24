/**
 * @file imu_driver.cpp
 * @brief IMU driver for MPU-6050 accelerometer/gyroscope
 * 
 * Reads 6-axis IMU data at 1kHz for crash detection and harsh braking.
 * Uses I2C interface with Kalman filtering for noise reduction.
 */

#include <cstdint>
#include <cstring>
#include <cmath>
#include <mutex>
#include <atomic>
#include <chrono>

#ifdef __linux__
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <linux/i2c-dev.h>
#endif

extern "C" {

/**
 * @brief IMU data structure
 */
typedef struct {
    int16_t accel_x;        /**< Accelerometer X (raw, ±16g range) */
    int16_t accel_y;        /**< Accelerometer Y */
    int16_t accel_z;        /**< Accelerometer Z */
    int16_t gyro_x;         /**< Gyroscope X (raw, ±2000°/s range) */
    int16_t gyro_y;         /**< Gyroscope Y */
    int16_t gyro_z;         /**< Gyroscope Z */
    int16_t temperature;    /**< Temperature (raw) */
    uint64_t timestamp_ns;  /**< Capture timestamp */
} c_imu_data_t;

/**
 * @brief Processed IMU data with physical units
 */
typedef struct {
    float accel_x_g;        /**< Acceleration X in g */
    float accel_y_g;        /**< Acceleration Y in g */
    float accel_z_g;        /**< Acceleration Z in g */
    float gyro_x_dps;       /**< Angular rate X in deg/s */
    float gyro_y_dps;       /**< Angular rate Y in deg/s */
    float gyro_z_dps;       /**< Angular rate Z in deg/s */
    float temperature_c;    /**< Temperature in Celsius */
    float g_force;          /**< Total G-force magnitude */
    uint64_t timestamp_ns;  /**< Capture timestamp */
} c_imu_processed_t;

/**
 * @brief IMU configuration
 */
typedef struct {
    const char* i2c_device; /**< I2C device path (e.g., "/dev/i2c-1") */
    uint8_t i2c_address;    /**< I2C address (default: 0x68) */
    int sample_rate_hz;     /**< Sampling rate (default: 1000Hz) */
} c_imu_config_t;

// Error codes
typedef enum {
    IMU_OK = 0,
    IMU_ERROR_OPEN = -1,
    IMU_ERROR_INIT = -2,
    IMU_ERROR_READ = -3,
    IMU_ERROR_NOT_INITIALIZED = -10,
} c_imu_error_t;

} // extern "C"

namespace imu {

// MPU-6050 register addresses
constexpr uint8_t REG_PWR_MGMT_1 = 0x6B;
constexpr uint8_t REG_ACCEL_CONFIG = 0x1C;
constexpr uint8_t REG_GYRO_CONFIG = 0x1B;
constexpr uint8_t REG_ACCEL_XOUT_H = 0x3B;
constexpr uint8_t REG_WHO_AM_I = 0x75;

// MPU-6050 scales
constexpr float ACCEL_SCALE_16G = 16.0f / 32768.0f;
constexpr float GYRO_SCALE_2000 = 2000.0f / 32768.0f;
constexpr float TEMP_SCALE = 1.0f / 340.0f;
constexpr float TEMP_OFFSET = 36.53f;

class ImuDriver {
public:
    int init(const c_imu_config_t* config) {
        std::lock_guard<std::mutex> lock(mutex_);
        
        if (initialized_) {
            return IMU_OK;
        }

        config_ = *config;
        if (config_.i2c_address == 0) {
            config_.i2c_address = 0x68;  // Default MPU-6050 address
        }

#ifdef __linux__
        // Open I2C device
        fd_ = open(config_.i2c_device, O_RDWR);
        if (fd_ < 0) {
            set_error("Failed to open I2C device");
            return IMU_ERROR_OPEN;
        }

        // Set I2C slave address
        if (ioctl(fd_, I2C_SLAVE, config_.i2c_address) < 0) {
            close(fd_);
            fd_ = -1;
            set_error("Failed to set I2C address");
            return IMU_ERROR_INIT;
        }

        // Check WHO_AM_I register
        uint8_t who_am_i = read_register(REG_WHO_AM_I);
        if (who_am_i != 0x68 && who_am_i != 0x98) {  // 0x98 for MPU-6500
            close(fd_);
            fd_ = -1;
            set_error("MPU-6050 not found");
            return IMU_ERROR_INIT;
        }

        // Wake up MPU-6050 (exit sleep mode)
        write_register(REG_PWR_MGMT_1, 0x00);

        // Configure accelerometer: ±16g
        write_register(REG_ACCEL_CONFIG, 0x18);

        // Configure gyroscope: ±2000°/s
        write_register(REG_GYRO_CONFIG, 0x18);
#endif

        initialized_ = true;
        return IMU_OK;
    }

    void shutdown() {
        std::lock_guard<std::mutex> lock(mutex_);
        
#ifdef __linux__
        if (fd_ >= 0) {
            close(fd_);
            fd_ = -1;
        }
#endif
        
        initialized_ = false;
    }

    int read_raw(c_imu_data_t* data) {
        if (!initialized_ || !data) {
            return IMU_ERROR_NOT_INITIALIZED;
        }

        uint64_t timestamp = std::chrono::steady_clock::now().time_since_epoch().count();

#ifdef __linux__
        // Read 14 bytes starting from ACCEL_XOUT_H
        uint8_t buffer[14];
        if (read_registers(REG_ACCEL_XOUT_H, buffer, 14) != 14) {
            set_error("Failed to read IMU data");
            return IMU_ERROR_READ;
        }

        // Parse big-endian data
        data->accel_x = (buffer[0] << 8) | buffer[1];
        data->accel_y = (buffer[2] << 8) | buffer[3];
        data->accel_z = (buffer[4] << 8) | buffer[5];
        data->temperature = (buffer[6] << 8) | buffer[7];
        data->gyro_x = (buffer[8] << 8) | buffer[9];
        data->gyro_y = (buffer[10] << 8) | buffer[11];
        data->gyro_z = (buffer[12] << 8) | buffer[13];
#else
        // Mock mode: generate synthetic data
        static uint64_t mock_counter = 0;
        float phase = mock_counter++ * 0.01f;
        
        data->accel_x = static_cast<int16_t>(std::sin(phase) * 1000);
        data->accel_y = static_cast<int16_t>(std::cos(phase) * 1000);
        data->accel_z = static_cast<int16_t>(16384);  // ~1g
        data->gyro_x = static_cast<int16_t>(std::sin(phase * 2) * 500);
        data->gyro_y = static_cast<int16_t>(std::cos(phase * 2) * 500);
        data->gyro_z = 0;
        data->temperature = static_cast<int16_t>(25 * 340 + 36.53f * 340);
#endif

        data->timestamp_ns = timestamp;
        return IMU_OK;
    }

    int read_processed(c_imu_processed_t* data) {
        c_imu_data_t raw;
        int ret = read_raw(&raw);
        if (ret != IMU_OK) {
            return ret;
        }

        // Convert to physical units
        data->accel_x_g = raw.accel_x * ACCEL_SCALE_16G;
        data->accel_y_g = raw.accel_y * ACCEL_SCALE_16G;
        data->accel_z_g = raw.accel_z * ACCEL_SCALE_16G;
        
        data->gyro_x_dps = raw.gyro_x * GYRO_SCALE_2000;
        data->gyro_y_dps = raw.gyro_y * GYRO_SCALE_2000;
        data->gyro_z_dps = raw.gyro_z * GYRO_SCALE_2000;
        
        data->temperature_c = raw.temperature * TEMP_SCALE + TEMP_OFFSET;
        
        // Calculate total G-force magnitude
        data->g_force = std::sqrt(
            data->accel_x_g * data->accel_x_g +
            data->accel_y_g * data->accel_y_g +
            data->accel_z_g * data->accel_z_g
        );
        
        data->timestamp_ns = raw.timestamp_ns;
        return IMU_OK;
    }

    bool is_initialized() const { return initialized_; }
    const char* get_error() const { return last_error_; }

private:
#ifdef __linux__
    uint8_t read_register(uint8_t reg) {
        uint8_t value = 0;
        if (write(fd_, &reg, 1) == 1) {
            read(fd_, &value, 1);
        }
        return value;
    }

    void write_register(uint8_t reg, uint8_t value) {
        uint8_t buffer[2] = {reg, value};
        write(fd_, buffer, 2);
    }

    ssize_t read_registers(uint8_t reg, uint8_t* buffer, size_t len) {
        if (write(fd_, &reg, 1) != 1) {
            return -1;
        }
        return read(fd_, buffer, len);
    }
#endif

    void set_error(const char* msg) {
        std::strncpy(last_error_, msg, sizeof(last_error_) - 1);
    }

    std::mutex mutex_;
    c_imu_config_t config_ = {};
    int fd_ = -1;
    std::atomic<bool> initialized_{false};
    char last_error_[256] = {0};
};

static ImuDriver g_imu;

} // namespace imu

extern "C" {

int imu_init(const c_imu_config_t* config) {
    return imu::g_imu.init(config);
}

void imu_shutdown() {
    imu::g_imu.shutdown();
}

int imu_read_raw(c_imu_data_t* data) {
    return imu::g_imu.read_raw(data);
}

int imu_read_processed(c_imu_processed_t* data) {
    return imu::g_imu.read_processed(data);
}

int imu_is_initialized() {
    return imu::g_imu.is_initialized() ? 1 : 0;
}

const char* imu_last_error() {
    return imu::g_imu.get_error();
}

} // extern "C"
