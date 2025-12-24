/**
 * @file frame_decoder.cpp
 * @brief CAN frame and OBD-II PID decoder
 */

#include "can_obd_driver.h"
#include <cstdint>

namespace {

/**
 * @brief Decode RPM from Mode 1 PID 0x0C response
 * Formula: ((A * 256) + B) / 4
 */
uint16_t decode_rpm(const uint8_t* data) {
    return ((static_cast<uint16_t>(data[0]) << 8) | data[1]) / 4;
}

/**
 * @brief Decode coolant temperature from Mode 1 PID 0x05 response
 * Formula: A - 40 (degrees Celsius)
 */
int8_t decode_coolant_temp(uint8_t data) {
    return static_cast<int8_t>(data) - 40;
}

/**
 * @brief Decode vehicle speed from Mode 1 PID 0x0D response
 * Formula: A (km/h)
 */
uint8_t decode_speed(uint8_t data) {
    return data;
}

/**
 * @brief Decode engine load from Mode 1 PID 0x04 response
 * Formula: A * 100 / 255 (%)
 */
uint8_t decode_engine_load(uint8_t data) {
    return (static_cast<uint16_t>(data) * 100) / 255;
}

/**
 * @brief Decode MAF from Mode 1 PID 0x10 response
 * Formula: ((A * 256) + B) / 100 (g/s)
 * We return g/s * 100 to preserve precision
 */
uint16_t decode_maf(const uint8_t* data) {
    return (static_cast<uint16_t>(data[0]) << 8) | data[1];
}

/**
 * @brief Decode throttle position from Mode 1 PID 0x11 response
 * Formula: A * 100 / 255 (%)
 */
uint8_t decode_throttle_position(uint8_t data) {
    return (static_cast<uint16_t>(data) * 100) / 255;
}

/**
 * @brief Decode fuel trim from Mode 1 PID 0x06/0x07/0x08/0x09 response
 * Formula: (A - 128) * 100 / 128 (%)
 */
int8_t decode_fuel_trim(uint8_t data) {
    return static_cast<int8_t>((static_cast<int16_t>(data) - 128) * 100 / 128);
}

} // anonymous namespace

extern "C" {

int frame_decode_obd_response(const uint8_t* raw_data, size_t raw_len,
                               uint8_t expected_mode, uint8_t expected_pid,
                               uint8_t* value_out, size_t* value_len_out,
                               size_t max_len) {
    if (!raw_data || raw_len < 2 || !value_out || !value_len_out) {
        return CAN_ERROR_INIT;
    }
    
    // Check response mode (mode + 0x40)
    if (raw_data[0] != (expected_mode + 0x40)) {
        return CAN_ERROR_INVALID_RESPONSE;
    }
    
    // Check PID
    if (raw_data[1] != expected_pid) {
        return CAN_ERROR_INVALID_RESPONSE;
    }
    
    // Copy data (skip mode and PID bytes)
    size_t data_len = raw_len - 2;
    if (data_len > max_len) {
        data_len = max_len;
    }
    
    for (size_t i = 0; i < data_len; ++i) {
        value_out[i] = raw_data[i + 2];
    }
    *value_len_out = data_len;
    
    return CAN_OK;
}

int frame_decode_sensor_data(const uint8_t* raw_data, size_t raw_len,
                              c_sensor_frame_t* sensor_out) {
    if (!raw_data || raw_len < 3 || !sensor_out) {
        return CAN_ERROR_INIT;
    }
    
    // Check it's a Mode 1 response
    if (raw_data[0] != 0x41) {
        return CAN_ERROR_INVALID_RESPONSE;
    }
    
    uint8_t pid = raw_data[1];
    const uint8_t* data = &raw_data[2];
    
    switch (pid) {
        case 0x04:  // Engine load
            sensor_out->engine_load = decode_engine_load(data[0]);
            sensor_out->valid_mask |= 0x08;
            break;
            
        case 0x05:  // Coolant temperature
            sensor_out->coolant_temp = static_cast<uint8_t>(decode_coolant_temp(data[0]) + 40);
            sensor_out->valid_mask |= 0x02;
            break;
            
        case 0x06:  // Short term fuel trim (bank 1)
            sensor_out->fuel_trim_short = decode_fuel_trim(data[0]);
            sensor_out->valid_mask |= 0x40;
            break;
            
        case 0x07:  // Long term fuel trim (bank 1)
            sensor_out->fuel_trim_long = decode_fuel_trim(data[0]);
            sensor_out->valid_mask |= 0x80;
            break;
            
        case 0x0C:  // RPM
            if (raw_len >= 4) {
                sensor_out->rpm = decode_rpm(data);
                sensor_out->valid_mask |= 0x01;
            }
            break;
            
        case 0x0D:  // Vehicle speed
            sensor_out->speed = decode_speed(data[0]);
            sensor_out->valid_mask |= 0x04;
            break;
            
        case 0x10:  // MAF
            if (raw_len >= 4) {
                sensor_out->maf = decode_maf(data);
                sensor_out->valid_mask |= 0x10;
            }
            break;
            
        case 0x11:  // Throttle position
            sensor_out->throttle_pos = decode_throttle_position(data[0]);
            sensor_out->valid_mask |= 0x20;
            break;
            
        default:
            // Unknown PID, ignore
            break;
    }
    
    return CAN_OK;
}

} // extern "C"
