/**
 * @file timing_service.cpp
 * @brief High-precision timing service using CLOCK_MONOTONIC
 */

#include <chrono>
#include <cstdint>

#ifdef _WIN32
#include <windows.h>
#else
#include <time.h>
#endif

extern "C" {

/**
 * @brief Get current monotonic timestamp in nanoseconds
 * @return Nanoseconds since an arbitrary epoch
 */
uint64_t timing_get_timestamp_ns() {
#ifdef _WIN32
    // Windows implementation using QueryPerformanceCounter
    static LARGE_INTEGER frequency = {0};
    if (frequency.QuadPart == 0) {
        QueryPerformanceFrequency(&frequency);
    }
    
    LARGE_INTEGER counter;
    QueryPerformanceCounter(&counter);
    
    // Convert to nanoseconds
    return static_cast<uint64_t>(
        (counter.QuadPart * 1000000000LL) / frequency.QuadPart
    );
#else
    // Linux/POSIX implementation using CLOCK_MONOTONIC
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return static_cast<uint64_t>(ts.tv_sec) * 1000000000ULL + 
           static_cast<uint64_t>(ts.tv_nsec);
#endif
}

/**
 * @brief Get current monotonic timestamp in microseconds
 * @return Microseconds since an arbitrary epoch
 */
uint64_t timing_get_timestamp_us() {
    return timing_get_timestamp_ns() / 1000;
}

/**
 * @brief Get current monotonic timestamp in milliseconds
 * @return Milliseconds since an arbitrary epoch
 */
uint64_t timing_get_timestamp_ms() {
    return timing_get_timestamp_ns() / 1000000;
}

/**
 * @brief Calculate elapsed time in nanoseconds
 * @param start_ns Start timestamp from timing_get_timestamp_ns()
 * @return Elapsed nanoseconds
 */
uint64_t timing_elapsed_ns(uint64_t start_ns) {
    return timing_get_timestamp_ns() - start_ns;
}

/**
 * @brief Calculate elapsed time in milliseconds
 * @param start_ns Start timestamp from timing_get_timestamp_ns()
 * @return Elapsed milliseconds
 */
uint64_t timing_elapsed_ms(uint64_t start_ns) {
    return timing_elapsed_ns(start_ns) / 1000000;
}

} // extern "C"
