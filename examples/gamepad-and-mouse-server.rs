use std::sync::{Arc, mpsc};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, Duration};
use std::thread;

use gilrs::{Gilrs, Button, Axis};

use pad_motion::protocol::*;
use pad_motion::server::*;

use std::fs::OpenOptions;
use std::io::{Read};
//use std::os::unix::io::{AsRawFd, RawFd};

struct Mizouse {
    left: u8,
    right: u8,
    middle: u8,
    x: i8,
    y: i8,
    wheel: i8,
}

fn mouse(tx: Sender<Mizouse>) {
    let device_path = "/dev/input/mice";
    let mut file = OpenOptions::new().read(true).write(true).open(device_path).unwrap();

    let mut data = [0; 4];
    let mut left = 0;
    let mut middle = 0;
    let mut right = 0;
    let mut x = 0;
    let mut y = 0;

    loop {
        let bytes = file.read(&mut data).unwrap();
        if bytes > 0 {
            let val = Mizouse {
                left: data[0] & 0x1,
                right: data[0] & 0x2,
                middle: data[0] & 0x4,

                x: data[1] as i8,
                y: data[2] as i8,
                wheel: data[3] as i8,
            };

            //send the message for the mouse movement
            tx.send(val).unwrap();
        }
    }
}


fn main() {
  let running = Arc::new(AtomicBool::new(true));
  let (m_tx, m_rx) = mpsc::channel();

  {
    let running = running.clone();
    ctrlc::set_handler(move || {
      running.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");
  }

  let server = Arc::new(Server::new(None, None).unwrap());
  let server_thread_join_handle = {
    let server = server.clone();
    server.start(running.clone())
  };

  let mouse_thread_join_handle = {
      thread::spawn(move|| {
          mouse(m_tx);
      })
  };

  let controller_info = ControllerInfo {
    slot_state: SlotState::Connected,
    device_type: DeviceType::FullGyro,
    connection_type: ConnectionType::USB,
    .. Default::default()
  };
  server.update_controller_info(controller_info);

  fn to_stick_value(input: f32) -> u8 {
    (input * 127.0 + 127.0) as u8 
  }

  let mut gilrs = Gilrs::new().unwrap();

  /*
  let mut mouse_manager = RawInputManager::new().unwrap();
  mouse_manager.register_devices(multiinput::DeviceType::Mice);
  */

  println!("Running");

  let now = Instant::now();
  while running.load(Ordering::SeqCst) {
    // Consume controller events
    while let Some(_event) = gilrs.next_event() {
    }

    let mut delta_rotation_x = 0.0;
    let mut delta_rotation_y = 0.0;
    let mut delta_mouse_wheel = 0.0;
    while let Ok(event) = m_rx.try_recv() {
        delta_rotation_x += event.x as f32;
        delta_rotation_y -= event.y as f32;
        delta_mouse_wheel += event.wheel as f32;
        //println!( "w={} x={}, y={}, left={}, middle={}, right={}", event.wheel, event.x, event.y, event.left, event.middle, event.right);
    }

    /*
    while let Some(event) = mouse_manager.get_event() {
      match event {
        RawEvent::MouseMoveEvent(_mouse_id, delta_x, delta_y) => {
          delta_rotation_x += delta_x as f32;
          delta_rotation_y += delta_y as f32;
        },
        RawEvent::MouseWheelEvent(_mouse_id, delta) => {
          delta_mouse_wheel += delta as f32;          
                _ => ()
      }
    }
    */

    let first_gamepad = gilrs.gamepads().next();
    let controller_data = {
      if let Some((_id, gamepad)) = first_gamepad {
        let analog_button_value = |button| {
          gamepad.button_data(button).map(|data| (data.value() * 255.0) as u8).unwrap_or(0)
        };

        ControllerData {
          connected: true,
          d_pad_left: gamepad.is_pressed(Button::DPadLeft),
          d_pad_down: gamepad.is_pressed(Button::DPadDown),
          d_pad_right: gamepad.is_pressed(Button::DPadRight),
          d_pad_up: gamepad.is_pressed(Button::DPadUp),
          start: gamepad.is_pressed(Button::Start),
          right_stick_button: gamepad.is_pressed(Button::RightThumb),
          left_stick_button: gamepad.is_pressed(Button::LeftThumb),
          select:  gamepad.is_pressed(Button::Select),
          triangle: gamepad.is_pressed(Button::North),
          circle: gamepad.is_pressed(Button::East),
          cross: gamepad.is_pressed(Button::South),
          square: gamepad.is_pressed(Button::West),
          r1: gamepad.is_pressed(Button::RightTrigger),
          l1: gamepad.is_pressed(Button::LeftTrigger),
          r2: gamepad.is_pressed(Button::RightTrigger2),
          l2: gamepad.is_pressed(Button::LeftTrigger2),
          ps: analog_button_value(Button::Mode),
          left_stick_x: to_stick_value(gamepad.value(Axis::LeftStickX)),
          left_stick_y: to_stick_value(gamepad.value(Axis::LeftStickY)),
          right_stick_x: to_stick_value(gamepad.value(Axis::RightStickX)),
          right_stick_y: to_stick_value(gamepad.value(Axis::RightStickY)),
          analog_d_pad_left: analog_button_value(Button::DPadLeft),
          analog_d_pad_down: analog_button_value(Button::DPadDown),
          analog_d_pad_right: analog_button_value(Button::DPadRight),
          analog_d_pad_up: analog_button_value(Button::DPadUp),
          analog_triangle: analog_button_value(Button::North),
          analog_circle: analog_button_value(Button::East),
          analog_cross: analog_button_value(Button::South),
          analog_square: analog_button_value(Button::West),
          analog_r1: analog_button_value(Button::RightTrigger),
          analog_l1: analog_button_value(Button::LeftTrigger),
          analog_r2: analog_button_value(Button::RightTrigger2),
          analog_l2: analog_button_value(Button::LeftTrigger2),
          motion_data_timestamp: now.elapsed().as_micros() as u64,
          gyroscope_pitch: -delta_rotation_y * 3.0,
          gyroscope_roll: -delta_rotation_x * 2.0,
          gyroscope_yaw: delta_mouse_wheel * 300.0,
          .. Default::default()
        }
      } 
      else {
        ControllerData {
          connected: true,
          motion_data_timestamp: now.elapsed().as_micros() as u64,
          gyroscope_pitch: -delta_rotation_y * 3.0,
          gyroscope_roll: -delta_rotation_x * 2.0,
          gyroscope_yaw: delta_mouse_wheel * 300.0,
          .. Default::default()
        }
      }
    };

    //println!("Hey {}", controller_data.gyroscope_roll);
    server.update_controller_data(0, controller_data);

    std::thread::sleep(Duration::from_millis(10));
  }

  server_thread_join_handle.join().unwrap();
}
