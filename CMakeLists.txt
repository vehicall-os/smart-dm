cmake_minimum_required(VERSION 3.20)
project(can_obd_driver LANGUAGES CXX C)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_C_STANDARD 11)

# Dependencies
find_package(Threads REQUIRED)

# Platform-specific settings
if(UNIX AND NOT APPLE)
    # Linux - SocketCAN support
    set(HAS_SOCKETCAN TRUE)
    add_definitions(-DHAS_SOCKETCAN=1)
else()
    # Windows/macOS - Mock drivers only
    set(HAS_SOCKETCAN FALSE)
    add_definitions(-DHAS_SOCKETCAN=0)
endif()

# Source files
set(SOURCES
    src_cpp/can_driver.cpp
    src_cpp/obd_client.cpp
    src_cpp/frame_decoder.cpp
    src_cpp/timing_service.cpp
    src_cpp/c_api.cpp
)

# Include directories
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include)

# Static library for Rust linking
add_library(can_obd_driver STATIC ${SOURCES})

target_link_libraries(can_obd_driver 
    PRIVATE 
    Threads::Threads
)

# Position Independent Code for Rust FFI linking
set_property(TARGET can_obd_driver PROPERTY POSITION_INDEPENDENT_CODE ON)

# Compiler warnings
if(CMAKE_CXX_COMPILER_ID MATCHES "Clang|GNU")
    target_compile_options(can_obd_driver PRIVATE 
        -Wall -Wextra -Wpedantic
        -fstack-protector-strong
    )
endif()

# Debug/Release flags
if(CMAKE_BUILD_TYPE STREQUAL "Release")
    target_compile_definitions(can_obd_driver PRIVATE NDEBUG)
    if(CMAKE_CXX_COMPILER_ID MATCHES "Clang|GNU")
        target_compile_options(can_obd_driver PRIVATE -O3)
    endif()
endif()

# Install targets
install(TARGETS can_obd_driver
    ARCHIVE DESTINATION lib
    LIBRARY DESTINATION lib
)

install(FILES include/can_obd_driver.h
    DESTINATION include
)

# Print configuration
message(STATUS "SocketCAN support: ${HAS_SOCKETCAN}")
message(STATUS "Build type: ${CMAKE_BUILD_TYPE}")
