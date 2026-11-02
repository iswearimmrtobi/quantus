# Quantus C6 firmware

The firmware of the ESP32-C6 on the dev kit, which does Wi-Fi for the ESP32-P4: ESP-Hosted's
co-processor (Wi-Fi over SDIO, no Bluetooth yet). It uses ESP-Hosted 3.0.10, the version of the
handheld's host side (`firmware/handheld/Cargo.toml`); `PROJECT_VER` in `CMakeLists.txt` says the
same, and the handheld compares the two. `main/main.c`, `sdkconfig.defaults*` and the partition
table come from ESP-Hosted's `examples/ota/coprocessor_ota/cp` (Apache-2.0).

The C6 is not flashed directly. The handheld sends it this image (see
[wlan.md](../docs/wlan.md#the-c6s-firmware)):

```sh
# ESP-IDF 5.5.5, the one esp-idf-sys installed; once, the tools idf.py wants besides it:
python ~/.espressif/esp-idf/v5.5.5/tools/idf_tools.py install
. ~/.espressif/esp-idf/v5.5.5/export.sh

cd firmware/c6
idf.py set-target esp32c6     # once
idf.py build                  # build/quantus_c6.bin, about 1.1 MB

# Into the P4's c6_fw partition; the handheld's next boot sends it to the C6.
espflash write-bin --port /dev/ttyACM0 0x800000 build/quantus_c6.bin
```

A new ESP-Hosted version goes into both places at once: here (`main/idf_component.yml`,
`PROJECT_VER`) and in the handheld's `Cargo.toml`.
