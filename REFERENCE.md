# References used for the SSD1683 / E-Paper Friend Rust driver

Sources consulted while writing `src/epd/ssd1683.rs`, `src/epd/sram23k256.rs`, and `src/epd/display420.rs`. Grouped by what each one was used for.

## Primary translation source — Adafruit_EPD (C++)

The Rust SSD1683 driver is a direct port of the relevant pieces of the Adafruit_EPD Arduino library.

- Repository — <https://github.com/adafruit/Adafruit_EPD>
- `src/drivers/Adafruit_SSD1683.h` — every `SSD1683_*` command hex value (`SW_RESET 0x12`, `DEEP_SLEEP 0x10`, `WRITE_RAM1 0x24`, `MASTER_ACTIVATE 0x20`, `DISP_CTRL2 0x22`, `SET_RAMXPOS 0x44`, `SET_RAMYPOS 0x45`, etc.) — copied verbatim into `cmd::*` constants.
- `src/drivers/Adafruit_SSD1683.cpp` — `ssd1683_default_init_code[]` (the SW_RESET → DISP_CTRL1 0x40 0x00 → WRITE_BORDER 0x05 → DATA_MODE 0x03 → TEMP_CONTROL 0x80 sequence), the `update()` function (DISP_CTRL2 0xF7 + MASTER_ACTIVATE), `powerDown()` (DEEP_SLEEP 0x01), and the `setRAMWindow` / `setRAMAddress` helpers — translated into `Ssd1683::init()`, `refresh()`, `sleep()`, `set_ram_window()`, `set_ram_address()`.
- `src/Adafruit_EPD.cpp` / `src/Adafruit_EPD.h` — framebuffer flow, BUSY polling pattern, the SRAM-streaming variant (`writeSRAMFramebufferToEPD`) that motivated `Display420Mono::flush_to_panel`.
- `src/Adafruit_MCPSRAM.h` / `Adafruit_MCPSRAM.cpp` — 23K256 command opcodes and sequential-mode usage.
- `src/Adafruit_ThinkInk.h` and `src/panels/ThinkInk_420_Grayscale4_MFGN.h` — confirms the 4.2" 300×400 mono panel binds to `Adafruit_SSD1683` and uses `0xF7` as the mono update value.
- `examples/ThinkInk_mono/ThinkInk_mono.ino` — sanity check of the Arduino-side init/refresh/sleep ordering.

## CircuitPython driver — cross-reference

- <https://github.com/adafruit/Adafruit_CircuitPython_SSD1683> — second-source confirmation of the SSD1683 init bytes and BUSY polarity.

## Adafruit hardware pages

- Product **6381** — 4.2" 300×400 monochrome panel (the panel this driver targets) — <https://www.adafruit.com/product/6381>
- Product **6382** — 4.2" 300×400 tri-color (same SSD1683 controller) — <https://www.adafruit.com/product/6382>
- Product **4224** — E-Paper Friend breakout (SSD1683 + 23K256 + 3.3 V LDO + 24-pin FPC) — <https://www.adafruit.com/product/4224>
- E-Paper Friend PCB schematic (confirms the 23K256 part and SRCS pin) — <https://github.com/adafruit/Adafruit-E-Paper-Display-Breakout-PCBs>
- Adafruit Learn — *Bare E-Ink Displays Crash Course: 4.2" 300×400 mono SSD1683* — <https://learn.adafruit.com/bare-e-ink-displays-crash-course/4-2-300x400-monochrome-or-4-gray-eink-ssd1683>
- Adafruit Learn — *Adafruit eInk Display Breakouts pinouts* — <https://learn.adafruit.com/adafruit-eink-display-breakouts>
- Panel datasheet (SSD1683 + 400×300 panel ZJY400300-042CABMFGN-R) — <https://cdn-shop.adafruit.com/product-files/6382/6382+C22266-001+datasheet+ZJY400300-042CABMFGN-R.pdf> *(the native panel scan order on this datasheet is what flagged the WIDTH/HEIGHT swap — 400 source × 300 gate)*

## Microchip 23K256 SRAM

- Datasheet (READ 0x03, WRITE 0x02, WRMR 0x01, RDMR 0x05; sequential mode = 0x40; SPI mode 0; up to 20 MHz at 3.3 V) — <https://ww1.microchip.com/downloads/en/DeviceDoc/23A256-23K256-256-Kbit-SPI-Bus-Low-Power-Serial-SRAM-20002100G.pdf>
- Linux kernel `mchp23k256` MTD driver — opcode confirmation — <https://github.com/torvalds/linux/blob/master/drivers/mtd/devices/mchp23k256.c>

## ESP32-C6 Feather

- Feather pinout (SCK = GPIO21, MOSI = GPIO22, MISO = GPIO23; strap pins to avoid; broken-out vs reserved pins) — <https://learn.adafruit.com/adafruit-esp32-c6-feather/pinouts>
- Feather PrettyPins PDF (in `docs/Adafruit Feather ESP32-C6 PrettyPins 2.pdf`) — <https://github.com/adafruit/Adafruit-ESP32-C6-Feather-PCB/blob/main/Adafruit%20Feather%20ESP32-C6%20PrettyPins%202.pdf>
- ESP32-C6 datasheet — <https://documentation.espressif.com/esp32-c6-mini-1_mini-1u_datasheet_en.pdf>

## Other Rust e-paper crates (consulted for shape, not copied)

None of these support the SSD1683 at 300×400, but they were useful for confirming the typical `DrawTarget` shape and bus-sharing patterns in the embedded-hal-1.0 ecosystem.

- `ssd1683` (1.54" only) — <https://github.com/Boudewijn26/ssd1683>
- `ssd1681` (1.54", same vendor family, near-identical command set) — <https://crates.io/crates/ssd1681> · <https://docs.rs/ssd1681/>
- `ssd1680` (2.13" WeAct, simpler cousin) — <https://crates.io/crates/ssd1680> · <https://docs.rs/ssd1680/>
- `epd-waveshare` (multi-controller umbrella; no SSD1683) — <https://crates.io/crates/epd-waveshare> · <https://docs.rs/epd-waveshare/>
- `uc8151` (Pimoroni Badger2040) — <https://crates.io/crates/uc8151>
