/**
 * @file can_driver.cpp
 * @brief CAN bus driver implementation using SocketCAN
 */

#include "can_obd_driver.h"

#include <cstring>
#include <chrono>
#include <atomic>
#include <mutex>

#if HAS_SOCKETCAN
#include <sys/socket.h>
#include <sys/ioctl.h>
#include <net/if.h>
#include <linux/can.h>
#include <linux/can/raw.h>
#include <unistd.h>
#include <fcntl.h>
#include <poll.h>
#endif

namespace {

// Thread-local error message buffer
thread_local char g_error_buffer[256] = {0};

// Global state
std::atomic<bool> g_initialized{false};
std::mutex g_mutex;

#if HAS_SOCKETCAN
int g_socket_fd = -1;
#endif

// Mock mode frame generator
std::atomic<uint64_t> g_mock_frame_count{0};

void set_error(const char* msg) {
    std::strncpy(g_error_buffer, msg, sizeof(g_error_buffer) - 1);
    g_error_buffer[sizeof(g_error_buffer) - 1] = '\0';
}

uint64_t get_timestamp_ns() {
    auto now = std::chrono::steady_clock::now();
    auto duration = now.time_since_epoch();
    return std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
}

#if HAS_SOCKETCAN
int init_socketcan(const char* interface) {
    // Create socket
    g_socket_fd = socket(PF_CAN, SOCK_RAW, CAN_RAW);
    if (g_socket_fd < 0) {
        set_error("Failed to create CAN socket");
        return CAN_ERROR_INIT;
    }

    // Get interface index
    struct ifreq ifr;
    std::strncpy(ifr.ifr_name, interface, IFNAMSIZ - 1);
    if (ioctl(g_socket_fd, SIOCGIFINDEX, &ifr) < 0) {
        close(g_socket_fd);
        g_socket_fd = -1;
        set_error("Failed to get interface index");
        return CAN_ERROR_INIT;
    }

    // Bind socket
    struct sockaddr_can addr;
    addr.can_family = AF_CAN;
    addr.can_ifindex = ifr.ifr_ifindex;
    if (bind(g_socket_fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(g_socket_fd);
        g_socket_fd = -1;
        set_error("Failed to bind CAN socket");
        return CAN_ERROR_INIT;
    }

    // Set non-blocking mode
    int flags = fcntl(g_socket_fd, F_GETFL, 0);
    fcntl(g_socket_fd, F_SETFL, flags | O_NONBLOCK);

    return CAN_OK;
}
#endif

c_can_frame_t generate_mock_frame() {
    c_can_frame_t frame;
    uint64_t count = g_mock_frame_count.fetch_add(1);
    
    frame.timestamp_ns = get_timestamp_ns();
    frame.can_id = 0x7E8;  // Standard OBD-II response ID
    frame.dlc = 8;
    
    // Generate mock OBD-II data
    uint8_t mock_pid = (count % 8);
    frame.data[0] = 0x04;  // Additional bytes
    frame.data[1] = 0x41;  // Mode 1 response
    
    switch (mock_pid) {
        case 0: // RPM
            frame.data[2] = 0x0C;
            frame.data[3] = ((2500 + (count % 500)) >> 8) & 0xFF;
            frame.data[4] = ((2500 + (count % 500)) * 4) & 0xFF;
            break;
        case 1: // Coolant temp
            frame.data[2] = 0x05;
            frame.data[3] = 85 + 40;  // 85°C
            break;
        case 2: // Speed
            frame.data[2] = 0x0D;
            frame.data[3] = 60 + (count % 20);  // 60-80 km/h
            break;
        case 3: // Engine load
            frame.data[2] = 0x04;
            frame.data[3] = 40 + (count % 30);  // 40-70%
            break;
        default:
            frame.data[2] = 0x00;
            frame.data[3] = 0x00;
    }
    
    frame.data[5] = 0x00;
    frame.data[6] = 0x00;
    frame.data[7] = 0x00;
    
    return frame;
}

} // anonymous namespace

extern "C" {

int can_driver_init(const c_driver_config_t* config) {
    std::lock_guard<std::mutex> lock(g_mutex);
    
    if (g_initialized.load()) {
        set_error("Driver already initialized");
        return CAN_ERROR_INIT;
    }
    
    if (!config) {
        set_error("Null configuration");
        return CAN_ERROR_INIT;
    }

#if HAS_SOCKETCAN
    if (config->can_interface && std::strlen(config->can_interface) > 0) {
        int ret = init_socketcan(config->can_interface);
        if (ret != CAN_OK) {
            return ret;
        }
    }
#else
    // Mock mode on non-Linux platforms
    (void)config;
#endif

    g_initialized.store(true);
    g_mock_frame_count.store(0);
    return CAN_OK;
}

void can_driver_shutdown() {
    std::lock_guard<std::mutex> lock(g_mutex);
    
#if HAS_SOCKETCAN
    if (g_socket_fd >= 0) {
        close(g_socket_fd);
        g_socket_fd = -1;
    }
#endif

    g_initialized.store(false);
}

int can_driver_is_initialized() {
    return g_initialized.load() ? 1 : 0;
}

int can_driver_read_frame(c_can_frame_t* frame_out) {
    if (!g_initialized.load()) {
        set_error("Driver not initialized");
        return CAN_ERROR_NOT_INITIALIZED;
    }
    
    if (!frame_out) {
        set_error("Null output pointer");
        return CAN_ERROR_INIT;
    }

#if HAS_SOCKETCAN
    if (g_socket_fd >= 0) {
        // Poll for available data (non-blocking)
        struct pollfd pfd;
        pfd.fd = g_socket_fd;
        pfd.events = POLLIN;
        
        int ret = poll(&pfd, 1, 0);  // Immediate timeout
        if (ret < 0) {
            set_error("Poll error");
            return CAN_ERROR_UNKNOWN;
        }
        if (ret == 0) {
            return 0;  // No data available
        }
        
        // Read frame
        struct can_frame raw_frame;
        ssize_t nbytes = read(g_socket_fd, &raw_frame, sizeof(raw_frame));
        if (nbytes < 0) {
            set_error("Read error");
            return CAN_ERROR_UNKNOWN;
        }
        if (nbytes < static_cast<ssize_t>(sizeof(raw_frame))) {
            return 0;  // Incomplete frame
        }
        
        // Copy to output
        frame_out->can_id = raw_frame.can_id;
        frame_out->dlc = raw_frame.can_dlc;
        std::memcpy(frame_out->data, raw_frame.data, 8);
        frame_out->timestamp_ns = get_timestamp_ns();
        
        return 1;
    }
#endif

    // Mock mode: generate synthetic frames
    *frame_out = generate_mock_frame();
    return 1;
}

int can_driver_read_sensor_frame(c_sensor_frame_t* frame_out) {
    if (!frame_out) {
        return CAN_ERROR_INIT;
    }
    
    // For now, read raw frame and decode
    c_can_frame_t raw;
    int ret = can_driver_read_frame(&raw);
    if (ret <= 0) {
        return ret;
    }
    
    // Basic decoding (mode 0x41 responses)
    frame_out->timestamp_ns = raw.timestamp_ns;
    frame_out->valid_mask = 0;
    
    if (raw.data[1] == 0x41) {  // Mode 1 response
        uint8_t pid = raw.data[2];
        switch (pid) {
            case 0x0C:  // RPM
                frame_out->rpm = ((raw.data[3] << 8) | raw.data[4]) / 4;
                frame_out->valid_mask |= 0x01;
                break;
            case 0x05:  // Coolant temp
                frame_out->coolant_temp = raw.data[3] - 40;
                frame_out->valid_mask |= 0x02;
                break;
            case 0x0D:  // Speed
                frame_out->speed = raw.data[3];
                frame_out->valid_mask |= 0x04;
                break;
            case 0x04:  // Engine load
                frame_out->engine_load = (raw.data[3] * 100) / 255;
                frame_out->valid_mask |= 0x08;
                break;
        }
    }
    
    return 1;
}

int can_driver_query_pid(uint8_t mode, uint8_t pid,
                         uint8_t* data_out, size_t* data_len_out,
                         size_t max_len) {
    (void)mode; (void)pid; (void)data_out; (void)data_len_out; (void)max_len;
    // TODO: Implement PID query via ELM327 or SocketCAN
    set_error("PID query not implemented");
    return CAN_ERROR_UNKNOWN;
}

const char* can_driver_last_error() {
    return g_error_buffer;
}

const char* can_driver_error_str(int code) {
    switch (code) {
        case CAN_OK: return "OK";
        case CAN_ERROR_INIT: return "Initialization error";
        case CAN_ERROR_NOT_INITIALIZED: return "Driver not initialized";
        case CAN_ERROR_BUS_OFF: return "CAN bus off";
        case CAN_ERROR_NO_ACK: return "No ACK received";
        case CAN_ERROR_TIMEOUT: return "Timeout";
        case CAN_ERROR_SERIAL_OPEN: return "Serial port open error";
        case CAN_ERROR_SERIAL_TIMEOUT: return "Serial timeout";
        case CAN_ERROR_PROTOCOL_MISMATCH: return "Protocol mismatch";
        case CAN_ERROR_INVALID_RESPONSE: return "Invalid response";
        case CAN_ERROR_NO_DATA: return "No data available";
        default: return "Unknown error";
    }
}

} // extern "C"
