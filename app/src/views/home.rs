use crate::ble;
use btleplug::api::Peripheral as _;
use btleplug::platform::Peripheral;
use dioxus::prelude::*;
use shared::{BadgeCommand, Color, DrawCommand, LightCommand, Point};
use std::time::Duration;

#[derive(Clone, PartialEq)]
enum Status {
    Idle,
    Scanning,
    Connecting,
    Connected,
    Error(String),
}

#[component]
pub fn Home() -> Element {
    let mut status = use_signal(|| Status::Idle);
    let mut devices = use_signal(Vec::<ble::Discovered>::new);
    let mut connected = use_signal(|| Option::<Peripheral>::None);
    let r = use_signal(|| 0u8);
    let g = use_signal(|| 4u8);
    let b = use_signal(|| 0u8);

    let draw_color = use_signal(|| Color::Black);
    let line_start = use_signal(|| Point { x: 20, y: 20 });
    let line_end = use_signal(|| Point { x: 380, y: 280 });

    let scan = move |_| {
        spawn(async move {
            status.set(Status::Scanning);
            devices.write().clear();
            let adapter = match ble::first_adapter().await {
                Ok(a) => a,
                Err(e) => {
                    status.set(Status::Error(e));
                    return;
                }
            };
            match ble::scan_all(&adapter, Duration::from_secs(2)).await {
                Ok(found) => {
                    // Auto-connect when exactly one badge is in range. Multiple
                    // matches → show the list and let the user pick.
                    if found.len() == 1 {
                        let peripheral = found[0].peripheral.clone();
                        devices.set(found);
                        status.set(Status::Connecting);
                        match ble::connect(&peripheral).await {
                            Ok(()) => {
                                connected.set(Some(peripheral));
                                status.set(Status::Connected);
                            }
                            Err(e) => status.set(Status::Error(e)),
                        }
                    } else {
                        devices.set(found);
                        status.set(Status::Idle);
                    }
                }
                Err(e) => status.set(Status::Error(e)),
            }
        });
    };

    let connect = move |peripheral: Peripheral| {
        spawn(async move {
            status.set(Status::Connecting);
            match ble::connect(&peripheral).await {
                Ok(()) => {
                    connected.set(Some(peripheral));
                    status.set(Status::Connected);
                }
                Err(e) => status.set(Status::Error(e)),
            }
        });
    };

    let disconnect = move |_| {
        spawn(async move {
            if let Some(p) = connected().clone() {
                connected.set(None);
                let _ = ble::disconnect(&p).await;
            } else {
                connected.set(None);
            }
            status.set(Status::Idle);
        });
    };

    let send = move |cmd: BadgeCommand| {
        spawn(async move {
            let Some(p) = connected().clone() else {
                return;
            };
            if let Err(e) = ble::write_command(&p, &cmd).await {
                status.set(Status::Error(e));
            }
        });
    };

    let send_rgb = move |_| {
        send(BadgeCommand::SetLight(LightCommand {
            r: r(),
            g: g(),
            b: b(),
        }));
    };

    let send_debug = move |_| send(BadgeCommand::Debug);

    let send_clear = move |_| {
        send(BadgeCommand::Drawing(DrawCommand::Clear {
            color: draw_color(),
        }));
    };

    let send_line = move |_| {
        send(BadgeCommand::Drawing(DrawCommand::Line {
            start: line_start(),
            end: line_end(),
            color: draw_color(),
        }));
    };

    let send_flush = move |_| send(BadgeCommand::Drawing(DrawCommand::Flush));

    rsx! {
        div { class: "p-6 max-w-md mx-auto space-y-4",
            h1 { class: "text-2xl font-bold", "Badge Connection" }

            div { class: "text-sm text-gray-600",
                {match status() {
                    Status::Idle => rsx! { "Ready" },
                    Status::Scanning => rsx! { "Scanning..." },
                    Status::Connecting => rsx! { "Connecting..." },
                    Status::Connected => rsx! { "Connected" },
                    Status::Error(msg) => rsx! { span { class: "text-red-600", "Error: {msg}" } },
                }}
            }

            if connected().is_none() {
                button {
                    class: "px-4 py-2 bg-blue-600 text-white rounded disabled:opacity-50",
                    disabled: matches!(status(), Status::Scanning | Status::Connecting),
                    onclick: scan,
                    "Scan for badge"
                }

                if !devices().is_empty() {
                    h2 { class: "text-lg font-semibold pt-2", "Discovered" }
                    ul { class: "space-y-2",
                        for d in devices() {
                            li {
                                key: "{d.peripheral.id()}",
                                class: "flex justify-between items-center border rounded px-3 py-2",
                                div {
                                    div { class: "font-medium", "{d.name}" }
                                    if let Some(rssi) = d.rssi {
                                        div { class: "text-xs text-gray-500", "RSSI: {rssi} dBm" }
                                    }
                                }
                                button {
                                    class: "px-3 py-1 bg-green-600 text-white rounded",
                                    onclick: {
                                        let p = d.peripheral.clone();
                                        move |_| connect(p.clone())
                                    },
                                    "Connect"
                                }
                            }
                        }
                    }
                } else if matches!(status(), Status::Idle) {
                    p { class: "text-sm text-gray-500", "No devices yet. Tap scan." }
                }
            } else {
                div { class: "border rounded p-4 space-y-3",
                    h2 { class: "text-lg font-semibold", "RGB" }

                    Slider { label: "R", value: r }
                    Slider { label: "G", value: g }
                    Slider { label: "B", value: b }

                    div {
                        class: "h-10 rounded border",
                        style: "background-color: rgb({r}, {g}, {b});",
                    }

                    div { class: "flex gap-2",
                        button {
                            class: "px-4 py-2 bg-blue-600 text-white rounded flex-1",
                            onclick: send_rgb,
                            "Send"
                        }
                        button {
                            class: "px-4 py-2 bg-gray-300 rounded",
                            onclick: disconnect,
                            "Disconnect"
                        }
                    }
                }

                div { class: "border rounded p-4 space-y-3",
                    h2 { class: "text-lg font-semibold", "Display" }

                    button {
                        class: "px-4 py-2 bg-purple-600 text-white rounded",
                        onclick: send_debug,
                        "Send Debug"
                    }

                    ColorChooser { value: draw_color }

                    div { class: "flex gap-2",
                        button {
                            class: "px-4 py-2 bg-blue-600 text-white rounded flex-1",
                            onclick: send_clear,
                            "Clear"
                        }
                        button {
                            class: "px-4 py-2 bg-amber-600 text-white rounded flex-1",
                            onclick: send_flush,
                            "Flush"
                        }
                    }

                    div { class: "space-y-2",
                        h3 { class: "font-medium", "Line" }
                        PointEditor { label: "start", value: line_start }
                        PointEditor { label: "end", value: line_end }
                        button {
                            class: "px-4 py-2 bg-green-600 text-white rounded w-full",
                            onclick: send_line,
                            "Draw line"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Slider(label: String, value: Signal<u8>) -> Element {
    rsx! {
        label { class: "block",
            div { class: "flex justify-between text-sm",
                span { "{label}" }
                span { "{value}" }
            }
            input {
                r#type: "range",
                min: "0",
                max: "255",
                value: "{value}",
                class: "w-full",
                oninput: move |e| {
                    if let Ok(v) = e.value().parse::<u8>() {
                        value.set(v);
                    }
                },
            }
        }
    }
}

#[component]
fn ColorChooser(value: Signal<Color>) -> Element {
    let options = [
        ("White", Color::White),
        ("Black", Color::Black),
        ("Red", Color::Red),
    ];
    rsx! {
        div { class: "flex gap-2",
            for (label, color) in options {
                button {
                    key: "{label}",
                    class: if value() == color {
                        "px-3 py-1 rounded border-2 border-blue-600 bg-blue-100 flex-1"
                    } else {
                        "px-3 py-1 rounded border flex-1"
                    },
                    onclick: move |_| value.set(color),
                    "{label}"
                }
            }
        }
    }
}

#[component]
fn PointEditor(label: String, value: Signal<Point>) -> Element {
    rsx! {
        div { class: "flex gap-2 items-center",
            span { class: "text-sm w-12", "{label}" }
            input {
                r#type: "number",
                class: "border rounded px-2 py-1 w-20 text-sm",
                value: "{value().x}",
                oninput: move |e| {
                    if let Ok(x) = e.value().parse::<i16>() {
                        let mut p = value();
                        p.x = x;
                        value.set(p);
                    }
                },
            }
            input {
                r#type: "number",
                class: "border rounded px-2 py-1 w-20 text-sm",
                value: "{value().y}",
                oninput: move |e| {
                    if let Ok(y) = e.value().parse::<i16>() {
                        let mut p = value();
                        p.y = y;
                        value.set(p);
                    }
                },
            }
        }
    }
}
