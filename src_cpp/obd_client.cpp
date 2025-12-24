/**
 * @file obd_client.cpp
 * @brief ELM327 OBD-II protocol client
 */

#include "can_obd_driver.h"

#include <cstring>
#include <string>
#include <vector>
#include <mutex>

#ifdef _WIN32
#include <windows.h>
#else
#include <fcntl.h>
#include <termios.h>
#include <unistd.h>
#include <sys/select.h>
#endif

namespace {

std::mutex g_serial_mutex;
int g_serial_fd = -1;
bool g_elm327_initialized = false;

#ifndef _WIN32
int set_serial_options(int fd, int baud_rate) {
    struct termios options;
    
    if (tcgetattr(fd, &options) < 0) {
        return -1;
    }
    
    // Set baud rate
    speed_t speed;
    switch (baud_rate) {
        case 9600: speed = B9600; break;
        case 19200: speed = B19200; break;
        case 38400: speed = B38400; break;
        case 57600: speed = B57600; break;
        case 115200: speed = B115200; break;
        default: speed = B38400;
    }
    cfsetispeed(&options, speed);
    cfsetospeed(&options, speed);
    
    // 8N1
    options.c_cflag &= ~PARENB;
    options.c_cflag &= ~CSTOPB;
    options.c_cflag &= ~CSIZE;
    options.c_cflag |= CS8;
    
    // No flow control
    options.c_cflag &= ~CRTSCTS;
    options.c_cflag |= CLOCAL | CREAD;
    
    // Raw input
    options.c_lflag &= ~(ICANON | ECHO | ECHOE | ISIG);
    options.c_iflag &= ~(IXON | IXOFF | IXANY);
    options.c_oflag &= ~OPOST;
    
    // Timeouts
    options.c_cc[VMIN] = 0;
    options.c_cc[VTIME] = 10;  // 1 second timeout
    
    return tcsetattr(fd, TCSANOW, &options);
}

std::string send_at_command(const std::string& cmd, int timeout_ms = 1000) {
    if (g_serial_fd < 0) {
        return "";
    }
    
    // Send command
    std::string full_cmd = cmd + "\r";
    write(g_serial_fd, full_cmd.c_str(), full_cmd.length());
    
    // Wait for response
    std::string response;
    char buf[256];
    
    fd_set readfds;
    struct timeval tv;
    
    int elapsed = 0;
    while (elapsed < timeout_ms) {
        FD_ZERO(&readfds);
        FD_SET(g_serial_fd, &readfds);
        tv.tv_sec = 0;
        tv.tv_usec = 100000;  // 100ms
        
        int ret = select(g_serial_fd + 1, &readfds, nullptr, nullptr, &tv);
        if (ret > 0) {
            ssize_t n = read(g_serial_fd, buf, sizeof(buf) - 1);
            if (n > 0) {
                buf[n] = '\0';
                response += buf;
                
                // Check for prompt
                if (response.find('>') != std::string::npos) {
                    break;
                }
            }
        }
        elapsed += 100;
    }
    
    return response;
}
#endif

} // anonymous namespace

extern "C" {

int obd_client_init(const char* device, int baud_rate) {
    std::lock_guard<std::mutex> lock(g_serial_mutex);
    
    if (g_elm327_initialized) {
        return CAN_OK;  // Already initialized
    }
    
#ifdef _WIN32
    // Windows serial port implementation
    (void)device;
    (void)baud_rate;
    // TODO: Implement Windows COM port handling
    return CAN_ERROR_SERIAL_OPEN;
#else
    // Open serial port
    g_serial_fd = open(device, O_RDWR | O_NOCTTY);
    if (g_serial_fd < 0) {
        return CAN_ERROR_SERIAL_OPEN;
    }
    
    // Configure serial port
    if (set_serial_options(g_serial_fd, baud_rate) < 0) {
        close(g_serial_fd);
        g_serial_fd = -1;
        return CAN_ERROR_SERIAL_OPEN;
    }
    
    // Initialize ELM327
    send_at_command("ATZ", 2000);  // Reset
    send_at_command("ATE0");       // Echo off
    send_at_command("ATL0");       // Linefeeds off
    send_at_command("ATS0");       // Spaces off
    send_at_command("ATH0");       // Headers off
    send_at_command("ATSP0");      // Auto-detect protocol
    
    g_elm327_initialized = true;
    return CAN_OK;
#endif
}

void obd_client_shutdown() {
    std::lock_guard<std::mutex> lock(g_serial_mutex);
    
#ifndef _WIN32
    if (g_serial_fd >= 0) {
        close(g_serial_fd);
        g_serial_fd = -1;
    }
#endif
    
    g_elm327_initialized = false;
}

int obd_client_query_pid(uint8_t mode, uint8_t pid, 
                         uint8_t* data_out, size_t* data_len_out,
                         size_t max_len) {
    std::lock_guard<std::mutex> lock(g_serial_mutex);
    
    if (!g_elm327_initialized) {
        return CAN_ERROR_NOT_INITIALIZED;
    }
    
#ifndef _WIN32
    // Format command
    char cmd[16];
    snprintf(cmd, sizeof(cmd), "%02X%02X", mode, pid);
    
    std::string response = send_at_command(cmd);
    
    // Parse hex response
    std::vector<uint8_t> bytes;
    for (size_t i = 0; i < response.length() - 1; i += 2) {
        if (std::isxdigit(response[i]) && std::isxdigit(response[i + 1])) {
            char hex[3] = {response[i], response[i + 1], '\0'};
            bytes.push_back(static_cast<uint8_t>(std::strtol(hex, nullptr, 16)));
        }
    }
    
    if (bytes.empty()) {
        return CAN_ERROR_NO_DATA;
    }
    
    // Copy to output
    size_t copy_len = std::min(bytes.size(), max_len);
    std::memcpy(data_out, bytes.data(), copy_len);
    *data_len_out = copy_len;
    
    return CAN_OK;
#else
    (void)mode; (void)pid; (void)data_out; (void)data_len_out; (void)max_len;
    return CAN_ERROR_NOT_INITIALIZED;
#endif
}

} // extern "C"
