#pragma once
/**
 * @file camera_capture.h
 * @brief C API for V4L2 camera capture - multi-camera support
 */

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Pixel formats supported
 */
typedef enum {
    PIXEL_FORMAT_RGB24 = 0,
    PIXEL_FORMAT_MJPEG = 1,
    PIXEL_FORMAT_H264 = 2,
    PIXEL_FORMAT_YUYV = 3,
    PIXEL_FORMAT_NV12 = 4,
} c_pixel_format_t;

/**
 * @brief Camera type enumeration
 */
typedef enum {
    CAMERA_TYPE_CABIN_IR = 0,   // Cabin-facing IR camera (DMS)
    CAMERA_TYPE_ROAD = 1,        // Road-facing dashcam (ADAS)
    CAMERA_TYPE_EXTERNAL = 2,    // External USB camera
} c_camera_type_t;

/**
 * @brief Video frame structure
 */
typedef struct {
    uint8_t* data;              /**< Frame data pointer */
    size_t size;                /**< Data size in bytes */
    uint32_t width;             /**< Frame width */
    uint32_t height;            /**< Frame height */
    uint32_t stride;            /**< Bytes per row */
    c_pixel_format_t format;    /**< Pixel format */
    uint64_t timestamp_ns;      /**< Capture timestamp (CLOCK_MONOTONIC) */
    uint32_t sequence;          /**< Frame sequence number */
    int32_t buffer_id;          /**< Internal buffer ID */
} c_video_frame_t;

/**
 * @brief Camera configuration
 */
typedef struct {
    const char* device;         /**< V4L2 device path (e.g., "/dev/video0") */
    c_camera_type_t type;       /**< Camera type */
    uint32_t width;             /**< Capture width */
    uint32_t height;            /**< Capture height */
    uint32_t fps;               /**< Target framerate */
    c_pixel_format_t format;    /**< Desired format */
    int enable_ir;              /**< Enable IR mode (cabin camera) */
    int buffer_count;           /**< V4L2 buffer count (default: 4) */
} c_camera_config_t;

/**
 * @brief Camera error codes
 */
typedef enum {
    CAM_OK = 0,
    CAM_ERROR_OPEN = -1,
    CAM_ERROR_FORMAT = -2,
    CAM_ERROR_BUFFER = -3,
    CAM_ERROR_STREAM = -4,
    CAM_ERROR_CAPTURE = -5,
    CAM_ERROR_NOT_INITIALIZED = -10,
    CAM_ERROR_TIMEOUT = -11,
    CAM_ERROR_UNKNOWN = -99,
} c_camera_error_t;

/* ==== Camera Lifecycle ==== */

/**
 * @brief Initialize a camera
 * @param camera_id Unique camera identifier (0-3)
 * @param config Camera configuration
 * @return CAM_OK on success
 */
int camera_init(int camera_id, const c_camera_config_t* config);

/**
 * @brief Start camera streaming
 * @param camera_id Camera identifier
 * @return CAM_OK on success
 */
int camera_start(int camera_id);

/**
 * @brief Stop camera streaming
 * @param camera_id Camera identifier
 */
void camera_stop(int camera_id);

/**
 * @brief Shutdown camera and release resources
 * @param camera_id Camera identifier
 */
void camera_shutdown(int camera_id);

/* ==== Frame Capture ==== */

/**
 * @brief Read next frame (non-blocking)
 * @param camera_id Camera identifier
 * @param timeout_ms Timeout in milliseconds (0 = non-blocking)
 * @return Pointer to frame, or NULL if no frame available
 * @note Caller must call camera_release_frame() when done
 */
c_video_frame_t* camera_read_frame(int camera_id, int timeout_ms);

/**
 * @brief Release frame back to capture buffer
 * @param camera_id Camera identifier
 * @param frame Frame to release
 */
void camera_release_frame(int camera_id, c_video_frame_t* frame);

/* ==== Utilities ==== */

/**
 * @brief Get last error message
 * @param camera_id Camera identifier
 * @return Error string (thread-local)
 */
const char* camera_last_error(int camera_id);

/**
 * @brief Check if camera is streaming
 * @param camera_id Camera identifier
 * @return 1 if streaming, 0 otherwise
 */
int camera_is_streaming(int camera_id);

/**
 * @brief Get camera configuration
 * @param camera_id Camera identifier
 * @param config_out Output configuration
 * @return CAM_OK on success
 */
int camera_get_config(int camera_id, c_camera_config_t* config_out);

#ifdef __cplusplus
}
#endif
