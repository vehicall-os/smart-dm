#pragma once
/**
 * @file can_obd_driver.h
 * @brief C API for CAN/OBD-II driver - FFI boundary for Rust
 * 
 * This header defines the C ABI for interoperability with Rust.
 * All functions are thread-safe and exception-free.
 */

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief CAN frame structure (matches Linux can_frame layout)
 */
typedef struct {
    uint32_t can_id;        /**< CAN identifier (11 or 29 bit) */
    uint8_t dlc;            /**< Data length code (0-8) */
    uint8_t data[8];        /**< Frame data */
    uint64_t timestamp_ns;  /**< Timestamp from CLOCK_MONOTONIC */
} c_can_frame_t;

/**
 * @brief OBD-II sensor frame with decoded values
 */
typedef struct {
    uint64_t timestamp_ns;  /**< Capture timestamp */
    uint16_t rpm;           /**< Engine RPM */
    uint8_t coolant_temp;   /**< Coolant temperature (°C, offset -40) */
    uint8_t speed;          /**< Vehicle speed (km/h) */
    uint8_t engine_load;    /**< Engine load (%) */
    uint16_t maf;           /**< MAF sensor (g/s * 100) */
    uint8_t throttle_pos;   /**< Throttle position (%) */
    int8_t fuel_trim_short; /**< Short term fuel trim (%) */
    int8_t fuel_trim_long;  /**< Long term fuel trim (%) */
    uint8_t valid_mask;     /**< Bitmask of valid fields */
} c_sensor_frame_t;

/**
 * @brief Driver configuration
 */
typedef struct {
    const char* can_interface;  /**< CAN interface name (e.g., "can0", "vcan0") */
    const char* serial_device;  /**< Serial device path (e.g., "/dev/ttyUSB0") */
    int serial_baud_rate;       /**< Baud rate for ELM327 (default: 38400) */
    int use_elm327;             /**< 1 = ELM327, 0 = SocketCAN only */
} c_driver_config_t;

/**
 * @brief Error codes
 */
typedef enum {
    CAN_OK = 0,
    CAN_ERROR_INIT = -1,
    CAN_ERROR_NOT_INITIALIZED = -2,
    CAN_ERROR_BUS_OFF = -10,
    CAN_ERROR_NO_ACK = -11,
    CAN_ERROR_TIMEOUT = -12,
    CAN_ERROR_SERIAL_OPEN = -20,
    CAN_ERROR_SERIAL_TIMEOUT = -21,
    CAN_ERROR_PROTOCOL_MISMATCH = -30,
    CAN_ERROR_INVALID_RESPONSE = -31,
    CAN_ERROR_NO_DATA = -40,
    CAN_ERROR_UNKNOWN = -99
} c_can_error_t;

/* ==== Initialization & Shutdown ==== */

/**
 * @brief Initialize the driver
 * @param config Driver configuration
 * @return CAN_OK on success, error code on failure
 * @note This function may block for up to 500ms during initialization
 */
int can_driver_init(const c_driver_config_t* config);

/**
 * @brief Shutdown the driver and release resources
 * @note Safe to call multiple times
 */
void can_driver_shutdown(void);

/**
 * @brief Check if driver is initialized
 * @return 1 if initialized, 0 otherwise
 */
int can_driver_is_initialized(void);

/* ==== Frame Reading ==== */

/**
 * @brief Read a raw CAN frame (non-blocking)
 * @param frame_out Pointer to receive the frame
 * @return 1 if frame available, 0 if no data, negative on error
 * @note Thread-safe, can be called from multiple threads
 */
int can_driver_read_frame(c_can_frame_t* frame_out);

/**
 * @brief Read a decoded sensor frame (non-blocking)
 * @param frame_out Pointer to receive the decoded frame
 * @return 1 if frame available, 0 if no data, negative on error
 * @note Combines data from multiple PID responses
 */
int can_driver_read_sensor_frame(c_sensor_frame_t* frame_out);

/* ==== PID Operations ==== */

/**
 * @brief Query a specific OBD-II PID
 * @param mode OBD-II mode (0x01 = current, 0x02 = freeze frame)
 * @param pid PID code
 * @param data_out Buffer for response data
 * @param data_len_out Pointer to receive data length
 * @param max_len Maximum buffer size
 * @return CAN_OK on success, error code on failure
 * @note This function may block for up to 100ms
 */
int can_driver_query_pid(uint8_t mode, uint8_t pid, 
                         uint8_t* data_out, size_t* data_len_out, 
                         size_t max_len);

/* ==== Error Handling ==== */

/**
 * @brief Get last error message
 * @return Pointer to error string (thread-local, valid until next call)
 */
const char* can_driver_last_error(void);

/**
 * @brief Get error code description
 * @param code Error code
 * @return Static string describing the error
 */
const char* can_driver_error_str(int code);

#ifdef __cplusplus
}
#endif
