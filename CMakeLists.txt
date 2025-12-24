cmake_minimum_required(VERSION 3.20)
project(vehicle_drivers LANGUAGES CXX C)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_C_STANDARD 11)

# Dependencies
find_package(Threads REQUIRED)

# Platform-specific settings
if(UNIX AND NOT APPLE)
    set(HAS_LINUX TRUE)
    add_definitions(-D__linux__=1)
else()
    set(HAS_LINUX FALSE)
endif()

# OBD/CAN Driver Sources
set(OBD_SOURCES
    src_cpp/can_driver.cpp
    src_cpp/obd_client.cpp
    src_cpp/frame_decoder.cpp
    src_cpp/timing_service.cpp
    src_cpp/c_api.cpp
)

# Camera Capture Sources
set(CAMERA_SOURCES
    src_cpp/cabin_capture.cpp
    src_cpp/road_capture.cpp
)

# IMU Driver Sources
set(IMU_SOURCES
    src_cpp/imu_driver.cpp
)

# All sources
set(ALL_SOURCES
    ${OBD_SOURCES}
    ${CAMERA_SOURCES}
    ${IMU_SOURCES}
)

# Include directories
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include)

# Static library for Rust linking
add_library(vehicle_drivers STATIC ${ALL_SOURCES})

target_link_libraries(vehicle_drivers 
    PRIVATE 
    Threads::Threads
)

# Position Independent Code for Rust FFI linking
set_property(TARGET vehicle_drivers PROPERTY POSITION_INDEPENDENT_CODE ON)

# Compiler warnings
if(CMAKE_CXX_COMPILER_ID MATCHES "Clang|GNU")
    target_compile_options(vehicle_drivers PRIVATE 
        -Wall -Wextra -Wpedantic
        -fstack-protector-strong
    )
endif()

# Debug/Release flags
if(CMAKE_BUILD_TYPE STREQUAL "Release")
    target_compile_definitions(vehicle_drivers PRIVATE NDEBUG)
    if(CMAKE_CXX_COMPILER_ID MATCHES "Clang|GNU")
        target_compile_options(vehicle_drivers PRIVATE -O3)
    endif()
endif()

# Install targets
install(TARGETS vehicle_drivers
    ARCHIVE DESTINATION lib
    LIBRARY DESTINATION lib
)

install(FILES 
    include/can_obd_driver.h
    include/camera_capture.h
    DESTINATION include
)

# Print configuration
message(STATUS "Linux support: ${HAS_LINUX}")
message(STATUS "Build type: ${CMAKE_BUILD_TYPE}")
