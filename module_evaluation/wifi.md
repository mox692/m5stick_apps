
# C code

```c
void setup(void) {
    StickCP2.begin();

    WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
    while (WiFi.status() != WL_CONNECTED) {
        Serial.print('.');
        delay(500);
    }
}
```

### `StickCP2.begin()`
- https://github.com/m5stack/M5StickCPlus2/blob/master/src/M5StickCPlus2.h
- `M5Unified::begin`: https://github.com/m5stack/M5Unified/blob/002b75f43ea87062ad846315335668741cb492f0/src/M5Unified.hpp#L323

### `WiFi.begin()`
- https://github.com/espressif/arduino-esp32/blob/2b43e5bb74e571c43c8a7bcbd24c4f05989b186b/libraries/WiFi/src/WiFiSTA.h#L147
- `WiFiSTAClass::begin`: https://github.com/espressif/arduino-esp32/blob/2b43e5bb74e571c43c8a7bcbd24c4f05989b186b/libraries/WiFi/src/WiFiSTA.cpp#L85
  - call `STAClass::begin(try_connect = false)`: https://github.com/espressif/arduino-esp32/blob/2b43e5bb74e571c43c8a7bcbd24c4f05989b186b/libraries/WiFi/src/STA.cpp#L308
    - `WiFi.enableSTA(true)`: https://github.com/espressif/arduino-esp32/blob/2b43e5bb74e571c43c8a7bcbd24c4f05989b186b/libraries/WiFi/src/WiFiGeneric.cpp#L732
      - `esp_wifi_get_mode` でmodeをとる
      - wifi typeの一覧: https://github.com/espressif/arduino-esp32/blob/2b43e5bb74e571c43c8a7bcbd24c4f05989b186b/libraries/WiFi/src/WiFiType.h#L36
    - `waitStatusBits`: https://github.com/espressif/arduino-esp32/blob/2b43e5bb74e571c43c8a7bcbd24c4f05989b186b/libraries/Network/src/NetworkEvents.cpp#L342
      - `xEventGroupWaitBits` でwait
  - `STA.connect(ssid, passphrase, channel, bssid, tryConnect = true)`: https://github.com/espressif/arduino-esp32/blob/2b43e5bb74e571c43c8a7bcbd24c4f05989b186b/libraries/WiFi/src/STA.cpp#L371
    - `bool NetworkInterface::connected()`: https://github.com/espressif/arduino-esp32/blob/2b43e5bb74e571c43c8a7bcbd24c4f05989b186b/libraries/Network/src/NetworkInterface.cpp#L316-L318
    - `esp_wifi_set_config(WIFI_IF_STA, &conf);`: This is provided by https://github.com/espressif/esp32-wifi-lib, which is `esp_wifi.h`.
    - `esp_wifi_connect`: This is provided by https://github.com/espressif/esp32-wifi-lib, which is `esp_wifi.h`.
  - `STA.status();`: https://github.com/espressif/arduino-esp32/blob/2b43e5bb74e571c43c8a7bcbd24c4f05989b186b/libraries/WiFi/src/STA.cpp#L249-L251


# Rust Code
