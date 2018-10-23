extern crate evdev;
use evdev::Device;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct TouchEvent {
    pos_x: f32,
    pos_y: f32,
}

pub struct InputHandler{
    device: Arc<Mutex<Device>>,
    touch_handler: Arc<Mutex<Box<FnMut(TouchEvent) + Send>>>,
    is_polling_events: std::sync::atomic::AtomicBool,
    polling_thread: Option<std::thread::JoinHandle<()>>,
}

const X_RES: f32 = 8640.0;
const Y_RES: f32 = 15360.0;

impl InputHandler {
    pub fn new(vendor: u16, product: u16, event_handler: Box<FnMut(TouchEvent) + Send>) -> Result<Self, String> {
        let mut devices_matching = evdev::enumerate()
            .into_iter()
            .filter(|d| 
                d.input_id().vendor == vendor 
                && d.input_id().product == product)
            .collect::<Vec<Device>>();

        let device = if devices_matching.len() > 0 {
            Ok(devices_matching.remove(0))
        } else {
            Err(format!("Could not open device with: vendor:{}, product: {}", vendor, product))
        }?;

        Ok(InputHandler {
            device: Arc::new(Mutex::new(device)),
            touch_handler: Arc::new(Mutex::new(event_handler)),
            is_polling_events: std::sync::atomic::AtomicBool::new(false),
            polling_thread: None,
        })
    }

    pub fn is_polling_events(&self) -> bool {
        self.is_polling_events.load(Ordering::SeqCst)
    }

    fn start_polling_thread(&mut self) {
        let dev = self.device.clone();
        let handler = self.touch_handler.clone();

        let mut cached_event = TouchEvent {
            pos_x: 0f32,
            pos_y: 0f32,
        };

        self.polling_thread = Some(std::thread::spawn(move || {
            loop {
                let mut d = dev.lock().expect("Could not lock the device for polling");

                if let Ok(raw_events) = d.events_no_sync() {
                    raw_events.into_iter()
                        .fold(TouchEvent{
                            pos_x: 0f32,
                            pos_y: 0f32,
                        }, |mut acc, ev| {
                            match ev._type {
                                3 => {
                                    match ev.code {
                                        // 53 == x, writing y to x because screen orientation
                                        53 => cached_event.pos_y = (ev.value as f32) / Y_RES,
                                        // 54 == y, negative value because screen orientation, 8640 == max res on y
                                        54 => cached_event.pos_x = (X_RES - ev.value as f32) / X_RES,
                                        _ => (),
                                    }
                                }
                                _ => ()
                            }
                            
                            acc = cached_event.clone();

                            let mut hndl = handler.lock().expect("Could not lock the handler for polling");
                            let fun = hndl.as_mut();
                            fun(acc.clone());
                            acc
                        });
                };
            }
        }));
    }

    pub fn start_polling_events(&mut self) {
        let is_polling = self.is_polling_events.load(Ordering::Acquire);
        if !is_polling {
            self.start_polling_thread();
            self.is_polling_events.store(is_polling, Ordering::Release);
        } else {
            self.is_polling_events.store(is_polling, Ordering::Release);
        }
    }
}