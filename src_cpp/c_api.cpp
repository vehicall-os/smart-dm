/**
 * @file c_api.cpp
 * @brief FFI boundary - extern "C" API for Rust integration
 * 
 * This file wraps all C++ implementations with exception-safe,
 * C ABI-compatible functions that can be called from Rust.
 */

#include "can_obd_driver.h"

#include <cstring>
#include <exception>
#include <mutex>
#include <atomic>

// Thread-local error buffer
thread_local char g_last_error[256] = {0};

// Global initialization state
static std::atomic<bool> g_driver_initialized{false};
static std::mutex g_init_mutex;

// Forward declarations for internal functions
extern int obd_client_init(const char* device, int baud_rate);
extern void obd_client_shutdown();
extern int obd_client_query_pid(uint8_t mode, uint8_t pid,
                                 uint8_t* data_out, size_t* data_len_out,
                                 size_t max_len);

namespace {

void set_last_error(const char* msg) {
    if (msg) {
        std::strncpy(g_last_error, msg, sizeof(g_last_error) - 1);
        g_last_error[sizeof(g_last_error) - 1] = '\0';
    } else {
        g_last_error[0] = '\0';
    }
}

template<typename Func>
int safe_call(Func&& func, const char* error_context) {
    try {
        return func();
    } catch (const std::exception& e) {
        char buf[256];
        snprintf(buf, sizeof(buf), "%s: %s", error_context, e.what());
        set_last_error(buf);
        return CAN_ERROR_UNKNOWN;
    } catch (...) {
        char buf[256];
        snprintf(buf, sizeof(buf), "%s: unknown exception", error_context);
        set_last_error(buf);
        return CAN_ERROR_UNKNOWN;
    }
}

} // anonymous namespace

extern "C" {

int can_driver_init(const c_driver_config_t* config) {
    return safe_call([&]() -> int {
        std::lock_guard<std::mutex> lock(g_init_mutex);
        
        if (g_driver_initialized.load()) {
            set_last_error("Driver already initialized");
            return CAN_OK;  // Idempotent
        }
        
        if (!config) {
            set_last_error("Null configuration");
            return CAN_ERROR_INIT;
        }
        
        // Initialize CAN driver (handled in can_driver.cpp)
        // This is called from the extern declaration
        
        // Initialize ELM327 if configured
        if (config->use_elm327 && config->serial_device) {
            int baud = config->serial_baud_rate > 0 ? 
                       config->serial_baud_rate : 38400;
            int ret = obd_client_init(config->serial_device, baud);
            if (ret != CAN_OK) {
                set_last_error("Failed to initialize ELM327 client");
                return ret;
            }
        }
        
        g_driver_initialized.store(true);
        set_last_error(nullptr);
        return CAN_OK;
    }, "can_driver_init");
}

void can_driver_shutdown() {
    try {
        std::lock_guard<std::mutex> lock(g_init_mutex);
        
        if (!g_driver_initialized.load()) {
            return;  // Already shutdown
        }
        
        obd_client_shutdown();
        g_driver_initialized.store(false);
        set_last_error(nullptr);
    } catch (...) {
        // Ignore exceptions during shutdown
    }
}

int can_driver_is_initialized() {
    return g_driver_initialized.load() ? 1 : 0;
}

const char* can_driver_last_error() {
    return g_last_error;
}

const char* can_driver_error_str(int code) {
    switch (code) {
        case CAN_OK: 
            return "OK";
        case CAN_ERROR_INIT: 
            return "Initialization error";
        case CAN_ERROR_NOT_INITIALIZED: 
            return "Driver not initialized";
        case CAN_ERROR_BUS_OFF: 
            return "CAN bus off";
        case CAN_ERROR_NO_ACK: 
            return "No ACK received";
        case CAN_ERROR_TIMEOUT: 
            return "Timeout";
        case CAN_ERROR_SERIAL_OPEN: 
            return "Failed to open serial port";
        case CAN_ERROR_SERIAL_TIMEOUT: 
            return "Serial port timeout";
        case CAN_ERROR_PROTOCOL_MISMATCH: 
            return "Protocol mismatch";
        case CAN_ERROR_INVALID_RESPONSE: 
            return "Invalid response";
        case CAN_ERROR_NO_DATA: 
            return "No data available";
        case CAN_ERROR_UNKNOWN:
        default: 
            return "Unknown error";
    }
}

int can_driver_query_pid(uint8_t mode, uint8_t pid,
                         uint8_t* data_out, size_t* data_len_out,
                         size_t max_len) {
    return safe_call([&]() -> int {
        if (!g_driver_initialized.load()) {
            set_last_error("Driver not initialized");
            return CAN_ERROR_NOT_INITIALIZED;
        }
        
        return obd_client_query_pid(mode, pid, data_out, data_len_out, max_len);
    }, "can_driver_query_pid");
}

} // extern "C"
