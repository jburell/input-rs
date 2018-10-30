extern crate evdev;
use evdev::Device;

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct TouchEvent {
    pub pos_x: f32,
    pub pos_y: f32,
}

pub struct InputHandler{
    device: Arc<Mutex<Device>>,
    touch_handler: Arc<Mutex<Box<FnMut(TouchEvent) + Send>>>,
    is_polling_events: std::sync::atomic::AtomicBool,
    polling_thread: Option<std::thread::JoinHandle<()>>,
    screen_rotation: ScreenRotation,
}

const X_RES: f32 = 8640.0;
const Y_RES: f32 = 15360.0;
const ABSOLUTE: u16 = 3;
const ABS_X:u16 = 53;
const ABS_Y:u16 = 54;

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum ScreenRotation {
    Deg_0,
    Deg_CW90,
    Deg_CW180,
    Deg_CCW90,
}

impl FromStr for ScreenRotation {
    type Err = String;
    fn from_str(val: &str) -> Result<Self, Self::Err> {
        match val {
            "0" => Ok(ScreenRotation::Deg_0),
            "CW90" => Ok(ScreenRotation::Deg_CW90),
            "CW180" => Ok(ScreenRotation::Deg_CW180),
            "CCW90" => Ok(ScreenRotation::Deg_CCW90),
            _ => Err(format!("Could not convert {} into screen rotation. Allowed values: 0, CW90, CW180, CCW90", val)),
        }
    }
}

fn formula_x(screen_rotation: ScreenRotation) -> Result<fn(i32) -> f32, String> {
    match screen_rotation {
        ScreenRotation::Deg_0 => Ok(|x| {(x as f32) / X_RES}),
        ScreenRotation::Deg_CW90 => Ok(|x| {(x as f32) / Y_RES}),
        _ => Err(format!("Unsupported screen rotation {:?}", screen_rotation)),
    }
} 

fn formula_y(screen_rotation: ScreenRotation) -> Result<fn(i32) -> f32, String> {
    match screen_rotation {
        ScreenRotation::Deg_0 => Ok(|y| {(y as f32) / Y_RES}),
        ScreenRotation::Deg_CW90 => Ok(|y| (X_RES - y as f32) / X_RES),
        _ => Err(format!("Unsupported screen rotation {:?}", screen_rotation)),
    }
} 

impl InputHandler {
    pub fn new(vendor: u16, product: u16, event_handler: Box<FnMut(TouchEvent) + Send>, screen_rotation: ScreenRotation) -> Result<Self, String> {
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
            screen_rotation: screen_rotation,
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

        let screen_func_x = formula_x(self.screen_rotation).unwrap();
        let screen_func_y = formula_y(self.screen_rotation).unwrap();
        
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
                                ABSOLUTE => {
                                    match ScreenRotation::Deg_CW90/*self.screen_rotation*/ {
                                        // TODO: Clean up this logic (quick hack)
                                        ScreenRotation::Deg_CW90 => {
                                            match ev.code {
                                                // writing y to x because screen orientation
                                                ABS_X => cached_event.pos_y = screen_func_x(ev.value),
                                                // negative value because screen orientation, 8640 == max res on y
                                                ABS_Y => cached_event.pos_x = screen_func_y(ev.value),
                                                _ => (),
                                            }
                                        }
                                        ScreenRotation::Deg_0 => {
                                            match ev.code {
                                                // writing y to x because screen orientation
                                                ABS_X => cached_event.pos_x = screen_func_x(ev.value),
                                                // negative value because screen orientation, 8640 == max res on y
                                                ABS_Y => cached_event.pos_y = screen_func_y(ev.value),
                                                _ => (),
                                            }
                                        }
                                        _ => (),//println!("{}", format!("Unsupported screen rotation {:?}", &self.screen_rotation)),
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