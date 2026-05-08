    // initialized by physical order
    //
    // GPIO6 - A2
    let epd_busy = Input::new(
        peripherals.GPIO6,
        esp_hal::gpio::InputConfig::default().with_pull(esp_hal::gpio::Pull::None),
    );
    // GPIO5 - A3
    let sram_cs = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default());

    // two pin gap

    {
        info!("EPD: configuring SPI2");
        let spi = esp_hal::spi::master::Spi::new(
            peripherals.SPI2,
            esp_hal::spi::master::Config::default()
                .with_frequency(esp_hal::time::Rate::from_mhz(4))
                .with_mode(esp_hal::spi::Mode::_0),
        )
        .expect("SPI2 config")
        .with_sck(peripherals.GPIO21)
        .with_mosi(peripherals.GPIO22)
        .with_miso(peripherals.GPIO23);

        let spi_bus = core::cell::RefCell::new(spi);

        let epd_dc = Output::new(peripherals.GPIO17, Level::High, OutputConfig::default());
        let epd_cs = Output::new(peripherals.GPIO16, Level::High, OutputConfig::default());

        // this one is on the right hand side
        let epd_rst = Output::new(peripherals.GPIO18, Level::High, OutputConfig::default());

        let epd_spi = embedded_hal_bus::spi::RefCellDevice::new(
            &spi_bus,
            epd_cs,
            esp_hal::delay::Delay::new(),
        )
        .expect("epd device");
        let sram_spi = embedded_hal_bus::spi::RefCellDevice::new(
            &spi_bus,
            sram_cs,
            esp_hal::delay::Delay::new(),
        )
        .expect("sram device");

        let mut sram = display::drivers::sram23k256::Sram23k256::new(sram_spi);
        if let Err(e) = sram.set_sequential_mode() {
            defmt::error!("SRAM seq mode failed: {:?}", e);
        } else {
            info!("SRAM seq mode OK");
        }

        let mut epd = display::drivers::ssd1683::Ssd1683::new(
            epd_spi,
            epd_dc,
            epd_rst,
            epd_busy,
            esp_hal::delay::Delay::new(),
        );
        match epd.init() {
            Ok(()) => info!(
                "EPD init OK ({} x {})",
                display::drivers::ssd1683::WIDTH,
                display::drivers::ssd1683::HEIGHT
            ),
            Err(e) => defmt::error!("EPD init failed: {:?}", e),
        }

        let mut display = display::drivers::display420::Display420Mono::new(&mut sram);
        if let Err(e) = display.clear_to(embedded_graphics::pixelcolor::BinaryColor::Off) {
            defmt::error!("EPD clear failed: {:?}", e);
        }

        if let Err(e) = embedded_graphics::primitives::Line::new(
            embedded_graphics::geometry::Point::new(0, 0),
            embedded_graphics::geometry::Point::new(399, 299),
        )
        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_stroke(
            embedded_graphics::pixelcolor::BinaryColor::On,
            1,
        ))
        .draw(&mut display)
        {
            defmt::error!("EPD draw failed: {:?}", e);
        }

        if let Err(e) = display.flush_to_panel(&mut epd) {
            defmt::error!("EPD flush failed: {:?}", e);
        } else {
            info!("EPD flush 15000 bytes OK");
        }

        if let Err(e) = epd.refresh() {
            defmt::error!("EPD refresh failed: {:?}", e);
        } else {
            info!("EPD refresh complete");
        }

        if let Err(e) = epd.sleep() {
            defmt::error!("EPD sleep failed: {:?}", e);
        } else {
            info!("EPD diagonal drawn + sleeping");
        }
    }
    loop {
        Timer::after(Duration::from_secs(60)).await;
    }
