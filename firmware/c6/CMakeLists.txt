# The ESP32-C6's firmware on the dev kit: ESP-Hosted's co-processor, Wi-Fi over SDIO to the P4
# (firmware/docs/wlan.md). The handheld sends it to the C6 itself (firmware/handheld/src/c6.rs).
cmake_minimum_required(VERSION 3.16)

# The version the handheld compares with the C6's: ESP-Hosted's.
set(PROJECT_VER "3.0.10")

include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(quantus_c6)
