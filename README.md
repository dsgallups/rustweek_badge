# Overview

- `/badge`: embedded device code
- `/mobile`: dioxus embedded device code

# Badge Notes 

Target: `riscv32imac-unknown-none-elf`


## Resources
### ESP32-C6 Feather
- Adafruit Page: [https://www.adafruit.com/product/5933#technical-details]
- Pinout: [https://github.com/adafruit/Adafruit-ESP32-C6-Feather-PCB/blob/main/Adafruit%20Feather%20ESP32-C6%20PrettyPins%202.pdf]
- ESP32-C6 Datasheet: [https://documentation.espressif.com/esp32-c6-mini-1_mini-1u_datasheet_en.pdf]

### Breakout board
- Adafruit page: [https://www.adafruit.com/product/4224]

### Display (2color/tri-color)
- Adafruit page: [https://www.adafruit.com/product/6381]
- [https://cdn-shop.adafruit.com/product-files/6381/P6381+C22265-001+datasheet+ZJY400300-042CAAMFGN.pdf]
- Tricolor DigiKey page: [https://www.digikey.com/en/products/detail/adafruit-industries-llc/6382/27525826]
- Tricolor Datasheet: [https://cdn-shop.adafruit.com/product-files/6382/6382+C22266-001+datasheet+ZJY400300-042CABMFGN-R.pdf]
- 


### Peripherals
- General Purpose Speaker: [https://www.digikey.com/en/products/detail/adafruit-industries-llc/4227/10245140]
- General Purpose Speaker Datasheet: [https://cdn-shop.adafruit.com/product-files/4227/C13238-001+spec+RB-2030008G-046LR-E+for+C13238-001++(1).pdf]
- Speaker Amplication Board: [https://www.digikey.com/en/products/detail/adafruit-industries-llc/987/5629428]
- Speaker Amplication Board Datasheet: [https://www.digikey.com/en/products/detail/adafruit-industries-llc/987/5629428]


## Wiring

| Signal | GPIO |
| ------ | ---- |
| SCK | 21 |
| MOSI | 22 |
| MISO | 23 |
| EPD CS (ECS) | 16 |
| EPD DC (EDC) | 17 | 
| EPD RST | 18 |
| EPD BUSY | 6 | 
| SRAM CS (SRCS) | 5 |


# Mobile notes
Reference for platform support: <https://dioxuslabs.com/learn/0.7/guides/platforms/mobile>
