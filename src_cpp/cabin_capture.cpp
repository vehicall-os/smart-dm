/**
 * @file cabin_capture.cpp
 * @brief Cabin IR camera capture for DMS (Driver Monitoring System)
 * 
 * Captures 640x480 @ 15fps MJPEG from Pi Camera v3 or compatible USB IR camera.
 * Optimized for face detection with IR illumination support.
 */

#include "camera_capture.h"

#include <cstring>
#include <mutex>
#include <thread>
#include <queue>
#include <atomic>
#include <chrono>
#include <vector>

#ifdef __linux__
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <linux/videodev2.h>
#include <poll.h>
#endif

namespace cabin {

// Buffer structure for V4L2 mmap
struct V4L2Buffer {
    void* data;
    size_t length;
    bool queued;
};

// Cabin camera state
class CabinCapture {
public:
    CabinCapture() = default;
    ~CabinCapture() { shutdown(); }

    int init(const c_camera_config_t* config) {
#ifdef __linux__
        std::lock_guard<std::mutex> lock(mutex_);
        
        if (initialized_) {
            return CAM_OK;
        }

        // Store config
        config_ = *config;

        // Open V4L2 device
        fd_ = open(config->device, O_RDWR | O_NONBLOCK);
        if (fd_ < 0) {
            set_error("Failed to open camera device");
            return CAM_ERROR_OPEN;
        }

        // Query capabilities
        v4l2_capability cap = {};
        if (ioctl(fd_, VIDIOC_QUERYCAP, &cap) < 0) {
            close(fd_);
            fd_ = -1;
            set_error("Failed to query capabilities");
            return CAM_ERROR_OPEN;
        }

        // Set format: MJPEG 640x480 for face detection
        v4l2_format fmt = {};
        fmt.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        fmt.fmt.pix.width = config->width > 0 ? config->width : 640;
        fmt.fmt.pix.height = config->height > 0 ? config->height : 480;
        fmt.fmt.pix.pixelformat = V4L2_PIX_FMT_MJPEG;
        fmt.fmt.pix.field = V4L2_FIELD_NONE;

        if (ioctl(fd_, VIDIOC_S_FMT, &fmt) < 0) {
            close(fd_);
            fd_ = -1;
            set_error("Failed to set format");
            return CAM_ERROR_FORMAT;
        }

        // Store actual format
        actual_width_ = fmt.fmt.pix.width;
        actual_height_ = fmt.fmt.pix.height;

        // Set framerate
        v4l2_streamparm parm = {};
        parm.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        parm.parm.capture.timeperframe.numerator = 1;
        parm.parm.capture.timeperframe.denominator = config->fps > 0 ? config->fps : 15;
        ioctl(fd_, VIDIOC_S_PARM, &parm);

        // Enable IR mode if supported
        if (config->enable_ir) {
            v4l2_control ctrl = {};
            ctrl.id = V4L2_CID_AUTO_WHITE_BALANCE;
            ctrl.value = 0;  // Disable AWB for IR
            ioctl(fd_, VIDIOC_S_CTRL, &ctrl);
        }

        // Request buffers
        int buf_count = config->buffer_count > 0 ? config->buffer_count : 4;
        v4l2_requestbuffers req = {};
        req.count = buf_count;
        req.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        req.memory = V4L2_MEMORY_MMAP;

        if (ioctl(fd_, VIDIOC_REQBUFS, &req) < 0) {
            close(fd_);
            fd_ = -1;
            set_error("Failed to request buffers");
            return CAM_ERROR_BUFFER;
        }

        // Map buffers
        buffers_.resize(req.count);
        for (uint32_t i = 0; i < req.count; ++i) {
            v4l2_buffer buf = {};
            buf.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
            buf.memory = V4L2_MEMORY_MMAP;
            buf.index = i;

            if (ioctl(fd_, VIDIOC_QUERYBUF, &buf) < 0) {
                cleanup_buffers();
                close(fd_);
                fd_ = -1;
                set_error("Failed to query buffer");
                return CAM_ERROR_BUFFER;
            }

            buffers_[i].data = mmap(nullptr, buf.length,
                                    PROT_READ | PROT_WRITE, MAP_SHARED,
                                    fd_, buf.m.offset);
            buffers_[i].length = buf.length;
            buffers_[i].queued = false;

            if (buffers_[i].data == MAP_FAILED) {
                cleanup_buffers();
                close(fd_);
                fd_ = -1;
                set_error("Failed to mmap buffer");
                return CAM_ERROR_BUFFER;
            }
        }

        initialized_ = true;
        return CAM_OK;
#else
        // Non-Linux: mock mode
        config_ = *config;
        actual_width_ = config->width > 0 ? config->width : 640;
        actual_height_ = config->height > 0 ? config->height : 480;
        initialized_ = true;
        return CAM_OK;
#endif
    }

    int start() {
        std::lock_guard<std::mutex> lock(mutex_);
        
        if (!initialized_) {
            return CAM_ERROR_NOT_INITIALIZED;
        }
        if (streaming_) {
            return CAM_OK;
        }

#ifdef __linux__
        // Queue all buffers
        for (size_t i = 0; i < buffers_.size(); ++i) {
            v4l2_buffer buf = {};
            buf.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
            buf.memory = V4L2_MEMORY_MMAP;
            buf.index = i;

            if (ioctl(fd_, VIDIOC_QBUF, &buf) < 0) {
                set_error("Failed to queue buffer");
                return CAM_ERROR_STREAM;
            }
            buffers_[i].queued = true;
        }

        // Start streaming
        v4l2_buf_type type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        if (ioctl(fd_, VIDIOC_STREAMON, &type) < 0) {
            set_error("Failed to start streaming");
            return CAM_ERROR_STREAM;
        }
#endif

        streaming_ = true;
        sequence_ = 0;
        return CAM_OK;
    }

    void stop() {
        std::lock_guard<std::mutex> lock(mutex_);
        
        if (!streaming_) {
            return;
        }

#ifdef __linux__
        v4l2_buf_type type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        ioctl(fd_, VIDIOC_STREAMOFF, &type);
#endif

        streaming_ = false;
    }

    void shutdown() {
        stop();
        
        std::lock_guard<std::mutex> lock(mutex_);
        
#ifdef __linux__
        cleanup_buffers();
        if (fd_ >= 0) {
            close(fd_);
            fd_ = -1;
        }
#endif
        
        // Clear frame pool
        while (!frame_pool_.empty()) {
            delete frame_pool_.front();
            frame_pool_.pop();
        }
        
        initialized_ = false;
    }

    c_video_frame_t* read_frame(int timeout_ms) {
        if (!streaming_) {
            return nullptr;
        }

#ifdef __linux__
        // Poll for available frame
        pollfd pfd = {};
        pfd.fd = fd_;
        pfd.events = POLLIN;

        int ret = poll(&pfd, 1, timeout_ms);
        if (ret <= 0) {
            return nullptr;  // Timeout or error
        }

        // Dequeue buffer
        v4l2_buffer buf = {};
        buf.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        buf.memory = V4L2_MEMORY_MMAP;

        if (ioctl(fd_, VIDIOC_DQBUF, &buf) < 0) {
            return nullptr;
        }

        // Get or create frame
        c_video_frame_t* frame = get_frame_from_pool();
        
        // Copy data (MJPEG is variable size)
        if (frame->size < buf.bytesused) {
            delete[] frame->data;
            frame->data = new uint8_t[buf.bytesused];
        }
        std::memcpy(frame->data, buffers_[buf.index].data, buf.bytesused);
        
        frame->size = buf.bytesused;
        frame->width = actual_width_;
        frame->height = actual_height_;
        frame->stride = actual_width_;
        frame->format = PIXEL_FORMAT_MJPEG;
        frame->timestamp_ns = buf.timestamp.tv_sec * 1000000000ULL + 
                               buf.timestamp.tv_usec * 1000ULL;
        frame->sequence = sequence_++;
        frame->buffer_id = buf.index;

        // Queue buffer back
        ioctl(fd_, VIDIOC_QBUF, &buf);

        return frame;
#else
        // Mock mode: generate synthetic frame
        (void)timeout_ms;
        c_video_frame_t* frame = get_frame_from_pool();
        
        // Allocate mock MJPEG data (just header + noise)
        size_t mock_size = actual_width_ * actual_height_ / 4;  // Compressed
        if (frame->size < mock_size) {
            delete[] frame->data;
            frame->data = new uint8_t[mock_size];
        }
        
        // Fill with mock pattern
        std::memset(frame->data, 0x80, mock_size);
        
        frame->size = mock_size;
        frame->width = actual_width_;
        frame->height = actual_height_;
        frame->stride = actual_width_;
        frame->format = PIXEL_FORMAT_MJPEG;
        frame->timestamp_ns = std::chrono::steady_clock::now().time_since_epoch().count();
        frame->sequence = sequence_++;
        frame->buffer_id = 0;
        
        return frame;
#endif
    }

    void release_frame(c_video_frame_t* frame) {
        if (frame) {
            std::lock_guard<std::mutex> lock(pool_mutex_);
            frame_pool_.push(frame);
        }
    }

    bool is_streaming() const { return streaming_; }
    
    void set_error(const char* msg) {
        std::strncpy(last_error_, msg, sizeof(last_error_) - 1);
    }
    const char* get_error() const { return last_error_; }

private:
    c_video_frame_t* get_frame_from_pool() {
        std::lock_guard<std::mutex> lock(pool_mutex_);
        if (frame_pool_.empty()) {
            auto* frame = new c_video_frame_t{};
            frame->data = new uint8_t[actual_width_ * actual_height_ * 3];
            frame->size = actual_width_ * actual_height_ * 3;
            return frame;
        }
        auto* frame = frame_pool_.front();
        frame_pool_.pop();
        return frame;
    }

#ifdef __linux__
    void cleanup_buffers() {
        for (auto& buf : buffers_) {
            if (buf.data && buf.data != MAP_FAILED) {
                munmap(buf.data, buf.length);
            }
        }
        buffers_.clear();
    }
#endif

    std::mutex mutex_;
    std::mutex pool_mutex_;
    std::queue<c_video_frame_t*> frame_pool_;
    
    c_camera_config_t config_ = {};
    std::vector<V4L2Buffer> buffers_;
    
    int fd_ = -1;
    uint32_t actual_width_ = 640;
    uint32_t actual_height_ = 480;
    uint32_t sequence_ = 0;
    
    std::atomic<bool> initialized_{false};
    std::atomic<bool> streaming_{false};
    
    char last_error_[256] = {0};
};

// Global cabin camera instance
static CabinCapture g_cabin_camera;

} // namespace cabin

// C API implementation for cabin camera (camera_id = 0)
extern "C" {

static cabin::CabinCapture* get_cabin_camera() {
    return &cabin::g_cabin_camera;
}

int cabin_camera_init(const c_camera_config_t* config) {
    return get_cabin_camera()->init(config);
}

int cabin_camera_start() {
    return get_cabin_camera()->start();
}

void cabin_camera_stop() {
    get_cabin_camera()->stop();
}

void cabin_camera_shutdown() {
    get_cabin_camera()->shutdown();
}

c_video_frame_t* cabin_camera_read_frame(int timeout_ms) {
    return get_cabin_camera()->read_frame(timeout_ms);
}

void cabin_camera_release_frame(c_video_frame_t* frame) {
    get_cabin_camera()->release_frame(frame);
}

int cabin_camera_is_streaming() {
    return get_cabin_camera()->is_streaming() ? 1 : 0;
}

const char* cabin_camera_last_error() {
    return get_cabin_camera()->get_error();
}

} // extern "C"
